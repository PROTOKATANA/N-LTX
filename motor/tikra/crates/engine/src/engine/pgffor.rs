//! \foreach: its list, its options, and one iteration.

use super::*;
use crate::tex::{self, Cat, Meaning, Token};
use crate::Error;

impl Engine {
    // ---------------------------------------------------------------- \foreach

    /// A pgfmath expression whose value is text: a quoted string (`"-- "`), or `ifthenelse` (or
    /// `?:`) choosing between two. `None` for a number.
    pub(super) fn math_text_value(&mut self, text: &str) -> Result<Option<String>, Error> {
        let t = text.trim();
        let t = t.strip_prefix('{').and_then(|x| x.strip_suffix('}')).unwrap_or(t).trim();
        if !t.contains('"') {
            return Ok(None);
        }
        if let Some(q) = t.strip_prefix('"').and_then(|x| x.strip_suffix('"')).filter(|q| !q.contains('"')) {
            return Ok(Some(q.to_string()));
        }
        // `ifthenelse(condition, a, b)`: its arguments at the commas outside quotes and parentheses
        // (the closing parenthesis may be missing at the end, as pgfmath lets it)
        let Some(args) = t.strip_prefix("ifthenelse").map(str::trim).and_then(|a| a.strip_prefix('(')).map(|a| a.strip_suffix(')').unwrap_or(a)) else { return Ok(None) };
        let mut parts = vec![String::new()];
        let (mut depth, mut quoted) = (0, false);
        for c in args.chars() {
            match c {
                '"' => quoted = !quoted,
                '(' if !quoted => depth += 1,
                ')' if !quoted => depth -= 1,
                ',' if !quoted && depth == 0 => {
                    parts.push(String::new());
                    continue;
                }
                _ => {}
            }
            parts.last_mut().unwrap().push(c);
        }
        let [cond, a, b] = &parts[..] else { return Ok(None) };
        let pick = if self.number(cond)? != 0.0 { a } else { b };
        Ok(Some(self.math_text_value(pick)?.unwrap_or_else(|| pick.trim().to_string())))
    }

