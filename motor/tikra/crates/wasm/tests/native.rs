//! Outside the browser: the happy path of the exported functions, and the errors before they become
//! a `JsError` (creating a `JsError` only works inside wasm; that is in tests/web.rs).

use tikra_wasm::{convert_json, png_bytes, svg, tikz_to_svg};

const FIGURE: &str = r"\begin{tikzpicture}\draw[->] (0,0) -- (1,1) node[right] {$x$};\end{tikzpicture}";

#[test]
fn happy_path() {
    assert!(tikz_to_svg(FIGURE, None, None, None).unwrap().starts_with("<svg"));
    assert_eq!(tikra_wasm::version(), tikra::VERSION);
}

#[test]
fn render_carries_svg_and_warnings() {
    let json: serde_json::Value = serde_json::from_str(&convert_json(FIGURE, None, None, None).unwrap()).unwrap();
    assert!(json["svg"].as_str().unwrap().starts_with("<svg"));
    assert_eq!(json["alt"], "Drawing of one shape with the text “x”");
    assert_eq!(json["description"], "Text in the figure: x.");
    // the words in another language, the drawing the same
    let json: serde_json::Value = serde_json::from_str(&convert_json(FIGURE, Some("ja".into()), None, None).unwrap()).unwrap();
    assert_eq!(json["description"], "図中のテキスト：x。");
    assert_eq!(svg(FIGURE, Some("ja".into()), None, None).unwrap().split("</metadata>").nth(1), svg(FIGURE, None, None, None).unwrap().split("</metadata>").nth(1));
    assert_eq!(json["warnings"], serde_json::json!([]));
    let json: serde_json::Value = serde_json::from_str(&convert_json(r"\begin{tikzpicture}\pic {x}; \draw (0,0) -- (1,0);\end{tikzpicture}", None, None, None).unwrap()).unwrap();
    assert!(!json["warnings"].as_array().unwrap().is_empty());
}

#[test]
fn render_carries_named_points_and_origin() {
    // (tip) is 1 cm right and 1 cm up from the origin; y is down in SVG
    let json: serde_json::Value =
        serde_json::from_str(&convert_json(r"\begin{tikzpicture}\coordinate (tip) at (1,1); \draw (0,0) -- (tip);\end{tikzpicture}", None, None, None).unwrap()).unwrap();
    let (tip, origin) = (&json["points"][0], &json["origin"]);
    assert_eq!(tip["name"], "tip");
    let cm = 72.0 / 2.54;
    assert!((tip["x"].as_f64().unwrap() - origin[0].as_f64().unwrap() - cm).abs() < 1e-6, "{json}");
    assert!((tip["y"].as_f64().unwrap() - origin[1].as_f64().unwrap() + cm).abs() < 1e-6, "{json}");
    let json: serde_json::Value = serde_json::from_str(&convert_json(FIGURE, None, None, None).unwrap()).unwrap();
    assert_eq!(json["points"], serde_json::json!([]));
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
    let svg = tikz_to_svg(FIGURE, None, None, None).unwrap();
    let opaque = png_bytes(&svg, None, None).unwrap();
    assert!(opaque.starts_with(b"\x89PNG"));
    assert_ne!(opaque, png_bytes(&svg, None, Some(false)).unwrap());
    assert!(png_bytes(&svg, Some(72.0), None).unwrap().len() < opaque.len());
}
