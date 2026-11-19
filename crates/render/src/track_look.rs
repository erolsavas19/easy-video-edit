//! Easy Video Edit: tracking by look, for objects with little detail of their own (a black dog, a
//! person in plain clothes) that the feature tracker in [`crate::track`] loses.
//!
//! The method is textbook template matching with a scale search: the picture inside the box is
//! kept as a small grey template; in the next frame every nearby position and a few sizes are
//! compared with it by zero-mean normalised cross-correlation (ZNCC), and the best one becomes the
//! new box. The template then takes in a little of the new picture, so a slowly turning or
//! growing object stays recognised. A best score under [`LOST_SCORE`] means the object is gone
//! (hidden or out of the picture).

use rayon::prelude::*;

use crate::track::Prepared;

/// Longest side of the template, in samples.
const TEMPLATE_SIDE: usize = 40;
/// Sizes tried per frame in the wide search, relative to the current box.
const SCALES: [f32; 5] = [0.94, 0.97, 1.0, 1.03, 1.06];
/// Sizes tried round the best place: an object walking up grows well under 1 % a frame.
const FINE_SCALES: [f32; 9] = [0.96, 0.97, 0.98, 0.99, 1.0, 1.01, 1.02, 1.03, 1.04];
/// How much of the new picture the template takes in per frame.
const LEARN: f32 = 0.12;
/// Share of the score that compares with the first frame's look.
const ANCHOR: f32 = 0.4;
/// Best ZNCC below this: the object is lost.
pub const LOST_SCORE: f32 = 0.35;
/// Smallest box side, in frame pixels.
const MIN_SIDE: f32 = 6.0;

/// An axis-aligned box: centre and size, in frame pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LookBox {
    pub cx: f32,
    pub cy: f32,
    pub w: f32,
    pub h: f32,
}

/// The object's look, carried from frame to frame.
#[derive(Clone, Debug)]
pub struct Look {
    tw: usize,
    th: usize,
    /// zero-mean, unit-norm template
    tpl: Vec<f32>,
    /// the first frame's template: anchors the size (a template that only follows the box would
    /// let the box stay small while the object grows)
    first: Vec<f32>,
    /// the last frame's movement: where to expect the object next
    vel: (f32, f32),
}

/// The template-sized patch of `g` under `b`, zero-mean and unit-norm; `None` when flat.
fn patch(p: &Prepared, b: LookBox, tw: usize, th: usize) -> Option<Vec<f32>> {
    // the pyramid level whose pixels are about as far apart as the samples: no aliasing on fine
    // texture, and a smooth score for the search
    let spacing = (b.w / tw as f32).max(b.h / th as f32);
    let lv = if spacing.is_finite() && spacing > 1.0 { (spacing.log2().floor() as usize).min(p.level_count().saturating_sub(1)) } else { 0 };
    let g = p.level(lv);
    let k = 1.0 / (1u32 << lv) as f32;
    let to = |v: f32| (v + 0.5) * k - 0.5;
    let (x0, y0) = (b.cx - b.w / 2.0, b.cy - b.h / 2.0);
    let (sx, sy) = (b.w / tw as f32, b.h / th as f32);
    let mut v = Vec::with_capacity(tw * th);
    for j in 0..th {
        let y = to(y0 + (j as f32 + 0.5) * sy);
        for i in 0..tw {
            // inside the picture (clamped there anyway): keeps hostile boxes from overflowing
            let (x, y) = (to(x0 + (i as f32 + 0.5) * sx).clamp(-1.0, g.w as f32), y.clamp(-1.0, g.h as f32));
            v.push(g.sample(x, y));
        }
    }
    normalise(&mut v).then_some(v)
}

fn normalise(v: &mut [f32]) -> bool {
    if v.is_empty() {
        return false;
    }
    let mean = v.iter().sum::<f32>() / v.len() as f32;
    let mut ss = 0.0;
    for x in v.iter_mut() {
        *x -= mean;
        ss += *x * *x;
    }
    let n = ss.sqrt();
    if !n.is_finite() || n < 1e-4 {
        return false;
    }
    for x in v.iter_mut() {
        *x /= n;
    }
    true
}

impl Look {
    /// The look of the picture inside `b`; `None` for a flat (featureless) or tiny box.
    pub fn new(p: &Prepared, b: LookBox) -> Option<Self> {
        if !(b.w.is_finite() && b.h.is_finite() && b.cx.is_finite() && b.cy.is_finite())
            || b.w < MIN_SIDE
            || b.h < MIN_SIDE
            || b.w > 4.0 * p.base().w as f32
            || b.h > 4.0 * p.base().h as f32
        {
            return None;
        }
        let long = b.w.max(b.h);
        let tw = ((TEMPLATE_SIDE as f32 * b.w / long).round() as usize).clamp(8, TEMPLATE_SIDE);
        let th = ((TEMPLATE_SIDE as f32 * b.h / long).round() as usize).clamp(8, TEMPLATE_SIDE);
        let tpl = patch(p, b, tw, th)?;
        Some(Look { tw, th, first: tpl.clone(), tpl, vel: (0.0, 0.0) })
    }

    fn score(&self, p: &Prepared, b: LookBox) -> f32 {
        patch(p, b, self.tw, self.th).map_or(-1.0, |p| {
            let now: f32 = p.iter().zip(&self.tpl).map(|(a, b)| a * b).sum();
            let first: f32 = p.iter().zip(&self.first).map(|(a, b)| a * b).sum();
            ANCHOR * first + (1.0 - ANCHOR) * now
        })
    }

