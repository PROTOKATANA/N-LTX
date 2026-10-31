//! The language layer of tikra: TeX's macros (`tex`), pgfmath's expressions (`expr`), the key tree
//! of pgfkeys (`keys`) and the metrics of TeX's fonts (`metrics`). It knows nothing of drawing.

use std::fmt;

pub mod dates;
pub mod expr;
pub mod files;
pub mod fpu;
pub mod keys;
pub mod metrics;
pub mod tex;

/// One TeX point, in centimeters.
pub const PT: f64 = 2.54 / 72.27;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Expression(String),
    Figure(String),
    Png(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Expression(m) => write!(f, "expression: {m}"),
            Error::Figure(m) => write!(f, "figure: {m}"),
            Error::Png(m) => write!(f, "png: {m}"),
        }
    }
}

impl std::error::Error for Error {}
