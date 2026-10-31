//! Frames and cancellation strokes drawn around a composed base.

use crate::tree::{Cancel, Frame, Node};
use crate::Error;

use super::glyphs::glyph;
use super::{em, k, Context, Item, MathBox, Style};

type R<T> = Result<T, Error>;

/// A base crossed out by a bar, a backslash, or both.
pub(super) struct Canceled<'a> {
    pub base: &'a Node,
    pub kind: Cancel,
}

/// A base and the frame drawn around it, with the colors the input gave.
pub(super) struct Framed<'a> {
    pub base: &'a Node,
    pub shape: Frame,
    pub background: Option<&'a str>,
    pub border: Option<&'a str>,
}

pub(super) fn compose_canceled(mark: &Canceled, e: Style, ctx: &mut Context) -> R<MathBox> {
    let (base, kind) = (mark.base, mark.kind);
    let b = super::compose(base, e, ctx)?;
    let t = k(e, |c| c.fraction_rule_thickness());
    let slack = em(e, 0.05);
    let (x0, x1, y0, y1) = (-slack, b.width + slack, -b.depth - slack, b.height + slack);
    let mut math_box = b.clone();
    math_box.single_glyph = false;
    let mut rule = |a: (f32, f32), z: (f32, f32)| {
        math_box.items.push(Item::Line { x1: a.0, y1: a.1, x2: z.0, y2: z.1, thickness: t, color: None });
    };
    if matches!(kind, Cancel::Bar | Cancel::X) {
        rule((x0, y0), (x1, y1));
    }
    if matches!(kind, Cancel::Backslash | Cancel::X) {
        rule((x0, y1), (x1, y0));
    }
    Ok(math_box)
}

pub(super) fn compose_frame(framed: &Framed, e: Style, ctx: &mut Context) -> R<MathBox> {
    let b = super::compose(framed.base, e, ctx)?;
    let math_box = match framed.shape {
        Frame::Rectangle | Frame::Fill => rectangle(b, framed, e),
        Frame::Circle => circle(b, e),
        Frame::Phase => phase(b, e),
        Frame::ActuarialAngle => actuarial(b, e),
    };
    Ok(math_box)
}

/// `\fbox` and `\colorbox`: padding, an optional fill, and a rectangular rule.
fn rectangle(b: MathBox, framed: &Framed, e: Style) -> MathBox {
    let t = em(e, 0.04);
    let p = em(e, 0.3);
    let width = b.width + 2.0 * (p + t);
    let top = b.height + p + t;
    let bottom = b.depth + p + t;
    let mut math_box = MathBox::space(width);
    // the background goes first, so the content is drawn over it
    if let Some(color) = framed.background {
        math_box.items.push(Item::Rule { x: 0.0, y: -bottom, width, height: top + bottom, color: Some(color.to_string()) });
    }
    math_box.place(&b, p + t, 0.0);
    if framed.shape == Frame::Rectangle {
        let color = framed.border.map(str::to_string);
        math_box.items.push(Item::Rule { x: 0.0, y: top - t, width, height: t, color: color.clone() });
        math_box.items.push(Item::Rule { x: 0.0, y: -bottom, width, height: t, color: color.clone() });
        math_box.items.push(Item::Rule { x: 0.0, y: -bottom, width: t, height: top + bottom, color: color.clone() });
        math_box.items.push(Item::Rule { x: width - t, y: -bottom, width: t, height: top + bottom, color });
    }
    math_box.height = top;
    math_box.depth = bottom;
    math_box
}

/// `\textcircled`: a circle centered on the content.
fn circle(b: MathBox, e: Style) -> MathBox {
    let t = em(e, 0.04);
    let radius = (b.width.max(b.total()) / 2.0 + em(e, 0.12)).max(em(e, 0.42));
    let width = 2.0 * radius + em(e, 0.06);
    let cx = width / 2.0;
    let cy = (b.height - b.depth) / 2.0;
    let mut math_box = MathBox::space(width);
    math_box.place(&b, cx - b.width / 2.0, 0.0);
    math_box.items.push(Item::Ellipse { cx, cy, rx: radius, ry: radius, thickness: t * 1.5, color: None });
    math_box.height = math_box.height.max(cy + radius + t);
    math_box.depth = math_box.depth.max(radius - cy + t);
    math_box
}

/// steinmetz.sty: the angle's foot .35ex below the content, its side rising with slope 2
/// to the content's height, the content starting where the side ends; lines of .6pt.
fn phase(b: MathBox, e: Style) -> MathBox {
    let t = em(e, 0.06);
    let foot = b.depth + 0.35 * k(e, |c| c.accent_base_height());
    let run = (foot + b.height) / 2.0;
    let width = b.width + run;
    let mut math_box = MathBox::space(width);
    math_box.place(&b, run, 0.0);
    let y = -foot + t / 2.0;
    math_box.items.push(Item::Line { x1: 0.0, y1: y, x2: width, y2: y, thickness: t, color: None });
    math_box.items.push(Item::Line { x1: 0.0, y1: y, x2: run, y2: b.height, thickness: t, color: None });
    math_box.depth = math_box.depth.max(foot + t);
    math_box
}

/// actuarialangle.sty: 1mu, the content and 2mu under an `\overline` rule, a rule down its
/// right side to the depth of a `\mathstrut`, 1mu.
fn actuarial(b: MathBox, e: Style) -> MathBox {
    let mu = em(e, 1.0 / 18.0);
    let t = k(e, |c| c.overbar_rule_thickness());
    let gap = k(e, |c| c.overbar_vertical_gap());
    let inner = b.width + 2.0 * mu;
    let top = b.height + gap;
    let strut = glyph('(', e).map_or(b.depth, |p| p.depth);
    let bottom = b.depth.max(strut);
    let mut math_box = MathBox::space(mu + inner + t + mu);
    math_box.place(&b, mu, 0.0);
    math_box.items.push(Item::Rule { x: mu, y: top, width: inner + t, height: t, color: None });
    math_box.items.push(Item::Rule { x: mu + inner, y: -bottom, width: t, height: top + bottom, color: None });
    math_box.height = top + t + k(e, |c| c.overbar_extra_ascender());
    math_box.depth = bottom;
    math_box
}
