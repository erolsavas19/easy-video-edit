//! VideoKurgu's visual galleries: effects, transitions and color looks as cards with real
//! previews rendered by FilmCraft's own engine (a transition plays while the pointer is over its
//! card), favorites first, then groups. A double click applies the item to the selection; a drag
//! drops it on the timeline, as in FilmCraft's Effects panel.

use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use filmcraft_project::{EffectDef, EffectKind};
use filmcraft_render::image::Image;
use filmcraft_render::lumetri_presets::{self as lp, LumetriPreset};
use serde_json::json;

use crate::FilmcraftApp;
use crate::icons::{self, Icon};
use crate::theme::Tokens;

/// Size of a rendered thumbnail (16:9).
const THUMB_W: usize = 192;
const THUMB_H: usize = 108;
/// Frames of a transition's hover animation.
const STEPS: usize = 12;
/// New thumbnails rendered per frame at most, so opening a gallery never stalls the window.
const RENDERS_PER_FRAME: usize = 6;
/// Prefix of a favorite that names a color look rather than an effect id.
const LOOK: &str = "lumetri:";

/// Which part of FilmCraft's effect catalog a gallery shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Source {
    VideoTransitions,
    VideoEffects,
    AudioEffects,
    AudioTransitions,
}

impl Source {
    fn includes(self, d: &EffectDef) -> bool {
        let top = d.category.first().copied().unwrap_or("");
        let sub = d.category.get(1).copied().unwrap_or("");
        match self {
            Source::VideoTransitions => d.kind == EffectKind::VideoTransition && top == "Video Transitions",
            Source::VideoEffects => d.kind == EffectKind::Video && top == "Video Effects" && sub != "Obsolete" && !d.intrinsic,
            Source::AudioEffects => d.kind == EffectKind::Audio && top == "Audio Effects" && !d.intrinsic,
            Source::AudioTransitions => d.kind == EffectKind::AudioTransition,
        }
    }
    fn is_audio(self) -> bool {
        matches!(self, Source::AudioEffects | Source::AudioTransitions)
    }
    fn is_transition(self) -> bool {
        matches!(self, Source::VideoTransitions | Source::AudioTransitions)
    }
    fn hint(self) -> &'static str {
        if self.is_transition() {
            "Çift tıklayın ya da iki klibin arasına sürükleyin"
        } else {
            "Çift tıklayın ya da zaman çizelgesindeki klibe sürükleyin"
        }
    }
}

/// One card of a gallery.
struct Card {
    /// Favorite key: an effect id, or `lumetri:<name>` for a color look.
    key: String,
    label: String,
    group: String,
}

