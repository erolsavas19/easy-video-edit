//! FilmCraft object segmentation: cut a boxed object (a person, a car, a pet…) out of a video
//! frame, for Easy Video Edit's background replacement.
//!
//! The model is MobileSAM (a TinyViT-5M image encoder with Segment Anything's prompt encoder and
//! mask decoder), run on the CPU with candle (feature `sam`). It is downloaded on first use, after
//! the user confirms, from the pinned [`MODEL`] files and checked against their SHA-256; it is
//! never bundled or committed. Licence: MobileSAM's code and weights are Apache-2.0
//! (<https://github.com/ChaoningZhang/MobileSAM>); the copy used is Kornia's Apache-2.0 mirror.
//!
//! The cut-out leaves this crate as an outline (a polygon in frame pixels), which FilmCraft keeps
//! as an Opacity mask path: it is saved with the project, undone, keyframed and rendered like any
//! mask. [`outline`] turns the model's mask logits into that polygon and needs no model, so it is
//! tested on its own.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::path::{Path, PathBuf};

#[cfg(feature = "sam")]
mod sam;
#[cfg(feature = "sam")]
pub use sam::Segmenter;

#[derive(Debug, thiserror::Error)]
pub enum SegmentError {
    #[error("object segmentation is not available in this build (built without the `sam` feature)")]
    Unavailable,
    #[error("the segmentation model is not installed")]
    NotInstalled,
    #[error("segmentation model: {0}")]
    Model(String),
    #[error("bad input: {0}")]
    Input(String),
}

pub type Result<T> = std::result::Result<T, SegmentError>;

/// One pinned file of the model.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct ModelFile {
    pub name: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub size: u64,
}

/// The segmentation model and where it comes from.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct ModelInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub license: &'static str,
    pub license_url: &'static str,
    pub author: &'static str,
    pub source: &'static str,
    pub file: ModelFile,
}

/// MobileSAM, pinned to a revision of Kornia's Apache-2.0 mirror of the original weights.
pub const MODEL: ModelInfo = ModelInfo {
    id: "mobile-sam",
    name: "MobileSAM",
    license: "Apache-2.0",
    license_url: "https://github.com/ChaoningZhang/MobileSAM/blob/master/LICENSE",
    author: "Chaoning Zhang et al. (MobileSAM); mirror by Kornia",
    source: "https://huggingface.co/kornia/mobile_sam",
    file: ModelFile {
        name: "mobile_sam.pt",
        url: "https://huggingface.co/kornia/mobile_sam/resolve/f4607d92819c1c75ea8f561e53063ca6728cd4eb/mobile_sam.pt",
        sha256: "6dbb90523a35330fedd7f1d3dfc66f995213d81b29a5ca8108dbcdd4e37d6c2f",
        size: 40_728_226,
    },
};

/// Whether this build can segment (feature `sam`).
pub fn available() -> bool {
    cfg!(feature = "sam")
}

/// Where the model's file lives under the models directory.
pub fn model_path(models_dir: &Path) -> PathBuf {
    models_dir.join(MODEL.id).join(MODEL.file.name)
}

/// Whether the model file is there with the expected size (the checksum is checked when it is
/// downloaded).
pub fn installed(models_dir: &Path) -> bool {
    std::fs::metadata(model_path(models_dir)).is_ok_and(|m| m.len() == MODEL.file.size)
}

/// Most vertices an outline keeps (the simplification loosens until it fits).
pub const MAX_OUTLINE_POINTS: usize = 400;

