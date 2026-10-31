//! Graphicx adjustments of a box that is already composed: scale, rotate, raise, resize, smash, phantom.

use crate::tree::{BoxLength, Node};
use crate::Error;

use super::{em, k, Context, MathBox, Style};

type R<T> = Result<T, Error>;

/// Width, height and depth of a box, frozen so a later change of the box does not feed the next length.
#[derive(Clone, Copy)]
struct BoxMeasures {
    width: f32,
    height: f32,
    depth: f32,
}

/// A graphicx length (`2\height`, `1cm`) for a box of the given measures.
fn length(l: &BoxLength, size: BoxMeasures, e: Style) -> f32 {
    em(e, l.em) + l.width * size.width + l.height * size.height + l.depth * size.depth
}

/// `\scalebox{x}[y]`: a negative factor mirrors the box inside its own width (x) or turns its height
/// into depth (y), as graphicx does.
pub(super) fn scale_xy(b: &mut MathBox, x: f32, y: f32) {
    if y > 0.0 && (x.abs() - y).abs() < 1e-6 {
        // the same factor both ways: scaled glyphs, no transform
        if y != 1.0 {
            b.scale_box(y);
        }
        if x < 0.0 {
            b.mirror();
        }
        return;
    }
    let (w, h, d) = (b.width, b.height, b.depth);
    b.transform([x, 0.0, 0.0, y, if x < 0.0 { -x * w } else { 0.0 }, 0.0]);
    b.width = x.abs() * w;
    (b.height, b.depth) = if y >= 0.0 { (y * h, y * d) } else { (-y * d, -y * h) };
}

/// `\rotatebox[origin=…]{degrees}`: turns the box counterclockwise around the origin point, and takes
/// the box that holds the turned one, as graphicx does. Without `origin`, the point is the left end of
/// the baseline; with it, each letter (l, r, t, b, B) sets one coordinate and the other is the center.
pub(super) fn rotate(b: &mut MathBox, degrees: f32, origin: &str) {
    let (w, h, d) = (b.width, b.height, b.depth);
    let (mut px, mut py) = if origin.is_empty() { (0.0, 0.0) } else { (w / 2.0, (h - d) / 2.0) };
    for letter in origin.chars() {
        match letter {
            'l' => px = 0.0,
            'r' => px = w,
            't' => py = h,
            'b' => py = -d,
            'B' => py = 0.0,
            _ => {}
        }
    }
    let snap = |v: f32| if v.abs() < 1e-6 { 0.0 } else { v };
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (sin, cos) = (snap(sin), snap(cos));
    let turn = |x: f32, y: f32| (px + cos * (x - px) - sin * (y - py), py + sin * (x - px) + cos * (y - py));
    let corners = [turn(0.0, -d), turn(w, -d), turn(0.0, h), turn(w, h)];
    let left = corners.iter().map(|c| c.0).fold(f32::INFINITY, f32::min);
    let right = corners.iter().map(|c| c.0).fold(f32::NEG_INFINITY, f32::max);
    let low = corners.iter().map(|c| c.1).fold(f32::INFINITY, f32::min);
    let high = corners.iter().map(|c| c.1).fold(f32::NEG_INFINITY, f32::max);
    let (e, f) = turn(0.0, 0.0);
    b.transform([cos, sin, -sin, cos, snap(e - left), snap(f)]);
    b.width = right - left;
    b.height = high.max(0.0);
    b.depth = (-low).max(0.0);
}

/// `\raisebox`: how far to lift, and the height and depth to force, if the input gave them.
pub(super) struct Raise<'a> {
    pub amount: f32,
    pub height: Option<&'a BoxLength>,
    pub depth: Option<&'a BoxLength>,
    pub base: &'a Node,
}

pub(super) fn compose_raised(raise: &Raise, e: Style, ctx: &mut Context) -> R<MathBox> {
    let mut math_box = super::compose(raise.base, e, ctx)?;
    math_box.shift_vertical(em(e, raise.amount));
    math_box.single_glyph = false;
    // \height and \depth in the optional arguments are those of the raised box, as in LaTeX
    let size = BoxMeasures { width: math_box.width, height: math_box.height, depth: math_box.depth };
    if let Some(height) = raise.height {
        math_box.height = length(height, size, e);
    }
    if let Some(depth) = raise.depth {
        math_box.depth = length(depth, size, e);
    }
    Ok(math_box)
}

