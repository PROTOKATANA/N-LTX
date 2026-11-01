use tikra_engine::keys;
use tikra_engine::tex::{self, Token};
use tikra_engine::Error;

use tikra_engine::engine::{tikra_tokens, Engine};
/// Sets a stored value (`\pgfkeyssetvalue`).
pub fn set(e: &mut Engine, path: &str, value: &str) {
    e.tex.define_string(&keys::macro_name(path), value, false);
}

/// A stored value as tokens, if it is not empty.
pub fn value_tokens(e: &Engine, path: &str) -> Option<Vec<Token>> {
    keys::value(&e.tex, path).filter(|v| !keys::trim(v).is_empty())
}

/// A stored value, expanded, as text (empty if unset).
pub fn value(e: &mut Engine, path: &str) -> String {
    match keys::value(&e.tex, path) {
        Some(v) => e.tex.expand_to_string(v).map(|s| s.trim().to_string()).unwrap_or_default(),
        None => String::new(),
    }
}

/// A boolean value (`true`, `false`).
pub fn flag(e: &mut Engine, path: &str) -> bool {
    value(e, path) == "true"
}

/// A numeric value, if set and valid.
pub fn number(e: &mut Engine, path: &str) -> Option<f64> {
    let s = value(e, path);
    if s.is_empty() {
        return None;
    }
    e.number(&s).ok().filter(|v| v.is_finite())
}

/// A length value in cm, if set and valid (a number without unit is in pt).
pub fn length(e: &mut Engine, path: &str) -> Option<f64> {
    let s = value(e, path);
    if s.is_empty() {
        return None;
    }
    e.length(&s).ok().filter(|v| v.is_finite())
}

/// Runs a key list under `/pgfplots`.
pub fn run(e: &mut Engine, list: &str) -> Result<(), Error> {
    let saved = e.st.key_path.clone();
    let r = e.process_keys(tikra_tokens(list), Some("/pgfplots"));
    e.st.key_path = saved;
    r
}

/// The compatibility level, as a number: 1.18 → 118, `default` → 0.
pub fn compat(e: &Engine) -> u32 {
    keys::value(&e.tex, "/pgfplots/@compat").map(|v| tex::tokens_to_string(&v)).and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}

/// Whether plain coordinates inside an axis are `axis cs` (`\ifpgfplots@substitute@pgfpoint`).
pub fn substitute_pgfpoint(e: &Engine) -> bool {
    keys::value(&e.tex, "/pgfplots/@substitute pgfpoint").is_some_and(|v| tex::tokens_to_string(&v).trim() == "true")
}
