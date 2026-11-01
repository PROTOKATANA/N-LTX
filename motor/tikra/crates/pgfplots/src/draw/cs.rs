use tikra_engine::engine::coords::Coord as TikzCoord;
use tikra_engine::scene::Point;
use tikra_engine::tex::Token;
use tikra_engine::Error;

use crate::Engine;

use super::frame::*;
use super::util::*;

/// `\pgfplotspointgetcoordinates{(point)}` in a node's text: the point's axis coordinates
/// as `/data point/x` and `/data point/y`. Returns where the text goes on.
pub fn getcoordinates(e: &mut Engine, tokens: &[Token], i: usize) -> Result<usize, Error> {
    let mut j = i + 1;
    while j < tokens.len() && tokens[j].is_space() {
        j += 1;
    }
    let (arg, end) = crate::group_at(tokens, j);
    let text = e.tex.expand_to_string(arg)?;
    let inner = text.trim().strip_prefix('(').and_then(|r| r.strip_suffix(')')).unwrap_or(text.trim()).to_string();
    let frame = crate::fig(e).frame.clone();
    let (Some(f), Ok(c)) = (frame, e.coordinate(&inner)) else { return Ok(end) };
    if let Some([x, y]) = f.data_at(c.p) {
        crate::keys::set(e, "/data point/x", &tikra_engine::engine::pgf_number(x));
        crate::keys::set(e, "/data point/y", &tikra_engine::engine::pgf_number(y));
    }
    Ok(end)
}

/// Arguments of an axis coordinate system: positional, or `x=…,y=…`.
pub(in crate::draw) struct CsArgs {
    parts: Vec<String>,
    positional: bool,
}

impl CsArgs {
    fn parse(args: &str) -> Self {
        let parts = tikra_engine::keys::split_top_level(args);
        let positional = parts.iter().all(|p| !p.contains('='));
        Self { parts, positional }
    }

    fn named(&self, k: &str) -> Option<String> {
        self.parts.iter().find_map(|p| p.split_once('=').filter(|(a, _)| a.trim() == k).map(|(_, b)| b.trim().to_string()))
    }

    /// One component: a symbolic name, a date, or a number. Empty stays missing.
    fn component(&self, e: &mut Engine, index: usize, key: &str) -> Result<Option<f64>, Error> {
        let t = if self.positional { self.parts.get(index).cloned() } else { self.named(key) };
        match t {
            // a name of a symbolic axis, or a date
            Some(t) if !t.trim().is_empty() && index < 3 && crate::symbolic::index(e, axis_char(index), &t).is_some() => Ok(crate::symbolic::index(e, axis_char(index), &t)),
            Some(t) if !t.trim().is_empty() => Ok(Some(e.number(tikra_engine::keys::strip_braces(t.trim()))?)),
            _ => Ok(None),
        }
    }
}

pub(in crate::draw) struct BoxQuery<'a> {
    system: &'a str,
    args: &'a str,
}

/// `boxplot cs`: the box's own coordinates, parsed again as plain numbers.
pub(in crate::draw) fn boxplot_cs(e: &mut Engine, f: &Frame, query: &BoxQuery<'_>) -> Result<Point, Error> {
    let parts = tikra_engine::keys::split_top_level(query.args);
    let a = parts.first().map(|p| e.number(p.trim())).transpose()?.unwrap_or(0.0);
    let b = parts.get(1).map(|p| e.number(p.trim())).transpose()?.unwrap_or(0.0);
    match crate::boxplot::cs(e, query.system, a, b) {
        Some([x, y]) => Ok(f.at([x, y, f.min[2]])),
        None => Err(Error::Figure(format!("{} cs outside a box plot", query.system))),
    }
}

pub(in crate::draw) struct TickQuery<'a> {
    system: &'a str,
    parts: &'a [String],
}

/// `ticklabel cs` and `xticklabel cs`: along the line that holds the tick labels.
pub(in crate::draw) fn ticklabel_cs(e: &mut Engine, f: &Frame, query: &TickQuery<'_>) -> Result<Point, Error> {
    let system = query.system;
    let parts = query.parts;
    let a = match system.trim_end_matches('*') {
        "xticklabel" => 0,
        "yticklabel" => 1,
        "zticklabel" => 2,
        _ => f.label_axis.get(),
    };
    let t = parts.first().map(|p| e.number(p)).transpose()?.unwrap_or(0.5);
    let extra = match parts.get(1) {
        Some(p) => e.length(p)?,
        None => 0.0,
    };
    let base = if f.ternary {
        crate::ternary::edge(f, a, t)
    } else if f.three_d {
        // a point of the line that holds the tick labels
        let line = f.label_line[a];
        let l = [0, 1, 2].map(|k| match line[k] {
            b'v' => f.min[k] + t * (f.max[k] - f.min[k]),
            b'1' => f.max[k],
            b'2' => f.zero[k],
            _ => f.min[k],
        });
        f.at(l)
    } else {
        let b = if a == 0 { 1 } else { 0 };
        let side = f.label_side[a];
        let bv = if side > 0 {
            f.max[b]
        } else if side == 0 {
            f.zero[b]
        } else {
            f.min[b]
        };
        let mut l = f.min;
        l[a] = f.min[a] + t * (f.max[a] - f.min[a]);
        l[b] = bv;
        f.at(l)
    };
    let shift = if system.ends_with('*') { extra } else { f.tick_dim.borrow()[a] + extra + f.label_shift[a] };
    Ok(base.plus(f.normal(a).scale(shift)))
}

