//! A decimal number: thousands groups and the locale's separator.

use crate::rules;
use crate::Error;

use super::glyphs::glyph;
use super::{em, Context, MathBox, Style};

type R<T> = Result<T, Error>;

/// A number as the tree stores it: canonical spelling, optional thousands groups, optional separator glyph.
pub(super) struct Number<'a> {
    pub value: &'a str,
    pub group: bool,
    pub thousands: Option<char>,
}

pub(super) fn compose_number(number: &Number, e: Style, ctx: &mut Context) -> R<MathBox> {
    let sep = ctx.locale.decimal_separator();
    if number.value.contains('.') && sep != '.' {
        ctx.registry.note(rules::L_NUM_2, format!("{} written with '{sep}' as decimal separator ({})", number.value, ctx.locale));
    }
    let (integer_part, fraction) = number.value.split_once('.').map_or((number.value, None), |(a, b)| (a, Some(b)));
    let mut math_box = MathBox::empty();
    let n = integer_part.chars().count();
    for (i, c) in integer_part.chars().enumerate() {
        // siunitx: groups of three from five digits on, with a thin space
        if number.group && n >= 5 && i > 0 && (n - i) % 3 == 0 {
            math_box.append(&MathBox::space(em(e, 3.0 / 18.0)));
        }
        if let Some(m) = number.thousands {
            if i > 0 && (n - i) % 3 == 0 {
                math_box.append(&glyph(m, e)?);
            }
        }
        math_box.append(&glyph(c, e)?);
    }
    if let Some(fr) = fraction {
        math_box.append(&glyph(sep, e)?);
        for c in fr.chars() {
            math_box.append(&glyph(c, e)?);
        }
    }
    math_box.single_glyph = number.value.chars().count() == 1;
    Ok(math_box)
}
