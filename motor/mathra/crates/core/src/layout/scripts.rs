//! Superscripts, subscripts and limits above and below an operator.

use crate::tree::{Class, Node};
use crate::Error;

use super::glyphs::glyph_id;
use super::{em, k, Context, Level, MathBox, Style};

type R<T> = Result<T, Error>;

/// Base with the superscript and subscript the input attached to it.
pub(super) struct Scripts<'a> {
    pub base: &'a Node,
    pub sup: Option<&'a Node>,
    pub sub: Option<&'a Node>,
}

/// A box that already exists, with optional limits above and below (`\overset`, display sum, `lim`).
pub(super) struct Limited<'a> {
    pub base: MathBox,
    pub above: Option<&'a Node>,
    pub below: Option<&'a Node>,
}

/// `\overset` / `\underset`: the base is still a node, composed before the limits go on.
pub(super) struct Stack<'a> {
    pub base: &'a Node,
    pub above: Option<&'a Node>,
    pub below: Option<&'a Node>,
}

fn uses_limits(base: &Node, e: Style) -> bool {
    let display = e.level == super::Level::Display;
    match base {
        Node::LargeOperator { symbol, limits } => {
            let by_default = !is_integral(*symbol);
            limits.unwrap_or(by_default && display)
        }
        Node::Function { limits, .. } => *limits && display,
        Node::ForcedClass { base, class: Class::Op } => uses_limits(base, e) || display,
        _ => false,
    }
}

/// Integrals take their limits on the side, even in display style.
pub(crate) fn is_integral(c: char) -> bool {
    matches!(c, '∫' | '∬' | '∭' | '⨌' | '∮' | '∯' | '∰' | '∱' | '∲' | '∳' | '⨏' | '⨖')
}

pub(super) fn compose_scripts(scripts: &Scripts, e: Style, ctx: &mut Context) -> R<MathBox> {
    let (base, sup, sub) = (scripts.base, scripts.sup, scripts.sub);
    let b = super::compose(base, e, ctx)?;
    if uses_limits(base, e) {
        return limits(Limited { base: b, above: sup, below: sub }, e, ctx);
    }
    let integral = matches!(base, Node::LargeOperator { .. });
    // the prime of f' is already drawn as a superscript: it goes at normal size, attached to the base
    let (primes, sup) = split_primes(sup);
    let mut b = b;
    if let Some(p) = primes {
        let mut with_primes = b.clone();
        with_primes.append(&MathBox::space(b.italic));
        with_primes.append(&super::compose(&p, e, ctx)?);
        with_primes.single_glyph = false;
        with_primes.italic = 0.0;
        b = with_primes;
        if sup.is_none() && sub.is_none() {
            return Ok(b);
        }
    }
    let is_simple = b.single_glyph && !integral;
    let s = sup.as_ref().map(|n| super::compose_list(n, e.sup(), ctx)).transpose()?;
    let d = sub.map(|n| super::compose_list(n, e.sub(), ctx)).transpose()?;

    let mut u = 0.0;
    let mut v = 0.0;
    if let Some(s) = &s {
        let rise = match e.font.display_superscript_shift() {
            Some(shift) if e.level == Level::Display && !e.cramped => em(e, shift),
            _ if e.cramped => k(e, |c| c.superscript_shift_up_cramped()),
            _ => k(e, |c| c.superscript_shift_up()),
        };
        let drop = if is_simple { 0.0 } else { b.height - k(e, |c| c.superscript_baseline_drop_max()) };
        u = rise.max(drop).max(s.depth + k(e, |c| c.superscript_bottom_min()));
    }
    if let Some(d) = &d {
        let descent = match e.font.lone_subscript_shift() {
            Some(shift) if s.is_none() => em(e, shift),
            _ => k(e, |c| c.subscript_shift_down()),
        };
        let drop = if is_simple { 0.0 } else { b.depth + k(e, |c| c.subscript_baseline_drop_min()) };
        v = descent.max(drop).max(d.height - k(e, |c| c.subscript_top_max()));
    }
    if let (Some(s), Some(d)) = (&s, &d) {
        let gap = (u - s.depth) - (d.height - v);
        let gap_min = k(e, |c| c.sub_superscript_gap_min());
        if gap < gap_min {
            v += gap_min - gap;
        }
        let raise_by = k(e, |c| c.superscript_bottom_max_with_subscript()) - (u - s.depth);
        if raise_by > 0.0 {
            u += raise_by;
            v -= raise_by;
        }
    }
    let delta = b.italic;
    let (x_sup, x_sub) = if integral { (b.width, b.width - delta) } else { (b.width + delta, b.width) };
    let mut math_box = b.clone();
    math_box.single_glyph = false;
    let mut width = b.width;
    if let Some(s) = &s {
        math_box.place(s, x_sup, u);
        width = width.max(x_sup + s.width);
    }
    if let Some(d) = &d {
        math_box.place(d, x_sub, -v);
        width = width.max(x_sub + d.width);
    }
    math_box.width = width + k(e, |c| c.space_after_script());
    math_box.italic = 0.0;
    Ok(math_box)
}

