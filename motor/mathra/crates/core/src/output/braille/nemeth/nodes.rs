//! Each math construct, written as Nemeth cells.

use super::super::{cells, letter};
use super::signs::{digit, enlarged, greek, sign, typeform};
use super::{Ctx, Item, Kind, Writer, SUBSCRIPT, SUPERSCRIPT};
use crate::tree::*;

/// Numerator, denominator and whether the bar is drawn.
#[derive(Clone, Copy)]
struct Fraction<'a> {
    num: &'a Node,
    den: &'a Node,
    rule: bool,
}

/// Base with the superscript and subscript attached to it.
#[derive(Clone, Copy)]
struct Scripts<'a> {
    base: &'a Node,
    sup: Option<&'a Node>,
    sub: Option<&'a Node>,
}

/// An expression and what is written directly under and over it (§15).
#[derive(Clone, Copy)]
struct Modified<'a> {
    base: &'a Node,
    under: Option<&'a Node>,
    over: Option<&'a Node>,
}

/// A horizontal arrow and the nodes set above and below it.
#[derive(Clone, Copy)]
struct LabeledArrow<'a> {
    symbol: char,
    above: Option<&'a Node>,
    below: Option<&'a Node>,
}

/// An accent and the base it sits on. `below` is an under-accent.
#[derive(Clone, Copy)]
struct Accent<'a> {
    base: &'a Node,
    accent: char,
    below: bool,
}

/// What a pair of fences encloses, and the fences between its parts.
#[derive(Clone, Copy)]
struct Delimited<'a> {
    open: Option<char>,
    close: Option<char>,
    parts: &'a [Node],
    middles: &'a [char],
}

/// Rows of a matrix or a system, with the enlarged grouping signs on every line.
#[derive(Clone, Copy)]
struct RowGroup<'a> {
    rows: &'a [Vec<Node>],
    open: &'a str,
    close: &'a str,
}

/// A horizontal brace or bracket, and the label it points at.
#[derive(Clone, Copy)]
struct Brace<'a> {
    base: &'a Node,
    above: bool,
    label: Option<&'a Node>,
    bracket: bool,
}