    /// Finds the object in `g` near `b` (its box in the previous frame): the new box and its score,
    /// or `None` when it is lost.
    pub fn step(&mut self, p: &Prepared, b: LookBox) -> Option<(LookBox, f32)> {
        let g = p.base();
        let reach = (0.35 * b.w.max(b.h)).clamp(6.0, 120.0);
        // coarse grid, then a fine one round the best
        let coarse = (reach / 10.0).max(1.0);
        let expect = (b.cx + self.vel.0, b.cy + self.vel.1);
        let best = |centre: (f32, f32), radius: f32, stride: f32, scales: &[f32], size: (f32, f32)| -> Option<(LookBox, f32)> {
            let n = (radius / stride).ceil() as i32;
            let cands: Vec<LookBox> = (-n..=n)
                .flat_map(|dy| (-n..=n).map(move |dx| (dx, dy)))
                .flat_map(|(dx, dy)| {
                    scales.iter().map(move |s| LookBox {
                        cx: centre.0 + dx as f32 * stride,
                        cy: centre.1 + dy as f32 * stride,
                        w: (size.0 * s).max(MIN_SIDE),
                        h: (size.1 * s).max(MIN_SIDE),
                    })
                })
                .filter(|c| c.cx >= 0.0 && c.cy >= 0.0 && c.cx < g.w as f32 && c.cy < g.h as f32)
                .collect();
            cands
                .par_iter()
                .map(|c| {
                    // a slight preference for keeping the size: noise must not breathe the box
                    let ds = (c.w / b.w).ln().abs();
                    // and for where it was heading: a look-alike further off must not win
                    let dx = (c.cx - expect.0) / reach;
                    let dy = (c.cy - expect.1) / reach;
                    (*c, self.score(p, *c) - 0.02 * ds - 0.25 * (dx * dx + dy * dy))
                })
                .max_by(|a, b| a.1.total_cmp(&b.1))
        };
        let (c1, _) = best(expect, reach, coarse, &SCALES, (b.w, b.h))?;
        let (c2, _) = best((c1.cx, c1.cy), coarse, 1.0, &FINE_SCALES, (c1.w, c1.h))?;
        let raw = self.score(p, c2);
        if !raw.is_finite() || raw < LOST_SCORE {
            return None;
        }
        // take in a little of the new look
        if let Some(np) = patch(p, c2, self.tw, self.th) {
            let mut t: Vec<f32> = self.tpl.iter().zip(&np).map(|(a, b)| a * (1.0 - LEARN) + b * LEARN).collect();
            if normalise(&mut t) {
                self.tpl = t;
            }
        }
        self.vel = (c2.cx - b.cx, c2.cy - b.cy);
        Some((c2, raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::track::Gray;

    /// A dark disc of radius `r` at (`cx`, `cy`) on a lighter, gently varying ground.
    fn frame(w: usize, h: usize, cx: f32, cy: f32, r: f32) -> Prepared {
        let mut px = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                let ground = 0.55 + 0.05 * ((x as f32) * 0.07).sin() * ((y as f32) * 0.05).cos();
                let k = ((r - d) / 1.5).clamp(0.0, 1.0);
                px.push(ground * (1.0 - k) + 0.05 * k);
            }
        }
        Prepared::new(Gray::new(w, h, px))
    }

    #[test]
    fn follows_a_dark_object_that_comes_closer() {
        // a featureless dark disc moving right and growing 2 % a frame, like a black dog walking up
        let (w, h) = (320, 200);
        let (mut cx, cy, mut r) = (80.0f32, 100.0f32, 14.0f32);
        let g0 = frame(w, h, cx, cy, r);
        let mut b = LookBox { cx, cy, w: 2.6 * r, h: 2.6 * r };
        let mut look = Look::new(&g0, b).unwrap();
        for _ in 0..30 {
            cx += 3.0;
            r *= 1.02;
            let g = frame(w, h, cx, cy, r);
            let (nb, _) = look.step(&g, b).expect("kept");
            b = nb;
        }
        assert!((b.cx - cx).abs() < 3.0 && (b.cy - cy).abs() < 3.0, "centre {b:?} vs ({cx}, {cy})");
        let want = 2.6 * r;
        assert!((b.w / want - 1.0).abs() < 0.15, "size {} vs {want}", b.w);
    }

    #[test]
    fn says_lost_when_the_object_is_gone() {
        let (w, h) = (200, 120);
        let g0 = frame(w, h, 60.0, 60.0, 12.0);
        let b = LookBox { cx: 60.0, cy: 60.0, w: 30.0, h: 30.0 };
        let mut look = Look::new(&g0, b).unwrap();
        let empty = frame(w, h, -100.0, -100.0, 1.0);
        assert!(look.step(&empty, b).is_none());
    }

    #[test]
    fn refuses_hostile_boxes() {
        let g = frame(64, 64, 32.0, 32.0, 8.0);
        for b in [
            LookBox { cx: f32::NAN, cy: 1.0, w: 10.0, h: 10.0 },
            LookBox { cx: 10.0, cy: 10.0, w: 1.0, h: 10.0 },
            LookBox { cx: 1e30, cy: 1e30, w: 1e30, h: 1e30 },
        ] {
            let _ = Look::new(&g, b).and_then(|mut l| l.step(&g, b));
        }
        // a flat picture has no look
        let flat = Prepared::new(Gray::new(32, 32, vec![0.5; 32 * 32]));
        assert!(Look::new(&flat, LookBox { cx: 16.0, cy: 16.0, w: 10.0, h: 10.0 }).is_none());
    }
}
