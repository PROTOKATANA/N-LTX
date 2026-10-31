//! mhchem formulas: elements, charges, bonds and reaction arrows.

use super::*;

/// The scan of one chemical formula.
struct Formula<'a> {
    c: &'a [char],
    i: usize,
    column: usize,
    coefficient_allowed: bool,
    children: Vec<Node>,
}

impl Reader<'_> {
    /// L-CHE-1: mhchem subset. Formulas with automatic subscripts, charges, isotopes, hydrates, bonds and
    /// coefficients (integer, decimal, fraction or `$…$`), arrows with labels, `\frac` and math commands.
    pub(super) fn chemistry(&mut self, raw: &str, col: usize) -> R<Node> {
        let c: Vec<char> = raw.chars().collect();
        let mut children: Vec<Node> = Vec::new();
        let mut i = 0;
        loop {
            while c.get(i).is_some_and(|x| x.is_whitespace()) {
                i += 1;
            }
            if i >= c.len() {
                break;
            }
            if let Some(previous) = children.last() {
                // a variable coefficient written apart ($x$ H2O, n H2O) gets the small space a number gets
                let coefficient = matches!(previous.clone().simplify(), Node::Identifier { .. });
                children.push(Node::Space { em: if coefficient { 2.0 / 18.0 } else { 0.0 } });
            }
            if let Some((symbol, len)) = chemical_arrow(&c[i..]) {
                i += len;
                let above = bracketed(&c, &mut i);
                let below = if above.is_some() { bracketed(&c, &mut i) } else { None };
                if above.is_none() {
                    children.push(Node::Operator { symbol, class: Class::Rel });
                    continue;
                }
                let label = |t: Option<String>, this: &mut Self| -> R<Option<Box<Node>>> {
                    match t.filter(|t| !t.trim().is_empty()) {
                        Some(t) => Ok(Some(Box::new(this.chemistry(&t, col)?.simplify()))),
                        None => Ok(None),
                    }
                };
                let above = label(above, self)?;
                let below = label(below, self)?;
                self.registry.note(rules::L_CHE_1, "arrow with a label drawn stretched over it");
                children.push(Node::Arrow { symbol, above, below });
                continue;
            }
            let token = chemical_token(&c, &mut i);
            match token.as_str() {
                "+" => children.push(Node::Operator { symbol: '+', class: Class::Bin }),
                "=" => children.push(Node::Operator { symbol: '=', class: Class::Rel }),
                "^" | "v" => children.push(Node::Operator { symbol: if token == "^" { '↑' } else { '↓' }, class: Class::Ord }),
                "→" | "⇌" | "←" => children.push(Node::Operator { symbol: token.chars().next().unwrap_or('→'), class: Class::Rel }),
                // a variable as coefficient (x, n): italic, like in math
                t if t.len() == 1 && t.chars().all(|x| x.is_ascii_lowercase()) => children.push(Node::Identifier { name: t.to_string(), variant: Variant::Italic }),
                // a word (heat, light, catalyst): upright text
                t if t.len() > 1 && t.chars().all(|x| x.is_alphabetic() && x.is_lowercase()) => children.push(Node::Text { parts: vec![TextPart::Text(t.to_string())] }),
                formula => children.push(self.chemical_formula(formula, col)?),
            }
        }
        Ok(Node::row(children))
    }

    pub(super) fn chemical_formula(&mut self, f: &str, col: usize) -> R<Node> {
        let c: Vec<char> = f.chars().collect();
        let mut formula = Formula { c: &c, i: 0, column: col, coefficient_allowed: true, children: Vec::new() };
        while formula.i < formula.c.len() {
            self.formula_piece(&mut formula)?;
        }
        Ok(Node::row(formula.children))
    }

    /// One character of a formula: a coefficient, an element, a script, a bond or a symbol.
    fn formula_piece(&mut self, formula: &mut Formula<'_>) -> R<()> {
        let ch = formula.c[formula.i];
        if formula.coefficient_allowed && ch.is_ascii_digit() {
            formula.coefficient_allowed = false;
            formula.children.push(chemical_coefficient(formula.c, &mut formula.i));
            formula.children.push(Node::Space { em: 2.0 / 18.0 });
            return Ok(());
        }
        formula.coefficient_allowed = false;
        if ch == '$' {
            let node = self.formula_math(formula)?;
            formula.children.push(node);
        } else if ch == '\\' {
            let node = self.chemical_command(formula.c, &mut formula.i, formula.column)?;
            formula.children.push(node);
        } else if ch.is_ascii_uppercase() {
            let node = formula_element(formula);
            formula.children.push(node);
        } else if ch.is_ascii_digit() {
            formula_digits(formula);
        } else if ch == '^' || ch == '_' {
            self.formula_script(formula, ch)?;
        } else if matches!(ch, '+' | '-') && !formula.children.is_empty() && (formula.i + 1 == formula.c.len() || matches!(formula.c[formula.i + 1], ']' | ')')) {
            // a charge at the end of the formula or before a closing bracket: Ag+, [H+], [A-]
            attach_script(&mut formula.children, Some(charge(&ch.to_string())), None);
            formula.i += 1;
        } else if let Some(state) = formula_state(formula) {
            formula.children.push(upright_text(state));
        } else if matches!(ch, '.' | '*' | '·') && formula.i > 0 {
            // hydrate or adduct: the dot is centered and a coefficient may follow
            formula.children.push(Node::Operator { symbol: '·', class: Class::Ord });
            formula.coefficient_allowed = true;
            formula.i += 1;
        } else if ch.is_alphabetic() && ch.is_lowercase() {
            // e- (electron), loose lowercase letters: upright, like the elements
            let node = formula_lowercase(formula);
            formula.children.push(node);
        } else {
            self.formula_symbol(formula, ch)?;
        }
        Ok(())
    }

    /// `$…$` inside a formula, read as math. A missing closer runs to the end.
    fn formula_math(&mut self, formula: &mut Formula<'_>) -> R<Node> {
        let end = formula.c[formula.i + 1..].iter().position(|&x| x == '$').map_or(formula.c.len(), |p| p + formula.i + 1);
        let snippet: String = formula.c[formula.i + 1..end].iter().collect();
        formula.i = (end + 1).min(formula.c.len());
        self.sub_reader(&snippet, Variant::Italic)
    }

    /// `^` or `_`: a charge above, or a subscript (a number, a state or math).
    fn formula_script(&mut self, formula: &mut Formula<'_>, ch: char) -> R<()> {
        formula.i += 1;
        let payload = if formula.c.get(formula.i) == Some(&'{') {
            braced(formula.c, &mut formula.i)
        } else {
            let mut p = String::new();
            while formula.i < formula.c.len() && (formula.c[formula.i].is_ascii_alphanumeric() || (ch == '^' && matches!(formula.c[formula.i], '+' | '-' | '.'))) {
                p.push(formula.c[formula.i]);
                formula.i += 1;
                if ch == '_' && !p.chars().all(|x| x.is_ascii_digit()) {
                    break;
                }
            }
            p
        };
        let node = if ch == '^' { charge(&payload) } else { self.chemical_subscript(&payload)? };
        if ch == '^' {
            attach_script(&mut formula.children, Some(node), None);
        } else {
            attach_script(&mut formula.children, None, Some(node));
        }
        Ok(())
    }

    /// A bond, a bracket or any other character. An unsupported one is drawn as it is, with a warning.
    fn formula_symbol(&mut self, formula: &mut Formula<'_>, ch: char) -> R<()> {
        let class = match ch {
            '(' | '[' => Class::Open,
            ')' | ']' => Class::Close,
            _ => Class::Ord,
        };
        // bonds: - single, = double, # triple
        if let Some(line) = match ch {
            '-' => Some('−'),
            '=' => Some('='),
            '#' => Some('≡'),
            _ => None,
        } {
            formula.children.push(bond(line));
            formula.i += 1;
            return Ok(());
        }
        let symbol = ch;
        if ch.is_alphabetic() || ch.is_ascii_punctuation() {
            formula.children.push(Node::Operator { symbol, class });
        } else {
            self.tolerate(rules::E_REC_9, formula.column, format!("unsupported character in \\ce: '{ch}'"), "drawn as is")?;
            formula.children.push(Node::Operator { symbol, class });
        }
        formula.i += 1;
        Ok(())
    }

    /// Subscript in a formula: a number, a state (`_{(aq)}`) as text, anything else as math (`NO_x`).
    pub(super) fn chemical_subscript(&mut self, payload: &str) -> R<Node> {
        if !payload.is_empty() && payload.chars().all(|x| x.is_ascii_digit()) {
            return Ok(Node::Number { value: payload.to_string(), group: false, thousands: None });
        }
        if payload.starts_with('(') {
            return Ok(Node::Text { parts: vec![TextPart::Text(payload.to_string())] });
        }
        self.sub_reader(payload, Variant::Italic)
    }

    /// A command inside `\ce`: `\bond{…}`, `\frac{…}{…}` with formulas inside, or any math command with
    /// the braced arguments that follow it (`\Delta`, `\mu`, `\text{…}`).
    pub(super) fn chemical_command(&mut self, c: &[char], i: &mut usize, col: usize) -> R<Node> {
        let start = *i;
        *i += 1;
        let mut name = String::new();
        while *i < c.len() && c[*i].is_ascii_alphabetic() {
            name.push(c[*i]);
            *i += 1;
        }
        if name.is_empty() && *i < c.len() {
            name.push(c[*i]);
            *i += 1;
        }
        let mut arguments = Vec::new();
        while c.get(*i) == Some(&'{') {
            arguments.push(braced(c, i));
        }
        match (name.as_str(), arguments.as_slice()) {
            ("bond", [b, ..]) => {
                let symbol = match b.trim() {
                    "-" | "1" => '−',
                    "=" | "2" => '=',
                    "#" | "3" => '≡',
                    "~" => '∼',
                    "..." => '⋯',
                    "->" => '→',
                    "<-" => '←',
                    other => {
                        self.tolerate(rules::E_REC_9, col, format!("unknown bond: \\bond{{{other}}}"), "drawn as a single bond")?;
                        '−'
                    }
                };
                Ok(bond(symbol))
            }
            ("frac", [a, b, ..]) => {
                let num = self.chemistry(a, col)?.simplify();
                let den = self.chemistry(b, col)?.simplify();
                Ok(Node::Fraction { num: Box::new(num), den: Box::new(den), style: FractionStyle::Auto, rule: true, thickness: None })
            }
            _ => {
                let raw: String = c[start..*i].iter().collect();
                self.sub_reader(&raw, Variant::Italic)
            }
        }
    }
}

