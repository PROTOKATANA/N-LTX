//! `\addplot`, as `pgfplotscoordprocessing.code.tex` reads it:
//! `\addplot[+][options] [plot[behavior]] <input> <trailing path>;` with the inputs `coordinates`,
//! `table`, `expression` (`{…}`, `(x(t),y(t))`), `file`, `fill between`, and `\addplot3`.
//! The survey computes the coordinates; [`super::draw`] draws them.

use tikra_engine::tex::{self, Cat, Token};
use tikra_engine::Error;

use super::keys;
use super::{Engine, Entry};

mod adjust;
mod coords;
mod input;
mod rows;
mod sample;
mod sheet;

pub use adjust::scatter_classes;
use input::survey;
pub use sample::split_commas;
pub use sheet::{table_read, table_typeset};

/// A data point. Unbounded or missing values are NaN.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coord {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub meta: f64,
    /// error bars: x minus, x plus, y minus, y plus (NaN: none)
    pub err: [f64; 4],
    /// The point meta as written (`[label]`, a table cell): an index into the plot's labels.
    pub label: u32,
    /// The sampling variable's value (expressions), and `quiver`'s (u, v, w).
    pub param: [f64; 2],
    pub uvw: [f64; 3],
    /// `coordinate style`: an index into the plot's `coord_styles` (`NO_LABEL`: none).
    pub style: u32,
    /// `visualization depends on`: an index into the plot's `deps` (`NO_LABEL`: none).
    pub deps: u32,
}

/// No label.
pub const NO_LABEL: u32 = u32::MAX;

fn add_label(e: &mut Engine, tokens: Vec<Token>) -> u32 {
    let l = &mut super::fig(e).labels;
    l.push(tokens);
    (l.len() - 1) as u32
}

