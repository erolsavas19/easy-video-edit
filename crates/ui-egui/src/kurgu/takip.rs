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
    Replace,
    /// Cuts the object out for the body effects (and, when it is ready, for Replace Background).
    Background,
}

const ACTIONS: [Action; 7] = [Action::Blur, Action::Mosaic, Action::Replace, Action::Cover, Action::TextAbove, Action::Arrow, Action::Background];

/// The Action index of the cut-out and its body effects (the Tools card opens the panel on it).
pub const BACKGROUND_ACTION: u8 = 6;

/// Replace Background is shown as coming soon: the cut-out does not yet hold a moving object well
/// enough to put another background behind it (the gaps between legs fill in, and it can jump to
/// what is behind the object). The body effects, which forgive a rough edge, keep using it.
pub const BACKGROUND_SOON: bool = true;

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
            Action::Arrow => tl!("Arrow · Sticker"),
            Action::Replace => tl!("Replace with Media"),
            Action::Background if BACKGROUND_SOON => tl!("Body Effects"),
            Action::Background => tl!("Replace Background"),
        }
    }
    fn icon(self) -> Icon {
        match self {
            Action::Blur => Icon::Sparkle,
            Action::Mosaic => Icon::Grid,
            Action::Cover | Action::TextAbove => Icon::Type,
            Action::Arrow => Icon::TrackMaskFwd,
            Action::Replace => Icon::Image,
            Action::Background if BACKGROUND_SOON => Icon::Sun,
            Action::Background => Icon::Image,
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
    /// Replace Background: the cut-out job, the Cutout mask (clip, Opacity index, mask index),
    /// how often the model runs (0 fast, 1 balanced, 2 best) and the background placed under it.
    pub cut_job: Option<u64>,
    pub cut_mask: Option<(u64, usize, usize)>,
    pub quality: u8,
    pub background: Option<u64>,
    /// `segment.model`: (available in this build, installed), read once.
    pub model: Option<(bool, bool)>,
    /// The colour chosen for Glow and Neon Edge (index into `BODY_COLORS`).
    pub body_color: u8,
    /// What Arrow · Sticker attaches (index into `STICKERS`; the last is the user's own picture).
    pub sticker: u8,
    /// The panel shows the teleport effect instead of the tracking steps.
    pub teleport: bool,
    /// Teleport: the frame with the place empty (clip, media ticks).
    pub plate: Option<(u64, i64)>,
    /// Teleport: how long the light lasts, in seconds (0 = the default).
    pub tp_secs: f32,
    /// Replace with Media: the whole picture inside the box (else it covers the box), its size in
    /// percent (0 = 100), and whether what is behind it is blurred.
    pub inside: bool,
    pub media_size: f32,
    pub hide_behind: bool,
}

/// Arrow · Sticker: the follow templates to choose from (id after `TRACK_PREFIX`, name), then the
/// user's own picture (the one selected in Media).
const STICKERS: [(&str, &str); 6] = [("ok", "Arrow"), ("yildiz", "Star"), ("kalp", "Heart"), ("nota", "Note"), ("parilti", "Sparkle"), ("", "My Picture")];

/// My Picture is placed this wide, as a share of the sequence's width.
const PICTURE_WIDTH: f64 = 0.12;

/// Share of the box (round its middle) the tracker looks in.
const TRACK_CORE: f64 = 0.8;

/// Frames between the model's results for each quality.
const QUALITY_STEP: [u64; 3] = [10, 5, 2];

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
    if app.ui.kurgu.track.teleport {
        teleport_panel(app, ui, rect);
        return;
    }
    // forget a box that undo or a deletion took away
    if let Some(tg) = app.ui.kurgu.track.target
        && target_state(app, tg).is_none()
    {
        app.ui.kurgu.track = TrackPanel { action: app.ui.kurgu.track.action, ..Default::default() };
    }
    // another clip selected: start fresh there (the earlier box and its effect stay)
    if let Some((clip, _, _)) = app.ui.kurgu.track.target
        && let Some((sel, _, _)) = selected_video(app)
        && sel.0 != clip
        && !job_running(app, app.ui.kurgu.track.job)
        && !job_running(app, app.ui.kurgu.track.cut_job)
    {
        app.ui.kurgu.track = fresh(&app.ui.kurgu.track);
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
    let cutting = app
        .ui
        .kurgu
        .track
        .cut_job
        .and_then(|id| app.session.jobs.iter().find(|j| j.id == id))
        .filter(|j| !j.progress.finished.load(std::sync::atomic::Ordering::Relaxed));
    let cut_progress = cutting.map(|j| {
        use std::sync::atomic::Ordering;
        let total = j.progress.total.load(Ordering::Relaxed).max(1);
        (j.progress.done.load(Ordering::Relaxed) as f32 / total as f32, j.progress.status.lock().map(|s| s.clone()).unwrap_or_default())
    });
    // the result of a finished cut-out (an error stays in the panel)
    let cut_error =
        app.ui.kurgu.track.cut_job.and_then(|id| app.session.jobs.iter().find(|j| j.id == id)).and_then(|j| match j.result.lock().map(|r| r.clone()) {
            Ok(Some(Err(e))) => Some(e),
            _ => None,
        });
    if running.is_some() || cutting.is_some() {
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
                if BACKGROUND_SOON && row.len() < cols {
                    soon_tile(app, ui, &t, w);
                }
            });
        }

        // the objects already tracked on this clip
        if let Some((clip, _, _)) = &video {
            tracked_list(app, ui, &t, clip.0);
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
                ui.horizontal(|ui| {
                    if small_button(ui, &t, tl!("Mark Another Object"), "kurgu.tracking.reset")
                        .on_hover_text(tl!("This object keeps its tracking; mark the next one (another face, another car)"))
                        .clicked()
                    {
                        app.ui.kurgu.track = fresh(&app.ui.kurgu.track);
                    }
                    if small_button(ui, &t, tl!("Remove Tracking"), "kurgu.tracking.remove")
                        .on_hover_text(tl!("Removes the box, its effect and the attached title"))
                        .clicked()
                    {
                        remove(app);
                    }
                });
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
                if target.is_some() && job_error(app, app.ui.kurgu.track.job).is_some() {
                    ui.label(egui::RichText::new(tl!("The object was lost (hidden, too small or blurred).")).size(11.5).color(t.danger));
                }
                if target.is_some_and(|tg| target_state(app, tg).unwrap_or(0) > 1) {
                    if small_button(ui, &t, tl!("Go to the Last Tracked Frame"), "kurgu.tracking.last").clicked() {
                        go_to_last_key(app);
                    }
                    hint(
                        ui,
                        &t,
                        tl!("If the box slips or stops (behind a tree, say): move the playhead to where the object is seen again, drag the box back onto it and press Track Forward. Tracking goes on from there."),
                    );
                }
            }
        }

        // 5. Body Effects / Replace Background: cut the object out, then what goes round or under it
        let action = Action::from_u8(app.ui.kurgu.track.action);
        if action == Action::Background {
            background_steps(app, ui, &t, target, progress.is_some(), cut_progress, cut_error);
        }

        // 5. Replace with Media: a picture or video takes the object's place
        if action == Action::Replace {
            replace_steps(app, ui, &t, target, progress.is_some());
        }

        // 5. attach the title or arrow
        if action.follower().is_some() {
            step(ui, &t, "5", tl!("Attach"));
            if action == Action::Arrow {
                ui.horizontal_wrapped(|ui| {
                    for (i, (_, name)) in STICKERS.iter().enumerate() {
                        let i = u8::try_from(i).unwrap_or(0);
                        let on = app.ui.kurgu.track.sticker == i;
                        let r = ui.selectable_label(on, egui::RichText::new(crate::i18n::t(name)).size(11.5));
                        app.auto.add(&format!("kurgu.tracking.sticker.{i}"), r.rect, crate::i18n::t(name));
                        if r.clicked() {
                            app.ui.kurgu.track.sticker = i;
                        }
                    }
                });
            }
            let tracked = target.and_then(|tg| target_state(app, tg)).unwrap_or(0) > 0 && progress.is_none();
            let own_picture = action == Action::Arrow && usize::from(app.ui.kurgu.track.sticker) == STICKERS.len() - 1;
            if own_picture {
                // the pictures in Media, each attached with one click
                let mut items: Vec<(filmcraft_project::ItemId, String)> = app
                    .session
                    .project
                    .items
                    .values()
                    .filter(|i| matches!(&i.kind, filmcraft_project::ItemKind::Media(m) if m.info.kind == filmcraft_media::MediaKind::Still))
                    .map(|i| (i.id, i.name.clone()))
                    .collect();
                items.sort_by(|a, b| a.1.cmp(&b.1));
                if items.is_empty() {
                    hint(ui, &t, tl!("Import a picture in Media first (a PNG with a transparent background looks best)."));
                }
                for (k, (id, name)) in items.into_iter().enumerate() {
                    ui.horizontal(|ui| {
                        let r = ui.add_enabled(tracked, egui::Button::new(egui::RichText::new(tl!("Attach")).size(11.5)).corner_radius(6));
                        app.auto.add(&format!("kurgu.tracking.picture.{k}"), r.rect, tl!("Attach"));
                        ui.label(egui::RichText::new(name).size(12.0).color(t.text));
                        if r.clicked() {
                            attach(app, action, Some(id));
                        }
                    });
                }
            } else {
                let label = if app.ui.kurgu.track.follower.is_some() { tl!("Attach Again") } else { tl!("Attach to the Object") };
                if big_button(app, ui, &t, label, "kurgu.tracking.attach", tracked, true).clicked() {
                    attach(app, action, None);
                }
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

/// Steps 5 and 6 of Replace Background.
fn background_steps(
    app: &mut FilmcraftApp,
    ui: &mut egui::Ui,
    t: &Tokens,
    target: Option<(u64, usize, usize)>,
    tracking: bool,
    cut_progress: Option<(f32, String)>,
    cut_error: Option<String>,
) {
    step(ui, t, "5", tl!("Cut out the object"));
    let (available, installed) = *app.ui.kurgu.track.model.get_or_insert_with(|| {
        let v = app.session.execute("segment.model", json!({})).unwrap_or_default();
        (v["available"].as_bool().unwrap_or(false), v["installed"].as_bool().unwrap_or(false))
    });
    if !available {
        hint(ui, t, tl!("This build has no background removal."));
        return;
    }
    match &cut_progress {
        Some((p, status)) => {
            ui.add(egui::ProgressBar::new(*p).show_percentage());
            if !status.is_empty() {
                hint(ui, t, status);
            }
            if small_button(ui, t, tl!("Stop"), "kurgu.cutout.stop").clicked()
                && let Some(j) = app.ui.kurgu.track.cut_job.and_then(|id| app.session.jobs.iter().find(|j| j.id == id))
            {
                j.progress.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
        None => {
            ui.horizontal(|ui| {
                for (i, label) in [tl!("Fast"), tl!("Balanced"), tl!("Best")].into_iter().enumerate() {
                    let q = u8::try_from(i).unwrap_or(0);
                    if ui.selectable_label(app.ui.kurgu.track.quality == q, egui::RichText::new(label).size(11.5)).clicked() {
                        app.ui.kurgu.track.quality = q;
                    }
                }
            });
            hint(ui, t, tl!("Fast is quicker; Best follows changes of shape more closely. Track a moving object first."));
            let m = filmcraft_engine::segment::MODEL;
            if !installed {
                let mb = format!("{:.1}", m.file.size as f64 / 1_000_000.0);
                hint(
                    ui,
                    t,
                    &tlf!(
                        "The AI model {name} ({mb} MB, {license}) is downloaded once from {source}. Your video stays on this computer.",
                        name = m.name,
                        mb = mb,
                        license = m.license,
                        source = m.source
                    ),
                );
            }
            let label = if installed { tl!("Cut Out") } else { tl!("Download Model and Cut Out") };
            let ok = target.is_some() && !tracking;
            if big_button(app, ui, t, label, "kurgu.cutout.start", ok, true).clicked() {
                cut(app, !installed);
            }
            if let Some(e) = cut_error {
                ui.label(egui::RichText::new(e).size(11.0).color(t.danger));
            } else if app.ui.kurgu.track.cut_mask.is_some() && app.ui.kurgu.track.cut_job.is_some() {
                ui.label(egui::RichText::new(tl!("✔ Background removed")).size(12.0).color(t.accent));
            }
        }
    }

    if BACKGROUND_SOON {
        body_steps(app, ui, t, cut_progress.is_some());
        return;
    }
    // 6. the new background
    step(ui, t, "6", tl!("Choose the new background"));
    let done = app.ui.kurgu.track.cut_mask.is_some() && cut_progress.is_none();
    let own = target.and_then(|(clip, _, _)| app.session.active_sequence()?.find_item(filmcraft_project::ClipId(clip)).map(|(_, it)| it.item));
    let mut items: Vec<(filmcraft_project::ItemId, String)> = app
        .session
        .project
        .items
        .values()
        .filter(|i| i.has_video() && Some(i.id) != own && matches!(i.kind, filmcraft_project::ItemKind::Media(_)))
        .map(|i| (i.id, i.name.clone()))
        .collect();
    items.sort_by(|a, b| a.1.cmp(&b.1));
    if items.is_empty() {
        hint(ui, t, tl!("Import a picture or video in Media to use it as the background."));
    }
    for (id, name) in items {
        ui.horizontal(|ui| {
            let r = ui.add_enabled(done, egui::Button::new(egui::RichText::new(tl!("Use")).size(11.5)).corner_radius(6));
            ui.label(egui::RichText::new(name).size(12.0).color(t.text));
            if r.clicked() {
                place_background(app, id);
            }
        });
    }
    if app.ui.kurgu.track.background.is_some() {
        ui.label(egui::RichText::new(tl!("✔ The background is on the layer under the video")).size(12.0).color(t.accent));
    }
    body_steps(app, ui, t, cut_progress.is_some());
}

/// Starts the cut-out of the box (downloading the model first when asked).
fn cut(app: &mut FilmcraftApp, download: bool) {
    let Some((clip, effect, mask)) = app.ui.kurgu.track.target else { return };
    let step = QUALITY_STEP.get(usize::from(app.ui.kurgu.track.quality)).copied().unwrap_or(5);
    // a new cut-out replaces the previous one
    remove_cut(app);
    match app.session.execute("masks.cutout", json!({"clip": clip, "effect": effect, "mask": mask, "step": step, "download": download})) {
        Ok(r) => {
            app.ui.kurgu.track.cut_job = r["job"].as_u64();
            let (oi, mo) = (r["effect"].as_u64().unwrap_or(0) as usize, r["mask"].as_u64().unwrap_or(0) as usize);
            app.ui.kurgu.track.cut_mask = Some((clip, oi, mo));
            // installed now (or being downloaded): read again
            app.ui.kurgu.track.model = None;
        }
        Err(e) => app.ui.status = e.to_string(),
    }
}

/// Takes the Cutout mask off the clip (and stops its job).
fn remove_cut(app: &mut FilmcraftApp) {
    if let Some(j) = app.ui.kurgu.track.cut_job.and_then(|id| app.session.jobs.iter().find(|j| j.id == id)) {
        j.progress.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    if let Some((clip, effect, mask)) = app.ui.kurgu.track.cut_mask.take() {
        let exists = app
            .session
            .active_sequence()
            .and_then(|q| q.find_item(filmcraft_project::ClipId(clip)))
            .is_some_and(|(_, it)| it.effects.get(effect).is_some_and(|e| mask < e.masks.len()));
        if exists && let Err(e) = app.session.execute("masks.remove", json!({"clip": clip, "effect": effect, "mask": mask})) {
            app.ui.status = e.to_string();
        }
    }
    app.ui.kurgu.track.cut_job = None;
}

/// Puts `item` on a new bottom video layer under the clip, as long as the clip.
fn place_background(app: &mut FilmcraftApp, item: filmcraft_project::ItemId) {
    let Some((clip, _, _)) = app.ui.kurgu.track.target else { return };
    let Some((start, dur)) = app.session.active_sequence().and_then(|q| q.find_item(filmcraft_project::ClipId(clip))).map(|(_, it)| (it.start, it.duration))
    else {
        return;
    };
    // a background placed before goes: the new one takes its place
    if let Some(old) = app.ui.kurgu.track.background.take()
        && app.session.active_sequence().is_some_and(|q| q.find_item(filmcraft_project::ClipId(old)).is_some())
    {
        let _ = app.session.execute("edit.clear", json!({"clip": old}));
    }
    let r = (|| -> Result<u64, String> {
        app.session.execute("sequence.addTracks", json!({"video": 1, "audio": 0, "videoAfter": "first"})).map_err(|e| e.to_string())?;
        let placed = app
            .session
            .execute("timeline.place", json!({"item": item.0, "time": start.0, "track": "V1", "duration": dur.0, "audio": false, "insert": false}))
            .map_err(|e| e.to_string())?;
        Ok(placed["clip"].as_u64().or_else(|| placed["clips"].get(0).and_then(serde_json::Value::as_u64)).unwrap_or(0))
    })();
    match r {
        Ok(id) => {
            app.ui.kurgu.track.background = Some(id);
            // the cut-out clip stays selected, so the panel keeps its place
            app.session.state.selection = vec![filmcraft_project::ClipId(clip)];
        }
        Err(e) => app.ui.status = e,
    }
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
        // place and size, no turning: the box grows as the object comes closer and shrinks as it
        // goes away, and a turning box drifts off a low-detail object
        app.session
            .execute("masks.set", json!({"clip": clip.0, "effect": ei, "mask": mi, "feather": 4.0, "trackMethod": "positionScale"}))
            .map_err(|e| e.to_string())?;
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

/// Takes the tracking away: the attached title, then the box's effect (with the box), and stops a
/// running track first.
fn remove(app: &mut FilmcraftApp) {
    remove_cut(app);
    let st = app.ui.kurgu.track.clone();
    if let Some(bg) = st.background
        && app.session.active_sequence().is_some_and(|q| q.find_item(filmcraft_project::ClipId(bg)).is_some())
    {
        let _ = app.session.execute("edit.clear", json!({"clip": bg}));
    }
    if let Some(j) = st.job.and_then(|id| app.session.jobs.iter().find(|j| j.id == id)) {
        j.progress.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    let mut err = None;
    if let Some(title) = st.follower
        && app.session.active_sequence().is_some_and(|q| q.find_item(filmcraft_project::ClipId(title)).is_some())
        && let Err(e) = app.session.execute("edit.clear", json!({"clip": title}))
    {
        err = Some(e.to_string());
    }
    if let Some((clip, effect, _)) = st.target
        && let Err(e) = app.session.execute("effects.remove", json!({"clip": clip, "index": effect}))
    {
        err = Some(e.to_string());
    }
    if let Some(e) = err {
        app.ui.status = e;
    }
    app.ui.kurgu.track = fresh(&st);
}

fn track(app: &mut FilmcraftApp, direction: &str) {
    let Some((clip, effect, mask)) = app.ui.kurgu.track.target else { return };
    // by look: follows plain, low-detail objects too (a black dog) and resizes the box with them
    match app.session.execute("masks.track", json!({"clip": clip, "effect": effect, "mask": mask, "direction": direction, "core": TRACK_CORE, "by": "look"})) {
        Ok(r) => app.ui.kurgu.track.job = r["job"].as_u64(),
        Err(e) => app.ui.status = e.to_string(),
    }
}

/// Places the action's title at the start of the tracked span, on its own layer, and makes it
/// follow the box.
fn attach(app: &mut FilmcraftApp, action: Action, picture: Option<filmcraft_project::ItemId>) {
    let (Some((clip, effect, mask)), Some((template, place))) = (app.ui.kurgu.track.target, action.follower()) else { return };
    // the earliest tracked moment, on the timeline
    let start = app.session.active_sequence().and_then(|q| {
        let (_, it) = q.find_item(filmcraft_project::ClipId(clip))?;
        let first = it.effects.get(effect)?.masks.get(mask)?.path.keyframes.iter().map(|k| k.time).min()?;
        let speed = it.speed.abs().max(1e-9);
        Some(filmcraft_time::Tick(it.start.0.saturating_add(((first.0 - it.source_in.0) as f64 / speed).round() as i64)).max(it.start))
    });
    let Some(start) = start else { return };
    // Arrow · Sticker: the chosen sticker, or the picture selected in Media
    let sticker = (action == Action::Arrow).then(|| STICKERS.get(usize::from(app.ui.kurgu.track.sticker)).map_or("ok", |s| s.0));
    let template = match sticker {
        Some(id) if !id.is_empty() => format!("{}{id}", filmcraft_project::kurgu_templates::TRACK_PREFIX),
        _ => template.to_string(),
    };
    // a title placed before replaces nothing: it stays, and a new one is attached
    let r = (|| -> Result<u64, String> {
        let title = match picture {
            Some(item) => place_picture(app, item, start)?,
            None => {
                let placed = app.session.execute("graphics.template.apply", json!({"template": template, "time": start.0})).map_err(|e| e.to_string())?;
                placed["clip"].as_u64().ok_or_else(|| "no title placed".to_string())?
            }
        };
        app.session
            .execute("masks.follow", json!({"clip": clip, "effect": effect, "mask": mask, "target": title, "place": place}))
            .map_err(|e| e.to_string())?;
        Ok(title)
    })();
    match r {
        Ok(title) => {
            app.ui.kurgu.track.follower = Some(title);
            // the tracked clip stays selected, so the panel keeps its place (another sticker can
            // be attached at once)
            app.session.state.selection = vec![filmcraft_project::ClipId(clip)];
        }
        Err(e) => app.ui.status = e,
    }
}

/// A body effect card: what it is and what it does.
struct Body {
    key: &'static str,
    label: &'static str,
    icon: Icon,
    /// `masks.bodyEffect` effect, or an orbiting sticker template.
    effect: Option<&'static str>,
    sticker: Option<&'static str>,
    tip: &'static str,
}

const BODIES: [Body; 8] = [
    Body { key: "glow", label: "Glow", icon: Icon::Sun, effect: Some("glow"), sticker: None, tip: "A soft coloured light round the person, beating slowly" },
    Body { key: "neon", label: "Neon Edge", icon: Icon::Sparkle, effect: Some("neon"), sticker: None, tip: "A bright coloured edge round the person" },
    Body { key: "portrait", label: "Portrait", icon: Icon::Camera, effect: Some("portrait"), sticker: None, tip: "The background blurred, the person sharp" },
    Body {
        key: "colorPop",
        label: "Color Pop",
        icon: Icon::Eyedropper,
        effect: Some("colorPop"),
        sticker: None,
        tip: "The background in black and white, the person in colour",
    },
    Body { key: "spotlight", label: "Spotlight", icon: Icon::Moon, effect: Some("spotlight"), sticker: None, tip: "The background darkened, the person lit" },
    Body { key: "stars", label: "Orbiting Stars", icon: Icon::Star, effect: None, sticker: Some("easyvideoedit:takip-yildiz"), tip: "Stars circling the head" },
    Body {
        key: "hearts",
        label: "Orbiting Hearts",
        icon: Icon::Star,
        effect: None,
        sticker: Some("easyvideoedit:takip-kalp"),
        tip: "Hearts circling the head",
    },
    Body { key: "notes", label: "Music Notes", icon: Icon::Audio, effect: None, sticker: Some("easyvideoedit:takip-nota"), tip: "Notes circling the head" },
];

/// Colours offered for Glow and Neon Edge.
const BODY_COLORS: [(&str, Color32); 5] = [
    ("#ffd75a", Color32::from_rgb(0xff, 0xd7, 0x5a)),
    ("#22d3ee", Color32::from_rgb(0x22, 0xd3, 0xee)),
    ("#f472b6", Color32::from_rgb(0xf4, 0x72, 0xb6)),
    ("#a3e635", Color32::from_rgb(0xa3, 0xe6, 0x35)),
    ("#ffffff", Color32::WHITE),
];

/// The selected video clip's Cutout mask: (clip, Opacity index, mask index).
fn cut_out_clip(app: &FilmcraftApp) -> Option<(u64, usize, usize)> {
    let seq = app.session.active_sequence()?;
    let c = app.ui.kurgu.track.target.map(|t| filmcraft_project::ClipId(t.0)).or_else(|| app.session.state.selection.first().copied())?;
    let (_, it) = seq.find_item(c)?;
    let oi = it.effects.iter().position(|e| e.effect == "opacity")?;
    let mi = it.effects.get(oi)?.masks.iter().rposition(|m| m.name == "Cutout")?;
    // only once the cut-out has its shapes (not while the model is still at work)
    (it.effects.get(oi)?.masks.get(mi)?.path.keyframes.len() > 1).then_some((c.0, oi, mi))
}

/// Whether `clip` is cut out (Replace Background) with something on a lower layer under it: the
/// teleport then needs no empty frame (the engine's own rule, `effects.teleport`).
fn over_new_background(app: &FilmcraftApp, clip: u64) -> bool {
    let Some(seq) = app.session.active_sequence() else { return false };
    let Some((track, it)) = seq.find_item(filmcraft_project::ClipId(clip)) else { return false };
    let cut = it.effects.iter().any(|e| e.effect == "opacity" && e.masks.iter().any(|m| m.name == "Cutout"));
    let Some(ti) = seq.video_tracks.iter().position(|t| t.id == track) else { return false };
    cut && seq.video_tracks.iter().take(ti).any(|t| t.items.iter().any(|o| o.start < it.end() && it.start < o.end()))
}

/// Step 7: body effects on the cut-out.
fn body_steps(app: &mut FilmcraftApp, ui: &mut egui::Ui, t: &Tokens, busy: bool) {
    step(ui, t, if BACKGROUND_SOON { "6" } else { "7" }, tl!("Body effects"));
    let Some((clip, oi, mi)) = cut_out_clip(app) else {
        hint(ui, t, tl!("Cut the person or object out first; then light, edges and stickers can go round it."));
        return;
    };
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tl!("Color")).size(11.5).color(t.text_dim));
        for (i, (_, c)) in BODY_COLORS.iter().enumerate() {
            let (r, resp) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::click());
            ui.painter().circle_filled(r.center(), 8.0, *c);
            if usize::from(app.ui.kurgu.track.body_color) == i {
                ui.painter().circle_stroke(r.center(), 10.0, Stroke::new(2.0, t.accent));
            }
            if resp.clicked() {
                app.ui.kurgu.track.body_color = u8::try_from(i).unwrap_or(0);
            }
        }
    });
    let w = (ui.available_width() - 8.0) / 2.0;
    for row in BODIES.chunks(2) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            for b in row {
                let (r, resp) = ui.allocate_exact_size(vec2(w, 46.0), if busy { Sense::hover() } else { Sense::click() });
                let resp = resp.on_hover_text(crate::i18n::t(b.tip));
                let fill = if resp.hovered() && !busy { t.hover } else { t.field_bg };
                ui.painter().rect_filled(r, CornerRadius::same(8), fill);
                let color = if busy { t.text_dim } else { t.text };
                icons::paint(ui.painter(), Rect::from_center_size(pos2(r.min.x + 20.0, r.center().y), vec2(16.0, 16.0)), b.icon, color);
                ui.painter().text(pos2(r.min.x + 36.0, r.center().y), Align2::LEFT_CENTER, crate::i18n::t(b.label), Tokens::ui(11.5), color);
                app.auto.add(&format!("kurgu.body.{}", b.key), r, b.label);
                if resp.clicked() {
                    apply_body(app, b, clip, oi, mi);
                }
            }
        });
    }
    hint(ui, t, tl!("Each effect goes on its own layer; select it in the timeline to change or delete it."));
    if small_button(ui, t, tl!("Teleport…"), "kurgu.body.teleport")
        .on_hover_text(tl!("The person vanishes or appears on the spot in a burst of light"))
        .clicked()
    {
        app.ui.kurgu.track.teleport = true;
    }
}

