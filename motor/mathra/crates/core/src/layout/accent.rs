//! Accents, bars and braces set on a base.

use crate::tree::Node;
use crate::Error;

use super::{k, Context, Item, MathBox, Style};

type R<T> = Result<T, Error>;

/// An accent and the base it sits on. `below` is an under-accent (`\underrightarrow`).
pub(super) struct Accent<'a> {
    pub base: &'a Node,
    pub accent: char,
    pub stretchy: bool,
    pub below: bool,
}

/// A stretchy accent drawn under its base.
pub(super) struct UnderAccent<'a> {
    pub base: &'a Node,
    pub accent: char,
}

/// `\overline` or `\underline`.
pub(super) struct Bar<'a> {
    pub base: &'a Node,
    pub above: bool,
}

/// `\overbrace` / `\underbrace` (or the square bracket) and its optional label.
pub(super) struct Brace<'a> {
    pub base: &'a Node,
    pub above: bool,
    pub label: Option<&'a Node>,
    pub bracket: bool,
}

fn combining(c: char) -> bool {
    matches!(c as u32, 0x0300..=0x036F | 0x20D0..=0x20FF)
}

pub(super) fn compose_accent(accented: &Accent, e: Style, ctx: &mut Context) -> R<MathBox> {
    let (base, accent, stretchy, below) = (accented.base, accented.accent, accented.stretchy, accented.below);
    if below {
        return compose_under_accent(&UnderAccent { base, accent }, e, ctx);
    }
    let b = super::compose(base, e.cramp(), ctx)?;
    let base_point = if b.single_glyph { b.accent_point.unwrap_or(b.width / 2.0) } else { b.width / 2.0 };
    let a = if stretchy { super::stretch::horizontal(accent, b.width, e)? } else { super::glyphs::glyph(accent, e)? };
    let point_a = if stretchy {
        a.width / 2.0
    } else {
        a.accent_point.unwrap_or_else(|| {
            let id = e.font().glyph(accent).expect("glyph already resolved");
            let (x0, _, x1, _) = e.font().limits(id);
            (x0 + x1) / 2.0 * e.scale()
        })
    };
    let dy = if combining(accent) {
        let base_height = k(e, |c| c.accent_base_height());
        (b.height - base_height).max(0.0)
    } else {
        b.height + k(e, |c| c.overbar_vertical_gap()) / 2.0 + a.depth
    };
    let mut math_box = b.clone();
    if b.width == 0.0 && b.items.is_empty() {
        // accent over nothing (\hat{}): takes the width of its drawing instead of leaking left
        // (combining accents have zero advance)
        let glyphs = a.items.iter().filter_map(|i| if let Item::Glyph { id, x, scale, .. } = i { Some((*id, *x, *scale)) } else { None });
        let (x0, x1) = glyphs.fold((0.0f32, a.width), |(x0, x1), (id, x, scale)| {
            let (g0, _, g1, _) = e.font().limits(id);
            (x0.min(x + g0 * scale), x1.max(x + g1 * scale))
        });
        math_box.place(&a, -x0, dy);
        math_box.width = x1 - x0;
    } else {
        math_box.place(&a, base_point - point_a, dy);
    }
    math_box.single_glyph = false;
    math_box.italic = b.italic;
    Ok(math_box)
}

/// Stretchy accent under the base (`\underrightarrow`, `\underleftarrow`), as far below as `\underline`.
pub(super) fn compose_under_accent(under: &UnderAccent, e: Style, ctx: &mut Context) -> R<MathBox> {
    let (base, accent) = (under.base, under.accent);
    let b = super::compose(base, e, ctx)?;
    let a = super::stretch::horizontal(accent, b.width, e)?;
    let width = b.width.max(a.width);
    let gap = k(e, |c| c.underbar_vertical_gap());
    let mut math_box = MathBox::space(width);
    math_box.place(&b, (width - b.width) / 2.0, 0.0);
    math_box.place(&a, (width - a.width) / 2.0, -(b.depth + gap + a.height));
    math_box.depth += k(e, |c| c.underbar_extra_descender());
    math_box.italic = b.italic;
    Ok(math_box)
}

pub(super) fn compose_bar(bar: &Bar, e: Style, ctx: &mut Context) -> R<MathBox> {
    let (base, above) = (bar.base, bar.above);
    let b = super::compose(base, if above { e.cramp() } else { e }, ctx)?;
    let mut math_box = b.clone();
    math_box.single_glyph = false;
    if above {
        let gap = k(e, |c| c.overbar_vertical_gap());
        let t = k(e, |c| c.overbar_rule_thickness());
        math_box.place(&MathBox::bar(b.width, t, b.height + gap), 0.0, 0.0);
        math_box.height += k(e, |c| c.overbar_extra_ascender());
    } else {
        let gap = k(e, |c| c.underbar_vertical_gap());
        let t = k(e, |c| c.underbar_rule_thickness());
        math_box.place(&MathBox::bar(b.width, t, -(b.depth + gap + t)), 0.0, 0.0);
        math_box.depth += k(e, |c| c.underbar_extra_descender());
    }
    Ok(math_box)
}

pub(super) fn compose_brace(brace: &Brace, e: Style, ctx: &mut Context) -> R<MathBox> {
    let (base, above, label, bracket) = (brace.base, brace.above, brace.label, brace.bracket);
    let b = super::compose(base, e, ctx)?;
    let symbol = match (above, bracket) {
        (true, false) => '⏞',
        (false, false) => '⏟',
        (true, true) => '⎴',
        (false, true) => '⎵',
    };
    let brace = super::stretch::horizontal(symbol, b.width, e)?;
    let r = label.map(|n| super::compose(n, e.sup(), ctx)).transpose()?;
    let width = b.width.max(brace.width).max(r.as_ref().map_or(0.0, |r| r.width));
    // the brace keeps the bar's gap from its content and the label keeps a limit's gap from the
    // brace (TeX: \overbrace is a \mathop with the label as \limits), measured on the ink
    let (bottom, top) = super::stretch::ink(&brace, e);
    let mut math_box = MathBox::space(width);
    math_box.place(&b, (width - b.width) / 2.0, 0.0);
    if above {
        let y = b.height + k(e, |c| c.overbar_vertical_gap()) - bottom;
        math_box.place(&brace, (width - brace.width) / 2.0, y);
        if let Some(r) = &r {
            math_box.place(r, (width - r.width) / 2.0, y + top + k(e, |c| c.upper_limit_gap_min()) + r.depth);
        }
    } else {
        let y = -(b.depth + k(e, |c| c.underbar_vertical_gap())) - top;
        math_box.place(&brace, (width - brace.width) / 2.0, y);
        if let Some(r) = &r {
            math_box.place(r, (width - r.width) / 2.0, y + bottom - k(e, |c| c.lower_limit_gap_min()) - r.height);
        }
    }
    Ok(math_box)
}
