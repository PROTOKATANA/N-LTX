//! Inside wasm, with `wasm-bindgen-test` on Node:
//! `cargo test -p tikra-wasm --target wasm32-unknown-unknown` (needs wasm-bindgen-test-runner).
#![cfg(target_arch = "wasm32")]

use tikra_wasm::{png, render, tikz_to_svg};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn svg_png_and_json() {
    let svg = tikz_to_svg(r"\tikz \draw (0,0) circle (1);", None, None, None).unwrap();
    assert!(svg.starts_with("<svg"));
    assert!(png(&svg, None, None).unwrap().starts_with(b"\x89PNG"));
    assert!(render(r"\tikz \draw (0,0) -- (1,0);", None, None, None).unwrap().contains("\"warnings\":[]"));
    assert!(render(r"\tikz \draw (0,0) -- (1,0);", Some("fr".into()), None, None).unwrap().contains("Dessin de 1 forme"));
}

#[wasm_bindgen_test]
fn error_becomes_js_error() {
    assert!(tikz_to_svg("no figure", None, None, None).is_err());
    assert!(render("no figure", None, None, None).is_err());
    assert!(render(r"\tikz \draw (0,0) -- (1,0);", Some("xx".into()), None, None).is_err());
    assert!(png("not svg", None, None).is_err());
}
