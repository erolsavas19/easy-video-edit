//! Easy Video Edit: a simple, visual layout over FilmCraft's engine and panels.
//!
//! The window keeps one fixed arrangement instead of FilmCraft's dockable workspaces: a top bar,
//! a rail of categories on the left (Media, Text, Transitions, Effects, Color, Audio, Voiceover,
//! Elements, Intros, Tools), the chosen category's content next to it, one preview monitor in the
//! middle, the selection's properties on the right and the timeline at the bottom. Each part is
//! drawn by FilmCraft's own panel code (`panels::show`), so every feature keeps working; the
//! dividers between the parts can be dragged. "Advanced Mode" switches back to FilmCraft's full
//! workspace layout.

use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use serde::{Deserialize, Serialize};

use crate::FilmcraftApp;
use crate::dock::PanelKind;
use crate::icons::{self, Icon};
use crate::state::{Mode, Tool};
use crate::theme::{ThemeKind, Tokens};

mod galeri;

/// Height of Easy Video Edit's top bar (it takes the place of FilmCraft's header).
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
            Category::Media => "Media",
            Category::Text => "Text",
            Category::Transitions => "Transitions",
            Category::Effects => "Effects",
            Category::Color => "Color",
            Category::Audio => "Audio",
            Category::Voiceover => "Voiceover",
            Category::Elements => "Elements",
            Category::Intros => "Intros",
            Category::Tools => "Tools",
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
            Category::Transitions | Category::Effects | Category::Color | Category::Audio => None,
            Category::Voiceover => Some(PanelKind::TextToSpeech),
            Category::Tools | Category::Elements | Category::Intros => None,
        }
    }
}

/// Easy Video Edit's layout state, saved with the rest of the UI state.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct KurguState {
    /// Easy Video Edit's layout is on; off shows FilmCraft's workspaces ("Advanced Mode").
    pub enabled: bool,
    pub category: Category,
    /// Width of the category content area (points).
    pub content_w: f32,
    /// Width of the properties area on the right (points).
    pub props_w: f32,
    /// Share of the body height the timeline takes (0..1).
    pub timeline_share: f32,
    /// Favorite effects, transitions (their ids) and color looks (`lumetri:<name>`).
    pub favorites: Vec<String>,
    /// The galleries' search text.
    #[serde(skip)]
    pub search: String,
    /// The card selected last in a gallery.
    #[serde(skip)]
    pub selected: Option<String>,
    /// Audio: Effects, Transitions or Sound settings.
    pub audio_tab: u8,
    /// Color: Looks or Adjust.
    pub color_tab: u8,
    /// The right side: Properties (0) or Effect Settings (1).
    pub right_tab: u8,
    /// The search in the top bar, across every category; while it holds text the results take
    /// the place of the category's content.
    #[serde(skip)]
    pub global_search: String,
}

impl Default for KurguState {
    fn default() -> Self {
        KurguState {
            enabled: true,
            category: Category::Media,
            content_w: 360.0,
            props_w: 300.0,
            timeline_share: 0.36,
            favorites: Vec::new(),
            search: String::new(),
            selected: None,
            audio_tab: 0,
            color_tab: 0,
            right_tab: 0,
            global_search: String::new(),
        }
    }
}

