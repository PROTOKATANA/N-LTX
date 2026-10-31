//! A siunitx quantity: the number and the unit written with a slash.

use super::*;

/// One entry of a siunitx range, list or product: the command decides whether it is a number, an angle or a quantity.
pub(super) struct ListedEntry<'a> {
    pub(super) command: &'a str,
    pub(super) value: &'a str,
    pub(super) unit: Option<&'a str>,
    pub(super) column: usize,
}

/// One factor of a unit written with commands: what is drawn, its exponent, and the names kept for speech.
struct UnitFactor {
    text: String,
    exponent: i32,
    prefix: String,
    name: String,
}

impl Reader<'_> {
    /// A siunitx number; in tolerant mode, something else is drawn as math, with a warning.
    pub(super) fn number_value(&mut self, value: &str, col: usize) -> R<Node> {
        if let Some(n) = siunitx_number(value, true) {
            return Ok(n);
        }
        self.tolerate(rules::E_REC_9, col, format!("{value} is not a number"), "drawn as written")?;
        self.sub_reader(value, Variant::Italic)
    }

    /// `\ang{30}` → 30°; `\ang{12;30;15}` → 12°30′15″ (degrees, minutes and seconds; any of them may be empty).
    pub(super) fn angle(&mut self, value: &str, col: usize) -> R<Node> {
        let mut children = Vec::new();
        for (part, mark) in value.split(';').zip(['°', '′', '″']) {
            if part.trim().is_empty() {
                continue;
            }
            children.push(self.number_value(part, col)?);
            children.push(Node::Operator { symbol: mark, class: Class::Ord });
        }
        if value.split(';').count() > 3 {
            self.tolerate(rules::E_REC_9, col, format!("\\ang{{{value}}}: more than degrees, minutes and seconds"), "the rest ignored")?;
        }
        Ok(Node::row(children))
    }

    /// One value of a range, list or product: a number, an angle or a quantity, by the command.
    pub(super) fn listed_value(&mut self, entry: ListedEntry<'_>) -> R<Node> {
        match entry.unit {
            Some(u) => self.quantity(Some(entry.value.to_string()), u, entry.column),
            None if entry.command.starts_with("ang") => self.angle(entry.value, entry.column),
            None => self.number_value(entry.value, entry.column),
        }
    }

    /// L-UNI-1: `\SI{9,8}{\meter\per\second\squared}` → 9,8 m/s² in pt-BR.
    pub(super) fn quantity(&mut self, value: Option<String>, unit: &str, col: usize) -> R<Node> {
        let unit_node = if unit.contains('\\') { self.command_units(unit, col)? } else { self.sub_reader(unit, Variant::Upright)? };
        self.quantity_node(value, unit_node, col)
    }

    /// A unit written as commands (`\meter\per\second\squared`): numerator, slash, denominator.
    fn command_units(&mut self, unit: &str, col: usize) -> R<Node> {
        let mut numerator: Vec<UnitFactor> = Vec::new();
        let mut denominator: Vec<UnitFactor> = Vec::new();
        let mut prefix = String::new();
        let mut prefix_names = String::new();
        let mut per = false;
        let mut last_in_den = false;
        for token in unit.split('\\').map(str::trim).filter(|t| !t.is_empty()) {
            let (name, remainder) = token.split_at(token.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(token.len()));
            match name {
                "per" => per = true,
                "squared" | "cubed" => {
                    let target = if last_in_den { denominator.last_mut() } else { numerator.last_mut() };
                    if let Some(u) = target {
                        u.exponent *= if name == "squared" { 2 } else { 3 };
                    }
                }
                "square" | "cubic" => prefix.push(if name == "square" { '²' } else { '³' }),
                "tothe" | "raiseto" => {
                    let n: i32 = remainder.trim_matches(['{', '}', ' ']).parse().unwrap_or(1);
                    let target = if last_in_den { denominator.last_mut() } else { numerator.last_mut() };
                    if let Some(u) = target {
                        u.exponent *= n;
                    }
                }
                _ => {
                    if let Some(p) = symbols::prefix(name) {
                        // \square\kilo\meter: the pending exponent still applies (km²)
                        prefix.insert_str(0, p);
                        prefix_names.insert_str(0, symbols::canonical_prefix(name));
                        continue;
                    }
                    let symbol = match symbols::unit(name) {
                        Some(symbol) => symbol.to_string(),
                        None => {
                            self.tolerate(rules::E_REC_1, col, format!("unsupported unit: \\{name}"), "written by its name")?;
                            name.to_string()
                        }
                    };
                    let exponent = if prefix.contains('²') {
                        2
                    } else if prefix.contains('³') {
                        3
                    } else {
                        1
                    };
                    let text = format!("{}{symbol}", prefix.trim_end_matches(['²', '³']));
                    prefix.clear();
                    let names = (std::mem::take(&mut prefix_names), symbols::canonical_unit(name).to_string());
                    let factor = UnitFactor { text, exponent, prefix: names.0, name: names.1 };
                    if per {
                        denominator.push(factor);
                        last_in_den = true;
                        per = false;
                    } else {
                        numerator.push(factor);
                        last_in_den = false;
                    }
                }
            }
        }
        let mut children = Vec::new();
        let unit_node = |u: &UnitFactor| -> Node {
            let text = Node::Text { parts: vec![TextPart::Text(u.text.clone())] };
            let base = Node::Unit { prefix: Some(u.prefix.clone()).filter(|p| !p.is_empty()), name: u.name.clone(), base: Box::new(text) };
            if u.exponent == 1 {
                base
            } else {
                Node::Scripts { base: Box::new(base), sup: Some(Box::new(Node::Number { value: u.exponent.to_string(), group: false, thousands: None })), sub: None }
            }
        };
        for (i, u) in numerator.iter().enumerate() {
            if i > 0 {
                children.push(Node::Space { em: 3.0 / 18.0 });
            }
            children.push(unit_node(u));
        }
        if !denominator.is_empty() {
            if numerator.is_empty() {
                children.push(Node::Number { value: "1".into(), group: false, thousands: None });
            }
            children.push(Node::Operator { symbol: '/', class: Class::Ord });
            let several = denominator.len() > 1;
            if several {
                children.push(Node::Operator { symbol: '(', class: Class::Open });
            }
            for (i, u) in denominator.iter().enumerate() {
                if i > 0 {
                    children.push(Node::Operator { symbol: '·', class: Class::Ord });
                }
                children.push(unit_node(u));
            }
            if several {
                children.push(Node::Operator { symbol: ')', class: Class::Close });
            }
        }
        self.registry.note(rules::L_UNI_1, format!("unit {unit} written with a slash for negative exponents"));
        let names: Vec<String> = numerator.iter().chain(&denominator).map(|u| format!("{}{}", u.prefix, u.name)).collect();
        self.registry.note(rules::L_UNI_3, format!("units kept by name for the speech: {}", names.join(", ")));
        Ok(Node::row(children))
    }

    /// Value, thin space and unit, as one quantity.
    pub(super) fn quantity_node(&mut self, value: Option<String>, unit_node: Node, col: usize) -> R<Node> {
        let value = match value {
            Some(v) => Some(self.number_value(&v, col)?),
            None => None,
        };
        let text_value = value.as_ref().map(|v| match v {
            Node::Number { value, .. } => value.clone(),
            _ => String::new(),
        });
        let mut children = Vec::new();
        if let Some(v) = value {
            children.push(v);
            children.push(Node::Space { em: 3.0 / 18.0 });
        }
        children.push(unit_node);
        Ok(Node::Quantity { value: text_value.filter(|v| !v.is_empty()), unit: Box::new(Node::row(children)) })
    }
}
