//! The xstring package's commands: parts of a text (`\StrLeft`, `\StrBefore`, `\StrCut`…), its
//! length and counts, substitutions, and tests (`\IfSubStr`, `\IfStrEq`…). As xstring does by
//! default, the texts are expanded first; a character is a token, and a `{…}` group counts as one.
//! A result goes into the macro named in brackets after the arguments (`\StrLeft{abc}{2}[\r]`),
//! locally, or is typeset where the command is.

use super::{string_tokens, Body, Cat, Macro, Meaning, Tex, Token};
use crate::Error;
use std::rc::Rc;

/// The commands, by name.
pub const COMMANDS: &[&str] = &[
    "StrLen",
    "StrLeft",
    "StrRight",
    "StrMid",
    "StrChar",
    "StrBefore",
    "StrBehind",
    "StrCut",
    "StrBetween",
    "StrSubstitute",
    "StrDel",
    "StrCount",
    "IfSubStr",
    "IfStrEq",
    "IfBeginWith",
    "IfEndWith",
];

/// The characters of a text: each token, or each `{…}` group with its braces.
fn units(v: &[Token]) -> Vec<&[Token]> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < v.len() {
        let mut j = i + 1;
        if matches!(v[i], Token::Char(_, Cat::Begin)) {
            let mut depth = 1usize;
            while j < v.len() && depth > 0 {
                match v[j] {
                    Token::Char(_, Cat::Begin) => depth += 1,
                    Token::Char(_, Cat::End) => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
        }
        out.push(&v[i..j]);
        i = j;
    }
    out
}

/// Where `p` occurs in `s`, from the left, without overlaps.
fn occurrences(s: &[&[Token]], p: &[&[Token]]) -> Vec<usize> {
    let mut out = Vec::new();
    if p.is_empty() {
        return out;
    }
    let mut i = 0;
    while i + p.len() <= s.len() {
        if s[i..i + p.len()] == *p {
            out.push(i);
            i += p.len();
        } else {
            i += 1;
        }
    }
    out
}

fn join(v: &[&[Token]]) -> Vec<Token> {
    v.iter().flat_map(|u| u.iter().cloned()).collect()
}

impl Tex {
    /// An argument, expanded.
    fn xstring_text(&mut self) -> Result<Vec<Token>, Error> {
        let a = self.read_undelimited()?;
        self.expand_fully(a)
    }

    /// An argument read as an integer.
    fn xstring_int(&mut self, a: Vec<Token>) -> Result<i64, Error> {
        let a = self.expand_fully(a)?;
        self.int_of(a)
    }

    /// `[…]` if it comes next, its numbers separated by commas.
    fn xstring_counts(&mut self) -> Result<Vec<i64>, Error> {
        let Some(t) = self.next_nonspace() else { return Ok(Vec::new()) };
        if !t.is_char('[') {
            self.back(t);
            return Ok(Vec::new());
        }
        let o = self.read_until_char(']', false)?;
        let mut out = Vec::new();
        for part in o.split(|t| t.is_char(',')) {
            out.push(self.xstring_int(part.to_vec())?);
        }
        Ok(out)
    }

    /// The result: into the macro named in `[\name]` if that follows, typeset otherwise.
    fn xstring_result(&mut self, v: Vec<Token>) -> Result<(), Error> {
        if let Some(t) = self.next_nonspace() {
            if t.is_char('[') {
                let name = self.read_until_char(']', false)?;
                if let Some(name) = name.into_iter().find(|x| !x.is_space()) {
                    self.define_tokens(&name, v);
                }
                return Ok(());
            }
            self.back(t);
        }
        self.push_tokens(v);
        Ok(())
    }

    fn define_tokens(&mut self, name: &Token, v: Vec<Token>) {
        let m = Macro { params: Vec::new(), body: v.into_iter().map(Body::Tok).collect(), optional: None, protected: false };
        self.set_meaning(name, Meaning::Macro(Rc::new(m)), false);
    }

    /// Runs the xstring command `name`.
    pub fn xstring(&mut self, name: &str) -> Result<(), Error> {
        // the occurrence counts in brackets before the texts
        let counts =
            if matches!(name, "StrBefore" | "StrBehind" | "StrCut" | "StrBetween" | "StrSubstitute" | "StrDel" | "IfSubStr") { self.xstring_counts()? } else { Vec::new() };
        let n = counts.first().copied();
        let text = self.xstring_text()?;
        let s = units(&text);
        if self.xstring_test(name, &s, n)? {
            return Ok(());
        }
        let result = match name {
            "StrLen" => string_tokens(&s.len().to_string()),
            "StrLeft" | "StrRight" | "StrChar" => self.xstring_ends(name, &s)?,
            "StrMid" => self.xstring_mid(&s)?,
            "StrSubstitute" | "StrDel" => self.xstring_replace(name, &s, n)?,
            "StrBetween" => self.xstring_between(&s, &counts)?,
            "StrCount" => {
                let p = self.xstring_text()?;
                string_tokens(&occurrences(&s, &units(&p)).len().to_string())
            }
            "StrBefore" | "StrBehind" => self.xstring_beside(name, &s, n)?,
            "StrCut" => return self.xstring_strcut(&s, &text, n),
            // a name that is not one of the commands above still stores the two sides
            _ => return self.xstring_define_sides(&s, n),
        };
        self.xstring_result(result)
    }

    fn xstring_ends(&mut self, name: &str, s: &[&[Token]]) -> Result<Vec<Token>, Error> {
        let a = self.read_undelimited()?;
        let n = self.xstring_int(a)?;
        let k = n.clamp(0, s.len() as i64) as usize;
        Ok(match name {
            "StrLeft" => join(&s[..k]),
            "StrRight" => join(&s[s.len() - k..]),
            _ => usize::try_from(n - 1).ok().and_then(|i| s.get(i)).map(|u| u.to_vec()).unwrap_or_default(),
        })
    }

    fn xstring_mid(&mut self, s: &[&[Token]]) -> Result<Vec<Token>, Error> {
        let (a, b) = (self.read_undelimited()?, self.read_undelimited()?);
        let a = self.xstring_int(a)?.max(1) as usize;
        let b = self.xstring_int(b)?.min(s.len() as i64).max(0) as usize;
        Ok(if a <= b { join(&s[a - 1..b]) } else { Vec::new() })
    }

    fn xstring_replace(&mut self, name: &str, s: &[&[Token]], n: Option<i64>) -> Result<Vec<Token>, Error> {
        let from = self.xstring_text()?;
        let to = if name == "StrSubstitute" { self.xstring_text()? } else { Vec::new() };
        let p = units(&from);
        // a count of 0 (the default) replaces them all
        let n = n.unwrap_or(0);
        let found = occurrences(s, &p);
        let mut out = Vec::new();
        let mut at = 0;
        for (k, &i) in found.iter().enumerate() {
            if n > 0 && k as i64 >= n {
                break;
            }
            out.extend(join(&s[at..i]));
            out.extend(to.iter().cloned());
            at = i + p.len();
        }
        out.extend(join(&s[at..]));
        Ok(out)
    }

    fn xstring_between(&mut self, s: &[&[Token]], counts: &[i64]) -> Result<Vec<Token>, Error> {
        let (a, b) = (self.xstring_text()?, self.xstring_text()?);
        let (pa, pb) = (units(&a), units(&b));
        // the n1-th A and the n2-th B, both counted from the start
        let n1 = counts.first().copied().unwrap_or(1);
        let n2 = counts.get(1).copied().unwrap_or(1);
        let nth = |found: Vec<usize>, k: i64| usize::try_from(k - 1).ok().and_then(|k| found.get(k).copied());
        Ok(match (nth(occurrences(s, &pa), n1), nth(occurrences(s, &pb), n2)) {
            (Some(i), Some(j)) if j >= i + pa.len() => join(&s[i + pa.len()..j]),
            _ => Vec::new(),
        })
    }

    /// `\IfSubStr`, `\IfStrEq`, `\IfBeginWith`, `\IfEndWith`. `Ok(false)` when `name` is none of them.
    fn xstring_test(&mut self, name: &str, s: &[&[Token]], n: Option<i64>) -> Result<bool, Error> {
        if !matches!(name, "IfSubStr" | "IfStrEq" | "IfBeginWith" | "IfEndWith") {
            return Ok(false);
        }
        let p = self.xstring_text()?;
        let p = units(&p);
        let test = match name {
            "IfSubStr" => occurrences(s, &p).len() as i64 >= n.unwrap_or(1),
            "IfStrEq" => s == p,
            "IfBeginWith" => s.starts_with(&p),
            _ => s.ends_with(&p),
        };
        let (yes, no) = (self.read_undelimited()?, self.read_undelimited()?);
        self.push_tokens(if test { yes } else { no });
        Ok(true)
    }

    /// `\StrBefore` and `\StrBehind`: the text on one side of the n-th occurrence.
    fn xstring_beside(&mut self, name: &str, s: &[&[Token]], n: Option<i64>) -> Result<Vec<Token>, Error> {
        let (before, behind) = self.sides_of(s, n)?;
        Ok(if name == "StrBefore" { before } else { behind })
    }

    /// `\StrCut`: the two sides of the n-th occurrence, each stored in the macro that follows.
    /// A miss keeps the whole text on the left and nothing on the right.
    fn xstring_strcut(&mut self, s: &[&[Token]], text: &[Token], n: Option<i64>) -> Result<(), Error> {
        let p = self.xstring_text()?;
        let p = units(&p);
        let found = occurrences(s, &p);
        let k = n.unwrap_or(1);
        let (before, behind) = match found.get(k.max(1) as usize - 1) {
            Some(&i) => (join(&s[..i]), join(&s[i + p.len()..])),
            None => (text.to_vec(), Vec::new()),
        };
        self.define_pair(before, behind)
    }

    /// Same storage as `\StrCut`, counting the occurrence as `\StrBefore` does. A miss stores nothing.
    fn xstring_define_sides(&mut self, s: &[&[Token]], n: Option<i64>) -> Result<(), Error> {
        let (before, behind) = self.sides_of(s, n)?;
        self.define_pair(before, behind)
    }

    /// The text before and after the n-th occurrence of the next argument. A miss is empty on both sides.
    fn sides_of(&mut self, s: &[&[Token]], n: Option<i64>) -> Result<(Vec<Token>, Vec<Token>), Error> {
        let p = self.xstring_text()?;
        let p = units(&p);
        let found = occurrences(s, &p);
        let k = n.unwrap_or(1);
        let at = usize::try_from(k - 1).ok().and_then(|k| found.get(k));
        Ok(match at {
            Some(&i) => (join(&s[..i]), join(&s[i + p.len()..])),
            None => (Vec::new(), Vec::new()),
        })
    }

    fn define_pair(&mut self, before: Vec<Token>, behind: Vec<Token>) -> Result<(), Error> {
        let (l, r) = (self.read_undelimited()?, self.read_undelimited()?);
        for (target, v) in [(l, before), (r, behind)] {
            if let Some(m) = target.into_iter().find(|x| !x.is_space()) {
                self.define_tokens(&m, v);
            }
        }
        Ok(())
    }
}
