//! The math library: `\tikzmath{…}` and the `evaluate={…}` key run statements ended by `;` —
//! declarations (`integer \i, \j;`, `real \x;`, `coordinate \c;`), assignments (`\x = 2*3;`,
//! `\a{\i,\j} = \i+\j;`, `\x1 = 4;`, `\c = (1,2)+(30:1);`), `let \x = text;`, `for \i in {list} {…};`,
//! `if cond then {…} else {…};` and `{code};` (TikZ code, run as it is). A variable is a macro
//! holding its value; one assigned with an index takes the index as its argument, and a coordinate
//! also sets `\cx` and `\cy` (in pt).

use std::collections::HashMap;

use tikra_engine::engine::piece::{CommandFn, Piece};
use tikra_engine::engine::{foreach_items, macro_from, macro_tokens, pgf_number, tikra_tokens, Engine, NATIVE_KEY_BASE};
use tikra_engine::keys::{self, macro_name};
use tikra_engine::tex::{self, Cat, Meaning, Token};
use tikra_engine::{expr, Error, PT};

/// The math library.
pub static PIECE: Piece = Piece { name: "tikzmath", commands: COMMANDS, define, ..Piece::EMPTY };

const COMMANDS: &[(&str, CommandFn)] = &[("tikzmath", tikzmath)];

/// What a declaration made a variable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Real,
    Integer,
    Coordinate,
}

type Kinds = HashMap<String, Kind>;

/// The statements of `body`, split at the `;` outside braces.
fn statements(body: &[Token]) -> Vec<Vec<Token>> {
    let mut out = vec![Vec::new()];
    let mut depth = 0usize;
    for t in body {
        match t {
            Token::Char(_, Cat::Begin) => depth += 1,
            Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
            Token::Char(';', _) if depth == 0 => {
                out.push(Vec::new());
                continue;
            }
            _ => {}
        }
        out.last_mut().expect("a statement").push(t.clone());
    }
    out.into_iter().map(|s| keys::trim(&s).to_vec()).filter(|s| !s.is_empty()).collect()
}

/// The leading word of a statement (`integer`, `for`…), and the rest.
fn first_word(v: &[Token]) -> (String, &[Token]) {
    let n = v.iter().take_while(|t| matches!(t, Token::Char(_, Cat::Letter))).count();
    (v[..n].iter().filter_map(|t| if let Token::Char(c, _) = t { Some(*c) } else { None }).collect(), &v[n..])
}

