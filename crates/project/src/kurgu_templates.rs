//! VideoKurgu's text templates: original designs generated in code from a few styles, layouts and
//! animations (titles, lower thirds, social media calls, labels, subtitle styles, chapter titles,
//! quotes and closing cards). Each is an ordinary graphics template, so it is placed, edited and
//! exported like FilmCraft's built-in ones. The animations sit in the protected intro and outro
//! (Responsive Design – Time), so they keep their timing when the clip is made longer or shorter.

use filmcraft_geom::Vec2;
use filmcraft_time::{TICKS_PER_SECOND, Tick};

use crate::effect::EffectInstance;
use crate::graphic::{new_shape_layer, new_text_layer};
use crate::graphic_design::{GraphicMeta, LayerExtra};
use crate::gtemplate::{ControlKind, GraphicsTemplate, TemplateControl};
use crate::keyframe::{Interpolation, Keyframe, Param, ParamValue};

const INTRO: f64 = 0.7;
const OUTRO: f64 = 0.6;

/// How a template's text looks.
#[derive(Clone, Copy)]
struct Style {
    id: &'static str,
    name: &'static str,
    font: &'static str,
    font_style: &'static str,
    fill: u32,
    gradient: Option<(u32, u32)>,
    stroke: Option<(u32, f64)>,
    /// A box behind the text: color, opacity %, padding, corner radius.
    boxed: Option<(u32, f64, f64, f64)>,
    /// color, opacity %, distance, blur.
    shadow: Option<(u32, f64, f64, f64)>,
    tracking: f64,
}

const fn style(id: &'static str, name: &'static str, font: &'static str, font_style: &'static str, fill: u32) -> Style {
    Style { id, name, font, font_style, fill, gradient: None, stroke: None, boxed: None, shadow: None, tracking: 0.0 }
}

// The display fonts come from VideoKurgu's craft-fonts manifest (CRAFT_FONTS_DIR); without it a
// missing family falls back to the bundled faces, so every template still renders.
const SADE: Style = Style { shadow: Some((0x000000, 70.0, 4.0, 18.0)), ..style("sade", "Sade", "Poppins", "Bold", 0xffffff) };
const KUTULU: Style = Style { boxed: Some((0x000000, 72.0, 22.0, 14.0)), ..style("kutulu", "Kutulu", "Poppins", "SemiBold", 0xffffff) };
const KONTURLU: Style = Style { stroke: Some((0x000000, 7.0)), ..style("konturlu", "Konturlu", "Poppins", "ExtraBold", 0xffffff) };
const NEON: Style =
    Style { stroke: Some((0x22d3ee, 3.0)), shadow: Some((0x22d3ee, 100.0, 0.0, 34.0)), ..style("neon", "Neon", "Righteous", "Regular", 0xecfeff) };
const GRADYAN: Style =
    Style { gradient: Some((0xf97316, 0xec4899)), shadow: Some((0x000000, 55.0, 4.0, 16.0)), ..style("gradyan", "Gradyan", "Poppins", "Black", 0xffffff) };
const SINEMA: Style =
    Style { tracking: 160.0, shadow: Some((0x000000, 60.0, 2.0, 20.0)), ..style("sinema", "Sinema", "Playfair Display SC", "Regular", 0xf5f5f4) };
const PASTEL: Style = Style { boxed: Some((0xfde68a, 100.0, 24.0, 26.0)), ..style("pastel", "Pastel", "Poppins", "Bold", 0x1f2937) };
const DAKTILO: Style = Style { boxed: Some((0xf5f0e1, 100.0, 18.0, 4.0)), ..style("daktilo", "Daktilo", "Special Elite", "Regular", 0x1c1917) };
const ALTIN: Style = Style {
    gradient: Some((0xfde68a, 0xb45309)),
    stroke: Some((0x3b2106, 2.0)),
    shadow: Some((0x000000, 60.0, 3.0, 14.0)),
    ..style("altin", "Altın", "Cinzel Decorative", "Bold", 0xfde68a)
};
const KALIN_GOLGE: Style = Style { shadow: Some((0x000000, 100.0, 9.0, 0.0)), ..style("kalin-golge", "Kalın Gölge", "Archivo Black", "Regular", 0xfacc15) };
const AFIS: Style = Style { tracking: 60.0, shadow: Some((0x000000, 65.0, 3.0, 14.0)), ..style("afis", "Afiş", "Bebas Neue", "Regular", 0xffffff) };
const MANSET: Style = Style { boxed: Some((0xdc2626, 100.0, 20.0, 0.0)), ..style("manset", "Manşet", "Anton", "Regular", 0xffffff) };
const KOMIK: Style =
    Style { stroke: Some((0x111111, 9.0)), shadow: Some((0x111111, 100.0, 8.0, 0.0)), ..style("komik", "Komik", "Bangers", "Regular", 0xfde047) };
const RETRO: Style = Style { shadow: Some((0xf472b6, 100.0, 0.0, 30.0)), ..style("retro", "Retro Neon", "Monoton", "Regular", 0xfbcfe8) };
const OYUN: Style = Style { boxed: Some((0x000000, 90.0, 20.0, 0.0)), ..style("oyun", "Oyun", "Press Start 2P", "Regular", 0x86efac) };
const EL_YAZISI: Style = Style { shadow: Some((0x000000, 60.0, 3.0, 12.0)), ..style("el-yazisi", "El Yazısı", "Pacifico", "Regular", 0xffffff) };
const ZARIF: Style =
    Style { gradient: Some((0xfef3c7, 0xd97706)), shadow: Some((0x000000, 55.0, 2.0, 10.0)), ..style("zarif", "Zarif", "Great Vibes", "Regular", 0xfde68a) };
const FIRCA: Style = Style { shadow: Some((0x000000, 70.0, 4.0, 16.0)), ..style("firca", "Fırça", "Kaushan Script", "Regular", 0xffffff) };
const LOBSTER: Style =
    Style { gradient: Some((0xfacc15, 0xf97316)), stroke: Some((0x7c2d12, 3.0)), ..style("lobster", "Tatlı", "Lobster", "Regular", 0xfacc15) };
const SPOR: Style =
    Style { stroke: Some((0x0ea5e9, 4.0)), shadow: Some((0x000000, 70.0, 5.0, 10.0)), ..style("spor", "Spor", "Russo One", "Regular", 0xffffff) };
const ASKERI: Style = Style { shadow: Some((0x000000, 80.0, 4.0, 6.0)), ..style("askeri", "Askeri", "Black Ops One", "Regular", 0xa3b18a) };
const KLASIK: Style = Style { tracking: 40.0, shadow: Some((0x000000, 60.0, 3.0, 16.0)), ..style("klasik", "Klasik", "Abril Fatface", "Regular", 0xffffff) };
const EGLENCELI: Style =
    Style { stroke: Some((0xffffff, 6.0)), shadow: Some((0x000000, 60.0, 6.0, 10.0)), ..style("eglenceli", "Eğlenceli", "Luckiest Guy", "Regular", 0xf43f5e) };