/// Takes the primes (′ ″ ‴) off the start of the superscript: f'^2 becomes f′ with exponent 2.
fn split_primes(sup: Option<&Node>) -> (Option<Node>, Option<Node>) {
    let prime = |n: &Node| matches!(n, Node::Operator { symbol: '′' | '″' | '‴', .. });
    match sup {
        Some(n) if prime(n) => (Some(n.clone()), None),
        Some(Node::Row { children }) if children.first().is_some_and(prime) => {
            let qty = children.iter().take_while(|f| prime(f)).count();
            let remainder = &children[qty..];
            let remainder = if remainder.is_empty() { None } else { Some(Node::row(remainder.to_vec()).simplify()) };
            (Some(Node::row(children[..qty].to_vec())), remainder)
        }
        other => (None, other.cloned()),
    }
}

/// Limits above and below (display sum, `lim`, `\overset`).
pub(super) fn limits(limited: Limited, e: Style, ctx: &mut Context) -> R<MathBox> {
    let Limited { base: b, above, below } = limited;
    let s = above.map(|n| super::compose(n, e.sup(), ctx)).transpose()?;
    let d = below.map(|n| super::compose(n, e.sub(), ctx)).transpose()?;
    let width = b.width.max(s.as_ref().map_or(0.0, |s| s.width)).max(d.as_ref().map_or(0.0, |d| d.width));
    let delta = b.italic;
    let mut math_box = MathBox::space(width);
    math_box.place(&b, (width - b.width) / 2.0, 0.0);
    if let Some(s) = &s {
        let gap = k(e, |c| c.upper_limit_gap_min()).max(k(e, |c| c.upper_limit_baseline_rise_min()) - s.depth);
        math_box.place(s, (width - s.width + delta) / 2.0, b.height + gap + s.depth);
    }
    if let Some(d) = &d {
        let gap = k(e, |c| c.lower_limit_gap_min()).max(k(e, |c| c.lower_limit_baseline_drop_min()) - d.height);
        math_box.place(d, (width - d.width - delta) / 2.0, -(b.depth + gap + d.height));
    }
    if let Some(padding) = e.font.limit_padding() {
        if s.is_some() {
            math_box.height += em(e, padding);
        }
        if d.is_some() {
            math_box.depth += em(e, padding);
        }
    }
    Ok(math_box)
}

pub(super) fn compose_stacked(stack: &Stack, e: Style, ctx: &mut Context) -> R<MathBox> {
    let base = super::compose(stack.base, e, ctx)?;
    limits(Limited { base, above: stack.above, below: stack.below }, e, ctx)
}

pub(super) fn large_operator(c: char, e: Style) -> R<MathBox> {
    let f = e.font();
    let id = f.glyph(c).ok_or(Error::MissingGlyph { character: c })?;
    let mut chosen = id;
    if e.level == super::Level::Display {
        let minimum = f.display_operator_min_height();
        for (v, measure) in f.variants(id, true) {
            chosen = v;
            if measure >= minimum {
                break;
            }
        }
    }
    let mut math_box = glyph_id(f.script_form(chosen, e.script_level()), e.scale(), f);
    // centered on the math axis
    let axis = k(e, |c| c.axis_height());
    let offset = axis - (math_box.height - math_box.depth) / 2.0;
    math_box.shift_vertical(offset);
    math_box.single_glyph = true;
    Ok(math_box)
}
