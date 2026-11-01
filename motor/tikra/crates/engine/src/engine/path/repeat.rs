use crate::keys;
use crate::tex::{self, Cat, Token};
use crate::Error;

use super::super::{macro_from, tikra_tokens, Engine};
use super::*;

/// `let \p1 = (a), \n1 = {expr} in`: defines `\p1`, `\x1`, `\y1`, `\n1` for the rest of the path.
pub(super) fn let_operation(e: &mut Engine, depth: Option<usize>) -> Result<(), Error> {
    loop {
        // \p and \n are read as they are: they are macros once a first one is defined
        let t = loop {
            let t = match depth {
                Some(d) => e.tex.get_token_until(d),
                None => e.tex.get_token(),
            };
            match t {
                Some(t) if t.is_space() => continue,
                other => break other,
            }
        };
        let Some(t) = t else { return Ok(()) };
        let name = match &t {
            Token::Cs(n) => n.to_string(),
            Token::Char('i', Cat::Letter) => {
                let w = word(e, 'i');
                if w == "in" {
                    return Ok(());
                }
                return Err(Error::Figure(format!("let: '{w}' unexpected")));
            }
            Token::Char(',', _) => continue,
            other => return Err(Error::Figure(format!("let: '{}' unexpected", tex::show_token(other)))),
        };
        // the number after \p, \n: \p1, \p{name} (expanded, as a \csname makes it)
        let index = match e.tex.get_token() {
            Some(Token::Char(_, Cat::Begin)) => {
                let name = e.tex.read_group_body()?;
                e.tex.expand_to_string(name)?
            }
            Some(Token::Char(c, _)) => c.to_string(),
            _ => String::new(),
        };
        match next(e, depth)? {
            Some(t) if t.is_char('=') => {}
            _ => return Err(Error::Figure("let: '=' missing".into())),
        }
        match name.as_str() {
            "p" => {
                let p = match next(e, depth)? {
                    Some(t) if t.is_char('(') => coordinate(e)?.p,
                    _ => return Err(Error::Figure("let \\p: a point is missing".into())),
                };
                let (x, y) = (p.x / crate::PT, p.y / crate::PT);
                // \x and \y are in the coordinates of the picture's transformation (a point read in a
                // shifted scope gives what it was given there), so `(\x1,0)` comes back to the point
                let l = e.st.transform.inverse().apply(p);
                let (lx, ly) = (l.x / crate::PT, l.y / crate::PT);
                // \p1, \p{name}: \p takes the name as its argument, as TikZ defines it
                for (letter, value) in [("p", format!("tikra cs:{x}pt,{y}pt")), ("x", format!("{lx}pt")), ("y", format!("{ly}pt"))] {
                    e.tex.set_meaning(&Token::cs(&format!("tikra@let@{letter}@{index}")), macro_from(&value), false);
                    let_macro(e, letter)?;
                }
            }
            "n" => {
                let v = match next(e, depth)? {
                    Some(Token::Char(_, Cat::Begin)) => e.tex.read_group_body()?,
                    Some(t) => vec![t],
                    None => Vec::new(),
                };
                let text = e.math_string(v)?;
                let (val, unit) = crate::expr::eval_length(&text, &e.vars())?;
                let s = if unit { format!("{}pt", super::super::pgf_number(val / crate::PT)) } else { super::super::pgf_number(val) };
                e.tex.set_meaning(&Token::cs(&format!("tikra@let@n@{index}")), macro_from(&s), false);
                let_macro(e, "n")?;
            }
            other => return Err(Error::Figure(format!("let: \\{other} is not \\p or \\n"))),
        }
    }
}

/// `\p` (`\x`, `\y`, `\n`) as `let` defines it: one argument, the name of what it holds.
pub(super) fn let_macro(e: &mut Engine, letter: &str) -> Result<(), Error> {
    e.run_tokens(tikra_tokens(&format!("\\def\\{letter}#1{{\\csname tikra@let@{letter}@#1\\endcsname}}")))
}

/// `\foreach` inside a path: its body is more path operations.
pub(super) fn path_foreach(e: &mut Engine, b: &mut Builder) -> Result<(), Error> {
    let (names, list, options) = foreach_header(e)?;
    let body = match e.tex.next_nonspace() {
        Some(Token::Char(_, Cat::Begin)) => e.tex.read_group_body()?,
        Some(t) => vec![t],
        None => Vec::new(),
    };
    foreach_run(e, b, ForeachJob { names, list, options, body })
}

