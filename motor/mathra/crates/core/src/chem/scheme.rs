//! The reaction scheme reader (L-CHE-2): `\schemestart … \schemestop` with `\arrow`, `\+`,
//! `\chemname`, `\subscheme` and `\chemmove`, and the compounds in it (`\chemfig{…}`, `\ce{…}`,
//! `$…$`, loose formulas). The LaTeX reader hands it the whole construction as written.

use super::chemfig;
use super::model::*;
use crate::rules::{self, Registry};
use crate::Error;

pub type R<T> = Result<T, Error>;

/// Shared by the chemistry readers: strict mode, the warnings of tolerant mode, and the column of the
/// command the construction started at (errors point there).
pub struct Reader<'w> {
    pub strict: bool,
    pub registry: &'w mut Registry,
    pub column: usize,
}

impl Reader<'_> {
    /// In strict mode, an error; in tolerant mode, a warning, and the caller's recovery goes on.
    pub fn tolerate(&mut self, rule: &'static str, problem: impl Into<String>, recovery: &str) -> R<()> {
        let problem = problem.into();
        if self.strict {
            return Err(Error::Syntax { column: self.column, message: problem });
        }
        self.registry.warn(rule, format!("column {}: {problem}; {recovery}", self.column));
        Ok(())
    }
}

/// LaTeX text (arrow labels, compound names) as math for mathra: `$…$` stays math, `\ce{…}` and other
/// commands are math as in LaTeX's text mode, and the words around them go in `\text{…}`.
pub fn text_to_tex(t: &str) -> String {
    let c: Vec<char> = t.trim().chars().collect();
    let mut out = String::new();
    let mut words = String::new();
    let flush = |words: &mut String, out: &mut String| {
        if !words.is_empty() {
            out.push_str(&format!("\\text{{{words}}}"));
            words.clear();
        }
    };
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            '$' => {
                flush(&mut words, &mut out);
                i += 1;
                while i < c.len() && c[i] != '$' {
                    out.push(c[i]);
                    i += 1;
                }
                i += 1;
            }
            '\\' if c.get(i + 1).is_some_and(|n| n.is_ascii_alphabetic()) => {
                flush(&mut words, &mut out);
                out.push('\\');
                i += 1;
                while i < c.len() && c[i].is_ascii_alphabetic() {
                    out.push(c[i]);
                    i += 1;
                }
                // the command's braced arguments go with it
                while i < c.len() && c[i] == '{' {
                    let mut level = 0;
                    while i < c.len() {
                        out.push(c[i]);
                        match c[i] {
                            '{' => level += 1,
                            '}' => {
                                level -= 1;
                                if level == 0 {
                                    i += 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                }
                out.push(' ');
            }
            ch => {
                words.push(ch);
                i += 1;
            }
        }
    }
    flush(&mut words, &mut out);
    out.trim_end().to_string()
}

/// Splits at `sep` outside braces and brackets.
pub fn split_top(s: &str, sep: char) -> Vec<String> {
    let mut parts = vec![String::new()];
    let mut level = 0i32;
    for c in s.chars() {
        match c {
            '{' | '[' | '(' => level += 1,
            '}' | ']' | ')' => level -= 1,
            x if x == sep && level == 0 => {
                parts.push(String::new());
                continue;
            }
            _ => {}
        }
        parts.last_mut().expect("never empty").push(c);
    }
    parts
}

/// Reads a scheme, or a single `\chemfig{…}`, as the LaTeX reader found it.
pub fn read(src: &str, reader: &mut Reader) -> R<Scheme> {
    let mut s = SchemeReader { c: src.trim().chars().collect(), pos: 0, reader, scheme: Scheme { items: vec![], moves: vec![] } };
    s.items()?;
    let mut scheme = s.scheme;
    // a mechanism arrow must point at hooked atoms
    let hooks: Vec<String> = scheme
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Compound(Compound { body: Body::Structure(m), .. }) => Some(m),
            _ => None,
        })
        .flat_map(|m| m.atoms.iter().filter_map(|a| a.hook.clone()))
        .collect();
    let mut kept = Vec::new();
    for m in std::mem::take(&mut scheme.moves) {
        let missing = [&m.from, &m.to].into_iter().find(|h| !hooks.contains(h)).cloned();
        match missing {
            Some(missing) => reader.tolerate(rules::E_REC_9, format!("no atom is hooked as @{{{missing}}}"), "arrow left out")?,
            None => kept.push(m),
        }
    }
    scheme.moves = kept;
    Ok(scheme)
}