/// A row of tabs at the top of `rect`; returns the area under them.
pub fn tabs(ui: &mut egui::Ui, t: &Tokens, rect: Rect, labels: &[&str], current: &mut u8) -> Rect {
    let h = 30.0;
    let n = labels.len().max(1) as f32;
    let bar = Rect::from_min_size(pos2(rect.min.x + 8.0, rect.min.y + 6.0), vec2(rect.width() - 16.0, h));
    ui.painter().rect_filled(bar, CornerRadius::same(8), t.field_bg);
    let w = bar.width() / n;
    for (i, label) in labels.iter().enumerate() {
        let r = Rect::from_min_size(pos2(bar.min.x + i as f32 * w, bar.min.y), vec2(w, h)).shrink(3.0);
        let resp = ui.interact(r, egui::Id::unique(("kurgu-tab", *label)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let active = usize::from(*current) == i;
        if active {
            ui.painter().rect_filled(r, CornerRadius::same(6), t.accent);
        } else if resp.hovered() {
            ui.painter().rect_filled(r, CornerRadius::same(6), t.hover);
        }
        let color = if active { Color32::WHITE } else { t.text_dim };
        ui.painter().text(r.center(), Align2::CENTER_CENTER, *label, Tokens::ui(12.0), color);
        if resp.clicked() {
            *current = u8::try_from(i).unwrap_or(0);
        }
    }
    Rect::from_min_max(pos2(rect.min.x, bar.max.y + 4.0), rect.max)
}

/// A gallery of effects or transitions from FilmCraft's catalog.
pub fn effects(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect, src: Source) {
    let body = search(app, ui, rect);
    let q = app.ui.kurgu.search.trim().to_lowercase();
    let cards: Vec<Card> = filmcraft_project::effect_defs()
        .iter()
        .filter(|d| src.includes(d))
        .map(|d| Card {
            key: d.id.to_string(),
            label: crate::i18n::t(d.name).to_string(),
            group: crate::i18n::t(d.category.get(1).copied().unwrap_or("Diğer")).to_string(),
        })
        .filter(|c| q.is_empty() || c.label.to_lowercase().contains(&q) || c.key.to_lowercase().contains(&q))
        .collect();
    let mut renders = 0usize;
    let action = grid(app, ui, body, &cards, src.hint(), &mut |app, ui, card, r, hovered| {
        if src.is_audio() {
            audio_tile(ui, &app.tokens, r, src.is_transition());
            return;
        }
        let step = if src.is_transition() { animation_step(ui, hovered) } else { STEPS / 2 };
        let name = format!("kurgu-fx-{}-{step}", card.key);
        let tex = match app.texture_existing(&name) {
            Some((id, _)) => Some(id),
            None if renders < RENDERS_PER_FRAME => {
                renders += 1;
                render_effect(&card.key, src.is_transition(), step).map(|img| upload(app, ui.ctx(), &name, &img))
            }
            None => {
                ui.ctx().request_repaint();
                None
            }
        };
        paint_thumb(ui, &app.tokens, r, tex);
    });
    match action {
        Some(Action::Apply(key)) => {
            if let Err(e) = app.session.execute("effects.apply", json!({"effect": key})) {
                app.ui.status = e.to_string();
            }
        }
        Some(Action::Drag(key)) => crate::panels::start_drag_effect(ui, &key),
        None => {}
    }
}

/// The color looks (FilmCraft's Color Grading Presets) as a gallery.
pub fn color_looks(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let body = search(app, ui, rect);
    let q = app.ui.kurgu.search.trim().to_lowercase();
    let presets: Vec<LumetriPreset> = lp::presets();
    let cards: Vec<Card> = presets
        .iter()
        .map(|p| Card { key: format!("{LOOK}{}", p.name), label: crate::i18n::t(p.name).to_string(), group: crate::i18n::t(p.folder).to_string() })
        .filter(|c| q.is_empty() || c.label.to_lowercase().contains(&q))
        .collect();
    let mut renders = 0usize;
    let action = grid(app, ui, body, &cards, "Çift tıklayın ya da zaman çizelgesindeki klibe sürükleyin", &mut |app, ui, card, r, _| {
        let name = format!("kurgu-look-{}", card.key);
        let tex = match app.texture_existing(&name) {
            Some((id, _)) => Some(id),
            None if renders < RENDERS_PER_FRAME => {
                renders += 1;
                let preset = card.key.strip_prefix(LOOK).and_then(|n| presets.iter().find(|p| p.name == n));
                preset.map(|p| lp::thumbnail(p, THUMB_W, THUMB_H)).map(|img| upload(app, ui.ctx(), &name, &img))
            }
            None => {
                ui.ctx().request_repaint();
                None
            }
        };
        paint_thumb(ui, &app.tokens, r, tex);
    });
    match action {
        Some(Action::Apply(key)) => {
            let name = key.strip_prefix(LOOK).unwrap_or(&key);
            if let Err(e) = app.session.execute("lumetri.applyPreset", json!({"name": name})) {
                app.ui.status = e.to_string();
            }
        }
        Some(Action::Drag(key)) => {
            let name = key.strip_prefix(LOOK).unwrap_or(&key).to_string();
            crate::panels::start_drag_lumetri_preset(ui, &name);
        }
        None => {}
    }
}

/// The search field at the top of a gallery; returns the area under it.
fn search(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) -> Rect {
    let t = app.tokens;
    let sr = Rect::from_min_size(rect.min + vec2(10.0, 8.0), vec2((rect.width() - 20.0).max(40.0), 24.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(sr).id_salt("kurgu-search"));
    let mut q = app.ui.kurgu.search.clone();
    crate::widgets::search_field(&mut child, &mut q, "Ara", sr.width(), &t);
    app.ui.kurgu.search = q;
    Rect::from_min_max(pos2(rect.min.x, sr.max.y + 6.0), rect.max)
}

enum Action {
    Apply(String),
    Drag(String),
}

type Thumb<'a> = dyn FnMut(&mut FilmcraftApp, &mut egui::Ui, &Card, Rect, bool) + 'a;

/// Lays the cards out: Favorites first, then one section per group, in a vertical scroll area.
fn grid(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect, cards: &[Card], hint: &str, thumb: &mut Thumb<'_>) -> Option<Action> {
    let t = app.tokens;
    let hint_h = 22.0;
    let area = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.max.y - hint_h));
    ui.painter().text(pos2(rect.center().x, rect.max.y - hint_h / 2.0), Align2::CENTER_CENTER, hint, Tokens::ui(11.0), t.text_faint);
    let mut action = None;
    let mut gui = ui.new_child(egui::UiBuilder::new().max_rect(area).id_salt("kurgu-grid"));
    gui.set_clip_rect(area.intersect(ui.clip_rect()));
    if cards.is_empty() {
        gui.painter().text(area.center(), Align2::CENTER_CENTER, "Sonuç yok", Tokens::ui(13.0), t.text_dim);
        return None;
    }
    let favorites: Vec<&Card> = cards.iter().filter(|c| app.ui.kurgu.favorites.contains(&c.key)).collect();
    let mut groups: Vec<&str> = Vec::new();
    for c in cards {
        if !groups.contains(&c.group.as_str()) {
            groups.push(&c.group);
        }
    }
    egui::ScrollArea::vertical().id_salt("kurgu-grid-scroll").auto_shrink([false, false]).show(&mut gui, |ui| {
        let width = ui.available_width() - 12.0;
        let cols = ((width / 150.0).floor() as usize).max(1);
        let card_w = (width - 8.0 * (cols as f32 - 1.0)) / cols as f32;
        let card_h = card_w * 9.0 / 16.0 + 24.0;
        let mut section = |ui: &mut egui::Ui, app: &mut FilmcraftApp, title: &str, list: &[&Card], salt: &str| {
            let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::hover());
            ui.painter().text(pos2(head.min.x + 8.0, head.center().y + 2.0), Align2::LEFT_CENTER, title, Tokens::ui(12.5), t.text);
            for row in list.chunks(cols) {
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), card_h + 8.0), Sense::hover());
                for (i, card) in row.iter().enumerate() {
                    let cr = Rect::from_min_size(pos2(r.min.x + 6.0 + i as f32 * (card_w + 8.0), r.min.y), vec2(card_w, card_h));
                    if !ui.is_rect_visible(cr) {
                        continue;
                    }
                    if let Some(a) = draw_card(app, ui, card, cr, salt, thumb) {
                        action = Some(a);
                    }
                }
            }
        };
        if !favorites.is_empty() {
            section(ui, app, "★ Favoriler", &favorites, "fav");
        }
        for g in &groups {
            let list: Vec<&Card> = cards.iter().filter(|c| c.group == *g).collect();
            section(ui, app, g, &list, g);
        }
    });
    action
}

