//! Outside the browser: the happy path of the exported functions, and the errors before they become
//! a `JsError` (creating a `JsError` only works inside wasm; that is in tests/web.rs).

use mathra_wasm::{convert, tex_to_svg, tex_to_tree};

#[test]
fn happy_path() {
    assert!(tex_to_svg("x", None, None, None, None, None, None, None).unwrap().starts_with("<svg"));
    assert!(tex_to_tree("x", Some("pt-BR".into())).unwrap().contains("identifier"));
}

#[test]
fn errors_become_text() {
    assert!(convert("x", Some("xx")).unwrap_err().contains("unknown locale"));
    let deep = format!("{}x{}", "{".repeat(100), "}".repeat(100));
    assert!(convert(&deep, None).unwrap_err().contains("nesting"));
}

#[test]
fn render_carries_every_output() {
    let json: serde_json::Value = serde_json::from_str(&mathra_wasm::render_json(r"\vertA\vert", None, true, false, None, None, false, false).unwrap()).unwrap();
    assert!(json["svg"].as_str().unwrap().starts_with("<svg"));
    assert!(json["speech"].as_str().unwrap().contains("absolute value"));
    assert_eq!(json["warnings"][0]["rule"], "E-REC-1");
    assert!(mathra_wasm::render_json(r"\vertA\vert", None, true, true, None, None, false, false).is_err());
}

#[test]
fn locales_lists_every_language() {
    let list: serde_json::Value = serde_json::from_str(&mathra_wasm::locales_json()).unwrap();
    assert_eq!(list[0]["code"], "en");
    assert!(list.as_array().unwrap().iter().any(|l| l["code"] == "ja" && l["name"] == "日本語"));
}

#[test]
fn input_formats() {
    assert!(tex_to_svg("sqrt(x)", None, None, None, Some("asciimath".into()), None, None, None).unwrap().contains("square root of x"));
    let json: serde_json::Value =
        serde_json::from_str(&mathra_wasm::render_json("<math><mi>x</mi></math>", None, true, true, Some("mathml"), None, false, false).unwrap()).unwrap();
    assert_eq!(json["text"], "x");
    assert!(mathra_wasm::convert_with("x", None, true, false, Some("word"), None, false, false).unwrap_err().contains("unknown input format"));
}

#[test]
fn braille_in_render() {
    let json: serde_json::Value = serde_json::from_str(&mathra_wasm::render_json(r"\frac{3}{4}", Some("pt-BR"), true, false, None, None, false, false).unwrap()).unwrap();
    assert_eq!(json["braille"], "⠼⠒⠙");
    assert_eq!(json["braille_code"], "cmu");
    let json: serde_json::Value = serde_json::from_str(&mathra_wasm::render_json(r"\frac{3}{4}", Some("pt-BR"), true, false, None, Some("nemeth"), false, false).unwrap()).unwrap();
    assert_eq!(json["braille_code"], "nemeth");
    assert!(mathra_wasm::convert_with("x", None, true, false, None, Some("ueb"), false, false).unwrap_err().contains("unknown braille code"));
}

#[test]
fn animate_and_steps() {
    let plain = tex_to_svg("x+1", None, None, None, None, None, None, None).unwrap();
    let animated = tex_to_svg("x+1", None, None, None, None, None, Some(true), None).unwrap();
    let steps = tex_to_svg("x+1", None, None, None, None, None, None, Some(true)).unwrap();
    assert!(!plain.contains("data-step"));
    assert!(animated.contains("@keyframes mr-f") && animated.contains(r#"data-steps="3""#));
    assert!(steps.contains(r#"data-steps="3""#) && !steps.contains("@keyframes"));
    let json: serde_json::Value = serde_json::from_str(&mathra_wasm::render_json("x", None, true, false, None, None, true, false).unwrap()).unwrap();
    assert!(json["svg"].as_str().unwrap().contains("mr-f"));
}
