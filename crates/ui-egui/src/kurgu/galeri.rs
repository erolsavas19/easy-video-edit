//! Easy Video Edit's visual galleries: effects, transitions and color looks as cards with real
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
            tl!("Select a clip and double-click · or drag between two clips")
        } else {
            tl!("Select a clip and double-click · or drag onto a clip")
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
    let mut cards: Vec<Card> = filmcraft_project::effect_defs()
        .iter()
        .filter(|d| src.includes(d))
        .map(|d| Card {
            key: d.id.to_string(),
            label: crate::i18n::t(d.name).to_string(),
            group: if src == Source::VideoTransitions && POPULAR_TRANSITIONS.contains(&d.id) {
                tl!("Most Used").to_string()
            } else {
                crate::i18n::t(d.category.get(1).copied().unwrap_or("Other")).to_string()
            },
        })
        .filter(|c| q.is_empty() || matches(&q, &[&c.label, &c.key]))
        .collect();
    // the most used transitions first, in the order of the list
    if src == Source::VideoTransitions {
        cards.sort_by_key(|c| POPULAR_TRANSITIONS.iter().position(|p| *p == c.key).unwrap_or(usize::MAX));
    }
    let mut renders = 0usize;
    let smp = sample(app, ui.ctx());
    let action = grid(app, ui, body, &cards, src.hint(), false, &mut |app, ui, card, r, hovered| {
        if src.is_audio() {
            audio_tile(ui, &app.tokens, r, src.is_transition());
            return;
        }
        let step = if src.is_transition() { animation_step(ui, hovered) } else { STEPS / 2 };
        let name = format!("kurgu-fx-{}-{step}-{}", card.key, smp.tag);
        let tex = match app.texture_existing(&name) {
            Some((id, _)) => Some(id),
            None if renders < RENDERS_PER_FRAME => {
                renders += 1;
                render_effect(&card.key, src.is_transition(), step, &smp.img).map(|img| upload(app, ui.ctx(), &name, &img))
            }
            None => {
                ui.ctx().request_repaint();
                None
            }
        };
        paint_thumb(ui, &app.tokens, r, tex);
    });
    match action {
        Some(Action::Apply(key)) => apply_effect(app, &key),
        Some(Action::Drag(key)) => crate::panels::start_drag_effect(ui, &key),
        None => {}
    }
}

/// Transitions most editors reach for, shown first under "Most Used".
const POPULAR_TRANSITIONS: &[&str] = &[
    "cross_dissolve",
    "dip_to_black",
    "dip_to_white",
    "blur_dissolve",
    "film_dissolve",
    "push",
    "slide",
    "zoom_blur",
    "cross_zoom",
    "whip",
    "flash",
    "light_leak",
    "glitch",
    "linear_wipe",
];

/// Applies an effect or transition to the selected clips; its settings open on the right.
fn apply_effect(app: &mut FilmcraftApp, id: &str) {
    if app.session.state.selection.is_empty() {
        app.ui.status = tl!("Select a clip in the timeline first").to_string();
        return;
    }
    match app.session.execute("effects.apply", json!({"effect": id})) {
        Ok(_) => {
            app.ui.kurgu.right_tab = 1;
            // fold the fixed effects (Motion, Opacity, Time Remapping) so the one just added is in
            // view in Effect Settings; they open again with a click
            let mut keys = Vec::new();
            if let Some(seq) = app.session.active_sequence() {
                for clip in &app.session.state.selection {
                    if let Some((_, it)) = seq.find_item(*clip) {
                        for (i, e) in it.effects.iter().enumerate() {
                            if e.def().is_some_and(|d| d.intrinsic) {
                                keys.push(format!("{}:{i}", clip.0));
                            }
                        }
                    }
                }
            }
            for k in keys {
                if !app.ui.collapsed_fx.contains(&k) {
                    app.ui.collapsed_fx.push(k);
                }
            }
        }
        Err(e) => app.ui.status = e.to_string(),
    }
}

/// Lower case for searching, with the Turkish dotted/dotless I folded, so "İ", "I", "ı" and "i"
/// all match one another.
fn fold(s: &str) -> String {
    s.to_lowercase().replace("i\u{307}", "i").replace('ı', "i")
}

