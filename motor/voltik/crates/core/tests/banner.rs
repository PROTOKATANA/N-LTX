//! The README banner is drawn by voltik itself from `docs/banner.tex`. The committed SVG must be
//! exactly what the current code draws: same input, same bytes.
//!
//! `VOLTIK_UPDATE_BANNER=1 cargo test -p voltik --test banner` rewrites `docs/banner.svg` and `docs/banner.png`.

use std::path::PathBuf;

/// Width of the committed PNG, in pixels.
const PNG_WIDTH: u32 = 1280;

fn docs() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs")
}

/// The dpi that makes the PNG exactly `PNG_WIDTH` wide (its size is rounded to whole pixels).
fn dpi_for(svg: &str) -> f32 {
    let pt: f32 = svg.split("width=\"").nth(1).unwrap().split("pt\"").next().unwrap().parse().unwrap();
    PNG_WIDTH as f32 * 72.0 / pt
}

#[test]
fn committed_banner_matches_the_code() {
    let source = std::fs::read_to_string(docs().join("banner.tex")).unwrap();
    let r = voltik::convert(&source).unwrap();
    assert!(r.warnings.is_empty(), "the banner uses only supported CircuiTikZ: {:?}", r.warnings);
    if std::env::var_os("VOLTIK_UPDATE_BANNER").is_some() {
        std::fs::write(docs().join("banner.svg"), &r.svg).unwrap();
        std::fs::write(docs().join("banner.png"), voltik::png(&r.svg, dpi_for(&r.svg), false).unwrap()).unwrap();
    }
    let committed = std::fs::read_to_string(docs().join("banner.svg")).unwrap();
    assert!(committed == r.svg, "docs/banner.svg is out of date: run VOLTIK_UPDATE_BANNER=1 cargo test -p voltik --test banner");
    let png = std::fs::read(docs().join("banner.png")).unwrap();
    assert_eq!(u32::from_be_bytes([png[16], png[17], png[18], png[19]]), PNG_WIDTH);
}
