//! The visualization phase of an axis (`\endpgfplots@environment@opt`): limits
//! (`\pgfplots@determinedefaultvalues`: enlargement, empty ranges), the unit vectors
//! (`\pgfplots@initsizes`: `width`/`height` less 45pt unless `scale only axis`), the ticks, then
//! drawing in pgfplots' order: grid, ticks, axis lines, tick labels, the plots clipped to the axis
//! (with the TikZ statements among them), the marks, the labels, the title and the legend.

mod anchor;
mod cs;
mod frame;
mod marks;
mod meta;
mod place;
mod plots;
mod prepare;
mod util;

pub use cs::{pgfplots_cs, pgfplots_plain};
pub use frame::{Cs, Frame, PlotPoints};
pub use meta::{colormap, colormap_named};
pub use place::draw_axis;
pub use util::solid;

pub use cs::getcoordinates;
pub(super) use frame::unit;
pub(super) use marks::{minor_ticks, tick_labels_at, tick_text_for};
pub(super) use prepare::Setup;
pub(super) use util::{axis_char, data_scale_trafo, draw_with, keys_tikz, line_shift, number_list, pv, tick_offset, visible};
