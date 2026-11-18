//! Background removal: `masks.cutout` cuts the object in a box mask (usually a tracked one) out of
//! its clip, as an Opacity mask, so what is under the clip shows round the object.
//!
//! MobileSAM ([`filmcraft_segment`], feature `segment`) runs on every `step`-th frame of the box's
//! frames; each result is resampled to [`filmcraft_segment::RESAMPLED_POINTS`] points relative to
//! the box, and the frames in between take the blend of the two nearest results placed on the
//! box of their own frame, so the cut-out moves with the tracked box and changes shape smoothly.
//! The model's work runs as a background job; the paths are written as Mask Path keyframes of a
//! new "Cutout" mask on the clip's Opacity as they arrive (one undo step), like mask tracking.

use filmcraft_geom::Vec2;
use filmcraft_project::{Mask, MaskPath};
use filmcraft_time::Tick;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

use crate::commands::{CommandSpec, always, bad, bool_p, has_seq, u64_p};
use crate::masks::MaskSel;
use crate::{EngineError, Result, Session};

pub(crate) fn commands() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            id: "masks.cutout",
            label: "Cut Out Object",
            menu: &[],
            shortcut: None,
            params: r#"{"clip":id?,"effect":index|id?,"mask":n?,"step":n?,"download":bool?,"wait":bool?}"#,
            enabled: has_seq,
            run: cutout,
            journal: true,
            named: crate::commands::NamedKeys::NONE,
        },
        CommandSpec {
            id: "segment.model",
            label: "Segmentation Model",
            menu: &[],
            shortcut: None,
            params: "{}",
            enabled: always,
            run: model_info,
            journal: false,
            named: crate::commands::NamedKeys::NONE,
        },
    ]
}

/// Whether this build cuts out objects, whether the model is installed, and where it comes from
/// (for the download confirmation).
fn model_info(_: &mut Session, _: &Value) -> Result<Value> {
    let m = filmcraft_segment::MODEL;
    let installed = crate::transcript::models_dir().is_some_and(|d| filmcraft_segment::installed(&d));
    Ok(json!({
        "available": cfg!(feature = "segment"),
        "installed": installed,
        "name": m.name,
        "size": m.file.size,
        "license": m.license,
        "licenseUrl": m.license_url,
        "author": m.author,
        "source": m.source,
        "url": m.file.url,
    }))
}

/// Model results kept apart by at most this many frames.
const MAX_STEP: u64 = 60;
/// Most frames one cut-out covers.
const MAX_FRAMES: usize = 20_000;
/// Longest side of the frames the model is given (the result is in clip pixels).
const CUT_MAX_SIDE: f64 = 1280.0;

