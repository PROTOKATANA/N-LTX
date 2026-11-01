//! Commands that are not a path: each one reads what it needs and does its one job.

use super::*;
use crate::canvas::Item;
use crate::color;
use crate::scene::Point;
use crate::tex::{self, Cat, Meaning, Prim, Token};
use crate::Error;
use std::rc::Rc;

impl Engine {
    /// Whether the command `id` only computes with pgfmath (`\pgfmathparse`, `\pgfmathadd`…): code
    /// that may come before a point, as in `{\pgfmathrand\pgfpoint{…}{\pgfmathresult pt}}`.
    pub(super) fn is_pgfmath(id: u16) -> bool {
        [Cmd::Pgfmathparse, Cmd::Pgfmathsetmacro, Cmd::Pgfmathtruncatemacro, Cmd::Pgfmathsetlengthmacro, Cmd::PgfmathFunction].iter().any(|c| *c as u16 == id)
    }

    /// `\documentclass[…]`, read with tolerance: a bracket left open ends before the next `\begin`.
    pub(super) fn documentclass_options(&mut self) -> Option<Vec<Token>> {
        let t = self.tex.next_nonspace()?;
        if !t.is_char('[') {
            self.tex.back(t);
            return None;
        }
        let mut out = Vec::new();
        let mut depth = 0usize;
        while let Some(x) = self.tex.get_token() {
            match &x {
                Token::Char(_, Cat::Begin) => depth += 1,
                Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
                x if depth == 0 && x.is_char(']') => return Some(out),
                x if x.is_cs("begin") => {
                    self.tex.back(x.clone());
                    return Some(out);
                }
                _ => {}
            }
            out.push(x);
        }
        Some(out)
    }

    /// The `border` option of `\documentclass[border=…]{standalone}`.
    /// xcolor's `dvipsnames`, `svgnames` and `x11names`: each defines its colors, over those of
    /// the same name before it (the options are taken in their order, the class's first).
    pub(super) fn load_color_sets(&mut self, options: &str) {
        for set in options.split(',').filter_map(color::sets::Set::of_option) {
            self.colors.extend(set.colors().into_iter().map(|(n, c)| (n.to_string(), c)));
        }
    }

    pub(super) fn standalone_options(&mut self, o: Vec<Token>) -> Result<(), Error> {
        for item in crate::keys::split_list(&o) {
            if item.key == "varwidth" {
                if let Some(v) = item.value {
                    let s = self.tex.expand_to_string(v)?;
                    if let Ok(w) = self.length(s.trim()) {
                        self.page.varwidth = Some(w);
                    }
                }
            } else if item.key == "border" {
                if let Some(v) = item.value {
                    let s = self.tex.expand_to_string(v)?;
                    let first = s.split_whitespace().next().unwrap_or("0pt").to_string();
                    if let Ok(m) = self.length(&first) {
                        self.canvas.margin = m;
                    }
                }
            }
        }
        Ok(())
    }
    pub(super) fn command_set_seed(&mut self) -> Result<(), Error> {
        // \pgfmathsetseed: the integer part of the value
        let v = self.argument_string()?;
        let n: f64 = v.trim().parse().unwrap_or(1.0);
        crate::expr::set_seed(n.trunc() as i64);
        Ok(())
    }

    pub(super) fn command_tikzset(&mut self) -> Result<(), Error> {
        let a = self.argument()?;
        self.keys_command(a, Some("/tikz"), true)?;
        // `\tikzset{xshift=1cm}` in a picture transforms what follows in the group
        if self.picture_depth > 0 {
            self.st.apply_pending();
        }
        Ok(())
    }

    pub(super) fn command_definecolor(&mut self) -> Result<(), Error> {
        // \definecolor{name}{model}{values}, with an optional [type] first
        let _ = self.optional()?;
        let name = self.argument_string()?;
        let model = self.argument_string()?;
        let values = self.argument_string()?;
        match color::define(&model, &values) {
            Some(c) => {
                self.colors.insert(name, c);
            }
            None => self.warn(format!("\\definecolor{{{name}}}{{{model}}}: model not supported")),
        }
        Ok(())
    }

    pub(super) fn command_colorlet(&mut self) -> Result<(), Error> {
        let _ = self.optional()?;
        let name = self.argument_string()?;
        let e = self.argument_string()?;
        match self.color(&e) {
            Some(c) => {
                self.colors.insert(name, c);
            }
            None => self.warn(format!("\\colorlet{{{name}}}: unknown color '{e}'")),
        }
        Ok(())
    }

