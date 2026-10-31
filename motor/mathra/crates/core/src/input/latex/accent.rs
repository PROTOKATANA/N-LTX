//! Accents, stretchable arrows, cancellation and sized delimiters.

use super::*;

impl Reader<'_> {
    pub(super) fn accent_command(&mut self, call: CommandCall<'_>) -> R<Option<Option<Stop>>> {
        let node: Node = match call.name {
            // accents, bars, braces
            "overline" | "underline" => Node::Bar { base: Box::new(self.argument()?), above: call.name == "overline" },
            "overbrace" | "underbrace" => Node::Brace { base: Box::new(self.argument()?), above: call.name == "overbrace", label: None, bracket: false },
            "overbracket" | "underbracket" => {
                // mathtools: [rule thickness][bracket height], which the font's bracket already has
                self.raw_optional();
                self.raw_optional();
                Node::Brace { base: Box::new(self.argument()?), above: call.name == "overbracket", label: None, bracket: true }
            }
            "underrightarrow" | "underleftarrow" | "underleftrightarrow" | "utilde" => {
                Node::Accent { base: Box::new(self.argument()?), accent: under_mark(call.name), stretchy: true, below: true }
            }
            "overset" | "stackrel" => {
                let above = self.argument()?;
                let base = self.base_with_class()?;
                Node::ForcedClass {
                    class: if call.name == "stackrel" { Class::Rel } else { natural_class(&base) },
                    base: Box::new(Node::Stacked { base: Box::new(base), above: Some(Box::new(above)), below: None }),
                }
            }
            "underset" => {
                let below = self.argument()?;
                let base = self.base_with_class()?;
                let class = natural_class(&base);
                Node::ForcedClass { class, base: Box::new(Node::Stacked { base: Box::new(base), above: None, below: Some(Box::new(below)) }) }
            }
            "xrightarrow" | "xleftarrow" | "xRightarrow" | "xLeftarrow" | "xleftrightarrow" | "xLeftrightarrow" | "xrightleftharpoons" | "xleftrightharpoons" | "xmapsto"
            | "xlongequal" | "xhookrightarrow" | "xhookleftarrow" | "xtwoheadrightarrow" | "xtwoheadleftarrow" | "xrightharpoonup" | "xrightharpoondown" | "xleftharpoonup"
            | "xleftharpoondown" | "xtofrom" => self.extended_arrow(extended_arrow_symbol(call.name))?,
            "cancel" | "bcancel" | "xcancel" | "sout" => {
                let kind = match call.name {
                    "bcancel" => Cancel::Backslash,
                    "xcancel" => Cancel::X,
                    _ => Cancel::Bar,
                };
                Node::Canceled { base: Box::new(self.argument()?), shape: kind }
            }
            "cancelto" => {
                let value = self.argument()?;
                let base = self.argument()?;
                Node::Scripts { base: Box::new(Node::Canceled { base: Box::new(base), shape: Cancel::Bar }), sup: Some(Box::new(value)), sub: None }
            }
            "boxed" => Node::Framed { base: Box::new(self.argument()?), shape: Frame::Rectangle, background: None, border: None },
            // steinmetz: \phase[style]{angle}, the angle in text style unless [0] to [3] say otherwise
            // (display, text, script, scriptscript); any other option leaves the current style, as
            // the package's \ifcase does
            "phase" => self.phase_angle()?,
            // actuarialangle: \angl{n}, and \angln, \anglr, \anglk for the common durations
            "angl" | "angln" | "anglr" | "anglk" => {
                let base = match call.name {
                    "angl" => self.argument()?,
                    _ => Node::Identifier { name: call.name[4..].to_string(), variant: Variant::Italic },
                };
                Node::Framed { base: Box::new(base), shape: Frame::ActuarialAngle, background: None, border: None }
            }
            "textcircled" | "circled" => Node::Framed { base: Box::new(self.argument()?), shape: Frame::Circle, background: None, border: None },
            "phantom" | "hphantom" | "vphantom" => Node::Phantom { base: Box::new(self.argument()?), width: call.name != "vphantom", height: call.name != "hphantom" },
            "textcolor" => {
                let color = self.color()?;
                let base = self.argument()?;
                match color {
                    Some(color) => Node::Colored { color, base: Box::new(base) },
                    None => base,
                }
            }
            // the color is the background, not the content's: the text keeps the page color
            "colorbox" => {
                let background = self.color()?;
                Node::Framed { base: Box::new(self.text()?), shape: Frame::Fill, background, border: None }
            }
            "fcolorbox" => {
                let border = self.color()?;
                let background = self.color()?;
                Node::Framed { base: Box::new(self.text()?), shape: Frame::Rectangle, background, border }
            }
            // \centernot (centernot package) is drawn like \not
            "not" | "centernot" => self.negated_operator(call.column)?,
            "big" | "Big" | "bigg" | "Bigg" | "bigl" | "Bigl" | "biggl" | "Biggl" | "bigr" | "Bigr" | "biggr" | "Biggr" | "bigm" | "Bigm" | "biggm" | "Biggm" => {
                match self.sized_delimiter(call.name)? {
                    Some(node) => node,
                    None => return Ok(Some(None)),
                }
            }
            _ => return Ok(None),
        };
        call.items.push(node);
        Ok(Some(None))
    }

    /// A stretchable arrow with an optional label below and the argument above. An empty above is no label.
    fn extended_arrow(&mut self, symbol: char) -> R<Node> {
        let below = self.optional_argument()?;
        let above = self.argument()?;
        let above = if empty(&above) { None } else { Some(Box::new(above)) };
        Ok(Node::Arrow { symbol, above, below: below.map(Box::new) })
    }

    /// `\phase[style]{angle}`: text style unless the option is 0, 1, 2 or 3.
    fn phase_angle(&mut self) -> R<Node> {
        let level = self.raw_optional().map_or(Some(1), |o| o.trim().parse::<u8>().ok().filter(|l| *l <= 3));
        let angle = self.argument()?;
        let base = match level {
            Some(level) => Node::Style { level, base: Box::new(angle) },
            None => angle,
        };
        Ok(Node::Framed { base: Box::new(base), shape: Frame::Phase, background: None, border: None })
    }

    /// `\not` and `\centernot`: a negated operator when one exists, otherwise a bar through whatever follows.
    fn negated_operator(&mut self, column: usize) -> R<Node> {
        self.skip_spaces();
        let mut following = Vec::new();
        if self.current() == Some('\\') {
            if let Some(p) = self.structural_command(&mut following)? {
                return self.error(column, format!("\\not before {p:?}"));
            }
        } else if self.current().is_some() {
            following.extend(self.atom()?);
        }
        match following.pop() {
            Some(Node::Operator { symbol, class }) => match negate(symbol) {
                Some(n) => Ok(Node::Operator { symbol: n, class }),
                None => Ok(Node::Canceled { base: Box::new(Node::Operator { symbol, class }), shape: Cancel::Bar }),
            },
            Some(other) => Ok(Node::Canceled { base: Box::new(other), shape: Cancel::Bar }),
            None => self.error(column, "\\not with no symbol after it"),
        }
    }

    /// `\big` and its family. `None` when the delimiter is missing: the command was read and draws nothing.
    fn sized_delimiter(&mut self, name: &str) -> R<Option<Node>> {
        let size = match name.trim_end_matches(['l', 'r', 'm']) {
            "big" => 1,
            "Big" => 2,
            "bigg" => 3,
            _ => 4,
        };
        let Some(symbol) = self.delimiter()? else {
            return Ok(None);
        };
        let class = match name.chars().last() {
            Some('l') => Class::Open,
            Some('r') => Class::Close,
            Some('m') => Class::Rel,
            _ => symbols::character(symbol).map_or(Class::Ord, |s| s.class),
        };
        Ok(Some(Node::FixedDelimiter { symbol, class, size }))
    }
}

