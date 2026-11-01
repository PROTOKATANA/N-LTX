//! The commands of TeX itself.

use super::*;
use crate::Error;
use std::rc::Rc;

impl Tex {
    // ---------------------------------------------------------------- the commands of TeX itself

    /// Runs a primitive or a character that the drawing layer does not handle. Returns false if `t` is not one.
    pub fn execute_basic(&mut self, t: Token) -> Result<bool, Error> {
        let m = self.meaning(&t);
        if m.is_expandable() {
            self.expand(&t, &m)?;
            return Ok(true);
        }
        match (&t, &m) {
            (Token::Char(_, Cat::Begin), _) => {
                self.begin_group();
                Ok(true)
            }
            (Token::Char(_, Cat::End), _) => {
                if !self.end_group() {
                    self.warn("extra '}'");
                }
                Ok(true)
            }
            (_, Meaning::Prim(p)) => self.execute_prim(*p, &t),
            (_, Meaning::Count(r)) => {
                let r = *r;
                self.scan_optional_equals()?;
                let v = self.scan_int()?;
                self.set_count(r, v, false);
                Ok(true)
            }
            (_, Meaning::Dimen(r)) => {
                let r = *r;
                self.scan_optional_equals()?;
                let v = self.scan_dimen()?;
                self.set_dimen(r, v, false);
                Ok(true)
            }
            (_, Meaning::Skip(r)) => {
                let r = *r;
                self.scan_optional_equals()?;
                let v = self.scan_dimen()?;
                self.skip_glue_parts()?;
                self.set_dimen(r, v, false);
                Ok(true)
            }
            (_, Meaning::Toks(r)) => {
                let r = *r;
                self.scan_optional_equals()?;
                let v = self.read_general_text()?;
                self.set_toks(r, v, false);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn skip_ignorespaces(&mut self) -> Result<(), Error> {
        while let Some(x) = self.get_x_token()? {
            if !x.is_space() {
                self.back(x);
                break;
            }
        }
        Ok(())
    }

    fn execute_protected(&mut self) -> Result<(), Error> {
        use Prim::*;
        let next = self.get_token().ok_or_else(|| Error::Figure("\\protected at the end".into()))?;
        match self.meaning(&next) {
            Meaning::Prim(Def | Gdef | Edef | Xdef) => {
                let Meaning::Prim(q) = self.meaning(&next) else { unreachable!() };
                self.definition(q, true)?;
            }
            Meaning::Prim(Global | Long) => {
                if self.meaning(&next) == Meaning::Prim(Global) {
                    self.global = true;
                }
                self.back(Token::cs("protected"));
            }
            _ => self.back(next),
        }
        Ok(())
    }

    fn execute_let(&mut self) -> Result<(), Error> {
        let name = self.get_token().ok_or_else(|| Error::Figure("\\let at the end".into()))?;
        // \let\a= b: one optional equals sign and one optional space
        let mut v = self.get_token().ok_or_else(|| Error::Figure("\\let at the end".into()))?;
        while v.is_space() {
            v = self.get_token().ok_or_else(|| Error::Figure("\\let at the end".into()))?;
        }
        if v.is_char('=') {
            v = self.get_token().ok_or_else(|| Error::Figure("\\let at the end".into()))?;
            if v.is_space() {
                v = self.get_token().ok_or_else(|| Error::Figure("\\let at the end".into()))?;
            }
        }
        let m = if v.is_cs("tikra@noexpand") { self.get_token().map(|x| self.meaning(&x)).unwrap_or(Meaning::Undefined) } else { self.meaning(&v) };
        self.set_meaning(&name, m, false);
        Ok(())
    }

    fn execute_register_def(&mut self, p: Prim) -> Result<(), Error> {
        use Prim::*;
        let name = self.get_token().ok_or_else(|| Error::Figure("missing name".into()))?;
        self.scan_optional_equals()?;
        let n = self.scan_int()?;
        let m = match p {
            Countdef => Meaning::Count(n.clamp(0, 65535) as u32),
            Dimendef => Meaning::Dimen(n.clamp(0, 65535) as u32),
            Skipdef => Meaning::Skip(n.clamp(0, 65535) as u32),
            Toksdef => Meaning::Toks(n.clamp(0, 65535) as u32),
            _ => Meaning::CharDef(n),
        };
        self.set_meaning(&name, m, false);
        Ok(())
    }

    fn execute_new_register(&mut self, p: Prim) -> Result<(), Error> {
        use Prim::*;
        let name = self.new_register_name(p)?;
        let r = self.new_register();
        let m = match p {
            Newcount | Newcounter => Meaning::Count(r),
            Newdimen | Newlength => Meaning::Dimen(r),
            Newskip => Meaning::Skip(r),
            _ => Meaning::Toks(r),
        };
        self.global = false;
        self.set_meaning(&name, m, true);
        match p {
            Newcount | Newcounter => {
                self.counts.insert(r, 0);
            }
            Newtoks => {
                self.toks.insert(r, Rc::new(Vec::new()));
            }
            _ => {
                self.dimens.insert(r, 0);
            }
        }
        Ok(())
    }

    /// The control sequence `\newcount` and its kin define, or `\c@name` for `\newcounter`.
    fn new_register_name(&mut self, p: Prim) -> Result<Token, Error> {
        if p != Prim::Newcounter {
            let t = self.read_undelimited()?;
            return t.into_iter().next().ok_or_else(|| Error::Figure("missing name".into()));
        }
        let n = tokens_to_string(&self.read_undelimited()?);
        // \newcounter{name}[parent]
        if let Some(x) = self.next_nonspace() {
            if x.is_char('[') {
                self.read_until_char(']', true)?;
            } else {
                self.back(x);
            }
        }
        // \the<name> prints the counter's value
        let the = vec![Token::cs("arabic"), Token::Char('{', Cat::Begin)];
        let the = the.into_iter().chain(string_tokens(n.trim())).chain([Token::Char('}', Cat::End)]).collect();
        self.define_macro(&format!("the{}", n.trim()), the, true);
        Ok(Token::Cs(Rc::from(format!("c@{}", n.trim()).as_str())))
    }

    fn execute_newif(&mut self) -> Result<(), Error> {
        use Prim::*;
        // \newif\iffoo defines \foofalse, \footrue and \iffoo
        let t = self.get_token().ok_or_else(|| Error::Figure("\\newif at the end".into()))?;
        let Token::Cs(name) = &t else { return Ok(()) };
        let base = name.strip_prefix("if").unwrap_or(name).to_string();
        self.global = false;
        self.set_meaning(&t, Meaning::Prim(Iffalse), true);
        let mk = |v: bool| -> Meaning {
            let body = vec![Body::Tok(Token::cs("let")), Body::Tok(t.clone()), Body::Tok(Token::cs(if v { "iftrue" } else { "iffalse" }))];
            Meaning::Macro(Rc::new(Macro { params: Vec::new(), body, optional: None, protected: false }))
        };
        self.set_meaning(&Token::Cs(Rc::from(format!("{base}true").as_str())), mk(true), true);
        self.set_meaning(&Token::Cs(Rc::from(format!("{base}false").as_str())), mk(false), true);
        Ok(())
    }

    fn execute_addtomacro(&mut self, t: &Token) -> Result<(), Error> {
        // \g@addto@macro\m{tokens}: the tokens put at the end of \m's body, unexpanded, for
        // the whole document; etoolbox's \appto (\preto at the start), with \g… global and
        // \e…, \x… (global) the tokens expanded; babel's \addto is global. An undefined \m
        // is defined.
        let name = show_token(t);
        let name = name.trim_start_matches('\\');
        let target = self.read_undelimited()?;
        let tokens = self.read_undelimited()?;
        let Some(m) = target.into_iter().find(|x| !x.is_space()) else { return Ok(()) };
        let tokens = if name.starts_with(['e', 'x']) { self.expand_fully(tokens)? } else { tokens };
        let global = !matches!(name, "appto" | "preto" | "eappto" | "epreto");
        let (params, old) = match self.meaning(&m) {
            Meaning::Macro(old) => (old.params.clone(), old.body.clone()),
            _ => (Vec::new(), Vec::new()),
        };
        let new = tokens.into_iter().map(Body::Tok);
        let body = if name.ends_with("preto") { new.chain(old).collect() } else { old.into_iter().chain(new).collect() };
        let def = Macro { params, body, optional: None, protected: false };
        self.set_meaning(&m, Meaning::Macro(Rc::new(def)), global);
        Ok(())
    }

    fn execute_setlength(&mut self, p: Prim) -> Result<(), Error> {
        use Prim::Addtolength;
        let target = self.read_undelimited()?;
        let value = self.read_undelimited()?;
        let Some(name) = target.first() else { return Ok(()) };
        let r = match self.meaning(name) {
            Meaning::Dimen(r) | Meaning::Skip(r) => r,
            _ => {
                self.warn(format!("\\setlength: {} is not a length", show_token(name)));
                return Ok(());
            }
        };
        let v = self.dimen_of(value)?;
        let v = if p == Addtolength { self.dimens.get(&r).copied().unwrap_or(0) + v } else { v };
        self.set_dimen(r, v, false);
        Ok(())
    }

    fn execute_setcounter(&mut self, p: Prim) -> Result<(), Error> {
        use Prim::*;
        let name = tokens_to_string(&self.read_undelimited()?);
        let v = if p == Stepcounter {
            1
        } else {
            let value = self.read_undelimited()?;
            self.int_of(value)?
        };
        if let Meaning::Count(r) = self.meaning_of(&format!("c@{}", name.trim())) {
            let v = if p == Setcounter { v } else { self.counts.get(&r).copied().unwrap_or(0) + v };
            self.set_count(r, v, true);
        }
        Ok(())
    }

    fn execute_arithmetic(&mut self, p: Prim) -> Result<(), Error> {
        use Prim::*;
        let target = self.get_x_token()?.ok_or_else(|| Error::Figure("missing register".into()))?;
        let (is_count, r) = match self.meaning(&target) {
            Meaning::Count(r) => (true, r),
            Meaning::Dimen(r) | Meaning::Skip(r) => (false, r),
            Meaning::Prim(Count) => (true, self.scan_register_number()?),
            Meaning::Prim(Dimen | Skip) => (false, self.scan_register_number()?),
            _ => return Err(Error::Figure(format!("you can't use {} after \\advance", show_token(&target)))),
        };
        self.scan_keyword("by")?;
        let old = if is_count { self.counts.get(&r).copied().unwrap_or(0) } else { self.dimens.get(&r).copied().unwrap_or(0) };
        let v = match p {
            Advance => old + if is_count { self.scan_int()? } else { self.scan_dimen()? },
            Multiply => old.saturating_mul(self.scan_int()?),
            _ => {
                let d = self.scan_int()?;
                if d == 0 {
                    return Err(Error::Figure("arithmetic overflow (division by 0)".into()));
                }
                old / d
            }
        };
        if is_count {
            self.set_count(r, v, false);
        } else {
            self.set_dimen(r, v, false);
        }
        Ok(())
    }

    fn execute_change_case(&mut self, p: Prim) -> Result<(), Error> {
        use Prim::Uppercase;
        let body = self.read_general_text()?;
        let out = body
            .into_iter()
            .map(|x| match x {
                Token::Char(c, cat) => Token::Char(if p == Uppercase { c.to_ascii_uppercase() } else { c.to_ascii_lowercase() }, cat),
                other => other,
            })
            .collect::<Vec<_>>();
        self.push_tokens(out);
        Ok(())
    }

    fn execute_setbox(&mut self) -> Result<(), Error> {
        // a box named by an undefined control sequence (pgf's node part boxes) is kept by name
        let named = match self.next_nonspace() {
            Some(Token::Cs(n)) if matches!(self.meaning(&Token::Cs(n.clone())), Meaning::Undefined) => Some(n.to_string()),
            Some(other) => {
                self.back(other);
                let _ = self.scan_int()?;
                None
            }
            None => None,
        };
        self.scan_optional_equals()?;
        // \setbox0=\hbox{…}: the box is measured as empty
        if let Some(x) = self.next_x_nonspace()? {
            if x.is_cs("hbox") || x.is_cs("vbox") {
                let body = self.read_general_text()?;
                if let Some(n) = named {
                    self.named_boxes.retain(|(m, _)| *m != n);
                    self.named_boxes.push((n, body));
                }
            } else if x.is_cs("box") || x.is_cs("copy") {
                // another box register's content: the register read (by name or number)
                match self.next_nonspace() {
                    Some(Token::Cs(_)) | None => {}
                    Some(other) => {
                        self.back(other);
                        let _ = self.scan_int()?;
                    }
                }
            } else {
                self.back(x);
            }
        }
        Ok(())
    }

    fn execute_prim(&mut self, p: Prim, t: &Token) -> Result<bool, Error> {
        use Prim::*;
        match p {
            Relax | Par | Endcsname | Outer | Long | Immediate => {}
            Ignorespaces => self.skip_ignorespaces()?,
            Global => {
                self.global = true;
                // the next command is the assignment; leave the flag for it
            }
            Protected => self.execute_protected()?,
            Def | Gdef | Edef | Xdef => self.definition(p, false)?,
            Let => self.execute_let()?,
            Futurelet => {
                let name = self.get_token().ok_or_else(|| Error::Figure("\\futurelet at the end".into()))?;
                let a = self.get_token().ok_or_else(|| Error::Figure("\\futurelet at the end".into()))?;
                let b = self.get_token().ok_or_else(|| Error::Figure("\\futurelet at the end".into()))?;
                let m = self.meaning(&b);
                self.set_meaning(&name, m, false);
                self.push_tokens([a, b]);
            }
            Begingroup => self.begin_group(),
            Endgroup => {
                if !self.end_group() {
                    self.warn("extra \\endgroup");
                }
            }
            Count => {
                let r = self.scan_register_number()?;
                self.scan_optional_equals()?;
                let v = self.scan_int()?;
                self.set_count(r, v, false);
            }
            Dimen | Skip => {
                let r = self.scan_register_number()?;
                self.scan_optional_equals()?;
                let v = self.scan_dimen()?;
                if p == Skip {
                    self.skip_glue_parts()?;
                }
                self.set_dimen(r, v, false);
            }
            Toks => {
                let r = self.scan_register_number()?;
                self.scan_optional_equals()?;
                let v = self.read_general_text()?;
                self.set_toks(r, v, false);
            }
            Countdef | Dimendef | Skipdef | Toksdef | Chardef | Mathchardef => self.execute_register_def(p)?,
            Newcount | Newdimen | Newskip | Newlength | Newtoks | Newcounter => self.execute_new_register(p)?,
            Newif => self.execute_newif()?,
            Newboolean => {
                let n = tokens_to_string(&self.read_undelimited()?);
                self.push_tokens([Token::cs("newif"), Token::Cs(Rc::from(format!("if{}", n.trim()).as_str()))]);
            }
            Setboolean => {
                let n = tokens_to_string(&self.read_undelimited()?);
                let v = tokens_to_string(&self.read_undelimited()?);
                self.push_tokens([Token::Cs(Rc::from(format!("{}{}", n.trim(), v.trim().to_lowercase()).as_str()))]);
            }
            Xstring => self.xstring(show_token(t).trim_start_matches('\\'))?,
            Addtomacro => self.execute_addtomacro(t)?,
            Setlength | Addtolength => self.execute_setlength(p)?,
            Setcounter | Addtocounter | Stepcounter => self.execute_setcounter(p)?,
            Advance | Multiply | Divide => self.execute_arithmetic(p)?,
            Catcode => {
                let c = self.scan_int()?;
                self.scan_optional_equals()?;
                let v = self.scan_int()?;
                if let (Some(ch), Some(cat)) = (char::from_u32(c.max(0) as u32), Cat::from_code(v)) {
                    self.set_catcode(ch, cat);
                }
            }
            Makeatletter => self.set_catcode('@', Cat::Letter),
            Makeatother => self.set_catcode('@', Cat::Other),
            Afterassignment => {
                self.after_assignment = self.get_token();
            }
            Aftergroup => {
                if let Some(x) = self.get_token() {
                    if let Some(level) = self.after_group.last_mut() {
                        level.push(x);
                    }
                }
            }
            Message | Typeout | Errmessage | Showtokens | Write => {
                if p == Write {
                    let _ = self.scan_int();
                }
                let _ = self.read_undelimited()?;
            }
            Show | Showthe => {
                let _ = self.get_token();
            }
            Numexpr | Dimexpr | Glueexpr => {
                // outside \the or a comparison: read and dropped
                let _ = if p == Numexpr { self.scan_numexpr()? } else { self.scan_dimexpr()? };
            }
            Newcommand | Renewcommand | Providecommand | Declarerobustcommand => self.newcommand(p)?,
            Newdocumentcommand => self.new_document_command(t)?,
            Newenvironment | Renewenvironment => self.newenvironment()?,
            Uppercase | Lowercase => self.execute_change_case(p)?,
            Setbox => self.execute_setbox()?,
            Hbox => {
                let _ = self.scan_keyword("to").and_then(|to| if to { self.scan_dimen().map(|_| ()) } else { Ok(()) });
                let body = self.read_general_text()?;
                self.push_tokens(body);
            }
            Wd | Ht | Dp => {
                let n = self.scan_int()?;
                self.scan_optional_equals()?;
                let v = self.scan_dimen()?;
                let k = match p {
                    Wd => 0,
                    Ht => 1,
                    _ => 2,
                };
                self.box_dims.entry(n).or_default()[k] = v;
            }
            _ => return Ok(false),
        }
        let _ = t;
        Ok(true)
    }

    /// Reads and drops what is left above `depth` (after reading a value from a pushed list).
    pub fn drain_to(&mut self, depth: usize) {
        while self.input.len() > depth {
            if self.get_token_until(depth).is_none() {
                break;
            }
        }
    }

    /// `\def\name<parameter text>{body}`, and `\edef` (body expanded at once).
    fn definition(&mut self, p: Prim, protected: bool) -> Result<(), Error> {
        let global = std::mem::take(&mut self.global) || matches!(p, Prim::Gdef | Prim::Xdef);
        let name = self.get_token().ok_or_else(|| Error::Figure("\\def at the end".into()))?;
        let mut params = Vec::new();
        let mut n = 0u8;
        loop {
            let t = self.get_token().ok_or_else(|| Error::Figure(format!("definition of {} without a body", show_token(&name))))?;
            match t {
                Token::Char(_, Cat::Begin) => break,
                Token::Char(_, Cat::Param) => {
                    let d = self.get_token().ok_or_else(|| Error::Figure("# at the end of a parameter text".into()))?;
                    match d {
                        Token::Char(c, _) if c.is_ascii_digit() => {
                            n += 1;
                            params.push(Param::Arg(c as u8 - b'0'));
                        }
                        // #{ : the last parameter is delimited by the opening brace, which stays
                        Token::Char(_, Cat::Begin) => {
                            params.push(Param::Tok(Token::Char('{', Cat::Begin)));
                            break;
                        }
                        other => params.push(Param::Tok(other)),
                    }
                }
                other => params.push(Param::Tok(other)),
            }
        }
        let raw = self.read_group_body()?;
        let raw = if matches!(p, Prim::Edef | Prim::Xdef) { self.expand_fully(raw)? } else { raw };
        let body = parse_body(raw);
        let _ = n;
        let m = Macro { params, body, optional: None, protected };
        self.global = false;
        self.set_meaning(&name, Meaning::Macro(Rc::new(m)), global);
        Ok(())
    }

    /// `\newcommand{\name}[n][default]{body}` (also `\newcommand*`, `\renewcommand`, `\providecommand`).
    fn newcommand(&mut self, p: Prim) -> Result<(), Error> {
        let mut t = self.next_nonspace().ok_or_else(|| Error::Figure("\\newcommand at the end".into()))?;
        if t.is_char('*') {
            t = self.next_nonspace().ok_or_else(|| Error::Figure("\\newcommand at the end".into()))?;
        }
        let name = if matches!(t, Token::Char(_, Cat::Begin)) {
            let body = self.read_group_body()?;
            body.into_iter().find(|x| !x.is_space()).ok_or_else(|| Error::Figure("\\newcommand without a name".into()))?
        } else {
            t
        };
        let mut n = 0u8;
        let mut optional = None;
        if let Some(x) = self.next_nonspace() {
            if x.is_char('[') {
                let k = self.read_until_char(']', true)?;
                n = tokens_to_string(&k).trim().parse().unwrap_or(0);
                if let Some(y) = self.next_nonspace() {
                    if y.is_char('[') {
                        optional = Some(self.read_until_char(']', false)?);
                    } else {
                        self.back(y);
                    }
                }
            } else {
                self.back(x);
            }
        }
        let body = self.read_undelimited()?;
        if p == Prim::Providecommand && !matches!(self.meaning(&name), Meaning::Undefined) {
            return Ok(());
        }
        let undelimited = if optional.is_some() { n.saturating_sub(1) } else { n };
        let first = if optional.is_some() { 2 } else { 1 };
        let params = (0..undelimited).map(|k| Param::Arg(first + k)).collect();
        let m = Macro { params, body: parse_body(body), optional, protected: false };
        self.set_meaning(&name, Meaning::Macro(Rc::new(m)), false);
        Ok(())
    }

    /// `\NewDocumentCommand\name{spec}{body}` (xparse, in the LaTeX kernel): `\name` reads its
    /// arguments by `spec`, then runs `body` with them as `#1`, `#2`…. The command is
    /// `\tikra@documentarguments{spec}` and a macro of undelimited parameters for the body.
    fn new_document_command(&mut self, t: &Token) -> Result<(), Error> {
        let kind = match t {
            Token::Cs(n) => n.to_string(),
            _ => String::new(),
        };
        let first = self.next_nonspace().ok_or_else(|| Error::Figure(format!("{} at the end", show_token(t))))?;
        let name = if matches!(first, Token::Char(_, Cat::Begin)) {
            let body = self.read_group_body()?;
            body.into_iter().find(|x| !x.is_space()).ok_or_else(|| Error::Figure(format!("{} without a name", show_token(t))))?
        } else {
            first
        };
        let spec = self.read_undelimited()?;
        let body = self.read_undelimited()?;
        let defined = !matches!(self.meaning(&name), Meaning::Undefined);
        if kind.starts_with("Provide") && defined {
            return Ok(());
        }
        let n = parse_arg_spec(&spec, &mut self.warnings).len().min(9) as u8;
        let inner = Token::Cs(Rc::from(format!("tikra@document@{}", show_token(&name).trim_start_matches('\\')).as_str()));
        let params = (1..=n).map(Param::Arg).collect();
        self.set_meaning(&inner, Meaning::Macro(Rc::new(Macro { params, body: parse_body(body), optional: None, protected: false })), false);
        let mut call = vec![Token::cs("tikra@documentarguments"), Token::Char('{', Cat::Begin)];
        call.extend(spec);
        call.extend([Token::Char('}', Cat::End), inner]);
        let m = Macro { params: Vec::new(), body: call.into_iter().map(Body::Tok).collect(), optional: None, protected: !kind.contains("Expandable") };
        self.set_meaning(&name, Meaning::Macro(Rc::new(m)), false);
        Ok(())
    }

    /// `\tikra@documentarguments{spec}\inner`: reads a document command's arguments from the
    /// input and expands `\inner`'s body with them.
    pub(super) fn document_arguments(&mut self) -> Result<(), Error> {
        let spec = self.read_undelimited()?;
        let inner = self.get_token().ok_or_else(|| Error::Figure("a document command without its body".into()))?;
        let Meaning::Macro(m) = self.meaning(&inner) else { return Ok(()) };
        let mut args = Vec::new();
        let novalue = || vec![Token::cs("tikra@novalue")];
        for a in parse_arg_spec(&spec, &mut Vec::new()) {
            let v = match a {
                ArgSpec::Mandatory => self.read_undelimited()?,
                ArgSpec::Optional(open, close, default) => match self.next_nonspace() {
                    Some(x) if x.is_char(open) => self.read_until_char(close, true)?,
                    other => {
                        if let Some(x) = other {
                            self.back(x);
                        }
                        default.unwrap_or_else(novalue)
                    }
                },
                ArgSpec::Required(open, close, default) => match self.next_nonspace() {
                    Some(x) if x.is_char(open) => self.read_until_char(close, true)?,
                    other => {
                        if let Some(x) = other {
                            self.back(x);
                        }
                        self.warnings.push(format!("{} is missing its argument in '{open}{close}'", show_token(&inner).replace("tikra@document@", "")));
                        default.unwrap_or_else(novalue)
                    }
                },
                ArgSpec::Flag(c) => match self.next_nonspace() {
                    Some(x) if x.is_char(c) => vec![Token::cs("BooleanTrue")],
                    other => {
                        if let Some(x) = other {
                            self.back(x);
                        }
                        vec![Token::cs("BooleanFalse")]
                    }
                },
            };
            args.push(v);
        }
        args.truncate(9);
        let body = substitute(&m.body, &args);
        self.push_tokens(body);
        Ok(())
    }

    /// `\newenvironment{name}[n][default]{begin}{end}`.
    fn newenvironment(&mut self) -> Result<(), Error> {
        let mut t = self.next_nonspace().ok_or_else(|| Error::Figure("\\newenvironment at the end".into()))?;
        if t.is_char('*') {
            t = self.next_nonspace().ok_or_else(|| Error::Figure("\\newenvironment at the end".into()))?;
        }
        self.back(t);
        let name = tokens_to_string(&self.read_undelimited()?).trim().to_string();
        let mut n = 0u8;
        let mut optional = None;
        if let Some(x) = self.next_nonspace() {
            if x.is_char('[') {
                n = tokens_to_string(&self.read_until_char(']', true)?).trim().parse().unwrap_or(0);
                if let Some(y) = self.next_nonspace() {
                    if y.is_char('[') {
                        optional = Some(self.read_until_char(']', false)?);
                    } else {
                        self.back(y);
                    }
                }
            } else {
                self.back(x);
            }
        }
        let begin = self.read_undelimited()?;
        let end = self.read_undelimited()?;
        let undelimited = if optional.is_some() { n.saturating_sub(1) } else { n };
        let first = if optional.is_some() { 2 } else { 1 };
        let params = (0..undelimited).map(|k| Param::Arg(first + k)).collect();
        let m = Macro { params, body: parse_body(begin), optional, protected: false };
        self.environments.insert(name, (Rc::new(m), end));
        Ok(())
    }

    /// Expands a `\newenvironment` begin code with its arguments read from the input.
    pub fn environment_begin(&mut self, name: &str) -> Result<Option<Vec<Token>>, Error> {
        let Some((m, end)) = self.environments.get(name).cloned() else { return Ok(None) };
        let args = self.read_arguments(&Token::cs(name), &m)?;
        let _ = end;
        Ok(Some(substitute(&m.body, &args)))
    }
}

/// `#1` → argument, `##` → `#`.
/// One argument of a document command's specification.
#[derive(Debug, Clone, PartialEq)]
enum ArgSpec {
    /// `m`
    Mandatory,
    /// `o`, `O{default}`, `d<>`, `D<>{default}`: between the two characters if they are there
    Optional(char, char, Option<Vec<Token>>),
    /// `r<>`, `R<>{default}`
    Required(char, char, Option<Vec<Token>>),
    /// `s` (a star), `t+`: whether the character is there
    Flag(char),
}

/// Reads a document command's argument specification (`O{} O{0.2cm} m m m`); the prefixes `+`,
/// `!` and `>{…}` change nothing here, and unknown types are read as `m` with a warning.
fn parse_arg_spec(spec: &[Token], warnings: &mut Vec<String>) -> Vec<ArgSpec> {
    let mut out = Vec::new();
    let mut i = 0;
    // the next token that is not a space
    let next = |i: &mut usize| -> Option<Token> {
        while spec.get(*i).is_some_and(|t| t.is_space()) {
            *i += 1;
        }
        let t = spec.get(*i).cloned();
        *i += 1;
        t
    };
    let char_of = |t: Option<Token>| match t {
        Some(Token::Char(c, _)) => Some(c),
        _ => None,
    };
    // a `{…}` group, without its braces; nothing if there is none
    let group = |i: &mut usize| -> Vec<Token> {
        let mut j = *i;
        if !matches!(next(&mut j), Some(Token::Char(_, Cat::Begin))) {
            return Vec::new();
        }
        let mut depth = 1;
        let mut inside = Vec::new();
        while let Some(t) = spec.get(j) {
            j += 1;
            match t {
                Token::Char(_, Cat::Begin) => depth += 1,
                Token::Char(_, Cat::End) => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            inside.push(t.clone());
        }
        *i = j;
        inside
    };
    while i < spec.len() {
        let Some(c) = char_of(next(&mut i)) else { continue };
        match c {
            '+' | '!' => {}
            '>' => {
                group(&mut i);
            }
            'm' => out.push(ArgSpec::Mandatory),
            'o' => out.push(ArgSpec::Optional('[', ']', None)),
            'O' => out.push(ArgSpec::Optional('[', ']', Some(group(&mut i)))),
            's' => out.push(ArgSpec::Flag('*')),
            't' => out.push(ArgSpec::Flag(char_of(next(&mut i)).unwrap_or('*'))),
            'd' | 'D' | 'r' | 'R' => {
                let a = char_of(next(&mut i)).unwrap_or('[');
                let b = char_of(next(&mut i)).unwrap_or(']');
                let default = c.is_uppercase().then(|| group(&mut i));
                out.push(if c.eq_ignore_ascii_case(&'d') { ArgSpec::Optional(a, b, default) } else { ArgSpec::Required(a, b, default) });
            }
            other => {
                warnings.push(format!("document command argument type '{other}' not supported: read as 'm'"));
                out.push(ArgSpec::Mandatory);
            }
        }
    }
    out
}
