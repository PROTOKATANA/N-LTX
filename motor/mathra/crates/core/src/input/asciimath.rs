//! Reader for AsciiMath (asciimath.org): `sum_(i=1)^n i^2 = (n(n+1)(2n+1))/6`. It builds the same tree of
//! meaning as the LaTeX reader. Symbols are given by their LaTeX spelling and read by the LaTeX reader,
//! so both notations draw and speak a symbol the same way.
//!
//! The grammar follows asciimath.org: an expression is a sequence of `I` or `I/I`; `I` is `S`, `S_S`,
//! `S^S` or `S_S^S`; `S` is a symbol, a bracketed expression, `unary S` or `binary S S`. Brackets around
//! a fraction's parts, a script or an argument are removed: `(a+b)/2` has no parentheses on top.

use crate::locale::Locale;
use crate::rules::{self, Registry};
use crate::tree::*;
use crate::Error;

use super::latex::{self, NESTING_LIMIT};
use super::symbols;

mod token;
use token::tokenize;

type R<T> = Result<T, Error>;

pub fn read_with(source: &str, locale: Locale, registry: &mut Registry, strict: bool) -> R<Node> {
    let tokens = tokenize(source, locale);
    let mut parser = Parser { tokens, pos: 0, locale, registry, strict, depth: 0 };
    let mut children = Vec::new();
    loop {
        children.extend(parser.expression(false)?);
        match parser.peek() {
            None => break,
            Some(Token { kind: Kind::Right(c), col, .. }) => {
                // a closing bracket nothing opened
                let (c, col) = (*c, *col);
                parser.pos += 1;
                parser.tolerate(rules::E_REC_5, col, format!("'{}' closes nothing", c.map_or(":)".into(), String::from)), "drawn as is")?;
                if let Some(c) = c {
                    children.push(symbol_node(c));
                }
            }
            Some(_) => unreachable!("an expression stops only at the end or at a closing bracket"),
        }
    }
    Ok(Node::row(children))
}

