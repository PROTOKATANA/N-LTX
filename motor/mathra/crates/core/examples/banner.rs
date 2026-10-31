//! README banner, drawn by mathra itself: every letter, the tagline and the equations come
//! from the embedded font, with no text or external font. The same version always writes the same bytes.
//!
//! `cargo run -p mathra --example banner` writes `docs/banner.svg`;
//! `tests/banner.rs` checks that the file in the repository is up to date.

use std::fmt::Write;

use mathra::layout::Item;
use mathra::{convert, font, Options};

const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 320.0;

const BACKGROUND: &str = "#0e1726";
const INK: &str = "#eef2f8";
const MUTED: &str = "#8796ad";
const BOX: &str = "#3a4b66";
const ACCENT: &str = "#f2b544";

/// mathra's SVG for `tex`, placed with its baseline at `(x, base)` and 1 em = `em` px.
/// Returns the element and the width it takes, in px.
fn equation(tex: &str, x: f32, base: f32, em: f32, color: &str) -> (String, f32) {
    let svg = convert(tex, &Options::default()).expect("the banner equations are valid").svg;
    // viewBox="left -top width top+bottom", in font units (1000 per em); left is below 0 when ink
    // passes the start of the box, which stays at x
    let vb: Vec<f32> = svg.split("viewBox=\"").nth(1).unwrap().split('"').next().unwrap().split(' ').map(|n| n.parse().unwrap()).collect();
    let per_unit = em / font::font().units_per_em();
    let (width, height) = (vb[2] * per_unit, vb[3] * per_unit);
    let top = base + vb[1] * per_unit;
    let placed = svg.replacen("<svg ", &format!(r#"<svg x="{}" y="{}" "#, num(x + vb[0] * per_unit), num(top)), 1);
    let placed = placed.replacen(r#"width=""#, r#"data-w=""#, 1).replacen(r#"height=""#, r#"data-h=""#, 1);
    let placed = placed.replacen("<svg ", &format!(r#"<svg width="{}" height="{}" "#, num(width), num(height)), 1);
    (format!(r#"<g color="{color}">{placed}</g>"#), width)
}

/// The box of each letter of the name: the width the layout reserves and the height of the line.
fn boxes(tex: &str, x: f32, base: f32, em: f32) -> String {
    let (math_box, _) = mathra::typeset(tex, &Options::default()).unwrap();
    let f = font::font();
    let per_unit = em / f.units_per_em();
    let top = base - math_box.height * per_unit;
    let bottom = base + math_box.depth.max(220.0) * per_unit;
    let mut s = String::new();
    for item in &math_box.items {
        if let Item::Glyph { id, x: gx, scale, .. } = item {
            let left_x = x + gx * per_unit;
            let w = f.advance(*id) * scale * per_unit;
            let _ = write!(s, r#"<rect x="{}" y="{}" width="{}" height="{}"/>"#, num(left_x), num(top), num(w), num(bottom - top));
        }
    }
    let end = x + math_box.width * per_unit;
    format!(
        r#"<g fill="none" stroke="{BOX}" stroke-width="1" stroke-dasharray="3 3">{s}</g><line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{ACCENT}" stroke-width="1.5"/>"#,
        num(x - 24.0),
        num(base),
        num(end + 24.0),
        num(base),
    )
}

fn num(v: f32) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r.fract() == 0.0 {
        format!("{}", r as i64)
    } else {
        format!("{r}")
    }
}

pub fn generate() -> String {
    let mut body = String::new();
    let (margin, name_baseline, name_em) = (88.0, 178.0, 116.0);

    body.push_str(&boxes(r"\mathbf{mathra}", margin, name_baseline, name_em));
    body.push_str(&equation(r"\mathbf{mathra}", margin, name_baseline, name_em, INK).0);
    let (label, _) = equation(r"\text{baseline}", margin + 4.0, name_baseline + 22.0, 13.0, ACCENT);
    body.push_str(&label);
    body.push_str(&equation(r"\text{LaTeX to accessible SVG, no TeX install}", margin, 262.0, 27.0, MUTED).0);

    // on the right, what it draws: from the simplest to the tallest
    let right = 812.0;
    body.push_str(&equation(r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}", right, 92.0, 30.0, INK).0);
    body.push_str(&equation(r"\int_0^{\infty} e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}", right, 172.0, 30.0, ACCENT).0);
    body.push_str(&equation(r"A = \begin{pmatrix} 1 & 0 \\ 0 & 1 \end{pmatrix}", right, 258.0, 30.0, INK).0);

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" role="img" aria-label="mathra: LaTeX to accessible SVG, no TeX install"><rect width="{w}" height="{h}" rx="16" fill="{BACKGROUND}"/><line x1="{x}" y1="40" x2="{x}" y2="280" stroke="{BOX}" stroke-width="1"/>{body}</svg>
"#,
        w = num(WIDTH),
        h = num(HEIGHT),
        x = num(right - 56.0),
    )
}

#[allow(dead_code)]
fn main() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/banner.svg");
    std::fs::write(path, generate()).expect("writes docs/banner.svg");
    println!("docs/banner.svg");
}