fn apply_body(app: &mut FilmcraftApp, b: &Body, clip: u64, oi: usize, mi: usize) {
    let color = BODY_COLORS.get(usize::from(app.ui.kurgu.track.body_color)).map_or("#ffd75a", |c| c.0);
    let r = if let Some(effect) = b.effect {
        app.session.execute("masks.bodyEffect", json!({"clip": clip, "effect": effect, "color": color})).map(|_| ()).map_err(|e| e.to_string())
    } else if let Some(sticker) = b.sticker {
        (|| -> Result<(), String> {
            let start = app.session.active_sequence().and_then(|q| q.find_item(filmcraft_project::ClipId(clip))).map(|(_, it)| it.start).ok_or("no clip")?;
            // three of them, a third of a turn apart
            for phase in [0.0, 120.0, 240.0] {
                let placed = app.session.execute("graphics.template.apply", json!({"template": sticker, "time": start.0})).map_err(|e| e.to_string())?;
                let id = placed["clip"].as_u64().ok_or("no sticker placed")?;
                app.session
                    .execute(
                        "masks.follow",
                        json!({"clip": clip, "effect": oi, "mask": mi, "target": id, "place": "top", "orbit": {"radius": 110.0, "period": 2.0, "phase": phase}}),
                    )
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        })()
    } else {
        Ok(())
    };
    match r {
        Ok(()) => {
            // the cut-out clip stays selected, so the panel keeps its place
            app.session.state.selection = vec![filmcraft_project::ClipId(clip)];
            app.ui.status = tlf!("{effect} added", effect = crate::i18n::t(b.label));
        }
        Err(e) => app.ui.status = e,
    }
}

/// A fresh panel that keeps the user's choices.
fn fresh(st: &TrackPanel) -> TrackPanel {
    TrackPanel {
        action: st.action,
        quality: st.quality,
        body_color: st.body_color,
        model: st.model,
        sticker: st.sticker,
        teleport: st.teleport,
        tp_secs: st.tp_secs,
        inside: st.inside,
        media_size: st.media_size,
        // blurring what is behind belongs to one box
        hide_behind: false,
        ..Default::default()
    }
}

/// Whether job `id` is still running.
fn job_running(app: &FilmcraftApp, id: Option<u64>) -> bool {
    id.and_then(|id| app.session.jobs.iter().find(|j| j.id == id)).is_some_and(|j| !j.progress.finished.load(std::sync::atomic::Ordering::Relaxed))
}

/// The error a finished job ended with.
fn job_error(app: &FilmcraftApp, id: Option<u64>) -> Option<String> {
    let j = id.and_then(|id| app.session.jobs.iter().find(|j| j.id == id))?;
    match j.result.lock().map(|r| r.clone()) {
        Ok(Some(Err(e))) => Some(e),
        _ => None,
    }
}

/// The boxes tracked on `clip`: (effect index, mask index, the action they belong to, tracked frames).
fn tracked_objects(app: &FilmcraftApp, clip: u64) -> Vec<(usize, usize, Option<Action>, usize)> {
    let Some((_, it)) = app.session.active_sequence().and_then(|q| q.find_item(filmcraft_project::ClipId(clip))) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (ei, e) in it.effects.iter().enumerate() {
        let action = match e.effect.as_str() {
            "mosaic" => Some(Action::Mosaic),
            "gaussian_blur" if e.params.get("blurriness").and_then(|p| p.value.as_f64()).unwrap_or(0.0) > 0.0 => Some(Action::Blur),
            // a box that only marks the object (a title, an arrow or a cut-out follows it)
            "gaussian_blur" => None,
            _ => continue,
        };
        for (mi, m) in e.masks.iter().enumerate() {
            let n = m.path.keyframes.len();
            if n > 1 {
                out.push((ei, mi, action, n));
            }
        }
    }
    out
}

/// The tracked objects of the selected clip, each with Select and Remove.
fn tracked_list(app: &mut FilmcraftApp, ui: &mut egui::Ui, t: &Tokens, clip: u64) {
    let list = tracked_objects(app, clip);
    if list.is_empty() || job_running(app, app.ui.kurgu.track.job) {
        return;
    }
    ui.label(egui::RichText::new(tl!("Tracked objects on this clip")).size(12.0).strong().color(t.text));
    let mut remove_at = None;
    for (k, (ei, mi, action, n)) in list.iter().enumerate() {
        let current = app.ui.kurgu.track.target == Some((clip, *ei, *mi));
        ui.horizontal(|ui| {
            let what = action.map_or(tl!("Marked object"), Action::label);
            let text = format!("{}. {} · {}", k + 1, what, tlf!("{n} frames", n = n));
            ui.label(egui::RichText::new(text).size(11.5).color(if current { t.accent } else { t.text }));
            if !current && small_button(ui, t, tl!("Select"), "kurgu.tracking.pick").clicked() {
                let mut st = fresh(&app.ui.kurgu.track);
                if let Some(a) = action {
                    // a blurred box is also how Replace with Media hides what is behind the picture
                    if *a == Action::Blur && Action::from_u8(st.action) == Action::Replace {
                        st.hide_behind = true;
                    } else {
                        st.action = a.index();
                    }
                }
                st.target = Some((clip, *ei, *mi));
                app.ui.kurgu.track = st;
            }
            if small_button(ui, t, tl!("Remove"), "kurgu.tracking.drop").clicked() {
                remove_at = Some(*ei);
            }
        });
    }
    let Some(ei) = remove_at else { return };
    if app.ui.kurgu.track.target.is_some_and(|(c, e, _)| c == clip && e == ei) {
        remove(app);
        return;
    }
    match app.session.execute("effects.remove", json!({"clip": clip, "index": ei})) {
        Ok(_) => {
            // the effects after it move up one place
            if let Some((c, e, m)) = app.ui.kurgu.track.target
                && c == clip
                && e > ei
            {
                app.ui.kurgu.track.target = Some((c, e - 1, m));
            }
        }
        Err(e) => app.ui.status = e.to_string(),
    }
}

/// Moves the playhead to the box's latest tracked frame (where Track Forward stopped).
fn go_to_last_key(app: &mut FilmcraftApp) {
    let Some((clip, effect, mask)) = app.ui.kurgu.track.target else { return };
    let Some((_, it)) = app.session.active_sequence().and_then(|q| q.find_item(filmcraft_project::ClipId(clip))) else { return };
    let Some(last) = it.effects.get(effect).and_then(|e| e.masks.get(mask)).and_then(|m| m.path.keyframes.iter().map(|k| k.time).max()) else {
        return;
    };
    // media time → timeline time
    let rel = last.0.saturating_sub(it.source_in.0) as f64 / it.speed.abs().max(1e-9);
    if !rel.is_finite() || it.duration.0 <= 0 {
        return;
    }
    let t = filmcraft_time::Tick(it.start.0.saturating_add(rel.round() as i64)).clamp(it.start, it.end() - filmcraft_time::Tick(1));
    app.session.set_playhead(t);
}

/// Arrow · Sticker ▸ My Picture: places picture `item` at `start` on a new top layer, sized to a
/// small sticker; returns its clip (`masks.follow` then lengthens it over the tracked span).
fn place_picture(app: &mut FilmcraftApp, item: filmcraft_project::ItemId, start: filmcraft_time::Tick) -> Result<u64, String> {
    let placed = app
        .session
        .execute("timeline.place", json!({"items": [item.0], "track": "new", "time": start.0, "audio": false, "insert": false}))
        .map_err(|e| e.to_string())?;
    let clip = placed["clip"].as_u64().or_else(|| placed["clips"].get(0).and_then(serde_json::Value::as_u64)).ok_or_else(|| "no picture placed".to_string())?;
    let width = filmcraft_render::source_size(&app.session.project, item).map_or(0.0, |(w, _)| f64::from(w));
    let seq_w = app.session.active_sequence().map_or(0.0, |q| f64::from(q.settings.width));
    if width > 0.0 && seq_w > 0.0 {
        let scale = (100.0 * PICTURE_WIDTH * seq_w / width).clamp(1.0, 400.0);
        app.session.execute("effects.setParam", json!({"clip": clip, "effect": "motion", "param": "scale", "value": scale})).map_err(|e| e.to_string())?;
    }
    Ok(clip)
}

/// The teleport effect: the person vanishes (or appears) on the spot in a burst of light, over a
/// frame of the same shot with the place empty (`effects.teleport`).
fn teleport_panel(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    let inner = rect.shrink2(vec2(12.0, 8.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).id_salt("kurgu-teleport"));
    egui::ScrollArea::vertical().id_salt("kurgu-teleport-scroll").auto_shrink([false, false]).show(&mut child, |ui| {
        ui.set_width(inner.width());
        ui.spacing_mut().item_spacing.y = 8.0;
        if small_button(ui, &t, tl!("← Object Tracking"), "kurgu.teleport.back").clicked() {
            app.ui.kurgu.track.teleport = false;
        }
        ui.label(egui::RichText::new(tl!("Teleport")).size(15.0).strong().color(t.text));
        ui.label(
            egui::RichText::new(tl!(
                "The person vanishes or appears in a burst of light. Film with the camera standing still, with a moment where the place is empty."
            ))
            .size(11.5)
            .color(t.text_dim),
        );

        // 1. the clip
        step(ui, &t, "1", tl!("Select the video clip in the timeline"));
        let video = selected_video(app);
        let clip = match &video {
            Some((c, name, _)) => {
                ui.label(egui::RichText::new(format!("✔ {name}")).size(12.0).color(t.accent));
                Some(c.0)
            }
            None => {
                ui.label(egui::RichText::new(tl!("No video clip selected")).size(12.0).color(t.text_dim));
                None
            }
        };
        let item = clip.and_then(|c| app.session.active_sequence()?.find_item(filmcraft_project::ClipId(c)).map(|(_, it)| it.clone()));
        let ph = app.session.playhead();
        let on_clip = item.as_ref().is_some_and(|it| it.start <= ph && ph < it.end());

        // 2. the empty place (not needed when the person is cut out over a new background)
        step(ui, &t, "2", tl!("The empty place"));
        let new_bg = clip.is_some_and(|c| over_new_background(app, c));
        let plate = app.ui.kurgu.track.plate.filter(|(c, _)| Some(*c) == clip && !new_bg);
        if new_bg {
            ui.label(egui::RichText::new(tl!("✔ The person is cut out over a new background: no empty frame is needed.")).size(12.0).color(t.accent));
        } else if let Some((_, h)) = plate {
            let secs = h as f64 / filmcraft_time::TICKS_PER_SECOND as f64;
            ui.label(egui::RichText::new(tlf!("✔ Empty place: the frame at {s} s of the clip", s = format!("{secs:.1}"))).size(12.0).color(t.accent));
        }
        if !new_bg
            && big_button(app, ui, &t, tl!("This Frame Is the Empty Place"), "kurgu.teleport.plate", on_clip, plate.is_none())
                .on_hover_text(tl!("Move the playhead to a moment where the place is empty, then press the button."))
                .clicked()
            && let (Some(c), Some(it)) = (clip, &item)
        {
            app.ui.kurgu.track.plate = Some((c, it.moving_source_time_at(ph).0));
        }
        if clip.is_some() && !on_clip {
            hint(ui, &t, tl!("The playhead is not on the selected clip."));
        }

        // 3. how it looks
        step(ui, &t, "3", tl!("Light Effect"));
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(tl!("Color")).size(11.5).color(t.text_dim));
            for (i, (_, c)) in BODY_COLORS.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::click());
                ui.painter().circle_filled(r.center(), 8.0, *c);
                if usize::from(app.ui.kurgu.track.body_color) == i {
                    ui.painter().circle_stroke(r.center(), 10.0, Stroke::new(2.0, t.accent));
                }
                if resp.clicked() {
                    app.ui.kurgu.track.body_color = u8::try_from(i).unwrap_or(0);
                }
            }
        });
        let mut secs = if app.ui.kurgu.track.tp_secs > 0.0 { app.ui.kurgu.track.tp_secs } else { 0.8 };
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(tl!("Duration")).size(11.5).color(t.text_dim));
            if ui.add(egui::Slider::new(&mut secs, 0.3..=2.0).suffix(" s").step_by(0.1)).changed() {
                app.ui.kurgu.track.tp_secs = secs;
            }
        })
        .response
        .on_hover_text(if cut_out_clip(app).is_some_and(|(c, _, _)| Some(c) == clip) {
            tl!("The person is cut out: their shape lights up as they go.")
        } else {
            tl!("Tip: cut the person out first (Body Effects) and their shape lights up as they go; without it the picture flashes bright.")
        });

        // 4. the moment
        step(ui, &t, "4", tl!("The moment"));
        let ready = on_clip && (plate.is_some() || new_bg);
        let mut mode = None;
        hint(ui, &t, tl!("Move the playhead to the moment of the teleport."));
        // side by side: both stay in view without scrolling
        ui.horizontal(|ui| {
            if big_button(app, ui, &t, tl!("Vanish Here"), "kurgu.teleport.out", ready, true).clicked() {
                mode = Some("out");
            }
            if big_button(app, ui, &t, tl!("Appear Here"), "kurgu.teleport.in", ready, true).clicked() {
                mode = Some("in");
            }
        });
        if let (Some(mode), Some(c), true) = (mode, clip, ready) {
            let color = BODY_COLORS.get(usize::from(app.ui.kurgu.track.body_color)).map_or("#ffd75a", |c| c.0);
            let r = app.session.execute(
                "effects.teleport",
                json!({"clip": c, "time": ph.0, "plate": plate.map(|(_, h)| h),"mode": mode, "seconds": secs, "color": color, "plateName": tl!("empty place"), "lightName": tl!("teleport light")}),
            );
            match r {
                Ok(_) => {
                    app.session.state.selection = vec![filmcraft_project::ClipId(c)];
                    app.ui.status = if mode == "out" {
                        tl!("Teleport added: the person vanishes here").to_string()
                    } else {
                        tl!("Teleport added: the person appears here").to_string()
                    };
                }
                Err(e) => app.ui.status = e.to_string(),
            }
        }
        hint(ui, &t, tl!("The empty place and the light go on their own layers; Undo takes the whole teleport back."));
    });
}

