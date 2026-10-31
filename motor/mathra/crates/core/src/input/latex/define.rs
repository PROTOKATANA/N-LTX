//! Macros defined in the equation itself (\newcommand, \def, \let).

use super::*;

impl Reader<'_> {
    /// Replaces `self.c[start..self.pos]` with `text` and resumes reading at `start`.
    /// In tolerant mode, an expansion beyond the limit disappears from the text with a warning.
    pub(super) fn inject(&mut self, start: usize, text: &str, col: usize) -> R<()> {
        let replacement: Vec<char> = text.chars().collect();
        let end = self.pos;
        // what was already read is not read again: a text that fits in the replaced span goes to its
        // end, without moving the rest of the input (\bgroup repeated a thousand times does not go quadratic)
        if replacement.len() <= end - start {
            let start = end - replacement.len();
            self.c[start..end].copy_from_slice(&replacement);
            self.pos = start;
            return Ok(());
        }
        // growing moves the rest of the input: expansions that grow count against the limit too
        self.macros.expansions += 1;
        let excess = self.macros.expansions > EXPANSION_LIMIT || self.c.len() + replacement.len() > self.macros.max_size;
        let replacement = if excess {
            if !self.macros.warned_excess {
                self.tolerate(rules::E_REC_1, col, "too many macros: a macro probably calls itself", "expansions beyond the limit were discarded")?;
                self.macros.warned_excess = true;
            }
            Vec::new()
        } else {
            replacement
        };
        self.c.splice(start..end, replacement);
        self.pos = start;
        Ok(())
    }

    pub(super) fn expand(&mut self, start: usize, m: &Macro, col: usize) -> R<()> {
        self.macros.expansions += 1;
        if let Some((open, close)) = &m.paired {
            // \abs{x} → \mathopen{|}x\mathclose{|};  \abs*{x} → \left|x\right|
            let star = self.current() == Some('*');
            if star {
                self.pos += 1;
            }
            let size = self.raw_optional();
            let x = self.raw_text()?;
            let text = match (star, size) {
                (true, _) => format!("\\left{open} {x} \\right{close}"),
                (false, Some(t)) => format!("{t}l{open} {x} {t}r{close}"),
                (false, None) => format!("\\mathopen{{{open}}} {x} \\mathclose{{{close}}}"),
            };
            return self.inject(start, &text, col);
        }
        let mut args = Vec::new();
        if let Some(default_arg) = &m.default_arg {
            args.push(self.raw_optional().unwrap_or_else(|| default_arg.clone()));
        }
        while args.len() < m.arguments {
            args.push(self.raw_text()?);
        }
        let mut body = String::new();
        let c: Vec<char> = m.body.chars().collect();
        let mut i = 0;
        while i < c.len() {
            if c[i] == '#' && i + 1 < c.len() {
                if let Some(d) = c[i + 1].to_digit(10) {
                    // #0 does not exist in TeX: it disappears, like a missing argument
                    if let Some(a) = (d as usize).checked_sub(1).and_then(|k| args.get(k)) {
                        body.push('{');
                        body.push_str(a);
                        body.push('}');
                    }
                    i += 2;
                    continue;
                }
            }
            body.push(c[i]);
            i += 1;
        }
        // a space keeps the macro from sticking to what follows (\foo x)
        body.push(' ');
        self.inject(start, &body, col)
    }

    /// Name being defined: `{\foo}` or `\foo`.
    pub(super) fn defined_name(&mut self, col: usize) -> R<String> {
        self.skip_spaces();
        let raw = if self.current() == Some('{') {
            self.raw_text()?
        } else if self.current() == Some('\\') {
            self.pos += 1;
            format!("\\{}", self.read_command_name())
        } else {
            return self.error(col, "missing macro name");
        };
        let name = raw.trim().trim_start_matches('\\').to_string();
        if name.is_empty() {
            return self.error(col, "empty macro name");
        }
        Ok(name)
    }

    /// Commands that define macros. Returns `true` if it handled the command.
    pub(super) fn definition(&mut self, name: &str, col: usize) -> R<bool> {
        match name {
            "newcommand" | "renewcommand" | "providecommand" | "DeclareRobustCommand" => self.define_newcommand(name, col),
            "def" | "gdef" | "edef" | "xdef" => self.define_def(col),
            "let" => self.define_let(col),
            "DeclareMathOperator" => self.define_operator(col),
            "DeclarePairedDelimiter" => {
                let target = self.defined_name(col)?;
                let open = self.raw_text()?.trim().to_string();
                let close = self.raw_text()?.trim().to_string();
                self.macros.commands.insert(target, Macro { arguments: 1, default_arg: None, body: String::new(), paired: Some((open, close)) });
                Ok(true)
            }
            "newenvironment" | "renewenvironment" => self.define_environment(col),
            "usepackage" | "RequirePackage" | "require" => {
                self.raw_optional();
                let package = self.raw_text()?;
                self.registry.note(rules::L_ENT_3, format!("\\{name}{{{package}}} ignored: the supported packages are already loaded"));
                Ok(true)
            }
            "makeatletter" | "makeatother" | "ignorespaces" | "leavevmode" | "sc" | "scshape" | "upshape" | "itshape" | "bfseries" | "mdseries" | "rmfamily" | "sffamily"
            | "ttfamily" => Ok(true),
            "aftergroup" => self.discard_aftergroup(),
            _ => Ok(false),
        }
    }

    /// `\newcommand`, `\renewcommand`, `\providecommand` and `\DeclareRobustCommand`.
    fn define_newcommand(&mut self, name: &str, col: usize) -> R<bool> {
        if self.current() == Some('*') {
            self.pos += 1;
        }
        let target = self.defined_name(col)?;
        let arguments = match self.raw_optional() {
            Some(n) => n.trim().parse::<usize>().map_err(|_| Error::Syntax { column: col, message: format!("invalid number of arguments: {n}") })?,
            None => 0,
        };
        let default_arg = self.raw_optional();
        let body = self.raw_text()?;
        if name == "providecommand" && self.macros.commands.contains_key(&target) {
            return Ok(true);
        }
        self.registry.note(rules::L_MAC_1, format!("macro \\{target} defined with {arguments} argument(s)"));
        self.macros.commands.insert(target, Macro { arguments, default_arg, body, paired: None });
        Ok(true)
    }

    /// `\def` and its global and expanded siblings. Only `#1#2…` parameters, not delimited ones.
    fn define_def(&mut self, col: usize) -> R<bool> {
        let target = self.defined_name(col)?;
        let mut arguments = 0;
        while self.current() != Some('{') {
            match (self.current(), self.em(1)) {
                (Some('#'), Some(d)) if d.is_ascii_digit() => {
                    arguments += 1;
                    self.pos += 2;
                }
                (Some(c), _) if c.is_whitespace() => self.pos += 1,
                (None, _) => return self.error(col, format!("\\def\\{target} without a body")),
                _ => return self.error(col, "\\def with delimited parameters is not supported (only #1#2…)"),
            }
        }
        let body = self.raw_text()?;
        self.registry.note(rules::L_MAC_1, format!("macro \\{target} defined with {arguments} argument(s)"));
        self.macros.commands.insert(target, Macro { arguments, default_arg: None, body, paired: None });
        Ok(true)
    }

    /// `\let\target=\meaning`. A known macro is copied; otherwise the meaning is one token.
    fn define_let(&mut self, col: usize) -> R<bool> {
        let target = self.defined_name(col)?;
        self.skip_spaces();
        if self.current() == Some('=') {
            self.pos += 1;
            self.skip_spaces();
        }
        let body = if self.current() == Some('\\') {
            self.pos += 1;
            let n = self.read_command_name();
            match self.macros.commands.get(&n).cloned() {
                Some(m) => {
                    self.macros.commands.insert(target, m);
                    return Ok(true);
                }
                None => format!("\\{n}"),
            }
        } else {
            let c = self.current().ok_or(Error::Syntax { column: col, message: "\\let without a target".into() })?;
            self.pos += 1;
            c.to_string()
        };
        self.macros.commands.insert(target, Macro { arguments: 0, default_arg: None, body, paired: None });
        Ok(true)
    }

    /// `\DeclareMathOperator` and its star, which keeps limits.
    fn define_operator(&mut self, col: usize) -> R<bool> {
        let star = self.current() == Some('*');
        if star {
            self.pos += 1;
        }
        let target = self.defined_name(col)?;
        let text = self.raw_text()?;
        let body = format!("\\operatorname{}{{{text}}}", if star { "*" } else { "" });
        self.registry.note(rules::L_MAC_1, format!("operator \\{target} defined as '{text}'"));
        self.macros.commands.insert(target, Macro { arguments: 0, default_arg: None, body, paired: None });
        Ok(true)
    }

    /// `\newenvironment` without arguments. The begin and end texts are stored as a pair.
    fn define_environment(&mut self, col: usize) -> R<bool> {
        let target = self.raw_text()?.trim().to_string();
        if let Some(n) = self.raw_optional() {
            if n.trim() != "0" {
                return self.error(col, "\\newenvironment with arguments is not supported");
            }
        }
        let open = self.raw_text()?;
        let close = self.raw_text()?;
        self.macros.environments.insert(target, (open, close));
        Ok(true)
    }

    /// `\aftergroup` defers the next token. Here that token is discarded.
    fn discard_aftergroup(&mut self) -> R<bool> {
        self.skip_spaces();
        if self.current() == Some('\\') {
            self.pos += 1;
            self.read_command_name();
        } else if self.current().is_some() {
            self.pos += 1;
        }
        Ok(true)
    }

    /// Raw content up to the matching `\end{name}` (which is consumed).
    pub(super) fn environment_body(&mut self, name: &str, col: usize) -> R<String> {
        let open: Vec<char> = format!("\\begin{{{name}}}").chars().collect();
        let close: Vec<char> = format!("\\end{{{name}}}").chars().collect();
        let start = self.pos;
        let mut level = 1;
        let mut i = start;
        while i < self.c.len() {
            if self.c[i..].starts_with(&open) {
                level += 1;
                i += open.len();
            } else if self.c[i..].starts_with(&close) {
                level -= 1;
                if level == 0 {
                    self.pos = i + close.len();
                    return Ok(self.c[start..i].iter().collect());
                }
                i += close.len();
            } else {
                i += 1;
            }
        }
        self.tolerate(rules::E_REC_5, col, format!("\\begin{{{name}}} without \\end{{{name}}}"), "closed at the end")?;
        self.pos = self.c.len();
        Ok(self.c[start..].iter().collect())
    }

    /// A braced argument, if one comes next (after spaces).
    pub(super) fn optional_braced(&mut self) -> R<Option<String>> {
        let saved = self.pos;
        self.skip_spaces();
        if self.current() == Some('{') {
            return Ok(Some(self.raw_text()?));
        }
        self.pos = saved;
        Ok(None)
    }

    /// Content between `open` and its matching `close` (the opening one is the current character).
    pub(super) fn raw_delimited(&mut self, open: char, close: char) -> String {
        self.pos += 1;
        let mut level = 0;
        let mut s = String::new();
        while let Some(c) = self.current() {
            self.pos += 1;
            if c == close && level == 0 {
                return s;
            }
            if c == open && open != close {
                level += 1;
            } else if c == close {
                level -= 1;
            }
            s.push(c);
        }
        s
    }

    /// Is the `\\qty` here physics' (brackets) and not siunitx's (`{value}{unit}`)?
    pub(super) fn physics_qty_follows(&mut self) -> bool {
        let saved = self.pos;
        self.skip_spaces();
        let physics = match self.current() {
            Some('(' | '|' | '<') => true,
            // \qty[options]{value}{unit} is siunitx's; physics's \qty[…] has no braces after it
            Some('[') => {
                let _ = self.raw_optional();
                self.skip_spaces();
                self.current() != Some('{') || {
                    let _ = self.raw_text();
                    self.skip_spaces();
                    self.current() != Some('{')
                }
            }
            Some('{') => {
                let _ = self.raw_text();
                self.skip_spaces();
                self.current() != Some('{')
            }
            _ => false,
        };
        self.pos = saved;
        physics
    }

    /// physics' automatic brackets: `(…)`, `[…]`, `|…|`, `{…}` or `<…>` sized by `\left…\right`.
    pub(super) fn physics_brackets(&mut self) -> R<Option<String>> {
        self.skip_spaces();
        let (open, close, left, right) = match self.current() {
            Some('(') => ('(', ')', "(", ")"),
            Some('[') => ('[', ']', "[", "]"),
            Some('|') => ('|', '|', "|", "|"),
            Some('<') => ('<', '>', "\\langle", "\\rangle"),
            Some('{') => {
                let x = self.raw_text()?;
                return Ok(Some(format!("\\left\\{{ {x} \\right\\}}")));
            }
            _ => return Ok(None),
        };
        let x = self.raw_delimited(open, close);
        Ok(Some(format!("\\left{left} {x} \\right{right}")))
    }
}