#[derive(Debug, Clone, PartialEq)]
enum Kind {
    /// A symbol, with its LaTeX spelling.
    Symbol(&'static str),
    Number(String),
    Letter(char),
    Text(String),
    /// `color(red)`, with the raw color.
    Color(String),
    Unary(&'static str),
    Binary(&'static str),
    /// Opening bracket; `None` is the invisible `{:`.
    Left(Option<char>),
    /// Closing bracket; `None` is the invisible `:}`.
    Right(Option<char>),
    /// `|` and `||`: opening or closing, decided by what follows.
    Bar(char),
    Over,
    Sub,
    Sup,
}

#[derive(Debug, Clone)]
struct Token {
    kind: Kind,
    col: usize,
}

struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    locale: Locale,
    registry: &'a mut Registry,
    strict: bool,
    depth: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn tolerate(&mut self, rule: &'static str, column: usize, problem: impl Into<String>, recovery: &str) -> R<()> {
        let problem = problem.into();
        if self.strict {
            return Err(Error::Syntax { column, message: problem });
        }
        self.registry.warn(rule, format!("column {column}: {problem}; {recovery}"));
        Ok(())
    }

    fn nested<T>(&mut self, col: usize, f: impl FnOnce(&mut Self) -> R<T>) -> R<T> {
        if self.depth >= NESTING_LIMIT {
            return Err(Error::Syntax { column: col, message: format!("nesting deeper than {NESTING_LIMIT} levels") });
        }
        self.depth += 1;
        let r = f(self);
        self.depth -= 1;
        r
    }

    /// `E ::= I E | I/I E`, up to the end, a closing bracket or (inside `|…|`) the closing bar.
    fn expression(&mut self, inside_bars: bool) -> R<Vec<Node>> {
        let mut out = Vec::new();
        loop {
            match self.peek().map(|t| &t.kind) {
                None | Some(Kind::Right(_)) => return Ok(out),
                Some(Kind::Bar(_)) if inside_bars => return Ok(out),
                _ => {}
            }
            let left = self.intermediate()?;
            if matches!(self.peek().map(|t| &t.kind), Some(Kind::Over)) {
                self.pos += 1;
                let right = if self.peek().is_some_and(|t| !matches!(t.kind, Kind::Right(_))) { self.intermediate()? } else { Node::row(vec![]) };
                out.push(Node::Fraction { num: Box::new(unbracket(left)), den: Box::new(unbracket(right)), style: FractionStyle::Auto, rule: true, thickness: None });
            } else {
                out.push(left);
            }
        }
    }

    /// `I ::= S | S_S | S^S | S_S^S`.
    fn intermediate(&mut self) -> R<Node> {
        let base = self.simple()?;
        let mut sub = None;
        let mut sup = None;
        // f'(x): primes belong to what precedes them, as in LaTeX
        let mut primes = 0;
        while matches!(self.peek().map(|t| &t.kind), Some(Kind::Symbol("'"))) {
            self.pos += 1;
            primes += 1;
        }
        if primes > 0 {
            sup = Some(Node::Operator { symbol: ['′', '″', '‴'][primes.min(3) - 1], class: Class::Ord });
        }
        loop {
            match self.peek().map(|t| &t.kind) {
                Some(Kind::Sub) if sub.is_none() => {
                    self.pos += 1;
                    sub = Some(unbracket(self.simple()?));
                }
                Some(Kind::Sup) if sup.is_none() || primes > 0 && sup.as_ref().is_some_and(|s| matches!(s, Node::Operator { .. })) => {
                    self.pos += 1;
                    let arg = unbracket(self.simple()?);
                    // f'^2: the prime and then the exponent, as LaTeX reads it
                    sup = Some(match sup {
                        Some(prime) => Node::row(vec![prime, arg]),
                        None => arg,
                    });
                }
                _ => break,
            }
        }
        if sub.is_none() && sup.is_none() {
            return Ok(base);
        }
        // ubrace(a+b)_n: the script is the brace's label
        if let Node::Brace { base: b, above, label: None, bracket } = &base {
            let label = if *above { sup.clone() } else { sub.clone() };
            if let Some(label) = label {
                return Ok(Node::Brace { base: b.clone(), above: *above, label: Some(Box::new(label)), bracket: *bracket });
            }
        }
        Ok(Node::Scripts { base: Box::new(base), sup: sup.map(Box::new), sub: sub.map(Box::new) })
    }

    /// `S`: one symbol, a bracketed expression, `unary S` or `binary S S`.
    fn simple(&mut self) -> R<Node> {
        let Some(token) = self.peek().cloned() else {
            return Ok(Node::row(vec![]));
        };
        self.nested(token.col, |this| this.simple_unguarded(token))
    }

    fn simple_unguarded(&mut self, token: Token) -> R<Node> {
        let col = token.col;
        self.pos += 1;
        Ok(match token.kind {
            Kind::Symbol(tex) => latex_symbol(tex, self.locale),
            Kind::Number(value) => {
                if value.contains('.') && self.locale.comma_is_decimal() {
                    self.registry.note(rules::L_NUM_1, format!("'{value}': read as a decimal number"));
                }
                Node::Number { value, group: false, thousands: None }
            }
            Kind::Letter(c) if c.is_alphabetic() => {
                let variant = if c.is_ascii_alphabetic() || ('α'..='ω').contains(&c) { Variant::Italic } else { Variant::Upright };
                Node::Identifier { name: c.to_string(), variant }
            }
            Kind::Letter(c) => symbol_node(c),
            Kind::Text(t) => Node::Text { parts: vec![TextPart::Text(t)] },
            Kind::Color(color) => {
                let base = self.simple()?;
                let valid = !color.is_empty() && color.len() < 40 && color.chars().all(|x| x.is_ascii_alphanumeric() || "#(),. ".contains(x));
                if valid {
                    Node::Colored { color: color.trim().to_string(), base: Box::new(unbracket(base)) }
                } else {
                    self.tolerate(rules::E_REC_9, col, format!("invalid color: {color}"), "drawn without color")?;
                    unbracket(base)
                }
            }
            Kind::Unary(name) => {
                let arg = self.simple()?;
                unary(name, arg)
            }
            Kind::Binary(name) => {
                let a = unbracket(self.simple()?);
                let b = unbracket(self.simple()?);
                match name {
                    "frac" => Node::Fraction { num: Box::new(a), den: Box::new(b), style: FractionStyle::Auto, rule: true, thickness: None },
                    "root" => Node::Root { radicand: Box::new(b), index: Some(Box::new(a)) },
                    "underset" => {
                        let class = class_of(&b);
                        Node::ForcedClass { class, base: Box::new(Node::Stacked { base: Box::new(b), above: None, below: Some(Box::new(a)) }) }
                    }
                    _ => {
                        let class = if name == "stackrel" { Class::Rel } else { class_of(&b) };
                        Node::ForcedClass { class, base: Box::new(Node::Stacked { base: Box::new(b), above: Some(Box::new(a)), below: None }) }
                    }
                }
            }
            Kind::Left(open) => {
                let inner = self.expression(false)?;
                match self.peek().map(|t| t.kind.clone()) {
                    Some(Kind::Right(close)) => {
                        self.pos += 1;
                        bracketed(open, close, inner)
                    }
                    _ => {
                        self.tolerate(rules::E_REC_5, col, format!("'{}' without a closing bracket", open.map_or("{:".into(), String::from)), "drawn as is")?;
                        let mut children: Vec<Node> = open.map(symbol_node).into_iter().collect();
                        children.extend(inner);
                        Node::row(children)
                    }
                }
            }
            Kind::Bar(bar) => {
                // |x| when a matching bar follows; otherwise a lone bar
                let saved = self.pos;
                let inner = self.expression(true)?;
                if matches!(self.peek().map(|t| &t.kind), Some(Kind::Bar(b)) if *b == bar) && !inner.is_empty() {
                    self.pos += 1;
                    Node::Delimited { open: Some(bar), close: Some(bar), parts: vec![Node::row(inner)], middles: vec![] }
                } else {
                    self.pos = saved;
                    Node::Operator { symbol: bar, class: Class::Ord }
                }
            }
            Kind::Sub | Kind::Sup => {
                // a script with no base: on an empty base, like {}_a in LaTeX
                let script = unbracket(self.simple()?);
                let (sup, sub) = if token.kind == Kind::Sup { (Some(script), None) } else { (None, Some(script)) };
                Node::Scripts { base: Box::new(Node::row(vec![])), sup: sup.map(Box::new), sub: sub.map(Box::new) }
            }
            Kind::Over => Node::Operator { symbol: '/', class: Class::Ord },
            Kind::Right(_) => unreachable!("expression() stops before a closing bracket"),
        })
    }
}

/// A symbol given by its LaTeX spelling, read by the LaTeX reader.
fn latex_symbol(tex: &str, locale: Locale) -> Node {
    let mut scratch = Registry::default();
    latex::read_with(tex, locale, &mut scratch, true).map(Node::simplify).expect("every AsciiMath symbol is valid LaTeX")
}

fn symbol_node(c: char) -> Node {
    match symbols::character(c) {
        Some(s) => Node::Operator { symbol: s.symbol, class: s.class },
        None => Node::Operator { symbol: c, class: symbols::class_of_char(c).unwrap_or(Class::Ord) },
    }
}

fn class_of(n: &Node) -> Class {
    match n {
        Node::Operator { class, .. } => *class,
        Node::LargeOperator { .. } | Node::Function { .. } => Class::Op,
        _ => Class::Ord,
    }
}

/// Without the outer brackets: `(a+b)/2` has no parentheses on top, `x^(2n)` neither.
fn unbracket(n: Node) -> Node {
    match n {
        Node::Delimited { open: Some('(' | '[' | '{') | None, close: Some(')' | ']' | '}') | None, mut parts, middles } if parts.len() == 1 && middles.is_empty() => {
            parts.pop().expect("one part").simplify()
        }
        other => other,
    }
}

fn unary(name: &str, arg: Node) -> Node {
    let delimited = |o: char, c: char, arg: Node| Node::Delimited { open: Some(o), close: Some(c), parts: vec![unbracket(arg)], middles: vec![] };
    let accent = |accent: char, stretchy: bool, arg: Node| Node::Accent { base: Box::new(unbracket(arg)), accent, stretchy, below: false };
    let wide = |arg: &Node| !matches!(unbracket(arg.clone()), Node::Identifier { .. } | Node::Number { .. });
    match name {
        "sqrt" => Node::Root { radicand: Box::new(unbracket(arg)), index: None },
        "abs" => delimited('|', '|', arg),
        "norm" => delimited('‖', '‖', arg),
        "floor" => delimited('⌊', '⌋', arg),
        "ceil" => delimited('⌈', '⌉', arg),
        "hat" => {
            let w = wide(&arg);
            accent('\u{0302}', w, arg)
        }
        "tilde" => {
            let w = wide(&arg);
            accent('\u{0303}', w, arg)
        }
        "vec" => {
            if wide(&arg) {
                accent('→', true, arg)
            } else {
                accent('\u{20D7}', false, arg)
            }
        }
        "dot" => accent('\u{0307}', false, arg),
        "ddot" => accent('\u{0308}', false, arg),
        "overarc" | "overparen" => accent('⏜', true, arg),
        "underparen" => Node::Accent { base: Box::new(unbracket(arg)), accent: '⏝', stretchy: true, below: true },
        "bar" | "overline" => Node::Bar { base: Box::new(unbracket(arg)), above: true },
        "ul" | "underline" => Node::Bar { base: Box::new(unbracket(arg)), above: false },
        "ubrace" | "underbrace" => Node::Brace { base: Box::new(unbracket(arg)), above: false, label: None, bracket: false },
        "obrace" | "overbrace" => Node::Brace { base: Box::new(unbracket(arg)), above: true, label: None, bracket: false },
        "cancel" => Node::Canceled { base: Box::new(unbracket(arg)), shape: Cancel::Bar },
        font => {
            let variant = match font {
                "bb" | "mathbf" => Variant::Bold,
                "bbb" | "mathbb" => Variant::DoubleStruck,
                "cc" | "mathcal" => Variant::Calligraphic,
                "tt" | "mathtt" => Variant::Monospace,
                "fr" | "mathfrak" => Variant::Fraktur,
                "sf" | "mathsf" => Variant::SansSerif,
                _ => Variant::Upright,
            };
            with_variant(unbracket(arg), variant)
        }
    }
}

/// The letters of `node` in another alphabet (`bb`, `bbb`, `cc`…); quoted text too (`bb "AaBb"`).
fn with_variant(node: Node, variant: Variant) -> Node {
    match node {
        Node::Identifier { name, .. } => Node::Identifier { name, variant },
        Node::Text { parts } if matches!(parts.as_slice(), [TextPart::Text(_)]) => match parts.into_iter().next() {
            Some(TextPart::Text(t)) => Node::Identifier { name: t, variant },
            _ => unreachable!("one text part"),
        },
        Node::Row { children } => Node::Row { children: children.into_iter().map(|c| with_variant(c, variant)).collect() },
        Node::Scripts { base, sup, sub } => Node::Scripts { base: Box::new(with_variant(*base, variant)), sup, sub },
        other => other,
    }
}

/// A bracketed expression: a matrix when it holds bracketed rows of equal length separated by commas
/// (`[[a,b],[c,d]]`), `cases` with `{` and the invisible `:}`, or delimiters that grow with the content.
fn bracketed(open: Option<char>, close: Option<char>, inner: Vec<Node>) -> Node {
    if let Some(rows) = matrix_rows(&inner) {
        let shape = match (open, close) {
            (Some('('), Some(')')) => Some(TableKind::PMatrix),
            (Some('['), Some(']')) => Some(TableKind::BMatrix),
            (Some('{'), Some('}')) => Some(TableKind::BraceMatrix),
            (Some('{'), None) => Some(TableKind::Cases),
            (None, Some('}')) => Some(TableKind::RCases),
            (None, None) => Some(TableKind::Matrix),
            _ => None,
        };
        if let Some(shape) = shape {
            let (rows, verticals) = augmented(rows);
            return Node::Table { shape, rows, columns: vec![], verticals, dashed: vec![], horizontals: vec![], between: vec![], intertexts: vec![], stretch: 1.0 };
        }
    }
    if open.is_none() && close.is_none() {
        return Node::row(inner);
    }
    Node::Delimited { open, close, parts: vec![Node::row(inner)], middles: vec![] }
}

/// An augmented matrix (`[[a,b,|,c],[d,e,|,f]]`): a column of bars in every row becomes a vertical rule.
fn augmented(rows: Vec<Vec<Node>>) -> (Vec<Vec<Node>>, Vec<usize>) {
    let bar = |n: &Node| matches!(n, Node::Operator { symbol: '|', .. });
    let ncols = rows.first().map_or(0, Vec::len);
    let bars: Vec<usize> = (0..ncols).filter(|&j| rows.iter().all(|r| r.get(j).is_some_and(bar))).collect();
    if bars.is_empty() {
        return (rows, vec![]);
    }
    let rows = rows.into_iter().map(|r| r.into_iter().enumerate().filter(|(j, _)| !bars.contains(j)).map(|(_, c)| c).collect()).collect();
    // the rule goes before the column that followed the bar, counted without the bar columns
    let verticals = bars.iter().enumerate().map(|(k, j)| j - k).collect();
    (rows, verticals)
}

fn matrix_rows(inner: &[Node]) -> Option<Vec<Vec<Node>>> {
    let comma = |n: &Node| matches!(n, Node::Operator { symbol: ',', .. });
    let mut rows: Vec<Vec<Node>> = Vec::new();
    let mut expect_row = true;
    let mut bracket: Option<(Option<char>, Option<char>)> = None;
    for n in inner {
        if expect_row {
            let Node::Delimited { open, close, parts, middles } = n else { return None };
            if !middles.is_empty() || parts.len() != 1 || !matches!(open, Some('(' | '[')) {
                return None;
            }
            // every row with the same brackets
            if bracket.get_or_insert((*open, *close)) != &(*open, *close) {
                return None;
            }
            let cells: Vec<Node> = parts[0].children().split(comma).map(|c| Node::row(c.to_vec()).simplify()).collect();
            rows.push(cells);
            expect_row = false;
        } else if comma(n) {
            expect_row = true;
        } else {
            return None;
        }
    }
    let ncols = rows.first()?.len();
    (!expect_row && rows.len() >= 2 && rows.iter().all(|r| r.len() == ncols)).then_some(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every symbol is read by the LaTeX reader in strict mode: none may fail at conversion time.
    #[test]
    fn every_symbol_is_valid_latex() {
        for (am, tex) in super::token::SYMBOLS {
            let mut registry = Registry::default();
            let r = latex::read_with(tex, Locale::En, &mut registry, true);
            assert!(r.is_ok(), "{am} → {tex}: {r:?}");
        }
    }
}
