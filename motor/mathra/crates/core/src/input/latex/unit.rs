//! siunitx, mhchem, arithmetic and the commands that build a quantity.

use super::formula::mhchem_unit;
use super::quantity::ListedEntry;
use super::*;

impl Reader<'_> {
    pub(super) fn unit_command(&mut self, call: CommandCall<'_>) -> R<Option<Option<Stop>>> {
        let node: Node = match call.name {
            // units, chemistry, arithmetic. siunitx options ([…]) come before the arguments and are ignored
            "SI" | "qty" => self.siunitx_quantity(call.column)?,
            "si" | "unit" => {
                self.raw_optional();
                let unit = self.raw_text()?;
                self.quantity(None, &unit, call.column)?
            }
            "num" => {
                self.raw_optional();
                let value = self.raw_text()?;
                self.raw_optional();
                self.number_value(&value, call.column)?
            }
            "ang" => {
                self.raw_optional();
                let value = self.raw_text()?;
                self.angle(&value, call.column)?
            }
            // L-UNI-2: ranges, lists and products repeat the unit on every value, as siunitx does by default
            "SIrange" | "qtyrange" | "numrange" | "angrange" => self.quantity_range(call.name, call.column)?,
            "SIlist" | "qtylist" | "numlist" | "anglist" => self.quantity_list(call.name, call.column)?,
            "SIproduct" | "qtyproduct" | "numproduct" => self.quantity_product(call.name, call.column)?,
            "ce" => {
                let raw = self.raw_text()?;
                self.chemistry(&raw, call.column)?
            }
            "pu" => self.physical_units(call.column)?,
            "xymatrix" => self.xymatrix(call.column)?,
            #[cfg(feature = "chem")]
            "chemfig" | "schemestart" | "chemname" => self.chem_construction(call.name, call.column)?,
            "divchave" => self.long_division(call.column)?,
            "contaarmada" => self.column_arithmetic(call.column)?,
            _ => return Ok(None),
        };
        call.items.push(node);
        Ok(Some(None))
    }

    /// `\SI` and `\qty`: drop an explicit plus and write the zero of a bare decimal point.
    fn siunitx_quantity(&mut self, column: usize) -> R<Node> {
        self.raw_optional();
        let value = self.raw_text()?;
        // siunitx leaves out an explicit plus (`retain-explicit-plus` is off) and adds the
        // zero of a bare point (`add-integer-zero`)
        let value = value.trim_start().strip_prefix('+').map_or(value.clone(), str::to_string);
        let value = match value.trim_start().strip_prefix('-') {
            Some(rest) if rest.starts_with('.') => format!("-0{rest}"),
            _ if value.trim_start().starts_with('.') => format!("0{}", value.trim_start()),
            _ => value,
        };
        let unit = self.raw_text()?;
        self.quantity(Some(value), &unit, column)
    }

    /// A range repeats the unit on both values. The phrase between them is the language's "to" when the font can write it.
    fn quantity_range(&mut self, name: &str, column: usize) -> R<Node> {
        self.raw_optional();
        let a = self.raw_text()?;
        let b = self.raw_text()?;
        let unit = if matches!(name, "SIrange" | "qtyrange") { Some(self.raw_text()?) } else { None };
        let a = self.listed_value(ListedEntry { command: name, value: &a, unit: unit.as_deref(), column })?;
        let b = self.listed_value(ListedEntry { command: name, value: &b, unit: unit.as_deref(), column })?;
        // siunitx's range-phrase: the language's "to", when the embedded font can write it, as for lists
        let to = Some(self.locale.phrase("op.–")).filter(|w| w.chars().all(|c| crate::font::font().glyph(c).is_some()));
        self.registry.note(rules::L_UNI_2, format!("\\{}: range written with '{}'", name, to.unwrap_or("–")));
        let between = match to {
            Some(to) => Node::Text { parts: vec![TextPart::Text(format!(" {to} "))] },
            None => Node::Operator { symbol: '–', class: Class::Rel },
        };
        Ok(Node::row(vec![a, between, b]))
    }

    /// A list repeats the unit. The last separator is the language's "and" when the font can write it.
    fn quantity_list(&mut self, name: &str, column: usize) -> R<Node> {
        self.raw_optional();
        let values = self.raw_text()?;
        let unit = if matches!(name, "SIlist" | "qtylist") { Some(self.raw_text()?) } else { None };
        let values: Vec<&str> = values.split(';').map(str::trim).filter(|v| !v.is_empty()).collect();
        // the last separator is the language's "and", when the embedded font can write it (it has
        // no CJK, Devanagari or Arabic letters); otherwise a comma, like the others
        let and = Some(self.locale.phrase("list_and")).filter(|w| w.chars().all(|c| crate::font::font().glyph(c).is_some()));
        let mut children = Vec::new();
        for (i, v) in values.iter().enumerate() {
            match and {
                Some(and) if i > 0 && i + 1 == values.len() => children.push(Node::Text { parts: vec![TextPart::Text(format!(" {and} "))] }),
                _ if i > 0 => children.push(Node::Operator { symbol: ',', class: Class::Punct }),
                _ => {}
            }
            children.push(self.listed_value(ListedEntry { command: name, value: v, unit: unit.as_deref(), column })?);
        }
        self.registry.note(rules::L_UNI_2, format!("\\{}: list closed with '{}'", name, and.unwrap_or(",")));
        Ok(Node::row(children))
    }

    /// A product repeats the unit and joins the values with ×.
    fn quantity_product(&mut self, name: &str, column: usize) -> R<Node> {
        self.raw_optional();
        let values = self.raw_text()?;
        let unit = if name == "numproduct" { None } else { Some(self.raw_text()?) };
        let mut children = Vec::new();
        for (i, v) in values.split('x').map(str::trim).enumerate() {
            if i > 0 {
                children.push(Node::Operator { symbol: '×', class: Class::Bin });
            }
            children.push(self.listed_value(ListedEntry { command: name, value: v, unit: unit.as_deref(), column })?);
        }
        Ok(Node::row(children))
    }

    /// mhchem's physical units: a number, then the unit with `^` exponents and `.` or `*` between units.
    fn physical_units(&mut self, column: usize) -> R<Node> {
        let raw = self.raw_text()?;
        let raw = raw.trim();
        let (value, unit) = match raw.split_once(char::is_whitespace) {
            Some((v, u)) if siunitx_number(v, false).is_some() => (Some(v.to_string()), u.trim().to_string()),
            _ if siunitx_number(raw, false).is_some() => (Some(raw.to_string()), String::new()),
            _ => (None, raw.to_string()),
        };
        let unit_node = self.sub_reader(&mhchem_unit(&unit), Variant::Upright)?;
        self.quantity_node(value, unit_node, column)
    }

    /// Long division of two integers. Anything else is written as inline division.
    fn long_division(&mut self, column: usize) -> R<Node> {
        let dividend = self.raw_text()?.trim().to_string();
        let divisor = self.raw_text()?.trim().to_string();
        if !is_integer(&dividend) || !is_integer(&divisor) || divisor.trim_start_matches('0').is_empty() {
            let problem = "\\divchave{dividend}{divisor} takes integers, with a nonzero divisor";
            self.tolerate(rules::E_REC_9, column, problem, "written as inline division")?;
            self.sub_reader(&format!("{{{dividend}}} \\div {{{divisor}}}"), self.variant)
        } else {
            Ok(Node::LongDivision { dividend, divisor })
        }
    }

    /// Column arithmetic of two integers. Anything else is written as an inline calculation.
    fn column_arithmetic(&mut self, column: usize) -> R<Node> {
        let op = self.raw_text()?.trim().to_string();
        let operation = match op.as_str() {
            "+" => Some('+'),
            "-" => Some('−'),
            "x" | "\\times" | "×" | "*" => Some('×'),
            _ => None,
        };
        let a = self.raw_text()?.trim().to_string();
        let b = self.raw_text()?.trim().to_string();
        match operation {
            Some(operation) if is_integer(&a) && is_integer(&b) => Ok(Node::ColumnArithmetic { operation, terms: vec![a, b] }),
            _ => {
                let problem = if operation.is_none() { "\\contaarmada{op}{a}{b}: op is +, - or \\times" } else { "\\contaarmada takes integers" };
                self.tolerate(rules::E_REC_9, column, problem, "written as an inline calculation")?;
                let symbol = operation.map_or(op, String::from);
                self.sub_reader(&format!("{{{a}}} {symbol} {{{b}}}"), self.variant)
            }
        }
    }
}