impl Writer {
    pub(super) fn node(&mut self, node: &Node, ctx: &Ctx) {
        match node {
            // \mathchoice: the text style's branch
            Node::Choice { branches } => {
                if let Some(b) = branches.get(1) {
                    self.node(b, ctx);
                }
            }
            Node::Number { value, .. } => self.number(value, ctx),
            Node::Identifier { name, variant } => self.identifier(name, *variant, ctx),
            Node::Function { name, .. } => self.function_name(name, ctx),
            Node::Operator { symbol, .. } | Node::FixedDelimiter { symbol, .. } => self.symbol(*symbol, ctx),
            Node::LargeOperator { symbol, .. } => self.large_operator(*symbol, ctx),
            Node::Row { children } => self.row(children, ctx),
            Node::Fraction { num, den, rule, .. } => self.fraction(&Fraction { num, den, rule: *rule }, ctx),
            Node::Root { radicand, index } => self.root(radicand, index.as_deref(), ctx),
            Node::Scripts { base, sup, sub } => self.scripts(&Scripts { base, sup: sup.as_deref(), sub: sub.as_deref() }, ctx),
            Node::Delimited { open, close, parts, middles } => self.delimited(&Delimited { open: *open, close: *close, parts, middles }, ctx),
            Node::Table { shape, rows, .. } => self.table(*shape, rows, ctx),
            Node::Accent { base, accent, below, .. } => self.accent(&Accent { base, accent: *accent, below: *below }, ctx),
            Node::Bar { base, above } => self.accent(&Accent { base, accent: '\u{0304}', below: !*above }, ctx),
            Node::Brace { base, above, label, bracket } => self.brace(&Brace { base, above: *above, label: label.as_deref(), bracket: *bracket }, ctx),
            Node::Text { parts } => {
                for p in parts {
                    match p {
                        TextPart::Text(t) => self.text(t, ctx),
                        TextPart::Math(m) => self.node(m, ctx),
                    }
                }
            }
            Node::Arrow { symbol, above, below } => self.labeled_arrow(&LabeledArrow { symbol: *symbol, above: above.as_deref(), below: below.as_deref() }, ctx),
            Node::Stacked { base, above, below } => self.modified(&Modified { base, under: below.as_deref(), over: above.as_deref() }, ctx),
            // cancellation indicators around the canceled expression (§12)
            Node::Canceled { base, .. } => {
                self.put(ctx, "⠪", Kind::Other);
                self.node(base, ctx);
                self.put(ctx, "⠻", Kind::Other);
            }
            // a unit is an abbreviation, spaced from its number (§10, §23.1)
            Node::Quantity { value: Some(_), unit } => self.quantity(unit, ctx),
            // a space as wide as a word space in print is a blank cell
            Node::Space { em } if *em > 0.3 => self.space_to(&ctx.level),
            // a boxed expression: the rectangle with the expression as its interior modifier (§15.10, Rule 17)
            Node::Framed { base, shape: Frame::Rectangle, .. } => {
                self.put_dots(ctx, "1246 1235 456 1246", Kind::Other);
                self.node(base, ctx);
                self.put(ctx, "⠻", Kind::Other);
            }
            // the phasor A∠θ: the angle sign, then the angle
            Node::Framed { base, shape: Frame::Phase, .. } => self.node(&Node::row(vec![Node::Operator { symbol: '∠', class: Class::Ord }, (**base).clone()]), ctx),
            // the code has no sign for the actuarial angle
            Node::Framed { base, shape: Frame::ActuarialAngle, .. } => {
                self.node(base, ctx);
                self.missing(ctx, '⌉');
            }
            Node::Framed { base, .. }
            | Node::Colored { base, .. }
            | Node::ForcedClass { base, .. }
            | Node::Style { base, .. }
            | Node::Multicolumn { base, .. }
            | Node::Raised { base, .. }
            | Node::Scaled { base, .. }
            | Node::Smashed { base, .. }
            | Node::VCenter { base }
            | Node::Resized { base, .. }
            | Node::Rotated { base, .. }
            | Node::Reflected { base }
            | Node::Overlapped { base, .. }
            | Node::Unit { base, .. }
            | Node::Quantity { unit: base, .. } => self.node(base, ctx),
            Node::Space { .. } | Node::Phantom { .. } | Node::Rule { .. } => {}
            Node::VerticalArrow { symbol, .. } => self.missing(ctx, *symbol),
            #[cfg(feature = "chem")]
            Node::Chemistry { scheme } => self.node(&crate::chem::speech::as_nodes(scheme), ctx),
            Node::ArrowDiagram { cells, arrows } => self.arrow_diagram(cells, arrows, ctx),
            Node::LongDivision { dividend, divisor } => {
                self.number(dividend, ctx);
                self.put_dots(ctx, "46 34", Kind::Other);
                self.number(divisor, ctx);
            }
            Node::ColumnArithmetic { operation, terms } => self.column_arithmetic(*operation, terms, ctx),
        }
    }

    /// A horizontal brace or bracket is a modifier; one pointing at a label cannot be drawn.
    fn brace(&mut self, brace: &Brace<'_>, ctx: &Ctx) {
        let Brace { base, above, label, bracket } = *brace;
        // a horizontal brace or bracket is a modifier (Ex. 19-17 to 19-20); one pointing to a
        // label must be drawn (§19.2), which braille text cannot do
        if label.is_some() {
            self.node(base, ctx);
            self.missing(ctx, if above { '⏞' } else { '⏟' });
            return;
        }
        let mark = match (above, bracket) {
            (true, false) => "46 12356",
            (false, false) => "46 23456",
            (true, true) => "4 12356",
            (false, true) => "4 23456",
        };
        self.put(ctx, "⠐", Kind::Indicator);
        self.node(base, ctx);
        self.put(ctx, if above { "⠣" } else { "⠩" }, Kind::Other);
        self.put_dots(ctx, mark, Kind::Other);
        self.put(ctx, "⠻", Kind::Other);
    }

