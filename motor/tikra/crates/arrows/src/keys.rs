//! Arrow keys and the values one tip is drawn with.

use std::collections::HashMap;

use crate::color::Color;
use crate::PT;

use super::spec::{strip_group, Kind};

/// Evaluates the values of arrow keys: lengths (in cm), numbers and colors.
pub trait Eval {
    fn dim(&mut self, text: &str) -> Option<f64>;
    fn num(&mut self, text: &str) -> Option<f64>;
    fn paint(&mut self, text: &str) -> Option<Color>;
}

/// Plain values only (`2pt`, `.5`, color names): for callers without an engine.
pub struct Plain;

impl Eval for Plain {
    fn dim(&mut self, _: &str) -> Option<f64> {
        None
    }
    fn num(&mut self, _: &str) -> Option<f64> {
        None
    }
    fn paint(&mut self, text: &str) -> Option<Color> {
        crate::color::parse(text, &HashMap::new())
    }
}

/// `3pt`, `+2mm`, `-.5cm` in points, without an expression evaluator.
fn plain_dim(s: &str) -> Option<f64> {
    let s = s.trim().trim_start_matches('+');
    let units = [("pt", 1.0), ("mm", 72.27 / 25.4), ("cm", 72.27 / 2.54), ("in", 72.27), ("bp", 72.27 / 72.0), ("sp", 1.0 / 65536.0)];
    for (u, k) in units {
        if let Some(n) = s.strip_suffix(u) {
            return n.trim().parse::<f64>().ok().map(|v| v * k);
        }
    }
    None
}

fn plain_num(s: &str) -> Option<f64> {
    s.trim().trim_start_matches('+').parse().ok()
}

/// `dimension factor factor` (`+3pt 4.5 .8`) of the length keys.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Dep(pub(crate) f64, pub(crate) f64, pub(crate) f64);

/// An arrow key, evaluated (`/pgf/arrow keys/…`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Opt {
    Length(Dep),
    Width(Dep),
    /// `width'`: a part of the length
    WidthOfLength(Dep),
    Inset(Dep),
    InsetOfLength(Dep),
    LineWidth(Dep),
    LineWidthOfLength(Dep),
    /// `angle=A:…`: the length along the sides, and the width it gives (set late)
    Angle(f64, Dep),
    AngleWidth(f64),
    /// `angle'=A`: the width for the length
    AngleOfLength(f64),
    CapAngle(f64),
    Scale(f64),
    ScaleLength(f64),
    ScaleWidth(f64),
    Slant(f64),
    Sep(Dep),
    Swap,
    Reversed,
    Harpoon(bool),
    Color(Color),
    Open(bool),
    FillColor(Color),
    RoundCap(bool),
    RoundJoin(bool),
    Arc(f64),
    N(u32),
    Flex(Flex),
}

impl Opt {
    /// pgf runs these after all others (`\pgfarrowsaddtolateoptions`): they read the length.
    fn late(&self) -> bool {
        matches!(
            self,
            Opt::Width(_)
                | Opt::WidthOfLength(_)
                | Opt::Inset(_)
                | Opt::InsetOfLength(_)
                | Opt::LineWidth(_)
                | Opt::LineWidthOfLength(_)
                | Opt::AngleWidth(_)
                | Opt::AngleOfLength(_)
        )
    }
}

/// How a tip sits on a curved end (the bending library, which the manual loads: its
/// `arrows=[flex]` is the default here).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Flex {
    /// along the tangent at the end of the path
    Quick,
    /// turned so that its visual ends lie on the path, with this rigidity
    Flex(f64),
    /// the same with its tip and back ends
    FlexPrime(f64),
    /// bent along the path: drawn flexed here
    Bend,
}

/// The values of the arrow keys for one tip, in pt.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Keys {
    pub(crate) length: f64,
    pub(crate) width: f64,
    pub(crate) inset: f64,
    pub(crate) line_width: f64,
    pub(crate) sep: f64,
    pub(crate) scale_length: f64,
    pub(crate) scale_width: f64,
    pub(crate) slant: f64,
    pub(crate) swap: bool,
    pub(crate) reversed: bool,
    pub(crate) harpoon: bool,
    pub(crate) open: bool,
    pub(crate) round_cap: bool,
    pub(crate) round_join: bool,
    pub(crate) color: Option<Color>,
    pub(crate) fill: Option<Color>,
    pub(crate) arc: f64,
    pub(crate) n: u32,
    pub(crate) flex: Flex,
}

