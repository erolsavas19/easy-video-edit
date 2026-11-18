//! `cargo run -p filmcraft-segment --features sam --release --example cut -- <mobile_sam.pt> <frame.rgb> <w> <h> <x0> <y0> <x1> <y1>`:
//! cuts the boxed object out of a raw RGB8 frame and prints its outline as JSON.
#![allow(clippy::unwrap_used, clippy::expect_used)]

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n = |i: usize| a[i].parse::<f32>().unwrap();
    let rgb = std::fs::read(&a[2]).unwrap();
    let t0 = std::time::Instant::now();
    let s = filmcraft_segment::Segmenter::load(std::path::Path::new(&a[1])).unwrap();
    eprintln!("load {:?}", t0.elapsed());
    for _ in 0..2 {
        let t = std::time::Instant::now();
        let o = s.cut(&rgb, n(3) as usize, n(4) as usize, [n(5), n(6), n(7), n(8)]).unwrap();
        eprintln!("cut {:?}, {} points", t.elapsed(), o.as_ref().map_or(0, Vec::len));
        if let Some(o) = o {
            println!("{}", serde_json_like(&o));
        }
    }
}

fn serde_json_like(o: &[[f32; 2]]) -> String {
    let v: Vec<String> = o.iter().map(|p| format!("[{:.1},{:.1}]", p[0], p[1])).collect();
    format!("[{}]", v.join(","))
}
