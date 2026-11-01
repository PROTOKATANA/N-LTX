//! Running pgfkeys: key lists, handlers (`.style`, `.code`, `.default`, `.initial`, `.store in`,
//! `.is choice`…), TikZ's `.unknown` (a color, an arrow specification, a shape name), and the keys
//! that TikZ defines with code, done natively. Keys that TikZ defines as styles are defined here the
//! same way, from TikZ's own definitions.

use std::rc::Rc;

use crate::keys::{self, macro_name};
use crate::tex::{self, Body, Cat, Macro, Meaning, Param, Token};
use crate::Error;

use super::{macro_from, macro_tokens, tikra_tokens, Engine, EXTENSION_KEY_BASE, NATIVE_KEY_BASE};

mod native;
pub(super) use native::node_distance;
pub use native::{deactivate, define_keys};

pub type KeyFn = fn(&mut Engine, Option<Vec<Token>>) -> Result<(), Error>;

fn group(v: Vec<Token>) -> Vec<Token> {
    let mut out = Vec::with_capacity(v.len() + 2);
    out.push(Token::Char('{', Cat::Begin));
    out.extend(v);
    out.push(Token::Char('}', Cat::End));
    out
}

/// Where a key list runs: its default path, and whether that path ends with the list.
#[derive(Clone, Copy)]
struct ListPath<'a> {
    path: Option<&'a str>,
    restore: bool,
}

/// What a key may have to be text alone: nothing that TeX reads as anything else.
fn is_plain(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b" /.-!:<>|_+*'\"".contains(&b)
}

fn is_novalue(v: &[Token]) -> bool {
    let v: Vec<&Token> = v.iter().filter(|t| !t.is_space()).collect();
    v.len() == 1 && v[0].is_cs("pgfkeysnovalue")
}

impl Engine {
    /// Runs a key list now, with `path` as the default path (or the current one).
    pub fn process_keys(&mut self, list: Vec<Token>, path: Option<&str>) -> Result<(), Error> {
        self.keys_command(list, path, true)
    }

    /// `\tikzset`, `\pgfkeys`, `\pgfkeysalso`: the list's keys, one after the other. They run
    /// directly and not as tokens through the interpreter: a key costs one lookup instead of being
    /// lexed, expanded and read back, 22 % less per element (`banco_animacion`, 5,000 elements:
    /// 68.1 to 53.0 ms per frame).
    pub fn keys_command(&mut self, list: Vec<Token>, path: Option<&str>, restore: bool) -> Result<(), Error> {
        self.run_items(keys::split_list(&list), ListPath { path, restore })
    }

    /// [`process_keys`] under `/tikz`, for what every path runs (`every path/.try`, `fill,draw`):
    /// a list of plain keys has nothing to read as TeX, which is 5 % of a `\fill` in a `\foreach`
    /// (`banco_animacion`, 5,000 elements: 40.8 to 38.8 ms per frame).
    ///
    /// [`process_keys`]: Engine::process_keys
    pub fn tikz_keys(&mut self, list: &str) -> Result<(), Error> {
        if !list.bytes().all(|b| is_plain(b) || b == b',') {
            return self.process_keys(tikra_tokens(list), Some("/tikz"));
        }
        let items = list.split(',').map(|k| keys::Item { key: k.to_string(), value: None });
        self.run_items(items.collect(), ListPath { path: Some("/tikz"), restore: true })
    }

    fn run_items(&mut self, items: Vec<keys::Item>, at: ListPath<'_>) -> Result<(), Error> {
        if self.keyrun.key_lists == 0 {
            self.keyrun.keys_allowed += super::KEYS_PER_LIST;
        }
        self.keyrun.key_lists += 1;
        let r = self.nested(|e, depth| e.list_of(items, at, depth));
        self.keyrun.key_lists -= 1;
        r
    }

    fn list_of(&mut self, items: Vec<keys::Item>, at: ListPath<'_>, depth: usize) -> Result<(), Error> {
        let old = self.st.key_path.clone();
        if let Some(p) = at.path {
            self.st.key_path = p.trim().to_string();
        }
        for item in items {
            // a style called without a value passes `\pgfkeysnovalue` on: pgfkeys skips it
            if item.key.trim() == "\\pgfkeysnovalue" && item.value.is_none() {
                continue;
            }
            let key = self.key_name(&item.key)?;
            self.run_item(&key, item.value)?;
            // what a key leaves in the input (a stored value is "typeset") goes before the next key
            self.run_until(depth)?;
        }
        if at.restore && at.path.is_some() {
            self.st.key_path = old;
        }
        Ok(())
    }

