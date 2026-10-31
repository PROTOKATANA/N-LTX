//! Roots: the radical sign, the rule, and the optional index.

use crate::tree::Node;
use crate::Error;

use super::{k, Context, Level, MathBox, Style};

type R<T> = Result<T, Error>;

/// The radicand and its index, when the root has one.
pub(super) struct Radical<'a> {
    pub radicand: &'a Node,
    pub index: Option<&'a Node>,
}

pub(super) fn compose_root(radical: &Radical, e: Style, ctx: &mut Context) -> R<MathBox> {
    let f = e.font();
    let base = super::compose_list(radical.radicand, e.cramp(), ctx)?;
    let thickness = k(e, |c| c.radical_rule_thickness());
    let mut gap = if e.level == Level::Display { k(e, |c| c.radical_display_style_vertical_gap()) } else { k(e, |c| c.radical_vertical_gap()) };
    let extra = k(e, |c| c.radical_extra_ascender());
    let target = base.height + base.depth + gap + thickness;
    let mut sign = super::stretch::vertical('√', target, e)?;
    let total = sign.total();
    if total > target {
        gap += (total - target) / 2.0;
    }
    // the top of the radical touches the rule above
    let top = base.height + gap + thickness;
    let dy = top - sign.height;
    sign.shift_vertical(dy);

    let mut math_box = MathBox::empty();
    if let Some(ind) = radical.index {
        let ei = Style { level: Level::ScriptScript, cramped: true, font: e.font };
        let i = super::compose(ind, ei, ctx)?;
        let before = k(e, |c| c.radical_kern_before_degree());
        let after = k(e, |c| c.radical_kern_after_degree());
        let raise = f.radical_degree_raise() * sign.total() - sign.depth;
        math_box.place(&i, before, raise + i.depth);
        math_box.width = (before + i.width + after).max(0.0);
    }
    let x_radical = math_box.width;
    math_box.place(&sign, x_radical, 0.0);
    let x_base = x_radical + sign.width;
    math_box.place(&base, x_base, 0.0);
    math_box.place(&MathBox::bar(base.width, thickness, top - thickness), x_base, 0.0);
    math_box.width = x_base + base.width;
    math_box.height = math_box.height.max(top + extra);
    Ok(math_box)
}