/// A chemical bond: drawn as its line, but it is not a minus, an equals or a congruence.
pub(super) fn bond(symbol: char) -> Node {
    Node::Text { parts: vec![TextPart::Text(symbol.to_string())] }
}

/// mhchem arrows, the longest first: (drawn symbol, length in the input).
pub(super) fn chemical_arrow(c: &[char]) -> Option<(char, usize)> {
    const ARROWS: [(&str, char); 7] = [("<=>>", '⥂'), ("<<=>", '⥄'), ("<-->", '⇄'), ("<=>", '⇌'), ("<->", '↔'), ("->", '→'), ("<-", '←')];
    ARROWS.iter().find(|(a, _)| c.iter().take(a.len()).copied().eq(a.chars())).map(|(a, s)| (*s, a.len()))
}

/// `[…]` right after an arrow (its label), honoring braces and `$…$`.
pub(super) fn bracketed(c: &[char], i: &mut usize) -> Option<String> {
    if c.get(*i) != Some(&'[') {
        return None;
    }
    let mut level = 0;
    let mut math = false;
    let mut s = String::new();
    let mut k = *i + 1;
    while k < c.len() {
        match c[k] {
            '$' => math = !math,
            '{' => level += 1,
            '}' => level -= 1,
            ']' if level == 0 && !math => {
                *i = k + 1;
                return Some(s);
            }
            _ => {}
        }
        s.push(c[k]);
        k += 1;
    }
    None
}

