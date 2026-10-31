//! From the tree to positioned boxes. Follows Appendix G of the TeXbook, with parameters
//! taken from the font's MATH table (as MathML Core describes).

pub mod alphabet;
mod arithmetic;
mod diagram;
pub mod kerning;
pub mod math_box;
mod stretch;
mod table;

mod accent;
mod arrow;
mod box_ops;
mod delimiter;
mod fraction;
mod frame;
mod glyphs;
mod number;
mod radical;
mod row;
mod scripts;
mod text;

pub use alphabet::math_italic;
pub use math_box::{Item, MathBox};
pub use row::{class_of, classes, compose_list};

pub(crate) use glyphs::{glyph, glyph_id};
pub(crate) use row::compose_row;
pub(crate) use scripts::is_integral;

use ttf_parser::math::Constants;

use crate::font::{Font, MathFont};
use crate::locale::Locale;
use crate::rules::{self, Registry};
use crate::tree::*;
use crate::Error;

type R<T> = Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Display,
    Text,
    Script,
    ScriptScript,
}

/// TeX style: size (display, text, script, scriptscript) and whether it is "cramped"
/// (superscripts rise less inside denominators and subscripts).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub level: Level,
    pub cramped: bool,
    /// The font the equation is set in.
    pub font: MathFont,
}

impl Style {
    pub fn display() -> Style {
        Style { level: Level::Display, cramped: false, font: MathFont::default() }
    }

    pub fn text() -> Style {
        Style { level: Level::Text, cramped: false, font: MathFont::default() }
    }

    /// The same style in another font.
    pub fn in_font(self, font: MathFont) -> Style {
        Style { font, ..self }
    }

    /// The style's font.
    pub fn font(self) -> &'static Font {
        self.font.get()
    }

    pub fn scale(self) -> f32 {
        match self.level {
            Level::Display | Level::Text => 1.0,
            Level::Script => self.font().script_scale(),
            Level::ScriptScript => self.font().script_script_scale(),
        }
    }

    /// 0 in text and display, 1 in scripts, 2 in scripts of scripts: the `ssty` form to draw.
    pub fn script_level(self) -> usize {
        match self.level {
            Level::Display | Level::Text => 0,
            Level::Script => 1,
            Level::ScriptScript => 2,
        }
    }

    pub fn is_script(self) -> bool {
        matches!(self.level, Level::Script | Level::ScriptScript)
    }

    fn sup(self) -> Style {
        let level = match self.level {
            Level::Display | Level::Text => Level::Script,
            _ => Level::ScriptScript,
        };
        Style { level, cramped: self.cramped, font: self.font }
    }

    fn sub(self) -> Style {
        Style { cramped: true, ..self.sup() }
    }

    fn num(self) -> Style {
        let level = match self.level {
            Level::Display => Level::Text,
            Level::Text => Level::Script,
            _ => Level::ScriptScript,
        };
        Style { level, cramped: self.cramped, font: self.font }
    }

    fn den(self) -> Style {
        Style { cramped: true, ..self.num() }
    }

    fn cramp(self) -> Style {
        Style { cramped: true, ..self }
    }

    fn not_display(self) -> Style {
        if self.level == Level::Display {
            Style { level: Level::Text, ..self }
        } else {
            self
        }
    }
}

pub struct Context<'a> {
    pub locale: Locale,
    pub registry: &'a mut Registry,
    /// A missing glyph becomes an error (strict) or a box with a warning (tolerant).
    pub strict: bool,
    /// How text is kerned.
    pub kerning: kerning::TextKerning,
}

