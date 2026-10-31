//! Outside the browser: the happy path of the exported functions, and the errors before they become
//! a `JsError` (creating a `JsError` only works inside wasm; that is in tests/web.rs).

use voltik_wasm::{circuitikz_to_svg, convert_json, png_bytes, svg};

const FIGURE: &str = r"\begin{circuitikz}\draw (0,0) to[R=$R$] (2,0);\end{circuitikz}";

#[test]
fn happy_path() {
    assert!(circuitikz_to_svg(FIGURE, None, None, None).unwrap().starts_with("<svg"));
    assert_eq!(voltik_wasm::version(), voltik::VERSION);
}

#[test]
fn render_carries_svg_and_warnings() {
    let json: serde_json::Value = serde_json::from_str(&convert_json(FIGURE, None, None, None).unwrap()).unwrap();
    assert!(json["svg"].as_str().unwrap().starts_with("<svg"));
    assert_eq!(json["alt"], "Drawing of 2 shapes with the text “R”");
    assert_eq!(json["description"], "Text in the figure: R.");
    // the words in another language, the drawing the same
    let json: serde_json::Value = serde_json::from_str(&convert_json(FIGURE, Some("ja".into()), None, None).unwrap()).unwrap();
    assert_eq!(json["description"], "図中のテキスト：R。");
    assert_eq!(svg(FIGURE, Some("ja".into()), None, None).unwrap().split("</metadata>").nth(1), svg(FIGURE, None, None, None).unwrap().split("</metadata>").nth(1));
    assert_eq!(json["warnings"], serde_json::json!([]));
    let json: serde_json::Value =
        serde_json::from_str(&convert_json(r"\begin{circuitikz}\draw (0,0) to[R, no such option] (2,0);\end{circuitikz}", None, None, None).unwrap()).unwrap();
    assert!(!json["warnings"].as_array().unwrap().is_empty());
}

#[test]
fn errors_become_text() {
    assert!(convert_json("no figure here", None, None, None).unwrap_err().contains("no tikzpicture"));
    assert!(convert_json(FIGURE, Some("xx".into()), None, None).unwrap_err().contains("unknown locale: xx"));
    assert!(svg(FIGURE, Some("xx".into()), None, None).unwrap_err().contains("unknown locale"));
    assert!(png_bytes("not svg", None, None).is_err());
    assert!(png_bytes("<svg/>", Some(0.0), None).unwrap_err().contains("invalid dpi"));
}

#[test]
fn png_defaults() {
    let svg = circuitikz_to_svg(FIGURE, None, None, None).unwrap();
    let opaque = png_bytes(&svg, None, None).unwrap();
    assert!(opaque.starts_with(b"\x89PNG"));
    assert_ne!(opaque, png_bytes(&svg, None, Some(false)).unwrap());
    assert!(png_bytes(&svg, Some(72.0), None).unwrap().len() < opaque.len());
}
