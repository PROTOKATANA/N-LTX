//! Arrows that stretch to their labels, and the vertical arrows of amscd.

use crate::tree::Node;
use crate::Error;

use super::{em, k, Context, MathBox, Style};

type R<T> = Result<T, Error>;

/// A horizontal arrow and the nodes set above and below it. `minimum` is a column width the arrow must fill.
#[derive(Clone, Copy)]
pub(super) struct LabeledArrow<'a> {
    pub symbol: char,
    pub above: Option<&'a Node>,
    pub below: Option<&'a Node>,
    pub minimum: f32,
}

/// An amscd vertical arrow and the labels on its two sides.
pub(super) struct VerticalArrow<'a> {
    pub symbol: char,
    pub left: Option<&'a Node>,
    pub right: Option<&'a Node>,
}

/// An arrow at its natural width: nothing forces it to a column.
pub(super) fn compose_arrow(arrow: &LabeledArrow, e: Style, ctx: &mut Context) -> R<MathBox> {
    let mut natural = *arrow;
    natural.minimum = 0.0;
    compose_wide_arrow(&natural, e, ctx)
}

pub(super) fn compose_wide_arrow(arrow: &LabeledArrow, e: Style, ctx: &mut Context) -> R<MathBox> {
    let (symbol, above, below, minimum) = (arrow.symbol, arrow.above, arrow.below, arrow.minimum);
    let s = above.map(|n| super::compose(n, e.sup(), ctx)).transpose()?;
    let d = below.map(|n| super::compose(n, e.sub(), ctx)).transpose()?;
    let labels = s.as_ref().map_or(0.0, |s| s.width).max(d.as_ref().map_or(0.0, |d| d.width));
    let arrow = super::stretch::horizontal(symbol, (labels + em(e, 0.8)).max(minimum), e)?;
    let width = arrow.width.max(labels);
    let mut math_box = MathBox::space(width);
    math_box.place(&arrow, (width - arrow.width) / 2.0, 0.0);
    if let Some(s) = &s {
        let y = k(e, |c| c.stretch_stack_top_shift_up()).max(arrow.height + k(e, |c| c.stretch_stack_gap_above_min()) + s.depth);
        math_box.place(s, (width - s.width) / 2.0, y);
    }
    if let Some(d) = &d {
        let y = k(e, |c| c.stretch_stack_bottom_shift_down()).max(arrow.depth + k(e, |c| c.stretch_stack_gap_below_min()) + d.height);
        math_box.place(d, (width - d.width) / 2.0, -y);
    }
    Ok(math_box)
}

/// amscd vertical arrow: ↓ stretched, centered on the axis, script-size labels on the sides.
pub(super) fn compose_vertical_arrow(arrow: &VerticalArrow, e: Style, ctx: &mut Context) -> R<MathBox> {
    let (symbol, left, right) = (arrow.symbol, arrow.left, arrow.right);
    let arrow = super::stretch::delimiter(symbol, em(e, 2.0), e)?;
    let es = e.sup();
    let l = left.map(|n| super::compose(n, es, ctx)).transpose()?;
    let r = right.map(|n| super::compose(n, es, ctx)).transpose()?;
    let axis = k(e, |c| c.axis_height());
    let slack = em(e, 0.17);
    let mut math_box = MathBox::empty();
    let center = |c: &MathBox| axis - (c.height - c.depth) / 2.0;
    if let Some(l) = &l {
        math_box.place(l, 0.0, center(l));
        math_box.width = l.width + slack;
    }
    let x = math_box.width;
    math_box.place(&arrow, x, 0.0);
    math_box.width = x + arrow.width;
    if let Some(r) = &r {
        let x = math_box.width + slack;
        math_box.place(r, x, center(r));
        math_box.width = x + r.width;
    }
    Ok(math_box)
}
