//! Inside wasm, with `wasm-bindgen-test` on Node:
//! `cargo test -p voltik-wasm --target wasm32-unknown-unknown` (needs wasm-bindgen-test-runner).
#![cfg(target_arch = "wasm32")]

use voltik_wasm::{circuitikz_to_svg, png, render};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn svg_png_and_json() {
    let svg = circuitikz_to_svg(r"\begin{circuitikz}\draw (0,0) to[C] (2,0);\end{circuitikz}", None, None, None).unwrap();
    assert!(svg.starts_with("<svg"));
    assert!(png(&svg, None, None).unwrap().starts_with(b"\x89PNG"));
    assert!(render(r"\begin{circuitikz}\draw (0,0) to[R] (2,0);\end{circuitikz}", None, None, None).unwrap().contains("\"warnings\":[]"));
    assert!(render(r"\begin{circuitikz}\draw (0,0) to[R] (2,0);\end{circuitikz}", Some("fr".into()), None, None).unwrap().contains("Dessin de 2 formes"));
}

#[wasm_bindgen_test]
fn error_becomes_js_error() {
    assert!(circuitikz_to_svg("no figure", None, None, None).is_err());
    assert!(render("no figure", None, None, None).is_err());
    assert!(render(r"\begin{circuitikz}\draw (0,0) to[R] (2,0);\end{circuitikz}", Some("xx".into()), None, None).is_err());
    assert!(png("not svg", None, None).is_err());
}
