//! Every figure of the corpus (`corpus/<topic>/*.tex`) converts, without warnings, to SVG and PNG.

use std::path::{Path, PathBuf};

/// The `.tex` files under `dir`, in its folders too, sorted.
fn figures(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().filter_map(Result::ok) {
        let p = e.path();
        if p.is_dir() {
            out.extend(figures(&p));
        } else if p.extension().is_some_and(|x| x == "tex") {
            out.push(p);
        }
    }
    out.sort();
    out
}

#[test]
fn every_corpus_figure_converts_without_warnings() {
    let files = figures(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../corpus")));
    assert!(files.len() >= 10, "{} figures", files.len());
    for f in files {
        let tex = std::fs::read_to_string(&f).unwrap();
        let out = voltik::convert(&tex).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        assert!(out.warnings.is_empty(), "{}: {:?}", f.display(), out.warnings);
        // the drawing, not its words (the description says "rainfall") nor its metadata (which
        // holds the source: `info=` has an "inf")
        let drawing = match (out.svg.find("<title"), out.svg.find("</metadata>")) {
            (Some(a), Some(b)) => format!("{}{}", &out.svg[..a], &out.svg[b..]),
            _ => out.svg.clone(),
        };
        assert!(!drawing.contains("NaN") && !drawing.contains("inf"), "{}", f.display());
        voltik::png(&out.svg, 96.0, true).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
    }
}