    pub(super) fn command_pagecolor(&mut self) -> Result<(), Error> {
        // `\pagecolor{olive!50}` or `\pagecolor[RGB]{200,30,30}`
        let model = self.optional()?.map(|m| tex::tokens_to_string(&m));
        let spec = self.argument_string()?;
        match model.map_or_else(|| self.color(&spec), |m| color::define(&m, &spec)) {
            Some(c) => self.page.page_color = Some(c),
            None => self.warn(format!("\\pagecolor: unknown color '{spec}'")),
        }
        Ok(())
    }

    pub(super) fn command_math_parse(&mut self) -> Result<(), Error> {
        let expr = self.argument()?;
        let text = self.math_string(expr)?;
        let mut vars = self.vars();
        vars.in_pt = true;
        let value = match crate::expr::eval_string(&text, &vars) {
            Ok(Some(s)) => Ok(s),
            Ok(None) => crate::expr::eval_length(&text, &vars).map(|(v, _)| pgf_number(v)),
            Err(e) => Err(e),
        };
        match value {
            Ok(v) => self.tex.define_string("pgfmathresult", &v, false),
            Err(e) => self.warn(format!("\\pgfmathparse: {e}")),
        }
        Ok(())
    }

    pub(super) fn command_math_register(&mut self, cmd: Cmd, t: &Token) -> Result<(), Error> {
        let target = self.argument()?;
        let expr = self.argument()?;
        let text = self.math_string(expr)?;
        let add = matches!(cmd, Cmd::Pgfmathaddtolength | Cmd::Pgfmathaddtocounter);
        // lengths in pt, as pgfmath counts them
        let mut vars = self.vars();
        vars.in_pt = true;
        // a LaTeX counter is the register `\c@name`; pgfmath sets it locally, unlike `\setcounter`
        let m = if matches!(cmd, Cmd::Pgfmathsetcounter | Cmd::Pgfmathaddtocounter) {
            self.tex.meaning_of(&format!("c@{}", tex::tokens_to_string(&target).trim()))
        } else {
            let Some(name) = target.iter().find(|x| !x.is_space()) else { return Ok(()) };
            self.tex.meaning(name)
        };
        match m {
            Meaning::Count(r) => {
                // the value is truncated before it is added
                let v = crate::expr::eval(&text, &vars)?.trunc() as i64;
                let old = if add { self.tex.counts.get(&r).copied().unwrap_or(0) } else { 0 };
                self.tex.set_count(r, old + v, false);
            }
            Meaning::Dimen(r) | Meaning::Skip(r) => {
                let pt = crate::expr::eval_length(&text, &vars)?.0;
                let old = if add { self.tex.dimens.get(&r).copied().unwrap_or(0) } else { 0 };
                self.tex.set_dimen(r, old + (pt * 65536.0).round() as i64, false);
            }
            _ => self.warn(format!("{}: no register or counter '{}'", tex::show_token(t), tex::tokens_to_string(&target).trim())),
        }
        Ok(())
    }

    pub(super) fn command_print_number(&mut self) -> Result<(), Error> {
        let o = self.optional()?;
        let a = self.argument()?;
        let v = self.number_of(a)?;
        let saved = self.st.number_format.clone();
        if let Some(o) = o {
            let path = self.st.key_path.clone();
            self.process_keys(o, Some("/pgf/number format"))?;
            self.st.key_path = path;
        }
        let s = self.st.number_format.print(v);
        self.st.number_format = saved;
        self.tex.push_tokens(tex::string_tokens(&s));
        Ok(())
    }

    pub(super) fn command_pass_options(&mut self) -> Result<(), Error> {
        let options = self.argument_string()?;
        let packages = self.argument_string()?;
        if packages.split(',').any(|p| p.trim() == "xcolor") {
            self.load_color_sets(&options);
        }
        Ok(())
    }

    pub(super) fn command_hspace(&mut self) -> Result<(), Error> {
        match self.tex.next_nonspace() {
            Some(t) if t.is_char('*') => {}
            Some(t) => self.tex.back(t),
            None => {}
        }
        let arg = self.argument()?;
        if self.picture_depth == 0 {
            let w = self.length_of(arg)?;
            self.page.layout.push(Layout::Skip(w));
        }
        Ok(())
    }