fn cutout(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "masks.cutout";
    if !cfg!(feature = "segment") {
        return Err(bad(cmd, filmcraft_segment::SegmentError::Unavailable.to_string()));
    }
    let clip = crate::commands::clip_p(p, "clip")
        .or_else(|| s.state.selected_mask.map(|m| m.clip))
        .or_else(|| s.state.selection.first().copied())
        .ok_or_else(|| bad(cmd, "need `clip`"))?;
    let sel = s.state.selected_mask;
    let seq_id = s.state.active_sequence.ok_or(EngineError::NoSequence)?;
    let seq = s.active_sequence().ok_or(EngineError::NoSequence)?;
    let (_, it) = seq.find_item(clip).ok_or_else(|| bad(cmd, "no such clip"))?;
    let it = it.clone();
    let ei = match p.get("effect") {
        Some(Value::Number(n)) => n.as_u64().map(|v| v as usize),
        Some(Value::String(id)) => it.effects.iter().position(|e| e.effect == *id),
        _ => sel.filter(|m| m.clip == clip).map(|m| m.effect),
    }
    .filter(|i| *i < it.effects.len())
    .ok_or_else(|| bad(cmd, "need `effect` (the effect holding the box)"))?;
    let mi = u64_p(p, "mask")
        .map(|m| m as usize)
        .or(sel.filter(|x| x.clip == clip && x.effect == ei).map(|x| x.mask))
        .ok_or_else(|| bad(cmd, "need `mask` (the box)"))?;
    let boxm = it.effects.get(ei).and_then(|e| e.masks.get(mi)).cloned().ok_or_else(|| bad(cmd, format!("no mask {mi}")))?;
    let oi = it.effects.iter().position(|e| e.effect == "opacity").ok_or_else(|| bad(cmd, "the clip has no Opacity"))?;
    let step = u64_p(p, "step").unwrap_or(5).clamp(1, MAX_STEP) as usize;
    let download = bool_p(p, "download").unwrap_or(false);
    let models = crate::transcript::models_dir().ok_or_else(|| bad(cmd, "no data directory for the segmentation model"))?;
    if !filmcraft_segment::installed(&models) && !download {
        return Err(bad(cmd, "the segmentation model is not installed: download it first (`download`: true)"));
    }
    // the frames: the box's tracked keyframes, else every frame of the clip
    let rate = s.project.item(it.item).map(|i| i.frame_rate()).unwrap_or(seq.settings.frame_rate);
    let fd = rate.frame_duration().0.max(1);
    let mut times: Vec<Tick> = boxm.path.keyframes.iter().map(|k| k.time).collect();
    times.sort();
    times.dedup();
    if times.len() < 2 {
        let (a, b) = (it.source_time_at(it.start), it.source_time_at(it.end() - Tick(1)));
        let (lo, hi) = (a.min(b), a.max(b));
        times = (0..).map(|i: i64| Tick(lo.0.saturating_add(i.saturating_mul(fd)))).take_while(|t| *t <= hi).take(MAX_FRAMES).collect();
    }
    times.truncate(MAX_FRAMES);
    if times.is_empty() {
        return Err(bad(cmd, "the clip has no frames to cut out"));
    }
    let src = s.source(it.item).ok_or_else(|| EngineError::Other("the clip has no media to cut out".into()))?;
    let size = filmcraft_render::source_size(&s.project, it.item).ok_or_else(|| bad(cmd, "the clip has no picture"))?;
    let scale = (CUT_MAX_SIDE / f64::from(size.0.max(size.1).max(1))).min(1.0) as f32;
    // the Cutout mask on Opacity, holding the box until the first result arrives
    let first_path = boxm.path_at(times[0]);
    let mo = s.edit("Cut Out Object", move |pr, _| {
        let q = pr.sequence_mut(seq_id).ok_or(EngineError::NoSequence)?;
        let (_, t) = q.find_item_mut(clip).ok_or(filmcraft_edit::EditError::NoItem(clip))?;
        let e = t.effects.get_mut(oi).ok_or_else(|| bad(cmd, "the clip has no Opacity"))?;
        let mut m = Mask::new("Cutout", first_path);
        m.feather.value = filmcraft_project::ParamValue::Float(3.0);
        e.masks.push(m);
        Ok(e.masks.len() - 1)
    })?;
    let id = s.jobs.iter().map(|j| j.id).max().unwrap_or(0) + 1;
    let samples: Vec<usize> = {
        let mut v: Vec<usize> = (0..times.len()).step_by(step).collect();
        if v.last() != Some(&(times.len() - 1)) {
            v.push(times.len() - 1);
        }
        v
    };
    let job = crate::Job { id, label: "Cut Out Object".into(), progress: Default::default(), result: Default::default() };
    job.progress.total.store(samples.len() as u64, std::sync::atomic::Ordering::Relaxed);
    let keys: Arc<Mutex<Vec<(Tick, MaskPath)>>> = Arc::default();
    let (prog, res, out) = (job.progress.clone(), job.result.clone(), keys.clone());
    let run = move || run_cutout(CutJob { src, size, scale, times, samples, boxm, models, download }, &prog, &res, &out);
    let run = crate::export_tools::guard_job(job.progress.clone(), job.result.clone(), run);
    s.jobs.push(job);
    s.mask_jobs.push(crate::masks::PendingTrack::new(id, MaskSel { clip, effect: oi, mask: mo }, seq_id, keys, s.document_epoch));
    let wait = bool_p(p, "wait").unwrap_or(false);
    if wait || cfg!(target_arch = "wasm32") {
        run();
        crate::masks::poll(s);
    } else {
        std::thread::Builder::new().name("filmcraft-cutout".into()).spawn(run).map_err(|e| EngineError::Other(e.to_string()))?;
    }
    let st = s.jobs.iter().find(|j| j.id == id).map(crate::Job::to_json).unwrap_or(Value::Null);
    Ok(json!({"job": id, "clip": clip.0, "effect": oi, "mask": mo, "status": st}))
}

