//! Brush titles, ornamented titles and social account cards. Every outline here is original work
//! computed in code: no third party's logo or artwork. The social cards name the platform in words
//! beside a generic icon (repository rule 1.8: no third-party marks).
//!
//! The bands are pinned to their text (Responsive Design – Position), so they grow with it. A layer
//! pinned left and right has its width and horizontal place set by the pin: animate only its
//! opacity, height and vertical place, or move the text it is pinned to.

use std::f32::consts::{PI, TAU};

use super::{
    EffectInstance, GraphicsTemplate, Interpolation, LayerExtra, ParamValue, STYLISH, Style, TemplateControl, Vec2, appear, baseline, broadcast, ctl, ellipse,
    gradient_fill, keys, new_shape_layer, news_style, pop_in, pulse, rgb, s, set, shadowed, slide_from, text,
};
use crate::graphic_design::{Pin, PinTarget};

type Contour = Vec<[f32; 2]>;

const BRUSH: &str = "Brush Titles";
const SOCIAL: &str = "Social Media";
const ACCOUNT: &str = "FreewareTR";

pub(super) fn add(v: &mut Vec<GraphicsTemplate>) {
    brushes(v);
    ornamented(v);
    subscribe_and_like(v);
    accounts(v);
    all_accounts(v);
}

// ---- drawing

/// Repeatable jitter, so a template is the same each time it is made.
struct Jitter(u32);

impl Jitter {
    fn next(&mut self, lo: f32, hi: f32) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        lo + (hi - lo) * ((self.0 >> 8) as f32 / 16_777_216.0)
    }
}

/// A shape layer of several contours in one colour. A shape holds one closed path, so each contour
/// is reached from the first by an edge walked there and back, which encloses no area. A contour
/// wound the other way inside another is a hole.
fn drawing(uid: u64, name: &str, at: (f64, f64), color: u32, parts: &[Contour]) -> EffectInstance {
    let mut pts: Contour = Vec::new();
    let mut origin = None;
    for part in parts {
        let Some(first) = part.first().copied() else { continue };
        pts.extend(part.iter().copied());
        pts.push(first);
        match origin {
            None => origin = Some(first),
            Some(o) => pts.push(o),
        }
    }
    for p in &mut pts {
        *p = [(p[0] * 10.0).round() / 10.0, (p[1] * 10.0).round() / 10.0];
    }
    let (w, h) = pts.iter().fold((0.0f32, 0.0f32), |(w, h), p| (w.max(p[0].abs()), h.max(p[1].abs())));
    let mut e = new_shape_layer(3, Vec2::new(at.0, at.1), Vec2::new(f64::from(w * 2.0).round(), f64::from(h * 2.0).round()), pts);
    set(&mut e, "name", ParamValue::Text(name.into()));
    set(&mut e, "fill_color", rgb(color));
    e.layer = Some(Box::new(LayerExtra { uid, ..Default::default() }));
    e
}

fn hole(mut c: Contour) -> Contour {
    c.reverse();
    c
}

fn oval(x: f32, y: f32, rx: f32, ry: f32) -> Contour {
    (0..28).map(|i| TAU * i as f32 / 28.0).map(|a| [x + rx * a.cos(), y + ry * a.sin()]).collect()
}

