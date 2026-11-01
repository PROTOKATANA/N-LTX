//! Reading arguments, numbers, lengths and colors, and the next picture statement.

use super::*;
use crate::color::{self, Color};
use crate::tex::{self, Cat, Meaning, Prim, Token};
use crate::Error;
use std::collections::HashMap;

impl Engine {
    // ---------------------------------------------------------------- reading arguments

    pub fn argument(&mut self) -> Result<Vec<Token>, Error> {
        self.tex.read_undelimited()
    }

    pub fn argument_string(&mut self) -> Result<String, Error> {
        let a = self.argument()?;
        Ok(self.tex.expand_to_string(a)?.trim().to_string())
    }

    /// `[…]` if it comes next (after spaces), without the brackets.
    pub fn optional(&mut self) -> Result<Option<Vec<Token>>, Error> {
        match self.tex.next_nonspace() {
            Some(t) if t.is_char('[') => Ok(Some(self.tex.read_until_char(']', false)?)),
            Some(t) => {
                self.tex.back(t);
                Ok(None)
            }
            None => Ok(None),
        }
    }

    /// Text for pgfmath: macros expanded, registers replaced by their values.
    pub fn math_string(&mut self, tokens: Vec<Token>) -> Result<String, Error> {
        let mut t = self.tex.expand_fully(tokens)?;
        // `\numexpr …\relax` and `\dimexpr …\relax` read as TeX reads them where pgfmath reads a number
        while let Some(i) = t.iter().position(|x| matches!(x, Token::Cs(_)) && matches!(self.tex.meaning(x), Meaning::Prim(Prim::Numexpr | Prim::Dimexpr))) {
            let dim = self.tex.meaning(&t[i]) == Meaning::Prim(Prim::Dimexpr);
            let (v, rest) = self.tex.expr_of(t.split_off(i + 1), dim)?;
            t.pop();
            t.extend(tex::string_tokens(&if dim { tex::dimen::show(v) } else { v.to_string() }));
            t.extend(rest);
        }
        let mut s = String::new();
        // `\pgf@sys@tonumber\reg`: the register without its unit
        let mut bare = false;
        for (i, x) in t.iter().enumerate() {
            if x.is_cs("pgf@sys@tonumber") {
                bare = true;
                continue;
            }
            match self.tex.meaning(x) {
                Meaning::Dimen(r) if bare && matches!(x, Token::Cs(_)) => {
                    bare = false;
                    let sp = self.tex.dimens.get(&r).copied().unwrap_or(0);
                    s.push_str(&pgf_number(sp as f64 / 65536.0));
                }
                Meaning::Count(r) if matches!(x, Token::Cs(_)) => s.push_str(&self.tex.counts.get(&r).copied().unwrap_or(0).to_string()),
                Meaning::CharDef(v) if matches!(x, Token::Cs(_)) => s.push_str(&v.to_string()),
                Meaning::Dimen(r) | Meaning::Skip(r) if matches!(x, Token::Cs(_)) => {
                    // `2\baselineskip` is a product
                    if s.ends_with(|c: char| c.is_ascii_digit() || c == '.') {
                        s.push('*');
                    }
                    let sp = self.tex.dimens.get(&r).copied().unwrap_or(0);
                    s.push_str(&format!("{}pt", sp as f64 / 65536.0));
                }
                // pgf's `\pgf@xx`, `\pgf@xy`, `\pgf@yx`, `\pgf@yy`: the parts of the x and y vectors, in pt
                Meaning::Undefined if matches!(x, Token::Cs(n) if matches!(&**n, "pgf@xx" | "pgf@xy" | "pgf@yx" | "pgf@yy")) => {
                    let Token::Cs(n) = x else { unreachable!() };
                    let v = if n.starts_with("pgf@x") { self.st.x } else { self.st.y };
                    let v = if n.ends_with('x') { v.x } else { v.y };
                    if s.ends_with(|c: char| c.is_ascii_digit() || c == '.') {
                        s.push('*');
                    }
                    s.push_str(&format!("{}pt", v / crate::PT));
                }
                _ => s.push_str(&tex::tokens_to_string(&t[i..i + 1])),
            }
            // keep `\x y` apart when a control word is followed by a letter
            if let (Token::Cs(_), Some(Token::Char(c, _))) = (x, t.get(i + 1)) {
                if c.is_alphabetic() && !s.ends_with(' ') {
                    s.push(' ');
                }
            }
        }
        Ok(s)
    }

    pub fn vars(&self) -> crate::expr::Variables {
        let sans = self.st.text.font.contains("\\sffamily");
        crate::expr::Variables {
            values: HashMap::new(),
            functions: self.functions_now(),
            font_scale: Some(self.st.text.ambient_size / 10.0),
            in_pt: false,
            sans,
            trig_rad: self.st.trig_rad,
            fpu: false,
        }
    }

