//! Easy Video Edit's Object Tracking panel (the right side's Tracking tab): mark an object in the
//! picture with a box, track it through the clip and choose what follows it — a blur, a mosaic, a
//! title over it or above it, or an arrow. Built on FilmCraft's mask tracker: the box is a mask on
//! a blur or mosaic effect (so the effect stays inside it), `masks.track` moves it frame by frame,
//! and `masks.follow` turns its path into Motion keyframes of a title placed above the video.

use egui::{Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use serde_json::json;

use crate::FilmcraftApp;
use crate::icons::{self, Icon};
use crate::theme::Tokens;

/// What follows the tracked object.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Blur,
    Mosaic,
    Cover,
    TextAbove,
    Arrow,
}

const ACTIONS: [Action; 5] = [Action::Blur, Action::Mosaic, Action::Cover, Action::TextAbove, Action::Arrow];

impl Action {
    fn from_u8(v: u8) -> Action {
        ACTIONS.get(usize::from(v)).copied().unwrap_or(Action::Blur)
    }
    fn index(self) -> u8 {
        ACTIONS.iter().position(|a| *a == self).and_then(|i| u8::try_from(i).ok()).unwrap_or(0)
    }
    fn label(self) -> &'static str {
        match self {
            Action::Blur => tl!("Blur"),
            Action::Mosaic => tl!("Mosaic"),
            Action::Cover => tl!("Text Over It"),
            Action::TextAbove => tl!("Text Above It"),
            Action::Arrow => tl!("Arrow"),
        }
    }
    fn icon(self) -> Icon {
        match self {
            Action::Blur => Icon::Sparkle,
            Action::Mosaic => Icon::Grid,
            Action::Cover | Action::TextAbove => Icon::Type,
            Action::Arrow => Icon::TrackMaskFwd,
        }
    }
    /// The effect the box (a mask) sits on, and its settings.
    fn effect(self) -> (&'static str, serde_json::Value) {
        match self {
            Action::Blur => ("gaussian_blur", json!({"blurriness": 45.0})),
            Action::Mosaic => ("mosaic", json!({"horizontal": 70.0, "vertical": 40.0})),
            // the box only marks the object: a blur of nothing changes no pixel
            _ => ("gaussian_blur", json!({"blurriness": 0.0})),
        }
    }
    /// The template attached to the object, and where.
    fn follower(self) -> Option<(&'static str, &'static str)> {
        match self {
            Action::Cover => Some(("easyvideoedit:takip-kapak", "center")),
            Action::TextAbove => Some(("easyvideoedit:takip-yazi", "above")),
            Action::Arrow => Some(("easyvideoedit:takip-ok", "above")),
            _ => None,
        }
    }
}

/// The panel's working state (not saved).
#[derive(Clone, Debug, Default)]
pub struct TrackPanel {
    pub action: u8,
    /// The box being tracked: (clip, effect index, mask index).
    pub target: Option<(u64, usize, usize)>,
    /// The tracking job running for it.
    pub job: Option<u64>,
    /// The title that follows it.
    pub follower: Option<u64>,
}

/// The first selected video clip of the timeline: (clip, name, source width and height).
fn selected_video(app: &FilmcraftApp) -> Option<(filmcraft_project::ClipId, String, (u32, u32))> {
    let seq = app.session.active_sequence()?;
    app.session.state.selection.iter().find_map(|c| {
        let (_, it) = seq.find_item(*c)?;
        let item = app.session.project.item(it.item)?;
        if !item.has_video() || matches!(item.kind, filmcraft_project::ItemKind::Graphic { .. }) {
            return None;
        }
        let size = filmcraft_render::source_size(&app.session.project, it.item)?;
        Some((*c, it.name.clone(), size))
    })
}

/// Whether the box still exists, and how many tracked positions it has.
fn target_state(app: &FilmcraftApp, (clip, effect, mask): (u64, usize, usize)) -> Option<usize> {
    let seq = app.session.active_sequence()?;
    let (_, it) = seq.find_item(filmcraft_project::ClipId(clip))?;
    Some(it.effects.get(effect)?.masks.get(mask)?.path.keyframes.len())
}

pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    // forget a box that undo or a deletion took away
    if let Some(tg) = app.ui.kurgu.track.target
        && target_state(app, tg).is_none()
    {
        app.ui.kurgu.track = TrackPanel { action: app.ui.kurgu.track.action, ..Default::default() };
    }
    let running = app
        .ui
        .kurgu
        .track
        .job
        .and_then(|id| app.session.jobs.iter().find(|j| j.id == id))
        .filter(|j| !j.progress.finished.load(std::sync::atomic::Ordering::Relaxed));
    let progress = running.map(|j| {
        use std::sync::atomic::Ordering;
        let total = j.progress.total.load(Ordering::Relaxed).max(1);
        (j.progress.done.load(Ordering::Relaxed) as f32 / total as f32, j.progress.status.lock().map(|s| s.clone()).unwrap_or_default())
    });
    if running.is_some() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    }
    let inner = rect.shrink2(vec2(12.0, 8.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).id_salt("kurgu-tracking"));
    egui::ScrollArea::vertical().id_salt("kurgu-tracking-scroll").auto_shrink([false, false]).show(&mut child, |ui| {
        ui.set_width(inner.width());
        ui.spacing_mut().item_spacing.y = 8.0;
        ui.label(egui::RichText::new(tl!("Object Tracking")).size(15.0).strong().color(t.text));
        ui.label(egui::RichText::new(tl!("Mark something in the picture and the effect or title follows it as it moves.")).size(11.5).color(t.text_dim));

        // 1. the clip
        step(ui, &t, "1", tl!("Select the video clip in the timeline"));
        let video = selected_video(app);
        match &video {
            Some((_, name, _)) => ui.label(egui::RichText::new(format!("✔ {name}")).size(12.0).color(t.accent)),
            None => ui.label(egui::RichText::new(tl!("No video clip selected")).size(12.0).color(t.text_dim)),
        };

        // 2. what follows the object
        step(ui, &t, "2", tl!("Choose what follows the object"));
        let busy = app.ui.kurgu.track.target.is_some();
        if busy {
            // chosen: one line, so the next steps stay in view
            let a = Action::from_u8(app.ui.kurgu.track.action);
            ui.label(egui::RichText::new(format!("✔ {}", a.label())).size(12.0).color(t.accent));
        }
        let cols = if busy { 0 } else { 2usize };
        let w = (ui.available_width() - 8.0) / cols.max(1) as f32;
        for row in ACTIONS.chunks(cols.max(1)).take(if busy { 0 } else { usize::MAX }) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for a in row {
                    let (r, resp) = ui.allocate_exact_size(vec2(w, 54.0), if busy { Sense::hover() } else { Sense::click() });
                    let on = Action::from_u8(app.ui.kurgu.track.action) == *a;
                    let fill = if on {
                        t.accent
                    } else if resp.hovered() && !busy {
                        t.hover
                    } else {
                        t.field_bg
                    };
                    ui.painter().rect_filled(r, CornerRadius::same(8), fill);
                    let color = if on {
                        Color32::WHITE
                    } else if busy {
                        t.text_dim
                    } else {
                        t.text
                    };
                    icons::paint(ui.painter(), Rect::from_center_size(pos2(r.center().x, r.min.y + 18.0), vec2(18.0, 18.0)), a.icon(), color);
                    ui.painter().text(pos2(r.center().x, r.max.y - 13.0), Align2::CENTER_CENTER, a.label(), Tokens::ui(11.5), color);
                    app.auto.add(&format!("kurgu.tracking.action.{}", a.index()), r, a.label());
                    if resp.clicked() {
                        app.ui.kurgu.track.action = a.index();
                    }
                }
            });
        }

        // 3. mark the object
        step(ui, &t, "3", tl!("Mark the object"));
        match app.ui.kurgu.track.target {
            None => {
                let ok = video.is_some();
                if big_button(app, ui, &t, tl!("Mark Object"), "kurgu.tracking.mark", ok, true).clicked()
                    && let Some((clip, _, size)) = video.clone()
                {
                    mark(app, clip, size);
                }
                if !ok {
                    hint(ui, &t, tl!("First select a video clip in the timeline."));
                }
            }
            Some(_) => {
                hint(
                    ui,
                    &t,
                    tl!("Drag the box onto the object in the preview; drag its corners to fit it. Move the playhead to where the object is clearly seen."),
                );
                if small_button(ui, &t, tl!("Start Over"), "kurgu.tracking.reset").clicked() {
                    // the box stays on the clip (undo removes it); the panel starts a new one
                    app.ui.kurgu.track = TrackPanel { action: app.ui.kurgu.track.action, ..Default::default() };
                }
            }
        }

        // 4. track
        step(ui, &t, "4", tl!("Track"));
        let target = app.ui.kurgu.track.target;
        match &progress {
            Some((p, status)) => {
                ui.add(egui::ProgressBar::new(*p).show_percentage());
                if !status.is_empty() {
                    hint(ui, &t, status);
                }
                if small_button(ui, &t, tl!("Stop"), "kurgu.tracking.stop").clicked()
                    && let Some(j) = app.ui.kurgu.track.job.and_then(|id| app.session.jobs.iter().find(|j| j.id == id))
                {
                    j.progress.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            None => {
                ui.horizontal(|ui| {
                    if big_button(app, ui, &t, tl!("Track Forward"), "kurgu.tracking.forward", target.is_some(), true).clicked() {
                        track(app, "forward");
                    }
                    if big_button(app, ui, &t, tl!("Backward"), "kurgu.tracking.backward", target.is_some(), false).clicked() {
                        track(app, "backward");
                    }
                });
                if let Some(n) = target.and_then(|tg| target_state(app, tg))
                    && n > 0
                {
                    ui.label(egui::RichText::new(tlf!("✔ Tracked over {n} frames", n = n)).size(12.0).color(t.accent));
                }
            }
        }

        // 5. attach the title or arrow
        let action = Action::from_u8(app.ui.kurgu.track.action);
        if action.follower().is_some() {
            step(ui, &t, "5", tl!("Attach"));
            let tracked = target.and_then(|tg| target_state(app, tg)).unwrap_or(0) > 0 && progress.is_none();
            let label = if app.ui.kurgu.track.follower.is_some() { tl!("Attach Again") } else { tl!("Attach to the Object") };
            if big_button(app, ui, &t, label, "kurgu.tracking.attach", tracked, true).clicked() {
                attach(app, action);
            }
            hint(
                ui,
                &t,
                if app.ui.kurgu.track.follower.is_some() {
                    tl!("Select the title in the timeline to change its text and colors on the right.")
                } else {
                    tl!("Track first; then the title is placed on its own layer and follows the object.")
                },
            );
        }
    });
}