/// Accent drawn under its base. Anything else is a two-headed arrow, as the old fallback was.
fn under_mark(name: &str) -> char {
    match name {
        "underrightarrow" => '→',
        "underleftarrow" => '←',
        // undertilde
        "utilde" => '\u{0303}',
        _ => '↔',
    }
}

/// Symbol of an extensible arrow. `\xlongequal` and any other name in that family is `=`.
fn extended_arrow_symbol(name: &str) -> char {
    match name {
        "xrightarrow" => '→',
        "xleftarrow" => '←',
        "xRightarrow" => '⇒',
        "xLeftarrow" => '⇐',
        "xleftrightarrow" => '↔',
        "xLeftrightarrow" => '⇔',
        "xrightleftharpoons" => '⇌',
        "xleftrightharpoons" => '⇋',
        "xmapsto" => '↦',
        "xhookrightarrow" => '↪',
        // mathtools and extarrows
        "xhookleftarrow" => '↩',
        "xtwoheadrightarrow" => '↠',
        "xtwoheadleftarrow" => '↞',
        "xrightharpoonup" => '⇀',
        "xrightharpoondown" => '⇁',
        "xleftharpoonup" => '↼',
        "xleftharpoondown" => '↽',
        "xtofrom" => '⇄',
        _ => '=',
    }
}