    /// The declared functions, the macros in their bodies replaced by what they hold now, as pgfmath
    /// reads a body when the function is used (`f(\q)=\q*\total;`, `\total` set later).
    pub(super) fn functions_now(&self) -> HashMap<String, (Vec<String>, String)> {
        let mut functions = (*self.st.functions).clone();
        for (params, body) in functions.values_mut() {
            // a macro may hold others: a few rounds
            for _ in 0..8 {
                let mut out = String::new();
                let mut changed = false;
                let mut rest = body.as_str();
                while let Some(i) = rest.find('\\') {
                    out.push_str(&rest[..i]);
                    let name: String = rest[i + 1..].chars().take_while(|c| c.is_ascii_alphabetic() || *c == '@').collect();
                    match self.tex.meaning_of(&name) {
                        tex::Meaning::Macro(m) if !name.is_empty() && m.params.is_empty() && !params.contains(&name) => {
                            out.push_str(&tex::tokens_to_string(&tex::substitute(&m.body, &[])));
                            changed = true;
                        }
                        _ => {
                            out.push('\\');
                            out.push_str(&name);
                        }
                    }
                    rest = &rest[i + 1 + name.len()..];
                }
                out.push_str(rest);
                *body = out;
                if !changed {
                    break;
                }
            }
        }
        functions
    }

    /// A pgfmath number from text (a length in it counts in pt, as pgfmath counts it: `30pt` is 30).
    pub fn number(&mut self, text: &str) -> Result<f64, Error> {
        let t = tex::string_tokens(text);
        let s = self.math_string(t)?;
        let mut vars = self.vars();
        vars.in_pt = true;
        crate::expr::eval(&s, &vars)
    }

    /// A pgfmath length in cm (a number without a unit is in pt, as pgf reads lengths).
    pub fn length(&mut self, text: &str) -> Result<f64, Error> {
        let s = self.math_string(tikra_tokens(text))?;
        let (v, unit) = crate::expr::eval_length(&s, &self.vars())?;
        Ok(if unit { v } else { v * crate::PT })
    }

    pub fn number_of(&mut self, tokens: Vec<Token>) -> Result<f64, Error> {
        let s = self.math_string(tokens)?;
        crate::expr::eval(&s, &self.vars())
    }

    pub fn length_of(&mut self, tokens: Vec<Token>) -> Result<f64, Error> {
        let s = self.math_string(tokens)?;
        let (v, unit) = crate::expr::eval_length(&s, &self.vars())?;
        Ok(if unit { v } else { v * crate::PT })
    }

    pub fn color(&self, text: &str) -> Option<Color> {
        // `.` is xcolor's current color (`draw=.!60!black`)
        if text.split('!').any(|p| p.trim() == ".") {
            let mut colors = self.colors.clone();
            colors.insert(".".into(), self.st.paint.color);
            return color::parse(text.trim(), &colors);
        }
        color::parse(text.trim(), &self.colors)
    }

    /// The next statement in a picture: a `{…}` group, or tokens up to and including `;`.
    pub fn next_statement(&mut self) -> Result<Vec<Token>, Error> {
        match self.tex.next_nonspace() {
            Some(Token::Char(_, Cat::Begin)) => self.tex.read_group_body(),
            // a \foreach as the statement: its variables, options and list, then its own body
            // (`\pgffor@collectforeach`)
            Some(t) if t.is_cs("foreach") => {
                let mut out = vec![t];
                while let Some(x) = self.tex.get_token() {
                    if x.is_char('[') {
                        out.push(x);
                        out.extend(self.tex.read_until_char(']', false)?);
                        out.push(Token::other(']'));
                        continue;
                    }
                    let is_i = x.is_char('i');
                    out.push(x);
                    if is_i {
                        match self.tex.get_token() {
                            Some(n) if n.is_char('n') => {
                                out.push(n);
                                break;
                            }
                            Some(n) => self.tex.back(n),
                            None => break,
                        }
                    }
                }
                // the list, and options after it
                for _ in 0..2 {
                    match self.tex.next_nonspace() {
                        Some(b @ Token::Char(_, Cat::Begin)) => {
                            out.push(b);
                            out.extend(self.tex.read_group_body()?);
                            out.push(Token::Char('}', Cat::End));
                        }
                        Some(x) if x.is_char('[') => {
                            out.push(x);
                            out.extend(self.tex.read_until_char(']', false)?);
                            out.push(Token::other(']'));
                            continue;
                        }
                        Some(x) => {
                            if out.last().is_some_and(|t| matches!(t, Token::Char(_, Cat::End))) {
                                self.tex.back(x);
                            } else {
                                out.push(x);
                            }
                        }
                        None => {}
                    }
                    break;
                }
                out.push(Token::Char(' ', Cat::Space));
                out.extend(self.next_statement_braced()?);
                Ok(out)
            }
            Some(t) => {
                self.tex.back(t);
                let mut out = Vec::new();
                let mut depth = 0usize;
                while let Some(x) = self.tex.get_token() {
                    match x {
                        Token::Char(_, Cat::Begin) => depth += 1,
                        Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
                        _ => {}
                    }
                    let end = depth == 0 && x.is_char(';');
                    out.push(x);
                    if end || out.len() > tex::MAX_TOKENS {
                        break;
                    }
                }
                Ok(out)
            }
            None => Ok(Vec::new()),
        }
    }

    /// The next statement, kept in its braces when it is a group (the body of a `\foreach`).
    pub(super) fn next_statement_braced(&mut self) -> Result<Vec<Token>, Error> {
        match self.tex.next_nonspace() {
            Some(b @ Token::Char(_, Cat::Begin)) => {
                let mut out = vec![b];
                out.extend(self.tex.read_group_body()?);
                out.push(Token::Char('}', Cat::End));
                Ok(out)
            }
            Some(t) => {
                self.tex.back(t);
                self.next_statement()
            }
            None => Ok(Vec::new()),
        }
    }
}
