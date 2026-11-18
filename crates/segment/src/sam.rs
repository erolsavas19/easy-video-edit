//! MobileSAM on candle: a boxed object cut out of a frame.

use std::path::Path;

use candle_core::{DType, Device, IndexOp, Module, Tensor};
use candle_transformers::models::segment_anything::mask_decoder::MaskDecoder;
use candle_transformers::models::segment_anything::prompt_encoder::PromptEncoder;
use candle_transformers::models::segment_anything::sam::IMAGE_SIZE;
use candle_transformers::models::segment_anything::tiny_vit::{TinyViT, tiny_vit_5m};

use crate::{Result, SegmentError};

fn model_err(e: candle_core::Error) -> SegmentError {
    SegmentError::Model(e.to_string())
}

/// A loaded MobileSAM: candle's parts, put together so the box itself is the prompt (candle's
/// `Sam` takes points only, and a box is what the model was trained to follow).
pub struct Segmenter {
    encoder: TinyViT,
    prompt: PromptEncoder,
    decoder: MaskDecoder,
    mean: Tensor,
    std: Tensor,
    device: Device,
}

/// Share of the box's size added round it before cropping: the model sees some background,
/// where the negative points go.
const MARGIN: f32 = 0.18;

impl Segmenter {
    /// Loads the model from its PyTorch checkpoint (`mobile_sam.pt`).
    pub fn load(path: &Path) -> Result<Segmenter> {
        let device = Device::Cpu;
        let vb = candle_nn::VarBuilder::from_pth(path, DType::F32, &device).map_err(model_err)?;
        let side = IMAGE_SIZE / 16;
        let encoder = tiny_vit_5m(vb.pp("image_encoder")).map_err(model_err)?;
        let prompt = PromptEncoder::new(256, (side, side), (IMAGE_SIZE, IMAGE_SIZE), 16, vb.pp("prompt_encoder")).map_err(model_err)?;
        let decoder = MaskDecoder::new(256, 3, 3, 256, vb.pp("mask_decoder")).map_err(model_err)?;
        let mean = Tensor::new(&[123.675f32, 116.28, 103.53], &device).and_then(|t| t.reshape((3, 1, 1))).map_err(model_err)?;
        let std = Tensor::new(&[58.395f32, 57.12, 57.375], &device).and_then(|t| t.reshape((3, 1, 1))).map_err(model_err)?;
        Ok(Segmenter { encoder, prompt, decoder, mean, std, device })
    }

