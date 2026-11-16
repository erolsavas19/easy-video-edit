//! VideoKurgu: a simple, visual layout over FilmCraft's engine and panels.
//!
//! The window keeps one fixed arrangement instead of FilmCraft's dockable workspaces: a top bar,
//! a rail of categories on the left (Media, Text, Transitions, Effects, Color, Audio, Voiceover,
//! Elements, Intros, Tools), the chosen category's content next to it, one preview monitor in the
//! middle, the selection's properties on the right and the timeline at the bottom. Each part is
//! drawn by FilmCraft's own panel code (`panels::show`), so every feature keeps working; the
//! dividers between the parts can be dragged. "Gelişmiş Mod" switches back to FilmCraft's full
//! workspace layout.

use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use serde::{Deserialize, Serialize};

use crate::FilmcraftApp;
use crate::dock::PanelKind;
use crate::icons::{self, Icon};
use crate::state::Mode;
use crate::theme::{ThemeKind, Tokens};

/// Height of VideoKurgu's top bar (it takes the place of FilmCraft's header).
pub const TOP_BAR_H: f32 = 44.0;
const RAIL_W: f32 = 76.0;
const DIVIDER: f32 = 6.0;

/// The categories of the left rail, in order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Category {
    #[default]
    Media,
    Text,
    Transitions,
    Effects,
    Color,
    Audio,
    Voiceover,
    Elements,
    Intros,
    Tools,
}

impl Category {
    pub const ALL: [Category; 10] = [
        Category::Media,
        Category::Text,
        Category::Transitions,
        Category::Effects,
        Category::Color,
        Category::Audio,
        Category::Voiceover,
        Category::Elements,
        Category::Intros,
        Category::Tools,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::Media => "Medya",
            Category::Text => "Yazılar",
            Category::Transitions => "Geçişler",
            Category::Effects => "Efektler",
            Category::Color => "Renk",
            Category::Audio => "Ses",
            Category::Voiceover => "Seslendir",
            Category::Elements => "Öğeler",
            Category::Intros => "İntrolar",
            Category::Tools => "Araçlar",
        }
    }

    fn icon(self) -> Icon {
        match self {
            Category::Media => Icon::Folder,
            Category::Text => Icon::Type,
            Category::Transitions => Icon::Nest,
            Category::Effects => Icon::Fx,
            Category::Color => Icon::Adjust,
            Category::Audio => Icon::Speaker,
            Category::Voiceover => Icon::Mic,
            Category::Elements => Icon::Image,
            Category::Intros => Icon::Film,
            Category::Tools => Icon::Wrench,
        }
    }

    /// The FilmCraft panel that fills the category's content area, `None` while the category has
    /// no content of its own yet.
    fn panel(self) -> Option<PanelKind> {
        match self {
            Category::Media => Some(PanelKind::Project),
            Category::Text => Some(PanelKind::EssentialGraphics),
            Category::Transitions | Category::Effects => Some(PanelKind::Effects),
            Category::Color => Some(PanelKind::LumetriColor),
            Category::Audio => Some(PanelKind::EssentialSound),
            Category::Voiceover => Some(PanelKind::TextToSpeech),
            Category::Tools => Some(PanelKind::EffectControls),
            Category::Elements | Category::Intros => None,
        }
    }
}

/// VideoKurgu's layout state, saved with the rest of the UI state.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct KurguState {
    /// VideoKurgu's layout is on; off shows FilmCraft's workspaces ("Gelişmiş Mod").
    pub enabled: bool,
    pub category: Category,
    /// Width of the category content area (points).
    pub content_w: f32,
    /// Width of the properties area on the right (points).
    pub props_w: f32,
    /// Share of the body height the timeline takes (0..1).
    pub timeline_share: f32,
}

impl Default for KurguState {
    fn default() -> Self {
        KurguState { enabled: true, category: Category::Media, content_w: 340.0, props_w: 300.0, timeline_share: 0.36 }
    }
}