/// One card: the thumbnail, its name, the favorite star; returns what the user did with it.
fn draw_card(app: &mut FilmcraftApp, ui: &mut egui::Ui, card: &Card, r: Rect, salt: &str, thumb: &mut Thumb<'_>) -> Option<Action> {
    let t = app.tokens;
    let resp = ui.interact(r, egui::Id::unique(("kurgu-card", salt, card.key.as_str())), Sense::click_and_drag());
    let hovered = resp.hovered();
    let selected = app.ui.kurgu.selected.as_deref() == Some(card.key.as_str());
    let img = Rect::from_min_size(r.min, vec2(r.width(), r.width() * 9.0 / 16.0));
    thumb(app, ui, card, img, hovered);
    let p = ui.painter();
    let border = if selected {
        Stroke::new(2.0, t.accent)
    } else if hovered {
        Stroke::new(1.5, t.text_dim)
    } else {
        Stroke::new(1.0, t.separator)
    };
    p.rect_stroke(img, CornerRadius::same(6), border, StrokeKind::Inside);
    let name = Rect::from_min_max(pos2(r.min.x + 2.0, img.max.y + 2.0), r.max);
    let galley = p.layout(card.label.clone(), Tokens::ui(11.0), if selected { t.text } else { t.text_dim }, name.width());
    p.galley(pos2(name.center().x - galley.size().x / 2.0, name.min.y + 2.0), galley, t.text);

    // Favorite star: always shown when on, on hover otherwise.
    let fav = app.ui.kurgu.favorites.contains(&card.key);
    let star = Rect::from_min_size(pos2(img.max.x - 24.0, img.min.y + 4.0), vec2(20.0, 20.0));
    let mut star_clicked = false;
    if fav || hovered {
        let sresp = ui.interact(star, egui::Id::unique(("kurgu-star", salt, card.key.as_str())), Sense::click()).on_hover_text(if fav {
            "Favorilerden çıkar"
        } else {
            "Favorilere ekle"
        });
        ui.painter().circle_filled(star.center(), 10.0, Color32::from_black_alpha(140));
        paint_star(ui.painter(), star.center(), 7.0, if fav { Color32::from_rgb(255, 200, 40) } else { Color32::from_gray(220) }, !fav);
        if sresp.clicked() {
            star_clicked = true;
            if fav {
                app.ui.kurgu.favorites.retain(|k| k != &card.key);
            } else {
                app.ui.kurgu.favorites.push(card.key.clone());
            }
        }
    }
    if star_clicked {
        return None;
    }
    let resp = resp.on_hover_text(card.label.as_str());
    if resp.clicked() {
        app.ui.kurgu.selected = Some(card.key.clone());
    }
    if resp.drag_started_by(egui::PointerButton::Primary) {
        return Some(Action::Drag(card.key.clone()));
    }
    if resp.double_clicked() {
        return Some(Action::Apply(card.key.clone()));
    }
    None
}