const TABELA: Style = Style { boxed: Some((0x1e293b, 92.0, 22.0, 18.0)), ..style("tabela", "Tabela", "Bungee", "Regular", 0xfb923c) };
const STYLES: [Style; 24] = [
    SADE,
    AFIS,
    KUTULU,
    MANSET,
    KONTURLU,
    KOMIK,
    NEON,
    RETRO,
    GRADYAN,
    LOBSTER,
    SINEMA,
    KLASIK,
    PASTEL,
    TABELA,
    DAKTILO,
    OYUN,
    ALTIN,
    ZARIF,
    KALIN_GOLGE,
    EGLENCELI,
    EL_YAZISI,
    FIRCA,
    SPOR,
    ASKERI,
];

/// How a template comes in (and goes out: every template fades out at the end).
#[derive(Clone, Copy, PartialEq)]
enum Anim {
    Fade,
    SlideUp,
    SlideLeft,
    Pop,
}

impl Anim {
    const ALL: [Anim; 4] = [Anim::Fade, Anim::SlideUp, Anim::SlideLeft, Anim::Pop];
    fn id(self) -> &'static str {
        match self {
            Anim::Fade => "belirme",
            Anim::SlideUp => "yukari",
            Anim::SlideLeft => "soldan",
            Anim::Pop => "buyume",
        }
    }
    fn name(self) -> &'static str {
        match self {
            Anim::Fade => "Belirme",
            Anim::SlideUp => "Yukarı Kayma",
            Anim::SlideLeft => "Soldan Gelme",
            Anim::Pop => "Büyüme",
        }
    }
}

fn rgb(hex: u32) -> ParamValue {
    let c = |s: u32| ((hex >> s) & 0xff) as f32 / 255.0;
    ParamValue::Color([c(16), c(8), c(0), 1.0])
}

fn set(e: &mut EffectInstance, id: &str, v: ParamValue) {
    e.params.insert(id.to_string(), Param::new(v));
}

fn secs(s: f64) -> Tick {
    Tick((s * TICKS_PER_SECOND as f64).round() as i64)
}

/// A text layer in `st`'s look; `align` 0 left, 1 centre.
fn text(uid: u64, name: &str, s: &str, pos: (f64, f64), size: f64, st: &Style, align: u32) -> EffectInstance {
    let mut e = new_text_layer(s, Vec2::new(pos.0, pos.1), size);
    set(&mut e, "name", ParamValue::Text(name.into()));
    set(&mut e, "font", ParamValue::Text(st.font.into()));
    set(&mut e, "font_style", ParamValue::Text(st.font_style.into()));
    set(&mut e, "align", ParamValue::Choice(align));
    set(&mut e, "fill_color", rgb(st.fill));
    if st.tracking != 0.0 {
        set(&mut e, "tracking", ParamValue::Float(st.tracking));
    }
    if let Some((a, b)) = st.gradient {
        set(&mut e, "fill_kind", ParamValue::Choice(1));
        set(&mut e, "gradient_start", rgb(a));
        set(&mut e, "gradient_end", rgb(b));
        set(&mut e, "gradient_angle", ParamValue::Float(90.0));
    }
    if let Some((c, w)) = st.stroke {
        set(&mut e, "stroke", ParamValue::Bool(true));
        set(&mut e, "stroke_color", rgb(c));
        set(&mut e, "stroke_width", ParamValue::Float(w));
    }
    if let Some((c, opacity, pad, radius)) = st.boxed {
        set(&mut e, "background", ParamValue::Bool(true));
        set(&mut e, "background_color", rgb(c));
        set(&mut e, "background_opacity", ParamValue::Float(opacity));
        set(&mut e, "background_size", ParamValue::Float(pad));
        set(&mut e, "background_radius", ParamValue::Float(radius));
    }
    if let Some((c, opacity, distance, blur)) = st.shadow {
        set(&mut e, "shadow", ParamValue::Bool(true));
        set(&mut e, "shadow_color", rgb(c));
        set(&mut e, "shadow_opacity", ParamValue::Float(opacity));
        set(&mut e, "shadow_distance", ParamValue::Float(distance));
        set(&mut e, "shadow_blur", ParamValue::Float(blur));
    }
    e.layer = Some(Box::new(LayerExtra { uid, ..Default::default() }));
    e
}

/// A rectangle centred at `pos`.
fn rect(uid: u64, name: &str, pos: (f64, f64), size: (f64, f64), color: u32, radius: f64) -> EffectInstance {
    let mut e = new_shape_layer(0, Vec2::new(pos.0, pos.1), Vec2::new(size.0, size.1), vec![]);
    set(&mut e, "name", ParamValue::Text(name.into()));
    set(&mut e, "fill_color", rgb(color));
    if radius > 0.0 {
        set(&mut e, "corner_radius", ParamValue::Float(radius));
    }
    e.layer = Some(Box::new(LayerExtra { uid, ..Default::default() }));
    e
}

/// Animates `param` through `(seconds, value)` keys; the static value is the settled one.
fn keys(e: &mut EffectInstance, param: &str, settled: ParamValue, frames: &[(f64, ParamValue, Interpolation)]) {
    let keyframes = frames
        .iter()
        .map(|(t, v, interp)| {
            let mut k = Keyframe::new(secs(*t), v.clone());
            k.interp = *interp;
            k
        })
        .collect();
    e.params.insert(param.to_string(), Param { value: settled, keyframes });
}

fn position_of(e: &EffectInstance) -> Vec2 {
    match e.params.get("position").map(|p| &p.value) {
        Some(ParamValue::Vec2(v)) => *v,
        _ => Vec2::new(960.0, 540.0),
    }
}

/// Puts `anim` on every layer, and the fade-out at the end of a `duration`-second clip.
fn animate(layers: &mut [EffectInstance], anim: Anim, duration: f64) {
    use Interpolation::{EaseOut, Linear};
    let end = duration;
    let out = (duration - OUTRO).max(INTRO);
    for e in layers.iter_mut() {
        let fade_in = if anim == Anim::Fade { 0.5 } else { 0.3 };
        keys(
            e,
            "opacity",
            ParamValue::Float(100.0),
            &[
                (0.0, ParamValue::Float(0.0), Linear),
                (fade_in, ParamValue::Float(100.0), Linear),
                (out, ParamValue::Float(100.0), Linear),
                (end, ParamValue::Float(0.0), Linear),
            ],
        );
        let p = position_of(e);
        let from = match anim {
            Anim::SlideUp => Some(Vec2::new(p.x, p.y + 70.0)),
            Anim::SlideLeft => Some(Vec2::new(p.x - 220.0, p.y)),
            _ => None,
        };
        if let Some(from) = from {
            keys(e, "position", ParamValue::Vec2(p), &[(0.0, ParamValue::Vec2(from), EaseOut), (INTRO - 0.1, ParamValue::Vec2(p), Linear)]);
        }
        if anim == Anim::Pop {
            keys(
                e,
                "scale",
                ParamValue::Float(100.0),
                &[(0.0, ParamValue::Float(35.0), EaseOut), (0.35, ParamValue::Float(112.0), Linear), (0.55, ParamValue::Float(100.0), Linear)],
            );
        }
    }
}

