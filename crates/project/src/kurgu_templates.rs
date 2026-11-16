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

const SADE: Style = Style { shadow: Some((0x000000, 70.0, 4.0, 18.0)), ..style("sade", "Sade", "Inter", "Bold", 0xffffff) };
const KUTULU: Style = Style { boxed: Some((0x000000, 72.0, 22.0, 14.0)), ..style("kutulu", "Kutulu", "Inter", "SemiBold", 0xffffff) };
const KONTURLU: Style = Style { stroke: Some((0x000000, 7.0)), ..style("konturlu", "Konturlu", "Inter", "Bold", 0xffffff) };
const NEON: Style = Style { stroke: Some((0x22d3ee, 3.0)), shadow: Some((0x22d3ee, 100.0, 0.0, 34.0)), ..style("neon", "Neon", "Inter", "Bold", 0xecfeff) };
const GRADYAN: Style =
    Style { gradient: Some((0xf97316, 0xec4899)), shadow: Some((0x000000, 55.0, 4.0, 16.0)), ..style("gradyan", "Gradyan", "Inter", "Bold", 0xffffff) };
const SINEMA: Style = Style { tracking: 220.0, shadow: Some((0x000000, 60.0, 2.0, 20.0)), ..style("sinema", "Sinema", "Noto Serif", "Regular", 0xf5f5f4) };
const PASTEL: Style = Style { boxed: Some((0xfde68a, 100.0, 24.0, 26.0)), ..style("pastel", "Pastel", "Inter", "Bold", 0x1f2937) };
const DAKTILO: Style = Style { boxed: Some((0x000000, 85.0, 18.0, 6.0)), ..style("daktilo", "Daktilo", "JetBrains Mono", "Regular", 0x4ade80) };
const ALTIN: Style = Style {
    gradient: Some((0xfde68a, 0xb45309)),
    stroke: Some((0x3b2106, 2.0)),
    shadow: Some((0x000000, 60.0, 3.0, 14.0)),
    ..style("altin", "Altın", "Noto Serif", "Regular", 0xfde68a)
};
const KALIN_GOLGE: Style = Style { shadow: Some((0x000000, 100.0, 9.0, 0.0)), ..style("kalin-golge", "Kalın Gölge", "Inter", "Bold", 0xfacc15) };

const STYLES: [Style; 10] = [SADE, KUTULU, KONTURLU, NEON, GRADYAN, SINEMA, PASTEL, DAKTILO, ALTIN, KALIN_GOLGE];

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
    for st in [SADE, KUTULU, KONTURLU, PASTEL, DAKTILO, KALIN_GOLGE] {
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
    for st in [SADE, SINEMA, GRADYAN, KUTULU] {
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
    for st in [SINEMA, SADE, ALTIN] {
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
    ];
    for (id, label, st, anim) in cards {
        let t = text(1, "Metin", label, (960.0, 570.0), 96.0, &st, 1);
        v.push(template(
            format!("kapanis-{id}"),
            format!("Kapanış · {label}"),
            "Kapanış",
            "Videonun sonu için bir kart.",
            5.0,
            vec![t],
            anim,
            text_controls(&[(1, "Metin")], 1, boxed_uid(&st, 1)),
        ));
    }
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
