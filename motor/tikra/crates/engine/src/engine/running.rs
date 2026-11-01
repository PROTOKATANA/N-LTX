//! Running input: statements, text between pictures, and recovering from an error.

use super::*;
use crate::tex::{self, Cat, Meaning, Prim, Token};
use crate::Error;

/// The text commands that change the style of their argument, set in the text around the pictures
/// as in a node's text.
const TEXT_STYLES: &[&str] = &["emph", "textit", "textbf", "textsl", "textsc", "texttt", "textsf", "textrm", "textup", "textmd", "textnormal"];

/// What starts a statement of a picture, or ends the picture.
const STATEMENTS: &[&str] =
    &["begin", "clip", "coordinate", "draw", "end", "fill", "filldraw", "foreach", "graph", "matrix", "node", "path", "pic", "shade", "shadedraw", "useasboundingbox"];

impl Engine {
    // ---------------------------------------------------------------- running

    pub fn warn(&mut self, s: impl Into<String>) {
        self.tex.warn(s);
    }

    /// Runs the input until it ends.
    pub fn run(&mut self) -> Result<(), Error> {
        while let Some(t) = self.tex.get_x_token()? {
            self.execute(t)?;
        }
        Ok(())
    }

    /// Runs `tokens` to completion (a loop body, a style's code), then returns.
    pub fn run_tokens(&mut self, tokens: Vec<Token>) -> Result<(), Error> {
        self.nested(|e, depth| {
            e.tex.push_frame(tokens);
            e.run_until(depth)
        })
    }

    /// Runs `work` one level deeper, with the depth of the input it started at: if it fails, what
    /// it left there is dropped.
    pub(crate) fn nested(&mut self, work: impl FnOnce(&mut Engine, usize) -> Result<(), Error>) -> Result<(), Error> {
        if self.nesting >= MAX_NESTING {
            // a style or macro that calls itself: TeX would run out of memory
            self.warn(format!("styles or macros nested more than {MAX_NESTING} levels (one calls itself?): the rest ignored"));
            return Ok(());
        }
        self.nesting += 1;
        let depth = self.tex.depth();
        let r = work(self, depth);
        if r.is_err() {
            self.tex.drain_to(depth);
        }
        self.nesting -= 1;
        r
    }

    /// Runs what is in the input above `depth`.
    pub(crate) fn run_until(&mut self, depth: usize) -> Result<(), Error> {
        while let Some(t) = self.tex.get_x_token_until(depth)? {
            self.execute(t)?;
        }
        Ok(())
    }

    pub fn count_step(&mut self) -> Result<(), Error> {
        self.steps += 1;
        if self.steps > MAX_STEPS {
            return Err(Error::Figure(format!("figure too large: more than {MAX_STEPS} statements")));
        }
        Ok(())
    }

    pub fn execute(&mut self, t: Token) -> Result<(), Error> {
        let m = self.tex.meaning(&t);
        match m {
            Meaning::Char(_, Cat::Begin) if self.picture_depth > 0 => {
                // the scopes library: `{[options] …}` is a scope
                match self.tex.next_nonspace() {
                    Some(n) if n.is_char('[') => {
                        let o = self.tex.read_until_char(']', false)?;
                        self.brace_scopes.push(true);
                        let r = self.open_tikz_scope(Some(o), false);
                        self.recover(r, &t)?;
                    }
                    other => {
                        if let Some(n) = other {
                            self.tex.back(n);
                        }
                        self.brace_scopes.push(false);
                        self.begin_group();
                    }
                }
            }
            Meaning::Char(_, Cat::End) if self.picture_depth > 0 => {
                if self.brace_scopes.pop() == Some(true) {
                    let r = self.close_tikz_scope();
                    self.recover(r, &t)?;
                } else {
                    self.end_group();
                }
            }
            Meaning::Char(_, Cat::Begin) => self.begin_group(),
            Meaning::Char(_, Cat::End) => self.end_group(),
            // `$=\implies$` in the text around the pictures: a formula, set as a word
            Meaning::Char(_, Cat::Math) if self.picture_depth == 0 => {
                let mut formula = Vec::new();
                while let Some(n) = self.tex.get_token() {
                    if matches!(self.tex.meaning(&n), Meaning::Char(_, Cat::Math)) {
                        break;
                    }
                    formula.push(n);
                }
                if formula.iter().any(|x| x.is_cs("tikz") || x.is_cs("begin")) {
                    // a picture inside: read as it comes
                    self.tex.push_tokens(formula);
                    return Ok(());
                }
                let text = format!("${}$", tex::tokens_to_string(&formula));
                match self.layout.last() {
                    Some(Layout::Word(i)) => self.words[*i].1.push_str(&text),
                    _ => {
                        self.layout.push(Layout::Word(self.words.len()));
                        self.words.push((self.st.text.font.clone(), text));
                    }
                }
            }
            Meaning::Prim(Prim::Par) if self.picture_depth == 0 => {
                if !self.layout.is_empty() && self.layout.last() != Some(&Layout::Par) {
                    self.layout.push(Layout::Par);
                }
            }
            Meaning::Prim(Prim::Begingroup) => self.begin_group(),
            Meaning::Prim(Prim::Endgroup) => self.end_group(),
            Meaning::Native(id) if id >= NATIVE_KEY_BASE => {
                // a key's code used as a command: nothing to do outside key processing
            }
            Meaning::Native(id) => {
                self.count_step()?;
                self.tex.new_statement();
                let r = self.command(id, &t);
                self.recover(r, &t)?;
            }
            Meaning::Char(c, _) if matches!(t, Token::Char(..)) => self.text(c),
            Meaning::Undefined => {
                if let Token::Cs(name) = &t {
                    self.unknown_command(name)?;
                } else if matches!(t, Token::Char('~', Cat::Active)) && self.picture_depth == 0 {
                    // `~`: an interword space where the line cannot break
                    self.layout.push(Layout::Skip(SPACE));
                }
            }
            _ => {
                if !self.tex.execute_basic(t.clone())? {
                    if let Meaning::Char(c, _) = self.tex.meaning(&t) {
                        self.text(c);
                    }
                }
            }
        }
        Ok(())
    }