impl Default for Keys {
    fn default() -> Self {
        Keys {
            length: 0.0,
            width: 0.0,
            inset: 0.0,
            line_width: 0.0,
            sep: 0.0,
            scale_length: 1.0,
            scale_width: 1.0,
            slant: 0.0,
            swap: false,
            reversed: false,
            harpoon: false,
            open: false,
            round_cap: false,
            round_join: false,
            color: None,
            fill: None,
            arc: 180.0,
            n: 4,
            flex: Flex::Flex(1.0),
        }
    }
}

/// The path's line widths (pt): pgf's `\pgflinewidth` and, for `double`, `\pgfinnerlinewidth`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Widths {
    pub line: f64,
    pub inner: f64,
}

impl Widths {
    /// `\pgfarrowslinewidthdependent`: a length plus a factor of the line width (of the visible
    /// line, between the inner and the outer one, for double lines).
    pub(crate) fn dep(&self, d: Dep) -> f64 {
        if self.inner > 0.0 {
            d.0 + d.1 * (self.line * (1.0 - d.2 / 2.0) - d.2 / 2.0 * self.inner)
        } else {
            d.0 + d.1 * self.line
        }
    }
}

impl Keys {
    fn apply(&mut self, o: &Opt, lw: Widths, late: &mut Vec<Opt>) {
        let of_length = |k: &Keys, d: Dep| d.0 + d.1 * k.length + d.2 * lw.line;
        match *o {
            Opt::Length(d) => self.length = lw.dep(d),
            Opt::Width(d) => self.width = lw.dep(d),
            Opt::WidthOfLength(d) => self.width = of_length(self, d),
            Opt::Inset(d) => self.inset = lw.dep(d),
            Opt::InsetOfLength(d) => self.inset = of_length(self, d),
            Opt::LineWidth(d) => self.line_width = lw.dep(d),
            Opt::LineWidthOfLength(d) => self.line_width = of_length(self, d),
            Opt::Angle(a, d) => {
                let x = lw.dep(d);
                let (s, c) = (a / 2.0).to_radians().sin_cos();
                self.length = x * c;
                late.push(Opt::AngleWidth(2.0 * x * s));
            }
            Opt::AngleWidth(w) => self.width = w,
            Opt::AngleOfLength(a) => self.width = 2.0 * (a / 2.0).to_radians().tan() * self.length,
            Opt::CapAngle(a) => self.length = 0.5 / (a / 2.0).to_radians().tan() * lw.line,
            Opt::Scale(s) => {
                self.scale_length = s;
                self.scale_width = s;
            }
            Opt::ScaleLength(s) => self.scale_length = s,
            Opt::ScaleWidth(s) => self.scale_width = s,
            Opt::Slant(s) => self.slant = s,
            Opt::Sep(d) => self.sep = lw.dep(d),
            Opt::Swap => self.swap = !self.swap,
            Opt::Reversed => self.reversed = !self.reversed,
            Opt::Harpoon(h) => self.harpoon = h,
            Opt::Color(c) => self.color = Some(c),
            Opt::Open(o) => self.open = o,
            Opt::FillColor(c) => {
                self.open = false;
                self.fill = Some(c);
            }
            Opt::RoundCap(r) => self.round_cap = r,
            Opt::RoundJoin(r) => self.round_join = r,
            Opt::Arc(a) => self.arc = a,
            Opt::N(n) => self.n = n.max(1),
            Opt::Flex(f) => self.flex = f,
        }
    }

    /// The keys of a tip, as `\pgfarrows@getid` runs them: the tip's defaults, the given options,
    /// the late options, then the scaling.
    pub(crate) fn of(kind: Kind, opts: &[Opt], lw: Widths) -> Keys {
        let mut k = Keys::default();
        let mut late = Vec::new();
        for o in defaults(kind).iter().chain(opts) {
            if o.late() {
                late.push(o.clone());
            } else {
                k.apply(o, lw, &mut late);
            }
        }
        let mut more = Vec::new();
        for o in &late {
            k.apply(o, lw, &mut more);
        }
        k.length *= k.scale_length;
        k.inset *= k.scale_length;
        k.width *= k.scale_width;
        k
    }
}