/// The top bar: name, mode buttons, theme switch, advanced mode and export.
pub fn top_bar(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    let p = ui.painter().clone();
    p.rect_filled(rect, 0.0, t.header_bg);
    p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0, t.separator));
    // Name, with the accent on "Easy".
    let x = rect.min.x + 16.0;
    let cy = rect.center().y;
    let easy = p.text(pos2(x, cy), Align2::LEFT_CENTER, "Easy", Tokens::ui(17.0), t.accent);
    let name = p.text(pos2(easy.max.x + 5.0, cy), Align2::LEFT_CENTER, "Video Edit", Tokens::ui(17.0), t.text);

    // Left side, after the name: the File menu, then Undo and Redo.
    let mut left = name.max.x + 22.0;
    let file = pill_left(ui, &t, &mut left, cy, tl!("File"));
    let mut chosen: Option<String> = None;
    egui::Popup::menu(&file).show(|ui| file_menu(app, ui, &mut chosen));
    left += 6.0;
    for (id, icon, label) in [("edit.undo", Icon::Undo, tl!("Undo")), ("edit.redo", Icon::Redo, tl!("Redo"))] {
        let enabled = filmcraft_engine::find_command(id).is_none_or(|c| (c.enabled)(&app.session).is_ok());
        let r = Rect::from_center_size(pos2(left + 14.0, cy), vec2(28.0, 28.0));
        left += 32.0;
        let resp = ui.interact(r, egui::Id::unique(("kurgu-edit", id)), if enabled { Sense::click() } else { Sense::hover() }).on_hover_text(label);
        if enabled && resp.hovered() {
            ui.painter().rect_filled(r, CornerRadius::same(6), t.hover);
        }
        icons::paint(
            ui.painter(),
            r.shrink(6.0),
            icon,
            if !enabled {
                t.text_faint
            } else if resp.hovered() {
                t.text
            } else {
                t.icon
            },
        );
        if enabled && resp.clicked() {
            chosen = Some(id.to_string());
        }
    }
    if let Some(id) = chosen {
        run(app, ui.ctx(), &id);
    }
    // The project's name after them, marked while it has unsaved changes; a new project's
    // engine-given name "Untitled" shows in the interface language.
    let project = if app.session.project.name == "Untitled" { tl!("Untitled").to_string() } else { app.session.project.name.clone() };
    let title = if app.session.is_dirty() { tlf!("{name} - Edited", name = project) } else { project };
    ui.painter().text(pos2(left + 10.0, cy), Align2::LEFT_CENTER, title, Tokens::ui(12.0), t.text_dim);

    // The search across everything (tools, effects, transitions, texts…) in the middle.
    let sw = (rect.width() * 0.24).clamp(180.0, 340.0);
    let sr = Rect::from_center_size(pos2(rect.center().x + 40.0, cy), vec2(sw, 26.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(sr).id_salt("kurgu-global-search"));
    let mut q = app.ui.kurgu.global_search.clone();
    crate::widgets::search_field(&mut child, &mut q, tl!("Search: chroma, stabilize, blur, title…"), sr.width(), &t);
    app.ui.kurgu.global_search = q;

    // Right side, from the right edge leftwards.
    let mut right = rect.max.x - 12.0;
    let export_label = if app.ui.mode == Mode::Export { tl!("Back to Editing") } else { tl!("Export") };
    if pill(ui, &t, &mut right, cy, export_label, true).clicked() {
        app.ui.mode = if app.ui.mode == Mode::Export { Mode::Edit } else { Mode::Export };
    }
    // Advanced Mode (FilmCraft's full layout) is a quiet icon, not one of the main buttons.
    let adv = Rect::from_center_size(pos2(right - 14.0, cy), vec2(28.0, 28.0));
    right -= 36.0;
    let resp = ui
        .interact(adv, egui::Id::unique("kurgu-advanced"), Sense::click())
        .on_hover_text(format!("{}: {}", tl!("Advanced Mode"), tl!("All panels and workspaces")))
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if resp.hovered() {
        ui.painter().rect_filled(adv, CornerRadius::same(6), t.hover);
    }
    icons::paint(ui.painter(), adv.shrink(6.0), Icon::Workspaces, if resp.hovered() { t.text } else { t.text_dim });
    if resp.clicked() {
        app.ui.kurgu.enabled = false;
        app.ui.mode = Mode::Edit;
    }
    // Settings: proxies, hardware decoding and the full Settings window.
    let gear = Rect::from_center_size(pos2(right - 14.0, cy), vec2(28.0, 28.0));
    right -= 36.0;
    let gresp =
        ui.interact(gear, egui::Id::unique("kurgu-settings"), Sense::click()).on_hover_text(tl!("Settings")).on_hover_cursor(egui::CursorIcon::PointingHand);
    if gresp.hovered() {
        ui.painter().rect_filled(gear, CornerRadius::same(6), t.hover);
    }
    icons::paint(ui.painter(), gear.shrink(6.0), Icon::Gear, if gresp.hovered() { t.text } else { t.text_dim });
    egui::Popup::menu(&gresp).show(|ui| settings_menu(app, ui));
    let light = app.tokens.kind == ThemeKind::Light;
    let theme_label = if light { tl!("Dark Theme") } else { tl!("Light Theme") };
    if pill(ui, &t, &mut right, cy, theme_label, false).clicked() {
        let ctx = ui.ctx().clone();
        crate::panels::settings::set_theme(app, &ctx, if light { ThemeKind::Dark } else { ThemeKind::Light });
    }
    if app.ui.mode == Mode::Edit && pill(ui, &t, &mut right, cy, tl!("Import"), false).clicked() {
        app.ui.mode = Mode::Import;
    }
    if app.ui.mode == Mode::Import && pill(ui, &t, &mut right, cy, tl!("Back to Editing"), false).clicked() {
        app.ui.mode = Mode::Edit;
    }
}

