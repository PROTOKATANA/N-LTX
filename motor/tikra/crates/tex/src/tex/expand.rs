//! Expansion: macros, conditionals and `\the`.

use super::*;
use crate::Error;
use std::rc::Rc;

impl Tex {
    // ---------------------------------------------------------------- expansion

    fn count_expansion(&mut self) -> Result<(), Error> {
        self.expansions += 1;
        self.statement_expansions += 1;
        if self.expansions > 10 * MAX_EXPANSIONS {
            return Err(Error::Figure(format!("figure too large: more than {} macro expansions", 10 * MAX_EXPANSIONS)));
        }
        if self.statement_expansions > MAX_EXPANSIONS {
            return Err(Error::Figure(format!("macro expansion did not end (more than {MAX_EXPANSIONS} steps)")));
        }
        Ok(())
    }

    /// A new statement starts: its expansions are counted afresh (the figure's total still is).
    pub fn new_statement(&mut self) {
        self.statement_expansions = 0;
    }

    /// The next token after expanding macros and expandable primitives.
    pub fn get_x_token(&mut self) -> Result<Option<Token>, Error> {
        loop {
            let Some(t) = self.get_token() else { return Ok(None) };
            let m = self.meaning(&t);
            if m.is_expandable() {
                self.expand(&t, &m)?;
                continue;
            }
            return Ok(Some(t));
        }
    }

    /// Expands `t` one level, pushing the result back on the input.
    pub fn expand(&mut self, t: &Token, m: &Meaning) -> Result<(), Error> {
        self.count_expansion()?;
        match m {
            Meaning::Macro(mac) => {
                let args = self.read_arguments(t, mac)?;
                let body = substitute(&mac.body, &args);
                self.push_tokens(body);
                Ok(())
            }
            Meaning::Prim(p) => self.expand_prim(*p, t),
            _ => Ok(()),
        }
    }

    /// Reads a macro's arguments from the input, matching its parameter text.
    pub(super) fn read_arguments(&mut self, name: &Token, m: &Macro) -> Result<Vec<Vec<Token>>, Error> {
        let mut args: Vec<Vec<Token>> = Vec::new();
        if let Some(default) = &m.optional {
            // LaTeX: an optional first argument in brackets
            let t = self.next_nonspace();
            match t {
                Some(t) if t.is_char('[') => args.push(self.read_until_char(']', true)?),
                Some(t) => {
                    self.back(t);
                    args.push(default.clone());
                }
                None => args.push(default.clone()),
            }
        }
        let mut i = 0;
        let p = &m.params;
        while i < p.len() {
            match &p[i] {
                Param::Tok(t) => {
                    let got = self.get_token().ok_or_else(|| Error::Figure(format!("use of {} does not match its definition", show_token(name))))?;
                    if &got != t {
                        return Err(Error::Figure(format!("use of {} does not match its definition", show_token(name))));
                    }
                    i += 1;
                }
                Param::Arg(_) => {
                    // delimited by the tokens up to the next parameter (or the end)
                    let mut delim = Vec::new();
                    let mut k = i + 1;
                    while k < p.len() {
                        match &p[k] {
                            Param::Tok(t) => delim.push(t.clone()),
                            Param::Arg(_) => break,
                        }
                        k += 1;
                    }
                    if delim.is_empty() {
                        args.push(self.read_undelimited()?);
                    } else {
                        args.push(self.read_delimited(&delim, name)?);
                    }
                    i = k;
                }
            }
        }
        Ok(args)
    }

    /// An undelimited argument: one token, or a braced group without its braces (spaces before are skipped).
    pub fn read_undelimited(&mut self) -> Result<Vec<Token>, Error> {
        let t = self.next_nonspace_par().ok_or_else(|| Error::Figure("a command is missing its argument".into()))?;
        if matches!(t, Token::Char(_, Cat::Begin)) {
            return self.read_group_body();
        }
        Ok(vec![t])
    }

    fn next_nonspace_par(&mut self) -> Option<Token> {
        loop {
            let t = self.get_token()?;
            if !t.is_space() && !t.is_cs("par") {
                return Some(t);
            }
        }
    }

    pub fn next_nonspace(&mut self) -> Option<Token> {
        loop {
            let t = self.get_token()?;
            if !t.is_space() {
                return Some(t);
            }
        }
    }

