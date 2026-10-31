//! Atoms of a formula: letters, numbers, groups, arguments and scripts.

use super::*;

impl Reader<'_> {
    /// One atom, without scripts. May return several nodes (a number with a thin space, for example).
    pub(super) fn atom(&mut self) -> R<Vec<Node>> {
        let col = self.column();
        let c = self.current().expect("called with a character");
        Ok(match c {
            '{' => vec![self.group()?],
            '0'..='9' | '.' if c != '.' || self.em(1).is_some_and(|d| d.is_ascii_digit()) => {
                vec![self.number()]
            }
            '~' => {
                self.pos += 1;
                vec![Node::Space { em: 0.333 }]
            }
            '$' => {
                // a stray dollar sign in the middle of the equation: a leftover delimiter
                self.pos += 1;
                vec![]
            }
            c if c.is_control() => {
                self.tolerate(rules::E_REC_9, col, format!("control character U+{:04X}", c as u32), "ignored")?;
                self.pos += 1;
                vec![]
            }
            '#' => {
                self.tolerate(rules::E_REC_9, col, "'#' outside a macro definition", "drawn as a symbol")?;
                self.pos += 1;
                vec![Node::Operator { symbol: '#', class: Class::Ord }]
            }
            c if c.is_alphabetic() => {
                self.pos += 1;
                vec![self.letter(c)]
            }
            c => {
                self.pos += 1;
                match symbols::character(c) {
                    Some(Symbol { symbol, class }) => vec![Node::Operator { symbol, class }],
                    None => vec![Node::Operator { symbol: c, class: Class::Ord }],
                }
            }
        })
    }

    pub(super) fn letter(&self, c: char) -> Node {
        let variant = if c.is_ascii_alphabetic() || ('α'..='ω').contains(&c) || c == 'ı' || c == 'ȷ' { self.variant } else { Variant::Upright };
        Node::Identifier { name: c.to_string(), variant }
    }

    pub(super) fn group(&mut self) -> R<Node> {
        let col = self.column();
        self.pos += 1;
        let variant = self.variant;
        let (items, stop) = self.list()?;
        self.variant = variant;
        match stop {
            Stop::CloseBrace => {}
            Stop::Eof => self.tolerate(rules::E_REC_4, col, "'{' without a matching '}'", "closed at the end")?,
            other => {
                self.tolerate(rules::E_REC_4, col, format!("group opened with '{{' ended in {other:?}"), "closed there")?;
            }
        }
        // as in TeX, an operator alone in braces becomes an ordinary atom: {,} gets no space
        Ok(match items.as_slice() {
            [Node::Operator { symbol, .. }] => Node::Operator { symbol: *symbol, class: Class::Ord },
            _ => Node::row(items),
        })
    }

    pub(super) fn number(&mut self) -> Node {
        if let Some(node) = self.number_with_thousands() {
            return node;
        }
        let start = self.pos;
        let mut value = String::new();
        while let Some(c) = self.current() {
            let decimal = (c == '.' || (c == ',' && self.locale.comma_is_decimal())) && self.em(1).is_some_and(|d| d.is_ascii_digit()) && !value.contains('.');
            if c.is_ascii_digit() {
                value.push(c);
            } else if decimal {
                value.push('.');
            } else {
                break;
            }
            self.pos += 1;
        }
        let written: String = self.c[start..self.pos].iter().collect();
        if written.contains(',') {
            self.registry.note(rules::L_NUM_1, format!("'{written}': comma between digits read as decimal separator ({value})"));
        }
        Node::Number { value, group: false, thousands: None }
    }

    /// L-NUM-3: `1.000` and `12.345,67` in pt-BR, `1,000` and `12,345.67` in English: the locale's thousands
    /// separator followed by groups of exactly three digits separates thousands and is kept in the drawing.
    pub(super) fn number_with_thousands(&mut self) -> Option<Node> {
        let thousands = self.locale.thousands_separator()?;
        let decimal = self.locale.decimal_separator();
        let mut i = self.pos;
        let digits = |i: &mut usize| -> String {
            let mut s = String::new();
            while let Some(c) = self.c.get(*i).filter(|c| c.is_ascii_digit()) {
                s.push(*c);
                *i += 1;
            }
            s
        };
        let first = digits(&mut i);
        if first.is_empty() || first.len() > 3 {
            return None;
        }
        let mut groups = vec![first];
        while self.c.get(i) == Some(&thousands) && self.c.get(i + 1).is_some_and(|c| c.is_ascii_digit()) {
            let mut j = i + 1;
            let g = digits(&mut j);
            if g.len() != 3 {
                break;
            }
            groups.push(g);
            i = j;
        }
        if groups.len() < 2 {
            return None;
        }
        // after the last group no other digit may follow the thousands separator directly
        if self.c.get(i) == Some(&thousands) && self.c.get(i + 1).is_some_and(|c| c.is_ascii_digit()) {
            return None;
        }
        let mut value: String = groups.concat();
        if self.c.get(i) == Some(&decimal) && self.c.get(i + 1).is_some_and(|c| c.is_ascii_digit()) {
            let mut j = i + 1;
            let f = digits(&mut j);
            value.push('.');
            value.push_str(&f);
            i = j;
        }
        let written: String = self.c[self.pos..i].iter().collect();
        self.pos = i;
        self.registry.note(rules::L_NUM_3, format!("'{written}': '{thousands}' separa milhares ({value})"));
        Some(Node::Number { value, group: false, thousands: Some(thousands) })
    }

    /// Argument of a command or script: a group or a single token.
    pub(super) fn argument(&mut self) -> R<Node> {
        self.skip_spaces();
        let col = self.column();
        match self.current() {
            None => {
                self.tolerate(rules::E_REC_7, col, "missing argument", "left empty")?;
                Ok(Node::row(vec![]))
            }
            Some('{') => Ok(self.group()?.simplify()),
            Some('}') => {
                self.tolerate(rules::E_REC_7, col, "missing argument", "left empty")?;
                Ok(Node::row(vec![]))
            }
            Some('\\') => {
                let mut items = Vec::new();
                if let Some(p) = self.structural_command(&mut items)? {
                    self.tolerate(rules::E_REC_7, col, format!("invalid argument: {p:?}"), "ignored")?;
                }
                Ok(Node::row(items).simplify())
            }
            Some(c) if c.is_ascii_digit() => {
                self.pos += 1;
                Ok(Node::Number { value: c.to_string(), group: false, thousands: None })
            }
            Some(_) => Ok(Node::row(self.atom()?).simplify()),
        }
    }

    /// Base of `\overset` and `\underset`: `{=}` stays a relation (the plain group would make it Ord).
    pub(super) fn base_with_class(&mut self) -> R<Node> {
        self.skip_spaces();
        if self.current() == Some('{') {
            let raw = self.raw_text()?;
            return self.sub_reader(&raw, self.variant);
        }
        self.argument()
    }

    pub(super) fn optional_argument(&mut self) -> R<Option<Node>> {
        match self.raw_optional() {
            Some(s) => Ok(Some(self.sub_reader(&s, self.variant)?)),
            None => Ok(None),
        }
    }

    /// `^`, `_` and `'` attach to the last atom.
    pub(super) fn scripts(&mut self, items: &mut Vec<Node>) -> R<()> {
        self.nested(|this| this.scripts_unguarded(items))
    }

    pub(super) fn scripts_unguarded(&mut self, items: &mut Vec<Node>) -> R<()> {
        // x_1_2_3…: each double script starts over on an empty base, in a loop (not by recursion)
        'index: loop {
            let previous = items.pop().unwrap_or(Node::row(vec![]));
            // the brace of \overbrace{…}^{label} takes the script as its label
            if let Node::Brace { above, label: None, .. } = &previous {
                let mark = if *above { '^' } else { '_' };
                if self.current() == Some(mark) {
                    self.pos += 1;
                    let label = self.argument()?;
                    if let Node::Brace { base, above, bracket, .. } = previous {
                        items.push(Node::Brace { base, above, label: Some(Box::new(label)), bracket });
                    }
                    return Ok(());
                }
            }
            let (base, mut sup, mut sub) = match previous {
                Node::Scripts { base, sup, sub } => (*base, sup.map(|b| *b), sub.map(|b| *b)),
                other => (other, None, None),
            };
            let only_primes = |s: &Option<Node>| matches!(s, Some(Node::Operator { symbol: '′' | '″' | '‴', .. }));
            loop {
                self.skip_spaces();
                match self.current() {
                    Some('\'') => {
                        let mut n = 0;
                        while self.current() == Some('\'') {
                            n += 1;
                            self.pos += 1;
                        }
                        let prime = Node::Operator { symbol: ['′', '″', '‴'][n.min(3) - 1], class: Class::Ord };
                        sup = Some(match sup {
                            None => prime,
                            Some(s) => Node::row(std::iter::once(prime).chain(undo_line(s)).collect()),
                        });
                    }
                    Some('^') if sup.is_none() || only_primes(&sup) => {
                        self.pos += 1;
                        let arg = self.argument()?;
                        sup = Some(match sup {
                            None => arg,
                            Some(prime) => Node::row(vec![prime, arg]),
                        });
                    }
                    Some('_') if sub.is_none() => {
                        self.pos += 1;
                        sub = Some(self.argument()?);
                    }
                    Some(c @ ('^' | '_')) => {
                        let col = self.column();
                        let which = if c == '^' { "superscript" } else { "subscript" };
                        self.tolerate(rules::E_REC_3, col, format!("double {which}: use braces, as in x{c}{{a{c}b}}"), "the second went on an empty base")?;
                        // close the current script and start over with an empty base
                        items.push(Node::Scripts { base: Box::new(base), sup: sup.take().map(Box::new), sub: sub.take().map(Box::new) });
                        items.push(Node::row(vec![]));
                        continue 'index;
                    }
                    _ => break,
                }
            }
            // we only get here after reading at least one script
            items.push(Node::Scripts { base: Box::new(base), sup: sup.map(Box::new), sub: sub.map(Box::new) });
            return Ok(());
        }
    }
}