struct SchemeReader<'r, 'w> {
    c: Vec<char>,
    pos: usize,
    reader: &'r mut Reader<'w>,
    scheme: Scheme,
}

impl SchemeReader<'_, '_> {
    fn current(&self) -> Option<char> {
        self.c.get(self.pos).copied()
    }

    fn skip_spaces(&mut self) {
        while self.current().is_some_and(char::is_whitespace) {
            self.pos += 1;
        }
    }

    fn group(&mut self, open: char, close: char) -> R<String> {
        let mut level = 0;
        let mut s = String::new();
        while let Some(ch) = self.current() {
            self.pos += 1;
            if ch == '\\' && level > 0 {
                s.push(ch);
                if let Some(n) = self.current() {
                    s.push(n);
                    self.pos += 1;
                }
                continue;
            }
            if ch == open {
                level += 1;
                if level == 1 {
                    continue;
                }
            } else if ch == close {
                level -= 1;
                if level == 0 {
                    return Ok(s);
                }
            }
            s.push(ch);
        }
        self.reader.tolerate(rules::E_REC_4, format!("'{open}' without a matching '{close}'"), "closed at the end")?;
        Ok(s)
    }

    fn braced(&mut self) -> R<String> {
        self.skip_spaces();
        if self.current() == Some('{') {
            self.group('{', '}')
        } else {
            self.reader.tolerate(rules::E_REC_4, "missing braced argument", "left empty")?;
            Ok(String::new())
        }
    }

    fn optional(&mut self) -> R<Option<String>> {
        let saved = self.pos;
        self.skip_spaces();
        if self.current() == Some('[') {
            return Ok(Some(self.group('[', ']')?));
        }
        self.pos = saved;
        Ok(None)
    }

    fn command(&mut self) -> String {
        self.pos += 1;
        let mut name = String::new();
        while let Some(l) = self.current().filter(char::is_ascii_alphabetic) {
            name.push(l);
            self.pos += 1;
        }
        if name.is_empty() {
            if let Some(sym) = self.current() {
                name.push(sym);
                self.pos += 1;
            }
        }
        name
    }

    fn compound(&mut self, body: Body, name: Option<String>) {
        self.scheme.items.push(Item::Compound(Compound { body, name }));
    }

    fn items(&mut self) -> R<()> {
        let mut text = String::new();
        while let Some(ch) = self.current() {
            if ch != '\\' && ch != '$' {
                text.push(ch);
                self.pos += 1;
                continue;
            }
            self.flush(&mut text);
            if ch == '$' {
                self.pos += 1;
                let mut math = String::new();
                while let Some(m) = self.current() {
                    self.pos += 1;
                    if m == '$' {
                        break;
                    }
                    math.push(m);
                }
                self.compound(Body::Math { tex: math.trim().to_string() }, None);
                continue;
            }
            let start = self.pos;
            let name = self.command();
            match name.as_str() {
                "schemestart" | "schemestop" | "par" => {}
                "chemfig" => {
                    let _ = self.optional()?;
                    let body = self.braced()?;
                    let m = chemfig::molecule(&body, self.reader)?;
                    self.compound(Body::Structure(m), None);
                }
                "ce" => {
                    let body = self.braced()?;
                    self.compound(Body::Math { tex: format!("\\ce{{{body}}}") }, None);
                }
                "chemname" => {
                    if self.current() == Some('*') {
                        self.pos += 1;
                    }
                    let _ = self.optional()?;
                    let inner = self.braced()?;
                    let label = self.braced()?;
                    let before = self.scheme.items.len();
                    let mut sub = SchemeReader { c: inner.chars().collect(), pos: 0, reader: self.reader, scheme: Scheme { items: vec![], moves: vec![] } };
                    sub.items()?;
                    let (items, moves) = (sub.scheme.items, sub.scheme.moves);
                    self.scheme.moves.extend(moves);
                    self.scheme.items.extend(items);
                    match self.scheme.items.get_mut(before) {
                        Some(Item::Compound(c)) => c.name = Some(label),
                        _ => self.compound(Body::Math { tex: String::new() }, Some(label)),
                    }
                }
                "+" => {
                    // \+{space before, space after, vertical shift}: the spacing is mathra's
                    if self.current() == Some('{') {
                        let _ = self.group('{', '}')?;
                    }
                    self.scheme.items.push(Item::Plus);
                }
                "arrow" => self.arrow()?,
                "chemmove" => {
                    let _ = self.optional()?;
                    let body = self.braced()?;
                    self.moves(&body)?;
                }
                "subscheme" => {
                    let _ = self.optional()?;
                    let body = self.braced()?;
                    let mut sub = SchemeReader { c: body.chars().collect(), pos: 0, reader: self.reader, scheme: Scheme { items: vec![], moves: vec![] } };
                    sub.items()?;
                    let (items, moves) = (sub.scheme.items, sub.scheme.moves);
                    self.scheme.items.extend(items);
                    self.scheme.moves.extend(moves);
                }
                "\\" | "," | ";" | "quad" | "qquad" | " " => {}
                _ => {
                    // any other command is math for mathra (\Delta, \text…), up to the next scheme command
                    self.pos = start;
                    let mut math = String::new();
                    while let Some(m) = self.current() {
                        if m == '\\' && self.starts_scheme_command() && !math.is_empty() {
                            break;
                        }
                        if m == '$' {
                            break;
                        }
                        math.push(m);
                        self.pos += 1;
                        if m == '\\' {
                            while let Some(l) = self.current().filter(char::is_ascii_alphabetic) {
                                math.push(l);
                                self.pos += 1;
                            }
                        }
                    }
                    self.compound(Body::Math { tex: math.trim().to_string() }, None);
                }
            }
        }
        self.flush(&mut text);
        Ok(())
    }