/// A braced group at the start of `v` (spaces skipped): its contents and the rest.
fn group(v: &[Token]) -> Option<(Vec<Token>, &[Token])> {
    let v = keys::trim(v);
    if !matches!(v.first(), Some(Token::Char(_, Cat::Begin))) {
        return None;
    }
    let mut depth = 0usize;
    for (i, t) in v.iter().enumerate() {
        match t {
            Token::Char(_, Cat::Begin) => depth += 1,
            Token::Char(_, Cat::End) => {
                depth -= 1;
                if depth == 0 {
                    return Some((v[1..i].to_vec(), &v[i + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

/// Where the word `word` is at the top level of `v`.
fn find_word(v: &[Token], word: &str) -> Option<usize> {
    let chars: Vec<char> = word.chars().collect();
    let mut depth = 0usize;
    for i in 0..v.len() {
        match &v[i] {
            Token::Char(_, Cat::Begin) => depth += 1,
            Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
            _ if depth == 0 && i + chars.len() <= v.len() => {
                let at_word_start = i == 0 || !matches!(v[i - 1], Token::Char(_, Cat::Letter));
                let matches = chars.iter().enumerate().all(|(k, c)| v[i + k].is_char(*c));
                let ends = !matches!(v.get(i + chars.len()), Some(Token::Char(_, Cat::Letter)));
                if at_word_start && matches && ends {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// `\tikzmath{statements}`.
fn tikzmath(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let body = e.argument()?;
    math_statements(e, &body)
}

fn math_statements(e: &mut Engine, body: &[Token]) -> Result<(), Error> {
    for s in statements(body) {
        math_statement(e, &s)?;
    }
    Ok(())
}

/// `/tikz/evaluate={…}`: the same statements as `\tikzmath`.
fn k_evaluate(e: &mut Engine, v: Option<Vec<Token>>) -> Result<(), Error> {
    math_statements(e, &v.unwrap_or_default())
}

fn define(e: &mut Engine) {
    let id = NATIVE_KEY_BASE + e.plug.key_fns.len() as u16;
    e.plug.key_fns.push(k_evaluate);
    e.tex.define(&macro_name("/tikz/evaluate/.@cmd"), Meaning::Native(id));
}

fn math_statement(e: &mut Engine, s: &[Token]) -> Result<(), Error> {
    // `{ \draw …; };`: TeX (TikZ) code, run as it is
    if let Some((code, after)) = group(s) {
        if keys::trim(after).is_empty() {
            return e.run_tokens(code);
        }
    }
    let (word, rest) = first_word(s);
    match word.as_str() {
        "int" | "integer" | "real" | "coordinate" => {
            let kind = match word.as_str() {
                "real" => Kind::Real,
                "coordinate" => Kind::Coordinate,
                _ => Kind::Integer,
            };
            for t in rest {
                if let Token::Cs(n) = t {
                    e.piece_state::<Kinds>().insert(n.to_string(), kind);
                }
            }
            Ok(())
        }
        "let" => {
            let Some(eq) = rest.iter().position(|t| t.is_char('=')) else { return Err(Error::Figure("\\tikzmath: let without '='".into())) };
            let value = e.tex.expand_fully(keys::trim(&rest[eq + 1..]).to_vec())?;
            math_set(e, &rest[..eq], tex::tokens_to_string(&value).trim(), None)
        }
        "for" => {
            let Some(i) = find_word(rest, "in") else { return Err(Error::Figure("\\tikzmath: for without 'in'".into())) };
            let var = rest[..i].iter().find(|t| matches!(t, Token::Cs(_))).cloned().ok_or_else(|| Error::Figure("\\tikzmath: for without a variable".into()))?;
            let Some((list, after)) = group(&rest[i + 2..]) else { return Err(Error::Figure("\\tikzmath: for without a list".into())) };
            let Some((body, _)) = group(after) else { return Err(Error::Figure("\\tikzmath: for without a body".into())) };
            let list = e.tex.expand_fully(list)?;
            for item in foreach_items(&list) {
                e.tex.set_meaning(&var, macro_tokens(item), false);
                math_statements(e, &body)?;
            }
            Ok(())
        }
        "if" => {
            let Some(then) = find_word(rest, "then") else { return Err(Error::Figure("\\tikzmath: if without 'then'".into())) };
            let text = e.math_string(rest[..then].to_vec())?;
            let condition = expr::eval(&text, &e.vars())? != 0.0;
            let Some((yes, after)) = group(&rest[then + 4..]) else { return Err(Error::Figure("\\tikzmath: if without a block".into())) };
            let no = match find_word(after, "else") {
                Some(k) => group(&after[k + 4..]).map(|(b, _)| b),
                None => None,
            };
            match (condition, no) {
                (true, _) => math_statements(e, &yes),
                (false, Some(no)) => math_statements(e, &no),
                (false, None) => Ok(()),
            }
        }
        // what print typesets is text outside a picture; functions are not supported yet
        "print" => Ok(()),
        "function" => {
            e.warn("\\tikzmath: function not supported: ignored");
            Ok(())
        }
        _ => {
            let Some(eq) = s.iter().position(|t| t.is_char('=')) else { return Err(Error::Figure(format!("\\tikzmath: '{}' is not a statement", tex::tokens_to_string(s)))) };
            math_assign(e, &s[..eq], &s[eq + 1..])
        }
    }
}

/// `\x = expr`, `\a{i} = expr`, `\x1 = expr`.
fn math_assign(e: &mut Engine, target: &[Token], expr: &[Token]) -> Result<(), Error> {
    let Some(Token::Cs(name)) = target.iter().find(|t| !t.is_space()) else { return Err(Error::Figure("\\tikzmath: an assignment needs a variable".into())) };
    let kind = e.piece_state::<Kinds>().get(&**name).copied().unwrap_or(Kind::Real);
    let text = e.math_string(keys::trim(expr).to_vec())?;
    if kind == Kind::Coordinate {
        let t = text.trim();
        let p = if t.starts_with('(') && t.ends_with(')') && t[1..t.len() - 1].find(['(', ')']).is_none() {
            e.coordinate(&t[1..t.len() - 1])?.p
        } else {
            e.coordinate(&format!("${t}$"))?.p
        };
        let (x, y) = (p.x / PT, p.y / PT);
        math_set(e, target, &format!("tikra cs:{x}pt,{y}pt"), Some((x, y)))
    } else {
        let (v, unit) = tikra_engine::expr::eval_length(&text, &e.vars())?;
        let v = if unit { v / PT } else { v };
        let v = if kind == Kind::Integer { v.trunc() } else { v };
        math_set(e, target, &pgf_number(v), None)
    }
}

/// The variable of `target` (with its index, if any) set to `value`; a coordinate's x and y too.
fn math_set(e: &mut Engine, target: &[Token], value: &str, xy: Option<(f64, f64)>) -> Result<(), Error> {
    let target = keys::trim(target);
    let Some(Token::Cs(name)) = target.first() else { return Err(Error::Figure("\\tikzmath: an assignment needs a variable".into())) };
    let name = name.to_string();
    let index = &target[1..];
    let mut values = vec![(name.clone(), value.to_string())];
    if let Some((x, y)) = xy {
        values.push((format!("{name}x"), format!("{x}pt")));
        values.push((format!("{name}y"), format!("{y}pt")));
    }
    if index.is_empty() {
        for (n, v) in values {
            e.tex.set_meaning(&Token::cs(&n), macro_from(&v), false);
        }
        return Ok(());
    }
    // \a{\i,\j} or \x1: the variable takes the index as its argument
    let index = match group(index) {
        Some((g, _)) => g,
        None => index.to_vec(),
    };
    let key = e.tex.expand_to_string(index)?.replace(' ', "");
    for (n, v) in values {
        e.tex.set_meaning(&Token::cs(&format!("tikra@math@{n}@{key}")), macro_from(&v), false);
        e.run_tokens(tikra_tokens(&format!("\\def\\{n}#1{{\\csname tikra@math@{n}@#1\\endcsname}}")))?;
    }
    Ok(())
}
