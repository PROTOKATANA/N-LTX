//! The SVGs in `docs/examples/` are drawn by tikra from figures in `corpus/`. Each committed file must
//! be exactly what the current code draws, on a white card so they stay readable on dark pages.
//!
//! `TIKRA_UPDATE_EXAMPLES=1 cargo test -p tikra --test examples` rewrites them.

use std::path::PathBuf;

/// Corpus figures shown in the README, by path in `corpus/` without `.tex`; the SVG takes the file's name.
const EXAMPLES: &[&str] = &[
    "plots-3d/lorenz-attractor",
    "charts/nightingale-rose",
    "plots-2d/normal-distribution",
    "diagrams/petersen-graph",
    "science/feynman-diagram",
    "geometry/golden-spiral",
    "nodes-and-text/periodic-table-corner",
    "plots-3d/surface-3d",
    "plots-3d/smooth-shaded-surface",
    "decorations/clock-snapshots",
    // shown by the READMEs of the versions already published (PyPI, crates.io, npm), which load
    // the images from main: they stay, and stay in sync
    "geometry/triangle-angles",
    "geometry/unit-circle",
    "plots-2d/pgfplots-legend",
    "diagrams/flowchart",
    "diagrams/graph-nodes",
    "science/spring-decorations",
    "charts/bar-chart",
    "nodes-and-text/standalone-document",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The figure's SVG with a white rectangle behind everything, covering the whole viewBox.
fn on_white(svg: &str) -> String {
    let view_box = svg.split("viewBox=\"").nth(1).unwrap().split('"').next().unwrap();
    let v: Vec<&str> = view_box.split(' ').collect();
    let open_end = svg.find('>').unwrap() + 1;
    let card = format!(r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#ffffff"/>"##, v[0], v[1], v[2], v[3]);
    format!("{}{card}{}", &svg[..open_end], &svg[open_end..])
}

#[test]
fn committed_examples_match_the_code() {
    let update = std::env::var_os("TIKRA_UPDATE_EXAMPLES").is_some();
    for name in EXAMPLES {
        let source = std::fs::read_to_string(root().join("corpus").join(format!("{name}.tex"))).unwrap();
        let r = tikra::convert(&source).unwrap();
        assert!(r.warnings.is_empty(), "{name}: {:?}", r.warnings);
        let svg = on_white(&r.svg);
        let file = name.rsplit('/').next().unwrap();
        let path = root().join("docs/examples").join(format!("{file}.svg"));
        if update {
            std::fs::write(&path, &svg).unwrap();
        }
        let committed = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(committed == svg, "docs/examples/{file}.svg is out of date: run TIKRA_UPDATE_EXAMPLES=1 cargo test -p tikra --test examples");
    }
}

#[test]
fn white_card_covers_the_view_box() {
    let svg = on_white(r#"<svg viewBox="-1 -2 3 4"><path d="M0 0"/></svg>"#);
    assert_eq!(svg, r##"<svg viewBox="-1 -2 3 4"><rect x="-1" y="-2" width="3" height="4" fill="#ffffff"/><path d="M0 0"/></svg>"##);
}

#[test]
fn the_readmes_show_examples_that_exist() {
    // the npm and PyPI pages show these images by their URL on GitHub: a renamed file breaks them
    for readme in ["README.md", "npm/README.md", "crates/python/README.md"] {
        let text = std::fs::read_to_string(root().join(readme)).unwrap();
        for part in text.split("docs/examples/").skip(1) {
            let file: String = part.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '.').collect();
            let file = file.trim_end_matches('.');
            assert!(root().join("docs/examples").join(file).exists(), "{readme} shows docs/examples/{file}, which does not exist");
        }
        for part in text.split("/corpus/").skip(1) {
            let file: String = part.chars().take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '/' | '.')).collect();
            if file.ends_with(".tex") {
                assert!(root().join("corpus").join(&file).exists(), "{readme} links corpus/{file}, which does not exist");
            }
        }
    }
}
