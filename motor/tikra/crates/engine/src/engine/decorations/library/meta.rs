//! Meta-decorations: a state decorates one stretch, then the automaton moves on.

use super::*;

impl Run<'_> {
    /// The options of a declared state, read as `/pgf/decoration automaton` keys.
    pub(super) fn user_options(&mut self, name: &str, state: &str) -> Vec<Opt> {
        let Some(tokens) = macro_body(self.e, &format!("pgf@decorate@@{name}@{state}@options")) else {
            return Vec::new();
        };
        super::super::set_registers(self.e, &self.w);
        let mut out = Vec::new();
        self.user_opts(&tokens, &mut out);
        out
    }

    fn user_opts(&mut self, tokens: &[Token], out: &mut Vec<Opt>) {
        for item in crate::keys::split_list(tokens) {
            let v = item.value.unwrap_or_default();
            let len = |r: &mut Run, v: Vec<Token>| match r.e.length_of(v) {
                Ok(l) => l / PT,
                Err(err) => {
                    r.e.warn(format!("decoration state option '{}': {err}", item.key));
                    0.0
                }
            };
            let target = |r: &mut Run, v: &[Token]| -> Option<(f64, &'static str)> {
                let (x, s) = super::super::split_word(v, "to")?;
                let x = len(r, x);
                Some((x, intern(crate::tex::tokens_to_string(&s).trim())))
            };
            match item.key.as_str() {
                "width" => out.push(Opt::Width(len(self, v))),
                "next state" => out.push(Opt::Next(intern(&self.e.tex.expand_to_string(v).unwrap_or_default()))),
                "switch if less than" => {
                    if let Some((x, s)) = target(self, &v) {
                        out.push(Opt::Switch(x, s));
                    }
                }
                "switch if input segment less than" => {
                    if let Some((x, s)) = target(self, &v) {
                        out.push(Opt::SwitchInput(x, s));
                    }
                }
                "repeat state" => out.push(Opt::Repeat(self.e.number_of(v).unwrap_or(0.0) as i64)),
                "persistent precomputation" => {
                    self.user_pre = v;
                    out.push(Opt::Pre);
                }
                "persistent postcomputation" => {
                    self.user_post = v;
                    out.push(Opt::Post);
                }
                "auto end on length" => out.push(Opt::AutoEnd(len(self, v))),
                "auto corner on length" => out.push(Opt::AutoCorner(len(self, v))),
                "if input segment is closepath" => {
                    if self.w.is_close {
                        self.user_opts(&v, out);
                    }
                }
                other => self.e.warn(format!("decoration state option '{other}' not supported: ignored")),
            }
        }
    }

    /// A declared meta-decoration (`\pgf@metadecoration@run`).
    fn run_user_meta(&mut self, name: &'static str) -> Result<(), Error> {
        let e = &mut *self.e;
        let initial = macro_body(e, &format!("pgf@metadecoration@@{name}@initial")).map(|v| crate::tex::tokens_to_string(&v)).unwrap_or_default();
        let mut state = intern(initial.trim());
        let defs =
            r"\def\decoration#1{\edef\pgf@decorate@tempname{#1}}\def\beforedecoration#1{\def\pgf@decorate@tempbefore{#1}}\def\afterdecoration#1{\def\pgf@decorate@tempafter{#1}}";
        self.e.run_tokens(crate::engine::tikra_tokens(defs))?;
        let mut guard = 0usize;
        loop {
            guard += 1;
            if guard > crate::engine::MAX_DECORATION_STEPS {
                self.e.warn("meta-decoration: too many steps, stopped");
                break;
            }
            if state == "final" {
                break;
            }
            self.install_points(&Transform::default());
            let mut width = 0.0;
            let mut next = state;
            let mut jump = false;
            if let Some(opts) = macro_body(self.e, &format!("pgf@metadecoration@@{name}@{state}@options")) {
                for item in crate::keys::split_list(&opts) {
                    let v = item.value.unwrap_or_default();
                    let mut switch = |r: &mut Run, v: &[Token], input: bool| -> Result<(), Error> {
                        if jump {
                            return Ok(());
                        }
                        let Some((x, s)) = super::super::split_word(v, "to") else {
                            return Ok(());
                        };
                        let x = r.e.length_of(x)? / PT;
                        let d = if input { r.w.seg_remaining } else { r.w.rem_total };
                        if d < x - EPS {
                            state = intern(crate::tex::tokens_to_string(&s).trim());
                            jump = true;
                        }
                        Ok(())
                    };
                    match item.key.as_str() {
                        "width" => {
                            width = self.e.length_of(v.clone())? / PT;
                            let mut t = v;
                            t.extend(crate::engine::tikra_tokens(" to final"));
                            switch(self, &t, false)?;
                        }
                        "switch if less than" => switch(self, &v, false)?,
                        "switch if input segment less than" => switch(self, &v, true)?,
                        "next state" => next = intern(&self.e.tex.expand_to_string(v)?),
                        other => self.e.warn(format!("meta-decoration state option '{other}' not supported: ignored")),
                    }
                }
            }
            if jump {
                continue;
            }
            let (dec, before, after) = self.meta_code(name, state)?;
            self.invoke_with(dec, width, before, after)?;
            state = next;
        }
        let (dec, before, after) = self.meta_code(name, "final")?;
        let rem = self.w.rem_total;
        self.invoke_with(dec, rem, before, after)
    }

    /// Runs a meta state's code: the decoration it names and its code before and after.
    fn meta_code(&mut self, name: &str, state: &str) -> Result<(Dec, Code, Code), Error> {
        for m in ["pgf@decorate@tempname", "pgf@decorate@tempbefore", "pgf@decorate@tempafter"] {
            self.e.tex.define_string(m, "", false);
        }
        if let Some(code) = macro_body(self.e, &format!("pgf@metadecoration@@{name}@{state}@code")) {
            self.tex_code(code, &Transform::default(), false, true)?;
            // what the code set (`\pgfset{/pgf/decoration/segment length=4pt}`) holds for the
            // decorations from here on
            self.s = self.e.st.decoration.clone();
        }
        let dname = macro_body(self.e, "pgf@decorate@tempname").map(|v| crate::tex::tokens_to_string(&v)).unwrap_or_default();
        let dname = if dname.trim().is_empty() { "moveto".to_string() } else { dname };
        let dec = resolve(self.e, &dname).unwrap_or_else(|| {
            self.e.warn(format!("decoration '{}' not supported: moved over", dname.trim()));
            Dec::MoveTo
        });
        let code = |r: &mut Run, n: &str| macro_body(r.e, n).filter(|v| !crate::keys::trim(v).is_empty());
        let before = code(self, "pgf@decorate@tempbefore");
        let after = code(self, "pgf@decorate@tempafter");
        Ok((dec, before, after))
    }

    /// `\pgfmetadecoration`: runs the meta-automaton, each state decorating a stretch.
    pub fn run_meta(&mut self, meta: Meta, main: Option<Dec>) -> Result<(), Error> {
        if let Meta::User(name) = meta {
            return self.run_user_meta(name);
        }
        let mut state = "initial";
        let main_name = main.unwrap_or(Dec::LineTo);
        let pre = self.key_text("pre");
        let post = self.key_text("post");
        let named = |e: &mut Engine, n: &str| -> Dec {
            resolve(e, n).unwrap_or_else(|| {
                e.warn(format!("decoration '{n}' not supported: drawn as lineto"));
                Dec::LineTo
            })
        };
        let mut guard = 0usize;
        loop {
            guard += 1;
            if guard > crate::engine::MAX_DECORATION_STEPS {
                self.e.warn("decoration: too many steps, stopped");
                break;
            }
            let (width, next, dec) = match (meta, state) {
                (Meta::StraightZigzag, "initial") => (self.s.meta_segment_length, "zigzag", Dec::CurveTo),
                (Meta::StraightZigzag, _) => (self.s.meta_segment_length, "initial", Dec::Zigzag),
                (Meta::Internal, "initial") => (self.key_pt("pre length", 0.0), "main", named(self.e, &pre)),
                (Meta::Internal, _) => (self.w.rem_total - self.key_pt("post length", 0.0), "final", main_name),
                (Meta::User(_), _) => unreachable!("run above"),
            };
            if self.w.rem_total < width - EPS {
                break;
            }
            if meta == Meta::Internal {
                self.extra = self.s.extra_transform();
            }
            self.invoke(dec, width)?;
            state = next;
            if state == "final" {
                break;
            }
        }
        let dec = match meta {
            Meta::StraightZigzag => Dec::CurveTo,
            Meta::Internal => {
                self.extra = self.s.extra_transform();
                named(self.e, &post)
            }
            Meta::User(_) => unreachable!("run above"),
        };
        let rem = self.w.rem_total;
        self.invoke(dec, rem)
    }
}