/// The place Replace Background will take.
fn soon_tile(app: &mut FilmcraftApp, ui: &mut egui::Ui, t: &Tokens, width: f32) {
    let (r, resp) = ui.allocate_exact_size(vec2(width, 54.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, CornerRadius::same(8), t.field_bg);
    icons::paint(p, Rect::from_center_size(pos2(r.center().x, r.min.y + 12.0), vec2(14.0, 14.0)), Icon::Image, t.text_faint);
    p.text(pos2(r.center().x, r.min.y + 28.0), Align2::CENTER_CENTER, tl!("Replace Background"), Tokens::ui(11.0), t.text_faint);
    p.text(pos2(r.center().x, r.max.y - 9.0), Align2::CENTER_CENTER, tl!("Soon"), Tokens::ui(10.0), t.accent);
    app.auto.add("kurgu.tracking.background.soon", r, tl!("Replace Background"));
    resp.on_hover_text(tl!("Coming soon: separating a moving object from its background is being improved."));
}

/// `masks.follow`'s `fit` from the panel's choices.
fn fit_of(st: &TrackPanel) -> serde_json::Value {
    let size = if st.media_size.is_finite() && st.media_size > 0.0 { st.media_size.clamp(50.0, 250.0) } else { 100.0 };
    json!({"mode": if st.inside { "contain" } else { "cover" }, "size": f64::from(size)})
}

/// Step 5 of Replace with Media: how it sits, then the pictures and videos in Media.
fn replace_steps(app: &mut FilmcraftApp, ui: &mut egui::Ui, t: &Tokens, target: Option<(u64, usize, usize)>, tracking: bool) {
    step(ui, t, "5", tl!("Choose what takes its place"));
    let mut refit = false;
    ui.horizontal_wrapped(|ui| {
        for (inside, label, auto) in [(false, tl!("Cover the Object"), "kurgu.replace.cover"), (true, tl!("Fit Inside"), "kurgu.replace.inside")] {
            let r = ui.selectable_label(app.ui.kurgu.track.inside == inside, egui::RichText::new(label).size(11.5));
            app.auto.add(auto, r.rect, label);
            if r.clicked() && app.ui.kurgu.track.inside != inside {
                app.ui.kurgu.track.inside = inside;
                refit = true;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tl!("Size")).size(11.5).color(t.text_dim));
        let mut size = if app.ui.kurgu.track.media_size > 0.0 { app.ui.kurgu.track.media_size } else { 100.0 };
        let r = ui.add(egui::Slider::new(&mut size, 50.0..=250.0).suffix("%").fixed_decimals(0));
        app.auto.add("kurgu.replace.size", r.rect, tl!("Size"));
        if r.changed() {
            app.ui.kurgu.track.media_size = size;
        }
        // once, when let go: each change is an undo step
        refit |= r.drag_stopped() || (r.changed() && !r.dragged());
    });
    let mut hide = app.ui.kurgu.track.hide_behind;
    let r = ui.add_enabled(target.is_some(), egui::Checkbox::new(&mut hide, egui::RichText::new(tl!("Blur what shows behind it")).size(11.5)));
    app.auto.add("kurgu.replace.hide", r.rect, tl!("Blur what shows behind it"));
    if r.changed()
        && let Some((clip, effect, mask)) = target
    {
        // strong, and soft at the edge: a hard-edged blurred patch shows more than the object did
        let (amount, feather) = if hide { (90.0, 36.0) } else { (0.0, 4.0) };
        let set = app
            .session
            .execute("effects.setParam", json!({"clip": clip, "effect": effect, "param": "blurriness", "value": amount}))
            .and_then(|_| app.session.execute("masks.set", json!({"clip": clip, "effect": effect, "mask": mask, "feather": feather})));
        match set {
            Ok(_) => app.ui.kurgu.track.hide_behind = hide,
            Err(e) => app.ui.status = e.to_string(),
        }
    }
    if refit {
        refit_media(app);
    }

    let tracked = !tracking && target.and_then(|tg| target_state(app, tg)).unwrap_or(0) > 1;
    let own = target.and_then(|(clip, _, _)| app.session.active_sequence()?.find_item(filmcraft_project::ClipId(clip)).map(|(_, it)| it.item));
    let mut items: Vec<(filmcraft_project::ItemId, String)> = app
        .session
        .project
        .items
        .values()
        .filter(|i| i.has_video() && Some(i.id) != own && matches!(i.kind, filmcraft_project::ItemKind::Media(_)))
        .map(|i| (i.id, i.name.clone()))
        .collect();
    items.sort_by(|a, b| a.1.cmp(&b.1));
    if items.is_empty() {
        hint(ui, t, tl!("Import a picture or video in Media first (a PNG with a transparent background looks best)."));
    }
    for (k, (id, name)) in items.into_iter().enumerate() {
        ui.horizontal(|ui| {
            let r = ui.add_enabled(tracked, egui::Button::new(egui::RichText::new(tl!("Use")).size(11.5)).corner_radius(6));
            app.auto.add(&format!("kurgu.replace.media.{k}"), r.rect, tl!("Use"));
            ui.add(egui::Label::new(egui::RichText::new(name).size(12.0).color(t.text)).truncate());
            if r.clicked() {
                replace_with(app, id);
            }
        });
    }
    let placed =
        app.ui.kurgu.track.follower.is_some_and(|c| app.session.active_sequence().is_some_and(|q| q.find_item(filmcraft_project::ClipId(c)).is_some()));
    if placed {
        ui.label(egui::RichText::new(tl!("✔ It follows the object and grows and shrinks with it")).size(12.0).color(t.accent));
        hint(ui, t, tl!("If you track further or move the box, press Use again."));
    } else {
        hint(ui, t, tl!("Track first; then press Use. The proportions are kept as the object comes nearer or moves away."));
    }
}

/// Sizes the media in the object's place again, after a choice changed.
fn refit_media(app: &mut FilmcraftApp) {
    let st = &app.ui.kurgu.track;
    let (Some((clip, effect, mask)), Some(media)) = (st.target, st.follower) else { return };
    let fit = fit_of(st);
    if app.session.active_sequence().is_none_or(|q| q.find_item(filmcraft_project::ClipId(media)).is_none()) {
        return;
    }
    if let Err(e) = app.session.execute("masks.follow", json!({"clip": clip, "effect": effect, "mask": mask, "target": media, "fit": fit})) {
        app.ui.status = e.to_string();
    }
}

/// Puts `item` in the tracked object's place on its own layer; it takes the place of the one before.
fn replace_with(app: &mut FilmcraftApp, item: filmcraft_project::ItemId) {
    let Some((clip, effect, mask)) = app.ui.kurgu.track.target else { return };
    let start = app.session.active_sequence().and_then(|q| {
        let (_, it) = q.find_item(filmcraft_project::ClipId(clip))?;
        let first = it.effects.get(effect)?.masks.get(mask)?.path.keyframes.iter().map(|k| k.time).min()?;
        let speed = it.speed.abs().max(1e-9);
        Some(filmcraft_time::Tick(it.start.0.saturating_add(((first.0 - it.source_in.0) as f64 / speed).round() as i64)).max(it.start))
    });
    let Some(start) = start else { return };
    let fit = fit_of(&app.ui.kurgu.track);
    let r = (|| -> Result<u64, String> {
        let placed = app
            .session
            .execute("timeline.place", json!({"items": [item.0], "track": "new", "time": start.0, "audio": false, "insert": false}))
            .map_err(|e| e.to_string())?;
        let media =
            placed["clip"].as_u64().or_else(|| placed["clips"].get(0).and_then(serde_json::Value::as_u64)).ok_or_else(|| "no media placed".to_string())?;
        if let Err(e) = app.session.execute("masks.follow", json!({"clip": clip, "effect": effect, "mask": mask, "target": media, "fit": fit})) {
            // not left lying full screen over the video
            let _ = app.session.execute("edit.clear", json!({"clip": media}));
            return Err(e.to_string());
        }
        Ok(media)
    })();
    match r {
        Ok(media) => {
            if let Some(old) = app.ui.kurgu.track.follower.replace(media)
                && app.session.active_sequence().is_some_and(|q| q.find_item(filmcraft_project::ClipId(old)).is_some())
                && let Err(e) = app.session.execute("edit.clear", json!({"clip": old}))
            {
                app.ui.status = e.to_string();
            }
            // the tracked clip stays selected, so the panel keeps its place
            app.session.state.selection = vec![filmcraft_project::ClipId(clip)];
        }
        Err(e) => app.ui.status = e,
    }
}