    /// A unit is an abbreviation: its spaces stay, and its letters take no English-letter indicator.
    fn quantity(&mut self, unit: &Node, ctx: &Ctx) {
        let start = self.items.len();
        match unit {
            Node::Row { children } => {
                for c in children {
                    match c {
                        Node::Space { .. } => self.space_to(&ctx.level),
                        other => self.node(other, ctx),
                    }
                }
            }
            other => self.node(other, ctx),
        }
        // a unit is an abbreviation: no English-letter indicator (§6.3.1 c)
        for item in &mut self.items[start..] {
            if let Item::Cells(_, kind @ Kind::Letter) = item {
                *kind = Kind::Other;
            }
        }
    }

    /// Each arrow is one more line, after the rows of cells.
    fn arrow_diagram(&mut self, cells: &[Vec<Node>], arrows: &[DiagramArrow], ctx: &Ctx) {
        let lines: Vec<Vec<Node>> = cells.iter().cloned().chain(arrows.iter().map(|a| vec![a.as_row(cells)])).collect();
        self.table(TableKind::Diagram, &lines, ctx);
    }

    fn column_arithmetic(&mut self, operation: char, terms: &[String], ctx: &Ctx) {
        for (i, t) in terms.iter().enumerate() {
            if i > 0 {
                self.symbol(operation, ctx);
            }
            self.number(t, ctx);
        }
    }

    fn row(&mut self, children: &[Node], ctx: &Ctx) {
        let lists = enclosed_lists(children);
        let mut i = 0;
        while i < children.len() {
            let child = &children[i];
            let list_ctx;
            let ctx = if lists[i] && !ctx.list {
                list_ctx = Ctx { list: true, ..ctx.clone() };
                &list_ctx
            } else {
                ctx
            };
            // a mixed number: a whole number followed by a fraction of numbers (§13.4)
            if self.mixed_number(child, children.get(i + 1), ctx) {
                i += 2;
                continue;
            }
            self.node(child, ctx);
            // what follows a system or a matrix is placed on its top line (§19.6)
            if self.place_on_first_row(&children[i + 1..], ctx) {
                return;
            }
            // a function name is followed by a space (§18), unless punctuation or a closing sign follows
            if is_function(child) && children.get(i + 1).is_some_and(|n| !matches!(n, Node::Operator { class: Class::Punct | Class::Close, .. })) {
                self.space(&ctx.level, true);
            }
            i += 1;
        }
    }

    /// A whole number followed by a fraction of numbers (§13.4). Returns whether it wrote one.
    fn mixed_number(&mut self, child: &Node, next: Option<&Node>, ctx: &Ctx) -> bool {
        let (Node::Number { value, .. }, Some(Node::Fraction { num, den, rule: true, .. })) = (child, next) else {
            return false;
        };
        if !(matches!(**num, Node::Number { .. }) && matches!(**den, Node::Number { .. }) && !value.contains('.')) {
            return false;
        }
        self.number(value, ctx);
        self.put_dots(ctx, "456 1456", Kind::Other);
        self.node(num, ctx);
        self.put_dots(ctx, "34", Kind::Other);
        self.node(den, ctx);
        self.put_dots(ctx, "456 3456", Kind::Other);
        true
    }

    /// What follows a system or a matrix is placed on its top line (§19.6).
    fn place_on_first_row(&mut self, rest: &[Node], ctx: &Ctx) -> bool {
        let Some(at) = self.first_row_end.take() else {
            return false;
        };
        let start = self.items.len();
        self.row(rest, ctx);
        let tail: Vec<Item> = self.items.drain(start..).collect();
        self.items.splice(at..at, tail);
        true
    }