fn ctl(id: &str, name: &str, layer: u64, param: &str) -> TemplateControl {
    TemplateControl { id: id.into(), name: name.into(), kind: ControlKind::for_param(param), layer, param: param.into(), min: None, max: None }
}

/// A template from its layers: animated, with the intro and outro protected.
fn template(
    id: String,
    name: String,
    category: &str,
    description: &str,
    seconds: f64,
    mut layers: Vec<EffectInstance>,
    anim: Anim,
    controls: Vec<TemplateControl>,
) -> GraphicsTemplate {
    animate(&mut layers, anim, seconds);
    let graphic = GraphicMeta { intro: secs(INTRO), outro: secs(OUTRO), design_in: Tick::ZERO, design_duration: secs(seconds), ..Default::default() };
    GraphicsTemplate {
        id: format!("videokurgu:{id}"),
        name,
        category: category.into(),
        description: description.into(),
        author: "VideoKurgu".into(),
        license: "MIT OR Apache-2.0".into(),
        tags: vec![category.to_lowercase(), anim.name().to_lowercase()],
        duration: secs(seconds),
        layers,
        graphic,
        controls,
        ..Default::default()
    }
}

/// Text controls for every text layer, then the colors.
fn text_controls(texts: &[(u64, &str)], color_layer: u64, boxed: Option<u64>) -> Vec<TemplateControl> {
    let mut v: Vec<TemplateControl> = texts.iter().enumerate().map(|(i, (uid, name))| ctl(&format!("text{i}"), name, *uid, "text")).collect();
    v.push(ctl("color", "Yazı Rengi", color_layer, "fill_color"));
    if let Some(b) = boxed {
        v.push(ctl("box", "Kutu Rengi", b, "background_color"));
    }
    v.push(ctl("font", "Yazı Tipi", color_layer, "font"));
    v
}

/// All of VideoKurgu's text templates.
pub fn templates() -> Vec<GraphicsTemplate> {
    let mut v = Vec::new();
    titles(&mut v);
    subtitles(&mut v);
    lower_thirds(&mut v);
    social(&mut v);
    labels(&mut v);
    chapters(&mut v);
    quotes(&mut v);
    closing(&mut v);
    elements(&mut v);
    intros(&mut v);
    v
}

fn boxed_uid(st: &Style, uid: u64) -> Option<u64> {
    st.boxed.map(|_| uid)
}

/// Large centred titles: every style with every animation.
fn titles(v: &mut Vec<GraphicsTemplate>) {
    // One group per animation, so each group shows every style side by side.
    for anim in Anim::ALL {
        let category = format!("Başlıklar · {}", anim.name());
        for st in &STYLES {
            let t = text(1, "Başlık", "BAŞLIĞINIZ BURADA", (960.0, 570.0), 118.0, st, 1);
            v.push(template(
                format!("baslik-{}-{}", st.id, anim.id()),
                format!("Başlık · {} · {}", st.name, anim.name()),
                &category,
                "Ortada büyük bir başlık.",
                5.0,
                vec![t],
                anim,
                text_controls(&[(1, "Başlık")], 1, boxed_uid(st, 1)),
            ));
        }
    }
}

/// Subtitle styles at the bottom of the frame.
fn subtitles(v: &mut Vec<GraphicsTemplate>) {
    for st in [SADE, KUTULU, KONTURLU, PASTEL, DAKTILO, KALIN_GOLGE, TABELA, AFIS] {
        let t = text(1, "Altyazı", "Buraya konuşma metni gelir", (960.0, 985.0), 54.0, &st, 1);
        v.push(template(
            format!("altyazi-{}", st.id),
            format!("Altyazı · {}", st.name),
            "Altyazı Stilleri",
            "Görüntünün altında okunaklı bir satır.",
            4.0,
            vec![t],
            Anim::Fade,
            text_controls(&[(1, "Metin")], 1, boxed_uid(&st, 1)),
        ));
    }
}

/// Name and role over a coloured bar, in five colours and two layouts.
fn lower_thirds(v: &mut Vec<GraphicsTemplate>) {
    let colors = [
        ("mavi", "Mavi", 0x1d4ed8),
        ("kirmizi", "Kırmızı", 0xdc2626),
        ("yesil", "Yeşil", 0x15803d),
        ("mor", "Mor", 0x7c3aed),
        ("turuncu", "Turuncu", 0xea580c),
    ];
    for (cid, cname, color) in colors {
        // Bar: a full slab with the name and role on it.
        let bar = rect(1, "Bant", (460.0, 920.0), (700.0, 150.0), color, 10.0);
        let accent = rect(4, "Vurgu", (118.0, 920.0), (14.0, 150.0), 0xfacc15, 0.0);
        let name = text(2, "Ad", "Ad Soyad", (150.0, 912.0), 60.0, &SADE, 0);
        let mut role = text(3, "Unvan", "Unvan / Görev", (152.0, 966.0), 34.0, &SADE, 0);
        set(&mut role, "fill_color", rgb(0xe5e7eb));
        let mut controls = text_controls(&[(2, "Ad"), (3, "Unvan")], 2, None);
        controls.push(ctl("bar", "Bant Rengi", 1, "fill_color"));
        v.push(template(
            format!("altbant-bant-{cid}"),
            format!("Alt Bant · Bant · {cname}"),
            "Alt Bantlar",
            "Renkli bir bant üzerinde ad ve unvan.",
            6.0,
            vec![bar, accent, name, role],
            Anim::SlideLeft,
            controls,
        ));
        // Line: the name over a thin coloured rule.
        let name = text(1, "Ad", "Ad Soyad", (140.0, 905.0), 62.0, &KONTURLU, 0);
        let rule = rect(2, "Çizgi", (340.0, 928.0), (400.0, 6.0), color, 3.0);
        let role = text(3, "Unvan", "Unvan / Görev", (142.0, 975.0), 34.0, &SADE, 0);
        let mut controls = text_controls(&[(1, "Ad"), (3, "Unvan")], 1, None);
        controls.push(ctl("rule", "Çizgi Rengi", 2, "fill_color"));
        v.push(template(
            format!("altbant-cizgi-{cid}"),
            format!("Alt Bant · Çizgi · {cname}"),
            "Alt Bantlar",
            "İnce renkli bir çizginin üstünde ad, altında unvan.",
            6.0,
            vec![name, rule, role],
            Anim::SlideUp,
            controls,
        ));
    }
}

/// A rounded button style for calls to action and labels.
fn button(color: u32, text_color: u32, radius: f64) -> Style {
    Style { boxed: Some((color, 100.0, 26.0, radius)), shadow: Some((0x000000, 45.0, 4.0, 14.0)), ..style("buton", "Buton", "Inter", "Bold", text_color) }
}

