//! A small TeX: what TikZ code needs from TeX itself, so figures read as TeX reads them.
//!
//! Tokens with category codes, read lazily from the text so `\makeatletter` applies to what follows;
//! macros with parameters (delimited ones too), `\def`, `\edef`, `\let`, `\newcommand` with an optional
//! argument, `\NewDocumentCommand` and its argument types, `\newenvironment`; groups, with local and
//! `\global` assignments; count and dimen registers; conditionals; `\expandafter`, `\csname`,
//! `\string`, `\the`, `\number`, `\noexpand` and the e-TeX extensions pgf relies on (`\numexpr`,
//! `\dimexpr`, `\unexpanded`, `\detokenize`, `\ifdefined`, `\ifcsname`, `\unless`, `\protected`).
//!
//! What is drawn is not here: commands such as `\draw` are [`Meaning::Native`], run by the TikZ layer
//! with full access to the token stream, the way `tikz.code.tex` reads its input.

mod builtin;
pub mod dimen;
mod execute;
mod expand;
mod groups;
mod input;
mod scan;
mod token;
mod xstring;

pub use token::{Body, Cat, Macro, Meaning, Param, Prim, Token};

use std::collections::{HashMap, VecDeque};
use std::hash::{BuildHasherDefault, Hasher};
use std::rc::Rc;

/// Where tokens come from: a token list, or text read with the current category codes.
#[derive(Clone)]
pub(in crate::tex) enum Frame {
    Tokens(VecDeque<Token>),
    Text { chars: Rc<Vec<char>>, pos: usize, skip_blanks: bool },
}

/// One value saved on entering a group, put back when it ends.
#[derive(Clone)]
pub(in crate::tex) enum Saved {
    Meaning(Rc<str>, Option<Meaning>),
    Active(char, Option<Meaning>),
    Count(u32, Option<i64>),
    Dimen(u32, Option<i64>),
    Toks(u32, Option<Rc<Vec<Token>>>),
    Catcode(char, Option<Cat>),
}

/// Cap on expansions per figure: `\def\a{\a}\a` stops with an error instead of hanging.
pub const MAX_EXPANSIONS: usize = 2_000_000;
/// Cap on tokens in one list (a macro doubling itself on each step stops here).
pub const MAX_TOKENS: usize = 1_000_000;

/// A fast hash for the tables TeX looks into at every token (names, registers, catcodes): FxHash,
/// as rustc uses. Names come from the document, so crafted ones can collide; that only slows the
/// lookups, and the engine's caps on statements and expansions bound the work.
#[derive(Default, Clone, Copy)]
pub struct FxHasher(u64);

