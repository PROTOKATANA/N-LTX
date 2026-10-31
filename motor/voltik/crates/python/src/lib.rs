use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

fn error(e: voltik::Error) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// The options from a language code (`pt-BR`, `fr`…), with the construction animation and the
/// numbered steps or not.
fn options(locale: &str, animate: bool, steps: bool) -> PyResult<voltik::Options> {
    let o: voltik::Options = locale.parse().map(voltik::Options::locale).map_err(PyValueError::new_err)?;
    let o = if animate { o.animated() } else { o };
    Ok(if steps { o.steps() } else { o })
}

/// Converts a CircuiTikZ circuit to SVG, with the figure in words in `locale`.
#[pyfunction]
#[pyo3(signature = (code, *, locale = "en", animate = false, steps = false))]
fn circuitikz_to_svg(code: &str, locale: &str, animate: bool, steps: bool) -> PyResult<String> {
    voltik::convert_with(code, &options(locale, animate, steps)?).map(|r| r.svg).map_err(error)
}

/// The SVG, the figure in words (alt text and description) and the warnings (what was ignored or
/// approximated); `voltik.render` turns them into a dict.
#[pyfunction]
#[pyo3(signature = (code, *, locale = "en", animate = false, steps = false))]
fn render(code: &str, locale: &str, animate: bool, steps: bool) -> PyResult<(String, String, String, Vec<String>)> {
    voltik::convert_with(code, &options(locale, animate, steps)?).map(|r| (r.svg, r.alt, r.description, r.warnings)).map_err(error)
}

/// Rasterizes an SVG written by voltik to PNG bytes.
#[pyfunction]
#[pyo3(signature = (svg, *, dpi = 300.0, background = true))]
fn png(svg: &str, dpi: f32, background: bool) -> PyResult<Vec<u8>> {
    voltik::png(svg, dpi, background).map_err(error)
}

#[pymodule]
#[pyo3(name = "_voltik")]
fn voltik_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(circuitikz_to_svg, m)?)?;
    m.add_function(wrap_pyfunction!(render, m)?)?;
    m.add_function(wrap_pyfunction!(png, m)?)?;
    // the Python package has its own version (crates/python/Cargo.toml); the core's is `core_version`
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("core_version", voltik::VERSION)?;
    Ok(())
}