/// Social media calls: subscribe, like, follow…
fn social(v: &mut Vec<GraphicsTemplate>) {
    let calls = [
        ("abone", "ABONE OL", 0xdc2626),
        ("begen", "BEĞEN", 0x2563eb),
        ("takip", "TAKİP ET", 0x7c3aed),
        ("yorum", "YORUM YAP", 0x16a34a),
        ("zil", "BİLDİRİMLERİ AÇ", 0xea580c),
        ("paylas", "PAYLAŞ", 0x0d9488),
    ];
    for (id, label, color) in calls {
        let st = button(color, 0xffffff, 40.0);
        let t = text(1, "Çağrı", label, (300.0, 930.0), 62.0, &st, 1);
        v.push(template(
            format!("sosyal-{id}"),
            format!("Sosyal · {}", title_case(label)),
            "Sosyal Medya",
            "İzleyiciyi harekete geçiren bir düğme.",
            4.0,
            vec![t],
            Anim::Pop,
            text_controls(&[(1, "Metin")], 1, Some(1)),
        ));
    }
}

/// Small labels in the top corner: new, live, sale…
fn labels(v: &mut Vec<GraphicsTemplate>) {
    let items = [
        ("yeni", "YENİ", 0x16a34a),
        ("canli", "CANLI", 0xdc2626),
        ("indirim", "İNDİRİM %50", 0xf59e0b),
        ("onemli", "ÖNEMLİ", 0x7c3aed),
        ("son-dakika", "SON DAKİKA", 0xb91c1c),
        ("kampanya", "KAMPANYA", 0x0ea5e9),
    ];
    for (id, label, color) in items {
        let st = button(color, 0xffffff, 10.0);
        let t = text(1, "Etiket", label, (1640.0, 150.0), 50.0, &st, 1);
        v.push(template(
            format!("etiket-{id}"),
            format!("Etiket · {}", title_case(label)),
            "Etiketler",
            "Köşede dikkat çeken küçük bir etiket.",
            4.0,
            vec![t],
            Anim::Pop,
            text_controls(&[(1, "Metin")], 1, Some(1)),
        ));
    }
}

/// "Bölüm 1" with a name under it.
fn chapters(v: &mut Vec<GraphicsTemplate>) {
    for st in [SADE, SINEMA, GRADYAN, KUTULU, AFIS, MANSET, OYUN, KLASIK] {
        let number = text(1, "Bölüm", "BÖLÜM 1", (960.0, 520.0), 64.0, &st, 1);
        let name = text(2, "Ad", "Başlangıç", (960.0, 640.0), 104.0, &st, 1);
        v.push(template(
            format!("bolum-{}", st.id),
            format!("Bölüm Başlığı · {}", st.name),
            "Bölüm Başlıkları",
            "Bölüm numarası ve adı.",
            4.0,
            vec![number, name],
            Anim::SlideUp,
            text_controls(&[(1, "Bölüm"), (2, "Ad")], 2, boxed_uid(&st, 2)),
        ));
    }
}

/// A quotation and its author.
fn quotes(v: &mut Vec<GraphicsTemplate>) {
    for st in [SINEMA, SADE, ALTIN, ZARIF, KLASIK, DAKTILO] {
        let quote = text(1, "Alıntı", "\u{201c}Güzel bir söz buraya yazılır.\u{201d}", (960.0, 520.0), 70.0, &st, 1);
        let author = text(2, "Yazar", "\u{2014} Yazarın Adı", (960.0, 620.0), 40.0, &st, 1);
        v.push(template(
            format!("alinti-{}", st.id),
            format!("Alıntı · {}", st.name),
            "Alıntılar",
            "Ortada bir alıntı ve altında yazarı.",
            6.0,
            vec![quote, author],
            Anim::Fade,
            text_controls(&[(1, "Alıntı"), (2, "Yazar")], 1, None),
        ));
    }
}

/// End cards.
fn closing(v: &mut Vec<GraphicsTemplate>) {
    let cards = [
        ("tesekkurler", "Teşekkürler", GRADYAN, Anim::Pop),
        ("izlediginiz", "İzlediğiniz İçin Teşekkürler", SADE, Anim::Fade),
        ("gorusmek", "Görüşmek Üzere", NEON, Anim::SlideUp),
        ("abone-unutma", "Abone Olmayı Unutmayın", KALIN_GOLGE, Anim::Pop),
        ("tesekkurler-el", "Teşekkürler", EL_YAZISI, Anim::Fade),
        ("gorusmek-zarif", "Görüşmek Üzere", ZARIF, Anim::Fade),
        ("son", "SON", AFIS, Anim::SlideUp),
        ("abone-komik", "Abone Ol!", KOMIK, Anim::Pop),
    ];
    for (id, label, st, anim) in cards {
        let t = text(1, "Metin", label, (960.0, 570.0), 96.0, &st, 1);
        v.push(template(
            format!("kapanis-{id}"),
            format!("Kapanış · {label} · {}", st.name),
            "Kapanış",
            "Videonun sonu için bir kart.",
            5.0,
            vec![t],
            anim,
            text_controls(&[(1, "Metin")], 1, boxed_uid(&st, 1)),
        ));
    }
}

// ------------------------------------------------------------------------------------------------
// Elements (Öğeler): arrows, icons, faces, highlights, speech bubbles and stickers, drawn with shape
// layers and the symbol fonts. Their ids start with `videokurgu:oge-`, which is how the Elements
// gallery tells them from the text templates.
// ------------------------------------------------------------------------------------------------

/// Prefix of an element template's id.
pub const ELEMENT_PREFIX: &str = "videokurgu:oge-";

const SYMBOLS: &str = "Noto Sans Symbols 2";

/// A symbol glyph centred on `c`, coloured, with an outline and a shadow so it reads as a sticker.
fn glyph(uid: u64, name: &str, ch: &str, c: (f64, f64), size: f64, color: u32, outline: u32) -> EffectInstance {
    let st = Style {
        stroke: Some((outline, size / 22.0)),
        shadow: Some((0x000000, 55.0, size / 40.0, size / 14.0)),
        ..style("oge", "Öğe", SYMBOLS, "Regular", color)
    };
    // the position is the text's baseline: about a third of the size under the centre
    text(uid, name, ch, (c.0, c.1 + size * 0.36), size, &st, 1)
}

fn ellipse(uid: u64, name: &str, pos: (f64, f64), size: (f64, f64), color: u32) -> EffectInstance {
    let mut e = rect(uid, name, pos, size, color, 0.0);
    set(&mut e, "shape", ParamValue::Choice(1));
    e
}

/// An outline only: no fill, a `width` stroke in `color`.
fn outline_only(e: &mut EffectInstance, color: u32, width: f64) {
    set(e, "fill", ParamValue::Bool(false));
    set(e, "stroke", ParamValue::Bool(true));
    set(e, "stroke_color", rgb(color));
    set(e, "stroke_width", ParamValue::Float(width));
    set(e, "stroke_type", ParamValue::Choice(1));
}