    pub(super) fn command_hskip(&mut self) -> Result<(), Error> {
        let w = self.tex.scan_dimen()?;
        for k in ["plus", "minus"] {
            if self.tex.scan_keyword(k)? {
                let _ = self.tex.scan_dimen();
                let _ = self.tex.scan_keyword("fill")? || self.tex.scan_keyword("fil")?;
            }
        }
        if self.picture_depth == 0 {
            self.page.layout.push(Layout::Skip(w as f64 / 65536.0 * crate::PT));
        }
        Ok(())
    }

    pub(super) fn command_check_unit(&mut self) -> Result<(), Error> {
        // \tikz@checkunit{value}: \iftikz@isdimension is true when the value, read by
        // pgfmath, has a unit (`1cm`, `2*3pt`, a length register), false for a plain number
        let a = self.argument()?;
        let s = self.math_string(a)?;
        let unit = crate::expr::eval_length(&s, &self.vars()).is_ok_and(|(_, unit)| unit);
        self.tex.set_meaning(&Token::cs("iftikz@isdimension"), Meaning::Prim(if unit { Prim::Iftrue } else { Prim::Iffalse }), false);
        Ok(())
    }

    pub(super) fn command_node(&mut self, t: &Token) -> Result<(), Error> {
        if self.node_options_are_matrix()? {
            return self.matrix();
        }
        self.tex.push_tokens(tikra_tokens("node "));
        let r = super::path::statement(self, "");
        self.recover(r, t)
    }

    /// `\node[matrix]` is a `\matrix`: the options are put back either way.
    fn node_options_are_matrix(&mut self) -> Result<bool, Error> {
        let Some(o) = self.tex.next_nonspace() else { return Ok(false) };
        let mut back = vec![o.clone()];
        let mut matrix = false;
        if o.is_char('[') {
            let options = self.tex.read_until_char(']', false)?;
            matrix = crate::keys::split_list(&options).iter().any(|i| i.key == "matrix" && i.value.is_none());
            back.extend(options);
            back.push(Token::other(']'));
        }
        self.tex.push_tokens(back);
        Ok(matrix)
    }

    pub(super) fn command_random_list(&mut self) -> Result<(), Error> {
        // global, as pgf has it: the count in `\pgfmath@randomlist@name`, the items in
        // `\pgfmath@randomlist@name@1`… (`\pgfmathrandomitem` picks one)
        let name = self.argument_string()?;
        let list = self.argument()?;
        let items = random_list_items(list);
        self.define_random_list(&name, &items);
        Ok(())
    }

    fn define_random_list(&mut self, name: &str, items: &[Vec<Token>]) {
        for (k, item) in items.iter().enumerate() {
            self.tex.set_meaning(&Token::cs(&format!("pgfmath@randomlist@{name}@{}", k + 1)), macro_tokens(item.clone()), true);
        }
        self.tex.define_string(&format!("pgfmath@randomlist@{name}"), &items.len().to_string(), true);
    }

    pub(super) fn command_scoped(&mut self) -> Result<(), Error> {
        let o = self.scoped_option_list()?;
        self.open_tikz_scope(o, false)?;
        // the next statement or group is the content
        let body = self.next_statement()?;
        self.run_tokens(body)?;
        self.close_tikz_scope()
    }

    /// `\scoped[…]` and `\scoped :rotate={…}`: the scope's options, animations included.
    fn scoped_option_list(&mut self) -> Result<Option<Vec<Token>>, Error> {
        let mut o = self.optional()?;
        // `\scoped :rotate = {…}`: animated options of the scope
        while let Some(t) = self.tex.next_nonspace() {
            if !t.is_char(':') {
                self.tex.back(t);
                break;
            }
            if let Some(a) = self.read_animation()? {
                let v = o.get_or_insert_with(Vec::new);
                if !v.is_empty() {
                    v.push(Token::other(','));
                }
                // a scope's transformation is animated on the canvas, text and all
                let name = tex::tokens_to_string(&a);
                if ["rotate", "shift", "xshift", "yshift", "scale", "xscale", "yscale", "xslant", "yslant"].iter().any(|k| name.starts_with(&format!("{k}="))) {
                    v.extend(tikra_tokens("transform canvas="));
                    v.push(Token::Char('{', Cat::Begin));
                    v.extend(a);
                    v.push(Token::Char('}', Cat::End));
                } else {
                    v.extend(a);
                }
            }
        }
        Ok(o)
    }