/// The top bar: name, mode buttons, theme switch, advanced mode and export.
pub fn top_bar(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    let p = ui.painter().clone();
    p.rect_filled(rect, 0.0, t.header_bg);
    p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0, t.separator));
    // Name, with the accent on "Kurgu".
    let x = rect.min.x + 16.0;
    let cy = rect.center().y;
    let video = p.text(pos2(x, cy), Align2::LEFT_CENTER, "Video", Tokens::ui(17.0), t.text);
    p.text(pos2(video.max.x, cy), Align2::LEFT_CENTER, "Kurgu", Tokens::ui(17.0), t.accent);

    // Right side, from the right edge leftwards.
    let mut right = rect.max.x - 12.0;
    let export_label = if app.ui.mode == Mode::Export { "Düzenlemeye Dön" } else { "Dışa Aktar" };
    if pill(ui, &t, &mut right, cy, export_label, true).clicked() {
        app.ui.mode = if app.ui.mode == Mode::Export { Mode::Edit } else { Mode::Export };
    }
    if pill(ui, &t, &mut right, cy, "Gelişmiş Mod", false).on_hover_text("FilmCraft'ın tüm panelleri ve çalışma alanları").clicked() {
        app.ui.kurgu.enabled = false;
        app.ui.mode = Mode::Edit;
    }
    let light = app.tokens.kind == ThemeKind::Light;
    let theme_label = if light { "Koyu Tema" } else { "Açık Tema" };
    if pill(ui, &t, &mut right, cy, theme_label, false).clicked() {
        let ctx = ui.ctx().clone();
        crate::panels::settings::set_theme(app, &ctx, if light { ThemeKind::Dark } else { ThemeKind::Light });
    }
    if app.ui.mode == Mode::Edit && pill(ui, &t, &mut right, cy, "İçe Aktar", false).clicked() {
        app.ui.mode = Mode::Import;
    }
    if app.ui.mode == Mode::Import && pill(ui, &t, &mut right, cy, "Düzenlemeye Dön", false).clicked() {
        app.ui.mode = Mode::Edit;
    }
}

/// A rounded button laid out right to left: draws it left of `right` and moves `right` past it.
fn pill(ui: &mut egui::Ui, t: &Tokens, right: &mut f32, cy: f32, label: &str, primary: bool) -> egui::Response {
    let font = Tokens::ui(12.5);
    let w = ui.fonts_mut(|f| f.layout_no_wrap(label.to_string(), font.clone(), t.text).size().x) + 24.0;
    let rect = Rect::from_center_size(pos2(*right - w / 2.0, cy), vec2(w, 28.0));
    *right -= w + 8.0;
    let resp = ui.interact(rect, egui::Id::unique(("kurgu-pill", label)), Sense::click());
    let (fill, text) = if primary {
        (if resp.hovered() { t.accent_hover } else { t.accent }, Color32::WHITE)
    } else {
        (if resp.hovered() { t.hover } else { t.field_bg }, t.text)
    };
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(14), fill);
    if !primary {
        p.rect_stroke(rect, CornerRadius::same(14), Stroke::new(1.0, t.field_border), StrokeKind::Inside);
    }
    p.text(rect.center(), Align2::CENTER_CENTER, label, font, text);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The editing body: rail, category content, preview, properties and timeline.
pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, body: Rect) {
    let t = app.tokens;
    let rail = Rect::from_min_size(body.min, vec2(RAIL_W, body.height()));
    draw_rail(app, ui, rail);

    let rest = Rect::from_min_max(pos2(rail.max.x + 1.0, body.min.y), body.max);
    // Timeline at the bottom, across everything right of the rail.
    let share = app.ui.kurgu.timeline_share.clamp(0.18, 0.7);
    let timeline_h = (rest.height() * share).round();
    let top = Rect::from_min_max(rest.min, pos2(rest.max.x, rest.max.y - timeline_h - DIVIDER));
    let timeline = Rect::from_min_max(pos2(rest.min.x, top.max.y + DIVIDER), rest.max);

    // Content | preview | properties across the top part.
    let max_side = (top.width() - 360.0).max(400.0) / 2.0;
    let content_w = app.ui.kurgu.content_w.clamp(220.0, max_side);
    let props_w = app.ui.kurgu.props_w.clamp(200.0, max_side);
    let content = Rect::from_min_max(top.min, pos2(top.min.x + content_w, top.max.y));
    let props = Rect::from_min_max(pos2(top.max.x - props_w, top.min.y), top.max);
    let preview = Rect::from_min_max(pos2(content.max.x + DIVIDER, top.min.y), pos2(props.min.x - DIVIDER, top.max.y));

    draw_content(app, ui, content);
    card(ui, &t, preview);
    panel(app, ui, PanelKind::Program, preview.shrink(1.0));
    card(ui, &t, props);
    let props_inner = titled(ui, &t, props, "Özellikler");
    panel(app, ui, PanelKind::Properties, props_inner);
    card(ui, &t, timeline);
    panel(app, ui, PanelKind::Timeline, timeline.shrink(1.0));

    // Dividers: content|preview, preview|props, top|timeline.
    let d1 = Rect::from_min_max(pos2(content.max.x, top.min.y), pos2(preview.min.x, top.max.y));
    if let Some(dx) = divider(ui, d1, "kurgu-div-content", false) {
        app.ui.kurgu.content_w = (content_w + dx).clamp(220.0, max_side);
    }
    let d2 = Rect::from_min_max(pos2(preview.max.x, top.min.y), pos2(props.min.x, top.max.y));
    if let Some(dx) = divider(ui, d2, "kurgu-div-props", false) {
        app.ui.kurgu.props_w = (props_w - dx).clamp(200.0, max_side);
    }
    let d3 = Rect::from_min_max(pos2(rest.min.x, top.max.y), pos2(rest.max.x, timeline.min.y));
    if let Some(dy) = divider(ui, d3, "kurgu-div-timeline", true)
        && rest.height() > 0.0
    {
        app.ui.kurgu.timeline_share = ((timeline_h - dy) / rest.height()).clamp(0.18, 0.7);
    }
}