    /// The list of a `\foreach`, for `foreach_items`: a macro that holds it, expanded; or a group,
    /// whose items stay as they are written (`{\a,\b}` is two items, whatever `\a` holds) but for
    /// the ends of a `...` range.
    pub fn foreach_list(&mut self) -> Result<Vec<Token>, Error> {
        let list = match self.tex.next_nonspace() {
            Some(Token::Char(_, Cat::Begin)) => self.tex.read_group_body()?,
            Some(t) => return self.tex.expand_fully(vec![t]),
            None => return Ok(Vec::new()),
        };
        let items = foreach_split(&list);
        let dots = |i: usize| items.get(i).is_some_and(|p| tex::tokens_to_string(p).trim() == "...");
        let mut out = Vec::new();
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                out.push(Token::other(','));
            }
            // `\a,\b,...,\c`: the step comes from the two before the dots
            if dots(i + 1) || dots(i + 2) || (i > 0 && dots(i - 1)) {
                out.extend(self.tex.expand_fully(item.clone())?);
            } else {
                out.extend(item.iter().cloned());
            }
        }
        Ok(out)
    }

    /// `\foreach \x/\y [count=\i, evaluate=\x as \v using …, remember=\x as \lastx (initially 0)] in {list} body`
    pub(super) fn foreach(&mut self) -> Result<(), Error> {
        // variables up to `[` or `in`
        let mut vars: Vec<Token> = Vec::new();
        let mut opts: Vec<Token> = Vec::new();
        loop {
            let Some(t) = self.tex.get_token() else { return Err(Error::Figure("\\foreach without 'in'".into())) };
            if t.is_char('[') {
                opts.extend(self.tex.read_until_char(']', false)?);
                continue;
            }
            if t.is_char('i') {
                if let Some(n) = self.tex.get_token() {
                    if n.is_char('n') {
                        break;
                    }
                    self.tex.back(n);
                }
            }
            vars.push(t);
            if vars.len() > 1000 {
                return Err(Error::Figure("\\foreach without 'in'".into()));
            }
        }
        let names: Vec<Token> = vars.iter().filter(|t| matches!(t, Token::Cs(_))).cloned().collect();
        let list = self.foreach_list()?;
        let items = foreach_items(&list);
        // options after the list too: \foreach \x in {…} [count=\i]
        if let Some(t) = self.tex.next_nonspace() {
            if t.is_char('[') {
                opts.extend(tex::string_tokens(","));
                opts.extend(self.tex.read_until_char(']', false)?);
            } else {
                self.tex.back(t);
            }
        }
        let body = self.next_statement()?;
        let o = foreach_options(&opts);
        if self.depth >= MAX_DEPTH {
            return Err(Error::Figure(format!("scope/\\foreach nesting went past {MAX_DEPTH} levels")));
        }
        self.depth += 1;
        // remembered values live across iterations
        let mut memory = o.initial_memory();
        for (k, item) in items.iter().enumerate() {
            self.count_step()?;
            self.begin_group();
            let parts = split_slash(item);
            for (i, name) in names.iter().enumerate() {
                let v = parts.get(i).or(parts.last()).cloned().unwrap_or_default();
                self.tex.set_meaning(name, macro_tokens(v), false);
            }
            self.foreach_iteration(&o, k, &memory)?;
            let r = self.run_tokens(body.clone());
            self.foreach_remember(&o, &mut memory);
            self.end_group();
            r?;
            // `\breakforeach`: this was the last iteration
            if matches!(self.tex.meaning_of("tikra@foreach@break"), Meaning::Macro(m) if !m.body.is_empty()) {
                self.tex.set_meaning(&Token::cs("tikra@foreach@break"), macro_tokens(Vec::new()), true);
                break;
            }
        }
        self.depth -= 1;
        Ok(())
    }

    /// What `\foreach`'s options set for iteration `k` (from 0): the count, the remembered values
    /// (`memory`, the last iteration's), the evaluated variables.
    pub fn foreach_iteration(&mut self, o: &ForeachOptions, k: usize, memory: &[Option<Vec<Token>>]) -> Result<(), Error> {
        if let Some((n, start)) = &o.count {
            self.tex.set_meaning(n, macro_from(&(start + k as i64).to_string()), false);
        }
        for ((_, as_name, _), m) in o.remember.iter().zip(memory) {
            if let Some(v) = m {
                self.tex.set_meaning(as_name, macro_tokens(v.clone()), false);
            }
        }
        for (var, target, using) in &o.evaluate {
            let expr = match using {
                Some(u) => u.clone(),
                None => vec![var.clone()],
            };
            let dest = target.clone().unwrap_or_else(|| var.clone());
            let text = self.tex.expand_to_string(expr.clone())?;
            if let Some(s) = self.math_text_value(&text)? {
                self.tex.set_meaning(&dest, macro_tokens(tikra_tokens(&s)), false);
                continue;
            }
            let v = self.number_of(expr)?;
            self.tex.set_meaning(&dest, macro_from(&pgf_number(v)), false);
        }
        Ok(())
    }

    /// The values to remember, after an iteration.
    pub fn foreach_remember(&self, o: &ForeachOptions, memory: &mut [Option<Vec<Token>>]) {
        for (i, (var, _, _)) in o.remember.iter().enumerate() {
            if let Meaning::Macro(m) = self.tex.meaning(var) {
                memory[i] = Some(tex::substitute(&m.body, &[]));
            }
        }
    }
}

/// `\foreach`'s options: `count=\i from 1`, `remember=\x as \last (initially v)`, `evaluate=\x as
/// \v using expr` (or `evaluate={\v=expr;}`).
#[derive(Default)]
pub struct ForeachOptions {
    count: Option<(Token, i64)>,
    remember: Vec<(Token, Token, Option<Vec<Token>>)>,
    evaluate: Vec<(Token, Option<Token>, Option<Vec<Token>>)>,
}

impl ForeachOptions {
    /// What the remembered variables start with (`initially`).
    pub fn initial_memory(&self) -> Vec<Option<Vec<Token>>> {
        self.remember.iter().map(|r| r.2.clone()).collect()
    }
}

