//! Category codes and the input stack.

use super::*;
use crate::Error;
use std::collections::VecDeque;
use std::rc::Rc;

impl Tex {
    // ---------------------------------------------------------------- category codes and input

    pub fn catcode(&self, c: char) -> Cat {
        if let Some(cat) = self.catcodes.get(&c) {
            return *cat;
        }
        match c {
            '\\' => Cat::Escape,
            '{' => Cat::Begin,
            '}' => Cat::End,
            '$' => Cat::Math,
            '&' => Cat::Align,
            '\r' | '\n' => Cat::EndLine,
            '#' => Cat::Param,
            '^' => Cat::Sup,
            '_' => Cat::Sub,
            '\0' => Cat::Ignored,
            ' ' | '\t' => Cat::Space,
            '~' => Cat::Active,
            '%' => Cat::Comment,
            '\u{7f}' => Cat::Invalid,
            c if c.is_alphabetic() => Cat::Letter,
            _ => Cat::Other,
        }
    }

    pub fn set_catcode(&mut self, c: char, cat: Cat) {
        let old = self.catcodes.get(&c).copied();
        if !self.global {
            if let Some(level) = self.save.last_mut() {
                level.push(Saved::Catcode(c, old));
            }
        }
        self.catcodes.insert(c, cat);
    }

    pub fn push_text(&mut self, text: &str) {
        self.input.push(Frame::Text { chars: Rc::new(text.chars().collect()), pos: 0, skip_blanks: false });
    }

    /// Puts tokens back at the front of the input.
    pub fn push_tokens(&mut self, tokens: impl IntoIterator<Item = Token>) {
        let list: VecDeque<Token> = tokens.into_iter().collect();
        if list.is_empty() {
            return;
        }
        if let Some(Frame::Tokens(front)) = self.input.last_mut() {
            // join with the frame on top, so deep chains of back-pushed tokens do not pile up frames
            for t in list.into_iter().rev() {
                front.push_front(t);
            }
            return;
        }
        self.input.push(Frame::Tokens(list));
    }

    /// Pushes tokens as a frame of their own (never joined to the one below), for bounded reading.
    pub fn push_frame(&mut self, tokens: Vec<Token>) {
        self.input.push(Frame::Tokens(tokens.into_iter().collect()));
    }

    /// Reads a dimension from `tokens` alone (`\setlength{\l}{2pt}`): what is left of them is dropped.
    pub fn dimen_of(&mut self, tokens: Vec<Token>) -> Result<i64, Error> {
        let depth = self.input.len();
        let mut v = tokens;
        v.push(Token::cs("relax"));
        self.push_frame(v);
        let r = self.scan_dimen().and_then(|d| self.skip_glue_parts().map(|_| d));
        self.drain_to(depth);
        r
    }

    /// Reads an integer from `tokens` alone.
    pub fn int_of(&mut self, tokens: Vec<Token>) -> Result<i64, Error> {
        let depth = self.input.len();
        let mut v = tokens;
        v.push(Token::cs("relax"));
        self.push_frame(v);
        let r = self.scan_int();
        self.drain_to(depth);
        r
    }

    /// Reads `\numexpr …` (or `\dimexpr …` if `dim`, in sp) from the start of `tokens`, the command
    /// itself already taken: its value and the tokens after it.
    pub fn expr_of(&mut self, tokens: Vec<Token>, dim: bool) -> Result<(i64, Vec<Token>), Error> {
        let depth = self.input.len();
        let end = Token::cs("tikra@endexpr");
        let mut v = tokens;
        v.push(end.clone());
        self.push_frame(v);
        let r = if dim { self.scan_dimexpr() } else { self.scan_numexpr() };
        let mut rest = Vec::new();
        while let Some(t) = self.get_token_until(depth) {
            if t == end {
                break;
            }
            rest.push(t);
        }
        self.drain_to(depth);
        Ok((r?, rest))
    }

    pub fn back(&mut self, t: Token) {
        self.push_tokens([t]);
    }

    /// The number of input frames (for reading only what was pushed after this point).
    pub fn depth(&self) -> usize {
        self.input.len()
    }

    /// Like [`Tex::get_x_token`], but only from frames above `depth`.
    pub fn get_x_token_until(&mut self, depth: usize) -> Result<Option<Token>, Error> {
        loop {
            let Some(t) = self.get_token_until(depth) else { return Ok(None) };
            let m = self.meaning(&t);
            if m.is_expandable() {
                self.expand(&t, &m)?;
                continue;
            }
            return Ok(Some(t));
        }
    }