/// MATH table constant already at the style's scale.
fn k(e: Style, f: impl Fn(&Constants<'static>) -> ttf_parser::math::MathValue<'static>) -> f32 {
    e.font().c(f) * e.scale()
}

fn em(e: Style, amount: f32) -> f32 {
    e.font().units_per_em() * amount * e.scale()
}

pub fn compose(node: &Node, e: Style, ctx: &mut Context) -> R<MathBox> {
    let leaf = matches!(node, Node::Identifier { .. } | Node::Operator { .. } | Node::Number { .. } | Node::FixedDelimiter { .. } | Node::LargeOperator { .. });
    match compose_node(node, e, ctx) {
        Err(Error::MissingGlyph { character }) if leaf && !ctx.strict => {
            ctx.registry.warn(rules::E_REC_8, format!("the font has no '{character}' (U+{:04X}): drew a box", character as u32));
            glyphs::glyph('□', e)
        }
        other => other,
    }
}

fn compose_node(node: &Node, e: Style, ctx: &mut Context) -> R<MathBox> {
    match node {
        Node::Number { value, group, thousands } => number::compose_number(&number::Number { value, group: *group, thousands: *thousands }, e, ctx),
        Node::Identifier { name, variant } => glyphs::compose_identifier(name, *variant, e),
        Node::Function { name, .. } => text::compose_function(name, e, ctx),
        Node::Operator { symbol: '…', .. } => glyphs::dots('.', e),
        Node::Operator { symbol: '⋯', .. } => glyphs::dots('⋅', e),
        Node::Operator { symbol, .. } => glyphs::glyph(*symbol, e),
        Node::FixedDelimiter { symbol, size, .. } => {
            let total = [1.2, 1.8, 2.4, 3.0][(*size as usize).clamp(1, 4) - 1];
            stretch::delimiter(*symbol, em(e, total), e)
        }
        Node::LargeOperator { symbol, .. } => scripts::large_operator(*symbol, e),
        Node::Row { children } => compose_row(children, e, ctx, false),
        Node::Fraction { num, den, style, rule, thickness } => {
            fraction::compose_fraction(&fraction::Fraction { num, den, style: *style, rule: *rule, thickness: *thickness }, e, ctx)
        }
        Node::Root { radicand, index } => radical::compose_root(&radical::Radical { radicand, index: index.as_deref() }, e, ctx),
        Node::Scripts { base, sup, sub } => scripts::compose_scripts(&scripts::Scripts { base, sup: sup.as_deref(), sub: sub.as_deref() }, e, ctx),
        Node::Delimited { open, close, parts, middles } => delimiter::compose_delimited(&delimiter::Delimited { open: *open, close: *close, parts, middles }, e, ctx),
        Node::Table { shape: kind, rows, columns, verticals, dashed, horizontals, between, intertexts, stretch } => {
            let lines = table::Lines { verticals, dashed, horizontals, stretch: *stretch };
            let spec = table::Table { kind: *kind, rows, columns, lines, between, intertexts };
            table::compose(&spec, e, ctx)
        }
        Node::Multicolumn { base, .. } => compose_list(base, e, ctx),
        Node::Unit { base, .. } => compose(base, e, ctx),
        Node::Raised { em: amount, height, depth, base } => {
            box_ops::compose_raised(&box_ops::Raise { amount: *amount, height: height.as_ref(), depth: depth.as_ref(), base }, e, ctx)
        }
        Node::Scaled { factor, vertical, base } => {
            let mut math_box = compose(base, e, ctx)?;
            box_ops::scale_xy(&mut math_box, *factor, vertical.unwrap_or(*factor));
            Ok(math_box)
        }
        Node::Resized { width, height, total, base } => box_ops::compose_resized(&box_ops::Resize { width, height, total: *total, base }, e, ctx),
        Node::Rotated { degrees, origin, base } => {
            let mut math_box = compose(base, e, ctx)?;
            box_ops::rotate(&mut math_box, *degrees, origin);
            Ok(math_box)
        }
        Node::Rule { width, height, raise } => Ok(MathBox::bar(em(e, *width), em(e, *height), em(e, *raise))),
        // \mathchoice: TeX sets the branch of the style it lands in
        Node::Choice { branches } => {
            let i = match e.level {
                Level::Display => 0,
                Level::Text => 1,
                Level::Script => 2,
                Level::ScriptScript => 3,
            };
            match branches.get(i) {
                Some(branch) => compose(branch, e, ctx),
                None => Ok(MathBox::empty()),
            }
        }
        // \vcenter: the box's middle on the math axis
        Node::VCenter { base } => box_ops::compose_vcenter(base, e, ctx),
        // \reflectbox: \scalebox{-1}[1], mirrored inside its own width
        Node::Reflected { base } => {
            let mut b = compose(base, e, ctx)?;
            b.mirror();
            Ok(b)
        }
        Node::Smashed { base, height, depth } => box_ops::compose_smashed(&box_ops::Smash { base, height: *height, depth: *depth }, e, ctx),
        Node::Overlapped { base, side } => box_ops::compose_overlap(&box_ops::Overlap { base, side: *side }, e, ctx),
        Node::VerticalArrow { symbol, left, right } => {
            arrow::compose_vertical_arrow(&arrow::VerticalArrow { symbol: *symbol, left: left.as_deref(), right: right.as_deref() }, e, ctx)
        }
        Node::Accent { base, accent, stretchy, below } => accent::compose_accent(&accent::Accent { base, accent: *accent, stretchy: *stretchy, below: *below }, e, ctx),
        Node::Bar { base, above } => accent::compose_bar(&accent::Bar { base, above: *above }, e, ctx),
        Node::Brace { base, above, label, bracket } => accent::compose_brace(&accent::Brace { base, above: *above, label: label.as_deref(), bracket: *bracket }, e, ctx),
        Node::Text { parts } => text::compose_text(parts, e, ctx),
        Node::Space { em: amount } => Ok(MathBox::space(em(e, *amount))),
        Node::Canceled { base, shape: kind } => frame::compose_canceled(&frame::Canceled { base, kind: *kind }, e, ctx),
        Node::Framed { base, shape, background, border } => {
            frame::compose_frame(&frame::Framed { base, shape: *shape, background: background.as_deref(), border: border.as_deref() }, e, ctx)
        }
        Node::Colored { color, base } => {
            let mut math_box = compose(base, e, ctx)?;
            math_box.colorize(color);
            Ok(math_box)
        }
        Node::Quantity { unit, .. } => compose(unit, e, ctx),
        Node::Arrow { symbol, above, below } => {
            arrow::compose_arrow(&arrow::LabeledArrow { symbol: *symbol, above: above.as_deref(), below: below.as_deref(), minimum: 0.0 }, e, ctx)
        }
        Node::Stacked { base, above, below } => scripts::compose_stacked(&scripts::Stack { base, above: above.as_deref(), below: below.as_deref() }, e, ctx),
        Node::Phantom { base, width, height } => box_ops::compose_phantom(&box_ops::Phantom { base, width: *width, height: *height }, e, ctx),
        Node::ForcedClass { base, .. } => compose(base, e, ctx),
        Node::Style { level, base } => box_ops::compose_styled(&box_ops::Styled { level: *level, base }, e, ctx),
        Node::ArrowDiagram { cells, arrows } => diagram::compose(&diagram::Diagram { cells, arrows }, e, ctx),
        #[cfg(feature = "chem")]
        Node::Chemistry { scheme } => crate::chem::layout::compose(scheme, e, ctx),
        Node::LongDivision { dividend, divisor } => arithmetic::long_division(dividend, divisor, e),
        Node::ColumnArithmetic { operation, terms } => arithmetic::column_arithmetic(*operation, terms, e),
    }
}