/// Content of `{…}` starting at `c[*i] == '{'`, without the outer braces.
pub(super) fn braced(c: &[char], i: &mut usize) -> String {
    let mut level = 0;
    let mut s = String::new();
    while *i < c.len() {
        let ch = c[*i];
        *i += 1;
        match ch {
            '{' => {
                level += 1;
                if level == 1 {
                    continue;
                }
            }
            '}' => {
                level -= 1;
                if level == 0 {
                    break;
                }
            }
            _ => {}
        }
        s.push(ch);
    }
    s
}

/// A token of `\ce`: up to a space outside braces and `$…$`.
pub(super) fn chemical_token(c: &[char], i: &mut usize) -> String {
    let mut level = 0;
    let mut math = false;
    let mut s = String::new();
    while *i < c.len() {
        let ch = c[*i];
        if ch.is_whitespace() && level == 0 && !math {
            break;
        }
        match ch {
            '$' => math = !math,
            '{' => level += 1,
            '}' => level -= 1,
            _ => {}
        }
        s.push(ch);
        *i += 1;
    }
    s
}

/// Stoichiometric coefficient: 2, 0.5, 1/2 (as a small fraction).
pub(super) fn chemical_coefficient(c: &[char], i: &mut usize) -> Node {
    let digits = |i: &mut usize| -> String {
        let mut d = String::new();
        while *i < c.len() && c[*i].is_ascii_digit() {
            d.push(c[*i]);
            *i += 1;
        }
        d
    };
    let mut value = digits(i);
    if c.get(*i) == Some(&'.') && c.get(*i + 1).is_some_and(char::is_ascii_digit) {
        *i += 1;
        value.push('.');
        value.push_str(&digits(i));
    } else if c.get(*i) == Some(&'/') && c.get(*i + 1).is_some_and(char::is_ascii_digit) {
        *i += 1;
        let den = digits(i);
        let number = |v: String| Box::new(Node::Number { value: v, group: false, thousands: None });
        return Node::Fraction { num: number(value), den: number(den), style: FractionStyle::Row, rule: true, thickness: None };
    }
    Node::Number { value, group: false, thousands: None }
}