/// The outline of the largest region where `logits` (a `w` × `h` grid, row by row) is above 0,
/// with its holes filled, as a polygon in grid units (a cell's corner is a whole number; cell
/// `(x, y)` spans `x..x+1`, `y..y+1`). The staircase of cell edges is smoothed and simplified to at
/// most [`MAX_OUTLINE_POINTS`] points. `None` when nothing is above 0 or the grid is malformed.
pub fn outline(logits: &[f32], w: usize, h: usize) -> Option<Vec<[f32; 2]>> {
    let n = w.checked_mul(h)?;
    if w == 0 || h == 0 || logits.len() < n || n > 16 << 20 {
        return None;
    }
    // padded grid: a one-cell empty border, so every region is closed and the outside is connected
    let (pw, ph) = (w + 2, h + 2);
    let mut inside = vec![false; pw * ph];
    for y in 0..h {
        for x in 0..w {
            inside[(y + 1) * pw + x + 1] = logits.get(y * w + x).is_some_and(|v| *v > 0.0);
        }
    }
    // the largest 4-connected region
    let mut label = vec![0u32; pw * ph];
    let (mut best, mut best_n, mut next) = (0u32, 0usize, 1u32);
    let mut stack = Vec::new();
    for start in 0..pw * ph {
        if !inside[start] || label[start] != 0 {
            continue;
        }
        let mut count = 0usize;
        label[start] = next;
        stack.push(start);
        while let Some(i) = stack.pop() {
            count += 1;
            for j in [i.wrapping_sub(1), i + 1, i.wrapping_sub(pw), i + pw] {
                if j < pw * ph && inside[j] && label[j] == 0 {
                    label[j] = next;
                    stack.push(j);
                }
            }
        }
        if count > best_n {
            (best, best_n) = (next, count);
        }
        next = next.saturating_add(1);
    }
    if best_n == 0 {
        return None;
    }
    // fill its holes: everything not reachable from the border without crossing it is inside
    let region: Vec<bool> = label.iter().map(|l| *l == best).collect();
    let mut outside = vec![false; pw * ph];
    outside[0] = true;
    stack.push(0);
    while let Some(i) = stack.pop() {
        let (x, y) = (i % pw, i / pw);
        let mut try_push = |j: usize| {
            if !region[j] && !outside[j] {
                outside[j] = true;
                stack.push(j);
            }
        };
        if x > 0 {
            try_push(i - 1);
        }
        if x + 1 < pw {
            try_push(i + 1);
        }
        if y > 0 {
            try_push(i - pw);
        }
        if y + 1 < ph {
            try_push(i + pw);
        }
    }
    let filled = |x: usize, y: usize| x < pw && y < ph && !outside[y * pw + x];
    let loop_ = trace(pw, ph, &filled)?;
    // back to grid units (the padding shifted everything by one cell)
    let pts: Vec<[f32; 2]> = loop_.iter().map(|(x, y)| [*x as f32 - 1.0, *y as f32 - 1.0]).collect();
    let smooth = chaikin(&chaikin(&pts));
    let mut eps = 0.35f32;
    let mut out = simplify(&smooth, eps);
    while out.len() > MAX_OUTLINE_POINTS && eps < 64.0 {
        eps *= 1.5;
        out = simplify(&smooth, eps);
    }
    (out.len() >= 3).then_some(out)
}

/// Points an outline is resampled to, so outlines of different frames match point for point
/// (an animated mask path interpolates between them).
pub const RESAMPLED_POINTS: usize = 160;