    pub(super) fn command_tikzstyle(&mut self) -> Result<(), Error> {
        let name = self.tikzstyle_name()?;
        let append = self.tikzstyle_appends()?;
        let o = self.optional()?.unwrap_or_default();
        let mut v = tikra_tokens(&format!("{name}/.{}=", if append { "append style" } else { "style" }));
        v.push(Token::Char('{', Cat::Begin));
        v.extend(o);
        v.push(Token::Char('}', Cat::End));
        self.keys_command(v, Some("/tikz"), true)
    }

    /// `\tikzstyle{name}=[options]` or `+=`; without braces the name runs to the `=`.
    fn tikzstyle_name(&mut self) -> Result<String, Error> {
        let name = match self.tex.next_nonspace() {
            Some(t) if !matches!(t, Token::Char(_, Cat::Begin)) => {
                let mut name = vec![t];
                while let Some(t) = self.tex.get_token() {
                    if t.is_char('=') {
                        self.tex.back(t);
                        break;
                    }
                    name.push(t);
                }
                tex::tokens_to_string(&name)
            }
            t => {
                if let Some(t) = t {
                    self.tex.back(t);
                }
                self.argument_string()?
            }
        };
        Ok(name)
    }

    fn tikzstyle_appends(&mut self) -> Result<bool, Error> {
        let mut append = false;
        if let Some(x) = self.tex.next_nonspace() {
            if x.is_char('+') {
                append = true;
            } else if !x.is_char('=') {
                self.tex.back(x);
            }
        }
        if append {
            if let Some(x) = self.tex.next_nonspace() {
                if !x.is_char('=') {
                    self.tex.back(x);
                }
            }
        }
        Ok(append)
    }

    pub(super) fn command_color(&mut self) -> Result<(), Error> {
        let _ = self.optional()?;
        let e = self.argument_string()?;
        self.apply_text_color(&e);
        self.note_varwidth_color();
        Ok(())
    }

    fn apply_text_color(&mut self, e: &str) {
        if let Some(c) = self.color(e) {
            self.st.paint.color = c;
            self.st.paint.draw_color = None;
            self.st.paint.fill_color = None;
            self.st.paint.text_color = None;
        }
    }

    /// A `\color` of the page's text, in no group: it is still on at the paragraph's end,
    /// and `varwidth` takes its whole width (a color's end is not text it can measure).
    fn note_varwidth_color(&mut self) {
        if let (0, Some(w), Some(d)) = (self.picture_depth, self.page.varwidth, self.page.document_depth) {
            if self.tex.group_depth() == d && !self.page.layout.contains(&Layout::Width(w)) {
                self.page.layout.push(Layout::Width(w));
            }
        }
    }

    pub(super) fn command_math_macro(&mut self, cmd: Cmd) -> Result<(), Error> {
        let name = self.argument()?;
        let expr = self.argument()?;
        let Some(name) = name.into_iter().find(|x| !x.is_space()) else { return Ok(()) };
        let text = self.math_string(expr)?;
        // lengths in pt, as pgfmath counts them
        let mut vars = self.vars();
        vars.in_pt = true;
        let store = MathStore { cmd, name: &name, text: &text, vars: &vars };
        if cmd == Cmd::Pgfmathsetmacro && self.store_math_text(&store) {
            return Ok(());
        }
        self.store_math_number(&store);
        Ok(())
    }

    fn store_math_text(&mut self, store: &MathStore) -> bool {
        if let Ok(Some(s)) = crate::expr::eval_string(store.text, store.vars) {
            self.tex.set_meaning(store.name, macro_from(&s), false);
            self.tex.define_string("pgfmathresult", &s, false);
            return true;
        }
        false
    }

    fn store_math_number(&mut self, store: &MathStore) {
        let v = crate::expr::eval_length(store.text, store.vars).map(|(v, _)| v);
        match v {
            Ok(v) if v.is_finite() => {
                let s = match store.cmd {
                    Cmd::Pgfmathtruncatemacro => format!("{}", v.trunc() as i64),
                    Cmd::Pgfmathsetlengthmacro => format!("{}pt", pgf_number(v)),
                    _ => pgf_number(v),
                };
                self.tex.set_meaning(store.name, macro_from(&s), false);
                self.tex.define_string("pgfmathresult", &s, false);
            }
            Ok(v) => self.warn(format!("\\pgfmathsetmacro{{{}}}: non-finite result ({v}), macro not defined", tex::show_token(store.name))),
            Err(e) => self.warn(format!("\\pgfmathsetmacro{{{}}}: {e}", tex::show_token(store.name))),
        }
    }