    fn identifier(&mut self, name: &str, variant: Variant, ctx: &Ctx) {
        let mut chars = name.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            if matches!(variant, Variant::Italic | Variant::Upright) {
                // a word: its letters, uncontracted
                self.text(name, ctx);
            } else {
                // letters in a typeform: each takes its indicators (§7)
                let letters: String = name.chars().filter_map(|c| self.letter_cells(c).map(|l| format!("{}{l}", typeform(c, variant)))).collect();
                self.put(ctx, &letters, Kind::Other);
            }
            return;
        };
        let Some(base) = self.letter_cells(c) else {
            // a letterlike symbol written as a letter (ℵ, ℏ in MathML's <mi>)
            self.symbol(c, ctx);
            return;
        };
        let prefix = typeform(c, variant);
        let kind = if prefix.is_empty() && c.is_ascii_alphabetic() && !ctx.list && !ctx.table { Kind::Letter } else { Kind::Other };
        self.put(ctx, &format!("{prefix}{base}"), kind);
    }

    /// A Latin letter (capitals with ⠠) or a Greek one (⠨, capitals ⠨⠠, alternative forms ⠨⠈), §6.1.
    fn letter_cells(&self, c: char) -> Option<String> {
        if c.is_ascii_alphabetic() {
            let base = cells(letter(c)?);
            return Some(if c.is_ascii_uppercase() { format!("⠠{base}") } else { base });
        }
        let (dots, capital, alternative) = greek(c)?;
        Some(format!("⠨{}{}{}", if alternative { "⠈" } else { "" }, if capital { "⠠" } else { "" }, cells(dots)))
    }

    fn function_name(&mut self, name: &str, ctx: &Ctx) {
        let letters: String = name.chars().filter(|c| !c.is_whitespace()).filter_map(|c| letter(c).map(cells)).collect();
        self.put(ctx, &letters, Kind::Function);
    }

    /// Words inside the equation: letters uncontracted, capitals with ⠠, numbers with their indicator.
    fn text(&mut self, t: &str, ctx: &Ctx) {
        let mut word = String::new();
        let mut digits = String::new();
        let flush_word = |w: &mut Self, word: &mut String| {
            if !word.is_empty() {
                w.put(ctx, word, Kind::Other);
                word.clear();
            }
        };
        for c in t.chars().filter(|c| !crate::input::latex::MARKERS.contains(c)) {
            if c.is_ascii_digit() {
                flush_word(self, &mut word);
                digits.push(c);
                continue;
            }
            if !digits.is_empty() {
                self.number(&digits, ctx);
                digits.clear();
            }
            if c.is_whitespace() {
                flush_word(self, &mut word);
                self.space_to(&ctx.level);
            } else if let Some(l) = self.letter_cells(c) {
                word.push_str(&l);
            } else {
                flush_word(self, &mut word);
                self.symbol(c, ctx);
            }
        }
        flush_word(self, &mut word);
        if !digits.is_empty() {
            self.number(&digits, ctx);
        }
    }

    fn symbol(&mut self, c: char, ctx: &Ctx) {
        // the degree sign is written at superscript level, even when print does not raise it (§23.10)
        if c == '°' {
            self.put_dots(&ctx.at(format!("{}{SUPERSCRIPT}", ctx.level)), "46 16", Kind::Other);
            return;
        }
        if let Some(l) = greek(c).and_then(|_| self.letter_cells(c)) {
            self.put(ctx, &l, Kind::Other);
            return;
        }
        let Some((dots, kind)) = sign(c) else {
            self.missing(ctx, c);
            return;
        };
        match kind {
            // comparison signs are spaced (§21.13); the space before one returns to the baseline, the
            // space after it keeps the level in effect (§14.8.7, f(x)|ₓ₌₄ = ⠳⠰⠭⠀⠰⠨⠅⠀⠼⠲)
            Kind::Comparison => {
                self.space_to("");
                self.put_dots(ctx, dots, kind);
                self.space_to(&ctx.level);
            }
            // the mathematical comma: ⠪ at script level (§14), and a space after it on the baseline
            Kind::Punctuation if c == ',' => {
                if ctx.level.is_empty() {
                    self.put_dots(ctx, dots, kind);
                    self.space_to("");
                } else {
                    self.put_dots(ctx, "246", Kind::Other);
                }
            }
            Kind::Shape => {
                self.put_dots(ctx, dots, kind);
                self.space_to(&ctx.level);
            }
            Kind::Spaced => {
                self.space_to(&ctx.level);
                self.put_dots(ctx, dots, kind);
                self.space_to(&ctx.level);
            }
            // other punctuation returns to the baseline (§14.8)
            Kind::Punctuation => {
                self.put_dots(ctx, dots, kind);
                self.level = String::new();
                if matches!(c, ';' | ':') {
                    self.space_to("");
                }
            }
            _ => self.put_dots(ctx, dots, kind),
        }
    }

    fn large_operator(&mut self, c: char, ctx: &Ctx) {
        let dots = match c {
            '∑' => "46 6 234",
            '∏' => "46 6 1234",
            '∫' => "2346",
            '∬' => "2346 2346",
            '∭' => "2346 2346 2346",
            '∮' => "2346 4 1246 14 12456",
            // ⋃ ⋂ ⋀ ⋁: the signs of union, intersection, logical product and sum
            other => match sign(other) {
                Some((dots, _)) => dots,
                None => {
                    self.missing(ctx, other);
                    return;
                }
            },
        };
        self.put_dots(ctx, dots, Kind::Other);
    }

    fn fraction(&mut self, fraction: &Fraction<'_>, ctx: &Ctx) {
        let Fraction { num, den, rule } = *fraction;
        if !rule {
            // binomial coefficient: upper part, directly-under indicator, lower part (§15.6)
            self.node(num, ctx);
            self.put_dots(ctx, "146", Kind::Other);
            self.node(den, ctx);
            return;
        }
        // simple ⠹⠌⠼, complex ⠠⠹⠠⠌⠠⠼, hypercomplex ⠠⠠⠹… (§13)
        let order = fraction_depth(num).max(fraction_depth(den));
        let prefix = "⠠".repeat(order);
        let inner = Ctx { list: false, ..ctx.clone() };
        self.put(ctx, &format!("{prefix}⠹"), Kind::Other);
        self.node(num, &inner);
        self.put(ctx, &format!("{prefix}⠌"), Kind::Other);
        self.node(den, &inner);
        self.put(ctx, &format!("{prefix}⠼"), Kind::Other);
    }

    fn root(&mut self, radicand: &Node, index: Option<&Node>, ctx: &Ctx) {
        // §16: index-of-radical ⠣, radical ⠜, termination ⠻; inner radicals take ⠨ per depth
        let order = "⠨".repeat(ctx.radicals);
        if let Some(i) = index {
            self.put(ctx, "⠣", Kind::Other);
            self.node(i, ctx);
        }
        self.put(ctx, &format!("{order}⠜"), Kind::Other);
        let inner = Ctx { radicals: ctx.radicals + 1, ..ctx.clone() };
        self.node(radicand, &inner);
        self.put(ctx, &format!("{order}⠻"), Kind::Other);
    }

    fn scripts(&mut self, scripts: &Scripts<'_>, ctx: &Ctx) {
        let Scripts { base, sup, sub } = *scripts;
        // Σ, ∏, lim, max…: limits under and over, as a modified expression (§15)
        if self.script_limits(base, sub) {
            self.modified(&Modified { base, under: sub, over: sup }, ctx);
            return;
        }
        self.node(base, ctx);
        // primes come first, at the level of the base (§14.12)
        let (primes, sup) = split_primes(sup);
        if primes > 0 {
            self.put(ctx, &"⠄".repeat(primes), Kind::Other);
        }
        if let Some(s) = sub {
            self.subscript_on(base, s, ctx);
        }
        if let Some(s) = sup {
            self.superscript_on(&s, ctx);
        }
    }

    fn script_limits(&self, base: &Node, sub: Option<&Node>) -> bool {
        match base {
            // integrals take their bounds as scripts (Ex. 23-16), unless print stacks them (Ex. 23-17)
            Node::LargeOperator { symbol, limits } => !matches!(symbol, '∫' | '∬' | '∭' | '∮') || *limits == Some(true),
            Node::Function { limits, .. } => *limits && sub.is_some(),
            _ => false,
        }
    }

    fn subscript_on(&mut self, base: &Node, s: &Node, ctx: &Ctx) {
        let numeric = matches!(s, Node::Number { value, .. } if !value.contains('.'));
        // a letter, also an upright one (H in H₂O), or an abbreviated function name
        let base = match base {
            Node::Unit { base, .. } => base.as_ref(),
            other => other,
        };
        let on_letter = matches!(base, Node::Identifier { name, .. } if name.chars().count() == 1)
            || matches!(base, Node::Function { .. })
            || matches!(base, Node::Text { parts } if matches!(parts.as_slice(), [TextPart::Text(t)] if !t.is_empty() && t.chars().all(|c| c.is_ascii_alphabetic())));
        if numeric && on_letter && ctx.level.is_empty() {
            // x₁: a numeric subscript of first order needs no indicator (§14.6)
            if let Node::Number { value, .. } = s {
                let digits: String = value.chars().map(|d| cells(digit(d))).collect();
                self.put(ctx, &digits, Kind::Other);
            }
        } else {
            self.node(s, &ctx.at(format!("{}{SUBSCRIPT}", ctx.level)));
        }
    }

    fn superscript_on(&mut self, s: &Node, ctx: &Ctx) {
        let level = ctx.at(format!("{}{SUPERSCRIPT}", ctx.level));
        if matches!(s, Node::Operator { symbol: '°' | '∘', .. }) {
            // the degree sign is written at superscript level (§23.10)
            self.put_dots(&level, "46 16", Kind::Other);
        } else {
            self.node(s, &level);
        }
    }

    /// Five steps (§15): ⠐, the expression, ⠩ under / ⠣ over (the under part first), the modifier, ⠻.
    fn modified(&mut self, modified: &Modified<'_>, ctx: &Ctx) {
        let Modified { base, under, over } = *modified;
        // a modified comparison sign is spaced as a whole, not inside the modification
        let comparison = match base {
            // a modified arrow takes its uncontracted form (§22.1)
            Node::Operator { symbol: '→', .. } => Some("1246 25 25 135"),
            Node::Operator { symbol, .. } => sign(*symbol).filter(|(_, k)| *k == Kind::Comparison).map(|(d, _)| d),
            _ => None,
        };
        if comparison.is_some() {
            self.space_to("");
        }
        // the indicator of a modified comparison sign belongs to it: no English-letter indicator next to it
        self.put(ctx, "⠐", if comparison.is_some() { Kind::Comparison } else { Kind::Indicator });
        match comparison {
            Some(dots) => self.put_dots(ctx, dots, Kind::Comparison),
            None => self.node(base, ctx),
        }
        if let Some(u) = under {
            self.put(ctx, "⠩", Kind::Other);
            self.node(u, ctx);
        }
        if let Some(o) = over {
            self.put(ctx, "⠣", Kind::Other);
            self.node(o, ctx);
        }
        self.put(ctx, "⠻", Kind::Other);
        if comparison.is_some() {
            self.space_to(&ctx.level);
        }
    }

    fn accent(&mut self, accented: &Accent<'_>, ctx: &Ctx) {
        let Accent { base, accent, below } = *accented;
        let modifier = match accent {
            '\u{0304}' => "156",
            '\u{0302}' => "456 126",
            '\u{20D7}' | '→' => "1246 135",
            '\u{0307}' => "16",
            // two dots over: the dot twice, as in the compound ⩷ (Rule 21 table)
            '\u{0308}' => "16 16",
            // arcs concave downward and upward (Ex. 15-53 to 15-56)
            '⏜' => "1246 1",
            '⏝' => "1246 3",
            '\u{0303}' => "4 156",
            other => {
                self.node(base, ctx);
                self.missing(ctx, other);
                return;
            }
        };
        // x̄: a bar over a single letter or digit is contracted (§15.3)
        let single = matches!(base, Node::Identifier { name, .. } if name.chars().count() == 1) || matches!(base, Node::Number { value, .. } if value.len() == 1);
        if accent == '\u{0304}' && single {
            // x̄ = ⠭⠱ (§15.2.1); a bar under a single letter or digit: ⠭⠩⠱ (§15.2.3)
            self.node(base, ctx);
            self.put_dots(ctx, if below { "146 156" } else { "156" }, Kind::Other);
            return;
        }
        self.put(ctx, "⠐", Kind::Indicator);
        self.node(base, ctx);
        self.put(ctx, if below { "⠩" } else { "⠣" }, Kind::Other);
        self.put_dots(ctx, modifier, Kind::Other);
        self.put(ctx, "⠻", Kind::Other);
    }

    fn labeled_arrow(&mut self, arrow: &LabeledArrow<'_>, ctx: &Ctx) {
        let LabeledArrow { symbol, above, below } = *arrow;
        if above.is_none() && below.is_none() {
            self.symbol(symbol, ctx);
            return;
        }
        // a modified arrow takes its uncontracted form (§22)
        let dots = match symbol {
            '→' => "1246 25 25 135",
            '←' => "1246 246 25 25",
            '↔' => "1246 246 25 25 135",
            '⇒' => "1246 2356 2356 135",
            '⇔' => "1246 246 2356 2356 135",
            // the arrows compounded vertically (⇌) have no contracted form: their own sign
            other => match sign(other) {
                Some((dots, Kind::Comparison)) => dots,
                _ => {
                    self.missing(ctx, other);
                    return;
                }
            },
        };
        self.space_to("");
        self.put(ctx, "⠐", Kind::Comparison);
        self.put_dots(ctx, dots, Kind::Comparison);
        if let Some(b) = below {
            self.put(ctx, "⠩", Kind::Other);
            self.node(b, ctx);
        }
        if let Some(a) = above {
            self.put(ctx, "⠣", Kind::Other);
            self.node(a, ctx);
        }
        self.put(ctx, "⠻", Kind::Other);
        self.space_to("");
    }

    fn delimited(&mut self, delimited: &Delimited<'_>, ctx: &Ctx) {
        let Delimited { open, close, parts, middles } = *delimited;
        // an enclosed list: its items have no numeric or English-letter indicators (§3.5)
        let list = parts.iter().any(|p| {
            let c = p.children();
            c.iter().any(|c| matches!(c, Node::Operator { symbol: ',', .. })) && !c.iter().any(|c| matches!(c, Node::Operator { class: Class::Rel, .. }))
        });
        let inner = Ctx { list: ctx.list || list, ..ctx.clone() };
        // a table between \left and \right: the delimiters become enlarged grouping on every row (§19.6)
        if let ([Node::Table { rows, .. }], Some(o), Some(c)) = (parts, open, close) {
            if let (Some(o), Some(c)) = (enlarged(o), enlarged(c)) {
                self.table_rows(&RowGroup { rows, open: o, close: c }, ctx);
                return;
            }
        }
        if let Some(o) = open {
            self.symbol(o, ctx);
        }
        for (i, p) in parts.iter().enumerate() {
            if i > 0 {
                // \middle| between \left\{ and \right\}: the bar means "such that", a comparison (§21.7)
                let m = middles.get(i - 1).copied().unwrap_or('|');
                self.symbol(if m == '|' { '∣' } else { m }, &inner);
            }
            self.node(p, &inner);
        }
        if let Some(c) = close {
            self.symbol(c, ctx);
        }
    }

    /// Matrices, determinants and systems (§19.6, §25.8): a braille line per row, enlarged grouping signs
    /// on every line, one blank cell between entries, a numeric indicator on every number.
    fn table(&mut self, shape: TableKind, rows: &[Vec<Node>], ctx: &Ctx) {
        let (open, close) = match shape {
            TableKind::PMatrix => ("6 12356", "6 23456"),
            TableKind::BMatrix => ("4 6 12356", "4 6 23456"),
            TableKind::BraceMatrix => ("46 6 12356", "46 6 23456"),
            TableKind::VMatrix => ("6 1256", "6 1256"),
            TableKind::VVMatrix => ("6 1256 6 1256", "6 1256 6 1256"),
            TableKind::Cases => ("46 6 12356", ""),
            TableKind::RCases => ("", "46 6 23456"),
            _ => ("", ""),
        };
        self.table_rows(&RowGroup { rows, open, close }, ctx);
    }

    fn table_rows(&mut self, group: &RowGroup<'_>, ctx: &Ctx) {
        let RowGroup { rows, open, close } = *group;
        let inner = Ctx { table: true, list: false, ..ctx.clone() };
        // the first row continues the current line; the others start below it, aligned with it (Ex. 19-30)
        let indent: usize = self
            .items
            .iter()
            .rev()
            .take_while(|i| !matches!(i, Item::Newline))
            .map(|i| match i {
                Item::Cells(c, _) => c.chars().count(),
                _ => 1,
            })
            .sum();
        let mut first_end = None;
        for (r, row) in rows.iter().enumerate() {
            if r == 1 {
                first_end = Some(self.items.len());
            }
            if r > 0 {
                self.items.push(Item::Newline);
                if indent > 0 {
                    self.items.push(Item::Cells("⠀".repeat(indent), Kind::Indicator));
                }
            }
            self.level = ctx.level.clone();
            if !open.is_empty() {
                self.put_dots(ctx, open, Kind::Other);
            }
            for (j, cell) in row.iter().enumerate() {
                if j > 0 {
                    self.space_to(&ctx.level);
                }
                self.node(cell, &inner);
            }
            if !close.is_empty() {
                self.put_dots(ctx, close, Kind::Other);
            }
        }
        self.level = ctx.level.clone();
        self.first_row_end = first_end;
    }
}