fn round_box(x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> Contour {
    let mut out = Vec::new();
    for (cx, cy, from) in [(x1 - r, y0 + r, -PI / 2.0), (x1 - r, y1 - r, 0.0), (x0 + r, y1 - r, PI / 2.0), (x0 + r, y0 + r, PI)] {
        for i in 0..=6 {
            let a = from + PI / 2.0 * i as f32 / 6.0;
            out.push([cx + r * a.cos(), cy + r * a.sin()]);
        }
    }
    out
}

/// A box with its corners cut off.
fn cut_box(w: f32, h: f32, cut: f32) -> Contour {
    vec![[-w + cut, -h], [w - cut, -h], [w, -h + cut], [w, h - cut], [w - cut, h], [-w + cut, h], [-w, h - cut], [-w, -h + cut]]
}

/// A dry brush stroke `w` each way from the middle: smooth where the title is, its edges a little
/// rough, with bristle marks at both ends (longer where the brush left the surface).
fn dry_stroke(seed: u32, w: f32, h: f32) -> Vec<Contour> {
    let mut r = Jitter(seed);
    let mut body: Contour = Vec::new();
    // a slow swell under the fine roughness, so the edge is hand drawn and not a saw
    for i in 0..=56 {
        let t = i as f32 / 56.0;
        body.push([-w + 2.0 * w * t, -h - 3.0 * (t * 8.0).sin() + r.next(-2.2, 2.2) + 5.0 * t]);
    }
    let n = 17;
    for i in 0..n {
        let y = |f: f32| -h + 5.0 + (2.0 * h - 7.0) * (i as f32 + f) / n as f32;
        let long = if i % 3 == 1 { r.next(80.0, 135.0) } else { r.next(18.0, 70.0) };
        body.push([w + long * 0.55, y(0.22) + r.next(-1.0, 1.0)]);
        body.push([w + long, y(0.5)]);
        body.push([w + long * 0.3, y(0.76)]);
        body.push([w - r.next(5.0, 75.0), y(0.98)]);
    }
    for i in 0..=56 {
        let t = 1.0 - i as f32 / 56.0;
        body.push([-w + 2.0 * w * t, h + 3.0 * (t * 6.0).sin() + r.next(-2.2, 2.2) - 3.0 * t]);
    }
    let n = 15;
    for i in 0..n {
        let y = |f: f32| h - 2.0 * h * (i as f32 + f) / n as f32;
        // rounded where the brush was pressed down
        let press = (1.0 - (y(0.5) / h).powi(2)).max(0.0).sqrt() * 34.0;
        let long = press + if i % 4 == 1 { r.next(60.0, 100.0) } else { r.next(6.0, 44.0) };
        body.push([-w - long * 0.6, y(0.2)]);
        body.push([-w - long, y(0.5)]);
        body.push([-w - long * 0.5, y(0.78)]);
        body.push([-w + r.next(5.0, 60.0), y(0.98)]);
    }
    let mut out = vec![body];
    // loose hairs past both ends
    for i in 0..7 {
        let y = -h + 2.0 * h * (i as f32 + 0.5) / 7.0 + r.next(-4.0, 4.0);
        for (from, dir) in [(w + r.next(142.0, 160.0), 1.0f32), (-w - r.next(120.0, 140.0), -1.0)] {
            let (len, t) = (r.next(14.0, 52.0), r.next(1.2, 3.2));
            let hair = vec![[from, y - t], [from + dir * len, y], [from, y + t]];
            out.push(if dir > 0.0 { hair } else { hole(hair) });
        }
    }
    out
}

/// An ink stroke: a blot at the start, a swelling body, split tapering tips at the end, drops.
fn ink_stroke(seed: u32, w: f32, h: f32) -> Vec<Contour> {
    let mut r = Jitter(seed);
    let mut body: Contour = Vec::new();
    let sag = |t: f32| 9.0 * (1.0 - 2.0 * t).powi(2);
    let half = |t: f32| h * (0.88 + 0.12 * (PI * t).sin());
    for i in 0..=48 {
        let t = i as f32 / 48.0;
        body.push([-w + 2.0 * w * t, sag(t) - half(t) + r.next(-1.5, 1.5)]);
    }
    let tips = [(-0.88, 150.0), (-0.6, 70.0), (-0.38, 115.0), (-0.1, 48.0), (0.12, 95.0), (0.36, 40.0), (0.56, 78.0), (0.8, 30.0)];
    for (y, long) in tips {
        body.push([w + long * 0.45, y * h + 5.0]);
        body.push([w + long, y * h + 3.0]);
        body.push([w + long * 0.3, y * h + 14.0]);
        body.push([w - r.next(4.0, 30.0), y * h + 19.0]);
    }
    for i in 0..=48 {
        let t = 1.0 - i as f32 / 48.0;
        body.push([-w + 2.0 * w * t, sag(t) + half(t) + r.next(-1.5, 1.5)]);
    }
    for i in 1..30 {
        let a = PI / 2.0 + PI * i as f32 / 30.0;
        let spike = if i % 3 == 0 { r.next(30.0, 78.0) } else { r.next(-16.0, 4.0) };
        body.push([-w + (70.0 + spike) * a.cos(), 9.0 + (h * 0.88 + spike * 0.3) * a.sin()]);
    }
    let mut out = vec![body];
    for (x, y, size) in
        [(-w - 160.0, -26.0, 8.0), (-w - 182.0, 22.0, 4.0), (-w - 130.0, 58.0, 5.0), (w + 170.0, 8.0, 5.0), (w + 135.0, 52.0, 7.0), (w + 198.0, -38.0, 3.5)]
    {
        out.push(oval(x, y, size, size * 0.85));
    }
    out
}

/// A laurel branch bending round the side `side` (-1 left, 1 right) of a plaque.
fn laurel(side: f32) -> Vec<Contour> {
    let mut out = Vec::new();
    for i in 0..6 {
        let t = i as f32 / 5.0;
        let (x, y) = (side * (34.0 * (PI * t).sin() + 4.0), 66.0 - 132.0 * t);
        for lean in [-1.0f32, 1.0] {
            let a = -PI / 2.0 + lean * 0.75 + side * (0.5 - t) * 0.9;
            let (dx, dy) = (a.cos(), a.sin());
            out.push(vec![
                [x, y],
                [x + 13.0 * dx + 7.0 * dy, y + 13.0 * dy - 7.0 * dx],
                [x + 31.0 * dx, y + 31.0 * dy],
                [x + 13.0 * dx - 7.0 * dy, y + 13.0 * dy + 7.0 * dx],
            ]);
        }
    }
    out
}

fn torn(seed: u32, w: f32, h: f32) -> Contour {
    let mut r = Jitter(seed);
    let mut out = Vec::new();
    let steps = 46;
    for i in 0..=steps {
        out.push([-w + 2.0 * w * i as f32 / steps as f32, -h + r.next(-6.0, 6.0)]);
    }
    for i in 1..8 {
        out.push([w + r.next(-7.0, 9.0), -h + 2.0 * h * i as f32 / 8.0]);
    }
    for i in (0..=steps).rev() {
        out.push([-w + 2.0 * w * i as f32 / steps as f32, h + r.next(-6.0, 6.0)]);
    }
    for i in (1..8).rev() {
        out.push([-w + r.next(-9.0, 7.0), -h + 2.0 * h * i as f32 / 8.0]);
    }
    out
}

// ---- layout and motion

fn pin(e: &mut EffectInstance, to: u64, left: Option<f64>, right: Option<f64>) {
    if let Some(extra) = e.layer.as_mut() {
        extra.pin = Some(Pin {
            to: PinTarget::Layer(to),
            left: left.is_some(),
            right: right.is_some(),
            offsets: [left.unwrap_or(0.0), 0.0, right.unwrap_or(0.0), 0.0],
            ..Default::default()
        });
    }
}

/// Opens from its middle to full height (safe on a layer pinned left and right).
fn unfold(e: &mut EffectInstance, delay: f64) {
    set(e, "uniform_scale", ParamValue::Bool(false));
    keys(
        e,
        "scale",
        ParamValue::Float(100.0),
        &[(delay, ParamValue::Float(4.0), Interpolation::EaseOut), (delay + 0.4, ParamValue::Float(100.0), Interpolation::Linear)],
    );
}

fn title(uid: u64, size: f64, c: (f64, f64), st: &Style) -> EffectInstance {
    text(uid, "Title", s("YOUR TITLE HERE", "BAŞLIĞINIZ BURADA"), (c.0, baseline(c.1, size)), size, st, 1)
}

fn title_controls(uid: u64) -> Vec<TemplateControl> {
    let mut size = ctl("size", "Font Size", uid, "size");
    (size.min, size.max) = (Some(16.0), Some(160.0));
    vec![ctl("title", "Title", uid, "text"), ctl("text_color", "Text Color", uid, "fill_color"), ctl("font", "Font", uid, "font"), size]
}

// ---- the templates

fn brushes(v: &mut Vec<GraphicsTemplate>) {
    let secs_ = 6.0;
    let c = (960.0, 890.0);
    for ink in [false, true] {
        let (font, light, dark, under) = if ink { ("Kaushan Script", 0x334155, 0x020617, 0xfacc15) } else { ("Anton", 0xf97316, 0xb91c1c, 0x450a0a) };
        let (parts, below) =
            if ink { (ink_stroke(41, 420.0, 70.0), ink_stroke(43, 420.0, 70.0)) } else { (dry_stroke(7, 420.0, 70.0), dry_stroke(19, 420.0, 70.0)) };
        let mut back = drawing(1, "Second Stroke", (c.0, c.1 + 12.0), under, &below);
        let mut stroke = gradient_fill(drawing(2, "Brush Stroke", c, light, &parts), light, dark, 0.0);
        let mut t = title(3, 62.0, c, &news_style(font, "Regular", 0xffffff));
        // room for the bristles, which stretch with the stroke
        let (before, after) = if ink { (290.0, 345.0) } else { (290.0, 285.0) };
        pin(&mut back, 3, Some(20.0 - before), Some(after - 20.0));
        pin(&mut stroke, 3, Some(-before), Some(after));
        // the strokes are pinned to the title, so moving it sweeps the whole stroke in
        slide_from(&mut t, -280.0, 0.0, 0.0);
        appear(&mut back, 0.12, secs_);
        appear(&mut stroke, 0.0, secs_);
        appear(&mut t, 0.2, secs_);
        let mut controls = title_controls(3);
        controls.extend([
            ctl("brush", "Brush Color", 2, "gradient_start"),
            ctl("brush2", "Brush End Color", 2, "gradient_end"),
            ctl("under", "Second Stroke Color", 1, "fill_color"),
            ctl("under_on", "Second Stroke", 1, "enabled"),
        ]);
        v.push(broadcast(
            if ink { "firca-murekkep" } else { "firca-kuru" },
            if ink { "Brush Stroke · Ink" } else { "Brush Stroke · Dry Brush" }.into(),
            BRUSH,
            "A brush stroke with bristle marks at both ends and a smooth middle for the title. It grows with the text.",
            secs_,
            vec![back, stroke, t],
            controls,
        ));
    }
}

fn ornamented(v: &mut Vec<GraphicsTemplate>) {
    let secs_ = 6.0;
    let c = (960.0, 890.0);
    let mut push = |id: &str, name: &str, description: &str, layers: Vec<EffectInstance>, uid: u64, colors: Vec<TemplateControl>| {
        let mut controls = title_controls(uid);
        controls.extend(colors);
        v.push(broadcast(id, name.into(), STYLISH, description, secs_, layers, controls));
    };

    {
        let plate = |w: f32, h: f32| vec![[-w + 50.0, -h], [w - 50.0, -h], [w, 0.0], [w - 50.0, h], [-w + 50.0, h], [-w, 0.0]];
        let mut left = gradient_fill(drawing(1, "Laurel", (470.0, c.1), 0xeab308, &laurel(-1.0)), 0xfde68a, 0xb45309, 90.0);
        let mut right = gradient_fill(drawing(2, "Laurel", (1450.0, c.1), 0xeab308, &laurel(1.0)), 0xfde68a, 0xb45309, 90.0);
        let mut rim = shadowed(gradient_fill(drawing(3, "Rim", c, 0xeab308, &[plate(440.0, 66.0)]), 0xfde68a, 0x92400e, 90.0), 0x000000, 55.0);
        let mut face = gradient_fill(drawing(4, "Plaque", c, 0x172554, &[plate(428.0, 57.0)]), 0x1e3a8a, 0x0b1638, 90.0);
        let mut t = title(5, 56.0, c, &news_style("Cinzel Decorative", "Bold", 0xfde68a));
        pin(&mut left, 3, Some(-86.0), None);
        pin(&mut right, 3, None, Some(86.0));
        pin(&mut rim, 5, Some(-92.0), Some(92.0));
        pin(&mut face, 5, Some(-78.0), Some(78.0));
        unfold(&mut rim, 0.0);
        unfold(&mut face, 0.08);
        for (e, d) in [(&mut left, 0.4), (&mut right, 0.4), (&mut rim, 0.0), (&mut face, 0.08), (&mut t, 0.4)] {
            appear(e, d, secs_);
        }
        push(
            "kraliyet-arma",
            "Royal Emblem",
            "A navy plaque with a gold rim between two laurel branches.",
            vec![left, right, rim, face, t],
            5,
            vec![
                ctl("plate", "Plaque Color", 4, "gradient_start"),
                ctl("plate2", "Plaque Shade", 4, "gradient_end"),
                ctl("rim", "Rim Color", 3, "gradient_start"),
                ctl("rim2", "Rim Shade", 3, "gradient_end"),
                ctl("leaf_l", "Left Branch Color", 1, "gradient_start"),
                ctl("leaf_r", "Right Branch Color", 2, "gradient_start"),
            ],
        );
    }

    {
        let tail = |d: f32| {
            let t = vec![[-85.0 * d, -50.0], [85.0 * d, -50.0], [85.0 * d, 50.0], [-85.0 * d, 50.0], [-45.0 * d, 0.0]];
            if d > 0.0 { t } else { hole(t) }
        };
        let mut tail_l = gradient_fill(drawing(1, "Tail", (470.0, c.1 + 30.0), 0x991b1b, &[tail(1.0)]), 0xb91c1c, 0x7f1d1d, 90.0);
        let mut tail_r = gradient_fill(drawing(2, "Tail", (1450.0, c.1 + 30.0), 0x991b1b, &[tail(-1.0)]), 0xb91c1c, 0x7f1d1d, 90.0);
        let mut fold_l = drawing(3, "Fold", (530.0, c.1 + 69.0), 0x450a0a, &[vec![[-32.0, -14.0], [32.0, -14.0], [32.0, 14.0]]]);
        let mut fold_r = drawing(4, "Fold", (1390.0, c.1 + 69.0), 0x450a0a, &[vec![[-32.0, -14.0], [32.0, -14.0], [-32.0, 14.0]]]);
        let mut band = gradient_fill(drawing(5, "Banner", c, 0xdc2626, &[round_box(-460.0, -56.0, 460.0, 56.0, 3.0)]), 0xef4444, 0x991b1b, 90.0);
        let seams = [round_box(-440.0, -45.0, 440.0, -41.0, 1.0), round_box(-440.0, 41.0, 440.0, 45.0, 1.0)];
        let mut seam = drawing(6, "Stripes", c, 0xfde68a, &seams);
        let mut t = title(7, 56.0, c, &news_style("Alfa Slab One", "Regular", 0xffffff));
        pin(&mut tail_l, 5, Some(-100.0), None);
        pin(&mut tail_r, 5, None, Some(100.0));
        pin(&mut fold_l, 5, Some(0.0), None);
        pin(&mut fold_r, 5, None, Some(0.0));
        pin(&mut band, 7, Some(-70.0), Some(70.0));
        pin(&mut seam, 7, Some(-70.0), Some(70.0));
        unfold(&mut band, 0.0);
        for (e, d) in [(&mut tail_l, 0.3), (&mut tail_r, 0.3), (&mut fold_l, 0.3), (&mut fold_r, 0.3), (&mut band, 0.0), (&mut seam, 0.3), (&mut t, 0.35)] {
            appear(e, d, secs_);
        }
        push(
            "katlamali-flama",
            "Folded Banner",
            "A folded banner with swallow tails and two fine stripes.",
            vec![tail_l, tail_r, fold_l, fold_r, band, seam, t],
            7,
            vec![
                ctl("band", "Band Color", 5, "gradient_start"),
                ctl("band2", "Band Shade", 5, "gradient_end"),
                ctl("tail_l", "Left Tail Color", 1, "gradient_start"),
                ctl("tail_r", "Right Tail Color", 2, "gradient_start"),
                ctl("stripe", "Stripe Color", 6, "fill_color"),
            ],
        );
    }

    {
        let mut rim = shadowed(gradient_fill(drawing(1, "Rim", c, 0xcbd5e1, &[cut_box(440.0, 68.0, 22.0)]), 0xf8fafc, 0x475569, 90.0), 0x000000, 55.0);
        let mut face = gradient_fill(drawing(2, "Plate", c, 0x94a3b8, &[cut_box(430.0, 58.0, 17.0)]), 0x64748b, 0xe2e8f0, 90.0);
        let mut shine =
            gradient_fill(drawing(3, "Shine", (c.0, c.1 - 25.0), 0xffffff, &[round_box(-420.0, -22.0, 420.0, 22.0, 8.0)]), 0xffffff, 0x94a3b8, 90.0);
        let rivets = |uid| drawing(uid, "Rivets", c, 0x334155, &[oval(0.0, -36.0, 7.0, 7.0), oval(0.0, 36.0, 7.0, 7.0)]);
        let (mut rivet_l, mut rivet_r) = (rivets(4), rivets(5));
        let engraved = Style { shadow: Some((0xffffff, 75.0, 2.0, 1.0)), ..news_style("Archivo Black", "Regular", 0x1e293b) };
        let mut t = title(6, 54.0, c, &engraved);
        pin(&mut rim, 6, Some(-86.0), Some(86.0));
        pin(&mut face, 6, Some(-76.0), Some(76.0));
        pin(&mut shine, 6, Some(-68.0), Some(68.0));
        pin(&mut rivet_l, 2, Some(18.0), None);
        pin(&mut rivet_r, 2, None, Some(-18.0));
        unfold(&mut rim, 0.0);
        unfold(&mut face, 0.06);
        for (e, d) in [(&mut rim, 0.0), (&mut face, 0.06), (&mut shine, 0.3), (&mut rivet_l, 0.4), (&mut rivet_r, 0.4), (&mut t, 0.35)] {
            appear(e, d, secs_);
        }
        push(
            "metal-plaka",
            "Metal Plate",
            "A polished metal plate with a bevel, rivets and an engraved title.",
            vec![rim, face, shine, rivet_l, rivet_r, t],
            6,
            vec![
                ctl("rim", "Rim Color", 1, "gradient_start"),
                ctl("rim2", "Rim Shade", 1, "gradient_end"),
                ctl("plate", "Plate Color", 2, "gradient_end"),
                ctl("plate2", "Plate Shade", 2, "gradient_start"),
                ctl("shine", "Shine", 3, "enabled"),
                ctl("shine_color", "Shine Color", 3, "gradient_start"),
            ],
        );
    }

    {
        let mut back = shadowed(drawing(1, "Back Paper", (c.0, c.1 + 10.0), 0xb45309, &[torn(5, 440.0, 64.0)]), 0x000000, 50.0);
        let mut paper = gradient_fill(drawing(2, "Paper", c, 0xfef3c7, &[torn(11, 440.0, 62.0)]), 0xfffbeb, 0xe9d8b4, 90.0);
        let strip = vec![[-62.0, 8.0], [-56.0, -2.0], [-60.0, -10.0], [52.0, -34.0], [60.0, -24.0], [56.0, -14.0], [62.0, -6.0], [-52.0, 20.0]];
        let mut tape_l = drawing(3, "Tape", (530.0, c.1 - 52.0), 0x38bdf8, std::slice::from_ref(&strip));
        let mirrored: Contour = strip.iter().rev().map(|p| [-p[0], p[1]]).collect();
        let mut tape_r = drawing(4, "Tape", (1390.0, c.1 - 52.0), 0x38bdf8, &[mirrored]);
        let mut t = title(5, 56.0, c, &news_style("Special Elite", "Regular", 0x292524));
        pin(&mut back, 5, Some(-64.0), Some(88.0));
        pin(&mut paper, 5, Some(-78.0), Some(78.0));
        pin(&mut tape_l, 2, Some(-38.0), None);
        pin(&mut tape_r, 2, None, Some(38.0));
        for (e, d) in [(&mut back, 0.0), (&mut paper, 0.05)] {
            slide_from(e, 0.0, 90.0, d);
            appear(e, d, secs_);
        }
        for e in [&mut tape_l, &mut tape_r] {
            slide_from(e, 0.0, -40.0, 0.4);
            appear(e, 0.4, secs_);
        }
        appear(&mut t, 0.35, secs_);
        push(
            "yirtik-kagit",
            "Torn Paper",
            "A torn strip of paper taped over a second one.",
            vec![back, paper, tape_l, tape_r, t],
            5,
            vec![
                ctl("paper", "Paper Color", 2, "gradient_start"),
                ctl("paper2", "Paper Shade", 2, "gradient_end"),
                ctl("back", "Back Paper Color", 1, "fill_color"),
                ctl("tape_l", "Left Tape Color", 3, "fill_color"),
                ctl("tape_r", "Right Tape Color", 4, "fill_color"),
            ],
        );
    }

    {
        let mut glow = gradient_fill(drawing(1, "Light Edge", c, 0x22d3ee, &[cut_box(440.0, 66.0, 28.0)]), 0x22d3ee, 0xe879f9, 0.0);
        set(&mut glow, "shadow", ParamValue::Bool(true));
        set(&mut glow, "shadow_color", rgb(0x67e8f9));
        set(&mut glow, "shadow_opacity", ParamValue::Float(90.0));
        set(&mut glow, "shadow_distance", ParamValue::Float(0.0));
        set(&mut glow, "shadow_blur", ParamValue::Float(34.0));
        let mut face = gradient_fill(drawing(2, "Panel", c, 0x0b1020, &[cut_box(435.0, 61.0, 25.0)]), 0x172033, 0x030712, 90.0);
        let chevrons = |d: f32| -> Vec<Contour> {
            [0.0f32, 22.0, 44.0]
                .into_iter()
                .map(|x| {
                    let p: Contour = [[-6.0f32, -20.0], [2.0, -20.0], [16.0, 0.0], [2.0, 20.0], [-6.0, 20.0], [8.0, 0.0]]
                        .iter()
                        .map(|q| [d * (q[0] + x - 22.0), q[1]])
                        .collect();
                    if d > 0.0 { p } else { hole(p) }
                })
                .collect()
        };
        let mut arrows_l = drawing(3, "Arrows", (500.0, c.1), 0x22d3ee, &chevrons(1.0));
        let mut arrows_r = drawing(4, "Arrows", (1420.0, c.1), 0xe879f9, &chevrons(-1.0));
        let lit = Style { shadow: Some((0x67e8f9, 90.0, 0.0, 16.0)), ..news_style("Righteous", "Regular", 0xecfeff) };
        let mut t = title(5, 58.0, c, &lit);
        pin(&mut glow, 5, Some(-120.0), Some(120.0));
        pin(&mut face, 5, Some(-115.0), Some(115.0));
        pin(&mut arrows_l, 2, Some(22.0), None);
        pin(&mut arrows_r, 2, None, Some(-22.0));
        unfold(&mut glow, 0.0);
        unfold(&mut face, 0.05);
        for (e, d) in [(&mut glow, 0.0), (&mut face, 0.05), (&mut arrows_l, 0.45), (&mut arrows_r, 0.55), (&mut t, 0.35)] {
            appear(e, d, secs_);
        }
        push(
            "neon-panel",
            "Neon Panel",
            "A dark panel with a glowing two-colour edge and arrows.",
            vec![glow, face, arrows_l, arrows_r, t],
            5,
            vec![
                ctl("edge", "Edge Color", 1, "gradient_start"),
                ctl("edge2", "Edge End Color", 1, "gradient_end"),
                ctl("glow", "Glow Color", 1, "shadow_color"),
                ctl("panel", "Panel Color", 2, "gradient_start"),
                ctl("arrows_l", "Left Arrows Color", 3, "fill_color"),
                ctl("arrows_r", "Right Arrows Color", 4, "fill_color"),
            ],
        );
    }
}

/// Generic icons: what each kind of place is for, never its logo.
#[derive(Clone, Copy)]
enum Sign {
    People,
    Photo,
    Bubble,
    Play,
    Phone,
    Briefcase,
    Chats,
    Dots,
    Globe,
    Envelope,
}

impl Sign {
    fn parts(self) -> Vec<Contour> {
        match self {
            Sign::People => {
                vec![oval(-11.0, -12.0, 10.0, 10.0), round_box(-28.0, 2.0, 6.0, 22.0, 8.0), oval(17.0, -9.0, 8.0, 8.0), round_box(10.0, 3.0, 29.0, 20.0, 7.0)]
            }
            Sign::Photo => vec![
                round_box(-28.0, -22.0, 28.0, 22.0, 4.0),
                hole(round_box(-23.0, -17.0, 23.0, 17.0, 2.0)),
                vec![[-20.0, 14.0], [-7.0, -6.0], [2.0, 6.0], [9.0, -2.0], [20.0, 14.0]],
                oval(14.0, -9.0, 4.0, 4.0),
            ],
            Sign::Bubble => {
                vec![round_box(-27.0, -23.0, 27.0, 14.0, 10.0), vec![[-5.0, 14.0], [-14.0, 27.0], [-16.0, 14.0]], hole(round_box(-16.0, -8.0, 16.0, -3.0, 2.0))]
            }
            Sign::Play => vec![oval(0.0, 0.0, 28.0, 28.0), hole(vec![[-8.0, -14.0], [16.0, 0.0], [-8.0, 14.0]])],
            Sign::Phone => {
                vec![round_box(-18.0, -28.0, 18.0, 28.0, 6.0), hole(round_box(-14.0, -21.0, 14.0, 19.0, 2.0)), vec![[-6.0, -10.0], [9.0, -1.0], [-6.0, 8.0]]]
            }
            Sign::Briefcase => vec![
                round_box(-28.0, -14.0, 28.0, 22.0, 5.0),
                hole(round_box(-6.0, -1.0, 6.0, 8.0, 2.0)),
                vec![[-12.0, -14.0], [-12.0, -24.0], [12.0, -24.0], [12.0, -14.0], [7.0, -14.0], [7.0, -19.0], [-7.0, -19.0], [-7.0, -14.0]],
            ],
            Sign::Chats => {
                vec![round_box(-29.0, -24.0, 13.0, 4.0, 8.0), vec![[-14.0, 4.0], [-23.0, 13.0], [-24.0, 4.0]], round_box(-4.0, 9.0, 29.0, 25.0, 7.0)]
            }
            Sign::Dots => {
                let mut v = vec![round_box(-28.0, -22.0, 28.0, 16.0, 12.0), vec![[16.0, 16.0], [18.0, 28.0], [4.0, 16.0]]];
                v.extend([-14.0, 0.0, 14.0].map(|x| hole(oval(x, -3.0, 4.5, 4.5))));
                v
            }
            Sign::Globe => {
                let mut v = vec![oval(0.0, 0.0, 28.0, 28.0)];
                for (x0, y0, x1, y1) in [(-22.0, -15.0, -4.0, -3.0), (4.0, -15.0, 22.0, -3.0), (-22.0, 3.0, -4.0, 15.0), (4.0, 3.0, 22.0, 15.0)] {
                    v.push(hole(round_box(x0, y0, x1, y1, 5.0)));
                }
                v
            }
            Sign::Envelope => vec![
                round_box(-28.0, -20.0, 28.0, 20.0, 4.0),
                hole(vec![[-23.0, -16.0], [23.0, -16.0], [0.0, 2.0]]),
                hole(vec![[-23.0, -8.0], [0.0, 10.0], [23.0, -8.0], [23.0, 15.0], [-23.0, 15.0]]),
            ],
        }
    }
}

/// (id, the name shown, the icon, the badge's two colours).
const PLACES: [(&str, &str, Sign, u32, u32); 10] = [
    ("facebook", "Facebook", Sign::People, 0x3b82f6, 0x1d4ed8),
    ("instagram", "Instagram", Sign::Photo, 0xf59e0b, 0xc026d3),
    ("x", "X", Sign::Bubble, 0x404040, 0x0a0a0a),
    ("youtube", "YouTube", Sign::Play, 0xef4444, 0xb91c1c),
    ("tiktok", "TikTok", Sign::Phone, 0x0891b2, 0xbe185d),
    ("linkedin", "LinkedIn", Sign::Briefcase, 0x0284c7, 0x075985),
    ("telegram", "Telegram", Sign::Chats, 0x38bdf8, 0x0369a1),
    ("whatsapp", "WhatsApp", Sign::Dots, 0x22c55e, 0x15803d),
    ("web", "Website", Sign::Globe, 0x14b8a6, 0x0f766e),
    ("eposta", "E-mail", Sign::Envelope, 0x8b5cf6, 0x5b21b6),
];

fn accounts(v: &mut Vec<GraphicsTemplate>) {
    let secs_ = 6.0;
    for (id, name, sign, a, b) in PLACES {
        let label = match id {
            "web" => s("Website", "Web Sitesi"),
            "eposta" => s("E-mail", "E-posta"),
            _ => name,
        };
        let y = 900.0;
        let mut bar = shadowed(
            gradient_fill(drawing(1, "Bar", (500.0, y), 0x111827, &[round_box(-300.0, -62.0, 300.0, 62.0, 30.0)]), 0x1f2937, 0x030712, 90.0),
            0x000000,
            50.0,
        );
        let mut badge = gradient_fill(ellipse(2, "Badge", (170.0, y), (112.0, 112.0), a), a, b, 45.0);
        let mut icon = drawing(3, "Icon", (170.0, y), 0xffffff, &sign.parts());
        let mut place = text(4, "Platform", label, (252.0, baseline(y - 27.0, 27.0)), 27.0, &news_style("Poppins", "Medium", 0xcbd5e1), 0);
        let mut account = text(5, "Address", ACCOUNT, (252.0, baseline(y + 18.0, 46.0)), 46.0, &news_style("Poppins", "Bold", 0xffffff), 0);
        pin(&mut bar, 5, Some(-158.0), Some(48.0));
        for (e, d) in [(&mut bar, 0.0), (&mut badge, 0.05), (&mut icon, 0.2), (&mut place, 0.3), (&mut account, 0.38)] {
            appear(e, d, secs_);
        }
        unfold(&mut bar, 0.0);
        pop_in(&mut badge, 0.05);
        pop_in(&mut icon, 0.2);
        v.push(broadcast(
            &format!("hesap-{id}"),
            format!("Social Account · {name}"),
            SOCIAL,
            "Your address on a platform, with an icon. Switch the icon off to put a picture of your own there.",
            secs_,
            vec![bar, badge, icon, place, account],
            vec![
                ctl("account", "Address", 5, "text"),
                ctl("platform", "Platform", 4, "text"),
                ctl("text_color", "Text Color", 5, "fill_color"),
                ctl("font", "Font", 5, "font"),
                ctl("bar", "Bar Color", 1, "gradient_start"),
                ctl("bar2", "Bar Shade", 1, "gradient_end"),
                ctl("badge", "Badge Color", 2, "gradient_start"),
                ctl("badge2", "Badge Shade", 2, "gradient_end"),
                ctl("icon_color", "Icon Color", 3, "fill_color"),
                ctl("icon_on", "Show Icon", 3, "enabled"),
            ],
        ));
    }
}

/// The five most used platforms in one column, one after another.
fn all_accounts(v: &mut Vec<GraphicsTemplate>) {
    let secs_ = 8.0;
    let (mut layers, mut controls) = (Vec::new(), Vec::new());
    for (i, (id, name, sign, a, b)) in PLACES.into_iter().take(5).enumerate() {
        let (n, y, delay) = (i as u64 * 5, 282.0 + 129.0 * i as f64, 0.14 * i as f64);
        let mut bar = drawing(n + 1, "Bar", (960.0, y), 0x111827, &[round_box(-340.0, -52.0, 340.0, 52.0, 26.0)]);
        let mut badge = gradient_fill(ellipse(n + 2, "Badge", (678.0, y), (92.0, 92.0), a), a, b, 45.0);
        let mut icon = drawing(n + 3, "Icon", (678.0, y), 0xffffff, &sign.parts());
        let mut place = text(n + 4, "Platform", name, (750.0, baseline(y - 23.0, 24.0)), 24.0, &news_style("Poppins", "Medium", 0xcbd5e1), 0);
        let mut account = text(n + 5, name, ACCOUNT, (750.0, baseline(y + 16.0, 40.0)), 40.0, &news_style("Poppins", "Bold", 0xffffff), 0);
        for e in [&mut bar, &mut badge, &mut icon, &mut place, &mut account] {
            slide_from(e, -90.0, 0.0, delay);
            appear(e, delay, secs_);
        }
        controls.push(ctl(id, name, n + 5, "text"));
        layers.extend([bar, badge, icon, place, account]);
    }
    v.push(broadcast(
        "hesap-hepsi",
        "Social Account · All Together".into(),
        SOCIAL,
        "Five of your addresses in a column, arriving one after another.",
        secs_,
        layers,
        controls,
    ));
}

fn subscribe_and_like(v: &mut Vec<GraphicsTemplate>) {
    let secs_ = 6.0;
    let y = 900.0;
    let word = news_style("Poppins", "Bold", 0xffffff);
    // the row opens from its middle: Subscribe grows to the left, Like to the right
    let mut red = shadowed(drawing(1, "Subscribe Button", (730.0, y), 0xdc2626, &[round_box(-150.0, -50.0, 150.0, 50.0, 48.0)]), 0x000000, 50.0);
    let mut blue = shadowed(drawing(2, "Like Button", (1190.0, y), 0x2563eb, &[round_box(-150.0, -50.0, 150.0, 50.0, 48.0)]), 0x000000, 50.0);
    let mut plate = drawing(3, "Joining Word Plate", (960.0, y), 0xffffff, &[round_box(-40.0, -30.0, 40.0, 30.0, 28.0)]);
    let mut badge = gradient_fill(ellipse(4, "Badge", (470.0, y), (124.0, 124.0), 0xef4444), 0xef4444, 0x991b1b, 90.0);
    let mut play = drawing(5, "Icon", (470.0, y), 0xffffff, &Sign::Play.parts());
    let thumb = [
        round_box(-28.0, -4.0, -15.0, 26.0, 3.0),
        vec![
            [-10.0, -5.0],
            [-1.0, -14.0],
            [2.0, -28.0],
            [8.0, -29.0],
            [12.0, -23.0],
            [9.0, -8.0],
            [25.0, -8.0],
            [29.0, -3.0],
            [23.0, 22.0],
            [17.0, 26.0],
            [-4.0, 26.0],
            [-10.0, 22.0],
        ],
    ];
    let mut like_icon = drawing(6, "Like Icon", (1078.0, y), 0xffffff, &thumb);
    let mut sub = text(7, "Subscribe", s("Subscribe", "Abone Ol"), (842.0, baseline(y, 50.0)), 50.0, &word, 2);
    let mut and = text(8, "Joining Word", s("and", "ve"), (960.0, baseline(y, 34.0)), 34.0, &news_style("Poppins", "SemiBold", 0x1f2937), 1);
    let mut like = text(9, "Like", s("Like", "Beğen"), (1126.0, baseline(y, 50.0)), 50.0, &word, 0);
    pin(&mut red, 7, Some(-46.0), Some(42.0));
    pin(&mut blue, 9, Some(-96.0), Some(46.0));
    pin(&mut plate, 8, Some(-17.0), Some(17.0));
    pin(&mut badge, 1, Some(-140.0), None);
    pin(&mut play, 4, Some(34.0), None);
    for (e, d) in [
        (&mut red, 0.15),
        (&mut blue, 0.6),
        (&mut plate, 0.45),
        (&mut badge, 0.0),
        (&mut play, 0.0),
        (&mut like_icon, 0.65),
        (&mut sub, 0.25),
        (&mut and, 0.5),
        (&mut like, 0.65),
    ] {
        appear(e, d, secs_);
    }
    unfold(&mut red, 0.15);
    unfold(&mut blue, 0.6);
    let mut beat = [like_icon];
    pulse(&mut beat, secs_);
    let [like_icon] = beat;
    v.push(broadcast(
        "abone-begen",
        "Subscribe and Like".into(),
        SOCIAL,
        "A play badge with a subscribe and a like button; every word can be changed.",
        secs_,
        vec![red, blue, plate, badge, play, like_icon, sub, and, like],
        vec![
            ctl("subscribe", "Subscribe Text", 7, "text"),
            ctl("and", "Joining Word", 8, "text"),
            ctl("like", "Like Text", 9, "text"),
            ctl("font", "Font", 7, "font"),
            ctl("sub_color", "Subscribe Button Color", 1, "fill_color"),
            ctl("like_color", "Like Button Color", 2, "fill_color"),
            ctl("and_color", "Joining Word Color", 3, "fill_color"),
            ctl("badge", "Badge Color", 4, "gradient_start"),
            ctl("icon_on", "Show Icon", 5, "enabled"),
        ],
    ));
}