    pub(super) fn command_math_function(&mut self, t: &Token) -> Result<(), Error> {
        let Some(name) = pgfmath_name(t) else { return Ok(()) };
        let args = self.read_math_args(&name)?;
        let text = math_call_text(&name, &args);
        self.store_math_call(&name, &text);
        Ok(())
    }

    fn read_math_args(&mut self, name: &str) -> Result<Vec<String>, Error> {
        let n = if matches!(name, "add" | "subtract" | "multiply" | "divide" | "max" | "min" | "mod" | "Mod" | "pow" | "veclen" | "atan2") { 2 } else { 1 };
        let mut args = Vec::new();
        for _ in 0..n {
            let a = self.argument()?;
            args.push(format!("({})", self.math_string(a)?));
        }
        Ok(args)
    }

    fn store_math_call(&mut self, name: &str, text: &str) {
        match crate::expr::eval_length(text, &self.vars()).map(|(v, u)| if u { v / crate::PT } else { v }) {
            Ok(v) => self.tex.define_string("pgfmathresult", &pgf_number(v), false),
            Err(e) => self.warn(format!("\\pgfmath{name}: {e}")),
        }
    }

    pub(super) fn command_declare_function(&mut self) -> Result<(), Error> {
        // \pgfmathdeclarefunction{name}{n}{code}: code sets \pgfmathresult; only `\pgfmathparse{expr}` bodies are understood
        let name = self.argument_string()?;
        let n: usize = self.argument_string()?.parse().unwrap_or(0);
        let body = self.argument()?;
        let Some(expr) = pgfmath_declared_expr(&body) else {
            self.warn(format!("\\pgfmathdeclarefunction{{{name}}}: only a \\pgfmathparse body is supported"));
            return Ok(());
        };
        let (params, expr) = bind_function_params(expr, n);
        Rc::make_mut(&mut self.st.functions).insert(name, (params, expr));
        Ok(())
    }

    pub(super) fn command_load(&mut self, cmd: Cmd) -> Result<(), Error> {
        let options = if cmd == Cmd::Documentclass { self.documentclass_options() } else { self.optional()? };
        let a = self.package_list_name()?;
        let a = self.note_library_flags(cmd, a);
        let options = self.note_fontenc(cmd, &a, options)?;
        let options = self.note_xcolor_package(cmd, &a, options)?;
        let options = self.note_paper(cmd, &a, options);
        let options = self.note_preview(cmd, &a, options)?;
        self.note_standalone(cmd, &a, options)
    }

    /// The name in braces (a broken \documentclass[ may have none).
    fn package_list_name(&mut self) -> Result<String, Error> {
        let a = match self.tex.next_nonspace() {
            Some(Token::Char(_, Cat::Begin)) => {
                let body = self.tex.read_group_body()?;
                self.tex.expand_to_string(body)?
            }
            Some(t) => {
                self.tex.back(t);
                String::new()
            }
            None => String::new(),
        };
        Ok(a)
    }

    /// The mindmap library sets `/pgf/decoration/angle=20` as it loads; positioning switches node placement on.
    fn note_library_flags(&mut self, cmd: Cmd, a: String) -> String {
        if cmd == Cmd::Usetikzlibrary && a.split(',').any(|l| l.trim() == "mindmap") {
            self.st.decoration.angle = 20.0;
        }
        if cmd == Cmd::Usetikzlibrary && a.split(',').any(|l| l.trim() == "positioning") {
            self.st.positioning = true;
        }
        a
    }

    /// `\usepackage[T1]{fontenc}`: text kerned as the T1 fonts kern it (the last encoding listed is the one in force).
    fn note_fontenc(&mut self, cmd: Cmd, a: &str, o: Option<Vec<Token>>) -> Result<Option<Vec<Token>>, Error> {
        if cmd == Cmd::Usepackage && a.split(',').any(|p| p.trim() == "fontenc") {
            let encodings = o.as_ref().map(|o| self.tex.expand_to_string(o.clone())).transpose()?.unwrap_or_default();
            let last = encodings.split(',').map(str::trim).rfind(|e| !e.is_empty()).unwrap_or("OT1");
            crate::label::set_kerning(if last == "T1" { mathra::layout::kerning::TextKerning::T1 } else { mathra::layout::kerning::TextKerning::Ot1 });
        }
        Ok(o)
    }