impl Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        const K: u64 = 0x51_7c_c1_b7_27_22_0a_95;
        let (chunks, rest) = bytes.as_chunks::<8>();
        for c in chunks {
            self.0 = (self.0.rotate_left(5) ^ u64::from_le_bytes(*c)).wrapping_mul(K);
        }
        for b in rest {
            self.0 = (self.0.rotate_left(5) ^ u64::from(*b)).wrapping_mul(K);
        }
    }
    fn write_u32(&mut self, v: u32) {
        self.0 = (self.0.rotate_left(5) ^ u64::from(v)).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn write_u64(&mut self, v: u64) {
        self.0 = (self.0.rotate_left(5) ^ v).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn write_usize(&mut self, v: usize) {
        self.write_u64(v as u64);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

pub type Map<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;

/// The state of TeX: input, meanings, registers, groups.
#[derive(Clone)]
pub struct Tex {
    input: Vec<Frame>,
    meanings: Map<Rc<str>, Meaning>,
    active: Map<char, Meaning>,
    pub counts: Map<u32, i64>,
    /// in scaled points (65536 sp = 1 pt)
    pub dimens: Map<u32, i64>,
    pub toks: Map<u32, Rc<Vec<Token>>>,
    catcodes: Map<char, Cat>,
    save: Vec<Vec<Saved>>,
    /// a `\global` prefix is pending
    global: bool,
    /// open conditionals whose true branch is being read (an `\else` there skips to `\fi`)
    conds: usize,
    next_register: u32,
    pub expansions: usize,
    /// expansions since the drawing layer last called [`Tex::new_statement`]
    pub statement_expansions: usize,
    after_group: Vec<Vec<Token>>,
    after_assignment: Option<Token>,
    pub warnings: Vec<String>,
    /// `\begin{name}` defined by `\newenvironment`: (begin code with its parameters, end code)
    pub environments: HashMap<String, (Rc<Macro>, Vec<Token>)>,
    /// the current environment names, innermost last, for `\end`
    pub env_stack: Vec<String>,
    /// `\setbox\name=\hbox{…}` for a box named by a control sequence (`\pgfnodepartlowerbox`):
    /// what was put in it, in order
    pub named_boxes: Vec<(String, Vec<Token>)>,
    /// the width, height and depth of box registers that have them (sp), by number: what
    /// `\wd`, `\ht`, `\dp` read (0pt otherwise)
    pub box_dims: Map<i64, [i64; 3]>,
}

impl Default for Tex {
    fn default() -> Self {
        Tex::new()
    }
}

pub fn parse_body(raw: Vec<Token>) -> Vec<Body> {
    let mut out = Vec::with_capacity(raw.len());
    let mut it = raw.into_iter().peekable();
    while let Some(t) = it.next() {
        if let Token::Char(c, Cat::Param) = t {
            match it.peek() {
                Some(Token::Char(d, _)) if d.is_ascii_digit() => {
                    let n = *d as u8 - b'0';
                    it.next();
                    out.push(Body::Arg(n));
                }
                Some(Token::Char(_, Cat::Param)) => {
                    // ## stays a parameter character (for definitions inside definitions)
                    let p = it.next().unwrap();
                    out.push(Body::Tok(p));
                }
                _ => out.push(Body::Tok(Token::Char(c, Cat::Param))),
            }
        } else {
            out.push(Body::Tok(t));
        }
    }
    out
}

pub fn substitute(body: &[Body], args: &[Vec<Token>]) -> Vec<Token> {
    let mut out = Vec::with_capacity(body.len());
    for b in body {
        match b {
            Body::Tok(t) => out.push(t.clone()),
            Body::Arg(n) => {
                if let Some(a) = args.get((*n as usize).wrapping_sub(1)) {
                    out.extend(a.iter().cloned());
                }
            }
        }
    }
    out
}

/// `{x}` → `x` when the braces enclose the whole list.
pub fn strip_outer_braces(v: Vec<Token>) -> Vec<Token> {
    if v.len() >= 2 && matches!(v[0], Token::Char(_, Cat::Begin)) && matches!(v[v.len() - 1], Token::Char(_, Cat::End)) {
        let mut depth = 0i32;
        for (i, t) in v.iter().enumerate() {
            match t {
                Token::Char(_, Cat::Begin) => depth += 1,
                Token::Char(_, Cat::End) => {
                    depth -= 1;
                    if depth == 0 && i != v.len() - 1 {
                        return v;
                    }
                }
                _ => {}
            }
        }
        return v[1..v.len() - 1].to_vec();
    }
    v
}

/// Text as character tokens (spaces as spaces, everything else "other"), as `\string` makes them.
pub fn string_tokens(s: &str) -> Vec<Token> {
    s.chars().map(|c| if c == ' ' { Token::Char(' ', Cat::Space) } else { Token::Char(c, Cat::Other) }).collect()
}

/// Tokens as text: control sequences as `\name` followed by a space when the name is a word.
pub fn tokens_to_string(v: &[Token]) -> String {
    // a reservation for the usual text: growing took as long as filling it
    let mut s = String::with_capacity(v.len());
    for (i, t) in v.iter().enumerate() {
        match t {
            Token::Cs(n) => {
                s.push('\\');
                s.push_str(n);
                let word = n.chars().next().is_some_and(|c| c.is_alphabetic() || c == '@');
                // a space only where a letter follows, so `\x+1` stays `\x+1`
                if word && matches!(v.get(i + 1), Some(Token::Char(c, _)) if c.is_alphabetic()) {
                    s.push(' ');
                }
            }
            Token::Char(c, _) => s.push(*c),
        }
    }
    s
}

/// Like [`tokens_to_string`], for text read again as TeX: a control word before a space (`\o` from
/// an active character's definition) gets `{}`, so the space stays.
pub fn tokens_to_text(v: &[Token]) -> String {
    let mut s = String::new();
    for (i, t) in v.iter().enumerate() {
        let word = matches!(t, Token::Cs(n) if n.chars().next().is_some_and(|c| c.is_alphabetic() || c == '@'));
        if word && matches!(v.get(i + 1), Some(Token::Char(_, Cat::Space))) {
            s.push_str(&tokens_to_string(std::slice::from_ref(t)));
            s.push_str("{}");
        } else {
            s.push_str(&tokens_to_string(&v[i..=i]));
            if word && matches!(v.get(i + 1), Some(Token::Char(c, _)) if c.is_alphabetic()) {
                s.push(' ');
            }
        }
    }
    s
}

pub fn show_token(t: &Token) -> String {
    match t {
        Token::Cs(n) => format!("\\{n}"),
        Token::Char(c, _) => c.to_string(),
    }
}

#[cfg(test)]
mod tests;