/// `\x/\y in {list}` of a `foreach` (the word already read): the variables and the list.
pub(super) fn foreach_header(e: &mut Engine) -> Result<(Vec<Token>, Vec<Token>, super::super::ForeachOptions), Error> {
    let mut vars: Vec<Token> = Vec::new();
    let mut opts: Vec<Token> = Vec::new();
    loop {
        let Some(t) = e.tex.get_token() else { return Err(Error::Figure("\\foreach without 'in'".into())) };
        if t.is_char('[') {
            opts.extend(e.tex.read_until_char(']', false)?);
            continue;
        }
        if t.is_char('i') {
            if let Some(n) = e.tex.get_token() {
                if n.is_char('n') {
                    break;
                }
                e.tex.back(n);
            }
        }
        vars.push(t);
    }
    let names: Vec<Token> = vars.iter().filter(|t| matches!(t, Token::Cs(_))).cloned().collect();
    let list = e.foreach_list()?;
    Ok((names, list, super::super::foreach_options(&opts)))
}

/// `node foreach \x in {list} [options] at (…) {text}`: the node once for each item.
pub(super) fn node_foreach(e: &mut Engine, b: &mut Builder, kind: &str) -> Result<(), Error> {
    let (names, list, options) = foreach_header(e)?;
    // the rest of the node as it is written, up to its text
    let mut spec = tikra_tokens(&format!("{kind} "));
    // the letters of the last word, spaces after it skipped
    let last_word = |spec: &[Token]| -> String {
        let s: Vec<char> =
            spec.iter().rev().skip_while(|t| matches!(t, Token::Char(_, Cat::Space))).map_while(|t| if let Token::Char(c, Cat::Letter) = t { Some(*c) } else { None }).collect();
        s.into_iter().rev().collect()
    };
    // inside another `foreach \y in {list}` (the nodes for each pair), before its `in`
    let mut header = false;
    while let Some(t) = e.tex.get_token() {
        if !matches!(t, Token::Char(_, Cat::Letter)) {
            match last_word(&spec).as_str() {
                "foreach" => header = true,
                "in" => header = false,
                _ => {}
            }
        }
        match &t {
            Token::Cs(_) | Token::Char('/', _) if header => spec.push(t),
            Token::Char(_, Cat::Begin) if last_word(&spec) == "in" => {
                header = false;
                spec.push(t);
                spec.extend(e.tex.read_group_body()?);
                spec.push(Token::Char('}', Cat::End));
            }
            Token::Char(_, Cat::Begin) if kind != "coordinate" => {
                spec.push(t);
                spec.extend(e.tex.read_group_body()?);
                spec.push(Token::Char('}', Cat::End));
                break;
            }
            Token::Char('[', _) | Token::Char('(', _) => {
                let close = if t.is_char('[') { ']' } else { ')' };
                spec.push(t);
                spec.extend(e.tex.read_until_char(close, false)?);
                spec.push(Token::other(close));
            }
            // `at +(x,y)`
            Token::Char(_, Cat::Letter) | Token::Char(_, Cat::Space) | Token::Char('+', _) => spec.push(t),
            _ => {
                e.tex.back(t);
                break;
            }
        }
    }
    foreach_run(e, b, ForeachJob { names, list, options, body: spec })
}

/// The path operations `body` once for each item of `list`, with the variables set.
pub(super) fn foreach_run(e: &mut Engine, b: &mut Builder, job: ForeachJob) -> Result<(), Error> {
    let ForeachJob { names, list, options, body } = job;
    let mut memory = options.initial_memory();
    for (k, item) in super::super::foreach_items(&list).into_iter().enumerate() {
        e.begin_group();
        let parts: Vec<Vec<Token>> = item.split(|t| t.is_char('/')).map(|p| keys::trim(p).to_vec()).collect();
        for (i, n) in names.iter().enumerate() {
            let v = parts.get(i).or(parts.last()).cloned().unwrap_or_default();
            e.tex.set_meaning(n, super::super::macro_tokens(v), false);
        }
        e.foreach_iteration(&options, k, &memory)?;
        let depth = e.tex.depth();
        e.tex.push_frame(body.clone());
        let r = operations(e, b, Some(depth));
        e.tex.drain_to(depth);
        e.foreach_remember(&options, &mut memory);
        // the path's settings stay; only the loop variables end with the iteration
        let st = e.st.clone();
        e.end_group();
        e.st = st;
        r?;
    }
    Ok(())
}
