//! Glyphs that grow: tall parentheses, radicals, braces and long arrows.
//! First try the font's ready-made variants; if none is enough, assemble from parts.

use crate::font::{Font, Part};
use crate::Error;

use super::{glyph_id, k, Item, MathBox, Style};

type R<T> = Result<T, Error>;

/// Vertical glyph with a total height of at least `target` (already at the style's scale).
pub fn vertical(c: char, target: f32, e: Style) -> R<MathBox> {
    let f = e.font();
    let s = e.scale();
    if f.glyph(c).is_none() {
        if let Some(parts) = moustache(c, f) {
            // as cmex draws it: the two halves at least, joined by extenders as tall as needed
            let least: f32 = parts.iter().filter(|p| !p.extender).map(|p| p.advance).sum::<f32>() - f.min_overlap();
            return Ok(assemble(&Assembly { parts: &parts, target: (target / s).max(least), scale: s, vertical: true }, f));
        }
    }
    let id = f.glyph(c).ok_or(Error::MissingGlyph { character: c })?;
    let base = glyph_id(id, s, f);
    if base.total() >= target {
        return Ok(base);
    }
    let variants = f.variants(id, true);
    for (v, measure) in &variants {
        if measure * s >= target {
            return Ok(glyph_id(*v, s, f));
        }
    }
    if let Some(parts) = f.assembly(id, true) {
        return Ok(assemble(&Assembly { parts: &parts, target: target / s, scale: s, vertical: true }, f));
    }
    Ok(variants.last().map_or(base, |(v, _)| glyph_id(*v, s, f)))
}

/// `\lmoustache` (⎰) and `\rmoustache` (⎱) in a font without them (Latin Modern Math): built as
/// TeX's cmex builds them, from the halves of the braces and their extender, bottom to top. The
/// connectors are the ones the braces' own assemblies give these parts.
fn moustache(c: char, f: &Font) -> Option<Vec<Part>> {
    let (bottom, top) = match c {
        '⎰' => ('⎭', '⎧'),
        '⎱' => ('⎩', '⎫'),
        _ => return None,
    };
    let known: Vec<Part> = ['{', '}'].iter().filter_map(|b| f.glyph(*b).and_then(|id| f.assembly(id, true))).flatten().collect();
    let part = |c: char| {
        let id = f.glyph(c)?;
        known.iter().find(|p| p.id == id).copied()
    };
    // (the braces' extender is a glyph of their own, its stem where the halves have theirs)
    let extender = known.iter().find(|p| p.extender).copied()?;
    Some(vec![part(bottom)?, extender, part(top)?])
}

/// Horizontal glyph with a width of at least `target`.
pub fn horizontal(c: char, target: f32, e: Style) -> R<MathBox> {
    let f = e.font();
    let s = e.scale();
    let id = f.glyph(c).ok_or(Error::MissingGlyph { character: c })?;
    let width = |g| {
        let (x0, _, x1, _) = f.limits(g);
        (x1 - x0) * s
    };
    if width(id) >= target {
        return Ok(glyph_id(id, s, f));
    }
    let variants = f.variants(id, false);
    for (v, _) in &variants {
        if width(*v) >= target {
            return Ok(glyph_id(*v, s, f));
        }
    }
    if let Some(parts) = f.assembly(id, false) {
        return Ok(assemble(&Assembly { parts: &parts, target: target / s, scale: s, vertical: false }, f));
    }
    if let Some(built) = composite_arrow(GlyphRef { symbol: c, id }, target, e)? {
        return Ok(built);
    }
    Ok(variants.last().map_or(glyph_id(id, s, f), |(v, _)| glyph_id(*v, s, f)))
}

/// The glyph being stretched, and the font id of that same glyph (its ink sets where a pair sits).
struct GlyphRef {
    symbol: char,
    id: ttf_parser::GlyphId,
}

/// Which stretchable arrows stand in for a glyph the font draws at one size only, and how much of
/// the target each one covers. `lower` is absent when the glyph is a single long arrow.
struct ArrowRecipe {
    upper: char,
    lower: Option<char>,
    upper_share: f32,
    lower_share: f32,
}

/// A pair of stretchable arrows stacked inside the bounds of `source`.
struct Stacked {
    source: ttf_parser::GlyphId,
    upper: char,
    lower: char,
    upper_share: f32,
    lower_share: f32,
    target: f32,
}