/// `defaults=` of each tip of arrows.meta.
fn defaults(kind: Kind) -> Vec<Opt> {
    use Opt::*;
    let lw = LineWidth(Dep(0.0, 1.0, 1.0));
    let round = [RoundCap(true), RoundJoin(true)];
    match kind {
        Kind::StraightBarb | Kind::ArcBarb => vec![Length(Dep(1.5, 2.0, 0.0)), WidthOfLength(Dep(0.0, 2.0, 0.0)), lw],
        Kind::Hooks => vec![Length(Dep(0.75, 1.25, 0.0)), WidthOfLength(Dep(0.0, 4.0, -1.0)), lw],
        Kind::TeeBarb => vec![Length(Dep(1.5, 2.0, 0.0)), Width(Dep(3.0, 4.0, 0.0)), InsetOfLength(Dep(0.0, 0.5, 0.0)), lw],
        Kind::ClassicalRightarrow => [vec![Length(Dep(1.05, 1.925, 0.0)), WidthOfLength(Dep(0.0, 1.9237, 0.0)), LineWidth(Dep(0.0, 0.8, 1.0))], round.to_vec()].concat(),
        Kind::CmRightarrow => [vec![Length(Dep(1.6, 2.2, 0.0)), WidthOfLength(Dep(0.0, 2.096774, 0.0)), lw], round.to_vec()].concat(),
        Kind::Latex => vec![Length(Dep(3.0, 4.5, 0.8)), WidthOfLength(Dep(0.0, 0.75, 0.0)), lw],
        Kind::Stealth => vec![Length(Dep(3.0, 4.5, 0.8)), WidthOfLength(Dep(0.0, 0.75, 0.0)), InsetOfLength(Dep(0.0, 0.325, 0.0)), lw],
        Kind::Kite => vec![Length(Dep(3.6, 5.4, 0.0)), WidthOfLength(Dep(0.0, 0.5, 0.0)), InsetOfLength(Dep(0.0, 0.25, 0.0)), lw],
        Kind::Square => vec![Length(Dep(2.12132, 2.828427, 0.0)), WidthOfLength(Dep(0.0, 1.0, 0.0)), lw],
        Kind::Circle => vec![Length(Dep(2.39365, 3.191538, 0.0)), WidthOfLength(Dep(0.0, 1.0, 0.0)), lw],
        Kind::RoundCap | Kind::ButtCap | Kind::TriangleCap => vec![Length(Dep(0.0, 0.5, 0.0))],
        Kind::FastTriangle | Kind::FastRound => vec![Length(Dep(0.0, 0.5, 0.0)), Inset(Dep(0.0, 0.5, 0.0))],
        Kind::Rays => vec![Length(Dep(3.0, 4.0, 0.0)), WidthOfLength(Dep(0.0, 1.0, 0.0)), lw],
        // 0.3cm by 0.15cm whatever the line width, open
        Kind::JackTap => vec![Length(Dep(8.53583, 0.0, 0.0)), Width(Dep(4.26791, 0.0, 0.0)), lw, Open(true)],
        Kind::Underscore => vec![Sep(Dep(0.88, 0.3, 1.0))],
        Kind::Implies | Kind::Old(_) => Vec::new(),
    }
}