/// A beat after the entrance: the element grows and shrinks a little until the outro.
fn pulse(layers: &mut [EffectInstance], duration: f64) {
    let end = (duration - OUTRO).max(INTRO);
    for e in layers.iter_mut() {
        let mut frames = vec![
            (0.0, ParamValue::Float(35.0), Interpolation::EaseOut),
            (0.35, ParamValue::Float(112.0), Interpolation::Linear),
            (0.55, ParamValue::Float(100.0), Interpolation::Linear),
        ];
        let mut t = 1.0;
        while t + 0.6 <= end {
            frames.push((t, ParamValue::Float(100.0), Interpolation::EaseOut));
            frames.push((t + 0.3, ParamValue::Float(112.0), Interpolation::EaseOut));
            frames.push((t + 0.6, ParamValue::Float(100.0), Interpolation::Linear));
            t += 0.9;
        }
        keys(e, "scale", ParamValue::Float(100.0), &frames);
    }
}

#[allow(clippy::too_many_arguments)]
fn element(
    id: &str,
    name: String,
    group: &str,
    seconds: f64,
    layers: Vec<EffectInstance>,
    anim: Anim,
    beat: bool,
    controls: Vec<TemplateControl>,
) -> GraphicsTemplate {
    let mut t = template(format!("oge-{id}"), name, group, "Videonun üstüne eklenen bir öğe.", seconds, layers, anim, controls);
    if beat {
        pulse(&mut t.layers, seconds);
    }
    t.tags.push("öğe".into());
    t
}