fn is_function(n: &Node) -> bool {
    match n {
        Node::Function { .. } => true,
        Node::Scripts { base, .. } => matches!(**base, Node::Function { .. }),
        _ => false,
    }
}

/// Which children of a row lie inside an enclosed list (§3.5): grouping signs that hold commas at their
/// own depth, as in `[0, 1]` or `(a, 2x, b)`.
fn enclosed_lists(children: &[Node]) -> Vec<bool> {
    let mut inside = vec![false; children.len()];
    let class = |n: &Node| match n {
        Node::Operator { class, .. } => Some(*class),
        _ => None,
    };
    for open in 0..children.len() {
        if class(&children[open]) != Some(Class::Open) {
            continue;
        }
        let mut depth = 0;
        let mut comma = false;
        for close in open + 1..children.len() {
            match class(&children[close]) {
                Some(Class::Open) => depth += 1,
                Some(Class::Close) if depth > 0 => depth -= 1,
                Some(Class::Close) => {
                    if comma {
                        inside[open + 1..close].iter_mut().for_each(|b| *b = true);
                    }
                    break;
                }
                Some(Class::Punct) if depth == 0 && matches!(children[close], Node::Operator { symbol: ',', .. }) => comma = true,
                // a comparison sign inside makes it not an enclosed list (Ex. 6-30)
                Some(Class::Rel) if depth == 0 => break,
                _ => {}
            }
        }
    }
    inside
}

