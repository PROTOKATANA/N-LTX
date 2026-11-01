//! The keys of `/pgfplots`. The ones that only hold a value or a style are described by the tables
//! below, with their defaults under `compat=default`, and turned into TeX once; the axis reads
//! their values when it is drawn. The keys whose code does work
//! (`compat`, `axis x line`, `/pgf/number format/…`, the plot handlers) are native.

mod emit;
mod native;
mod read;
mod table;

pub use native::define;
pub use read::{compat, flag, length, number, run, set, substitute_pgfpoint, value, value_tokens};