fn elements(v: &mut Vec<GraphicsTemplate>) {
    // Arrows, in three colours, pulsing to point at something.
    let arrows = [
        ("sag", "Sağa", "\u{279c}"),
        ("sag2", "Sağa Kalın", "\u{27a4}"),
        ("sol", "Sola", "\u{2b05}"),
        ("yukari", "Yukarı", "\u{2b06}"),
        ("asagi", "Aşağı", "\u{2b07}"),
        ("el-sag", "El Sağa", "\u{1f449}"),
        ("el-sol", "El Sola", "\u{1f448}"),
        ("el-yukari", "El Yukarı", "\u{1f446}"),
        ("el-asagi", "El Aşağı", "\u{1f447}"),
    ];
    for (id, name, ch) in arrows {
        for (cid, cname, color, outline) in
            [("kirmizi", "Kırmızı", 0xef4444, 0xffffff), ("sari", "Sarı", 0xfacc15, 0x111111), ("beyaz", "Beyaz", 0xffffff, 0x111111)]
        {
            let g = glyph(1, "Ok", ch, (960.0, 540.0), 300.0, color, outline);
            v.push(element(
                &format!("ok-{id}-{cid}"),
                format!("Ok · {name} · {cname}"),
                "Oklar",
                4.0,
                vec![g],
                Anim::Pop,
                true,
                vec![ctl("color", "Renk", 1, "fill_color")],
            ));
        }
    }
    // Icons.
    let icons = [
        ("yildiz", "Yıldız", "\u{2b50}", 0xfacc15, 0x92400e),
        ("kalp", "Kalp", "\u{2764}", 0xef4444, 0xffffff),
        ("onay", "Onay", "\u{2714}", 0x22c55e, 0xffffff),
        ("carpi", "Çarpı", "\u{2716}", 0xef4444, 0xffffff),
        ("simsek", "Şimşek", "\u{26a1}", 0xfacc15, 0x111111),
        ("uyari", "Uyarı", "\u{26a0}", 0xf59e0b, 0x111111),
        ("unlem", "Ünlem", "\u{2757}", 0xef4444, 0xffffff),
        ("soru", "Soru", "\u{2753}", 0x3b82f6, 0xffffff),
        ("begen", "Beğen", "\u{1f44d}", 0x3b82f6, 0xffffff),
        ("begenme", "Beğenme", "\u{1f44e}", 0x64748b, 0xffffff),
        ("kupa", "Kupa", "\u{1f3c6}", 0xf59e0b, 0x78350f),
        ("gunes", "Güneş", "\u{2600}", 0xfbbf24, 0xb45309),
        ("bulut", "Bulut", "\u{2601}", 0xe2e8f0, 0x475569),
        ("semsiye", "Şemsiye", "\u{2602}", 0x8b5cf6, 0xffffff),
        ("kardan-adam", "Kardan Adam", "\u{2603}", 0xffffff, 0x334155),
        ("nota", "Nota", "\u{266b}", 0xec4899, 0xffffff),
        ("telefon", "Telefon", "\u{260e}", 0x111827, 0xffffff),
        ("ucak", "Uçak", "\u{2708}", 0x0ea5e9, 0xffffff),
        ("zarf", "Zarf", "\u{2709}", 0xf8fafc, 0x334155),
        ("top", "Futbol Topu", "\u{26bd}", 0xffffff, 0x111111),
        ("makas", "Makas", "\u{2702}", 0xf43f5e, 0xffffff),
        ("kalem", "Kalem", "\u{270f}", 0xf59e0b, 0x111111),
        ("kum-saati", "Kum Saati", "\u{231b}", 0xd97706, 0xffffff),
        ("cicek", "Çiçek", "\u{2740}", 0xf472b6, 0xffffff),
        ("parilti", "Parıltı", "\u{2726}", 0xfde047, 0xffffff),
        ("geri-donusum", "Geri Dönüşüm", "\u{267b}", 0x22c55e, 0xffffff),
    ];
    for (id, name, ch, color, outline) in icons {
        let g = glyph(1, name, ch, (960.0, 540.0), 300.0, color, outline);
        v.push(element(
            &format!("simge-{id}"),
            format!("Simge · {name}"),
            "Simgeler",
            4.0,
            vec![g],
            Anim::Pop,
            false,
            vec![ctl("color", "Renk", 1, "fill_color")],
        ));
    }
    // Faces drawn with shapes: a yellow disc, eyes and a mouth.
    let faces = [
        ("gulen", "Gülen", "\u{25e1}", false),
        ("uzgun", "Üzgün", "\u{25e0}", false),
        ("saskin", "Şaşkın", "\u{25cf}", false),
        ("asik", "Âşık", "\u{25e1}", true),
    ];
    for (id, name, mouth, hearts) in faces {
        let mut disc = ellipse(1, "Yüz", (960.0, 540.0), (420.0, 420.0), 0xfacc15);
        set(&mut disc, "stroke", ParamValue::Bool(true));
        set(&mut disc, "stroke_color", rgb(0xb45309));
        set(&mut disc, "stroke_width", ParamValue::Float(10.0));
        let mut layers = vec![disc];
        if hearts {
            layers.push(glyph(2, "Sol Göz", "\u{2764}", (885.0, 480.0), 110.0, 0xef4444, 0xffffff));
            layers.push(glyph(3, "Sağ Göz", "\u{2764}", (1035.0, 480.0), 110.0, 0xef4444, 0xffffff));
        } else {
            layers.push(ellipse(2, "Sol Göz", (885.0, 480.0), (52.0, 78.0), 0x1f2937));
            layers.push(ellipse(3, "Sağ Göz", (1035.0, 480.0), (52.0, 78.0), 0x1f2937));
        }
        let (size, y) = if id == "saskin" { (110.0, 640.0) } else { (230.0, 625.0) };
        let mut m = glyph(4, "Ağız", mouth, (960.0, y), size, 0x1f2937, 0x1f2937);
        set(&mut m, "shadow", ParamValue::Bool(false));
        layers.push(m);
        v.push(element(
            &format!("yuz-{id}"),
            format!("Yüz · {name}"),
            "Yüzler",
            4.0,
            layers,
            Anim::Pop,
            false,
            vec![ctl("color", "Yüz Rengi", 1, "fill_color")],
        ));
    }
    // Highlights: to circle, frame, underline or mark something in the picture.
    {
        let mut c = ellipse(1, "Daire", (960.0, 540.0), (560.0, 340.0), 0xef4444);
        outline_only(&mut c, 0xef4444, 14.0);
        v.push(element(
            "vurgu-daire",
            "Vurgu · Daire".into(),
            "Vurgular",
            4.0,
            vec![c],
            Anim::Pop,
            false,
            vec![ctl("color", "Çizgi Rengi", 1, "stroke_color")],
        ));
        let mut r = rect(1, "Çerçeve", (960.0, 540.0), (600.0, 360.0), 0xfacc15, 28.0);
        outline_only(&mut r, 0xfacc15, 14.0);
        v.push(element(
            "vurgu-cerceve",
            "Vurgu · Çerçeve".into(),
            "Vurgular",
            4.0,
            vec![r],
            Anim::Pop,
            false,
            vec![ctl("color", "Çizgi Rengi", 1, "stroke_color")],
        ));
        let u = rect(1, "Alt Çizgi", (960.0, 610.0), (560.0, 18.0), 0xef4444, 9.0);
        v.push(element(
            "vurgu-alt-cizgi",
            "Vurgu · Alt Çizgi".into(),
            "Vurgular",
            4.0,
            vec![u],
            Anim::SlideLeft,
            false,
            vec![ctl("color", "Renk", 1, "fill_color")],
        ));
        let mut m = rect(1, "İşaretleyici", (960.0, 540.0), (600.0, 120.0), 0xfde047, 10.0);
        set(&mut m, "fill_color", ParamValue::Color([0.99, 0.88, 0.28, 0.45]));
        v.push(element(
            "vurgu-isaretleyici",
            "Vurgu · Fosforlu Kalem".into(),
            "Vurgular",
            4.0,
            vec![m],
            Anim::SlideLeft,
            false,
            vec![ctl("color", "Renk", 1, "fill_color")],
        ));
    }
    // Speech and thought bubbles.
    for (cid, cname, fill, ink) in [("beyaz", "Beyaz", 0xffffff, 0x111827), ("sari", "Sarı", 0xfde047, 0x111827), ("mavi", "Mavi", 0x3b82f6, 0xffffff)] {
        let body = rect(1, "Balon", (960.0, 500.0), (640.0, 260.0), fill, 70.0);
        let mut tail = rect(2, "Kuyruk", (830.0, 655.0), (110.0, 110.0), fill, 0.0);
        set(&mut tail, "shape", ParamValue::Choice(2));
        set(&mut tail, "sides", ParamValue::Float(3.0));
        set(&mut tail, "rotation", ParamValue::Float(180.0));
        let mut words = text(3, "Metin", "Merhaba!", (960.0, 532.0), 96.0, &SADE, 1);
        set(&mut words, "fill_color", rgb(ink));
        set(&mut words, "shadow", ParamValue::Bool(false));
        let mut controls = text_controls(&[(3, "Metin")], 3, None);
        controls.push(ctl("bubble", "Balon Rengi", 1, "fill_color"));
        v.push(element(
            &format!("balon-konusma-{cid}"),
            format!("Konuşma Balonu · {cname}"),
            "Balonlar",
            4.0,
            vec![body, tail, words],
            Anim::Pop,
            false,
            controls.clone(),
        ));
        let cloud = ellipse(1, "Balon", (960.0, 500.0), (680.0, 300.0), fill);
        let dot1 = ellipse(2, "Nokta 1", (790.0, 700.0), (70.0, 70.0), fill);
        let dot2 = ellipse(4, "Nokta 2", (740.0, 770.0), (40.0, 40.0), fill);
        let mut words = text(3, "Metin", "Hmm...", (960.0, 532.0), 96.0, &SADE, 1);
        set(&mut words, "fill_color", rgb(ink));
        set(&mut words, "shadow", ParamValue::Bool(false));
        v.push(element(
            &format!("balon-dusunce-{cid}"),
            format!("Düşünce Balonu · {cname}"),
            "Balonlar",
            4.0,
            vec![cloud, dot1, dot2, words],
            Anim::Pop,
            false,
            controls,
        ));
    }
    // Stickers: a symbol and a word on a pill.
    let stickers = [
        ("begen", "\u{1f44d}", "BEĞEN", 0x2563eb),
        ("yeni", "\u{2b50}", "YENİ", 0x16a34a),
        ("hizli", "\u{26a1}", "HIZLI", 0xf59e0b),
        ("tesekkurler", "\u{2764}", "TEŞEKKÜRLER", 0xdb2777),
        ("tamam", "\u{2714}", "TAMAM", 0x059669),
        ("dikkat", "\u{2757}", "DİKKAT", 0xdc2626),
        ("kazanan", "\u{1f3c6}", "KAZANAN", 0xb45309),
        ("muzik", "\u{266b}", "MÜZİK", 0x7c3aed),
    ];
    for (id, ch, word, color) in stickers {
        let w = 200.0 + word.chars().count() as f64 * 46.0;
        let pill = rect(1, "Zemin", (960.0, 540.0), (w, 130.0), color, 65.0);
        let left = 960.0 - w / 2.0;
        let mut sym = glyph(2, "Simge", ch, (left + 85.0, 540.0), 84.0, 0xffffff, color);
        set(&mut sym, "shadow", ParamValue::Bool(false));
        let mut label = text(3, "Metin", word, (left + 150.0, 566.0), 70.0, &SADE, 0);
        set(&mut label, "shadow", ParamValue::Bool(false));
        let mut controls = text_controls(&[(3, "Metin")], 3, None);
        controls.push(ctl("pill", "Zemin Rengi", 1, "fill_color"));
        v.push(element(
            &format!("cikartma-{id}"),
            format!("Çıkartma · {}", title_case(word)),
            "Çıkartmalar",
            4.0,
            vec![pill, sym, label],
            Anim::Pop,
            true,
            controls,
        ));
    }
}