/// Whether every word of the query (already folded) is in one of the texts.
fn matches(q: &str, texts: &[&str]) -> bool {
    let hay: String = texts.iter().map(|t| fold(t)).collect::<Vec<_>>().join(" ");
    fold(q).split_whitespace().all(|w| hay.contains(w))
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
    let smp = sample(app, ui.ctx());
    let action = grid(app, ui, body, &cards, tl!("Select a clip and double-click · or drag onto a clip"), false, &mut |app, ui, card, r, _| {
        let name = format!("kurgu-look-{}-{}", card.key, smp.tag);
        let tex = match app.texture_existing(&name) {
            Some((id, _)) => Some(id),
            None if renders < RENDERS_PER_FRAME => {
                renders += 1;
                let preset = card.key.strip_prefix(LOOK).and_then(|n| presets.iter().find(|p| p.name == n));
                preset.map(|p| render_look(p, &smp.img)).map(|img| upload(app, ui.ctx(), &name, &img))
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

/// The text templates (FilmCraft's graphics template library, Easy Video Edit's own first) as a
/// gallery: each card shows the template over the sample picture and plays its intro on hover.
/// Which part of the template library a gallery shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Library {
    /// Text templates: titles, lower thirds, labels…
    Texts,
    /// Elements: arrows, icons, faces, bubbles, stickers.
    Elements,
    /// Intros: full-frame openings.
    Intros,
}

impl Library {
    fn includes(self, id: &str) -> bool {
        use filmcraft_project::kurgu_templates::{ELEMENT_PREFIX, INTRO_PREFIX};
        match self {
            Library::Texts => !id.starts_with(ELEMENT_PREFIX) && !id.starts_with(INTRO_PREFIX),
            Library::Elements => id.starts_with(ELEMENT_PREFIX),
            Library::Intros => id.starts_with(INTRO_PREFIX),
        }
    }
    /// How much a card enlarges the middle of the frame: elements are small in a full frame.
    fn zoom(self) -> f64 {
        if self == Library::Elements { 2.4 } else { 1.0 }
    }
    /// Seconds a card's animation covers: an intro's layers come in one after another.
    fn span(self) -> f64 {
        if self == Library::Intros { 2.0 } else { 1.0 }
    }
}

pub fn texts(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect, lib: Library) {
    let body = search(app, ui, rect);
    let q = app.ui.kurgu.search.trim().to_lowercase();
    let library = template_library(app, ui.ctx());
    let mut ordered: Vec<&filmcraft_engine::graphic_templates::LibraryEntry> = library.iter().filter(|e| lib.includes(&e.template.id)).collect();
    // Easy Video Edit's templates first, then FilmCraft's built-in ones, then the user's own.
    ordered.sort_by_key(|e| {
        if e.template.id.starts_with("easyvideoedit:") {
            0
        } else if e.path.is_none() {
            1
        } else {
            2
        }
    });
    let cards: Vec<Card> = ordered
        .iter()
        .map(|e| Card { key: e.template.id.clone(), label: translate_parts(&e.template.name), group: translate_parts(&e.template.category) })
        .filter(|c| q.is_empty() || c.label.to_lowercase().contains(&q) || c.group.to_lowercase().contains(&q))
        .collect();
    let mut renders = 0usize;
    let smp = sample(app, ui.ctx());
    let action = grid(app, ui, body, &cards, tl!("Double-click: add at the playhead · or drag onto the timeline"), false, &mut |app, ui, card, r, hovered| {
        let step = text_step(ui, hovered);
        // the language is in the name: the sample text of a template follows it
        let name = format!("kurgu-tpl-{}-{}-{step}-{}", card.key, crate::i18n::current().bcp47(), smp.tag);
        let tex = match app.texture_existing(&name) {
            Some((id, _)) => Some(id),
            None if renders < RENDERS_PER_FRAME => {
                renders += 1;
                let entry = library.iter().find(|e| e.template.id == card.key);
                entry.map(|e| render_template(&e.template, step, lib.zoom(), lib.span(), &smp.img)).map(|img| upload(app, ui.ctx(), &name, &img))
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
            if let Err(e) = app.session.execute("graphics.template.apply", json!({"template": key})) {
                app.ui.status = e.to_string();
            }
        }
        Some(Action::Drag(key)) => {
            let name = library.iter().find(|e| e.template.id == key).map(|e| e.template.name.clone()).unwrap_or_default();
            crate::panels::start_drag_template(ui, &key, &name);
        }
        None => {}
    }
}

/// A name made of parts ("Title · Simple · Fade"), translated part by part.
fn translate_parts(name: &str) -> String {
    name.split(" · ").map(crate::i18n::t).collect::<Vec<_>>().join(" · ")
}

/// The template library, read again at most every five seconds (user templates can be added
/// while the app runs).
fn template_library(app: &FilmcraftApp, ctx: &egui::Context) -> std::sync::Arc<Vec<filmcraft_engine::graphic_templates::LibraryEntry>> {
    type Lib = std::sync::Arc<Vec<filmcraft_engine::graphic_templates::LibraryEntry>>;
    let id = egui::Id::unique(("kurgu-template-library", filmcraft_project::kurgu_templates::is_turkish()));
    let now = ctx.input(|i| i.time);
    if let Some((at, lib)) = ctx.data(|d| d.get_temp::<(f64, Lib)>(id))
        && now - at < 5.0
    {
        return lib;
    }
    let lib: Lib = std::sync::Arc::new(filmcraft_engine::graphic_templates::library(&app.session));
    ctx.data_mut(|d| d.insert_temp(id, (now, lib.clone())));
    lib
}

/// Frames of a text template's hover animation (its intro, about one second).
const TEXT_STEPS: usize = 10;

/// The frame of a template's preview: settled at rest, the intro playing while hovered.
fn text_step(ui: &egui::Ui, hovered: bool) -> usize {
    if !hovered {
        return TEXT_STEPS;
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
    let time = ui.input(|i| i.time);
    let cycle = TEXT_STEPS + 8;
    (((time * 12.0) as usize) % cycle).min(TEXT_STEPS)
}

/// Renders a template over the dimmed sample picture at frame `step` of its intro
/// (`TEXT_STEPS` = settled, one second in).
fn render_template(t: &filmcraft_project::gtemplate::GraphicsTemplate, step: usize, zoom: f64, span: f64, base: &Image) -> Image {
    let mut img = base.clone();
    for (i, v) in img.px.iter_mut().enumerate() {
        if i % 4 != 3 {
            *v *= 0.55;
        }
    }
    let secs = step.min(TEXT_STEPS) as f64 / TEXT_STEPS as f64 * span;
    let at = filmcraft_time::Tick((secs * filmcraft_time::TICKS_PER_SECOND as f64) as i64).min(t.duration);
    let canvas = (t.canvas[0].max(1), t.canvas[1].max(1));
    let k = (THUMB_W as f64 / canvas.0 as f64).min(THUMB_H as f64 / canvas.1 as f64);
    // the middle of the canvas at the middle of the card, enlarged `zoom` times
    let s = k * zoom.max(1.0);
    // `a.then_apply(b)` applies `b` first: centre the canvas on the origin, scale, then move to the card's middle
    let m = filmcraft_geom::Affine::translate(THUMB_W as f64 / 2.0, THUMB_H as f64 / 2.0)
        .then_apply(&filmcraft_geom::Affine::scale(s, s))
        .then_apply(&filmcraft_geom::Affine::translate(-(canvas.0 as f64) / 2.0, -(canvas.1 as f64) / 2.0));
    filmcraft_render::graphic_clip::render_graphic_layers(&t.layers, at, canvas, &m, &mut img);
    img
}

/// The search field at the top of a gallery; returns the area under it.
fn search(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) -> Rect {
    let t = app.tokens;
    let sr = Rect::from_min_size(rect.min + vec2(10.0, 8.0), vec2((rect.width() - 20.0).max(40.0), 24.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(sr).id_salt("kurgu-search"));
    let mut q = app.ui.kurgu.search.clone();
    crate::widgets::search_field(&mut child, &mut q, tl!("Search"), sr.width(), &t);
    app.ui.kurgu.search = q;
    Rect::from_min_max(pos2(rect.min.x, sr.max.y + 6.0), rect.max)
}

enum Action {
    Apply(String),
    Drag(String),
}

type Thumb<'a> = dyn FnMut(&mut FilmcraftApp, &mut egui::Ui, &Card, Rect, bool) + 'a;

/// Lays the cards out: Favorites first, then one section per group, in a vertical scroll area.
/// `single`: one click applies a card (the Tools), otherwise a double click does.
fn grid(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect, cards: &[Card], hint: &str, single: bool, thumb: &mut Thumb<'_>) -> Option<Action> {
    let t = app.tokens;
    let hint_h = 22.0;
    let area = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.max.y - hint_h));
    ui.painter().text(pos2(rect.center().x, rect.max.y - hint_h / 2.0), Align2::CENTER_CENTER, hint, Tokens::ui(11.0), t.text_faint);
    let mut action = None;
    let mut gui = ui.new_child(egui::UiBuilder::new().max_rect(area).id_salt("kurgu-grid"));
    gui.set_clip_rect(area.intersect(ui.clip_rect()));
    if cards.is_empty() {
        gui.painter().text(area.center(), Align2::CENTER_CENTER, tl!("No results"), Tokens::ui(13.0), t.text_dim);
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
                    if let Some(a) = draw_card(app, ui, card, cr, salt, single, thumb) {
                        action = Some(a);
                    }
                }
            }
        };
        if !favorites.is_empty() {
            section(ui, app, &format!("★ {}", tl!("Favorites")), &favorites, "fav");
        }
        for g in &groups {
            let list: Vec<&Card> = cards.iter().filter(|c| c.group == *g).collect();
            section(ui, app, g, &list, g);
        }
    });
    action
}

/// One card: the thumbnail, its name, the favorite star; returns what the user did with it.
fn draw_card(app: &mut FilmcraftApp, ui: &mut egui::Ui, card: &Card, r: Rect, salt: &str, single: bool, thumb: &mut Thumb<'_>) -> Option<Action> {
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
            tl!("Remove from Favorites")
        } else {
            tl!("Add to Favorites")
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
    if resp.double_clicked() || (single && resp.clicked()) {
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
fn render_effect(id: &str, transition: bool, step: usize, base: &Image) -> Option<Image> {
    let def = filmcraft_project::effect_defs().iter().find(|d| d.id == id)?;
    let inst = def.instance();
    let a = base.clone();
    if transition {
        let b = second_picture(&a);
        let p = step as f32 / (STEPS - 1) as f32;
        return Some(filmcraft_render::transitions::apply(&inst, &a, &b, p));
    }
    let mut img = a;
    filmcraft_render::effects::apply(&mut img, &inst, &fx_ctx());
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

// ---------------------------------------------------------------- Sample picture

/// The picture the previews are drawn on: a frame of the clip selected in the timeline (or of the
/// video selected in Media), so an effect is shown on the user's own footage; FilmCraft's
/// procedural landscape when nothing with a picture is selected or its frame is not decoded yet.
#[derive(Clone)]
pub struct Sample {
    /// Part of every texture name, so the cards are drawn again when the picture changes.
    tag: String,
    img: std::sync::Arc<Image>,
}

/// Width the sample frame is asked for before it is cropped to the card.
const SAMPLE_REQ_W: u32 = 320;

pub fn sample(app: &mut FilmcraftApp, ctx: &egui::Context) -> Sample {
    let cache_id = egui::Id::unique("kurgu-sample-picture");
    let fallback = || {
        let img = ctx.data_mut(|d| d.get_temp::<std::sync::Arc<Image>>(egui::Id::unique("kurgu-sample-landscape")));
        let img = img.unwrap_or_else(|| {
            let i = std::sync::Arc::new(lp::preview_image(THUMB_W, THUMB_H));
            ctx.data_mut(|d| d.insert_temp(egui::Id::unique("kurgu-sample-landscape"), i.clone()));
            i
        });
        Sample { tag: "demo".into(), img }
    };
    // the item and media time to show
    let mut pick: Option<(filmcraft_project::ItemId, filmcraft_time::Tick)> = None;
    if let Some(seq) = app.session.active_sequence() {
        for clip in &app.session.state.selection {
            if let Some((_, it)) = seq.find_item(*clip) {
                let has_video = app.session.project.item(it.item).is_some_and(|p| p.has_video());
                if has_video {
                    let half = filmcraft_time::Tick(it.duration.0 / 2);
                    pick = Some((it.item, filmcraft_time::Tick(it.source_in.0.saturating_add(half.0.max(0)))));
                    break;
                }
            }
        }
    }
    if pick.is_none() {
        pick = app.session.state.project_selection.iter().find_map(|i| {
            let it = app.session.project.item(*i)?;
            it.has_video().then(|| (*i, filmcraft_time::Tick(it.duration().0 / 3)))
        });
    }
    let Some((item, at)) = pick else { return fallback() };
    let Some(pitem) = app.session.project.item(item) else { return fallback() };
    let rate = pitem.frame_rate();
    let src_w = pitem.as_media().and_then(|m| m.info.video.as_ref()).map_or(1920, |v| v.width).max(1);
    let at = at.min(filmcraft_time::Tick(pitem.duration().0.saturating_sub(1).max(0)));
    let frame = rate.frame_at(crate::panels::project::quantize(at));
    let key = crate::frames::FrameKey { target: crate::frames::Target::Item(item), frame, size: SAMPLE_REQ_W, revision: app.item_revision(item), draft: false };
    let tag = format!("{}-{}-{}", item.0, frame, key.revision);
    if let Some(s) = ctx.data_mut(|d| d.get_temp::<Sample>(cache_id)).filter(|s| s.tag == tag) {
        return s;
    }
    let Some(rgba) = app.frames.get(&key) else {
        let proj = app.session.project.clone();
        app.frames.request(key, rate.tick_of(frame), SAMPLE_REQ_W as f32 / src_w as f32, &proj, 40);
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
        return fallback();
    };
    let Some(img) = fill_card(&rgba) else { return fallback() };
    let s = Sample { tag, img: std::sync::Arc::new(img) };
    ctx.data_mut(|d| d.insert_temp(cache_id, s.clone()));
    s
}

/// A decoded frame scaled and cropped to fill a card, as linear light.
fn fill_card(f: &crate::frames::Rgba) -> Option<Image> {
    if f.w == 0 || f.h == 0 || f.px.len() < f.w.checked_mul(f.h)?.checked_mul(4)? {
        return None;
    }
    let mut src = Image::new(f.w, f.h);
    for (d, s) in src.px.as_chunks_mut::<4>().0.iter_mut().zip(f.px.as_chunks::<4>().0) {
        d[0] = filmcraft_color::srgb_to_linear(f32::from(s[0]) / 255.0);
        d[1] = filmcraft_color::srgb_to_linear(f32::from(s[1]) / 255.0);
        d[2] = filmcraft_color::srgb_to_linear(f32::from(s[2]) / 255.0);
        d[3] = 1.0;
    }
    // cover the card: the larger scale, centred
    let k = (THUMB_W as f32 / f.w as f32).max(THUMB_H as f32 / f.h as f32);
    let (ox, oy) = ((f.w as f32 * k - THUMB_W as f32) / 2.0, (f.h as f32 * k - THUMB_H as f32) / 2.0);
    let mut out = Image::new(THUMB_W, THUMB_H);
    for y in 0..THUMB_H {
        for x in 0..THUMB_W {
            let (sx, sy) = ((x as f32 + 0.5 + ox) / k - 0.5, (y as f32 + 0.5 + oy) / k - 0.5);
            let p = src.sample_bilinear_clamped(sx, sy);
            let i = (y * THUMB_W + x) * 4;
            if let Some(d) = out.px.get_mut(i..i + 4) {
                d.copy_from_slice(&[p[0], p[1], p[2], 1.0]);
            }
        }
    }
    Some(out)
}

/// A color look over the sample picture.
fn render_look(p: &LumetriPreset, base: &Image) -> Image {
    let mut img = base.clone();
    filmcraft_render::effects::apply(&mut img, &p.instance(), &fx_ctx());
    img
}

fn fx_ctx() -> filmcraft_render::effects::FxCtx<'static> {
    filmcraft_render::effects::FxCtx {
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
    }
}

// ---------------------------------------------------------------- Tools

/// One of the Tools: an effect applied to the selected clips (its settings open on the right), a
/// menu command, or a tool still being prepared.
pub struct ToolDef {
    pub key: &'static str,
    pub label: &'static str,
    pub icon: Icon,
    pub group: &'static str,
    pub effect: Option<&'static str>,
    pub command: Option<&'static str>,
    pub soon: bool,
    /// Extra words it is found by in the search, English and Turkish.
    pub keywords: &'static str,
}

const fn tool(key: &'static str, label: &'static str, icon: Icon, group: &'static str, effect: &'static str, keywords: &'static str) -> ToolDef {
    ToolDef { key, label, icon, group, effect: Some(effect), command: None, soon: false, keywords }
}

pub const TOOLS: &[ToolDef] = &[
    tool(
        "tool:chroma",
        "Chroma Key",
        Icon::Eyedropper,
        "Background",
        "ultra_key",
        "chroma key green screen blue screen ultra key replace background krom anahtar anahtarı yeşil perde yeşil ekran mavi perde arka plan değiştir",
    ),
    tool("tool:colorkey", "Color Key", Icon::Eyedropper, "Background", "color_key", "color key remove color renk anahtarı rengi kaldır arka plan"),
    tool(
        "tool:lumakey",
        "Luma Key",
        Icon::Sun,
        "Background",
        "luma_key",
        "luma key brightness key remove black white parlaklık anahtarı siyahı kaldır beyazı kaldır",
    ),
    ToolDef {
        key: "tool:bgremove",
        label: "Background Removal",
        icon: Icon::Sparkle,
        group: "Background",
        effect: None,
        command: None,
        soon: super::takip::BACKGROUND_SOON,
        keywords: "background removal remove background person cutout arka plan kaldır arka planı sil kişi seç dekupe yapay zeka",
    },
    ToolDef {
        key: "tool:body",
        label: "Body Effects",
        icon: Icon::Sun,
        group: "Background",
        effect: None,
        command: None,
        soon: false,
        keywords: "body effects glow neon outline portrait blur background color pop spotlight orbit stars hearts vücut efektleri parlama ışıltı neon kenar portre arka planı bulanıklaştır renk vurgusu siyah beyaz spot ışık dönen yıldızlar kalpler",
    },
    ToolDef {
        key: "tool:teleport",
        label: "Teleport",
        icon: Icon::Sparkle,
        group: "Background",
        effect: None,
        command: None,
        soon: false,
        keywords: "teleport vanish appear disappear magic beam ışınlanma ışınla kaybol kaybolma belir belirme yok ol sihir büyü",
    },
    tool("tool:stabilize", "Stabilize", Icon::Camera, "Picture", "warp_stabilizer", "stabilize stabilizer warp shake sabitle sabitleme titreme sarsıntı"),
    tool("tool:crop", "Crop", Icon::Square, "Picture", "crop", "crop trim edges kırp kenar"),
    tool("tool:fliph", "Flip Horizontal", Icon::Slide, "Picture", "horizontal_flip", "flip mirror horizontal ayna çevir yatay"),
    tool("tool:flipv", "Flip Vertical", Icon::Rolling, "Picture", "vertical_flip", "flip vertical upside down dikey çevir ters"),
    tool("tool:blur", "Blur", Icon::Adjust, "Picture", "gaussian_blur", "blur soften gaussian bulanık bulanıklaştır flu"),
    tool("tool:mosaic", "Mosaic", Icon::Grid, "Picture", "mosaic", "mosaic pixelate censor hide face plate mozaik piksel sansür yüz gizle plaka"),
    tool("tool:sharpen", "Sharpen", Icon::Sparkle, "Picture", "sharpen", "sharpen detail keskinleştir netleştir"),
    tool("tool:glow", "Glow", Icon::Sun, "Style", "glow", "glow shine light parlama ışıma ışık"),
    tool("tool:shadow", "Drop Shadow", Icon::Square, "Style", "drop_shadow", "drop shadow gölge"),
    ToolDef {
        key: "tool:speed",
        label: "Speed",
        icon: Icon::Clock,
        group: "Motion and Time",
        effect: None,
        command: Some("clip.speedDuration"),
        soon: false,
        keywords: "speed duration slow motion fast reverse hız süre yavaş çekim hızlandır ters oynat geri",
    },
    ToolDef {
        key: "tool:track",
        label: "Object Tracking",
        icon: Icon::TrackMaskFwd,
        group: "Motion and Time",
        effect: None,
        command: None,
        soon: false,
        keywords: "object tracking motion tracking follow replace object with media picture nesneyi değiştir medya ile değiştir yerine koy blur plate license plate face nesne takibi takip izle hareket takibi plaka bulanıklaştır yüz gizle",
    },
];

fn tool_def(key: &str) -> Option<&'static ToolDef> {
    TOOLS.iter().find(|t| t.key == key)
}

fn tool_matches(t: &ToolDef, q: &str) -> bool {
    q.is_empty() || matches(q, &[crate::i18n::t(t.label), t.label, t.keywords, crate::i18n::t(t.group)])
}

/// The Tools as big cards; one click applies a tool to the selected clips.
pub fn tools(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let body = search(app, ui, rect);
    let q = app.ui.kurgu.search.trim().to_string();
    let cards: Vec<Card> = TOOLS
        .iter()
        .filter(|t| tool_matches(t, &q))
        .map(|t| Card { key: t.key.to_string(), label: crate::i18n::t(t.label).to_string(), group: crate::i18n::t(t.group).to_string() })
        .collect();
    let action = grid(app, ui, body, &cards, tl!("Select a clip in the timeline, then click a tool"), true, &mut |app, ui, card, r, hovered| {
        tool_tile(ui, &app.tokens, r, card, hovered);
    });
    if let Some(Action::Apply(key)) = action {
        apply_tool(app, ui.ctx(), &key);
    }
}

/// A tool's card picture: its icon on a tile, marked while the tool is being prepared.
fn tool_tile(ui: &egui::Ui, t: &Tokens, r: Rect, card: &Card, hovered: bool) {
    let p = ui.painter();
    p.rect_filled(r, CornerRadius::same(6), if hovered { t.hover } else { t.field_bg });
    let Some(def) = tool_def(&card.key) else { return };
    let s = (r.height() * 0.42).clamp(18.0, 56.0);
    icons::paint(p, Rect::from_center_size(r.center(), vec2(s, s)), def.icon, if def.soon { t.text_faint } else { t.accent });
    if def.soon {
        let badge = Rect::from_min_size(r.min + vec2(6.0, 6.0), vec2(58.0, 18.0));
        p.rect_filled(badge, CornerRadius::same(9), Color32::from_black_alpha(150));
        p.text(badge.center(), Align2::CENTER_CENTER, tl!("Soon"), Tokens::ui(10.5), Color32::WHITE);
    }
}

/// Runs a tool on the selected clips.
fn apply_tool(app: &mut FilmcraftApp, ctx: &egui::Context, key: &str) {
    let Some(def) = tool_def(key) else { return };
    if def.soon {
        app.ui.status = tlf!("{tool}: coming soon", tool = crate::i18n::t(def.label));
        return;
    }
    if def.key == "tool:track" {
        // the Tracking tab on the right walks through it
        app.ui.kurgu.right_tab = 2;
    } else if def.key == "tool:teleport" {
        // the right side's panel walks through it
        app.ui.kurgu.track.teleport = true;
        app.ui.kurgu.right_tab = 2;
    } else if def.key == "tool:bgremove" || def.key == "tool:body" {
        // so does the cut-out, with the body effects on it
        app.ui.kurgu.track.teleport = false;
        if app.ui.kurgu.track.target.is_none() {
            app.ui.kurgu.track.action = super::takip::BACKGROUND_ACTION;
        }
        app.ui.kurgu.right_tab = 2;
    } else if let Some(id) = def.effect {
        apply_effect(app, id);
    } else if let Some(cmd) = def.command {
        if app.session.state.selection.is_empty() {
            app.ui.status = tl!("Select a clip in the timeline first").to_string();
        } else if let Err(e) = crate::menus::invoke(app, ctx, cmd, json!({})) {
            app.ui.status = e;
        }
    }
}

// ---------------------------------------------------------------- search everywhere

/// What a search result is; decides its picture and what applying it does.
enum Kind<'a> {
    Tool,
    Look(&'a LumetriPreset),
    Effect { transition: bool, audio: bool },
    Template(&'a filmcraft_engine::graphic_templates::LibraryEntry, Library),
}

/// Results of the search in the top bar: tools, effects, transitions, texts, elements, intros and
/// color looks whose name (in either language), folder or keywords hold every word of `query`.
pub fn search_all(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect, query: &str) {
    let q = query.trim();
    let library = template_library(app, ui.ctx());
    let presets: Vec<LumetriPreset> = lp::presets();
    let mut cards: Vec<Card> = Vec::new();
    for t in TOOLS.iter().filter(|t| tool_matches(t, q)) {
        cards.push(Card { key: t.key.to_string(), label: crate::i18n::t(t.label).to_string(), group: crate::i18n::t("Tools").to_string() });
    }
    for (src, group) in
        [(Source::VideoTransitions, "Transitions"), (Source::VideoEffects, "Effects"), (Source::AudioEffects, "Audio"), (Source::AudioTransitions, "Audio")]
    {
        for d in filmcraft_project::effect_defs().iter().filter(|d| src.includes(d)) {
            let folder = d.category.get(1).copied().unwrap_or("");
            if matches(q, &[crate::i18n::t(d.name), d.name, d.id, crate::i18n::t(folder), folder]) {
                cards.push(Card { key: d.id.to_string(), label: crate::i18n::t(d.name).to_string(), group: crate::i18n::t(group).to_string() });
            }
        }
    }
    for (lib, group) in [(Library::Texts, "Text"), (Library::Elements, "Elements"), (Library::Intros, "Intros")] {
        for e in library.iter().filter(|e| lib.includes(&e.template.id)) {
            let (name, cat) = (translate_parts(&e.template.name), translate_parts(&e.template.category));
            if matches(q, &[&name, &cat, &e.template.name, &e.template.category]) {
                cards.push(Card { key: e.template.id.clone(), label: name, group: crate::i18n::t(group).to_string() });
            }
        }
    }
    for p in &presets {
        if matches(q, &[crate::i18n::t(p.name), p.name, crate::i18n::t(p.folder), "color look renk görünüm"]) {
            cards.push(Card { key: format!("{LOOK}{}", p.name), label: crate::i18n::t(p.name).to_string(), group: crate::i18n::t("Looks").to_string() });
        }
    }
    let kind_of = |key: &str| -> Option<Kind<'_>> {
        if tool_def(key).is_some() {
            return Some(Kind::Tool);
        }
        if let Some(name) = key.strip_prefix(LOOK) {
            return presets.iter().find(|p| p.name == name).map(Kind::Look);
        }
        if let Some(d) = filmcraft_project::effect_defs().iter().find(|d| d.id == key) {
            let transition = matches!(d.kind, EffectKind::VideoTransition | EffectKind::AudioTransition);
            let audio = matches!(d.kind, EffectKind::Audio | EffectKind::AudioTransition);
            return Some(Kind::Effect { transition, audio });
        }
        let e = library.iter().find(|e| e.template.id == key)?;
        let lib = [Library::Elements, Library::Intros].into_iter().find(|l| l.includes(key)).unwrap_or(Library::Texts);
        Some(Kind::Template(e, lib))
    };
    let mut renders = 0usize;
    let smp = sample(app, ui.ctx());
    let action = grid(app, ui, rect, &cards, tl!("Double-click to use · or drag onto the timeline"), false, &mut |app, ui, card, r, hovered| {
        let Some(kind) = kind_of(&card.key) else { return };
        let (name, render): (String, Box<dyn Fn() -> Option<Image> + '_>) = match kind {
            Kind::Tool => {
                tool_tile(ui, &app.tokens, r, card, hovered);
                return;
            }
            Kind::Effect { audio: true, transition } => {
                audio_tile(ui, &app.tokens, r, transition);
                return;
            }
            Kind::Effect { transition, .. } => {
                let step = if transition { animation_step(ui, hovered) } else { STEPS / 2 };
                let key = card.key.clone();
                let base = smp.img.clone();
                (format!("kurgu-fx-{}-{step}-{}", card.key, smp.tag), Box::new(move || render_effect(&key, transition, step, &base)))
            }
            Kind::Look(p) => {
                let base = smp.img.clone();
                (format!("kurgu-look-{}-{}", card.key, smp.tag), Box::new(move || Some(render_look(p, &base))))
            }
            Kind::Template(e, lib) => {
                let step = text_step(ui, hovered);
                let name = format!("kurgu-tpl-{}-{}-{step}-{}", card.key, crate::i18n::current().bcp47(), smp.tag);
                let base = smp.img.clone();
                (name, Box::new(move || Some(render_template(&e.template, step, lib.zoom(), lib.span(), &base))))
            }
        };
        let tex = match app.texture_existing(&name) {
            Some((id, _)) => Some(id),
            None if renders < RENDERS_PER_FRAME => {
                renders += 1;
                render().map(|img| upload(app, ui.ctx(), &name, &img))
            }
            None => {
                ui.ctx().request_repaint();
                None
            }
        };
        paint_thumb(ui, &app.tokens, r, tex);
    });
    match action {
        Some(Action::Apply(key)) => match kind_of(&key) {
            Some(Kind::Tool) => apply_tool(app, ui.ctx(), &key),
            Some(Kind::Look(p)) => {
                if let Err(e) = app.session.execute("lumetri.applyPreset", json!({"name": p.name})) {
                    app.ui.status = e.to_string();
                }
            }
            Some(Kind::Effect { .. }) => apply_effect(app, &key),
            Some(Kind::Template(..)) => {
                if let Err(e) = app.session.execute("graphics.template.apply", json!({"template": key})) {
                    app.ui.status = e.to_string();
                }
            }
            None => {}
        },
        Some(Action::Drag(key)) => match kind_of(&key) {
            Some(Kind::Look(p)) => crate::panels::start_drag_lumetri_preset(ui, p.name),
            Some(Kind::Effect { .. }) => crate::panels::start_drag_effect(ui, &key),
            Some(Kind::Template(e, _)) => crate::panels::start_drag_template(ui, &key, &e.template.name),
            _ => {}
        },
        None => {}
    }
}
