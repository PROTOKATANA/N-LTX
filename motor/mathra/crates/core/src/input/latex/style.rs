//! Style, color and infix fractions (\over, \atop, \choose, \above).

use super::*;

impl Reader<'_> {
    pub(super) fn style_command(&mut self, call: CommandCall<'_>) -> R<Option<Option<Stop>>> {
        let node: Node = match call.name {
            // style and font for the rest of the group
            "displaystyle" | "textstyle" | "scriptstyle" | "scriptscriptstyle" => {
                let level = match call.name {
                    "displaystyle" => 0,
                    "textstyle" => 1,
                    "scriptstyle" => 2,
                    _ => 3,
                };
                return self.styled_rest(level, call.items);
            }
            "color" => return self.colored_rest(call.items),
            "over" | "choose" | "atop" | "brack" | "brace" | "above" => return self.infix_fraction(call.name, call.items, call.column),
            "bf" | "rm" | "it" | "sf" | "tt" | "cal" | "mit" | "boldmath" | "unboldmath" | "normalfont" => {
                self.variant = declared_variant(call.name);
                return Ok(Some(None));
            }
            "tiny" | "scriptsize" | "footnotesize" | "small" | "normalsize" | "large" | "Large" | "LARGE" | "huge" | "Huge" => {
                self.registry.note(rules::L_ENT_3, format!("\\{} ignored: the size comes from whoever uses the SVG", call.name));
                return Ok(Some(None));
            }
            "limits" | "nolimits" | "displaylimits" => {
                let wanted = match call.name {
                    "limits" => Some(true),
                    "nolimits" => Some(false),
                    _ => None,
                };
                if let Some(last_item) = call.items.last_mut() {
                    set_limits(last_item, wanted);
                }
                return Ok(Some(None));
            }
            "label" | "ref" | "eqref" => {
                let label = self.raw_text()?;
                if call.name == "label" {
                    return Ok(Some(None));
                }
                Node::Text { parts: vec![TextPart::Text(if call.name == "eqref" { format!("({label})") } else { label })] }
            }
            "nonumber" | "notag" | "nolinebreak" | "allowbreak" | "displaybreak" | "protect" | "relax" | "noindent" | "centering" | "ensuremath" => return Ok(Some(None)),
            "strut" | "mathstrut" => Node::Phantom { base: Box::new(Node::Operator { symbol: '(', class: Class::Open }), width: false, height: true },
            "tag" => {
                let label = self.raw_text()?;
                let text = if call.starred { label } else { format!("({label})") };
                call.items.push(Node::Space { em: 2.0 });
                Node::Text { parts: vec![TextPart::Text(text)] }
            }
            _ => return Ok(None),
        };
        call.items.push(node);
        Ok(Some(None))
    }

    /// The command sets the style of everything read until the group ends.
    fn styled_rest(&mut self, level: u8, items: &mut Vec<Node>) -> R<Option<Option<Stop>>> {
        let (remainder, stop) = self.list()?;
        items.push(Node::Style { level, base: Box::new(Node::row(remainder)) });
        Ok(Some(Some(stop)))
    }

    /// `\color{…}` paints everything read until the group ends.
    fn colored_rest(&mut self, items: &mut Vec<Node>) -> R<Option<Option<Stop>>> {
        let color = self.color()?;
        let (remainder, stop) = self.list()?;
        let base = Node::row(remainder);
        items.push(match color {
            Some(color) => Node::Colored { color, base: Box::new(base) },
            None => base,
        });
        Ok(Some(Some(stop)))
    }

    /// An infix fraction: the atoms already read are the numerator, the rest of the group the denominator.
    fn infix_fraction(&mut self, name: &str, items: &mut Vec<Node>, column: usize) -> R<Option<Option<Stop>>> {
        let num = Node::row(std::mem::take(items));
        let thickness = if name == "above" { Some(self.above_dimension(column)?) } else { None };
        let (remainder, stop) = self.list()?;
        let den = Node::row(remainder);
        let rule = matches!(name, "over" | "above");
        let fraction = Node::Fraction { num: Box::new(num), den: Box::new(den), style: FractionStyle::Auto, rule, thickness };
        items.push(match name {
            "choose" => Node::Delimited { open: Some('('), close: Some(')'), parts: vec![fraction], middles: vec![] },
            "brack" => Node::Delimited { open: Some('['), close: Some(']'), parts: vec![fraction], middles: vec![] },
            "brace" => Node::Delimited { open: Some('{'), close: Some('}'), parts: vec![fraction], middles: vec![] },
            _ => fraction,
        });
        Ok(Some(Some(stop)))
    }

    /// `\above<dimen>`: TeX reads a dimension, written bare (`2pt`) or, as KaTeX allows, braced.
    fn above_dimension(&mut self, column: usize) -> R<f32> {
        self.skip_spaces();
        let raw = if self.current() == Some('{') {
            self.raw_text()?
        } else {
            let start = self.pos;
            while self.current().is_some_and(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-')) {
                self.pos += 1;
            }
            while self.current().is_some_and(|c| c.is_ascii_alphabetic()) && self.pos - start < 12 {
                self.pos += 1;
                if measure_em(&self.c[start..self.pos].iter().collect::<String>()).is_some() {
                    break;
                }
            }
            self.c[start..self.pos].iter().collect()
        };
        Ok(measure_em(&raw).ok_or(Error::Syntax { column, message: format!("\\above needs a dimension, not {raw:?}") })?.max(0.0))
    }
}

/// Math alphabet declared for the rest of the group. The names this does not know stay italic.
fn declared_variant(name: &str) -> Variant {
    match name {
        "bf" | "boldmath" => Variant::Bold,
        "rm" | "normalfont" => Variant::Upright,
        "sf" => Variant::SansSerif,
        "tt" => Variant::Monospace,
        "cal" => Variant::Calligraphic,
        _ => Variant::Italic,
    }
}