fn step(ui: &mut egui::Ui, t: &Tokens, n: &str, label: &str) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
        ui.painter().circle_filled(r.center(), 10.0, t.accent);
        ui.painter().text(r.center(), Align2::CENTER_CENTER, n, Tokens::ui(11.0), Color32::WHITE);
        ui.label(egui::RichText::new(label).size(12.5).strong().color(t.text));
    });
}

fn hint(ui: &mut egui::Ui, t: &Tokens, s: &str) {
    ui.label(egui::RichText::new(s).size(11.0).color(t.text_dim));
}

fn big_button(app: &mut FilmcraftApp, ui: &mut egui::Ui, t: &Tokens, label: &str, auto: &str, enabled: bool, primary: bool) -> egui::Response {
    let font = Tokens::ui(12.5);
    let w = ui.fonts_mut(|f| f.layout_no_wrap(label.to_string(), font.clone(), t.text).size().x) + 28.0;
    let (r, resp) = ui.allocate_exact_size(vec2(w, 30.0), if enabled { Sense::click() } else { Sense::hover() });
    let resp = resp.on_hover_cursor(if enabled { egui::CursorIcon::PointingHand } else { egui::CursorIcon::Default });
    let (fill, text) = match (enabled, primary) {
        (false, _) => (t.field_bg, t.text_dim),
        (true, true) => (if resp.hovered() { t.accent_hover } else { t.accent }, Color32::WHITE),
        (true, false) => (if resp.hovered() { t.hover } else { t.field_bg }, t.text),
    };
    ui.painter().rect_filled(r, CornerRadius::same(8), fill);
    if !primary || !enabled {
        ui.painter().rect_stroke(r, CornerRadius::same(8), Stroke::new(1.0, t.separator), StrokeKind::Inside);
    }
    ui.painter().text(r.center(), Align2::CENTER_CENTER, label, font, text);
    app.auto.add(auto, r, label);
    resp
}

