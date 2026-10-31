//! The engine of tikra: it reads a figure with the language layer (`tikra_tex`) and leaves what it
//! draws on the canvas (`tikra_canvas`).

pub use tikra_canvas::{canvas, color, scene};
pub use tikra_label::{label, times};
pub use tikra_tex::dates as tex_dates;
pub use tikra_tex::{expr, keys, tex, Error, PT};

pub mod describe;
pub mod engine;
pub mod extension;