impl Coord {
    pub fn new(x: f64, y: f64, z: f64) -> Coord {
        Coord { x, y, z, meta: f64::NAN, err: [f64::NAN; 4], label: NO_LABEL, param: [f64::NAN; 2], uvw: [f64::NAN; 3], style: NO_LABEL, deps: NO_LABEL }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Plot {
    /// The full option list it is drawn with.
    pub style: Vec<Token>,
    pub coords: Vec<Coord>,
    pub three_d: bool,
    /// What follows the input up to the `;` (`\closedcycle`, `-- cycle`, `node {…}`).
    pub post: Vec<Token>,
    pub forget: bool,
    /// `update limits=false`: it is drawn, but the axis makes no room for it
    pub keep_limits: bool,
    /// `/tikz/@plot handler` (empty: lines or `smooth`)
    pub handler: String,
    /// The plot's number (`\plotnum`) and its number among plots of the same handler.
    pub index: usize,
    /// `stack plots=x|y`, with `stack dir=minus`
    pub stack: Option<char>,
    pub stack_minus: bool,
    /// For a lattice (`\addplot3` of an expression, `mesh/rows`): (rows, cols).
    pub lattice: Option<(usize, usize)>,
    /// `fill between[of=a and b]`
    pub fill_between: Option<Vec<Token>>,
    /// `bar shift` (or `bar shift auto`) given to the axis or the plot: stacked bars keep it.
    pub bar_shift_given: bool,
    /// `x tick=data`: the first plot's x coordinates are kept.
    pub symbolic: bool,
    /// `mesh` (`'m'`) or `surf`/`patch` (`'s'`) plots, colored by the colormap.
    pub mesh: Option<char>,
    pub patch: bool,
    /// `scatter`: marks colored by their point meta.
    pub scatter: bool,
    /// The plot has numeric point meta (`point meta=y`, `explicit`…).
    pub has_meta: bool,
    /// Where the input's scanlines end (empty lines in tables and coordinates).
    pub scanlines: Vec<usize>,
    /// The point metas as written, for `point meta=explicit symbolic` (see `Coord::label`).
    pub labels: Vec<Vec<Token>>,
    /// `mesh input=image` (`matrix plot`): the points are the centers of the cells.
    pub image: bool,
    /// `late options`: axis options the plot asks for (`matrix plot`: `y dir=reverse`).
    pub late: Vec<Token>,
    /// `boxplot`, `boxplot prepared`: the box's numbers (the points are the outliers).
    pub boxplot: Option<Box<super::boxplot::BoxStats>>,
    /// The styles of single coordinates (`coordinate style/.condition`, `.from`).
    pub coord_styles: Vec<Vec<Token>>,
    /// `visualization depends on={expr \as \macro}`: the macros' values at single coordinates.
    pub deps: Vec<Vec<(String, String)>>,
    /// The formula it was sampled from (`\addplot {x^2}`), for the figure's description.
    pub expression: String,
    /// `\addplot graphics`: the image and where it goes.
    pub graphics: Option<Box<tikra_engine::engine::graphics::Graphics>>,
}

/// Letters following in the input: a word like `coordinates`.
fn word(e: &mut Engine) -> String {
    let mut w = String::new();
    loop {
        match e.tex.get_token() {
            Some(Token::Char(c, Cat::Letter)) => w.push(c),
            Some(t) => {
                e.tex.back(t);
                break;
            }
            None => break,
        }
        if w.len() > 40 {
            break;
        }
    }
    w
}

fn peek(e: &mut Engine) -> Option<Token> {
    let t = e.tex.next_nonspace();
    if let Some(t) = &t {
        e.tex.back(t.clone());
    }
    t
}

/// `\addplot…;`
pub fn addplot(e: &mut Engine) -> Result<(), Error> {
    let mut three_d = false;
    let mut plus = false;
    if let Some(t) = e.tex.get_token() {
        if t.is_char('3') {
            three_d = true;
        } else {
            e.tex.back(t);
        }
    }
    if let Some(t) = e.tex.next_nonspace() {
        if t.is_char('+') {
            plus = true;
        } else {
            e.tex.back(t);
        }
    }
    let user = e.optional()?;
    // `plot[behavior options]`
    let mut behavior = Vec::new();
    if let Some(Token::Char('p', Cat::Letter)) = peek(e) {
        let w = word(e);
        if w == "plot" {
            if let Some(o) = e.optional()? {
                behavior = o;
            }
        } else {
            e.tex.push_tokens(tex::string_tokens(&w));
        }
    }
    let (index, cycle) = {
        let a = super::fig(e).axis.as_ref().expect("in an axis");
        (a.numplots, a.numplots)
    };
    let user_options = user.clone().unwrap_or_default();
    let mut spec = Vec::new();
    if user.is_none() || plus {
        spec.extend(cycle_entry(e, cycle)?);
        if plus && !user_options.is_empty() {
            spec.push(Token::other(','));
        }
    }
    spec.extend(user_options.clone());
    if !behavior.is_empty() {
        spec.push(Token::other(','));
        spec.extend(behavior);
    }
    let mut style = super::toks(&format!("/pgfplots/every axis plot,/pgfplots/every axis plot except legend,/pgfplots/every axis plot no {index}/.try,"));
    style.extend(spec.clone());
    style.extend(super::toks(",/pgfplots/.cd,/pgfplots/every axis plot post"));

    // the survey: the plot's keys, then its input
    e.begin_group();
    let r = survey(e, &style, three_d);
    e.end_group();
    let mut plot = r?;
    // a forgotten plot has `every forget plot` instead of the styles of the plot's number
    let style = if plot.forget {
        let mut forget = super::toks("/pgfplots/every axis plot,/pgfplots/every forget plot,");
        forget.extend(spec.clone());
        forget.extend(super::toks(",/pgfplots/.cd,/pgfplots/every forget plot,/pgfplots/every axis plot post"));
        forget
    } else {
        style
    };
    plot.style = style;
    plot.index = index;
    if !plot.forget {
        // a scatter plot with classes has a legend entry per class: its mark
        let classes = if plot.scatter {
            e.begin_group();
            let r = e.process_keys(style_for_classes(&spec), Some("/pgfplots"));
            let c = scatter_classes(e);
            e.end_group();
            r?;
            c
        } else {
            Vec::new()
        };
        let a = super::fig(e).axis.as_mut().expect("in an axis");
        a.numplots += 1;
        if classes.is_empty() {
            a.images.push(spec);
        } else {
            for (_, s) in classes {
                let mut image = spec.clone();
                image.extend(super::toks(",/tikz/only marks,"));
                image.extend(s);
                a.images.push(image);
            }
        }
    }
    super::fig(e).axis.as_mut().expect("in an axis").entries.push(Entry::Plot(Box::new(plot)));
    Ok(())
}

/// The plot's options, for reading its `scatter/classes`.
fn style_for_classes(spec: &[Token]) -> Vec<Token> {
    let mut s = super::toks("/pgfplots/every axis plot,");
    s.extend(spec.iter().cloned());
    s
}

/// The entry of the cycle list for plot `k` (`\pgfplots@getautoplotspec`).
pub fn cycle_entry(e: &mut Engine, k: usize) -> Result<Vec<Token>, Error> {
    let name = keys::value(e, "/pgfplots/cycle list name");
    let shift = keys::number(e, "/pgfplots/cycle list shift").unwrap_or(0.0) as i64;
    let Some(list) = tikra_engine::keys::value(&e.tex, &format!("/pgfplots/@cycle lists/{name}")) else {
        e.warn(format!("cycle list '{name}' not defined"));
        return Ok(Vec::new());
    };
    let k = (k as i64 + shift).max(0) as usize;
    // `cycle multi list={a\nextlist b}`: one entry of each, the last list fastest
    if list.iter().any(|t| t.is_cs("nextlist")) {
        let lists: Vec<Vec<Vec<Token>>> = list.split(|t| t.is_cs("nextlist")).map(|part| multi_part(e, part)).filter(|l| !l.is_empty()).collect();
        let mut out = Vec::new();
        let mut rest = k;
        let mut chosen = Vec::new();
        if name.starts_with("@multiindex") {
            // `cycle multiindex list`: the same index into every list, the shorter ones padded
            // with nothing (restarted with `cycle multiindex* list`)
            let longest = lists.iter().map(Vec::len).max().unwrap_or(1);
            for l in &lists {
                let i = if name == "@multiindex*@" { k % l.len() } else { k % longest };
                chosen.push(l.get(i).cloned().unwrap_or_default());
            }
            chosen.reverse();
        } else {
            for l in lists.iter().rev() {
                chosen.push(l[rest % l.len()].clone());
                rest /= l.len();
            }
        }
        for entry in chosen.into_iter().rev().filter(|c| !c.is_empty()) {
            if !out.is_empty() {
                out.push(Token::other(','));
            }
            out.extend(entry);
        }
        return Ok(out);
    }
    let entries = split_cycle_list(&list);
    if entries.is_empty() {
        return Ok(Vec::new());
    }
    Ok(entries[k % entries.len()].clone())
}

/// A list of `cycle multi list`: a cycle list's name (`[2 of]mark list`: its first two entries)
/// or the entries themselves.
fn multi_part(e: &mut Engine, part: &[Token]) -> Vec<Vec<Token>> {
    let text = tex::tokens_to_string(tikra_engine::keys::trim(part)).trim().to_string();
    let (limit, name) = match text.strip_prefix('[').and_then(|r| r.split_once(" of]")) {
        Some((n, rest)) => (n.trim().parse::<usize>().ok(), rest.trim().to_string()),
        None => (None, text.clone()),
    };
    if let Some(list) = tikra_engine::keys::value(&e.tex, &format!("/pgfplots/@cycle lists/{name}")) {
        let mut entries = split_cycle_list(&list);
        if let Some(n) = limit {
            entries.truncate(n.max(1));
        }
        return entries;
    }
    split_cycle_list(part)
}

/// A cycle list: entries separated by `\\`, or by commas when there is no `\\`.
pub fn split_cycle_list(list: &[Token]) -> Vec<Vec<Token>> {
    let backslashes = list.iter().any(|t| t.is_cs("\\"));
    let mut out = Vec::new();
    let mut cur = Vec::new();
    let mut depth = 0usize;
    for t in list {
        match t {
            Token::Char(_, Cat::Begin) => depth += 1,
            Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
            _ => {}
        }
        let sep = depth == 0 && if backslashes { t.is_cs("\\") } else { t.is_char(',') };
        if sep {
            out.push(std::mem::take(&mut cur));
            continue;
        }
        cur.push(t.clone());
    }
    out.push(cur);
    out.into_iter().map(|v| tex::strip_outer_braces(tikra_engine::keys::trim(&v).to_vec())).filter(|v| !v.is_empty()).collect()
}