/// What the background job needs.
#[cfg_attr(not(feature = "segment"), allow(dead_code))]
struct CutJob {
    src: Arc<dyn filmcraft_media::MediaSource>,
    size: (u32, u32),
    scale: f32,
    /// Media times of the frames, in order.
    times: Vec<Tick>,
    /// Indices (into `times`) the model runs on.
    samples: Vec<usize>,
    boxm: Mask,
    models: std::path::PathBuf,
    download: bool,
}

type JobResult = Arc<Mutex<Option<std::result::Result<filmcraft_export::Report, String>>>>;

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The box of the mask at `t`: its bounds, in clip pixels.
#[cfg_attr(not(feature = "segment"), allow(dead_code))]
fn box_at(m: &Mask, t: Tick) -> Option<(Vec2, Vec2)> {
    let (lo, hi) = m.path_at(t).bounds();
    (lo.x.is_finite() && lo.y.is_finite() && hi.x.is_finite() && hi.y.is_finite() && hi.x > lo.x && hi.y > lo.y).then_some((lo, hi))
}

/// A shape relative to its box (0..1 across it) placed on box `(lo, hi)`.
#[cfg_attr(not(feature = "segment"), allow(dead_code))]
fn place(shape: &[[f32; 2]], (lo, hi): (Vec2, Vec2)) -> MaskPath {
    let pts: Vec<Vec2> = shape.iter().map(|q| Vec2::new(lo.x + f64::from(q[0]) * (hi.x - lo.x), lo.y + f64::from(q[1]) * (hi.y - lo.y))).collect();
    MaskPath::polygon(&pts)
}

fn run_cutout(j: CutJob, prog: &Arc<filmcraft_export::Progress>, res: &JobResult, out: &Arc<Mutex<Vec<(Tick, MaskPath)>>>) {
    use std::sync::atomic::Ordering;
    let t0 = web_time::Instant::now();
    let r = cut_all(&j, prog, out);
    let secs = t0.elapsed().as_secs_f64();
    let done = prog.done.load(Ordering::Relaxed);
    *lock(&prog.status) = match &r {
        Ok(()) => format!("Cut out {} frame(s) in {secs:.0}s", lock(out).len()),
        Err(e) => e.clone(),
    };
    let report = match r {
        Err(e) if e != "stopped" => Err(e),
        _ => Ok(filmcraft_export::Report { frames: done, seconds: secs, render_fps: done as f64 / secs.max(1e-6), ..Default::default() }),
    };
    prog.finished.store(true, Ordering::Relaxed);
    *lock(res) = Some(report);
}