/// `p` (a closed polygon) as `n` points evenly spaced along it, clockwise on screen, starting at
/// its top point (the leftmost of the highest), so two outlines of one object line up.
pub fn resample(p: &[[f32; 2]], n: usize) -> Option<Vec<[f32; 2]>> {
    if p.len() < 3 || !(3..=100_000).contains(&n) || p.iter().any(|q| !q[0].is_finite() || !q[1].is_finite()) {
        return None;
    }
    // clockwise on screen (y down): positive shoelace sum
    let area: f32 = (0..p.len())
        .map(|i| {
            let (a, b) = (p[i], p[(i + 1) % p.len()]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum();
    let mut pts: Vec<[f32; 2]> = p.to_vec();
    if area < 0.0 {
        pts.reverse();
    }
    let start = (0..pts.len()).min_by(|a, b| pts[*a][1].total_cmp(&pts[*b][1]).then(pts[*a][0].total_cmp(&pts[*b][0])))?;
    pts.rotate_left(start);
    let m = pts.len();
    let seg = |i: usize| {
        let (a, b) = (pts[i], pts[(i + 1) % m]);
        ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt()
    };
    let total: f32 = (0..m).map(seg).sum();
    if total.is_nan() || total <= 0.0 {
        return None;
    }
    let mut out = Vec::with_capacity(n);
    let (mut i, mut acc) = (0usize, 0.0f32);
    for k in 0..n {
        let target = total * k as f32 / n as f32;
        while i < m && acc + seg(i) < target {
            acc += seg(i);
            i += 1;
        }
        let i0 = i.min(m - 1);
        let (a, b) = (pts[i0], pts[(i0 + 1) % m]);
        let l = seg(i0).max(1e-9);
        let t = ((target - acc) / l).clamp(0.0, 1.0);
        out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
    }
    Some(out)
}

/// The boundary of the filled cells as one loop of corner points, following cell edges with the
/// region on the right (clockwise on screen).
fn trace(pw: usize, ph: usize, filled: &dyn Fn(usize, usize) -> bool) -> Option<Vec<(usize, usize)>> {
    use std::collections::HashMap;
    // directed boundary edges: start corner → end corners
    let mut edges: HashMap<(usize, usize), Vec<(usize, usize)>> = HashMap::new();
    let mut count = 0usize;
    for y in 0..ph {
        for x in 0..pw {
            if !filled(x, y) {
                continue;
            }
            let out = |dx: isize, dy: isize| {
                let (nx, ny) = (x as isize + dx, y as isize + dy);
                nx < 0 || ny < 0 || !filled(nx as usize, ny as usize)
            };
            let mut add = |a: (usize, usize), b: (usize, usize)| {
                edges.entry(a).or_default().push(b);
                count += 1;
            };
            if out(0, -1) {
                add((x, y), (x + 1, y));
            }
            if out(1, 0) {
                add((x + 1, y), (x + 1, y + 1));
            }
            if out(0, 1) {
                add((x + 1, y + 1), (x, y + 1));
            }
            if out(-1, 0) {
                add((x, y + 1), (x, y));
            }
        }
    }
    let start = *edges.keys().min_by_key(|(x, y)| (*y, *x))?;
    let mut pts = vec![start];
    let mut cur = start;
    let mut dir: (isize, isize) = (1, 0);
    for _ in 0..count {
        let outs = edges.get_mut(&cur)?;
        if outs.is_empty() {
            return None;
        }
        // at a pinch point prefer turning right, then straight, then left
        let pick = |d: (isize, isize), e: &(usize, usize)| {
            let nd = (e.0 as isize - cur.0 as isize, e.1 as isize - cur.1 as isize);
            let cross = d.0 * nd.1 - d.1 * nd.0;
            let dot = d.0 * nd.0 + d.1 * nd.1;
            if cross > 0 {
                0
            } else if dot > 0 {
                1
            } else {
                2
            }
        };
        let k = (0..outs.len()).min_by_key(|i| outs.get(*i).map_or(3, |e| pick(dir, e)))?;
        let nxt = outs.swap_remove(k);
        dir = (nxt.0 as isize - cur.0 as isize, nxt.1 as isize - cur.1 as isize);
        cur = nxt;
        if cur == start {
            break;
        }
        pts.push(cur);
    }
    (pts.len() >= 4).then_some(pts)
}

/// One round of Chaikin corner cutting on a closed polygon.
fn chaikin(p: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let n = p.len();
    let mut out = Vec::with_capacity(n * 2);
    for i in 0..n {
        let (a, b) = (p[i], p[(i + 1) % n]);
        out.push([a[0] * 0.75 + b[0] * 0.25, a[1] * 0.75 + b[1] * 0.25]);
        out.push([a[0] * 0.25 + b[0] * 0.75, a[1] * 0.25 + b[1] * 0.75]);
    }
    out
}

/// Douglas–Peucker on a closed polygon (split at the point farthest from the first).
fn simplify(p: &[[f32; 2]], eps: f32) -> Vec<[f32; 2]> {
    let n = p.len();
    if n < 4 {
        return p.to_vec();
    }
    let d2 = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2);
    let far = (1..n).max_by(|i, j| d2(p[0], p[*i]).total_cmp(&d2(p[0], p[*j]))).unwrap_or(n / 2);
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[far] = true;
    let mut stack = vec![(0usize, far), (far, n)];
    while let Some((a, b)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        let (pa, pb) = (p[a], p[b % n]);
        let (dx, dy) = (pb[0] - pa[0], pb[1] - pa[1]);
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let (mut idx, mut dmax) = (a, 0.0f32);
        for (i, q) in p.iter().enumerate().take(b).skip(a + 1) {
            let d = ((q[0] - pa[0]) * dy - (q[1] - pa[1]) * dx).abs() / len;
            if d > dmax {
                (idx, dmax) = (i, d);
            }
        }
        if dmax > eps {
            keep[idx] = true;
            stack.push((a, idx));
            stack.push((idx, b));
        }
    }
    p.iter().zip(keep).filter(|(_, k)| *k).map(|(q, _)| *q).collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn disc(w: usize, h: usize, cx: f32, cy: f32, r: f32) -> Vec<f32> {
        (0..w * h).map(|i| r - (((i % w) as f32 + 0.5 - cx).powi(2) + ((i / w) as f32 + 0.5 - cy).powi(2)).sqrt()).collect()
    }

    #[test]
    fn outline_follows_a_disc() {
        let (w, h) = (64, 48);
        let o = outline(&disc(w, h, 30.0, 20.0, 12.0), w, h).unwrap();
        assert!(o.len() >= 8 && o.len() <= MAX_OUTLINE_POINTS, "{}", o.len());
        for p in &o {
            let r = ((p[0] - 30.0).powi(2) + (p[1] - 20.0).powi(2)).sqrt();
            assert!((r - 12.0).abs() < 1.5, "point {p:?} at radius {r}");
        }
    }

    #[test]
    fn largest_region_wins_and_holes_are_filled() {
        let (w, h) = (40, 40);
        let big = disc(w, h, 12.0, 12.0, 9.0);
        let small = disc(w, h, 32.0, 32.0, 4.0);
        let mut g: Vec<f32> = big.iter().zip(&small).map(|(a, b)| a.max(*b)).collect();
        // a hole in the big disc
        for y in 10..14 {
            for x in 10..14 {
                g[y * w + x] = -1.0;
            }
        }
        let o = outline(&g, w, h).unwrap();
        assert!(o.iter().all(|p| p[0] < 24.0 && p[1] < 24.0), "only the big disc");
        assert!(o.iter().all(|p| ((p[0] - 12.0).powi(2) + (p[1] - 12.0).powi(2)).sqrt() > 6.0), "the hole leaves no inner loop");
    }

    #[test]
    fn resampling_lines_outlines_up() {
        // a square given counter-clockwise from a corner, and clockwise from another
        let ccw = [[0.0, 0.0], [0.0, 10.0], [10.0, 10.0], [10.0, 0.0]];
        let cw = [[10.0, 10.0], [0.0, 10.0], [0.0, 0.0], [10.0, 0.0]];
        let a = resample(&ccw, 8).unwrap();
        let b = resample(&cw, 8).unwrap();
        assert_eq!(a.len(), 8);
        for (p, q) in a.iter().zip(&b) {
            assert!((p[0] - q[0]).abs() < 1e-4 && (p[1] - q[1]).abs() < 1e-4, "{a:?} vs {b:?}");
        }
        assert_eq!(a[0], [0.0, 0.0], "starts at the top-left");
        assert!(a[1][0] > a[0][0], "then runs clockwise (right along the top)");
        assert!(resample(&[[0.0, 0.0], [1.0, 1.0]], 8).is_none());
        assert!(resample(&[[0.0, 0.0]; 5], 8).is_none(), "no length");
        assert!(resample(&[[f32::NAN, 0.0], [1.0, 0.0], [0.0, 1.0]], 8).is_none());
    }

    #[test]
    fn hostile_grids_give_none_without_panicking() {
        assert!(outline(&[], 0, 0).is_none());
        assert!(outline(&[1.0; 3], 4, 4).is_none(), "too short");
        assert!(outline(&[-1.0; 16], 4, 4).is_none(), "nothing inside");
        assert!(outline(&[f32::NAN; 16], 4, 4).is_none());
        assert!(outline(&[1.0], usize::MAX, 2).is_none(), "overflowing size");
        // a single cell and a full grid still close
        let mut one = vec![-1.0; 25];
        one[12] = 1.0;
        assert!(outline(&one, 5, 5).is_some());
        assert!(outline(&[1.0; 25], 5, 5).is_some());
        // a checkerboard (diagonal pinch points everywhere)
        let cb: Vec<f32> = (0..400).map(|i| if (i % 20 + i / 20) % 2 == 0 { 1.0 } else { -1.0 }).collect();
        let _ = outline(&cb, 20, 20);
    }
}
