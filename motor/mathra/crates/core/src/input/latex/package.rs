//! Commands of physics, braket, mathtools and the other packages, rewritten as primitives.

use super::formula::staggered_indices;
use super::*;

impl Reader<'_> {
    /// Package commands defined through other commands (physics, braket, xfrac, units, tensor, mathtools).
    pub(super) fn package_expansion(&mut self, name: &str, col: usize) -> R<Option<String>> {
        if let Some(tex) = colon_relation(name) {
            return Ok(Some(tex.to_string()));
        }
        if let Some(tex) = limit_operator(name) {
            return Ok(Some(tex.to_string()));
        }
        Ok(Some(match name {
            "fdv" | "functionalderivative" => self.functional_derivative()?,
            "dv" | "odv" | "pdv" | "derivative" | "partialderivative" | "pd" => self.derivative(name)?,
            "ket" | "Ket" => format!("\\left| {} \\right\\rangle", self.raw_text()?),
            "bra" | "Bra" => format!("\\left\\langle {} \\right|", self.raw_text()?),
            "operatornamewithlimits" => format!("\\operatorname*{{{}}}", self.raw_text()?),
            // mathtools: the operator with its limits; the limits keep their width (partial support)
            "smashoperator" => {
                self.raw_optional();
                format!("{{{}}}", self.raw_text()?)
            }
            // the LaTeX kernel
            "underbar" => format!("\\underline{{{}}}", self.raw_text()?),
            // definitions already last for the whole equation: \global changes nothing
            "global" => String::new(),
            "space" => "\\ ".into(),
            "nobreakspace" => "~".into(),
            // hyperref: the link's text is what the equation shows
            "href" => {
                self.raw_text()?;
                format!("{{{}}}", self.raw_text()?)
            }
            // TeX's \char: a character by its code, decimal, "hex, 'octal or `character
            "char" => self.char_code(col)?,
            "braket" | "Braket" | "innerproduct" | "ip" => self.angle_bracket()?,
            "ketbra" | "outerproduct" | "dyad" => {
                let a = self.raw_text()?;
                let b = self.optional_braced()?.unwrap_or_else(|| a.clone());
                format!("\\left| {a} \\right\\rangle\\!\\left\\langle {b} \\right|")
            }
            "expval" | "ev" | "expectationvalue" => {
                let a = self.raw_text()?;
                match self.optional_braced()? {
                    Some(psi) => format!("\\left\\langle {psi} \\middle| {a} \\middle| {psi} \\right\\rangle"),
                    None => format!("\\left\\langle {a} \\right\\rangle"),
                }
            }
            "mel" | "matrixel" | "matrixelement" => {
                let a = self.raw_text()?;
                let op = self.raw_text()?;
                let b = self.raw_text()?;
                format!("\\left\\langle {a} \\middle| {op} \\middle| {b} \\right\\rangle")
            }
            "comm" | "commutator" | "acomm" | "anticommutator" | "pb" | "poissonbracket" => {
                let a = self.raw_text()?;
                let b = self.raw_text()?;
                if matches!(name, "comm" | "commutator") {
                    format!("\\left[ {a}, {b} \\right]")
                } else {
                    format!("\\left\\{{ {a}, {b} \\right\\}}")
                }
            }
            // physics' vector calculus: the nabla in bold (U+1D6C1), like the package
            "grad" | "gradient" => "\\unicode{1D6C1} ".into(),
            "divergence" => "\\unicode{1D6C1} \\cdot ".into(),
            "curl" | "rot" => "\\unicode{1D6C1} \\times ".into(),
            "laplacian" => "\\nabla^{2} ".into(),
            "cross" | "crossproduct" => "\\times ".into(),
            "vdot" | "dotproduct" => "\\cdot ".into(),
            "vb" | "vectorbold" | "vu" | "vectorunit" | "va" | "vectorarrow" => self.physics_vector(name)?,
            "order" => match self.physics_brackets()? {
                Some(inner) if inner.starts_with("\\left\\{") => format!("\\mathcal{{O}}{}", inner.replacen("\\left\\{", "\\left(", 1).replacen("\\right\\}", "\\right)", 1)),
                Some(inner) => format!("\\mathcal{{O}}{inner}"),
                None => "\\mathcal{O}".into(),
            },
            "Tr" | "tr" | "rank" | "Res" | "erf" | "Rank" | "trace" => {
                let shown = if name == "trace" { "tr" } else { name };
                format!("\\operatorname{{{shown}}}")
            }
            "dd" | "differential" => {
                let order = self.raw_optional();
                let power = order.map(|n| format!("^{{{n}}}")).unwrap_or_default();
                let x = self.optional_braced()?.unwrap_or_default();
                format!("\\mathop{{}}\\!\\mathrm{{d}}{power}{x} ")
            }
            "eval" | "evaluated" => self.evaluated()?,
            // physics' \qty(…), \qty[…], \qty|…|, \qty{x}; siunitx's \qty{3}{m} (two groups) goes to the units
            "qty" | "quantity" if name == "quantity" || self.physics_qty_follows() => self.physics_brackets()?.unwrap_or_default(),
            "pqty" | "bqty" | "vqty" | "Bqty" => {
                let x = self.raw_text()?;
                match name {
                    "pqty" => format!("\\left( {x} \\right)"),
                    "bqty" => format!("\\left[ {x} \\right]"),
                    "vqty" => format!("\\left| {x} \\right|"),
                    _ => format!("\\left\\{{ {x} \\right\\}}"),
                }
            }
            "idotsint" => "\\int\\!\\cdots\\!\\int ".into(),
            // mathtools: \prescript{sup}{sub}{base}
            "prescript" => {
                let sup = self.raw_text()?;
                let sub = self.raw_text()?;
                let base = self.raw_text()?;
                format!("{{}}^{{{sup}}}_{{{sub}}}{{{base}}}")
            }
            // tensor package: \tensor[pre]{T}{^\mu_\nu}, \indices{^\mu_\nu}
            "tensor" | "indices" => self.tensor_indices(name)?,
            "Set" | "set" => {
                let x = self.raw_text()?;
                match x.split_once('|') {
                    Some((a, b)) => format!("\\left\\{{ {a} \\;\\middle|\\; {b} \\right\\}}"),
                    None => format!("\\left\\{{ {x} \\right\\}}"),
                }
            }
            "abs" => format!("\\left| {} \\right|", self.raw_text()?),
            "norm" => format!("\\left\\| {} \\right\\|", self.raw_text()?),
            "sfrac" | "nicefrac" => {
                let a = self.raw_text()?;
                let b = self.raw_text()?;
                format!("{{}}^{{{a}}}\\!/{{}}_{{{b}}}")
            }
            "unitfrac" => {
                let v = self.raw_optional();
                let a = self.raw_text()?;
                let b = self.raw_text()?;
                format!("{}\\,{{}}^{{\\mathrm{{{a}}}}}\\!/{{}}_{{\\mathrm{{{b}}}}}", v.unwrap_or_default())
            }
            "unit" if self.current() == Some('[') => {
                // \unit[3]{m} from the units package
                let v = self.raw_optional().unwrap_or_default();
                let u = self.raw_text()?;
                format!("{v}\\,\\mathrm{{{u}}}")
            }
            _ => {
                let _ = col;
                return Ok(None);
            }
        }))
    }

    fn functional_derivative(&mut self) -> R<String> {
        let f = self.raw_text()?;
        Ok(match self.optional_braced()? {
            Some(x) => format!("\\frac{{\\delta {f}}}{{\\delta {x}}}"),
            None => format!("\\frac{{\\delta}}{{\\delta {f}}}"),
        })
    }

    /// Ordinary or partial derivative, with an optional order and one or two variables.
    fn derivative(&mut self, name: &str) -> R<String> {
        let partial = name.starts_with('p');
        let order = self.raw_optional();
        let f = self.raw_text()?;
        self.skip_spaces();
        let x = if self.current() == Some('{') { Some(self.raw_text()?) } else { None };
        self.skip_spaces();
        let y = if partial && x.is_some() && self.current() == Some('{') { Some(self.raw_text()?) } else { None };
        let dd = if partial { "\\partial" } else { "\\mathrm{d}" };
        Ok(match (x, y, order) {
            (None, _, None) => format!("\\frac{{{dd}}}{{{dd} {f}}}"),
            (None, _, Some(n)) => format!("\\frac{{{dd}^{{{n}}}}}{{{dd} {f}^{{{n}}}}}"),
            (Some(x), None, None) => format!("\\frac{{{dd} {f}}}{{{dd} {x}}}"),
            (Some(x), None, Some(n)) => format!("\\frac{{{dd}^{{{n}}} {f}}}{{{dd} {x}^{{{n}}}}}"),
            (Some(x), Some(y), _) => format!("\\frac{{{dd}^{{2}} {f}}}{{{dd} {x} \\, {dd} {y}}}"),
        })
    }

    /// `\char`: a character by its code, decimal, `"hex`, `'octal` or `` `character ``.
    fn char_code(&mut self, col: usize) -> R<String> {
        self.skip_spaces();
        if self.current() == Some('`') {
            // `a, or `\a: the character itself
            self.pos += 1;
            if self.current() == Some('\\') {
                self.pos += 1;
            }
            let c = self.current().ok_or(Error::Syntax { column: col, message: "\\char` without a character".into() })?;
            self.pos += 1;
            return Ok(format!("\\unicode{{{:X}}}", c as u32));
        }
        let (radix, digits): (u32, fn(char) -> bool) = match self.current() {
            Some('"') => (16, |c| c.is_ascii_hexdigit()),
            Some('\'') => (8, |c| ('0'..='7').contains(&c)),
            _ => (10, |c| c.is_ascii_digit()),
        };
        if radix != 10 {
            self.pos += 1;
        }
        let code: String = self.c[self.pos..].iter().take_while(|c| digits(**c)).collect();
        self.pos += code.chars().count();
        match u32::from_str_radix(&code, radix).ok().and_then(char::from_u32).filter(|c| !c.is_control()) {
            Some(c) => Ok(format!("\\unicode{{{:X}}}", c as u32)),
            None => self.error(col, format!("\\char needs a character code, not \"{code}\"")),
        }
    }

    /// `\braket{a}{b}` or `\braket{a|b}`.
    fn angle_bracket(&mut self) -> R<String> {
        let x = self.raw_text()?;
        Ok(match self.optional_braced()? {
            // physics: \braket{a}{b}
            Some(y) => format!("\\left\\langle {x} \\middle| {y} \\right\\rangle"),
            // braket: \braket{a|b}
            None => format!("\\left\\langle {} \\right\\rangle", x.replacen('|', " \\middle| ", 2)),
        })
    }

    /// `\eval` at a bar, or the bar after a group.
    fn evaluated(&mut self) -> R<String> {
        self.skip_spaces();
        Ok(match self.current() {
            Some(o @ ('(' | '[')) => {
                let close = if o == '(' { ')' } else { ']' };
                let x = self.raw_delimited(o, close);
                self.skip_spaces();
                if self.current() == Some('|') {
                    self.pos += 1;
                }
                format!("\\left{o} {x} \\right|")
            }
            _ => format!("\\left. {} \\right|", self.raw_text()?),
        })
    }

    /// A physics vector: bold, a unit vector, or an arrow. A star switches bold to `\boldsymbol`.
    fn physics_vector(&mut self, name: &str) -> R<String> {
        let starred = self.take_star();
        let bold = if starred { "\\boldsymbol" } else { "\\mathbf" };
        let body = self.raw_text()?;
        Ok(match name {
            "vb" | "vectorbold" => format!("{bold}{{{body}}}"),
            "vu" | "vectorunit" => format!("\\hat{{{bold}{{{body}}}}}"),
            // \va* consumes the star and still draws \vec, as the package does
            _ => format!("\\vec{{{body}}}"),
        })
    }

    /// `\tensor[pre]{base}{indices}` and `\indices{indices}`. A star is accepted and ignored.
    fn tensor_indices(&mut self, name: &str) -> R<String> {
        self.take_star();
        let pre = self.raw_optional();
        let base = if name == "tensor" { self.raw_text()? } else { String::new() };
        let indices = self.raw_text()?;
        Ok(format!("{}{{{base}}}{}", pre.map(|p| staggered_indices(&p, true)).unwrap_or_default(), staggered_indices(&indices, false)))
    }

    fn take_star(&mut self) -> bool {
        let starred = self.current() == Some('*');
        if starred {
            self.pos += 1;
        }
        starred
    }
}

