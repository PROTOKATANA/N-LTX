//! Upright text inside a formula: function names, `\text`, and units.

use crate::input::latex::{BOLD_END, BOLD_START, ITALIC_CORRECTED_END, ITALIC_END, ITALIC_START, MARKERS, MONO_END, MONO_START, SMALL_CAPS_END, SMALL_CAPS_START};
use crate::rules;
use crate::tree::{TextPart, Variant};
use crate::Error;

use super::glyphs::{glyph, glyph_id};
use super::{alphabet, em, kerning, Context, MathBox, Style};

type R<T> = Result<T, Error>;

/// Size of the capitals that stand for lowercase letters in `\textsc`.
const SMALL_CAPS_SCALE: f32 = 0.76;

/// Upright text (function names, `\text`, units), and the italic inside it.
fn upright_text(t: &str, e: Style, ctx: &mut Context) -> R<MathBox> {
    let mut math_box = MathBox::empty();
    let mut bold = 0;
    let mut small_caps = 0;
    let mut mono = 0;
    let mut italic = 0;
    // L-TXT-5: the italic correction of the last letter, for the end of a `\textit{…}`
    let mut last_italic = 0.0;
    // the previous character and its weight, for TeX's kern between the two (none across a change)
    let mut previous: Option<(char, bool)> = None;
    let chars: Vec<char> = t.chars().collect();
    for (k, &c) in chars.iter().enumerate() {
        match c {
            BOLD_START => bold += 1,
            BOLD_END => bold -= 1,
            SMALL_CAPS_START => small_caps += 1,
            SMALL_CAPS_END => small_caps -= 1,
            MONO_START => mono += 1,
            MONO_END => mono -= 1,
            ITALIC_START => italic += 1,
            ITALIC_END => italic -= 1,
            ITALIC_CORRECTED_END => {
                italic -= 1;
                // LaTeX's \/ after the group, unless a period or a comma follows it
                let next = chars[k + 1..].iter().find(|x| !MARKERS.contains(x));
                if !matches!(next, Some('.' | ',')) {
                    math_box.append(&MathBox::space(last_italic));
                }
            }
            _ => {}
        }
        if MARKERS.contains(&c) {
            previous = None;
            continue;
        }
        // TeX's kerns are the upright fonts'; italic letters keep none between them
        if small_caps == 0 && mono == 0 && italic == 0 {
            if let Some((p, weight)) = previous {
                let k = kerning::kern(ctx.kerning, weight, p, c);
                if k != 0.0 {
                    math_box.append(&MathBox::space(em(e, k)));
                }
            }
            previous = Some((c, bold > 0));
        } else {
            previous = None;
        }
        // small capitals: a lowercase letter is its capital at the height of the x
        if small_caps > 0 && c.is_lowercase() {
            if let Some(id) = c.to_uppercase().next().and_then(|u| e.font().glyph(u)) {
                math_box.append(&glyph_id(id, e.scale() * SMALL_CAPS_SCALE, e.font()));
                continue;
            }
        }
        // bold through the math block letters; accented ones stay regular
        // typewriter likewise: letters and digits; the font has no typewriter punctuation
        let c = if mono > 0 {
            alphabet::character(c, Variant::Monospace)
        } else if italic > 0 {
            alphabet::character(c, if bold > 0 { Variant::BoldItalic } else { Variant::Italic })
        } else if bold > 0 {
            alphabet::character(c, Variant::Bold)
        } else {
            c
        };
        if c == ' ' || c == '\u{00A0}' {
            last_italic = 0.0;
            math_box.append(&MathBox::space(em(e, e.font.word_space())));
            continue;
        }
        if c == '\u{2003}' {
            math_box.append(&MathBox::space(em(e, 1.0)));
            continue;
        }
        if c == '\u{200B}' {
            continue;
        }
        match glyph(c, e) {
            Ok(g) => {
                last_italic = if italic > 0 { g.italic } else { 0.0 };
                math_box.append(&g);
            }
            Err(Error::MissingGlyph { character }) if !ctx.strict => {
                ctx.registry.warn(rules::E_REC_8, format!("the font has no '{character}' (U+{:04X}): drew a box", character as u32));
                math_box.append(&glyph('□', e)?);
            }
            Err(error) => return Err(error),
        }
    }
    math_box.single_glyph = false;
    Ok(math_box)
}

/// A function name in the locale's spelling, noted when it differs from the canonical English one.
pub(super) fn compose_function(name: &str, e: Style, ctx: &mut Context) -> R<MathBox> {
    let local = ctx.locale.function_name(name);
    if local != name {
        ctx.registry.note(rules::L_FUN_1, format!("function {name} written as '{local}' ({})", ctx.locale));
    }
    upright_text(&local, e, ctx)
}

/// `\text`: upright runs and math snippets, one after another.
pub(super) fn compose_text(parts: &[TextPart], e: Style, ctx: &mut Context) -> R<MathBox> {
    let mut math_box = MathBox::empty();
    for p in parts {
        match p {
            TextPart::Text(t) => math_box.append(&upright_text(t, e, ctx)?),
            TextPart::Math(m) => math_box.append(&super::compose_list(m, e.not_display(), ctx)?),
        }
    }
    math_box.single_glyph = false;
    Ok(math_box)
}