/// In FilmCraft's full layout ("Advanced Mode"): a "Simple Mode" button at the left of FilmCraft's
/// header, which goes back to Easy Video Edit's layout. Returns the header area left for FilmCraft.
pub fn simple_mode_button(app: &mut FilmcraftApp, ui: &mut egui::Ui, header: Rect) -> Rect {
    let t = app.tokens;
    let w = 96.0;
    let strip = Rect::from_min_size(header.min, vec2(w + 12.0, header.height()));
    ui.painter().rect_filled(strip, 0.0, t.header_bg);
    let r = Rect::from_center_size(strip.center(), vec2(w, (header.height() - 10.0).max(20.0)));
    let resp = ui.interact(r, egui::Id::unique("kurgu-simple-mode"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    let fill = if resp.hovered() { t.accent_hover } else { t.accent };
    ui.painter().rect_filled(r, CornerRadius::same(12), fill);
    ui.painter().text(r.center(), Align2::CENTER_CENTER, tl!("Simple Mode"), Tokens::ui(12.0), Color32::WHITE);
    if resp.on_hover_text(tl!("Back to the simple layout")).clicked() {
        app.ui.kurgu.enabled = true;
        app.ui.mode = Mode::Edit;
    }
    Rect::from_min_max(pos2(strip.max.x, header.min.y), header.max)
}

/// The File menu of the top bar; the picked command goes to `chosen`, run once the menu is drawn.
fn file_menu(app: &FilmcraftApp, ui: &mut egui::Ui, chosen: &mut Option<String>) {
    ui.set_min_width(200.0);
    fn item(ui: &mut egui::Ui, chosen: &mut Option<String>, label: &str, id: &str) {
        if ui.button(label).clicked() {
            *chosen = Some(id.to_string());
            ui.close();
        }
    }
    item(ui, chosen, tl!("New Project…"), "file.newProject");
    item(ui, chosen, tl!("Open Project…"), "file.open");
    let recent = &app.session.prefs.general.recent_projects;
    ui.add_enabled_ui(!recent.is_empty(), |ui| {
        ui.menu_button(tl!("Open Recent"), |ui| {
            for (i, path) in recent.iter().enumerate().take(10) {
                let name = std::path::Path::new(path).file_stem().map_or_else(|| path.clone(), |s| s.to_string_lossy().into_owned());
                if ui.button(name).on_hover_text(path).clicked() {
                    *chosen = Some(format!("file.openRecent.{i}"));
                    ui.close();
                }
            }
        });
    });
    ui.separator();
    item(ui, chosen, tl!("Save"), "file.save");
    item(ui, chosen, tl!("Save As…"), "file.saveAs");
    ui.separator();
    item(ui, chosen, tl!("Import Media…"), "file.import");
}

/// The Settings menu of the top bar: proxies, hardware decoding and FilmCraft's Settings window.
fn settings_menu(app: &mut FilmcraftApp, ui: &mut egui::Ui) {
    ui.set_min_width(260.0);
    let mut proxies = app.session.prefs.media.enable_proxies;
    if ui.checkbox(&mut proxies, tl!("Use proxies (smoother playback)")).changed()
        && let Err(e) = app.session.execute("media.toggleProxies", serde_json::json!({"enabled": proxies}))
    {
        app.ui.status = e.to_string();
    }
    if ui.button(tl!("Create Proxies for Selected Media…")).clicked() {
        ui.close();
        run(app, ui.ctx(), "media.createProxies");
    }
    ui.separator();
    let mut hw = app.session.prefs.playback.hardware_decoding != "off";
    if ui
        .checkbox(&mut hw, tl!("Hardware-accelerated decoding (graphics card)"))
        .on_hover_text(tl!("Media already open keeps its decoder until it is reopened"))
        .changed()
    {
        let mut next = app.session.prefs.clone();
        next.playback.hardware_decoding = if hw { "auto" } else { "off" }.to_string();
        if let Err(e) = app.session.set_prefs(next) {
            app.ui.status = e.to_string();
        }
    }
    ui.separator();
    if ui.button(tl!("All Settings…")).clicked() {
        ui.close();
        app.dialog = Some(crate::Dialog::Preferences);
    }
}

/// Runs a menu command the way FilmCraft's menus do (with their file dialogs and "save changes?"
/// questions); a failure shows in the status bar.
fn run(app: &mut FilmcraftApp, ctx: &egui::Context, id: &str) {
    if let Err(e) = crate::menus::invoke(app, ctx, id, serde_json::json!({})) {
        app.ui.status = e;
    }
}

/// A rounded button laid out left to right: draws it right of `left` and moves `left` past it.
fn pill_left(ui: &mut egui::Ui, t: &Tokens, left: &mut f32, cy: f32, label: &str) -> egui::Response {
    let font = Tokens::ui(12.5);
    let w = ui.fonts_mut(|f| f.layout_no_wrap(label.to_string(), font.clone(), t.text).size().x) + 24.0;
    let rect = Rect::from_min_size(pos2(*left, cy - 14.0), vec2(w, 28.0));
    *left += w + 8.0;
    pill_at(ui, t, rect, label, false)
}

/// A rounded button laid out right to left: draws it left of `right` and moves `right` past it.
fn pill(ui: &mut egui::Ui, t: &Tokens, right: &mut f32, cy: f32, label: &str, primary: bool) -> egui::Response {
    let font = Tokens::ui(12.5);
    let w = ui.fonts_mut(|f| f.layout_no_wrap(label.to_string(), font.clone(), t.text).size().x) + 24.0;
    let rect = Rect::from_center_size(pos2(*right - w / 2.0, cy), vec2(w, 28.0));
    *right -= w + 8.0;
    pill_at(ui, t, rect, label, primary)
}

fn pill_at(ui: &mut egui::Ui, t: &Tokens, rect: Rect, label: &str, primary: bool) -> egui::Response {
    let font = Tokens::ui(12.5);
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
    // Templates are made with sample text in the interface language.
    filmcraft_project::kurgu_templates::set_turkish(app.ui.language == crate::i18n::Language::Tr);
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
    // Properties (position, scale, crop, volume) or Effect Settings (every effect on the clip: a
    // tool's or gallery effect's settings open here once it is applied).
    let mut tab = app.ui.kurgu.right_tab.min(1);
    let props_inner = galeri::tabs(ui, &t, props, &[tl!("Properties"), tl!("Effect Settings")], &mut tab);
    app.ui.kurgu.right_tab = tab;
    let props_inner = Rect::from_min_max(props_inner.min + vec2(1.0, 0.0), props_inner.max - vec2(1.0, 1.0));
    panel(app, ui, if tab == 0 { PanelKind::Properties } else { PanelKind::EffectControls }, props_inner);
    // The editing tools (Selection, Razor, …) in a strip left of the timeline: one column when the
    // timeline is tall enough for all of them, two when it is not.
    // Under them, "+" adds a layer (a video or audio track).
    let tools_need = Tool::groups().len() as f32 * 36.0 + 8.0 + LAYER_BTN;
    let tools_w = if timeline.height() >= tools_need { 42.0 } else { 80.0 };
    let tools = Rect::from_min_size(timeline.min, vec2(tools_w, timeline.height()));
    let timeline = Rect::from_min_max(pos2(tools.max.x + 4.0, timeline.min.y), timeline.max);
    card(ui, &t, tools);
    let tools_body = Rect::from_min_max(tools.min, pos2(tools.max.x, tools.max.y - LAYER_BTN));
    panel(app, ui, PanelKind::Tools, tools_body.shrink(1.0));
    add_layer_button(app, ui, Rect::from_min_max(pos2(tools.min.x, tools_body.max.y), tools.max));
    card(ui, &t, timeline);
    panel(app, ui, PanelKind::Timeline, timeline.shrink(1.0));
    // Where the panels are, for drop targets (a clip, effect or title dropped on the timeline).
    let layer = ui.layer_id();
    crate::FilmcraftApp::record_panel_bodies(
        ui.ctx(),
        vec![(PanelKind::Timeline, timeline.shrink(1.0), layer), (PanelKind::Program, preview.shrink(1.0), layer), (PanelKind::Properties, props_inner, layer)],
    );

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

/// Height of the "+" (add layer) button under the editing tools.
const LAYER_BTN: f32 = 34.0;

/// "+" under the editing tools: adds a video layer (a track above the others) or an audio layer.
fn add_layer_button(app: &mut FilmcraftApp, ui: &mut egui::Ui, area: Rect) {
    let t = app.tokens;
    let r = Rect::from_center_size(area.center(), vec2(30.0, 26.0));
    let has_seq = app.session.active_sequence().is_some();
    let resp = ui
        .interact(r, egui::Id::unique("kurgu-add-layer"), if has_seq { Sense::click() } else { Sense::hover() })
        .on_hover_text(tl!("Add Layer"))
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    app.auto.add("kurgu.addLayer", r, tl!("Add Layer"));
    let fill = if resp.hovered() && has_seq { t.accent } else { t.field_bg };
    ui.painter().rect_filled(r, CornerRadius::same(6), fill);
    let color = if !has_seq {
        t.text_dim
    } else if resp.hovered() {
        Color32::WHITE
    } else {
        t.text
    };
    icons::paint(ui.painter(), Rect::from_center_size(r.center(), vec2(16.0, 16.0)), Icon::Plus, color);
    let mut chosen: Option<serde_json::Value> = None;
    egui::Popup::menu(&resp).show(|ui| {
        if ui.button(tl!("Add Video Layer")).clicked() {
            chosen = Some(serde_json::json!({"video": 1, "audio": 0}));
        }
        if ui.button(tl!("Add Audio Layer")).clicked() {
            chosen = Some(serde_json::json!({"video": 0, "audio": 1}));
        }
    });
    if let Some(p) = chosen
        && let Err(e) = app.session.execute("sequence.addTracks", p)
    {
        app.ui.status = e.to_string();
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
        p.text(
            pos2(r.center().x, r.max.y - 9.0),
            Align2::CENTER_CENTER,
            crate::i18n::t(cat.label()),
            Tokens::ui(11.0),
            if active { t.text } else { t.text_dim },
        );
        if resp.clicked() {
            select(app, cat);
        }
    }
}

/// Opens a category: Transitions and Effects share FilmCraft's Effects panel, filtered.
fn select(app: &mut FilmcraftApp, cat: Category) {
    let was = app.ui.kurgu.category;
    app.ui.kurgu.category = cat;
    // picking a category leaves the search across everything
    app.ui.kurgu.global_search.clear();
    if was != cat {
        app.ui.kurgu.search.clear();
    }
    if let Some(kind) = cat.panel() {
        app.ui.focused = kind;
    }
}

/// The chosen category's content: a title and FilmCraft's panel, or a "coming soon" card.
fn draw_content(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    card(ui, &t, rect);
    let query = app.ui.kurgu.global_search.trim().to_string();
    if !query.is_empty() {
        let inner = titled(ui, &t, rect, &tlf!("Search: “{query}”", query = query));
        galeri::search_all(app, ui, inner, &query);
        return;
    }
    let cat = app.ui.kurgu.category;
    let mut inner = titled(ui, &t, rect, crate::i18n::t(cat.label()));
    if cat == Category::Media {
        add_to_timeline_button(app, ui, Rect::from_min_size(rect.min, vec2(rect.width(), 30.0)));
    }
    // How to add media to the timeline, along the bottom (the galleries show their own hint).
    let hint = (cat == Category::Media).then(|| tl!("Drag onto the timeline · or select and press “Add to Timeline”"));
    if let Some(hint) = hint
        && inner.height() > 120.0
    {
        let strip = Rect::from_min_max(pos2(inner.min.x, inner.max.y - 24.0), inner.max);
        ui.painter().line_segment([pos2(strip.min.x + 8.0, strip.min.y), pos2(strip.max.x - 8.0, strip.min.y)], Stroke::new(1.0, t.separator));
        let galley = ui.fonts_mut(|f| f.layout(hint.to_string(), Tokens::ui(11.0), t.text_dim, strip.width() - 16.0));
        ui.painter().galley(pos2(strip.min.x + 8.0, strip.center().y - galley.size().y / 2.0), galley, t.text_dim);
        inner.max.y = strip.min.y;
    }
    match cat {
        Category::Text => galeri::texts(app, ui, inner, galeri::Library::Texts),
        Category::Elements => galeri::texts(app, ui, inner, galeri::Library::Elements),
        Category::Intros => galeri::texts(app, ui, inner, galeri::Library::Intros),
        Category::Transitions => galeri::effects(app, ui, inner, galeri::Source::VideoTransitions),
        Category::Effects => galeri::effects(app, ui, inner, galeri::Source::VideoEffects),
        Category::Tools => galeri::tools(app, ui, inner),
        Category::Color => {
            let mut tab = app.ui.kurgu.color_tab;
            let body = galeri::tabs(ui, &t, inner, &[tl!("Looks"), tl!("Adjust")], &mut tab);
            app.ui.kurgu.color_tab = tab;
            match tab {
                0 => galeri::color_looks(app, ui, body),
                _ => panel(app, ui, PanelKind::LumetriColor, body),
            }
        }
        Category::Audio => {
            let mut tab = app.ui.kurgu.audio_tab;
            let body = galeri::tabs(ui, &t, inner, &[tl!("Effects"), tl!("Transitions"), tl!("Sound Settings")], &mut tab);
            app.ui.kurgu.audio_tab = tab;
            match tab {
                0 => galeri::effects(app, ui, body, galeri::Source::AudioEffects),
                1 => galeri::effects(app, ui, body, galeri::Source::AudioTransitions),
                _ => panel(app, ui, PanelKind::EssentialSound, body),
            }
        }
        _ => match cat.panel() {
            Some(kind) => panel(app, ui, kind, inner),
            None => coming_soon(ui, &t, inner, cat),
        },
    }
}

/// "Add to Timeline" at the right of the Media title: places the clips selected in Media at the end
/// of the open sequence (or makes a sequence from them when none is open).
fn add_to_timeline_button(app: &mut FilmcraftApp, ui: &mut egui::Ui, strip: Rect) {
    let t = app.tokens;
    let items: Vec<filmcraft_project::ItemId> = app
        .session
        .state
        .project_selection
        .iter()
        .copied()
        .filter(|i| app.session.project.item(*i).is_some_and(|it| it.has_video() || it.has_audio()))
        .collect();
    let label = tl!("Add to Timeline");
    let font = Tokens::ui(11.5);
    let w = ui.fonts_mut(|f| f.layout_no_wrap(label.to_string(), font.clone(), t.text).size().x) + 30.0;
    let r = Rect::from_min_size(pos2(strip.max.x - w - 8.0, strip.center().y - 11.0), vec2(w, 22.0));
    let enabled = !items.is_empty();
    let resp = ui.interact(r, egui::Id::unique("kurgu-add-to-timeline"), if enabled { Sense::click() } else { Sense::hover() });
    let resp = resp.on_hover_text(if enabled { tl!("Place the selected clips at the end of the timeline") } else { tl!("Select clips in Media first") });
    let fill = if !enabled {
        t.field_bg
    } else if resp.hovered() {
        t.accent_hover
    } else {
        t.accent
    };
    let fg = if enabled { Color32::WHITE } else { t.text_faint };
    ui.painter().rect_filled(r, CornerRadius::same(11), fill);
    icons::paint(ui.painter(), Rect::from_center_size(pos2(r.min.x + 13.0, r.center().y), vec2(12.0, 12.0)), Icon::Plus, fg);
    ui.painter().text(pos2(r.min.x + 22.0, r.center().y), Align2::LEFT_CENTER, label, font, fg);
    if !(enabled && resp.clicked()) {
        return;
    }
    let ids: Vec<u64> = items.iter().map(|i| i.0).collect();
    let result = match app.session.active_sequence().map(|s| s.duration()) {
        // no sequence open yet: one made from the clips, matching the first
        None => app.session.execute("file.newSequence", serde_json::json!({"fromItems": ids})).map_err(|e| e.to_string()),
        Some(end) => {
            let mut params = serde_json::json!({"time": end.0, "insert": false});
            match ids.as_slice() {
                [one] => params["item"] = serde_json::json!(one),
                many => params["items"] = serde_json::json!(many),
            }
            crate::panels::clip_mismatch::execute(app, "timeline.place", params)
        }
    };
    if let Err(e) = result {
        app.ui.status = e;
    }
}

fn coming_soon(ui: &mut egui::Ui, t: &Tokens, rect: Rect, cat: Category) {
    let text = match cat {
        Category::Elements => tl!("Coming soon"),
        _ => tl!("Coming soon"),
    };
    let p = ui.painter();
    icons::paint(p, Rect::from_center_size(rect.center() - vec2(0.0, 40.0), vec2(40.0, 40.0)), cat.icon(), t.text_faint);
    p.text(rect.center() + vec2(0.0, 10.0), Align2::CENTER_TOP, tl!("Coming soon"), Tokens::ui(15.0), t.text);
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