/// Charge or oxidation state: `2-` → 2−, `.-` → •− (radical), `II` → II.
pub(super) fn charge(payload: &str) -> Node {
    if !payload.is_empty() && payload.chars().all(|x| x.is_ascii_digit()) {
        return Node::Number { value: payload.to_string(), group: false, thousands: None };
    }
    let text: String = payload
        .chars()
        .map(|x| match x {
            '-' => '−',
            '.' | '*' => '•',
            other => other,
        })
        .collect();
    Node::Text { parts: vec![TextPart::Text(text)] }
}

/// Attaches a superscript and/or subscript to the last node of the formula; at the start of the formula
/// (an isotope's `^{14}_{6}C`) they go on an empty base.
pub(super) fn attach_script(children: &mut Vec<Node>, sup: Option<Node>, sub: Option<Node>) {
    // after a coefficient (3^{1}_{0}n) the scripts start a new base, not the space
    if matches!(children.last(), Some(Node::Space { .. })) {
        children.push(Node::row(vec![]));
    }
    let last_item = children.pop().unwrap_or(Node::row(vec![]));
    children.push(match last_item {
        Node::Scripts { base, sup: None, sub: s } if sup.is_some() => Node::Scripts { base, sup: sup.map(Box::new), sub: s.or(sub.map(Box::new)) },
        Node::Scripts { base, sup: s, sub: None } if sub.is_some() => Node::Scripts { base, sup: s, sub: sub.map(Box::new) },
        other => Node::Scripts { base: Box::new(other), sup: sup.map(Box::new), sub: sub.map(Box::new) },
    });
}

