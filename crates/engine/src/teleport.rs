//! Easy Video Edit: the teleport effect. A person vanishes (or appears) on the spot in a burst of
//! light, while the place stays as it was.
//!
//! What it needs is a frame of the same shot with the place empty (before the person walked in,
//! or after they left; the camera standing still). That frame, held, goes on a layer under the
//! clip; the clip fades out (or in) round the chosen moment, so only the person goes. A cut-out
//! (Replace Background) adds a coloured glow of the person's shape that flares up as they go;
//! without one the picture flashes bright instead.

use filmcraft_project::{ClipId, Interpolation, Keyframe, ParamValue as V, TrackItem};
use filmcraft_time::Tick;
use serde_json::{Value, json};

use crate::commands::{CommandSpec, bad, clip_p, f64_p, has_seq, str_p, time_p};
use crate::{EngineError, Result, Session};

pub(crate) fn commands() -> Vec<CommandSpec> {
    vec![CommandSpec {
        id: "effects.teleport",
        label: "Teleport",
        menu: &[],
        shortcut: None,
        params: r##"{"clip":id?,"mode":"out|in","time":ticks?,"plate":ticks?,"seconds":0.2-3?,"color":"#rrggbb"?,"plateName":str?,"lightName":str?}"##,
        enabled: has_seq,
        run: teleport,
        journal: true,
        named: crate::commands::NamedKeys::NONE,
    }]
}

/// The clip's Cutout mask (Replace Background): (Opacity index, mask index).
fn cutout(it: &TrackItem) -> Option<(usize, usize)> {
    let oi = it.effects.iter().position(|e| e.effect == "opacity")?;
    let mi = it.effects.get(oi)?.masks.iter().rposition(|m| m.name == "Cutout")?;
    Some((oi, mi))
}

fn hex(s: &str) -> Option<[f32; 4]> {
    let h = s.strip_prefix('#').unwrap_or(s);
    if h.len() != 6 || !h.is_ascii() {
        return None;
    }
    let c = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok().map(|v| f32::from(v) / 255.0);
    Some([c(0)?, c(2)?, c(4)?, 1.0])
}

/// Sets keyframes `keys` (effect time, value) on `param`, replacing any between the first and last.
fn set_keys(param: &mut filmcraft_project::Param, keys: &[(Tick, f64)]) {
    let (Some(a), Some(b)) = (keys.first().map(|k| k.0), keys.last().map(|k| k.0)) else { return };
    param.keyframes.retain(|k| k.time < a || k.time > b);
    for (t, v) in keys {
        let mut k = Keyframe::new(*t, V::Float(*v));
        k.interp = Interpolation::EaseOut;
        param.keyframes.push(k);
    }
    param.keyframes.sort_by_key(|k| k.time);
}