fn small_button(ui: &mut egui::Ui, t: &Tokens, label: &str, _auto: &str) -> egui::Response {
    ui.add(egui::Button::new(egui::RichText::new(label).size(11.5).color(t.text)).corner_radius(6))
}

/// Puts the box on the clip: the action's effect with a rectangle mask in the middle of the
/// picture, selected so it can be moved and sized on the preview.
fn mark(app: &mut FilmcraftApp, clip: filmcraft_project::ClipId, size: (u32, u32)) {
    let action = Action::from_u8(app.ui.kurgu.track.action);
    let (effect, params) = action.effect();
    app.session.state.selection = vec![clip];
    let r = (|| -> Result<(usize, usize), String> {
        app.session.execute("effects.apply", json!({"effect": effect})).map_err(|e| e.to_string())?;
        let ei = app
            .session
            .active_sequence()
            .and_then(|q| q.find_item(clip))
            .map(|(_, it)| it.effects.len().saturating_sub(1))
            .ok_or_else(|| tl!("Select a clip in the timeline first").to_string())?;
        if let Some(obj) = params.as_object() {
            for (k, v) in obj {
                app.session.execute("effects.setParam", json!({"clip": clip.0, "effect": ei, "param": k, "value": v})).map_err(|e| e.to_string())?;
            }
        }
        let (w, h) = (f64::from(size.0), f64::from(size.1));
        let m = app
            .session
            .execute("masks.add", json!({"clip": clip.0, "effect": ei, "shape": "rectangle", "center": [w / 2.0, h / 2.0], "size": [w * 0.22, h * 0.22]}))
            .map_err(|e| e.to_string())?;
        let mi = m["mask"].as_u64().map_or(0, |v| v as usize);
        app.session.execute("masks.set", json!({"clip": clip.0, "effect": ei, "mask": mi, "feather": 4.0})).map_err(|e| e.to_string())?;
        Ok((ei, mi))
    })();
    match r {
        Ok((ei, mi)) => {
            app.ui.kurgu.track.target = Some((clip.0, ei, mi));
            app.ui.kurgu.track.follower = None;
        }
        Err(e) => app.ui.status = e,
    }
}

fn track(app: &mut FilmcraftApp, direction: &str) {
    let Some((clip, effect, mask)) = app.ui.kurgu.track.target else { return };
    match app.session.execute("masks.track", json!({"clip": clip, "effect": effect, "mask": mask, "direction": direction})) {
        Ok(r) => app.ui.kurgu.track.job = r["job"].as_u64(),
        Err(e) => app.ui.status = e.to_string(),
    }
}

/// Places the action's title at the start of the tracked span, on its own layer, and makes it
/// follow the box.
fn attach(app: &mut FilmcraftApp, action: Action) {
    let (Some((clip, effect, mask)), Some((template, place))) = (app.ui.kurgu.track.target, action.follower()) else { return };
    // the earliest tracked moment, on the timeline
    let start = app.session.active_sequence().and_then(|q| {
        let (_, it) = q.find_item(filmcraft_project::ClipId(clip))?;
        let first = it.effects.get(effect)?.masks.get(mask)?.path.keyframes.iter().map(|k| k.time).min()?;
        let speed = it.speed.abs().max(1e-9);
        Some(filmcraft_time::Tick(it.start.0.saturating_add(((first.0 - it.source_in.0) as f64 / speed).round() as i64)).max(it.start))
    });
    let Some(start) = start else { return };
    // a title placed before replaces nothing: it stays, and a new one is attached
    let r = (|| -> Result<u64, String> {
        let placed = app.session.execute("graphics.template.apply", json!({"template": template, "time": start.0})).map_err(|e| e.to_string())?;
        let title = placed["clip"].as_u64().ok_or_else(|| "no title placed".to_string())?;
        app.session
            .execute("masks.follow", json!({"clip": clip, "effect": effect, "mask": mask, "target": title, "place": place}))
            .map_err(|e| e.to_string())?;
        Ok(title)
    })();
    match r {
        Ok(title) => app.ui.kurgu.track.follower = Some(title),
        Err(e) => app.ui.status = e,
    }
}