/// A five-pointed star, filled or outlined.
fn paint_star(p: &egui::Painter, c: Pos2, r: f32, color: Color32, outline: bool) {
    let pts: Vec<Pos2> = (0..10)
        .map(|i| {
            let a = std::f32::consts::PI * (i as f32) / 5.0 - std::f32::consts::FRAC_PI_2;
            let rr = if i % 2 == 0 { r } else { r * 0.45 };
            pos2(c.x + rr * a.cos(), c.y + rr * a.sin())
        })
        .collect();
    if outline {
        p.add(egui::Shape::closed_line(pts, Stroke::new(1.3, color)));
    } else {
        p.add(egui::Shape::convex_polygon(pts.clone(), color, Stroke::NONE));
        // a star is not convex: fill the centre too
        p.circle_filled(c, r * 0.45, color);
    }
}

/// The frame of a transition's animation: the middle frame at rest, cycling while hovered.
fn animation_step(ui: &egui::Ui, hovered: bool) -> usize {
    if !hovered {
        return STEPS / 2;
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
    let time = ui.input(|i| i.time);
    // two frames of A, the move, two frames of B
    let cycle = STEPS + 4;
    let k = ((time * 10.0) as usize) % cycle;
    k.saturating_sub(2).min(STEPS - 1)
}

/// Draws a thumbnail texture, or a placeholder while it is being rendered.
fn paint_thumb(ui: &egui::Ui, t: &Tokens, r: Rect, tex: Option<egui::TextureId>) {
    match tex {
        Some(id) => {
            egui::Image::new((id, r.size())).corner_radius(CornerRadius::same(6)).paint_at(ui, r);
        }
        None => {
            ui.painter().rect_filled(r, CornerRadius::same(6), t.field_bg);
        }
    }
}

/// An audio item's tile: a speaker over a waveform, with two crossing ramps for a transition.
fn audio_tile(ui: &egui::Ui, t: &Tokens, r: Rect, transition: bool) {
    let p = ui.painter();
    p.rect_filled(r, CornerRadius::same(6), t.field_bg);
    let mid = r.center().y + r.height() * 0.18;
    let n = 28;
    for i in 0..n {
        let x = r.min.x + 10.0 + (r.width() - 20.0) * i as f32 / (n - 1) as f32;
        let a = ((i as f32 * 0.9).sin() * 0.5 + 0.5) * (r.height() * 0.22) + 2.0;
        p.line_segment([pos2(x, mid - a), pos2(x, mid + a)], Stroke::new(2.0, t.accent.gamma_multiply(0.6)));
    }
    if transition {
        let (a, b) = (pos2(r.min.x + 12.0, r.min.y + 14.0), pos2(r.max.x - 12.0, mid + 4.0));
        p.line_segment([a, b], Stroke::new(1.5, t.text_dim));
        p.line_segment([pos2(a.x, b.y), pos2(b.x, a.y)], Stroke::new(1.5, t.text_dim));
    } else {
        icons::paint(p, Rect::from_center_size(pos2(r.center().x, r.min.y + r.height() * 0.3), vec2(22.0, 22.0)), Icon::Speaker, t.text);
    }
}

/// Renders effect `id`: a transition between two sample pictures at frame `step`, or an effect
/// over the sample picture.
fn render_effect(id: &str, transition: bool, step: usize) -> Option<Image> {
    let def = filmcraft_project::effect_defs().iter().find(|d| d.id == id)?;
    let inst = def.instance();
    let a = lp::preview_image(THUMB_W, THUMB_H);
    if transition {
        let b = second_picture(&a);
        let p = step as f32 / (STEPS - 1) as f32;
        return Some(filmcraft_render::transitions::apply(&inst, &a, &b, p));
    }
    let mut img = a;
    let cx = filmcraft_render::effects::FxCtx {
        t: filmcraft_time::Tick::ZERO,
        px_scale: THUMB_W as f32 / 1920.0,
        mask_scale: THUMB_W as f32 / 1920.0,
        seconds: 1.0,
        timecode: "00:00:01:00",
        clip_name: "",
        project: None,
        env: None,
        frame_rate: 24.0,
        working: filmcraft_color::WorkingSpace::Rec709,
    };
    filmcraft_render::effects::apply(&mut img, &inst, &cx);
    Some(img)
}

/// The second picture of a transition preview: the sample mirrored, its colors turned to cool
/// blue-greens, so the two sides of a transition are easy to tell apart.
fn second_picture(a: &Image) -> Image {
    let mut b = Image::new(a.w, a.h);
    for y in 0..a.h {
        for x in 0..a.w {
            let src = (y * a.w + (a.w - 1 - x)) * 4;
            let dst = (y * a.w + x) * 4;
            if let (Some(s), Some(d)) = (a.px.get(src..src + 4), b.px.get_mut(dst..dst + 4)) {
                d[0] = s[2] * 0.6;
                d[1] = s[0] * 0.8;
                d[2] = s[1].max(s[0]) * 1.1;
                d[3] = s[3];
            }
        }
    }
    b
}

/// Uploads a rendered picture as a texture named `name`.
fn upload(app: &mut FilmcraftApp, ctx: &egui::Context, name: &str, img: &Image) -> egui::TextureId {
    let rgba = crate::frames::Rgba { w: img.w, h: img.h, px: img.over_black_rgba8() };
    let key = crate::frames::FrameKey {
        target: crate::frames::Target::Item(filmcraft_project::ItemId(u64::MAX)),
        frame: 0,
        size: img.w as u32,
        revision: 0,
        draft: false,
    };
    app.texture_for(ctx, name, key, &rgba)
}