    /// The command at the current position is one of the scheme's own.
    fn starts_scheme_command(&self) -> bool {
        let rest = &self.c[self.pos + 1..];
        if rest.first() == Some(&'+') {
            return true;
        }
        let name: String = rest.iter().take_while(|c| c.is_ascii_alphabetic()).collect();
        matches!(name.as_str(), "chemfig" | "schemestart" | "schemestop" | "ce" | "arrow" | "chemname" | "chemmove" | "subscheme")
    }

    /// Loose text between commands is a formula for mhchem: `A`, `2 H2O`.
    fn flush(&mut self, text: &mut String) {
        let t = text.trim();
        if !t.is_empty() {
            let tex = format!("\\ce{{{t}}}");
            self.compound(Body::Math { tex }, None);
        }
        text.clear();
    }

    /// `\arrow{type[above][below]}[angle,length]`, or a bare `\arrow` (a plain arrow).
    fn arrow(&mut self) -> R<()> {
        self.skip_spaces();
        let spec = if self.current() == Some('{') { self.group('{', '}')? } else { String::new() };
        let spec = spec.trim();
        let type_end = spec.find('[').unwrap_or(spec.len());
        let (kind_text, rest) = spec.split_at(type_end);
        let kind = match kind_text.trim() {
            "" | "->" => ArrowKind::Forward,
            "<-" => ArrowKind::Backward,
            "<=>" => ArrowKind::Equilibrium,
            "<->>" => ArrowKind::EquilibriumRight,
            "<<->" => ArrowKind::EquilibriumLeft,
            "<->" => ArrowKind::Resonance,
            "=>" => ArrowKind::Retro,
            "-/>" => ArrowKind::NoReaction,
            "0" => ArrowKind::Invisible,
            other => {
                self.reader.tolerate(rules::E_REC_1, format!("unknown arrow type: {other}"), "drawn as ->")?;
                ArrowKind::Forward
            }
        };
        let mut labels = Vec::new();
        let mut sub = SchemeReader { c: rest.chars().collect(), pos: 0, reader: self.reader, scheme: Scheme { items: vec![], moves: vec![] } };
        while let Some(text) = sub.optional()? {
            labels.push(text);
        }
        let mut label = |text: Option<&String>| -> R<Option<Label>> {
            let Some(t) = text.map(|t| t.trim()).filter(|t| !t.is_empty()) else { return Ok(None) };
            if let Some(start) = t.find("\\chemfig") {
                let mut r = SchemeReader { c: t[start + "\\chemfig".len()..].chars().collect(), pos: 0, reader: self.reader, scheme: Scheme { items: vec![], moves: vec![] } };
                let _ = r.optional()?;
                let body = r.braced()?;
                return Ok(Some(Label::Structure(chemfig::molecule(&body, self.reader)?)));
            }
            Ok(Some(Label::Text { tex: t.to_string() }))
        };
        let above = label(labels.first())?;
        let below = label(labels.get(1))?;
        let (mut angle, mut length) = (0.0, 1.0);
        if let Some(opts) = self.optional()? {
            let parts = split_top(&opts, ',');
            if let Some(a) = parts.first().and_then(|a| a.trim().parse::<f32>().ok()) {
                angle = a;
            }
            if let Some(l) = parts.get(1).and_then(|l| l.trim().parse::<f32>().ok()).filter(|l| *l > 0.0) {
                length = l;
            }
        }
        self.scheme.items.push(Item::Arrow(Arrow { kind, above, below, angle, length }));
        Ok(())
    }