fn teleport(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "effects.teleport";
    let out = match str_p(p, "mode").unwrap_or("out") {
        "out" => true,
        "in" => false,
        m => return Err(bad(cmd, format!("mode `{m}`: out (vanish) or in (appear)"))),
    };
    let secs = match p.get("seconds") {
        None | Some(Value::Null) => 0.8,
        Some(_) => f64_p(p, "seconds").ok_or_else(|| bad(cmd, "`seconds` is a number"))?,
    };
    if !secs.is_finite() || !(0.2..=3.0).contains(&secs) {
        return Err(bad(cmd, "`seconds` is 0.2 to 3"));
    }
    let color = match str_p(p, "color") {
        Some(c) => hex(c).ok_or_else(|| bad(cmd, "`color` is #rrggbb"))?,
        None => [0.55, 0.85, 1.0, 1.0],
    };
    let clip = clip_p(p, "clip").or_else(|| s.state.selection.first().copied()).ok_or_else(|| bad(cmd, "select a video clip"))?;
    let at = time_p(s, p, "").unwrap_or(s.playhead());
    let seq = s.active_sequence().ok_or(EngineError::NoSequence)?;
    let (track, it) = seq.find_item(clip).ok_or_else(|| bad(cmd, "no such clip"))?;
    let ti = seq.video_tracks.iter().position(|t| t.id == track).ok_or_else(|| bad(cmd, "the teleport goes on a video clip"))?;
    if at < it.start || at >= it.end() {
        return Err(bad(cmd, "move the playhead onto the clip, to the moment of the teleport"));
    }
    let has_cutout = cutout(it).is_some();
    // something already under the clip (a new background): no empty frame needed there
    let covered = seq.video_tracks.iter().take(ti).any(|t| t.items.iter().any(|o| o.start < it.end() && it.start < o.end()));
    let plate = match p.get("plate") {
        None | Some(Value::Null) => None,
        Some(v) => {
            let t = v.as_i64().map(Tick).ok_or_else(|| bad(cmd, "`plate` is media ticks"))?;
            let (a, b) = (it.source_time_at(it.start), it.source_time_at(it.end() - Tick(1)));
            if t < a.min(b) || t > a.max(b) {
                return Err(bad(cmd, "the empty frame must be a frame of this clip"));
            }
            Some(t)
        }
    };
    if plate.is_none() && !(has_cutout && covered) {
        return Err(bad(cmd, "first pick a frame where the place is empty"));
    }
    // the new layers' names, in the interface's language when the caller gives them
    let name = |key: &str, fallback: &str| -> String {
        str_p(p, key).map(str::trim).filter(|n| !n.is_empty()).map(|n| n.chars().take(80).collect()).unwrap_or_else(|| fallback.to_string())
    };
    let (plate_name, light_name) = (name("plateName", "empty place"), name("lightName", "teleport light"));
    let half = Tick(((secs / 2.0) * filmcraft_time::TICKS_PER_SECOND as f64).round() as i64);
    let (t0, t1) = (it.effect_time_at((at - half).max(it.start)), it.effect_time_at((at + half).min(it.end() - Tick(1))));
    let tm = it.effect_time_at(at);
    let label = if out { "Teleport Out" } else { "Teleport In" };
    let ids = s.edit_sequence(label, move |q, ctx, _| {
        let mut made = Vec::new();
        let mut ti = q.video_tracks.iter().position(|t| t.id == track).ok_or_else(|| bad(cmd, "the clip's track is gone"))?;
        let it = q.video_tracks.get(ti).and_then(|t| t.items.iter().find(|i| i.id == clip)).cloned().ok_or(filmcraft_edit::EditError::NoItem(clip))?;
        // 1. the empty place, held, on its own layer under the clip
        if let Some(h) = plate.filter(|_| !(has_cutout && covered)) {
            let mut bg = it.clone();
            bg.id = ClipId(ctx.alloc());
            bg.link = None;
            bg.group = None;
            bg.name = format!("{} · {plate_name}", it.name);
            bg.frame_hold = Some(h);
            // just the picture: no tracking boxes, no cut-out, no fades
            bg.effects.retain(|e| e.masks.is_empty() || e.effect == "opacity");
            if let Some(o) = bg.effects.iter_mut().find(|e| e.effect == "opacity") {
                o.masks.clear();
                if let Some(op) = o.params.get_mut("opacity") {
                    op.keyframes.clear();
                    op.value = V::Float(100.0);
                }
            }
            let mut t = filmcraft_project::Track::new(filmcraft_project::TrackId(ctx.alloc()), filmcraft_project::TrackKind::Video, "Video 0".into());
            t.items.push(bg.clone());
            q.video_tracks.insert(ti, t);
            ti += 1;
            made.push(bg.id);
        }
        // 2. the person goes (or comes): the clip fades round the moment
        {
            let c = q.video_tracks.get_mut(ti).and_then(|t| t.items.iter_mut().find(|i| i.id == clip)).ok_or(filmcraft_edit::EditError::NoItem(clip))?;
            let o = c.effects.iter_mut().find(|e| e.effect == "opacity").ok_or_else(|| bad(cmd, "the clip has no Opacity"))?;
            let op = o.params.get_mut("opacity").ok_or_else(|| bad(cmd, "the clip has no Opacity"))?;
            let keys = if out { [(t0, 100.0), (t1, 0.0)] } else { [(t0, 0.0), (t1, 100.0)] };
            set_keys(op, &keys);
            if !has_cutout {
                // no shape to light up: the picture flashes bright as the person goes
                let mut fx = filmcraft_project::find_effect("brightness_contrast").ok_or_else(|| bad(cmd, "no Brightness & Contrast"))?.instance();
                if let Some(b) = fx.params.get_mut("brightness") {
                    set_keys(b, &[(t0, 0.0), (tm, 100.0), (t1, 0.0)]);
                }
                c.effects.push(fx);
            }
        }
        // 3. with a cut-out: a glow of the person's shape flares up on a layer above
        if has_cutout {
            let (oi, mi) = cutout(&it).ok_or_else(|| bad(cmd, "no cut-out"))?;
            let mut glow = it.clone();
            glow.id = ClipId(ctx.alloc());
            glow.link = None;
            glow.group = None;
            glow.name = format!("{} · {light_name}", it.name);
            let o = glow.effects.get_mut(oi).ok_or_else(|| bad(cmd, "no Opacity"))?;
            let mut m = o.masks.get(mi).cloned().ok_or_else(|| bad(cmd, "no cut-out"))?;
            m.expansion.value = V::Float(18.0);
            m.feather.value = V::Float(30.0);
            o.masks = vec![m];
            if let Some(op) = o.params.get_mut("opacity") {
                op.keyframes.clear();
                op.value = V::Float(0.0);
                set_keys(op, &[(t0, 0.0), (tm, 100.0), (t1, 0.0)]);
            }
            let mut tint = filmcraft_project::find_effect("tint").ok_or_else(|| bad(cmd, "no Tint"))?.instance();
            for (k, v) in [("black", V::Color(color)), ("white", V::Color([1.0, 1.0, 1.0, 1.0])), ("amount", V::Float(100.0))] {
                if let Some(x) = tint.params.get_mut(k) {
                    x.value = v;
                }
            }
            glow.effects.push(tint);
            let mut t = filmcraft_project::Track::new(filmcraft_project::TrackId(ctx.alloc()), filmcraft_project::TrackKind::Video, "Video 0".into());
            t.items.push(glow.clone());
            q.video_tracks.insert(ti + 1, t);
            made.push(glow.id);
        }
        crate::sequence_tools::renumber(&mut q.video_tracks, "Video");
        Ok(made)
    })?;
    Ok(json!({"clips": ids.iter().map(|c| c.0).collect::<Vec<_>>(), "glow": has_cutout}))
}
