//! Inside wasm, with `wasm-bindgen-test` on Node:
//! `cargo test -p mathra-wasm --target wasm32-unknown-unknown` (needs wasm-bindgen-test-runner).
#![cfg(target_arch = "wasm32")]

use mathra_wasm::{tex_to_svg, tex_to_tree};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn svg_and_tree() {
    assert!(tex_to_svg(r"\frac{1}{2}", None, None, None, None, None, None, None).unwrap().contains("1 over 2"));
    assert!(tex_to_svg(r"\frac{1}{2}", Some("pt-BR".into()), None, None, None, None, None, None).unwrap().contains("um meio"));
    assert!(tex_to_tree("x", None).unwrap().contains("identifier"));
}

#[wasm_bindgen_test]
fn error_becomes_js_error() {
    assert!(tex_to_svg("x", Some("xx".into()), None, None, None, None, None, None).is_err());
    assert!(tex_to_tree(&format!("{}x{}", "{".repeat(100), "}".repeat(100)), None).is_err());
}