// ------------------------------------------------------------------------------------------------
// Intros (İntrolar): full-frame openings whose layers come in one after another. Ids start with
// `videokurgu:intro-`.
// ------------------------------------------------------------------------------------------------

/// Prefix of an intro template's id.
pub const INTRO_PREFIX: &str = "videokurgu:intro-";

/// The intro's protected opening: long enough for its last layer to settle.
const INTRO_LEN: f64 = 1.8;

/// `animate`, starting `delay` seconds into the clip.
fn animate_from(layers: &mut [EffectInstance], anim: Anim, duration: f64, delay: f64) {
    animate(layers, anim, duration);
    for e in layers.iter_mut() {
        for p in ["opacity", "position", "scale"] {
            if let Some(param) = e.params.get_mut(p) {
                let n = param.keyframes.len();
                for (i, k) in param.keyframes.iter_mut().enumerate() {
                    // the fade-out keeps its place at the end; the entrance moves by `delay`
                    let is_outro = p == "opacity" && i + 2 >= n;
                    if !is_outro {
                        k.time = Tick(k.time.0.saturating_add(secs(delay).0));
                    }
                }
                if p == "opacity"
                    && let Some(first) = param.keyframes.first().cloned()
                    && delay > 0.0
                {
                    // invisible until its entrance starts
                    param.keyframes.insert(0, Keyframe::new(Tick::ZERO, first.value));
                }
            }
        }
    }
}

/// A full-frame background: a colour, or a vertical gradient.
fn backdrop(uid: u64, top: u32, bottom: Option<u32>) -> EffectInstance {
    let mut e = rect(uid, "Arka Plan", (960.0, 540.0), (1920.0, 1080.0), top, 0.0);
    if let Some(b) = bottom {
        set(&mut e, "fill_kind", ParamValue::Choice(1));
        set(&mut e, "gradient_start", rgb(top));
        set(&mut e, "gradient_end", rgb(b));
        set(&mut e, "gradient_angle", ParamValue::Float(90.0));
    }
    e
}

/// One layer group of an intro and how and when it comes in.
struct Part {
    layers: Vec<EffectInstance>,
    anim: Anim,
    delay: f64,
}

fn part(layers: Vec<EffectInstance>, anim: Anim, delay: f64) -> Part {
    Part { layers, anim, delay }
}

fn intro(id: &str, name: &str, seconds: f64, parts: Vec<Part>, controls: Vec<TemplateControl>) -> GraphicsTemplate {
    let mut layers = Vec::new();
    for mut p in parts {
        animate_from(&mut p.layers, p.anim, seconds, p.delay);
        layers.extend(p.layers);
    }
    let graphic = GraphicMeta { intro: secs(INTRO_LEN), outro: secs(OUTRO), design_in: Tick::ZERO, design_duration: secs(seconds), ..Default::default() };
    GraphicsTemplate {
        id: format!("videokurgu:intro-{id}"),
        name: format!("İntro · {name}"),
        category: "İntrolar".into(),
        description: "Videonun başı için hazır bir açılış; yazılar ve renkler değiştirilebilir.".into(),
        author: "VideoKurgu".into(),
        license: "MIT OR Apache-2.0".into(),
        tags: vec!["intro".into(), "açılış".into()],
        duration: secs(seconds),
        layers,
        graphic,
        controls,
        ..Default::default()
    }
}

fn intro_controls(bg: Option<u64>) -> Vec<TemplateControl> {
    let mut v = vec![
        ctl("title", "Başlık", 10, "text"),
        ctl("subtitle", "Alt Başlık", 11, "text"),
        ctl("title_color", "Başlık Rengi", 10, "fill_color"),
        ctl("font", "Yazı Tipi", 10, "font"),
    ];
    if let Some(b) = bg {
        v.push(ctl("bg", "Arka Plan Rengi", b, "fill_color"));
    }
    v
}

