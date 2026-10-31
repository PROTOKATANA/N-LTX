use wasm_bindgen::prelude::*;

/// What JavaScript gets, without JavaScript types: testable outside the browser.
pub fn convert(tex: &str, locale: Option<&str>) -> Result<mathra::Output, String> {
    convert_with(tex, locale, true, false, None, None, false, false)
}

/// `input`: "latex" (default), "mathml" or "asciimath"; `braille`: "cmu" or "nemeth" (default: the locale's);
/// `animate`: the SVG writes itself; `steps`: each element numbered by its step (S-SVG-10).
#[allow(clippy::too_many_arguments)]
pub fn convert_with(
    tex: &str,
    locale: Option<&str>,
    display: bool,
    strict: bool,
    input: Option<&str>,
    braille: Option<&str>,
    animate: bool,
    steps: bool,
) -> Result<mathra::Output, String> {
    let locale = locale.map_or(Ok(mathra::Locale::default()), str::parse)?;
    let input = input.map_or(Ok(mathra::Input::default()), str::parse)?;
    let braille = braille.map(str::parse).transpose()?;
    mathra::convert(tex, &mathra::Options { locale, display, strict, input, braille, animate, steps, ..Default::default() }).map_err(|e| e.to_string())
}

/// Everything at once, as JSON: `svg`, `speech`, `text`, `braille`, `braille_code`, `tree`, `applied` and `warnings`.
#[allow(clippy::too_many_arguments)]
pub fn render_json(tex: &str, locale: Option<&str>, display: bool, strict: bool, input: Option<&str>, braille: Option<&str>, animate: bool, steps: bool) -> Result<String, String> {
    convert_with(tex, locale, display, strict, input, braille, animate, steps).map(|r| r.to_json())
}

/// Converts an equation to SVG. `locale`: a language code such as "en" (default), "pt-BR", "es" or "ja";
/// `display` defaults to true, `strict` to false; `input` is "latex" (default), "mathml" or "asciimath";
/// `animate` and `steps` default to false.
#[wasm_bindgen(js_name = texToSvg)]
#[allow(clippy::too_many_arguments)]
pub fn tex_to_svg(
    tex: &str,
    locale: Option<String>,
    display: Option<bool>,
    strict: Option<bool>,
    input: Option<String>,
    braille: Option<String>,
    animate: Option<bool>,
    steps: Option<bool>,
) -> Result<String, JsError> {
    convert_with(tex, locale.as_deref(), display.unwrap_or(true), strict.unwrap_or(false), input.as_deref(), braille.as_deref(), animate.unwrap_or(false), steps.unwrap_or(false))
        .map(|r| r.svg)
        .map_err(|e| JsError::new(&e))
}

/// Tree of meaning as JSON.
#[wasm_bindgen(js_name = texToTree)]
pub fn tex_to_tree(tex: &str, locale: Option<String>) -> Result<String, JsError> {
    convert(tex, locale.as_deref()).map(|r| r.tree.to_json()).map_err(|e| JsError::new(&e))
}

/// The supported languages as JSON: `[{"code":"en","name":"English"}, …]`.
pub fn locales_json() -> String {
    let list: Vec<serde_json::Value> = mathra::Locale::ALL.iter().map(|l| serde_json::json!({ "code": l.code(), "name": l.name() })).collect();
    serde_json::Value::Array(list).to_string()
}

#[wasm_bindgen]
pub fn locales() -> String {
    locales_json()
}

/// Every output in one JSON string (see `render_json`). `display` defaults to true, `strict` to false,
/// `input` to "latex", `animate` and `steps` to false.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn render(
    tex: &str,
    locale: Option<String>,
    display: Option<bool>,
    strict: Option<bool>,
    input: Option<String>,
    braille: Option<String>,
    animate: Option<bool>,
    steps: Option<bool>,
) -> Result<String, JsError> {
    render_json(tex, locale.as_deref(), display.unwrap_or(true), strict.unwrap_or(false), input.as_deref(), braille.as_deref(), animate.unwrap_or(false), steps.unwrap_or(false))
        .map_err(|e| JsError::new(&e))
}