    /// The rest of a group whose `{` was just read, up to its matching `}` (not included).
    pub fn read_group_body(&mut self) -> Result<Vec<Token>, Error> {
        let mut out = Vec::new();
        let mut depth = 1usize;
        loop {
            let t = self.get_token().ok_or_else(|| Error::Figure("unmatched brace: a group '{' never closes".into()))?;
            match t {
                Token::Char(_, Cat::Begin) => depth += 1,
                Token::Char(_, Cat::End) => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(out);
                    }
                }
                _ => {}
            }
            out.push(t);
            if out.len() > MAX_TOKENS {
                return Err(Error::Figure("argument too long".into()));
            }
        }
    }

    /// Tokens up to `delim` at group level 0; one pair of braces around the whole argument is removed, as TeX does.
    fn read_delimited(&mut self, delim: &[Token], name: &Token) -> Result<Vec<Token>, Error> {
        let mut out: Vec<Token> = Vec::new();
        let mut depth = 0usize;
        loop {
            let t = self.get_token().ok_or_else(|| Error::Figure(format!("{} ends before its delimiter", show_token(name))))?;
            match t {
                Token::Char(_, Cat::Begin) => depth += 1,
                Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
                _ => {}
            }
            out.push(t);
            if depth == 0 && out.ends_with(delim) {
                out.truncate(out.len() - delim.len());
                return Ok(strip_outer_braces(out));
            }
            if out.len() > MAX_TOKENS {
                return Err(Error::Figure(format!("{} ends before its delimiter", show_token(name))));
            }
        }
    }

    /// Tokens up to the character `c` at brace level 0 (for `[…]`), the `c` consumed.
    pub fn read_until_char(&mut self, c: char, strip: bool) -> Result<Vec<Token>, Error> {
        let mut out = Vec::new();
        let mut depth = 0usize;
        loop {
            let t = self.get_token().ok_or_else(|| Error::Figure(if c == ']' { "unmatched bracket: ']' missing".into() } else { format!("'{c}' missing") }))?;
            match t {
                Token::Char(_, Cat::Begin) => depth += 1,
                Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
                ref x if depth == 0 && x.is_char(c) => return Ok(if strip { strip_outer_braces(out) } else { out }),
                _ => {}
            }
            out.push(t);
            if out.len() > MAX_TOKENS {
                return Err(Error::Figure(if c == ']' { "unmatched bracket: ']' missing".into() } else { format!("'{c}' missing") }));
            }
        }
    }

    fn expand_expandafter(&mut self) -> Result<(), Error> {
        let a = self.get_token().ok_or_else(|| Error::Figure("\\expandafter at the end".into()))?;
        if let Some(b) = self.get_token() {
            let m = self.meaning(&b);
            if m.is_expandable() {
                self.expand(&b, &m)?;
            } else {
                self.back(b);
            }
        }
        self.back(a);
        Ok(())
    }

    fn expand_noexpand(&mut self) -> Result<(), Error> {
        // the next token, marked so it is not expanded this time: \relax-like when it was expandable
        if let Some(n) = self.get_token() {
            if self.meaning(&n).is_expandable() {
                self.push_tokens([Token::cs("tikra@noexpand"), n]);
            } else {
                self.back(n);
            }
        }
        Ok(())
    }

    fn expand_csname(&mut self) -> Result<(), Error> {
        use Prim::Relax;
        let mut name = String::new();
        loop {
            let x = self.get_x_token()?.ok_or_else(|| Error::Figure("\\csname without \\endcsname".into()))?;
            match x {
                Token::Cs(ref n) if &**n == "endcsname" => break,
                Token::Cs(ref n) => name.push_str(&format!("\\{n}")),
                Token::Char(c, _) => name.push(c),
            }
            if name.len() > 10_000 {
                return Err(Error::Figure("\\csname too long".into()));
            }
        }
        let tok = Token::Cs(Rc::from(name.as_str()));
        if matches!(self.meaning(&tok), Meaning::Undefined) {
            // \csname makes an undefined name mean \relax
            self.meanings.insert(Rc::from(name.as_str()), Meaning::Prim(Relax));
        }
        self.back(tok);
        Ok(())
    }

    fn expand_unexpanded(&mut self) -> Result<(), Error> {
        let body = self.read_general_text()?;
        // marked as not to expand in \edef
        let mut out = Vec::with_capacity(body.len() * 2);
        for b in body {
            out.push(Token::cs("tikra@noexpand"));
            out.push(b);
        }
        self.push_tokens(out);
        Ok(())
    }

    fn expand_pdfstrcmp(&mut self) -> Result<(), Error> {
        let a = self.read_expanded_general_text()?;
        let b = self.read_expanded_general_text()?;
        let (a, b) = (tokens_to_string(&a), tokens_to_string(&b));
        let r = match a.cmp(&b) {
            std::cmp::Ordering::Less => "-1",
            std::cmp::Ordering::Equal => "0",
            std::cmp::Ordering::Greater => "1",
        };
        self.push_tokens(string_tokens(r));
        Ok(())
    }

    fn expand_unless(&mut self) -> Result<(), Error> {
        use Prim::*;
        let n = self.get_token().ok_or_else(|| Error::Figure("\\unless at the end".into()))?;
        let Meaning::Prim(q) = self.meaning(&n) else { return Err(Error::Figure("\\unless must be followed by a conditional".into())) };
        if !q.is_conditional() || q == Ifcase {
            return Err(Error::Figure("\\unless must be followed by a conditional".into()));
        }
        let b = self.evaluate_condition(q)?;
        self.conditional(!b)
    }

    fn expand_ifcase(&mut self) -> Result<(), Error> {
        let n = self.scan_int()?;
        self.conds += 1;
        // skip to the n-th \or (or \else)
        let mut k = 0;
        while k < n {
            match self.skip_branch()? {
                BranchEnd::Or => k += 1,
                BranchEnd::Else => return Ok(()),
                BranchEnd::Fi => {
                    self.close_cond();
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn expand_ifnextchar(&mut self) -> Result<(), Error> {
        // \@ifnextchar c{yes}{no}: looks past spaces
        let c = self.get_token().ok_or_else(|| Error::Figure("\\@ifnextchar without a character".into()))?;
        let yes = self.read_undelimited()?;
        let no = self.read_undelimited()?;
        let next = self.next_nonspace();
        let take = next.as_ref() == Some(&c) || matches!((&next, &c), (Some(Token::Char(a, _)), Token::Char(b, _)) if a == b);
        if let Some(n) = next {
            self.back(n);
        }
        self.push_tokens(if take { yes } else { no });
        Ok(())
    }

    fn expand_whiledo(&mut self) -> Result<(), Error> {
        let test = self.read_undelimited()?;
        let body = self.read_undelimited()?;
        if self.ifthen_test(test.clone())? {
            // the body, then the loop again
            let mut again = body.clone();
            again.push(Token::cs("whiledo"));
            again.push(Token::Char('{', Cat::Begin));
            again.extend(test);
            again.push(Token::Char('}', Cat::End));
            again.push(Token::Char('{', Cat::Begin));
            again.extend(body);
            again.push(Token::Char('}', Cat::End));
            self.push_tokens(again);
        }
        Ok(())
    }

    fn expand_ifstrempty(&mut self, t: &Token) -> Result<(), Error> {
        // etoolbox, nothing expanded: \ifstrempty{text} (no tokens at all), \ifblank{text}
        // (spaces only), \ifdefempty{\m} (a macro without parameters and with an empty body)
        let name = show_token(t);
        let name = name.trim_start_matches('\\');
        let arg = self.read_undelimited()?;
        let test = match name {
            "ifstrempty" => arg.is_empty(),
            "ifblank" => arg.iter().all(Token::is_space),
            _ => arg.first().is_some_and(|m| matches!(self.meaning(m), Meaning::Macro(m) if m.params.is_empty() && m.body.is_empty())),
        };
        let yes = self.read_undelimited()?;
        let no = self.read_undelimited()?;
        self.push_tokens(if test { yes } else { no });
        Ok(())
    }

    fn expand_documentif(&mut self, t: &Token) -> Result<(), Error> {
        let name = match t {
            Token::Cs(n) => n.to_string(),
            _ => String::new(),
        };
        let arg: Vec<Token> = self.read_undelimited()?.into_iter().filter(|x| !x.is_space()).collect();
        let test = if name.starts_with("IfBoolean") {
            arg.len() == 1 && arg[0].is_cs("BooleanTrue")
        } else {
            let missing = arg.len() == 1 && arg[0].is_cs("tikra@novalue");
            missing == name.starts_with("IfNoValue")
        };
        let (yes, no) = if name.ends_with("TF") {
            (self.read_undelimited()?, self.read_undelimited()?)
        } else if name.ends_with('T') {
            (self.read_undelimited()?, Vec::new())
        } else {
            (Vec::new(), self.read_undelimited()?)
        };
        self.push_tokens(if test { yes } else { no });
        Ok(())
    }

    fn expand_prim(&mut self, p: Prim, t: &Token) -> Result<(), Error> {
        use Prim::*;
        match p {
            Expandafter => self.expand_expandafter()?,
            Noexpand => self.expand_noexpand()?,
            Csname => self.expand_csname()?,
            Stringify => {
                let x = self.get_token().unwrap_or(Token::other(' '));
                self.push_tokens(string_tokens(&show_token(&x)));
            }
            Number => {
                let n = self.scan_int()?;
                self.push_tokens(string_tokens(&n.to_string()));
            }
            Romannumeral => {
                let n = self.scan_int()?;
                self.push_tokens(string_tokens(&roman(n)));
            }
            The => {
                let v = self.the()?;
                self.push_tokens(v);
            }
            ShowMeaning => {
                let x = self.get_token().unwrap_or(Token::other(' '));
                let m = self.meaning(&x);
                self.push_tokens(string_tokens(&describe(&m)));
            }
            Unexpanded => self.expand_unexpanded()?,
            Detokenize => {
                let body = self.read_general_text()?;
                self.push_tokens(string_tokens(&tokens_to_string(&body)));
            }
            Jobname => self.push_tokens(string_tokens("figure")),
            Pdfstrcmp => self.expand_pdfstrcmp()?,
            Else | Or => {
                // reached the end of a taken branch: skip to the matching \fi
                if !self.close_cond() {
                    self.warn(format!("extra {}", show_token(t)));
                    return Ok(());
                }
                self.skip_to_fi()?;
            }
            Fi => {
                if !self.close_cond() {
                    self.warn("extra \\fi");
                }
            }
            Unless => self.expand_unless()?,
            Ifcase => self.expand_ifcase()?,
            q if q.is_conditional() => {
                let b = self.evaluate_condition(q)?;
                self.conditional(b)?;
            }
            Ifnextchar => self.expand_ifnextchar()?,
            Firstoftwo => {
                let a = self.read_undelimited()?;
                let _ = self.read_undelimited()?;
                self.push_tokens(a);
            }
            Secondoftwo => {
                let _ = self.read_undelimited()?;
                let b = self.read_undelimited()?;
                self.push_tokens(b);
            }
            Gobble => {
                let _ = self.read_undelimited()?;
            }
            Gobbletwo => {
                let _ = self.read_undelimited()?;
                let _ = self.read_undelimited()?;
            }
            Firstofone => {
                let a = self.read_undelimited()?;
                self.push_tokens(a);
            }
            Value => {
                let name = tokens_to_string(&self.read_undelimited()?);
                let v = self.counter(&name);
                self.push_tokens(string_tokens(&v.to_string()));
            }
            Arabic => {
                let name = tokens_to_string(&self.read_undelimited()?);
                let v = self.counter(&name);
                self.push_tokens(string_tokens(&v.to_string()));
            }
            Ifthenelse => {
                let test = self.read_undelimited()?;
                let yes = self.read_undelimited()?;
                let no = self.read_undelimited()?;
                let b = self.ifthen_test(test)?;
                self.push_tokens(if b { yes } else { no });
            }
            Whiledo => self.expand_whiledo()?,
            Ifstrequal => {
                let a = tokens_to_string(&self.read_expanded_general_undelimited()?);
                let b = tokens_to_string(&self.read_expanded_general_undelimited()?);
                let yes = self.read_undelimited()?;
                let no = self.read_undelimited()?;
                self.push_tokens(if a == b { yes } else { no });
            }
            Ifstrempty => self.expand_ifstrempty(t)?,
            Ifboolean => {
                let name = tokens_to_string(&self.read_undelimited()?);
                self.push_tokens([Token::Cs(Rc::from(format!("if{}", name.trim()).as_str()))]);
            }
            Documentarguments => self.document_arguments()?,
            Documentif => self.expand_documentif(t)?,
            _ => {}
        }
        Ok(())
    }

    fn close_cond(&mut self) -> bool {
        if self.conds == 0 {
            return false;
        }
        self.conds -= 1;
        true
    }

    fn counter(&self, name: &str) -> i64 {
        match self.meaning_of(&format!("c@{}", name.trim())) {
            Meaning::Count(r) => self.counts.get(&r).copied().unwrap_or(0),
            _ => 0,
        }
    }

    /// `ifthen` tests: `\equal{a}{b}`, `\isodd{n}`, `a < b`, `\boolean{x}`, `\not`, `\and`, `\or`.
    fn ifthen_test(&mut self, test: Vec<Token>) -> Result<bool, Error> {
        let mut t: Vec<Token> = Vec::new();
        // expand fully, keeping \equal & co. as markers
        let depth = self.input.len();
        self.push_frame(test);
        while self.input.len() > depth {
            let Some(x) = self.get_token_until(depth) else { break };
            if x.is_cs("equal")
                || x.is_cs("isodd")
                || x.is_cs("not")
                || x.is_cs("NOT")
                || x.is_cs("and")
                || x.is_cs("AND")
                || x.is_cs("or")
                || x.is_cs("OR")
                || x.is_cs("lengthtest")
                || x.is_cs("boolean")
            {
                t.push(x);
                continue;
            }
            let m = self.meaning(&x);
            if m.is_expandable() {
                self.expand(&x, &m)?;
            } else {
                t.push(x);
            }
        }
        ifthen_eval(self, &t)
    }

    fn skip_to_fi(&mut self) -> Result<(), Error> {
        let mut depth = 0usize;
        loop {
            let t = self.get_token().ok_or_else(|| Error::Figure("conditional without \\fi".into()))?;
            match self.meaning(&t) {
                Meaning::Prim(p) if p.is_conditional() => depth += 1,
                Meaning::Prim(Prim::Fi) => {
                    if depth == 0 {
                        return Ok(());
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }
    }

    /// Skips a false branch: up to the matching `\else`, `\or` or `\fi`.
    fn skip_branch(&mut self) -> Result<BranchEnd, Error> {
        let mut depth = 0usize;
        loop {
            let t = self.get_token().ok_or_else(|| Error::Figure("conditional without \\fi".into()))?;
            match self.meaning(&t) {
                Meaning::Prim(p) if p.is_conditional() => depth += 1,
                Meaning::Prim(Prim::Fi) => {
                    if depth == 0 {
                        return Ok(BranchEnd::Fi);
                    }
                    depth -= 1;
                }
                Meaning::Prim(Prim::Else) if depth == 0 => return Ok(BranchEnd::Else),
                Meaning::Prim(Prim::Or) if depth == 0 => return Ok(BranchEnd::Or),
                _ => {}
            }
        }
    }

    /// Takes the true branch (reading on), or skips to `\else`/`\fi`.
    fn conditional(&mut self, b: bool) -> Result<(), Error> {
        self.conds += 1;
        if b {
            return Ok(());
        }
        match self.skip_branch()? {
            BranchEnd::Else => Ok(()),
            BranchEnd::Fi => {
                self.close_cond();
                Ok(())
            }
            BranchEnd::Or => {
                self.close_cond();
                Err(Error::Figure("\\or outside \\ifcase".into()))
            }
        }
    }

    fn evaluate_condition(&mut self, p: Prim) -> Result<bool, Error> {
        use Prim::*;
        Ok(match p {
            Iftrue => true,
            Iffalse => false,
            Ifvmode | Ifinner => false,
            Ifhmode => true,
            Ifmmode => false,
            Ifnum => {
                let a = self.scan_int()?;
                let rel = self.scan_relation()?;
                let b = self.scan_int()?;
                compare(a, rel, b)
            }
            Ifdim => {
                let a = self.scan_dimen()?;
                let rel = self.scan_relation()?;
                let b = self.scan_dimen()?;
                compare(a, rel, b)
            }
            Ifodd => self.scan_int()? % 2 != 0,
            Ifx => {
                let a = self.get_token().ok_or_else(|| Error::Figure("\\ifx at the end".into()))?;
                let b = self.get_token().ok_or_else(|| Error::Figure("\\ifx at the end".into()))?;
                let (ma, mb) = (self.meaning(&a), self.meaning(&b));
                match (&ma, &mb) {
                    (Meaning::Macro(x), Meaning::Macro(y)) => x.params == y.params && x.body == y.body && x.optional == y.optional,
                    _ => ma == mb,
                }
            }
            If | Ifcat => {
                let a = self.x_token_for_if()?;
                let b = self.x_token_for_if()?;
                let code = |t: &Token| -> (Option<char>, Option<Cat>) {
                    match t {
                        Token::Char(c, cat) => (Some(*c), Some(*cat)),
                        _ => (None, None),
                    }
                };
                let (ca, ka) = code(&a);
                let (cb, kb) = code(&b);
                if p == If {
                    ca == cb
                } else {
                    ka == kb
                }
            }
            Ifdefined => {
                let a = self.get_token().ok_or_else(|| Error::Figure("\\ifdefined at the end".into()))?;
                !matches!(self.meaning(&a), Meaning::Undefined)
            }
            Ifcsname => {
                let mut name = String::new();
                loop {
                    let x = self.get_x_token()?.ok_or_else(|| Error::Figure("\\ifcsname without \\endcsname".into()))?;
                    match x {
                        Token::Cs(ref n) if &**n == "endcsname" => break,
                        Token::Cs(ref n) => name.push_str(&format!("\\{n}")),
                        Token::Char(c, _) => name.push(c),
                    }
                }
                self.is_defined(&name)
            }
            _ => false,
        })
    }

    /// For `\if`: an expanded token; a control sequence that is not a character counts as character 256.
    fn x_token_for_if(&mut self) -> Result<Token, Error> {
        let t = self.get_x_token()?.ok_or_else(|| Error::Figure("\\if at the end".into()))?;
        if t.is_cs("tikra@noexpand") {
            return Ok(self.get_token().unwrap_or(t));
        }
        Ok(match self.meaning(&t) {
            Meaning::Char(c, cat) => Token::Char(c, cat),
            _ => t,
        })
    }

    fn scan_relation(&mut self) -> Result<char, Error> {
        let t = self.next_x_nonspace()?.ok_or_else(|| Error::Figure("missing = < or > in a comparison".into()))?;
        match t {
            Token::Char(c @ ('<' | '=' | '>'), _) => Ok(c),
            _ => Err(Error::Figure("missing = < or > in a comparison".into())),
        }
    }

    pub fn next_x_nonspace(&mut self) -> Result<Option<Token>, Error> {
        loop {
            let t = self.get_x_token()?;
            match t {
                Some(t) if t.is_space() => continue,
                other => return Ok(other),
            }
        }
    }

    /// `\the`: the value of a register or internal quantity, as tokens.
    fn the(&mut self) -> Result<Vec<Token>, Error> {
        let t = self.get_x_token()?.ok_or_else(|| Error::Figure("\\the at the end".into()))?;
        let m = self.meaning(&t);
        Ok(match m {
            Meaning::Count(r) => string_tokens(&self.counts.get(&r).copied().unwrap_or(0).to_string()),
            Meaning::Dimen(r) | Meaning::Skip(r) => string_tokens(&dimen::show(self.dimens.get(&r).copied().unwrap_or(0))),
            Meaning::Toks(r) => self.toks.get(&r).map(|v| (**v).clone()).unwrap_or_default(),
            Meaning::CharDef(v) => string_tokens(&v.to_string()),
            Meaning::Prim(Prim::Count) => {
                let r = self.scan_register_number()?;
                string_tokens(&self.counts.get(&r).copied().unwrap_or(0).to_string())
            }
            Meaning::Prim(Prim::Dimen | Prim::Skip) => {
                let r = self.scan_register_number()?;
                string_tokens(&dimen::show(self.dimens.get(&r).copied().unwrap_or(0)))
            }
            Meaning::Prim(Prim::Toks) => {
                let r = self.scan_register_number()?;
                self.toks.get(&r).map(|v| (**v).clone()).unwrap_or_default()
            }
            Meaning::Prim(Prim::Numexpr) => string_tokens(&self.scan_numexpr()?.to_string()),
            Meaning::Prim(Prim::Dimexpr | Prim::Glueexpr) => string_tokens(&dimen::show(self.scan_dimexpr()?)),
            Meaning::Prim(Prim::Catcode) => {
                let c = self.scan_int()?;
                let ch = char::from_u32(c.max(0) as u32).unwrap_or(' ');
                string_tokens(&self.catcode(ch).code().to_string())
            }
            Meaning::Prim(p @ (Prim::Wd | Prim::Ht | Prim::Dp)) => {
                let n = self.scan_int()?;
                string_tokens(&dimen::show(self.box_dimen(p, n)))
            }
            _ => {
                self.warn(format!("\\the{} is not supported", show_token(&t)));
                Vec::new()
            }
        })
    }

    /// `{…}` after optional spaces and `\relax` (for `\unexpanded`, `\detokenize`), unexpanded.
    pub fn read_general_text(&mut self) -> Result<Vec<Token>, Error> {
        loop {
            let t = self.get_x_token()?.ok_or_else(|| Error::Figure("missing '{'".into()))?;
            if t.is_space() || t.is_cs("relax") {
                continue;
            }
            if matches!(t, Token::Char(_, Cat::Begin)) {
                return self.read_group_body();
            }
            self.back(t);
            return Err(Error::Figure("missing '{'".into()));
        }
    }

    fn read_expanded_general_text(&mut self) -> Result<Vec<Token>, Error> {
        let body = self.read_general_text()?;
        self.expand_fully(body)
    }

    fn read_expanded_general_undelimited(&mut self) -> Result<Vec<Token>, Error> {
        let body = self.read_undelimited()?;
        self.expand_fully(body)
    }

    /// Expands tokens as `\edef` does: everything expandable, except what `\noexpand`/`\unexpanded` protect,
    /// protected macros, and `\the` of a token register (inserted as is).
    pub fn expand_fully(&mut self, tokens: Vec<Token>) -> Result<Vec<Token>, Error> {
        let depth = self.input.len();
        // a frame of its own, so we never read past our tokens
        self.input.push(Frame::Tokens(tokens.into_iter().collect()));
        let r = self.expand_frame(depth);
        if r.is_err() {
            // what was left of the expansion goes, so the error does not come back
            self.drain_to(depth);
        }
        r
    }

    fn expand_frame(&mut self, depth: usize) -> Result<Vec<Token>, Error> {
        self.expand_frame_until(depth, &[]).map(|(out, _)| out)
    }

    /// Starts expanding `tokens` piece by piece, with `expand_more`: the depth of the input that
    /// they end at.
    pub fn expand_start(&mut self, tokens: Vec<Token>) -> usize {
        let depth = self.input.len();
        self.input.push(Frame::Tokens(tokens.into_iter().collect()));
        depth
    }

    /// Expands what `expand_start` began, to its end or to the first unexpandable control sequence
    /// named in `stops` or register (whose assignment it begins): what it expanded to, and that
    /// control sequence, after which the input is what follows it (for the caller to read its
    /// arguments and run it, as TeX would while typesetting, and then to call again).
    pub fn expand_more(&mut self, depth: usize, stops: &[&str]) -> Result<(Vec<Token>, Option<Token>), Error> {
        let r = self.expand_frame_until(depth, stops);
        if r.is_err() {
            self.drain_to(depth);
        }
        r
    }

    fn expand_frame_until(&mut self, depth: usize, stops: &[&str]) -> Result<(Vec<Token>, Option<Token>), Error> {
        let mut out = Vec::new();
        while self.input.len() > depth {
            let Some(t) = self.get_token_until(depth) else { break };
            if let Token::Cs(name) = &t {
                if stops.contains(&name.as_ref()) && !self.meaning(&t).is_expandable() {
                    return Ok((out, Some(t)));
                }
                if !stops.is_empty() && matches!(self.meaning(&t), Meaning::Count(_) | Meaning::Dimen(_) | Meaning::Skip(_) | Meaning::Toks(_)) {
                    return Ok((out, Some(t)));
                }
            }
            if t.is_cs("tikra@noexpand") {
                if let Some(n) = self.get_token_until(depth) {
                    out.push(n);
                }
                continue;
            }
            let m = self.meaning(&t);
            match &m {
                Meaning::Macro(mac) if mac.protected => out.push(t),
                Meaning::Prim(Prim::The) => {
                    self.count_expansion()?;
                    let v = self.the()?;
                    out.extend(v);
                }
                m if m.is_expandable() => self.expand(&t, m)?,
                _ => out.push(t),
            }
            if out.len() > MAX_TOKENS {
                return Err(Error::Figure("expansion too long".into()));
            }
        }
        Ok((out, None))
    }

    /// Expands tokens fully and returns them as text (control sequences written `\name `).
    pub fn expand_to_string(&mut self, tokens: Vec<Token>) -> Result<String, Error> {
        let t = self.expand_fully(tokens)?;
        Ok(tokens_to_string(&t))
    }
}

enum BranchEnd {
    Else,
    Or,
    Fi,
}

fn compare(a: i64, rel: char, b: i64) -> bool {
    match rel {
        '<' => a < b,
        '>' => a > b,
        _ => a == b,
    }
}

fn describe(m: &Meaning) -> String {
    match m {
        Meaning::Macro(mac) => {
            let params: String = mac
                .params
                .iter()
                .map(|p| match p {
                    Param::Tok(t) => show_token(t),
                    Param::Arg(n) => format!("#{n}"),
                })
                .collect();
            let body: String = mac
                .body
                .iter()
                .map(|b| match b {
                    Body::Tok(t) => show_token(t),
                    Body::Arg(n) => format!("#{n}"),
                })
                .collect();
            format!("macro:{params}->{body}")
        }
        Meaning::Char(c, _) => format!("the character {c}"),
        Meaning::Undefined => "undefined".into(),
        Meaning::Prim(p) => format!("\\{}", format!("{p:?}").to_lowercase()),
        Meaning::Count(r) => format!("\\count{r}"),
        Meaning::Dimen(r) => format!("\\dimen{r}"),
        Meaning::Skip(r) => format!("\\skip{r}"),
        Meaning::Toks(r) => format!("\\toks{r}"),
        Meaning::CharDef(v) => format!("\\char\"{v:X}"),
        Meaning::Native(_) => "\\relax".into(),
    }
}

fn roman(n: i64) -> String {
    if n <= 0 {
        return String::new();
    }
    let mut n = n.min(100_000);
    let mut s = String::new();
    for (v, r) in [(1000, "m"), (900, "cm"), (500, "d"), (400, "cd"), (100, "c"), (90, "xc"), (50, "l"), (40, "xl"), (10, "x"), (9, "ix"), (5, "v"), (4, "iv"), (1, "i")] {
        while n >= v {
            s.push_str(r);
            n -= v;
        }
    }
    s
}

fn ifthen_eval(tex: &mut Tex, t: &[Token]) -> Result<bool, Error> {
    // split by \or (lowest), then \and, with \not and parentheses \( \)
    fn split<'a>(t: &'a [Token], names: &[&str]) -> Vec<&'a [Token]> {
        let mut parts = Vec::new();
        let mut start = 0;
        for (i, x) in t.iter().enumerate() {
            if names.iter().any(|n| x.is_cs(n)) {
                parts.push(&t[start..i]);
                start = i + 1;
            }
        }
        parts.push(&t[start..]);
        parts
    }
    let ors = split(t, &["or", "OR"]);
    if ors.len() > 1 {
        for p in ors {
            if ifthen_eval(tex, p)? {
                return Ok(true);
            }
        }
        return Ok(false);
    }
    let ands = split(t, &["and", "AND"]);
    if ands.len() > 1 {
        for p in ands {
            if !ifthen_eval(tex, p)? {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    let t: Vec<Token> = t.iter().filter(|x| !x.is_space()).cloned().collect();
    if let Some(first) = t.first() {
        if first.is_cs("not") || first.is_cs("NOT") {
            return Ok(!ifthen_eval(tex, &t[1..])?);
        }
        if first.is_cs("equal") {
            // \equal{a}{b}: the two groups that follow
            let rest = &t[1..];
            let (a, after) = take_group(rest);
            let (b, _) = take_group(after);
            return Ok(tokens_to_string(&a).trim() == tokens_to_string(&b).trim());
        }
        if first.is_cs("isodd") {
            let (a, _) = take_group(&t[1..]);
            let v: i64 = tokens_to_string(&a).trim().parse().unwrap_or(0);
            return Ok(v % 2 != 0);
        }
        if first.is_cs("boolean") {
            let (a, _) = take_group(&t[1..]);
            let name = format!("if{}", tokens_to_string(&a).trim());
            return Ok(tex.meaning_of(&name) == Meaning::Prim(Prim::Iftrue));
        }
        if first.is_cs("lengthtest") {
            let (a, _) = take_group(&t[1..]);
            let s = tokens_to_string(&a);
            let Some(i) = s.find(['<', '=', '>']) else { return Ok(false) };
            let rel = s[i..].chars().next().unwrap_or('=');
            let x = crate::expr::eval_length(s[..i].trim(), &crate::expr::Variables::new()).map(|v| v.0).unwrap_or(0.0);
            let y = crate::expr::eval_length(s[i + 1..].trim(), &crate::expr::Variables::new()).map(|v| v.0).unwrap_or(0.0);
            return Ok(match rel {
                '<' => x < y,
                '>' => x > y,
                _ => (x - y).abs() < 1e-9,
            });
        }
    }
    // a numeric comparison: a < b, a = b, a > b
    let s = tokens_to_string(&t);
    let Some(i) = s.find(['<', '=', '>']) else { return Ok(false) };
    let rel = s[i..].chars().next().unwrap_or('=');
    let a: f64 = crate::expr::eval(s[..i].trim(), &crate::expr::Variables::new()).unwrap_or(0.0);
    let b: f64 = crate::expr::eval(s[i + 1..].trim(), &crate::expr::Variables::new()).unwrap_or(0.0);
    Ok(match rel {
        '<' => a < b,
        '>' => a > b,
        _ => a == b,
    })
}

/// The tokens of a `{…}` group at the start of `t` (or the first token), and what follows.
fn take_group(t: &[Token]) -> (Vec<Token>, &[Token]) {
    let mut i = 0;
    while i < t.len() && t[i].is_space() {
        i += 1;
    }
    if i >= t.len() {
        return (Vec::new(), &t[i..]);
    }
    if !matches!(t[i], Token::Char(_, Cat::Begin)) {
        return (vec![t[i].clone()], &t[i + 1..]);
    }
    let mut depth = 0;
    for k in i..t.len() {
        match t[k] {
            Token::Char(_, Cat::Begin) => depth += 1,
            Token::Char(_, Cat::End) => {
                depth -= 1;
                if depth == 0 {
                    return (t[i + 1..k].to_vec(), &t[k + 1..]);
                }
            }
            _ => {}
        }
    }
    (t[i + 1..].to_vec(), &[])
}