/// Arrows the font draws at one size only (STIX Two Math has no assembly for ⇌, ⇄ or ⟶): built
/// from arrows it can stretch, as `\xrightleftharpoons` and mhchem's `<=>` need. A pair is stacked
/// where the glyph has its two arrows (the upper one's top on the glyph's top, the lower one's
/// bottom on its bottom); a long arrow is its short one stretched.
fn composite_arrow(glyph: GlyphRef, target: f32, e: Style) -> R<Option<MathBox>> {
    let Some(recipe) = arrow_recipe(glyph.symbol) else {
        return Ok(None);
    };
    if !parts_stretch(e, &recipe) {
        return Ok(None);
    }
    match recipe.lower {
        None => horizontal(recipe.upper, target, e).map(Some),
        Some(lower) => stack_arrows(&Stacked { source: glyph.id, upper: recipe.upper, lower, upper_share: recipe.upper_share, lower_share: recipe.lower_share, target }, e),
    }
}

/// The second number is the length of the upper and lower arrow, as a share of the whole (⥂ has a
/// short lower arrow).
fn arrow_recipe(symbol: char) -> Option<ArrowRecipe> {
    let (upper, lower, upper_share, lower_share) = match symbol {
        '⇌' => ('⇀', Some('↽'), 1.0, 1.0),
        '⇋' => ('↼', Some('⇁'), 1.0, 1.0),
        '⇄' => ('→', Some('←'), 1.0, 1.0),
        '⇆' => ('←', Some('→'), 1.0, 1.0),
        '⇉' => ('→', Some('→'), 1.0, 1.0),
        '⇇' => ('←', Some('←'), 1.0, 1.0),
        '⥂' => ('→', Some('←'), 1.0, 0.6),
        '⥄' => ('→', Some('←'), 0.6, 1.0),
        '⟶' => ('→', None, 1.0, 1.0),
        '⟵' => ('←', None, 1.0, 1.0),
        '⟷' => ('↔', None, 1.0, 1.0),
        '⟹' => ('⇒', None, 1.0, 1.0),
        '⟸' => ('⇐', None, 1.0, 1.0),
        '⟺' => ('⇔', None, 1.0, 1.0),
        '⟼' => ('↦', None, 1.0, 1.0),
        _ => return None,
    };
    Some(ArrowRecipe { upper, lower, upper_share, lower_share })
}

/// Both stand-ins have to be arrows the font can assemble; otherwise the glyph stays at its only size.
fn parts_stretch(e: Style, recipe: &ArrowRecipe) -> bool {
    let f = e.font();
    [Some(recipe.upper), recipe.lower].into_iter().flatten().all(|part| f.glyph(part).and_then(|id| f.assembly(id, false)).is_some())
}

/// Places the two arrows where the source glyph keeps them (and TeX's 2pt): the upper one's top on
/// the glyph's top, the lower one's bottom on its bottom, shafts three rule thicknesses apart. The
/// stretchable harpoons have taller barbs than the ones inside ⇌.
fn stack_arrows(stacked: &Stacked, e: Style) -> R<Option<MathBox>> {
    let f = e.font();
    let s = e.scale();
    let (_, bottom, _, top) = f.limits(stacked.source);
    let above = horizontal(stacked.upper, stacked.target * stacked.upper_share, e)?;
    let below = horizontal(stacked.lower, stacked.target * stacked.lower_share, e)?;
    let width = above.width.max(below.width).max(stacked.target);
    let mut math_box = MathBox::space(width);
    let (_, above_top) = ink(&above, e);
    let (below_bottom, _) = ink(&below, e);
    let (up, down) = separated(top * s - above_top, bottom * s - below_bottom, 3.0 * k(e, |c| c.fraction_rule_thickness()));
    math_box.place(&above, (width - above.width) / 2.0, up);
    math_box.place(&below, (width - below.width) / 2.0, down);
    Ok(Some(math_box))
}

/// Shifts the two shafts apart, equally, until the gap is at least `apart`.
fn separated(up: f32, down: f32, apart: f32) -> (f32, f32) {
    if up - down < apart {
        let extra = (apart - (up - down)) / 2.0;
        (up + extra, down - extra)
    } else {
        (up, down)
    }
}