    /// The `\draw` paths of a `\chemmove`, between hooks: `(a)..controls +(90:1cm) and +(45:5mm)..(b)`,
    /// `(a) to[out=90,in=45] (b)`, `(a) to[bend left] (b)`.
    fn moves(&mut self, body: &str) -> R<()> {
        for statement in body.split(';') {
            let st = statement.trim();
            if st.is_empty() {
                continue;
            }
            let Some(path) = st.strip_prefix("\\draw") else {
                self.reader.tolerate(rules::E_REC_1, format!("only \\draw is read inside \\chemmove: {st}"), "ignored")?;
                continue;
            };
            let mut p = SchemeReader { c: path.chars().collect(), pos: 0, reader: self.reader, scheme: Scheme { items: vec![], moves: vec![] } };
            let options = p.optional()?.unwrap_or_default();
            let single = ["harpoon", "left to", "right to", "fishhook", "half"].iter().any(|k| options.contains(k));
            let names: Vec<String> = parens(path).into_iter().filter(|n| !n.contains(':')).collect();
            if names.len() < 2 {
                self.reader.tolerate(rules::E_REC_9, format!("an electron arrow needs two hooks: {st}"), "ignored")?;
                continue;
            }
            let controls: Vec<(f32, f32)> = parens(path).iter().filter_map(|n| polar(n)).collect();
            let mut m = ElectronMove {
                from: names[0].split('.').next().unwrap_or("").to_string(),
                to: names[names.len() - 1].split('.').next().unwrap_or("").to_string(),
                out_angle: None,
                in_angle: None,
                reach: None,
                bend: 45.0,
                single,
            };
            if controls.len() >= 2 {
                m.out_angle = Some(controls[0].0);
                m.in_angle = Some(controls[1].0);
                m.reach = Some((controls[0].1, controls[1].1));
            }
            let key = |k: &str| -> Option<f32> {
                let at = path.find(k)?;
                let rest = &path[at + k.len()..];
                let end = rest.find([',', ']']).unwrap_or(rest.len());
                rest[..end].trim().parse().ok()
            };
            if let (Some(o), Some(i)) = (key("out="), key("in=")) {
                m.out_angle = Some(o);
                m.in_angle = Some(i);
            }
            if path.contains("bend right") {
                m.bend = -key("bend right=").unwrap_or(45.0);
            } else if path.contains("bend left") {
                m.bend = key("bend left=").unwrap_or(45.0);
            }
            self.scheme.moves.push(m);
        }
        Ok(())
    }
}

/// The contents of each `( … )` in a TikZ path.
fn parens(path: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut level = 0;
    let mut s = String::new();
    for c in path.chars() {
        match c {
            '(' => {
                level += 1;
                if level == 1 {
                    s.clear();
                    continue;
                }
            }
            ')' => {
                level -= 1;
                if level == 0 {
                    out.push(s.trim().to_string());
                    continue;
                }
            }
            _ => {}
        }
        if level > 0 {
            s.push(c);
        }
    }
    out
}

/// `90:1cm` as (90°, 2.845 em).
fn polar(s: &str) -> Option<(f32, f32)> {
    let (a, l) = s.split_once(':')?;
    Some((a.trim().parse().ok()?, length_em(l.trim())?))
}

/// A TeX length in em, with the em of 10 pt type: 1 cm = 2.845 em, 1 pt = 0.1 em.
pub fn length_em(s: &str) -> Option<f32> {
    let split = s.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(s.len());
    let (n, unit) = s.split_at(split);
    let n: f32 = n.trim().parse().ok()?;
    let factor = match unit.trim() {
        "cm" => 2.845,
        "mm" => 0.2845,
        "pt" | "bp" => 0.1,
        "in" => 7.227,
        "ex" => 0.43,
        "em" | "" => 1.0,
        _ => return None,
    };
    Some(n * factor)
}