    /// xcolor takes the class's options too (tikz loads xcolor).
    fn note_xcolor_package(&mut self, cmd: Cmd, a: &str, o: Option<Vec<Token>>) -> Result<Option<Vec<Token>>, Error> {
        if cmd == Cmd::Documentclass || a.split(',').any(|p| p.trim() == "xcolor") {
            if let Some(o) = &o {
                let options = self.tex.expand_to_string(o.clone())?;
                self.load_color_sets(&options);
            }
        }
        Ok(o)
    }

    /// The paper's size, from the class options or geometry's.
    fn note_paper(&mut self, cmd: Cmd, a: &str, o: Option<Vec<Token>>) -> Option<Vec<Token>> {
        if cmd == Cmd::Documentclass || (cmd == Cmd::Usepackage && a.split(',').any(|p| p.trim() == "geometry")) {
            if let Some(o) = &o {
                self.tex.paper_options(&tex::tokens_to_string(o));
            }
        }
        o
    }

    /// `\usepackage[active]{preview}`: each picture is a page of its own.
    fn note_preview(&mut self, cmd: Cmd, a: &str, o: Option<Vec<Token>>) -> Result<Option<Vec<Token>>, Error> {
        if cmd == Cmd::Usepackage && a.split(',').any(|p| p.trim() == "preview") {
            let options = o.as_ref().map(|o| self.tex.expand_to_string(o.clone())).transpose()?.unwrap_or_default();
            if options.split(',').any(|o| o.trim() == "active") {
                let m = self.tex.meaning_of("tikra@previewenvironment");
                self.tex.set_meaning(&Token::cs("PreviewEnvironment"), m, false);
            }
        }
        Ok(o)
    }

    fn note_standalone(&mut self, cmd: Cmd, a: &str, o: Option<Vec<Token>>) -> Result<(), Error> {
        if cmd == Cmd::Documentclass && a.contains("standalone") {
            if let Some(o) = o {
                self.standalone_options(o)?;
            }
        }
        Ok(())
    }

    pub(super) fn command_fbox(&mut self) -> Result<(), Error> {
        let arg = self.argument()?;
        let start = self.run_in_group(arg)?;
        if self.picture_depth == 0 {
            self.frame_fboxes(start);
        }
        Ok(())
    }

    pub(super) fn command_scalebox(&mut self) -> Result<(), Error> {
        let scale = self.scalebox_factors()?;
        let arg = self.argument()?;
        let start = self.run_in_group(arg)?;
        if self.picture_depth == 0 {
            self.scale_boxes(start, move |_, _, _| (scale.x, scale.y));
        }
        Ok(())
    }

    fn scalebox_factors(&mut self) -> Result<ScaleBy, Error> {
        let fx = self.argument_string()?;
        let fx = self.number(&fx)?;
        let fy = match self.optional()? {
            Some(t) => {
                let t = self.tex.expand_to_string(t)?;
                self.number(&t)?
            }
            None => fx,
        };
        Ok(ScaleBy { x: fx, y: fy })
    }

    pub(super) fn command_resizebox(&mut self) -> Result<(), Error> {
        let to = self.resize_target()?;
        let arg = self.argument()?;
        let start = self.run_in_group(arg)?;
        if self.picture_depth == 0 {
            self.scale_boxes(start, move |width, height, depth| {
                let fx = to.size[0].map(|x| x / width);
                let fy = to.size[1].map(|y| y / if to.star { height + depth } else { height });
                (fx.or(fy).unwrap_or(1.0), fy.or(fx).unwrap_or(1.0))
            });
        }
        Ok(())
    }

    fn resize_target(&mut self) -> Result<ResizeTo, Error> {
        let next = self.tex.next_nonspace();
        let star = next.as_ref().is_some_and(|t| t.is_char('*'));
        if let Some(t) = next.filter(|_| !star) {
            self.tex.back(t);
        }
        let mut size = [None, None];
        for s in &mut size {
            let t = self.argument_string()?;
            if t.trim() != "!" {
                match self.length(&t) {
                    Ok(v) => *s = Some(v),
                    // (`\width`, the box's own size, is not known here)
                    Err(_) => self.warn(format!("\\resizebox: size '{t}' not understood: not resized to it")),
                }
            }
        }
        Ok(ResizeTo { star, size })
    }