/// How deep the fractions inside go: 0 for a simple fraction's parts.
fn fraction_depth(n: &Node) -> usize {
    match n {
        Node::Fraction { num, den, rule: true, .. } => 1 + fraction_depth(num).max(fraction_depth(den)),
        Node::Row { children } => children.iter().map(fraction_depth).max().unwrap_or(0),
        Node::Delimited { parts, .. } => parts.iter().map(fraction_depth).max().unwrap_or(0),
        Node::Root { radicand, .. } => fraction_depth(radicand),
        // a fraction at script level does not make the outer one complex (§13.5)
        Node::Scripts { base, .. } => fraction_depth(base),
        Node::Colored { base, .. } | Node::ForcedClass { base, .. } | Node::Style { base, .. } | Node::Framed { base, .. } | Node::Unit { base, .. } => fraction_depth(base),
        _ => 0,
    }
}

/// The primes at the start of a superscript, and what remains of it.
fn split_primes(sup: Option<&Node>) -> (usize, Option<Node>) {
    let count = |n: &Node| match n {
        Node::Operator { symbol: '′', .. } => 1,
        Node::Operator { symbol: '″', .. } => 2,
        Node::Operator { symbol: '‴', .. } => 3,
        _ => 0,
    };
    match sup {
        Some(n) if count(n) > 0 => (count(n), None),
        Some(Node::Row { children }) if children.first().is_some_and(|c| count(c) > 0) => {
            let k = children.iter().take_while(|c| count(c) > 0).count();
            let primes = children[..k].iter().map(count).sum();
            let rest = &children[k..];
            (primes, if rest.is_empty() { None } else { Some(Node::row(rest.to_vec()).simplify()) })
        }
        other => (0, other.cloned()),
    }
}