/// Lowest and highest ink of the glyphs in a box, from their outlines. A box's height and depth
/// never go below zero, so they hide where a glyph that floats above (⏞) or below (⏟) the baseline
/// really is.
pub fn ink(math_box: &MathBox, e: Style) -> (f32, f32) {
    let f = e.font();
    let mut bounds = (f32::INFINITY, f32::NEG_INFINITY);
    for item in &math_box.items {
        if let Item::Glyph { id, y, scale, .. } = item {
            let (_, y0, _, y1) = f.limits(*id);
            bounds = (bounds.0.min(y + y0 * scale), bounds.1.max(y + y1 * scale));
        }
    }
    if bounds.0 > bounds.1 {
        (-math_box.depth, math_box.height)
    } else {
        bounds
    }
}

/// Delimiter centered on the math axis, covering `target` of total height.
pub fn delimiter(c: char, target: f32, e: Style) -> R<MathBox> {
    let mut math_box = vertical(c, target, e)?;
    let axis = k(e, |c| c.axis_height());
    let offset = axis - (math_box.height - math_box.depth) / 2.0;
    math_box.shift_vertical(offset);
    math_box.single_glyph = false;
    Ok(math_box)
}

/// Parts of one growing glyph, the size to reach (in font units) and the style scale to draw at.
struct Assembly<'a> {
    parts: &'a [Part],
    target: f32,
    scale: f32,
    vertical: bool,
}

/// The repeated parts and the overlap that makes them land on `target`.
struct Fitted {
    sequence: Vec<Part>,
    overlap: f32,
}

/// Assembles the parts (bottom to top or left to right), repeating the extenders
/// as few times as needed to reach the size, with overlap between connectors.
fn assemble(assembly: &Assembly, font: &Font) -> MathBox {
    place(&fit(assembly.parts, assembly.target, font), assembly, font)
}

/// Repeats the extenders until the run covers `target`, then widens the overlap to absorb the leftover.
fn fit(parts: &[Part], target: f32, font: &Font) -> Fitted {
    let overlap = font.min_overlap();
    let sequence = |n: usize| -> Vec<Part> { parts.iter().flat_map(|p| std::iter::repeat_n(*p, if p.extender { n } else { 1 })).collect() };
    let size = |seq: &[Part], ov: f32| -> f32 { seq.iter().map(|p| p.advance).sum::<f32>() - ov * seq.len().saturating_sub(1) as f32 };
    let mut n = 0;
    let mut seq = sequence(n);
    while size(&seq, overlap) < target && n < 200 {
        n += 1;
        seq = sequence(n);
    }
    // spread the leftover by increasing the overlap, never beyond the shortest connector
    let joined = seq.len().saturating_sub(1).max(1) as f32;
    let slack = seq.windows(2).map(|w| w[0].end.min(w[1].start)).fold(f32::INFINITY, f32::min);
    let excess = (size(&seq, overlap) - target).max(0.0);
    Fitted { sequence: seq, overlap: (overlap + excess / joined).min(slack.max(overlap)) }
}

/// Lays the fitted parts along the axis, each connector overlapping the previous one.
fn place(fitted: &Fitted, assembly: &Assembly, font: &Font) -> MathBox {
    let mut math_box = MathBox::empty();
    let mut cursor = 0.0;
    let scale = assembly.scale;
    for part in &fitted.sequence {
        let (x0, y0, _, _) = font.limits(part.id);
        let (x, y) = if assembly.vertical { (0.0, (cursor - y0) * scale) } else { ((cursor - x0) * scale, 0.0) };
        math_box.items.push(Item::Glyph { id: part.id, x, y, scale, color: None, flip: false });
        let glyph = glyph_id(part.id, scale, font);
        if assembly.vertical {
            math_box.width = math_box.width.max(glyph.width);
        } else {
            math_box.height = math_box.height.max(glyph.height);
            math_box.depth = math_box.depth.max(glyph.depth);
        }
        cursor += part.advance - fitted.overlap;
    }
    let total = (cursor + fitted.overlap) * scale;
    if assembly.vertical {
        math_box.height = total;
        math_box.depth = 0.0;
    } else {
        math_box.width = total;
    }
    math_box
}