/// `axis cs`, `rel axis cs`, `axis description cs`, `ticklabel cs`: points of the axis being
/// drawn, for TikZ statements inside it.
pub fn pgfplots_cs(e: &mut Engine, system: &str, args: &str) -> Option<Result<TikzCoord, Error>> {
    let f = crate::fig(e).frame.clone()?;
    let parsed = CsArgs::parse(args);
    let r = (|| -> Result<Point, Error> {
        match system {
            "cartesian" if f.ternary => {
                let x = parsed.component(e, 0, "x")?.unwrap_or(0.0);
                let y = parsed.component(e, 1, "y")?.unwrap_or(0.0);
                Ok(crate::ternary::cartesian(&f, x, y))
            }
            "cartesian" if f.smith => {
                let x = parsed.component(e, 0, "x")?.unwrap_or(0.0);
                let y = parsed.component(e, 1, "y")?.unwrap_or(0.0);
                Ok(crate::smith::canvas(&f, x, y))
            }
            "axis" => {
                let x = parsed.component(e, 0, "x")?.unwrap_or(0.0);
                let y = parsed.component(e, 1, "y")?.unwrap_or(0.0);
                // a ternary axis deduces a missing z
                let z = parsed.component(e, 2, "z")?.unwrap_or(if f.ternary { f64::NAN } else { 0.0 });
                let l = [f.logical(0, x), f.logical(1, y), if f.three_d { f.logical(2, z) } else { f.min[2] }];
                Ok(f.at(l))
            }
            "rel axis" => {
                let x = parsed.component(e, 0, "x")?.unwrap_or(0.0);
                let y = parsed.component(e, 1, "y")?.unwrap_or(0.0);
                let z = parsed.component(e, 2, "z")?.unwrap_or(0.0);
                Ok(f.rel([x, y, z]))
            }
            "axis description" => {
                let x = parsed.component(e, 0, "x")?.unwrap_or(0.0);
                let y = parsed.component(e, 1, "y")?.unwrap_or(0.0);
                Ok(f.desc(x, y))
            }
            "boxplot" | "boxplot box" | "boxplot whisker" => boxplot_cs(e, &f, &BoxQuery { system, args }),
            "axis direction" => {
                let x = parsed.component(e, 0, "x")?.unwrap_or(0.0);
                let y = parsed.component(e, 1, "y")?.unwrap_or(0.0);
                let z = parsed.component(e, 2, "z")?.unwrap_or(0.0);
                let o = f.at(f.min);
                let l = [f.min[0] + x, f.min[1] + y, f.min[2] + z];
                Ok(f.at(l).minus(o))
            }
            "ticklabel" | "ticklabel*" | "xticklabel" | "yticklabel" | "zticklabel" | "xticklabel*" | "yticklabel*" | "zticklabel*" => {
                ticklabel_cs(e, &f, &TickQuery { system, parts: &parsed.parts })
            }
            _ => Err(Error::Figure(String::new())),
        }
    })();
    match r {
        Ok(p) => Some(Ok(TikzCoord { p, node: None })),
        Err(err) if err.to_string() == "figure: " => None,
        Err(err) => Some(Err(err)),
    }
}
/// A plain `(x,y)` in an axis: `axis cs` (compat 1.11 on) or the description system.
pub fn pgfplots_plain(e: &mut Engine, parts: &[String]) -> Option<Result<Point, Error>> {
    let f = crate::fig(e).frame.clone()?;
    match f.cs.get() {
        // plain coordinates of a ternary axis are cartesian
        Cs::Plain if f.ternary => {
            let r = (|| -> Result<Point, Error> {
                let x = e.number(tikra_engine::keys::strip_braces(parts.first().map_or("0", |p| p.trim())))?;
                let y = e.number(tikra_engine::keys::strip_braces(parts.get(1).map_or("0", |p| p.trim())))?;
                Ok(crate::ternary::cartesian(&f, x, y))
            })();
            Some(r)
        }
        // a coordinate with units (`(1cm,0pt)`) stays TikZ's
        Cs::Plain if parts.iter().any(|p| has_unit(p)) => None,
        // before compat 1.11, the coordinates of the data scale transformation: 10^E times the
        // data, less the smallest data value (`\pgfplots@set@optimal@datatrafo@for@`)
        Cs::Plain if !f.polar && !f.smith => {
            let r = (|| -> Result<Point, Error> {
                let mut l = [f.min[0], f.min[1], f.min[2]];
                for (a, la) in l.iter_mut().enumerate().take(if f.three_d { 3 } else { 2 }) {
                    // an empty coordinate (`\pgfkeysvalueof{/pgfplots/xmin}` unset) is 0, as TeX recovers
                    let v = match tikra_engine::keys::strip_braces(parts.get(a).map_or("0", |p| p.trim())) {
                        "" => 0.0,
                        p => e.number(p)?,
                    };
                    let (factor, shift) = data_scale_trafo(e, &f, a);
                    *la = shift + v / factor;
                }
                Ok(f.at(l))
            })();
            Some(r)
        }
        Cs::Plain => None,
        cs => {
            let r = (|| -> Result<Point, Error> {
                let mut v = [0.0; 3];
                for (i, p) in parts.iter().enumerate().take(3) {
                    v[i] = e.number(tikra_engine::keys::strip_braces(p.trim()))?;
                }
                Ok(if cs == Cs::Axis { f.at([f.logical(0, v[0]), f.logical(1, v[1]), if f.three_d { f.logical(2, v[2]) } else { f.min[2] }]) } else { f.desc(v[0], v[1]) })
            })();
            Some(r)
        }
    }
}
