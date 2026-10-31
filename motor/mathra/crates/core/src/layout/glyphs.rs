//! One character, or a run of them, turned into a box.

use crate::font::Font;
use crate::tree::Variant;
use crate::Error;

use super::{alphabet, em, MathBox, Style};

type R<T> = Result<T, Error>;

/// Characters the font lacks, drawn with an equivalent it has.
fn equivalent(c: char) -> Option<char> {
    Some(match c {
        '✔' | '✅' => '✓',
        // the font has no ✗: the cross-product × is the closest drawing
        '✗' | '✘' | '❌' => '⨯',
        _ => return None,
    })
}

pub(crate) fn glyph_id(id: ttf_parser::GlyphId, s: f32, f: &Font) -> MathBox {
    let (_, y0, _, y1) = f.limits(id);
    MathBox {
        width: f.advance(id) * s,
        height: y1.max(0.0) * s,
        depth: (-y0).max(0.0) * s,
        items: vec![super::Item::Glyph { id, x: 0.0, y: 0.0, scale: s, color: None, flip: false }],
        italic: f.italic_correction(id) * s,
        accent_point: f.accent_point(id).map(|v| v * s),
        single_glyph: true,
    }
}

pub(crate) fn glyph(c: char, e: Style) -> R<MathBox> {
    let f = e.font();
    let id = f.glyph(c).or_else(|| equivalent(c).and_then(|x| f.glyph(x))).ok_or(Error::MissingGlyph { character: c })?;
    // in a script, the font's form for that size (`ssty`), as TeX draws cmr7 rather than cmr10
    let mut math_box = glyph_id(f.script_form(id, e.script_level()), e.scale(), f);
    // (with the letter's italic correction when that form has none of its own, as cmmi7's has)
    if math_box.italic == 0.0 {
        math_box.italic = f.italic_correction(id) * e.scale();
    }
    Ok(math_box)
}

/// `\ldots` and `\cdots` as TeX sets them: `\mathinner{\ldotp\ldotp\ldotp}`, three dots with a
/// thin space between them (punctuation's, so none in scripts), each in the script's own form.
pub(super) fn dots(dot: char, e: Style) -> R<MathBox> {
    let g = glyph(dot, e)?;
    let mut math_box = g.clone();
    for _ in 0..2 {
        if !e.is_script() {
            math_box.append(&MathBox::space(em(e, 3.0 / 18.0)));
        }
        math_box.append(&g);
    }
    math_box.single_glyph = false;
    Ok(math_box)
}

/// A variable: each character in its math alphabet. One character keeps the glyph's italic and accent point.
pub(super) fn compose_identifier(name: &str, variant: Variant, e: Style) -> R<MathBox> {
    let mut math_box = MathBox::empty();
    for c in name.chars() {
        let mapped = alphabet::character(c, variant);
        let g = glyph(mapped, e).or_else(|_| glyph(c, e))?;
        if name.chars().count() == 1 {
            return Ok(g);
        }
        math_box.append(&g);
    }
    Ok(math_box)
}