    /// The object in box `bx` (`[x0, y0, x1, y1]`, frame pixels) of an RGB8 frame `w` × `h`: its
    /// outline in frame pixels, or `None` when the model finds nothing.
    pub fn cut(&self, rgb: &[u8], w: usize, h: usize, bx: [f32; 4]) -> Result<Option<Vec<[f32; 2]>>> {
        let n = w.checked_mul(h).and_then(|n| n.checked_mul(3)).ok_or_else(|| SegmentError::Input("frame too large".into()))?;
        if w == 0 || h == 0 || rgb.len() < n {
            return Err(SegmentError::Input("frame size does not match its pixels".into()));
        }
        if bx.iter().any(|v| !v.is_finite()) {
            return Err(SegmentError::Input("box is not finite".into()));
        }
        let (bx0, bx1) = (bx[0].min(bx[2]), bx[0].max(bx[2]));
        let (by0, by1) = (bx[1].min(bx[3]), bx[1].max(bx[3]));
        let (mx, my) = ((bx1 - bx0) * MARGIN, (by1 - by0) * MARGIN);
        let cx0 = (bx0 - mx).floor().clamp(0.0, w as f32 - 1.0) as usize;
        let cy0 = (by0 - my).floor().clamp(0.0, h as f32 - 1.0) as usize;
        let cx1 = (bx1 + mx).ceil().clamp(cx0 as f32 + 1.0, w as f32) as usize;
        let cy1 = (by1 + my).ceil().clamp(cy0 as f32 + 1.0, h as f32) as usize;
        let (cw, ch) = (cx1 - cx0, cy1 - cy0);
        if cw < 4 || ch < 4 {
            return Ok(None);
        }
        // the crop scaled so its longer side is the model's input size
        let k = IMAGE_SIZE as f32 / cw.max(ch) as f32;
        let (rw, rh) = (((cw as f32 * k).round() as usize).clamp(1, IMAGE_SIZE), ((ch as f32 * k).round() as usize).clamp(1, IMAGE_SIZE));
        let mut planar = vec![0u8; 3 * rw * rh];
        for y in 0..rh {
            let sy = (cy0 as f32 + (y as f32 + 0.5) / k - 0.5).clamp(0.0, h as f32 - 1.0);
            for x in 0..rw {
                let sx = (cx0 as f32 + (x as f32 + 0.5) / k - 0.5).clamp(0.0, w as f32 - 1.0);
                let px = bilinear(rgb, w, h, sx, sy);
                for c in 0..3 {
                    if let Some(d) = planar.get_mut(c * rw * rh + y * rw + x) {
                        *d = px[c];
                    }
                }
            }
        }
        let img = Tensor::from_vec(planar, (3, rh, rw), &self.device).map_err(model_err)?;
        let emb = (|| {
            let x = img.to_dtype(DType::F32)?.broadcast_sub(&self.mean)?.broadcast_div(&self.std)?;
            let x = x.pad_with_zeros(1, 0, IMAGE_SIZE - rh)?.pad_with_zeros(2, 0, IMAGE_SIZE - rw)?;
            self.encoder.forward(&x.unsqueeze(0)?)
        })()
        .map_err(model_err)?;
        // prompt: the box, in the scaled crop's pixels
        let to_in = |v: f32, o: usize| (v - o as f32) * k;
        let boxes = Tensor::new(&[[to_in(bx0, cx0), to_in(by0, cy0), to_in(bx1, cx0), to_in(by1, cy0)]], &self.device).map_err(model_err)?;
        let (sparse, dense) = self.prompt.forward(None, Some(&boxes), None).map_err(model_err)?;
        // candle joins the two corner embeddings side by side ([1, 512]); the decoder wants them
        // as two tokens ([1, 2, 256])
        let sparse = if sparse.rank() == 2 { sparse.reshape((1, (), 256)).map_err(model_err)? } else { sparse };
        let pe = self.prompt.get_dense_pe().map_err(model_err)?;
        // one mask: a box is not ambiguous the way a single point is
        let (low, _iou) = self.decoder.forward(&emb, &pe, &sparse, &dense, false).map_err(model_err)?;
        let best = 0usize;
        let grid = low.i((0, best)).map_err(model_err)?;
        let (gh, gw) = grid.dims2().map_err(model_err)?;
        let mut logits: Vec<f32> = grid.flatten_all().and_then(|t| t.to_vec1()).map_err(model_err)?;
        // cells beyond the crop (the model pads it to a square) are background
        let cell = IMAGE_SIZE as f32 / gw.max(1) as f32;
        let (vw, vh) = (((rw as f32) / cell).ceil() as usize, ((rh as f32) / cell).ceil() as usize);
        for y in 0..gh {
            for x in 0..gw {
                if (x >= vw || y >= vh)
                    && let Some(v) = logits.get_mut(y * gw + x)
                {
                    *v = -1.0;
                }
            }
        }
        // and so is everything well outside the box: the object is the boxed one
        let to_grid = |v: f32, o: usize| (v - o as f32) * k / cell;
        let pad = 0.06 * (bx1 - bx0).max(by1 - by0);
        let (gx0, gx1) = (to_grid(bx0 - pad, cx0), to_grid(bx1 + pad, cx0));
        let (gy0, gy1) = (to_grid(by0 - pad, cy0), to_grid(by1 + pad, cy0));
        for y in 0..gh {
            for x in 0..gw {
                let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                if (fx < gx0 || fx > gx1 || fy < gy0 || fy > gy1)
                    && let Some(v) = logits.get_mut(y * gw + x)
                {
                    *v = -1.0;
                }
            }
        }
        let Some(o) = crate::outline(&logits, gw, gh) else { return Ok(None) };
        // grid → scaled crop → crop → frame
        Ok(Some(o.into_iter().map(|p| [cx0 as f32 + p[0] * cell / k, cy0 as f32 + p[1] * cell / k]).collect()))
    }
}

fn bilinear(rgb: &[u8], w: usize, h: usize, x: f32, y: f32) -> [u8; 3] {
    let (x0, y0) = (x.floor().max(0.0) as usize, y.floor().max(0.0) as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let at = |xx: usize, yy: usize, c: usize| f32::from(rgb.get((yy * w + xx) * 3 + c).copied().unwrap_or(0));
    let mut out = [0u8; 3];
    for (c, o) in out.iter_mut().enumerate() {
        let top = at(x0, y0, c) * (1.0 - fx) + at(x1, y0, c) * fx;
        let bot = at(x0, y1, c) * (1.0 - fx) + at(x1, y1, c) * fx;
        *o = (top * (1.0 - fy) + bot * fy).round().clamp(0.0, 255.0) as u8;
    }
    out
}