    /// A key as written, which a macro in it may complete. Text alone has nothing to expand.
    fn key_name(&mut self, text: &str) -> Result<String, Error> {
        if text.bytes().all(is_plain) && !text.contains("  ") {
            return Ok(text.trim().to_string());
        }
        Ok(self.tex.expand_to_string(tikra_tokens(text))?.trim().to_string())
    }

    fn run_item(&mut self, key: &str, value: Option<Vec<Token>>) -> Result<(), Error> {
        let value = value.filter(|v| !is_novalue(v));
        // a label's text is a node's: it keeps them with `handle active characters in nodes`
        let text = matches!(key, "label" | "pin" | "node contents");
        let value = match value {
            Some(v) if self.st.active_in_code && !(text && self.st.active_in_nodes) => Some(deactivate(v)),
            v => v,
        };
        self.invoke(key, value)
    }

    pub fn invoke(&mut self, key: &str, value: Option<Vec<Token>>) -> Result<(), Error> {
        self.keyrun.keys_run += 1;
        if self.keyrun.keys_run > self.keyrun.keys_allowed {
            if !std::mem::replace(&mut self.keyrun.keys_capped, true) {
                self.warn(format!("more than {} options (styles that multiply?): the rest ignored", self.keyrun.keys_allowed));
            }
            return Ok(());
        }
        // `/handlers/first char syntax`: a key that starts with a character that has a macro there
        // is that macro, with the whole key (and its value) as argument
        // (looked up only once it is on: every key passes here)
        if let Some(c) = key.trim_start().chars().next().filter(|_| self.keyrun.first_char_syntax) {
            let handler = format!("/handlers/first char syntax/the character {c}");
            if let Some(code) = keys::value(&self.tex, &handler).filter(|_| self.key_text("/handlers/first char syntax").trim() == "true") {
                let mut whole = tikra_tokens(key.trim());
                if let Some(v) = value {
                    whole.push(Token::Char('=', tex::Cat::Other));
                    whole.extend(v);
                }
                let mut run = code;
                run.extend(group(whole));
                return self.run_tokens(run);
            }
        }
        let path = keys::full_path(&self.st.key_path, key);
        if path == "/handlers/first char syntax" {
            self.keyrun.first_char_syntax = true;
        }
        // a key named like a handler (`characters/.append`) is a key: pgfkeys looks for keys first
        if let Some((base, h)) = keys::handler(&path).filter(|_| !self.tex.is_defined(&macro_name(&format!("{path}/.@cmd")))) {
            let (mut base, h) = (base.to_string(), h.to_string());
            // `/handler config=full or existing`: a handler that changes a key that does not exist
            // goes to the key of the same name on the search path (pgfplots' `every mark/.append style`)
            if matches!(h.as_str(), ".append style" | ".prefix style" | ".append code" | ".prefix code" | ".append" | ".prefix" | ".add") && !self.key_exists(&base) {
                if let Some(list) = keys::value(&self.tex, &format!("{}/.@searchalso", keys::parent(&base))) {
                    let name = keys::name(&base).to_string();
                    let found = tex::tokens_to_string(&list).split(',').map(|p| format!("{}/{name}", p.trim().trim_end_matches('/'))).find(|other| self.key_exists(other));
                    if let Some(other) = found {
                        base = other;
                    }
                }
            }
            return self.handler(&base, &h, value);
        }
        if self.run_key(&path, value.clone())? {
            return Ok(());
        }
        self.unknown(&path, value)
    }

    /// Whether a key has code or a value.
    pub fn key_exists(&self, path: &str) -> bool {
        self.tex.is_defined(&macro_name(path)) || self.tex.is_defined(&macro_name(&format!("{path}/.@cmd")))
    }