    /// Drops all the input left (`\end{document}`: TeX reads no further).
    pub fn clear_input(&mut self) {
        self.input.clear();
    }

    /// Whether input is left (at any depth).
    pub fn has_input(&mut self) -> bool {
        if let Some(t) = self.get_token() {
            self.back(t);
            true
        } else {
            false
        }
    }

    /// The next token, unexpanded, or None at the end of the input.
    pub fn get_token(&mut self) -> Option<Token> {
        self.get_token_until(0)
    }

    /// A token from frames above `depth` only.
    pub fn get_token_until(&mut self, depth: usize) -> Option<Token> {
        while self.input.len() > depth {
            let from_list = match self.input.last_mut()? {
                Frame::Tokens(list) => Some(list.pop_front()),
                Frame::Text { .. } => None,
            };
            let t = match from_list {
                Some(t) => t,
                None => self.read_text_token(),
            };
            match t {
                Some(t) => return Some(t),
                None => {
                    self.input.pop();
                }
            }
        }
        None
    }

    /// Reads one token from the text frame on top, applying TeX's rules for spaces, lines and comments.
    fn read_text_token(&mut self) -> Option<Token> {
        let (chars, mut pos, mut skip) = match self.input.last() {
            Some(Frame::Text { chars, pos, skip_blanks }) => (chars.clone(), *pos, *skip_blanks),
            _ => return None,
        };
        let result = loop {
            let Some(&c) = chars.get(pos) else { break None };
            let next = chars.get(pos + 1).copied();
            let cat = self.catcode(c);
            match cat {
                Cat::Escape => {
                    pos += 1;
                    let Some(&first) = chars.get(pos) else {
                        skip = false;
                        break Some(Token::cs(""));
                    };
                    if self.catcode(first) == Cat::Letter {
                        let start = pos;
                        while pos < chars.len() && self.catcode(chars[pos]) == Cat::Letter {
                            pos += 1;
                        }
                        let name: String = chars[start..pos].iter().collect();
                        skip = true;
                        break Some(Token::Cs(Rc::from(name.as_str())));
                    }
                    pos += 1;
                    // after a one-character name, spaces are skipped only for `\ `
                    skip = self.catcode(first) == Cat::Space;
                    break Some(Token::Cs(Rc::from(first.to_string().as_str())));
                }
                Cat::Space => {
                    pos += 1;
                    if skip {
                        continue;
                    }
                    skip = true;
                    break Some(Token::Char(' ', Cat::Space));
                }
                Cat::EndLine => {
                    pos += 1;
                    if c == '\r' && next == Some('\n') {
                        pos += 1;
                    }
                    // an empty line is a paragraph break; another end of line is a space
                    let mut k = pos;
                    while k < chars.len() && matches!(chars[k], ' ' | '\t') {
                        k += 1;
                    }
                    let was_skipping = skip;
                    skip = true;
                    if matches!(chars.get(k), Some('\n') | Some('\r')) {
                        pos = k + 1;
                        break Some(Token::cs("par"));
                    }
                    if was_skipping {
                        continue;
                    }
                    break Some(Token::Char(' ', Cat::Space));
                }
                Cat::Comment => {
                    while pos < chars.len() && chars[pos] != '\n' {
                        pos += 1;
                    }
                    // the end of line goes too, and the next line's leading spaces
                    pos = (pos + 1).min(chars.len());
                    while pos < chars.len() && matches!(chars[pos], ' ' | '\t') {
                        pos += 1;
                    }
                    skip = false;
                    continue;
                }
                Cat::Ignored | Cat::Invalid => {
                    pos += 1;
                    continue;
                }
                Cat::Sup if next == Some(c) && pos + 2 < chars.len() => {
                    // ^^M, ^^?: the character 64 away
                    let x = chars[pos + 2];
                    pos += 3;
                    let code = x as u32;
                    let ch = char::from_u32(if code < 64 { code + 64 } else { code - 64 }).unwrap_or(x);
                    skip = false;
                    let cat = self.catcode(ch);
                    break Some(Token::Char(ch, cat));
                }
                _ => {
                    pos += 1;
                    skip = false;
                    break Some(Token::Char(c, cat));
                }
            }
        };
        if let Some(Frame::Text { pos: p, skip_blanks, .. }) = self.input.last_mut() {
            *p = pos;
            *skip_blanks = skip;
        }
        result
    }
}