    fn run_in_group(&mut self, arg: Vec<Token>) -> Result<usize, Error> {
        let start = self.page.boxes.len();
        self.begin_group();
        let r = self.run_tokens(arg);
        self.end_group();
        r?;
        Ok(start)
    }

    fn frame_fboxes(&mut self, start: usize) {
        let sep = self.tex.dimens.get(&247).copied().unwrap_or(0) as f64 / 65536.0 * crate::PT;
        let rule = self.tex.dimens.get(&246).copied().unwrap_or(0) as f64 / 65536.0 * crate::PT;
        for b in &mut self.page.boxes[start..] {
            let (Some(lo), Some(hi)) = (b.canvas.min, b.canvas.max) else { continue };
            let d = sep + rule / 2.0;
            let (lo, hi) = (Point::new(lo.x - d, lo.y - d), Point::new(hi.x + d, hi.y + d));
            use crate::scene::Segment as S;
            let segments = vec![S::MoveTo(lo), S::LineTo(Point::new(hi.x, lo.y)), S::LineTo(hi), S::LineTo(Point::new(lo.x, hi.y)), S::Close];
            let stroke = crate::canvas::Stroke {
                color: self.st.paint.ink.unwrap_or(crate::color::Color::BLACK),
                width: rule,
                dash: None,
                cap: crate::canvas::Cap::Butt,
                join: crate::canvas::Join::Miter,
                miter_limit: 10.0,
                opacity: 1.0,
            };
            b.canvas.items.push(Item::Path { segments, fill: None, stroke: Some(stroke) });
            b.canvas.include(Point::new(lo.x - rule / 2.0, lo.y - rule / 2.0));
            b.canvas.include(Point::new(hi.x + rule / 2.0, hi.y + rule / 2.0));
        }
    }
}

struct MathStore<'a> {
    cmd: Cmd,
    name: &'a Token,
    text: &'a str,
    vars: &'a crate::expr::Variables,
}

fn pgfmath_name(t: &Token) -> Option<String> {
    match t {
        Token::Cs(n) => Some(n.trim_start_matches("pgfmath").trim_end_matches('@').to_string()),
        _ => None,
    }
}

fn math_call_text(name: &str, args: &[String]) -> String {
    match name {
        "add" => format!("{}+{}", args[0], args[1]),
        "subtract" => format!("{}-{}", args[0], args[1]),
        "multiply" => format!("{}*{}", args[0], args[1]),
        "divide" => format!("{}/{}", args[0], args[1]),
        "neg" => format!("-{}", args[0]),
        other => format!("{other}({})", args.join(",")),
    }
}

fn pgfmath_declared_expr(body: &[Token]) -> Option<String> {
    let text = tex::tokens_to_string(body);
    text.split("\\pgfmathparse").nth(1).map(|r| r.trim().trim_start_matches('{').trim_end_matches('}').to_string())
}

/// `#1`, `#2`… become variables the expressions read (`\tikraarga`…).
fn bind_function_params(mut expr: String, n: usize) -> (Vec<String>, String) {
    let params: Vec<String> = (1..=n.min(9)).map(|k| format!("tikraarg{}", (b'a' + k as u8 - 1) as char)).collect();
    for (k, p) in params.iter().enumerate() {
        let var = format!("\\{p}");
        expr = expr.replace(&format!("##{}", k + 1), &var).replace(&format!("#{}", k + 1), &var);
    }
    (params, expr)
}

fn random_list_items(list: Vec<Token>) -> Vec<Vec<Token>> {
    let mut items: Vec<Vec<Token>> = Vec::new();
    let mut depth = 0;
    for x in list {
        match &x {
            Token::Char(_, Cat::Begin) => {
                depth += 1;
                if depth == 1 {
                    items.push(Vec::new());
                    continue;
                }
            }
            Token::Char(_, Cat::End) => {
                depth -= 1;
                if depth == 0 {
                    continue;
                }
            }
            _ if depth == 0 => {
                // an item without braces is its one token
                if !x.is_space() {
                    items.push(vec![x]);
                }
                continue;
            }
            _ => {}
        }
        if let Some(last) = items.last_mut() {
            last.push(x);
        }
    }
    items
}

struct ScaleBy {
    x: f64,
    y: f64,
}

struct ResizeTo {
    star: bool,
    size: [Option<f64>; 2],
}