/// mathtools (its names since 2022) and colonequals: a colon on the axis beside another relation.
/// `\coloneq`, `\coloneqq`, `\eqcolon` and `\eqqcolon` are single symbols and are not in this table.
fn colon_relation(name: &str) -> Option<&'static str> {
    Some(match name {
        "Coloneq" | "coloncolonequals" => "\\Coloneqq ",
        "Eqcolon" | "Eqqcolon" | "equalscoloncolon" => "\\mathrel{\\mathord{=}\\mkern-1.2mu\\mathord{\\dblcolon}}",
        "colondash" | "colonminus" => "\\mathrel{\\mathord{\\vcentcolon}\\mkern-1.2mu\\mathord{-}}",
        "Colondash" | "coloncolonminus" => "\\mathrel{\\mathord{\\dblcolon}\\mkern-1.2mu\\mathord{-}}",
        "dashcolon" | "minuscolon" => "\\mathrel{\\mathord{-}\\mkern-1.2mu\\mathord{\\vcentcolon}}",
        "Dashcolon" | "minuscoloncolon" => "\\mathrel{\\mathord{-}\\mkern-1.2mu\\mathord{\\dblcolon}}",
        "colonapprox" => "\\mathrel{\\mathord{\\vcentcolon}\\mkern-1.2mu\\mathord{\\approx}}",
        "Colonapprox" | "coloncolonapprox" => "\\mathrel{\\mathord{\\dblcolon}\\mkern-1.2mu\\mathord{\\approx}}",
        "colonsim" => "\\mathrel{\\mathord{\\vcentcolon}\\mkern-1.2mu\\mathord{\\sim}}",
        "Colonsim" | "coloncolonsim" => "\\mathrel{\\mathord{\\dblcolon}\\mkern-1.2mu\\mathord{\\sim}}",
        "approxcolon" => "\\mathrel{\\mathord{\\approx}\\mkern-1.2mu\\mathord{\\vcentcolon}}",
        "Approxcolon" | "approxcoloncolon" => "\\mathrel{\\mathord{\\approx}\\mkern-1.2mu\\mathord{\\dblcolon}}",
        "simcolon" => "\\mathrel{\\mathord{\\sim}\\mkern-1.2mu\\mathord{\\vcentcolon}}",
        "Simcolon" | "simcoloncolon" => "\\mathrel{\\mathord{\\sim}\\mkern-1.2mu\\mathord{\\dblcolon}}",
        _ => return None,
    })
}

/// amsmath: the limits of category theory and of sets.
fn limit_operator(name: &str) -> Option<&'static str> {
    Some(match name {
        "injlim" => "\\operatorname*{inj\\,lim}",
        "projlim" => "\\operatorname*{proj\\,lim}",
        "varinjlim" => "\\mathop{\\underrightarrow{\\mathrm{lim}}}",
        "varprojlim" => "\\mathop{\\underleftarrow{\\mathrm{lim}}}",
        "varliminf" => "\\mathop{\\underline{\\mathrm{lim}}}",
        "varlimsup" => "\\mathop{\\overline{\\mathrm{lim}}}",
        _ => return None,
    })
}