/// The options of a `\foreach` (between its brackets).
pub fn foreach_options(opts: &[Token]) -> ForeachOptions {
    let options = crate::keys::split_list(opts);
    let mut count: Option<(Token, i64)> = None;
    let mut remember: Vec<(Token, Token, Option<Vec<Token>>)> = Vec::new();
    let mut evaluate: Vec<(Token, Option<Token>, Option<Vec<Token>>)> = Vec::new();
    for o in &options {
        let v = o.value.clone().unwrap_or_default();
        let parts: Vec<Token> = v.iter().filter(|t| !t.is_space()).cloned().collect();
        match o.key.as_str() {
            "count" => {
                // count=\i from 0
                let name = parts.first().cloned();
                let s = tex::tokens_to_string(&v);
                let start = s.split("from").nth(1).and_then(|x| x.trim().parse().ok()).unwrap_or(1);
                if let Some(n) = name {
                    count = Some((n, start));
                }
            }
            "remember" => {
                let s = tex::tokens_to_string(&v);
                let initially = s.split("initially").nth(1).map(|x| tex::string_tokens(x.trim().trim_start_matches('(').trim_end_matches(')')));
                let cs: Vec<Token> = v.iter().filter(|t| matches!(t, Token::Cs(_))).cloned().collect();
                if cs.len() >= 2 {
                    remember.push((cs[0].clone(), cs[1].clone(), initially));
                }
            }
            // `evaluate={\c=\x*10;}`: assignments, each a variable and its expression
            "evaluate" if !tex::tokens_to_string(&v).contains(" as ") && tex::tokens_to_string(&v).contains('=') => {
                let s = tex::tokens_to_string(&v);
                for statement in s.split(';') {
                    if let Some((name, expr)) = statement.split_once('=') {
                        if let Some(var) = tikra_tokens(name.trim()).into_iter().find(|t| matches!(t, Token::Cs(_))) {
                            evaluate.push((var.clone(), Some(var), Some(tikra_tokens(expr.trim()))));
                        }
                    }
                }
            }
            "evaluate" => {
                let s = tex::tokens_to_string(&v);
                let cs: Vec<Token> = v.iter().filter(|t| matches!(t, Token::Cs(_))).cloned().collect();
                let using = s.split("using").nth(1).map(|x| tikra_tokens(x.trim()));
                let target = if s.contains(" as ") { cs.get(1).cloned() } else { None };
                if let Some(var) = cs.first() {
                    evaluate.push((var.clone(), target, using));
                }
            }
            "parse" => {}
            _ => {}
        }
    }
    ForeachOptions { count, remember, evaluate }
}

/// A `\foreach` list cut at its commas, the items as they are written.
fn foreach_split(list: &[Token]) -> Vec<Vec<Token>> {
    let mut raw: Vec<Vec<Token>> = Vec::new();
    let mut current: Vec<Token> = Vec::new();
    let mut depth = 0usize;
    // an item that starts with `(` runs to its `)` (`(0,0), (1,1)`), as pgffor reads it
    let mut parens = 0usize;
    for t in list {
        match t {
            Token::Char(_, Cat::Begin) => depth += 1,
            Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
            Token::Char('(', _) if depth == 0 && (parens > 0 || current.iter().all(Token::is_space)) => parens += 1,
            Token::Char(')', _) if depth == 0 && parens > 0 => parens -= 1,
            Token::Char(',', _) if depth == 0 && parens == 0 => {
                raw.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(t.clone());
    }
    raw.push(current);
    raw
}

/// The items of a `\foreach` list, with `...` ranges expanded as pgffor does (numbers and letters).
pub fn foreach_items(list: &[Token]) -> Vec<Vec<Token>> {
    let raw: Vec<Vec<Token>> = foreach_split(list).into_iter().map(|p| tex::strip_outer_braces(crate::keys::trim(&p).to_vec())).filter(|p| !p.is_empty()).collect();
    let text = |v: &[Token]| tex::tokens_to_string(v).trim().to_string();
    let mut out: Vec<Vec<Token>> = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        if text(&raw[i]) == "..." && !out.is_empty() && i + 1 < raw.len() {
            let end_s = text(&raw[i + 1]);
            let last_s = text(&out[out.len() - 1]);
            let before = if out.len() >= 2 { Some(text(&out[out.len() - 2])) } else { None };
            // letters: a,...,e
            let letter = |s: &str| if s.chars().count() == 1 && s.chars().all(|c| c.is_ascii_alphabetic()) { s.chars().next() } else { None };
            if let (Some(a), Some(b)) = (letter(&last_s), letter(&end_s)) {
                let step: i32 = before.as_deref().and_then(letter).map_or(1, |p| a as i32 - p as i32).max(1);
                let mut c = a as i32 + step;
                while c <= b as i32 && out.len() < 100_000 {
                    out.push(tex::string_tokens(&char::from_u32(c as u32).unwrap_or('?').to_string()));
                    c += step;
                }
                i += 2;
                continue;
            }
            let (Ok(last), Ok(end)) = (last_s.parse::<f64>(), end_s.parse::<f64>()) else {
                i += 1;
                continue;
            };
            let step = before.and_then(|b| b.parse::<f64>().ok()).map_or(if end >= last { 1.0 } else { -1.0 }, |b| last - b);
            if step.abs() > 1e-12 {
                let mut k = 1.0;
                loop {
                    let v = last + step * k;
                    if (step > 0.0 && v > end + 1e-9) || (step < 0.0 && v < end - 1e-9) || out.len() > 100_000 {
                        break;
                    }
                    out.push(tex::string_tokens(&pgf_number(v)));
                    k += 1.0;
                }
            }
            i += 2;
            continue;
        }
        out.push(raw[i].clone());
        i += 1;
    }
    out
}