/// The left rail of categories.
fn draw_rail(app: &mut FilmcraftApp, ui: &mut egui::Ui, rail: Rect) {
    let t = app.tokens;
    ui.painter().rect_filled(rail, 0.0, t.header_bg);
    let item_h = ((rail.height() - 8.0) / Category::ALL.len() as f32).clamp(44.0, 64.0);
    for (i, cat) in Category::ALL.into_iter().enumerate() {
        let r = Rect::from_min_size(pos2(rail.min.x + 6.0, rail.min.y + 6.0 + i as f32 * item_h), vec2(rail.width() - 12.0, item_h - 4.0));
        let resp = ui.interact(r, egui::Id::unique(("kurgu-rail", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let active = app.ui.kurgu.category == cat;
        let p = ui.painter();
        if active {
            p.rect_filled(r, CornerRadius::same(10), t.pill_active_bg);
        } else if resp.hovered() {
            p.rect_filled(r, CornerRadius::same(10), t.hover);
        }
        let color = if active { t.accent } else { t.icon };
        let icon_rect = Rect::from_center_size(pos2(r.center().x, r.min.y + r.height() * 0.38), vec2(22.0, 22.0));
        icons::paint(p, icon_rect, cat.icon(), color);
        p.text(pos2(r.center().x, r.max.y - 9.0), Align2::CENTER_CENTER, cat.label(), Tokens::ui(11.0), if active { t.text } else { t.text_dim });
        if resp.clicked() {
            select(app, cat);
        }
    }
}

/// Opens a category: Transitions and Effects share FilmCraft's Effects panel, filtered.
fn select(app: &mut FilmcraftApp, cat: Category) {
    let was = app.ui.kurgu.category;
    app.ui.kurgu.category = cat;
    match cat {
        Category::Transitions => app.ui.effects_search = "Transition".into(),
        Category::Effects if was == Category::Transitions => app.ui.effects_search.clear(),
        _ => {}
    }
    if let Some(kind) = cat.panel() {
        app.ui.focused = kind;
    }
}

/// The chosen category's content: a title and FilmCraft's panel, or a "coming soon" card.
fn draw_content(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    card(ui, &t, rect);
    let cat = app.ui.kurgu.category;
    let inner = titled(ui, &t, rect, cat.label());
    match cat.panel() {
        Some(kind) => panel(app, ui, kind, inner),
        None => coming_soon(ui, &t, inner, cat),
    }
}

fn coming_soon(ui: &mut egui::Ui, t: &Tokens, rect: Rect, cat: Category) {
    let text = match cat {
        Category::Elements => "Emojiler, çıkartmalar, \"abone ol\" butonları ve süslemeler\nburada olacak.",
        _ => "Renkleri, yazıları ve şekilleri düzenlenebilen\nhazır introlar burada olacak.",
    };
    let p = ui.painter();
    icons::paint(p, Rect::from_center_size(rect.center() - vec2(0.0, 40.0), vec2(40.0, 40.0)), cat.icon(), t.text_faint);
    p.text(rect.center() + vec2(0.0, 10.0), Align2::CENTER_TOP, "Yakında", Tokens::ui(15.0), t.text);
    p.text(rect.center() + vec2(0.0, 34.0), Align2::CENTER_TOP, text, Tokens::ui(12.0), t.text_dim);
}

/// A rounded card behind one part of the layout.
fn card(ui: &mut egui::Ui, t: &Tokens, rect: Rect) {
    ui.painter().rect_filled(rect, CornerRadius::same(8), t.panel_bg);
}

/// Draws a title strip at the top of `rect` and returns the area under it.
fn titled(ui: &mut egui::Ui, t: &Tokens, rect: Rect, title: &str) -> Rect {
    let strip = Rect::from_min_size(rect.min, vec2(rect.width(), 30.0));
    let p = ui.painter();
    p.text(pos2(strip.min.x + 12.0, strip.center().y), Align2::LEFT_CENTER, title, Tokens::ui(13.0), t.text);
    p.line_segment([pos2(strip.min.x + 8.0, strip.max.y), pos2(strip.max.x - 8.0, strip.max.y)], Stroke::new(1.0, t.separator));
    Rect::from_min_max(pos2(rect.min.x + 1.0, strip.max.y + 1.0), pos2(rect.max.x - 1.0, rect.max.y - 1.0))
}

/// Draws a FilmCraft panel into `rect`, in a child UI clipped to it; a press inside focuses it,
/// so its keyboard shortcuts follow the part the user works in.
fn panel(app: &mut FilmcraftApp, ui: &mut egui::Ui, kind: PanelKind, rect: Rect) {
    if rect.width() < 20.0 || rect.height() < 20.0 {
        return;
    }
    if ui.input(|i| i.pointer.primary_pressed() && i.pointer.interact_pos().is_some_and(|p: Pos2| rect.contains(p))) {
        app.ui.focused = kind;
    }
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).id_salt(("kurgu-panel", kind as u8)));
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    crate::panels::show(app, &mut child, kind, rect);
}

/// A draggable divider; returns the drag delta along its axis while it is dragged.
fn divider(ui: &mut egui::Ui, rect: Rect, id: &str, horizontal: bool) -> Option<f32> {
    let resp = ui.interact(rect, egui::Id::unique(id), Sense::drag());
    let cursor = if horizontal { egui::CursorIcon::ResizeVertical } else { egui::CursorIcon::ResizeHorizontal };
    let resp = resp.on_hover_cursor(cursor);
    if resp.hovered() || resp.dragged() {
        let c = rect.center();
        let (a, b) = if horizontal { (pos2(c.x - 18.0, c.y), pos2(c.x + 18.0, c.y)) } else { (pos2(c.x, c.y - 18.0), pos2(c.x, c.y + 18.0)) };
        ui.painter().line_segment([a, b], Stroke::new(3.0, ui.visuals().widgets.hovered.fg_stroke.color));
    }
    resp.dragged().then(|| if horizontal { resp.drag_delta().y } else { resp.drag_delta().x })
}