fn intros(v: &mut Vec<GraphicsTemplate>) {
    let title = |st: &Style, s: &str, size: f64| text(10, "Başlık", s, (960.0, 560.0), size, st, 1);
    let subtitle = |st: &Style, s: &str| text(11, "Alt Başlık", s, (960.0, 660.0), 46.0, st, 1);
    let line = |uid: u64, y: f64, color: u32, w: f64| rect(uid, "Çizgi", (960.0, y), (w, 6.0), color, 3.0);

    v.push(intro(
        "sade-siyah",
        "Sade Siyah",
        5.0,
        vec![
            part(vec![backdrop(1, 0x000000, None)], Anim::Fade, 0.0),
            part(vec![title(&SADE, "HİKÂYEMİZ", 120.0)], Anim::Fade, 0.3),
            part(vec![subtitle(&SADE, "Sizin hikâyeniz")], Anim::SlideUp, 0.9),
        ],
        intro_controls(Some(1)),
    ));
    v.push(intro(
        "gun-batimi",
        "Gün Batımı",
        5.0,
        vec![
            part(vec![backdrop(1, 0xf97316, Some(0x7c3aed))], Anim::Fade, 0.0),
            part(vec![title(&SADE, "YAZ TATİLİ", 140.0)], Anim::Pop, 0.4),
            part(vec![subtitle(&SADE, "Anılarımız 2026")], Anim::SlideUp, 1.0),
        ],
        intro_controls(Some(1)),
    ));
    v.push(intro(
        "neon-gece",
        "Neon Gece",
        5.0,
        vec![
            part(vec![backdrop(1, 0x0b1026, Some(0x1e1b4b))], Anim::Fade, 0.0),
            part(vec![line(2, 470.0, 0x22d3ee, 900.0), line(3, 700.0, 0xf472b6, 900.0)], Anim::SlideLeft, 0.2),
            part(vec![title(&NEON, "GECE YARISI", 130.0)], Anim::Fade, 0.6),
            part(vec![subtitle(&SADE, "Şehir ışıkları")], Anim::SlideUp, 1.1),
        ],
        intro_controls(Some(1)),
    ));
    v.push(intro(
        "sinema-perdesi",
        "Sinema Perdesi",
        6.0,
        vec![
            part(
                vec![rect(1, "Üst Bant", (960.0, 70.0), (1920.0, 140.0), 0x000000, 0.0), rect(2, "Alt Bant", (960.0, 1010.0), (1920.0, 140.0), 0x000000, 0.0)],
                Anim::Fade,
                0.0,
            ),
            part(vec![title(&SINEMA, "UZUN YOL", 120.0)], Anim::Fade, 0.5),
            part(vec![subtitle(&SINEMA, "bir kısa film")], Anim::Fade, 1.1),
        ],
        intro_controls(None),
    ));
    v.push(intro(
        "kurumsal",
        "Kurumsal",
        5.0,
        vec![
            part(vec![backdrop(1, 0x1e3a8a, Some(0x1d4ed8))], Anim::Fade, 0.0),
            part(vec![rect(2, "Vurgu", (960.0, 610.0), (240.0, 8.0), 0xfacc15, 4.0)], Anim::SlideLeft, 0.3),
            part(vec![title(&SADE, "ŞİRKET ADI", 120.0)], Anim::SlideUp, 0.5),
            part(vec![subtitle(&SADE, "Yıllık Değerlendirme Toplantısı")], Anim::Fade, 1.0),
        ],
        intro_controls(Some(1)),
    ));
    v.push(intro(
        "komik",
        "Komik",
        4.0,
        vec![
            part(vec![backdrop(1, 0xfde047, Some(0xf97316))], Anim::Fade, 0.0),
            part(
                vec![
                    glyph(2, "Yıldız 1", "\u{2b50}", (420.0, 300.0), 180.0, 0xffffff, 0x111111),
                    glyph(3, "Yıldız 2", "\u{2b50}", (1500.0, 780.0), 140.0, 0xffffff, 0x111111),
                ],
                Anim::Pop,
                0.2,
            ),
            part(vec![title(&KOMIK, "ŞAKA ZAMANI!", 150.0)], Anim::Pop, 0.5),
            part(vec![subtitle(&KONTURLU, "Gülmeye hazır mısınız?")], Anim::SlideUp, 1.0),
        ],
        intro_controls(Some(1)),
    ));
    v.push(intro(
        "retro",
        "Retro",
        5.0,
        vec![
            part(vec![backdrop(1, 0x2e1065, Some(0x0f172a))], Anim::Fade, 0.0),
            part(vec![ellipse(2, "Güneş", (960.0, 300.0), (260.0, 260.0), 0xf472b6)], Anim::SlideUp, 0.2),
            part(vec![title(&RETRO, "80'LER", 150.0)], Anim::Fade, 0.7),
            part(vec![subtitle(&SADE, "Geçmişe yolculuk")], Anim::Fade, 1.2),
        ],
        intro_controls(Some(1)),
    ));
    v.push(intro(
        "doga",
        "Doğa",
        5.0,
        vec![
            part(vec![backdrop(1, 0x14532d, Some(0x65a30d))], Anim::Fade, 0.0),
            part(vec![glyph(2, "Çiçek", "\u{2740}", (960.0, 340.0), 150.0, 0xfef9c3, 0x14532d)], Anim::Pop, 0.3),
            part(vec![title(&EL_YAZISI, "Doğanın Sesi", 130.0)], Anim::Fade, 0.6),
            part(vec![subtitle(&SADE, "Bir belgesel")], Anim::SlideUp, 1.1),
        ],
        intro_controls(Some(1)),
    ));
    v.push(intro(
        "oyun",
        "Oyun",
        4.0,
        vec![
            part(vec![backdrop(1, 0x000000, None)], Anim::Fade, 0.0),
            part(vec![title(&OYUN, "SEVİYE 1", 90.0)], Anim::Pop, 0.3),
            part(vec![subtitle(&OYUN, "BAŞLAMAK İÇİN HAZIR OL")], Anim::Fade, 1.0),
        ],
        intro_controls(Some(1)),
    ));
    v.push(intro(
        "zarif",
        "Zarif",
        6.0,
        vec![
            part(vec![backdrop(1, 0x1c1917, Some(0x292524))], Anim::Fade, 0.0),
            part(vec![line(2, 460.0, 0xd97706, 520.0), line(3, 700.0, 0xd97706, 520.0)], Anim::Fade, 0.3),
            part(vec![title(&ZARIF, "Ayşe & Mehmet", 140.0)], Anim::Fade, 0.6),
            part(vec![subtitle(&SINEMA, "12 HAZİRAN 2026")], Anim::Fade, 1.2),
        ],
        intro_controls(Some(1)),
    ));
    v.push(intro(
        "spor",
        "Spor",
        4.0,
        vec![
            part(vec![backdrop(1, 0x111827, Some(0x7f1d1d))], Anim::Fade, 0.0),
            part(vec![rect(2, "Şerit", (960.0, 560.0), (1920.0, 200.0), 0xdc2626, 0.0)], Anim::SlideLeft, 0.2),
            part(vec![title(&SPOR, "MAÇ GÜNÜ", 140.0)], Anim::Pop, 0.5),
            part(vec![subtitle(&SADE, "Final · Canlı")], Anim::SlideUp, 1.0),
        ],
        intro_controls(Some(1)),
    ));
    v.push(intro(
        "vlog",
        "Vlog",
        4.0,
        vec![
            part(vec![backdrop(1, 0xfdf2f8, Some(0xe0f2fe))], Anim::Fade, 0.0),
            part(vec![rect(2, "Zemin", (960.0, 530.0), (900.0, 200.0), 0xffffff, 100.0)], Anim::Pop, 0.2),
            part(
                vec![{
                    let mut t = title(&SADE, "GÜNLÜK VLOG", 110.0);
                    set(&mut t, "fill_color", rgb(0x1f2937));
                    set(&mut t, "shadow", ParamValue::Bool(false));
                    t
                }],
                Anim::SlideUp,
                0.5,
            ),
            part(
                vec![{
                    let mut s = subtitle(&SADE, "Bugün neler yaptık?");
                    set(&mut s, "fill_color", rgb(0xdb2777));
                    set(&mut s, "shadow", ParamValue::Bool(false));
                    s
                }],
                Anim::Fade,
                1.0,
            ),
        ],
        intro_controls(Some(1)),
    ));
}

/// "ABONE OL" -> "Abone Ol", with Turkish dotted and dotless I.
fn title_case(s: &str) -> String {
    let lower = |c: char| match c {
        'I' => "ı".to_string(),
        'İ' => "i".to_string(),
        c => c.to_lowercase().to_string(),
    };
    s.split(' ')
        .map(|w| {
            let mut cs = w.chars();
            match cs.next() {
                Some(first) => first.to_string() + &cs.map(lower).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn templates_have_unique_ids_and_valid_controls() {
        let all = templates();
        assert!(all.len() >= 70, "{}", all.len());
        let mut ids: Vec<&str> = all.iter().map(|t| t.id.as_str()).collect();
        ids.sort_unstable();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, "template ids are unique");
        for t in &all {
            let uids: Vec<u64> = t.layers.iter().filter_map(|l| l.layer.as_ref().map(|x| x.uid)).collect();
            for c in &t.controls {
                assert!(uids.contains(&c.layer), "{}: control {} names a layer that is not there", t.id, c.id);
            }
            assert!(t.graphic.intro.0 > 0 && t.graphic.outro.0 > 0);
        }
    }

    #[test]
    fn title_case_handles_turkish_i() {
        assert_eq!(title_case("ABONE OL"), "Abone Ol");
        assert_eq!(title_case("TAKİP ET"), "Takip Et");
        assert_eq!(title_case("BİLDİRİMLERİ AÇ"), "Bildirimleri Aç");
        assert_eq!(title_case("İNDİRİM %50"), "İndirim %50");
    }
}