    /// A box of text in the paragraph, a word of its own (a `tabular`).
    pub fn typeset_word(&mut self, text: String) {
        self.layout.push(Layout::Word(self.words.len()));
        self.words.push((self.st.text.font.clone(), text));
    }

    /// Whether a `tabular` around the pictures is open.
    pub(super) fn in_table(&self) -> bool {
        let opened = self.layout.iter().filter(|l| matches!(l, Layout::TabStart(_))).count();
        opened > self.layout.iter().filter(|l| matches!(l, Layout::TabEnd)).count()
    }

    /// A character outside any statement: TeX would typeset it; in a picture TikZ ignores it.
    pub(super) fn text(&mut self, c: char) {
        if self.picture_depth == 0 && c == '&' && self.in_table() {
            self.layout.push(Layout::Cell);
            return;
        }
        if self.picture_depth == 0 && c.is_whitespace() {
            // a space after a picture or a word is TeX's interword space; spaces with commands
            // between them (`\tikzset{…}` on its own line) are glue each, so is one after `\hspace{…}`
            if matches!(self.layout.last(), Some(Layout::Picture(_) | Layout::Word(_) | Layout::Space | Layout::Skip(_))) {
                self.layout.push(Layout::Space);
            }
            return;
        }
        if self.picture_depth == 0 {
            // text of the paragraph, set word by word around the pictures
            match self.layout.last() {
                Some(Layout::Word(i)) => self.words[*i].1.push(c),
                _ => {
                    self.layout.push(Layout::Word(self.words.len()));
                    self.words.push((self.st.text.font.clone(), c.to_string()));
                }
            }
            return;
        }
        if c.is_whitespace() || c == ';' || self.text_reported {
            return;
        }
        if self.picture_depth > 0 {
            self.text_reported = true;
            self.warn("text outside a statement ignored");
        }
    }

    /// An undefined command: in a picture, the statement it starts is skipped to its `;`.
    pub(super) fn unknown_command(&mut self, name: &str) -> Result<(), Error> {
        if self.picture_depth == 0 && name == "\\" {
            // `\\[2pt]`: the space it asks for is not kept
            if let Some(t) = self.tex.next_nonspace() {
                if t.is_char('[') {
                    let _ = self.tex.read_until_char(']', false)?;
                } else {
                    self.tex.back(t);
                }
            }
            self.layout.push(if self.in_table() { Layout::Row } else { Layout::Break });
            return Ok(());
        }
        if self.picture_depth == 0 && name == " " {
            // `\ `: an interword space, also after a command that eats the spaces after it
            self.layout.push(Layout::Space);
            return Ok(());
        }
        if self.picture_depth == 0 && TEXT_STYLES.contains(&name) {
            // `Ti\emph{k}Z`: set with the word it is in, as a node's text is
            let arg = self.argument()?;
            let text = format!("\\{name}{{{}}}", tex::tokens_to_string(&self.tex.expand_fully(arg)?));
            match self.layout.last() {
                Some(Layout::Word(i)) => self.words[*i].1.push_str(&text),
                _ => {
                    self.layout.push(Layout::Word(self.words.len()));
                    self.words.push((self.st.text.font.clone(), text));
                }
            }
            return Ok(());
        }
        if self.picture_depth > 0 {
            let rest = self.skip_up_to(STATEMENTS)?;
            self.warn(format!("\\{name} not supported: '{}' ignored", rest.chars().take(40).collect::<String>().trim()));
        }
        Ok(())
    }

    /// Reads up to the `;` that ends a statement (outside braces), and returns it as text.
    pub fn skip_statement(&mut self) -> Result<String, Error> {
        self.skip_up_to(&[])
    }

    /// [`Self::skip_statement`], stopping too before any of the commands `next`: a command of a
    /// package may not end in `;`, and the statement after it has nothing to do with it.
    pub(super) fn skip_up_to(&mut self, next: &[&str]) -> Result<String, Error> {
        let mut out = Vec::new();
        let mut depth = 0usize;
        while let Some(t) = self.tex.get_token() {
            match t {
                Token::Char(_, Cat::Begin) => depth += 1,
                Token::Char(_, Cat::End) => {
                    if depth == 0 {
                        self.tex.back(t);
                        break;
                    }
                    depth -= 1;
                }
                Token::Char(';', _) if depth == 0 => break,
                ref x if depth == 0 && next.iter().any(|n| x.is_cs(n)) => {
                    self.tex.back(t);
                    break;
                }
                _ => {}
            }
            out.push(t);
            if out.len() > tex::MAX_TOKENS {
                break;
            }
        }
        Ok(tex::tokens_to_string(&out))
    }

    /// An error in a statement: reported, and the rest of the statement skipped, as TeX goes on after an
    /// error. Limits (endless loops, a figure too large) stop the figure.
    pub fn recover(&mut self, r: Result<(), Error>, t: &Token) -> Result<(), Error> {
        match r {
            Ok(()) => Ok(()),
            Err(e) if is_limit(&e) => Err(e),
            Err(e) => {
                self.warn(format!("{}: {e}; statement ignored", tex::show_token(t)));
                let _ = self.skip_statement();
                Ok(())
            }
        }
    }
}
