//! Readable linear text (close to UnicodeMath): goes into the SVG's selection layer and `--text`.

use crate::layout::classes;
use crate::locale::Locale;
use crate::tree::*;

pub fn linear(node: &Node, loc: Locale) -> String {
    let mut s = String::new();
    TextWriter { loc, out: &mut s }.write_node(node);
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn alone(node: &Node) -> bool {
    match node {
        Node::Number { .. } | Node::Identifier { .. } | Node::Operator { .. } => true,
        Node::Row { children } => children.len() == 1 && alone(&children[0]),
        Node::Text { parts } => parts.len() == 1,
        Node::Unit { base, .. } => alone(base),
        _ => false,
    }
}

/// What a pair of fences encloses, and the fences between its parts.
struct Delimited<'a> {
    open: Option<char>,
    close: Option<char>,
    parts: &'a [Node],
    middles: &'a [char],
}

/// Locale and the buffer `linear` is writing. They travel together through every piece.
struct TextWriter<'a> {
    loc: Locale,
    out: &'a mut String,
}

impl TextWriter<'_> {
    /// In parentheses when the piece has more than one atom.
    fn group(&mut self, node: &Node) {
        if alone(node) {
            self.write_node(node);
        } else {
            self.out.push('(');
            self.write_node(node);
            self.out.push(')');
        }
    }

    fn write_node(&mut self, node: &Node) {
        match node {
            Node::Number { value, thousands, .. } => self.out.push_str(&super::written_number(value, *thousands, self.loc)),
            Node::Identifier { name, variant } => {
                let mapped: String = name.chars().map(|c| crate::layout::alphabet::character(c, *variant)).collect();
                self.out.push_str(if matches!(variant, Variant::Italic | Variant::Upright) { name } else { &mapped });
            }
            Node::Function { name, .. } => {
                self.out.push_str(&self.loc.function_name(name));
                self.out.push(' ');
            }
            Node::Operator { symbol, .. } | Node::FixedDelimiter { symbol, .. } => self.out.push(*symbol),
            Node::LargeOperator { symbol, .. } => {
                self.out.push(*symbol);
                self.out.push(' ');
            }
            Node::Row { children } => self.write_row(children),
            Node::Fraction { num, den, rule, .. } => self.write_fraction(num, den, *rule),
            Node::Root { radicand, index } => self.write_root(radicand, index),
            Node::Scripts { base, sup, sub } => self.write_scripts(base, sup, sub),
            Node::Delimited { open, close, parts, middles } => self.write_delimited(&Delimited { open: *open, close: *close, parts, middles }),
            Node::Table { shape: kind, rows, .. } => self.write_table(*kind, rows),
            Node::Accent { base, accent, .. } => {
                self.group(base);
                if matches!(*accent as u32, 0x0300..=0x036F | 0x20D0..=0x20FF) {
                    self.out.push(*accent);
                }
            }
            Node::Bar { base, above } => {
                if *above {
                    self.out.push('‾');
                }
                self.group(base);
            }
            Node::Brace { base, label, .. } => {
                self.write_node(base);
                if let Some(r) = label {
                    self.out.push_str(" [");
                    self.write_node(r);
                    self.out.push(']');
                }
            }
            Node::Text { parts } => {
                for p in parts {
                    match p {
                        TextPart::Text(t) => self.out.push_str(&t.replace(crate::input::latex::MARKERS, "")),
                        TextPart::Math(m) => self.write_node(m),
                    }
                }
            }
            Node::Space { em } => {
                if *em > 0.1 {
                    self.out.push(' ');
                }
            }
            Node::Phantom { .. } => {}
            // \mathchoice: the text style's branch
            Node::Choice { branches } => {
                if let Some(b) = branches.get(1) {
                    self.write_node(b);
                }
            }
            Node::Canceled { base, .. } | Node::Colored { base, .. } | Node::ForcedClass { base, .. } | Node::Style { base, .. } | Node::Unit { base, .. } => self.write_node(base),
            Node::Framed { base, shape, .. } => self.write_framed(base, *shape),
            Node::Quantity { unit, .. } => self.write_node(unit),
            Node::Arrow { symbol, above, .. } => {
                self.out.push(' ');
                self.out.push(*symbol);
                if let Some(a) = above {
                    self.out.push('[');
                    self.write_node(a);
                    self.out.push(']');
                }
                self.out.push(' ');
            }
            Node::Stacked { base, .. }
            | Node::Multicolumn { base, .. }
            | Node::Raised { base, .. }
            | Node::Scaled { base, .. }
            | Node::Smashed { base, .. }
            | Node::VCenter { base }
            | Node::Resized { base, .. }
            | Node::Rotated { base, .. }
            | Node::Reflected { base }
            | Node::Overlapped { base, .. } => self.write_node(base),
            Node::Rule { .. } => {}
            Node::VerticalArrow { symbol, .. } => {
                self.out.push(' ');
                self.out.push(*symbol);
                self.out.push(' ');
            }
            #[cfg(feature = "chem")]
            Node::Chemistry { scheme } => self.out.push_str(&crate::chem::speech::linear(scheme, self.loc)),
            Node::ArrowDiagram { cells, arrows } => self.write_arrow_diagram(cells, arrows),
            Node::LongDivision { dividend, divisor } => self.write_long_division(dividend, divisor),
            Node::ColumnArithmetic { operation, terms } => self.write_column_arithmetic(*operation, terms),
        }
    }

    fn write_row(&mut self, children: &[Node]) {
        let cl = classes(children, self.loc, None, false);
        for (child, class) in children.iter().zip(cl) {
            match (child, class) {
                (Node::Operator { symbol, .. }, Some(Class::Bin | Class::Rel)) => {
                    self.out.push(' ');
                    self.out.push(*symbol);
                    self.out.push(' ');
                }
                (Node::Operator { symbol, .. }, Some(Class::Punct)) => {
                    self.out.push(*symbol);
                    self.out.push(' ');
                }
                (other, _) => self.write_node(other),
            }
        }
    }

    fn write_fraction(&mut self, num: &Node, den: &Node, rule: bool) {
        if rule {
            self.group(num);
            self.out.push('/');
            self.group(den);
        } else {
            self.out.push('(');
            self.write_node(num);
            self.out.push_str(" ¦ ");
            self.write_node(den);
            self.out.push(')');
        }
    }

    fn write_root(&mut self, radicand: &Node, index: &Option<Box<Node>>) {
        match index.as_deref() {
            None => self.out.push('√'),
            Some(Node::Number { value, .. }) if value == "3" => self.out.push('∛'),
            Some(Node::Number { value, .. }) if value == "4" => self.out.push('∜'),
            Some(i) => {
                self.out.push_str("√[");
                self.write_node(i);
                self.out.push(']');
            }
        }
        self.group(radicand);
    }

    /// sin²x, log₂x, ∑_(i = 1)^n i: the scripts stick to the function name or the large operator,
    /// and the space goes after them.
    fn write_scripts(&mut self, base: &Node, sup: &Option<Box<Node>>, sub: &Option<Box<Node>>) {
        let spaced = matches!(base, Node::Function { .. } | Node::LargeOperator { .. });
        self.write_script_base(base);
        if let Some(b) = sub {
            self.out.push('_');
            self.group(b);
        }
        if let Some(p) = sup {
            self.write_superscript(p);
        }
        if spaced {
            self.out.push(' ');
        }
    }

    fn write_script_base(&mut self, base: &Node) {
        match base {
            Node::Function { name, .. } => self.out.push_str(&self.loc.function_name(name)),
            Node::LargeOperator { symbol, .. } => self.out.push(*symbol),
            other => self.write_node(other),
        }
    }

    fn write_superscript(&mut self, sup: &Node) {
        let prime = |n: &Node| matches!(n, Node::Operator { symbol: '′' | '″' | '‴', .. });
        match sup {
            n if prime(n) => self.write_node(n),
            Node::Row { children } if children.first().is_some_and(prime) => {
                let qty = children.iter().take_while(|f| prime(f)).count();
                for f in &children[..qty] {
                    self.write_node(f);
                }
                self.out.push('^');
                self.group(&Node::row(children[qty..].to_vec()).simplify());
            }
            other => {
                self.out.push('^');
                self.group(other);
            }
        }
    }

    fn write_delimited(&mut self, delimited: &Delimited<'_>) {
        if let Some(a) = delimited.open {
            self.out.push(a);
        }
        // \binom is a fraction without a rule inside parentheses: (n ¦ k), not ((n ¦ k))
        if self.write_binomial(delimited) {
            return;
        }
        for (i, p) in delimited.parts.iter().enumerate() {
            if i > 0 {
                self.out.push(' ');
                self.out.push(delimited.middles.get(i - 1).copied().unwrap_or('|'));
                self.out.push(' ');
            }
            self.write_node(p);
        }
        if let Some(f) = delimited.close {
            self.out.push(f);
        }
    }

    /// The opening fence is already written. Returns whether this was a binomial, closing fence included.
    fn write_binomial(&mut self, delimited: &Delimited<'_>) -> bool {
        let [p] = delimited.parts else {
            return false;
        };
        let Node::Fraction { num, den, rule: false, .. } = p.single() else {
            return false;
        };
        self.write_node(num);
        self.out.push_str(" ¦ ");
        self.write_node(den);
        if let Some(f) = delimited.close {
            self.out.push(f);
        }
        true
    }

    fn write_table(&mut self, kind: TableKind, rows: &[Vec<Node>]) {
        match kind {
            TableKind::Aligned | TableKind::Centered | TableKind::Cases | TableKind::RCases | TableKind::Array | TableKind::Substack | TableKind::Diagram => {
                self.write_row_list(kind, rows);
            }
            _ => self.write_matrix(kind, rows),
        }
    }

    fn write_row_list(&mut self, kind: TableKind, rows: &[Vec<Node>]) {
        let loc = self.loc;
        let join_rows = if kind == TableKind::Aligned { " " } else { ", " };
        let lines: Vec<String> = rows.iter().map(|line| line.iter().map(|c| linear(c, loc)).collect::<Vec<_>>().join(join_rows)).collect();
        if kind == TableKind::Cases {
            self.out.push('{');
        }
        self.out.push_str(&lines.join("; "));
        if kind == TableKind::RCases {
            self.out.push('}');
        }
    }

    fn write_matrix(&mut self, kind: TableKind, rows: &[Vec<Node>]) {
        let loc = self.loc;
        let lines: Vec<String> = rows.iter().map(|line| format!("[{}]", line.iter().map(|c| linear(c, loc)).collect::<Vec<_>>().join(", "))).collect();
        let (open, close) = match kind {
            TableKind::PMatrix => ("(", ")"),
            TableKind::VMatrix => ("|", "|"),
            _ => ("[", "]"),
        };
        self.out.push_str(open);
        self.out.push_str(&lines.join(", "));
        self.out.push_str(close);
    }

    fn write_framed(&mut self, base: &Node, shape: Frame) {
        match shape {
            Frame::Circle => {
                self.out.push('(');
                self.write_node(base);
                self.out.push(')');
            }
            // A∠θ, as the phasor is written in plain text; the annuity's a_(n⌉)
            Frame::Phase => {
                self.out.push('∠');
                self.write_node(base);
            }
            Frame::ActuarialAngle => {
                self.write_node(base);
                self.out.push('⌉');
            }
            _ => self.write_node(base),
        }
    }

    fn write_arrow_diagram(&mut self, cells: &[Vec<Node>], arrows: &[DiagramArrow]) {
        let loc = self.loc;
        let rows: Vec<String> = cells.iter().map(|line| line.iter().map(|c| linear(c, loc)).collect::<Vec<_>>().join(", ")).collect();
        self.out.push_str(&rows.join("; "));
        for arrow in arrows {
            self.out.push_str("; ");
            self.out.push_str(linear(&arrow.as_row(cells), loc).trim());
        }
    }

    fn write_long_division(&mut self, dividend: &str, divisor: &str) {
        let (q, r) = match (dividend.parse::<u128>(), divisor.parse::<u128>()) {
            (Ok(a), Ok(b)) if b > 0 => (a / b, a % b),
            _ => (0, 0),
        };
        let remainder = self.loc.phrase("remainder");
        self.out.push_str(&format!("{dividend} ÷ {divisor} = {q} ({remainder} {r})"));
    }

    fn write_column_arithmetic(&mut self, operation: char, terms: &[String]) {
        let a: i128 = terms[0].parse().unwrap_or(0);
        let b: i128 = terms[1].parse().unwrap_or(0);
        let result = match operation {
            '+' => a + b,
            '−' => a - b,
            _ => a.saturating_mul(b),
        };
        self.out.push_str(&format!("{a} {operation} {b} = {result}"));
    }
}
