//! Delimiters that grow with the formula they enclose.

use crate::tree::Node;
use crate::Error;

use super::{em, k, Context, MathBox, Style};

type R<T> = Result<T, Error>;

/// What `\left` … `\right` encloses, and the delimiters around it and between its parts.
pub(super) struct Delimited<'a> {
    pub open: Option<char>,
    pub close: Option<char>,
    pub parts: &'a [Node],
    pub middles: &'a [char],
}

pub(super) fn compose_delimited(delimited: &Delimited, e: Style, ctx: &mut Context) -> R<MathBox> {
    let (open, close, parts, middles) = (delimited.open, delimited.close, delimited.parts, delimited.middles);
    let contents = parts.iter().map(|p| super::compose(p, e, ctx)).collect::<R<Vec<_>>>()?;
    let axis = k(e, |c| c.axis_height());
    let half = contents.iter().map(|c| (c.height - axis).max(c.depth + axis)).fold(0.0f32, f32::max);
    // \delimiterfactor = 901 and \delimitershortfall = 5pt
    let target = (2.0 * half * 0.901).max(2.0 * half - em(e, 0.5));
    let delim = |c: Option<char>| -> R<MathBox> {
        match c {
            Some(c) => super::stretch::delimiter(c, target, e),
            None => Ok(MathBox::space(em(e, 0.12))),
        }
    };
    let mut math_box = delim(open)?;
    for (i, content) in contents.iter().enumerate() {
        if i > 0 {
            math_box.append(&delim(middles.get(i - 1).copied())?);
        }
        math_box.append(content);
    }
    math_box.append(&delim(close)?);
    math_box.single_glyph = false;
    Ok(math_box)
}