/// tensor package indices: `^\mu_\nu^\rho` → `^{\mu}{}_{\nu}{}^{\rho}`, each change of side on a new empty
/// base so the indices stagger. Before the base (`pre`), the empty base comes first.
pub(super) fn staggered_indices(raw: &str, pre: bool) -> String {
    let c: Vec<char> = raw.chars().collect();
    let mut groups: Vec<(char, String)> = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let side = c[i];
        i += 1;
        if !matches!(side, '^' | '_') {
            continue;
        }
        while c.get(i) == Some(&' ') {
            i += 1;
        }
        let index = match c.get(i) {
            Some('{') => braced(&c, &mut i),
            Some('\\') => {
                let mut s = String::from('\\');
                i += 1;
                while i < c.len() && c[i].is_ascii_alphabetic() {
                    s.push(c[i]);
                    i += 1;
                }
                if s.len() == 1 && i < c.len() {
                    s.push(c[i]);
                    i += 1;
                }
                s
            }
            Some(x) => {
                i += 1;
                x.to_string()
            }
            None => String::new(),
        };
        match groups.last_mut() {
            Some((last, content)) if *last == side => {
                content.push(' ');
                content.push_str(&index);
            }
            _ => groups.push((side, index)),
        }
    }
    let scripts: Vec<String> = groups.iter().map(|(side, content)| format!("{side}{{{content}}}")).collect();
    let joined = scripts.join("{}");
    if pre {
        format!("{{}}{joined}")
    } else {
        joined
    }
}

/// mhchem's `\pu` unit to math: `J mol^-1` → `J\,mol^{-1}`, `.` and `*` between units → `\cdot`.
pub(super) fn mhchem_unit(unit: &str) -> String {
    let c: Vec<char> = unit.chars().collect();
    let mut s = String::new();
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            '^' => {
                let mut exponent = String::new();
                i += 1;
                while i < c.len() && (c[i].is_ascii_digit() || (c[i] == '-' && exponent.is_empty())) {
                    exponent.push(c[i]);
                    i += 1;
                }
                s.push_str(&format!("^{{{exponent}}}"));
                continue;
            }
            '.' | '*' => s.push_str("\\cdot "),
            ' ' => s.push_str("\\,"),
            other => s.push(other),
        }
        i += 1;
    }
    s
}

fn upright_text(t: String) -> Node {
    Node::Text { parts: vec![TextPart::Text(t)] }
}

fn formula_element(formula: &mut Formula<'_>) -> Node {
    let mut el = formula.c[formula.i].to_string();
    formula.i += 1;
    while formula.i < formula.c.len() && formula.c[formula.i].is_ascii_lowercase() {
        el.push(formula.c[formula.i]);
        formula.i += 1;
    }
    upright_text(el)
}

fn formula_digits(formula: &mut Formula<'_>) {
    let mut d = String::new();
    while formula.i < formula.c.len() && formula.c[formula.i].is_ascii_digit() {
        d.push(formula.c[formula.i]);
        formula.i += 1;
    }
    attach_script(&mut formula.children, None, Some(Node::Number { value: d, group: false, thousands: None }));
}

fn formula_state(formula: &mut Formula<'_>) -> Option<String> {
    let found = ["(aq)", "(s)", "(l)", "(g)", "(sol)", "(cr)"].into_iter().find(|e| formula.c[formula.i..].iter().take(e.len()).copied().eq(e.chars()));
    found.map(|state| {
        formula.i += state.chars().count();
        state.to_string()
    })
}

fn formula_lowercase(formula: &mut Formula<'_>) -> Node {
    let mut t = String::new();
    while formula.i < formula.c.len() && formula.c[formula.i].is_alphabetic() && formula.c[formula.i].is_lowercase() {
        t.push(formula.c[formula.i]);
        formula.i += 1;
    }
    upright_text(t)
}