#[cfg(feature = "segment")]
fn cut_all(j: &CutJob, prog: &Arc<filmcraft_export::Progress>, out: &Arc<Mutex<Vec<(Tick, MaskPath)>>>) -> std::result::Result<(), String> {
    use std::sync::atomic::Ordering;
    let model = filmcraft_segment::model_path(&j.models);
    if !filmcraft_segment::installed(&j.models) {
        if !j.download {
            return Err("the segmentation model is not installed".into());
        }
        *lock(&prog.status) = "Downloading the model…".into();
        let m = filmcraft_segment::MODEL;
        let file = filmcraft_speech::models::ModelFile { name: m.file.name, url: m.file.url, sha256: m.file.sha256, size: m.file.size };
        let dir = j.models.join(m.id);
        let p2 = prog.clone();
        filmcraft_speech::models::download_files(&dir, &[file], &mut |got, total, _| {
            *lock(&p2.status) = format!("Downloading the model… {} / {} MB", got / 1_000_000, total.max(1) / 1_000_000);
            !p2.cancel.load(Ordering::Relaxed)
        })
        .map_err(|e| e.to_string())?;
    }
    *lock(&prog.status) = "Loading the model…".into();
    let seg = filmcraft_segment::Segmenter::load(&model).map_err(|e| e.to_string())?;
    let k = |f_w: u32| f64::from(f_w) / f64::from(j.size.0.max(1));
    let n = filmcraft_segment::RESAMPLED_POINTS;
    // the model's shape at frame `i`, relative to that frame's box
    let shape_at = |i: usize| -> std::result::Result<Option<Vec<[f32; 2]>>, String> {
        let t = j.times[i];
        let Some((lo, hi)) = box_at(&j.boxm, t) else { return Ok(None) };
        let f = j.src.video_frame(filmcraft_media::FrameRequest { time: t, scale: j.scale }).map_err(|e| format!("can't decode a frame: {e}"))?;
        let (w, h) = (f.width as usize, f.height as usize);
        let rgba = f.to_rgba8();
        let rgb: Vec<u8> = rgba.as_chunks::<4>().0.iter().flat_map(|p| [p[0], p[1], p[2]]).collect();
        let kk = k(f.width);
        let bx = [(lo.x * kk) as f32, (lo.y * kk) as f32, (hi.x * kk) as f32, (hi.y * kk) as f32];
        let Some(o) = seg.cut(&rgb, w, h, bx).map_err(|e| e.to_string())? else { return Ok(None) };
        let (bw, bh) = ((hi.x - lo.x) as f32, (hi.y - lo.y) as f32);
        let rel: Vec<[f32; 2]> = o.iter().map(|q| [(q[0] / kk as f32 - lo.x as f32) / bw, (q[1] / kk as f32 - lo.y as f32) / bh]).collect();
        Ok(filmcraft_segment::resample(&rel, n))
    };
    let mut prev: Option<(usize, Option<Vec<[f32; 2]>>)> = None;
    for (si, &i) in j.samples.iter().enumerate() {
        if prog.cancel.load(Ordering::Relaxed) {
            return Err("stopped".into());
        }
        *lock(&prog.status) = format!("Frame {} of {}", si + 1, j.samples.len());
        let cur = shape_at(i)?;
        let last = si + 1 == j.samples.len();
        if let Some((a, sa)) = &prev {
            // the frames from the previous result up to this one (this one too at the end)
            let end = if last { i + 1 } else { i };
            let span = (i - a).max(1) as f32;
            let mut batch = Vec::new();
            for f in *a..end {
                let w = (f - a) as f32 / span;
                let shape: Option<Vec<[f32; 2]>> = match (sa, &cur) {
                    (Some(x), Some(y)) => Some(x.iter().zip(y).map(|(p, q)| [p[0] + (q[0] - p[0]) * w, p[1] + (q[1] - p[1]) * w]).collect()),
                    (Some(x), None) | (None, Some(x)) => Some(x.clone()),
                    (None, None) => None,
                };
                if let (Some(sh), Some(b)) = (shape, box_at(&j.boxm, j.times[f])) {
                    batch.push((j.times[f], place(&sh, b)));
                }
            }
            lock(out).extend(batch);
        } else if last && let (Some(sh), Some(b)) = (&cur, box_at(&j.boxm, j.times[i])) {
            lock(out).push((j.times[i], place(sh, b)));
        }
        prev = Some((i, cur));
        prog.done.store(si as u64 + 1, Ordering::Relaxed);
    }
    if lock(out).is_empty() {
        return Err("nothing was found in the box".into());
    }
    Ok(())
}

#[cfg(not(feature = "segment"))]
fn cut_all(_: &CutJob, _: &Arc<filmcraft_export::Progress>, _: &Arc<Mutex<Vec<(Tick, MaskPath)>>>) -> std::result::Result<(), String> {
    Err(filmcraft_segment::SegmentError::Unavailable.to_string())
}