/// Splits at commas outside braces and brackets.
fn split_list(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut level = 0;
    let mut cur = String::new();
    for ch in s.chars() {
        match ch {
            '{' | '[' | '(' => level += 1,
            '}' | ']' | ')' => level -= 1,
            ',' if level == 0 => {
                out.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(ch);
    }
    out.push(cur);
    out.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

/// Reads arrow keys; unknown ones go to `warn`.
pub(crate) fn parse_opts(ev: &mut dyn Eval, text: &str, warn: &mut Vec<String>) -> Vec<Opt> {
    let mut out = Vec::new();
    let dim = |ev: &mut dyn Eval, s: &str| -> f64 { plain_dim(s).or_else(|| ev.dim(s).map(|v| v / PT)).unwrap_or(0.0) };
    let num = |ev: &mut dyn Eval, s: &str| -> f64 { plain_num(s).or_else(|| ev.num(s)).unwrap_or(0.0) };
    for item in split_list(text) {
        let (key, value) = match item.split_once('=') {
            Some((k, v)) => (k.trim().to_string(), Some(strip_group(v.trim()).to_string())),
            None => (item.clone(), None),
        };
        let v = value.clone().unwrap_or_default();
        // `\pgfarrowsthreeparameters`: a dimension, then two factors
        let three = |ev: &mut dyn Eval, s: &str| -> Dep {
            let mut parts = s.split_whitespace();
            let d = parts.next().map(|p| dim(ev, p)).unwrap_or(0.0);
            let a = parts.next().map(|p| num(ev, p)).unwrap_or(0.0);
            let b = parts.next().map(|p| num(ev, p)).unwrap_or(0.0);
            Dep(d, a, b)
        };
        let o = match key.as_str() {
            "length" => Opt::Length(three(ev, &v)),
            "width" => Opt::Width(three(ev, &v)),
            "width'" => Opt::WidthOfLength(three(ev, &v)),
            "inset" => Opt::Inset(three(ev, &v)),
            "inset'" => Opt::InsetOfLength(three(ev, &v)),
            "line width" => Opt::LineWidth(three(ev, &v)),
            "line width'" => Opt::LineWidthOfLength(three(ev, &v)),
            "sep" => Opt::Sep(if value.is_some() { three(ev, &v) } else { Dep(0.88, 0.3, 1.0) }),
            "angle" => {
                let (a, rest) = v.split_once(':').unwrap_or((&v, ""));
                Opt::Angle(num(ev, a), three(ev, rest))
            }
            "angle'" => Opt::AngleOfLength(num(ev, &v)),
            "cap angle" => Opt::CapAngle(num(ev, &v)),
            "scale" => Opt::Scale(num(ev, &v)),
            "scale length" => Opt::ScaleLength(num(ev, &v)),
            "scale width" => Opt::ScaleWidth(num(ev, &v)),
            "slant" => Opt::Slant(num(ev, &v)),
            "swap" => Opt::Swap,
            "reversed" => Opt::Reversed,
            "harpoon" => Opt::Harpoon(v != "false"),
            "left" => Opt::Harpoon(true),
            "right" => {
                out.push(Opt::Harpoon(true));
                Opt::Swap
            }
            "open" => Opt::Open(true),
            "fill" => match v.as_str() {
                "none" => Opt::Open(true),
                "" => Opt::Open(false),
                c => match ev.paint(c) {
                    Some(c) => Opt::FillColor(c),
                    None => {
                        warn.push(format!("arrow tip fill '{c}' is not a color: ignored"));
                        continue;
                    }
                },
            },
            "color" => match ev.paint(&v) {
                Some(c) => Opt::Color(c),
                None => {
                    warn.push(format!("arrow tip color '{v}' is not a color: ignored"));
                    continue;
                }
            },
            "line cap" => Opt::RoundCap(v == "round"),
            "line join" => Opt::RoundJoin(v == "round"),
            "round" => {
                out.push(Opt::RoundCap(true));
                Opt::RoundJoin(true)
            }
            "sharp" => {
                out.push(Opt::RoundCap(false));
                Opt::RoundJoin(false)
            }
            "arc" => Opt::Arc(num(ev, &v)),
            "n" => Opt::N(num(ev, &v).max(0.0) as u32),
            // the bending library's modes
            "quick" => Opt::Flex(Flex::Quick),
            "flex" => Opt::Flex(Flex::Flex(if value.is_some() { num(ev, &v) } else { 1.0 })),
            "flex'" => Opt::Flex(Flex::FlexPrime(if value.is_some() { num(ev, &v) } else { 1.0 })),
            "bend" => Opt::Flex(Flex::Bend),
            _ => match value.is_none().then(|| ev.paint(&key)).flatten() {
                Some(c) => Opt::Color(c),
                None => {
                    warn.push(format!("arrow key '{key}' not supported: ignored"));
                    continue;
                }
            },
        };
        out.push(o);
    }
    out
}