    /// Runs the key at `path` if it is defined; false if it is not.
    pub fn run_key(&mut self, path: &str, value: Option<Vec<Token>>) -> Result<bool, Error> {
        // the two names differ in their end: one text for both, as this runs for every key
        let mut name = macro_name(path);
        let length = name.len();
        name.push_str("/.@cmd");
        let cmd = self.tex.meaning_of(&name);
        name.truncate(length);
        name.push_str("/.@def");
        let value = value.or_else(|| keys::stored(&self.tex, &name));
        match cmd {
            Meaning::Native(id) if id >= EXTENSION_KEY_BASE => {
                let f = self.plug.extension_fns[(id - EXTENSION_KEY_BASE) as usize];
                self.keyrun.current_key.clear();
                self.keyrun.current_key.push_str(path);
                let v = value.filter(|v| !is_novalue(v)).map(crate::extension::Tokens);
                f(&mut crate::extension::Context { e: self }, v)?;
                Ok(true)
            }
            Meaning::Native(id) if id >= NATIVE_KEY_BASE => {
                let f = self.plug.key_fns[(id - NATIVE_KEY_BASE) as usize];
                self.keyrun.current_key.clear();
                self.keyrun.current_key.push_str(path);
                let v = value.filter(|v| !is_novalue(v));
                f(self, v)?;
                Ok(true)
            }
            Meaning::Macro(m) => {
                let n = m.params.iter().filter(|p| matches!(p, Param::Arg(_))).count();
                let v = value.unwrap_or_else(|| vec![Token::cs("pgfkeysnovalue")]);
                let args = if n <= 1 { vec![v] } else { split_arguments(&v, n) };
                let code = tex::substitute(&m.body, &args);
                self.run_tokens(code)?;
                Ok(true)
            }
            _ => {
                if self.tex.is_defined(&macro_name(path)) {
                    match value {
                        Some(v) => self.tex.set_meaning(&Token::cs(&macro_name(path)), macro_tokens(v), false),
                        None => {
                            // a stored value used as a key: its value is "typeset" (inserted)
                            if let Some(v) = keys::value(&self.tex, path) {
                                self.tex.push_tokens(v);
                            }
                        }
                    }
                    return Ok(true);
                }
                Ok(false)
            }
        }
    }

