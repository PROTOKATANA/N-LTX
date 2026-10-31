//! Fractions, binomials and generalized fractions (TeX rule 15).

use crate::tree::{FractionStyle, Node};
use crate::Error;

use super::{em, k, Context, Level, MathBox, Style};

type R<T> = Result<T, Error>;

/// Numerator, denominator and the rule the input asked for.
pub(super) struct Fraction<'a> {
    pub num: &'a Node,
    pub den: &'a Node,
    pub style: FractionStyle,
    pub rule: bool,
    pub thickness: Option<f32>,
}

pub(super) fn compose_fraction(fraction: &Fraction, e: Style, ctx: &mut Context) -> R<MathBox> {
    let given = fraction.thickness;
    let e = match fraction.style {
        FractionStyle::Display => Style { level: Level::Display, ..e },
        FractionStyle::Row => e.not_display(),
        FractionStyle::Auto => e,
    };
    let n = super::compose_list(fraction.num, e.num(), ctx)?;
    let d = super::compose_list(fraction.den, e.den(), ctx)?;
    let display = e.level == Level::Display;
    let axis = k(e, |c| c.axis_height());
    let (mut u, mut v);
    let mut thickness = 0.0;
    // a thickness of 0 (\above0pt) is a fraction without a rule, as in TeX
    let rule = fraction.rule && given.is_none_or(|t| t > 0.0);
    if rule {
        thickness = k(e, |c| c.fraction_rule_thickness());
        u = if display { k(e, |c| c.fraction_numerator_display_style_shift_up()) } else { k(e, |c| c.fraction_numerator_shift_up()) };
        v = if display { k(e, |c| c.fraction_denominator_display_style_shift_down()) } else { k(e, |c| c.fraction_denominator_shift_down()) };
        let mut gap_n = if display { k(e, |c| c.fraction_num_display_style_gap_min()) } else { k(e, |c| c.fraction_numerator_gap_min()) };
        let mut gap_d = if display { k(e, |c| c.fraction_denom_display_style_gap_min()) } else { k(e, |c| c.fraction_denominator_gap_min()) };
        // a thickness the input gives (\above, \genfrac) is absolute, as TeX's dimensions are, and the
        // clearance follows it (TeX's rule 15d: 3θ in display style, θ otherwise)
        if let Some(t) = given {
            thickness = e.font().units_per_em() * t;
            gap_n = if display { 3.0 * thickness } else { thickness };
            gap_d = gap_n;
        }
        u = u.max(gap_n + thickness / 2.0 + axis + n.depth);
        v = v.max(gap_d + thickness / 2.0 - axis + d.height);
    } else {
        u = if display { k(e, |c| c.stack_top_display_style_shift_up()) } else { k(e, |c| c.stack_top_shift_up()) };
        v = if display { k(e, |c| c.stack_bottom_display_style_shift_down()) } else { k(e, |c| c.stack_bottom_shift_down()) };
        let gap_min = if display { k(e, |c| c.stack_display_style_gap_min()) } else { k(e, |c| c.stack_gap_min()) };
        let gap = (u - n.depth) - (d.height - v);
        if gap < gap_min {
            let delta = (gap_min - gap) / 2.0;
            u += delta;
            v += delta;
        }
    }
    // \nulldelimiterspace on each side, as in TeX
    let margin = em(e, 0.12);
    let width = n.width.max(d.width);
    let mut math_box = MathBox::space(width + 2.0 * margin);
    math_box.place(&n, margin + (width - n.width) / 2.0, u);
    math_box.place(&d, margin + (width - d.width) / 2.0, -v);
    if rule {
        math_box.place(&MathBox::bar(width, thickness, axis - thickness / 2.0), margin, 0.0);
    }
    Ok(math_box)
}