/// `\resizebox`: the target width and height (`None` keeps the ratio) and whether height includes depth.
pub(super) struct Resize<'a> {
    pub width: &'a Option<BoxLength>,
    pub height: &'a Option<BoxLength>,
    pub total: bool,
    pub base: &'a Node,
}

pub(super) fn compose_resized(resize: &Resize, e: Style, ctx: &mut Context) -> R<MathBox> {
    let mut math_box = super::compose(resize.base, e, ctx)?;
    let size = BoxMeasures { width: math_box.width, height: math_box.height, depth: math_box.depth };
    let tall = if resize.total { size.height + size.depth } else { size.height };
    let factor = |target: &Option<BoxLength>, natural: f32| target.as_ref().map(|t| length(t, size, e) / natural).filter(|f| f.is_finite() && *f != 0.0);
    let (x, y) = (factor(resize.width, size.width), factor(resize.height, tall));
    // `!` keeps the ratio; a size that cannot be reached (an empty box) leaves it as it is
    let (x, y) = match (x, y) {
        (Some(x), Some(y)) => (x, y),
        (Some(f), None) | (None, Some(f)) => (f, f),
        (None, None) => (1.0, 1.0),
    };
    let limit = |f: f32| f.signum() * f.abs().clamp(0.05, 20.0);
    scale_xy(&mut math_box, limit(x), limit(y));
    Ok(math_box)
}

/// `\vcenter`: the box's middle on the math axis.
pub(super) fn compose_vcenter(base: &Node, e: Style, ctx: &mut Context) -> R<MathBox> {
    let mut b = super::compose(base, e, ctx)?;
    let axis = k(e, |c| c.axis_height());
    b.shift_vertical(axis - (b.height - b.depth) / 2.0);
    b.single_glyph = false;
    Ok(b)
}

/// `\smash`: which of the two dimensions to zero.
pub(super) struct Smash<'a> {
    pub base: &'a Node,
    pub height: bool,
    pub depth: bool,
}

pub(super) fn compose_smashed(smash: &Smash, e: Style, ctx: &mut Context) -> R<MathBox> {
    let mut math_box = super::compose(smash.base, e, ctx)?;
    if smash.height {
        math_box.height = 0.0;
    }
    if smash.depth {
        math_box.depth = 0.0;
    }
    math_box.single_glyph = false;
    Ok(math_box)
}

/// `\mathllap`, `\mathclap`, `\mathrlap`: the side tells which way the box hangs off its anchor.
pub(super) struct Overlap<'a> {
    pub base: &'a Node,
    pub side: char,
}

pub(super) fn compose_overlap(overlap: &Overlap, e: Style, ctx: &mut Context) -> R<MathBox> {
    let b = super::compose_list(overlap.base, e, ctx)?;
    let dx = match overlap.side {
        'l' => -b.width,
        'c' => -b.width / 2.0,
        _ => 0.0,
    };
    let mut math_box = MathBox::empty();
    math_box.place(&b, dx, 0.0);
    Ok(math_box)
}

/// `\phantom`: which dimensions of the base to keep once its ink is dropped.
pub(super) struct Phantom<'a> {
    pub base: &'a Node,
    pub width: bool,
    pub height: bool,
}

pub(super) fn compose_phantom(phantom: &Phantom, e: Style, ctx: &mut Context) -> R<MathBox> {
    let mut math_box = super::compose_list(phantom.base, e, ctx)?;
    math_box.items.clear();
    if !phantom.width {
        math_box.width = 0.0;
    }
    if !phantom.height {
        math_box.height = 0.0;
        math_box.depth = 0.0;
    }
    Ok(math_box)
}

/// `\displaystyle` and the other style switches: the level code and the base it applies to.
pub(super) struct Styled<'a> {
    pub level: u8,
    pub base: &'a Node,
}

pub(super) fn compose_styled(styled: &Styled, e: Style, ctx: &mut Context) -> R<MathBox> {
    use super::Level;
    let level = match styled.level {
        0 => Level::Display,
        1 => Level::Text,
        2 => Level::Script,
        _ => Level::ScriptScript,
    };
    super::compose(styled.base, Style { level, cramped: e.cramped, font: e.font }, ctx)
}