    fn unknown(&mut self, path: &str, value: Option<Vec<Token>>) -> Result<(), Error> {
        let parent = keys::parent(path).to_string();
        let name = keys::name(path).to_string();
        // the parent's .unknown handler (TikZ's: a color, an arrow specification, a shape)
        let handler = macro_name(&format!("{parent}/.unknown/.@cmd"));
        if self.tex.is_defined(&handler) {
            self.tex.define_string("pgfkeyscurrentname", &name, false);
            self.tex.define_string("pgfkeyscurrentkey", path, false);
            self.tex.set_meaning(&Token::cs("pgfkeyscurrentvalue"), macro_tokens(value.clone().unwrap_or_else(|| vec![Token::cs("pgfkeysnovalue")])), false);
            if self.run_key(&format!("{parent}/.unknown"), value.clone())? {
                return Ok(());
            }
        }
        // .search also
        if let Some(list) = keys::value(&self.tex, &format!("{parent}/.@searchalso")) {
            let text = tex::tokens_to_string(&list);
            for p in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                let other = format!("{}/{name}", p.trim_end_matches('/'));
                if self.run_key(&other, value.clone())? {
                    return Ok(());
                }
            }
        }
        self.unknown_warning(&name, path);
        Ok(())
    }

    fn unknown_warning(&mut self, name: &str, path: &str) {
        let _ = path;
        self.warn(format!("option '{name}' not supported: ignored"));
    }

    /// `/tikz/.unknown`: a color, an arrow specification or a shape name; otherwise a `/pgf/` key.
    fn tikz_unknown(&mut self, value: Option<Vec<Token>>) -> Result<(), Error> {
        let name = keys::value(&self.tex, "pgfkeyscurrentname").or_else(|| match self.tex.meaning_of("pgfkeyscurrentname") {
            Meaning::Macro(m) => Some(tex::substitute(&m.body, &[])),
            _ => None,
        });
        let name = name.map(|v| tex::tokens_to_string(&v)).unwrap_or_default();
        if name.starts_with('"') {
            return self.quotes(&name, value);
        }
        // `myself:rotate={…}`, `:fill={…}`: an animated attribute
        if let (Some(v), true) = (&value, name.contains(':')) {
            if let Some(o) = self.animation_option(&name, v.clone())? {
                self.process_keys(o, Some("/tikz"))?;
            }
            return Ok(());
        }
        if value.is_none() {
            if let Some(c) = self.color(&name) {
                self.st.paint.color = c;
                self.st.paint.draw_color = None;
                self.st.paint.fill_color = None;
                self.st.paint.text_color = None;
                return Ok(());
            }
            if let Some((a, b)) = super::arrows::parse_arrows(&name, &self.st.pen.tips) {
                self.st.pen.start_arrow = a;
                self.st.pen.end_arrow = b;
                return Ok(());
            }
            if super::nodes::is_shape(&name) || self.declared.user_shapes.contains_key(&name) {
                self.tex.define_string(&macro_name("/tikz/shape"), &name, false);
                return Ok(());
            }
        }
        let pgf = format!("/pgf/{name}");
        if self.run_key(&pgf, value)? {
            return Ok(());
        }
        self.unknown_warning(&name, &format!("/tikz/{name}"));
        Ok(())
    }

    /// The quotes library: `"text" options` makes a label in node options, and a node on the path in
    /// `to`/`edge` options; `'` after the quotes means `swap`.
    fn quotes(&mut self, key: &str, value: Option<Vec<Token>>) -> Result<(), Error> {
        // pgfkeys split `"x=1"` at its `=`: put it back together
        let mut whole = key.to_string();
        if let Some(v) = value {
            whole.push('=');
            whole.push_str(&tex::tokens_to_string(&v));
        }
        let Some(end) = whole[1..].find('"').map(|i| i + 1) else {
            self.warn(format!("quotes: closing '\"' missing in {whole}"));
            return Ok(());
        };
        let text = whole[1..end].to_string();
        let mut options = whole[end + 1..].trim().to_string();
        if let Some(rest) = options.strip_prefix('\'') {
            options = format!("swap,{}", rest.trim());
        }
        // `"text" above, red` is one key in a list only up to the comma: the rest are options
        let options = options.split_whitespace().collect::<Vec<_>>().join(" ");
        if self.st.pic.quotes_in_pic {
            self.st.pic.pic_text = Some((tikra_tokens(&text), tikra_tokens(&options)));
        } else if self.st.quotes_on_path {
            let mut o = tikra_tokens("every edge quotes/.try,");
            o.extend(tikra_tokens(&options));
            self.st.quoted.push((tikra_tokens(&text), o));
        } else {
            // `quotes mean pin`: pins instead of labels
            let pin = matches!(self.tex.meaning_of("tikra@quotes@pin"), Meaning::Macro(m) if tex::tokens_to_string(&tex::substitute(&m.body, &[])).trim() == "1");
            let spec = format!("[direction shorthands,every {} quotes/.try,{options}]{text}", if pin { "pin" } else { "label" });
            self.st.node.labels.push((tikra_tokens(&spec), pin));
        }
        Ok(())
    }

    // ---------------------------------------------------------------- handlers

    fn define_code(&mut self, path: &str, n: usize, body: Vec<Token>) {
        let params = (1..=n as u8).map(Param::Arg).collect();
        let m = Macro { params, body: tex::parse_body(body), optional: None, protected: false };
        self.tex.set_meaning(&Token::cs(&macro_name(&format!("{path}/.@cmd"))), Meaning::Macro(Rc::new(m)), false);
    }

    /// The code of a key, as tokens with `#1`… in place, if it is a macro.
    fn code_tokens(&self, path: &str) -> Option<(usize, Vec<Token>)> {
        match self.tex.meaning_of(&macro_name(&format!("{path}/.@cmd"))) {
            Meaning::Macro(m) => {
                let n = m.params.len();
                let mut v = Vec::new();
                for b in &m.body {
                    match b {
                        Body::Tok(t) => {
                            if matches!(t, Token::Char(_, Cat::Param)) {
                                // a ## inside: keep it doubled so the code stays the same once defined again
                                v.push(t.clone());
                            }
                            v.push(t.clone());
                        }
                        Body::Arg(k) => {
                            v.push(Token::Char('#', Cat::Param));
                            v.push(Token::other(char::from(b'0' + k)));
                        }
                    }
                }
                // single # were doubled above; undo for plain tokens
                let mut out = Vec::with_capacity(v.len());
                let mut i = 0;
                while i < v.len() {
                    if matches!(v[i], Token::Char(_, Cat::Param))
                        && matches!(v.get(i + 1), Some(Token::Char(_, Cat::Param)))
                        && !matches!(v.get(i + 2), Some(Token::Char(c, _)) if c.is_ascii_digit())
                    {
                        out.push(v[i].clone());
                        i += 2;
                        continue;
                    }
                    out.push(v[i].clone());
                    i += 1;
                }
                Some((n, out))
            }
            _ => None,
        }
    }

    fn style_body(value: Vec<Token>) -> Vec<Token> {
        let mut b = vec![Token::cs("pgfkeysalso")];
        b.extend(group(value));
        b
    }

    fn handler_style(&mut self, base: &str, h: &str, v: Vec<Token>) -> Result<(), Error> {
        let v = if h == ".estyle" { self.tex.expand_fully(v)? } else { v };
        self.define_code(base, 1, Self::style_body(v));
        Ok(())
    }

    fn handler_restyle(&mut self, base: &str, h: &str, v: Vec<Token>) -> Result<(), Error> {
        let (n, old) = self.code_tokens(base).unwrap_or((1, Vec::new()));
        let new = Self::style_body(v);
        let body = if h == ".append style" { [old, new].concat() } else { [new, old].concat() };
        self.define_code(base, n.max(1), body);
        Ok(())
    }

    fn handler_code(&mut self, base: &str, h: &str, v: Vec<Token>) -> Result<(), Error> {
        let v = if h == ".ecode" { self.tex.expand_fully(v)? } else { v };
        self.define_code(base, 1, v);
        // a pgfplots coordinate filter given as code drops its expressions
        if base.ends_with(" filter") {
            self.tex.set_meaning(&Token::cs(&macro_name(&format!("{base}/@expressions"))), macro_tokens(Vec::new()), false);
        }
        Ok(())
    }

    fn handler_condition(&mut self, base: &str, v: Vec<Token>) -> Result<(), Error> {
        let path = format!("{base}/@conditions");
        let mut list = keys::value(&self.tex, &path).unwrap_or_default();
        list.extend(v);
        self.tex.set_meaning(&Token::cs(&macro_name(&path)), macro_tokens(list), false);
        Ok(())
    }

    fn handler_expression(&mut self, base: &str, h: &str, v: Vec<Token>) -> Result<(), Error> {
        let path = format!("{base}/@expressions");
        let mut list = if h == ".expression" {
            self.define_code(base, 1, Vec::new());
            Vec::new()
        } else {
            keys::value(&self.tex, &path).unwrap_or_default()
        };
        list.extend(group(v));
        self.tex.set_meaning(&Token::cs(&macro_name(&path)), macro_tokens(list), false);
        Ok(())
    }

    fn handler_affix_code(&mut self, base: &str, h: &str, v: Vec<Token>) -> Result<(), Error> {
        let (n, old) = self.code_tokens(base).unwrap_or((1, Vec::new()));
        let body = if h == ".append code" { [old, v].concat() } else { [v, old].concat() };
        self.define_code(base, n.max(1), body);
        Ok(())
    }

    fn handler_n_args(&mut self, base: &str, h: &str, v: Vec<Token>) -> Result<(), Error> {
        let (n, body) = if h.contains("n args") {
            let args = split_arguments(&v, 2);
            let n: usize = tex::tokens_to_string(&args[0]).trim().parse().unwrap_or(1);
            (n, args[1].clone())
        } else {
            (2, v)
        };
        let body = if h.contains("style") { Self::style_body(body) } else { body };
        self.define_code(base, n, body);
        Ok(())
    }

    fn handler_code_args(&mut self, base: &str, h: &str, v: Vec<Token>) -> Result<(), Error> {
        // as pgf: a macro with the parameter text, called with the value and \pgfeov
        let args = split_arguments(&v, 2);
        let body = if h.contains("style") { Self::style_body(args[1].clone()) } else { args[1].clone() };
        let aux = Token::cs(&macro_name(&format!("{base}/.@args")));
        let mut def = vec![Token::cs("def"), aux.clone()];
        def.extend(args[0].clone());
        def.push(Token::cs("pgfeov"));
        def.extend(group(body));
        self.run_tokens(def)?;
        self.define_code(base, 1, vec![aux, Token::Char('#', Cat::Param), Token::other('1'), Token::cs("pgfeov")]);
        Ok(())
    }

    fn handler_tip(&mut self, base: &str, v: Vec<Token>) -> Result<(), Error> {
        // `name/.tip=spec`: \pgfdeclarearrow{name=name, means=spec}, the name being the last part of the path
        let name = base.rsplit('/').next().unwrap_or(base).trim().to_string();
        let means = tex::tokens_to_string(&v);
        std::rc::Rc::make_mut(&mut self.st.pen.tips).declare(&name, &means);
        Ok(())
    }

    fn handler_store(&mut self, base: &str, h: &str, v: Vec<Token>) -> Result<(), Error> {
        let target: Vec<Token> = v.into_iter().filter(|t| !t.is_space()).collect();
        let mut body = vec![Token::cs(if h == ".store in" { "def" } else { "edef" })];
        body.extend(target);
        body.extend(group(vec![Token::Char('#', Cat::Param), Token::other('1')]));
        self.define_code(base, 1, body);
        Ok(())
    }

    fn handler_is_if(&mut self, base: &str, v: Vec<Token>) -> Result<(), Error> {
        let name = tex::tokens_to_string(&v).trim().to_string();
        // key=true runs \nametrue
        let body = vec![Token::cs("csname"), Token::other(' ')].into_iter().take(1).collect::<Vec<_>>();
        let mut body = body;
        body.extend(tex::string_tokens(&name));
        body.push(Token::Char('#', Cat::Param));
        body.push(Token::other('1'));
        body.push(Token::cs("endcsname"));
        self.define_code(base, 1, body);
        self.tex.set_meaning(&Token::cs(&macro_name(&format!("{base}/.@def"))), macro_from("true"), false);
        Ok(())
    }

    fn handler_is_choice(&mut self, base: &str) -> Result<(), Error> {
        // base=choice runs base/choice
        let mut body = vec![Token::cs("pgfkeysalso")];
        let mut inner = tex::string_tokens(&format!("{base}/"));
        inner.push(Token::Char('#', Cat::Param));
        inner.push(Token::other('1'));
        body.extend(group(inner));
        self.define_code(base, 1, body);
        Ok(())
    }

    fn handler_expand(&mut self, base: &str, v: Vec<Token>) -> Result<(), Error> {
        let v = self.tex.expand_fully(v)?;
        if !self.run_key(base, Some(v.clone()))? {
            self.unknown(base, Some(v))?;
        }
        Ok(())
    }

    fn handler_get(&mut self, base: &str, v: Vec<Token>) -> Result<(), Error> {
        if let Some(target) = v.into_iter().find(|t| !t.is_space()) {
            let val = keys::value(&self.tex, base).unwrap_or_default();
            self.tex.set_meaning(&target, macro_tokens(val), false);
        }
        Ok(())
    }

    fn handler_add(&mut self, base: &str, v: Vec<Token>) -> Result<(), Error> {
        let args = split_arguments(&v, 2);
        let old = keys::value(&self.tex, base).unwrap_or_default();
        let new = [args[0].clone(), old, args[1].clone()].concat();
        self.tex.set_meaning(&Token::cs(&macro_name(base)), macro_tokens(new), false);
        Ok(())
    }

    fn handler_affix(&mut self, base: &str, h: &str, v: Vec<Token>) -> Result<(), Error> {
        let old = keys::value(&self.tex, base).unwrap_or_default();
        let new = if h == ".append" { [old, v].concat() } else { [v, old].concat() };
        self.tex.set_meaning(&Token::cs(&macro_name(base)), macro_tokens(new), false);
        Ok(())
    }

    fn handler_forward(&mut self, base: &str, v: Vec<Token>) -> Result<(), Error> {
        let target = tex::tokens_to_string(&v).trim().to_string();
        let mut inner = tex::string_tokens(&format!("{target}="));
        inner.extend(group(vec![Token::Char('#', Cat::Param), Token::other('1')]));
        self.define_code(base, 1, Self::style_body(inner));
        Ok(())
    }

    fn handler_list(&mut self, base: &str, v: Vec<Token>) -> Result<(), Error> {
        for item in super::foreach_items(&v) {
            if !self.run_key(base, Some(item.clone()))? {
                self.unknown(base, Some(item))?;
            }
        }
        Ok(())
    }

    fn handler_pic(&mut self, base: &str, v: Vec<Token>) -> Result<(), Error> {
        // name/.pic={code}: the pic type `name`, under /tikz/pics
        let name = base.trim_start_matches("/tikz/").trim_start_matches('/');
        let mut body = tikra_tokens("code=");
        body.extend(group(v));
        self.define_code(&format!("/tikz/pics/{name}"), 1, Self::style_body(body));
        Ok(())
    }

    fn handler_user(&mut self, base: &str, other: &str, value: Option<Vec<Token>>) -> Result<(), Error> {
        // a user handler: /handlers/<name>/.code
        let user = format!("/handlers/{}", other);
        self.tex.define_string("pgfkeyscurrentpath", base, false);
        if !self.run_key(&user, value)? {
            self.warn(format!("key handler '{other}' not supported (in '{base}/{other}'): ignored"));
        }
        Ok(())
    }

    fn handler(&mut self, base: &str, h: &str, value: Option<Vec<Token>>) -> Result<(), Error> {
        let v = value.clone().unwrap_or_default();
        match h {
            ".cd" => self.st.key_path = base.to_string(),
            ".style" | ".estyle" => self.handler_style(base, h, v)?,
            ".append style" | ".prefix style" => self.handler_restyle(base, h, v)?,
            ".code" | ".ecode" => self.handler_code(base, h, v)?,
            ".condition" => self.handler_condition(base, v)?,
            ".from" => {
                self.tex.set_meaning(&Token::cs(&macro_name(&format!("{base}/@from"))), macro_tokens(v), false);
            }
            ".expression" | ".append expression" => self.handler_expression(base, h, v)?,
            ".append code" | ".prefix code" => self.handler_affix_code(base, h, v)?,
            ".code 2 args" | ".style 2 args" | ".code n args" | ".style n args" | ".ecode 2 args" | ".estyle 2 args" => self.handler_n_args(base, h, v)?,
            ".code args" | ".style args" | ".ecode args" | ".estyle args" => self.handler_code_args(base, h, v)?,
            ".tip" => self.handler_tip(base, v)?,
            ".default" => self.tex.set_meaning(&Token::cs(&macro_name(&format!("{base}/.@def"))), macro_tokens(v), false),
            ".initial" => self.tex.set_meaning(&Token::cs(&macro_name(base)), macro_tokens(v), false),
            ".is family" => self.define_code(base, 0, tikra_tokens(&format!("\\pgfkeysalso{{{base}/.cd}}"))),
            ".value required" | ".value forbidden" | ".show code" | ".show value" | ".activate family" | ".deactivate family" | ".belongs to family" => {}
            ".store in" | ".estore in" => self.handler_store(base, h, v)?,
            ".is if" => self.handler_is_if(base, v)?,
            ".is choice" => self.handler_is_choice(base)?,
            ".search also" => {
                let s = tex::tokens_to_string(&v);
                self.tex.define_string(&macro_name(&format!("{base}/.@searchalso")), &s, false);
            }
            ".try" | ".retry" | ".lastretry" => {
                let _ = self.run_key(base, value)?;
            }
            ".expand once" | ".expand twice" | ".expanded" => self.handler_expand(base, v)?,
            ".get" => self.handler_get(base, v)?,
            ".add" => self.handler_add(base, v)?,
            ".append" | ".prefix" => self.handler_affix(base, h, v)?,
            ".forward to" => self.handler_forward(base, v)?,
            ".list" => self.handler_list(base, v)?,
            ".pic" => self.handler_pic(base, v)?,
            ".unknown" => {
                // `/foo/.unknown/.code={…}` reaches here as a handler of the path /foo/.unknown
                self.define_code(&format!("{base}/.unknown"), 1, v);
            }
            other => self.handler_user(base, other, value)?,
        }
        Ok(())
    }
}

/// `{a}{b}` (or `ab`) as `n` arguments, TeX-style.
pub fn split_arguments(v: &[Token], n: usize) -> Vec<Vec<Token>> {
    let mut out = Vec::new();
    let mut i = 0;
    while out.len() < n {
        while i < v.len() && v[i].is_space() {
            i += 1;
        }
        if i >= v.len() {
            out.push(Vec::new());
            continue;
        }
        if matches!(v[i], Token::Char(_, Cat::Begin)) {
            let mut depth = 0usize;
            let start = i;
            while i < v.len() {
                match v[i] {
                    Token::Char(_, Cat::Begin) => depth += 1,
                    Token::Char(_, Cat::End) => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            out.push(v[start + 1..i.min(v.len())].to_vec());
            i += 1;
        } else {
            out.push(vec![v[i].clone()]);
            i += 1;
        }
    }
    out
}
