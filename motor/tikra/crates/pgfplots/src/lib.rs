//! pgfplots on the engine, as `pgfplots.code.tex` works: `\begin{axis}` opens a survey phase in
//! which `\addplot` computes and stores its coordinates and TikZ statements are put aside; at
//! `\end{axis}` the limits, the scaling and the ticks are computed, then the axis, the plots (in
//! the order they came, with the TikZ statements among them), the descriptions and the legend
//! are drawn ([`draw`]).

pub mod attime;
pub mod boxplot;
pub mod colorbar;
pub mod colormap;
pub mod contour;
pub mod cube;
pub mod dateplot;
pub mod draw;
pub mod fillbetween;
pub use tikra_engine::fpu;
pub mod graphics;
pub mod group;
pub mod keys;
pub mod legend;
pub mod marks;
pub mod mesh;
pub mod patch;
pub mod plot;
pub mod polar;
pub mod scaling;
pub mod smith;
pub mod symbolic;
pub mod table;
pub mod ternary;
pub mod ticks;
pub mod visual;

use tikra_engine::tex::{self, Cat, Token};
use tikra_engine::Error;

pub use draw::Frame;
pub use plot::{Coord, Plot};

use tikra_engine::engine::{tikra_tokens, Engine};

/// What the survey phase of an axis collected.
#[derive(Debug, Clone, Default)]
pub struct Axis {
    /// `axis`, `semilogxaxis`, `semilogyaxis`, `loglogaxis`.
    pub kind: String,
    /// Plots and TikZ statements, in order.
    pub entries: Vec<Entry>,
    /// `\addlegendentry` texts with their options.
    pub legend: Vec<LegendEntry>,
    /// The legend images: one per plot that is not forgotten, and `\addlegendimage`s, in order.
    pub images: Vec<Vec<Token>>,
    /// Plots that count (not `forget plot`): the index into the cycle list.
    pub numplots: usize,
    /// Whether a picture was opened for this axis (an `axis` outside `tikzpicture`).
    pub opened_picture: bool,
    /// The axis' options, as given.
    pub options: Vec<Token>,
    /// The environment's group depth, to close it at `\end{axis}`.
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub enum Entry {
    Plot(Box<Plot>),
    /// A TikZ statement (`\draw …;`, `\node …;`), run once the axis is set up.
    Tikz(Vec<Token>),
}

#[derive(Debug, Clone)]
pub struct LegendEntry {
    pub options: Vec<Token>,
    pub text: Vec<Token>,
}

/// Commands a survey keeps for later (TikZ statements inside an axis).
pub fn is_deferred(name: &str) -> bool {
    matches!(name, "draw" | "fill" | "filldraw" | "path" | "node" | "coordinate" | "clip" | "shade" | "shadedraw" | "pattern" | "matrix" | "pic" | "useasboundingbox")
}

/// `\begin{axis}[options]`: the survey starts.
pub fn begin_axis(e: &mut Engine, kind: &str) -> Result<(), Error> {
    let o = e.optional()?.unwrap_or_default();
    begin_axis_with(e, kind, o)
}

/// An axis with these options (`\nextgroupplot` makes them).
pub fn begin_axis_with(e: &mut Engine, kind: &str, o: Vec<Token>) -> Result<(), Error> {
    if fig(e).axis.is_some() {
        e.warn("nested axis environments are not supported (pgfplots refuses them too): the inner one is drawn on its own");
    }
    let opened = e.picture_depth == 0;
    if opened {
        e.begin_picture(None)?;
    }
    e.begin_group();
    dateplot::reset(e);
    let mut axis = Axis { kind: kind.to_string(), opened_picture: opened, depth: e.tex.group_depth(), ..Default::default() };
    let style = match kind {
        "polaraxis" => "enlarge x limits=false,every polar axis",
        "smithchart" => "every smithchart axis",
        "ternaryaxis" => {
            "enlargelimits=false,xmin=0,xmax=1,ymin=0,ymax=1,zmin=0,zmax=1,\
                 footnotesize/.append style={every axis title shift=15pt},tiny/.append style={every axis title shift=15pt},every ternary axis"
        }
        "semilogxaxis" => "every semilogx axis,xmode=log",
        "semilogyaxis" => "every semilogy axis,ymode=log",
        "loglogaxis" => "every loglog axis,xmode=log,ymode=log",
        _ => "every linear axis",
    };
    e.st.key_path = "/pgfplots".into();
    keys::run(e, "every axis/.try")?;
    keys::run(e, style)?;
    let saved = e.st.key_path.clone();
    axis.options = o.clone();
    // pgfplots' axis types add `data cs` after the options (`\pgfplotsdefineaxistype`)
    let o = if kind == "polaraxis" {
        let mut o = o;
        o.extend(toks(",data cs=polar"));
        o
    } else {
        o
    };
    keys::set(e, "/pgfplots/@axis type", if kind == "polaraxis" { "polar" } else { "cart" });
    e.process_keys(o, Some("/pgfplots")).map_err(|err| Error::Figure(format!("axis options: {err}")))?;
    if kind == "smithchart" {
        // the ticks by size, unless given
        let width = keys::length(e, "/pgfplots/width").unwrap_or(207.0 * tikra_engine::PT);
        keys::run(e, smith::size_style(width))?;
        let given = [keys::value(e, "/pgfplots/xtick").is_empty(), keys::value(e, "/pgfplots/ytick").is_empty()];
        if given[0] {
            keys::run(e, "default smithchart xtick")?;
        }
        if given[1] {
            keys::run(e, "default smithchart ytick")?;
        }
        if given[0] && given[1] {
            keys::run(e, "default smithchart xytick")?;
        }
        keys::set(e, "/pgfplots/is smithchart cs", "false");
    }
    if kind == "ternaryaxis" {
        // percentages: whole numbers, never scaled
        if keys::flag(e, "/pgfplots/ternary limits relative") {
            keys::run(e, "scaled ticks=false,ticklabel style={/pgf/number format/precision=0}")?;
        }
        for c in ['x', 'y', 'z'] {
            if keys::value(e, &format!("/pgfplots/{c} dir")) == "reverse" {
                e.warn(format!("ternaryaxis: '{c} dir=reverse' not supported: drawn forward"));
            }
        }
    }
    e.st.key_path = saved;
    axis.numplots = 0;
    if let Some(code) = keys::value_tokens(e, "/pgfplots/execute at begin axis") {
        e.run_tokens(code)?;
    }
    fig(e).axis = Some(Box::new(axis));
    Ok(())
}

/// `\end{axis}`: everything is drawn.
pub fn end_axis(e: &mut Engine) -> Result<(), Error> {
    let Some(axis) = fig(e).axis.take() else { return Ok(()) };
    if let Some(code) = keys::value_tokens(e, "/pgfplots/execute at end axis") {
        e.run_tokens(code)?;
    }
    let r = draw::draw_axis(e, &axis);
    // close the groups the body left open, then the axis' own
    while e.tex.group_depth() > axis.depth {
        e.end_group();
    }
    e.end_group();
    let r = match r {
        Ok(Some(cb)) => colorbar::draw_colorbar(e, &cb),
        Ok(None) => Ok(()),
        Err(err) => Err(err),
    };
    if axis.opened_picture {
        e.end_picture();
    }
    r
}

/// A TikZ statement inside an axis, kept until the axis is drawn: the command and its tokens up
/// to the `;`.
pub fn defer_statement(e: &mut Engine, t: &Token) -> Result<(), Error> {
    let mut out = vec![t.clone()];
    let mut depth = 0usize;
    while let Some(x) = e.tex.get_token() {
        match x {
            Token::Char(_, Cat::Begin) => depth += 1,
            Token::Char(_, Cat::End) => {
                if depth == 0 {
                    e.tex.back(x);
                    break;
                }
                depth -= 1;
            }
            _ => {}
        }
        let end = depth == 0 && x.is_char(';');
        out.push(x);
        if end || out.len() > tex::MAX_TOKENS {
            break;
        }
    }
    if let Some(a) = fig(e).axis.as_mut() {
        a.entries.push(Entry::Tikz(out));
    }
    Ok(())
}

/// `\legend{a,b,{c,d}}`: entries split at top-level commas (or `\\`).
fn split_legend(list: &[Token]) -> Vec<LegendEntry> {
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
    if !tikra_engine::keys::trim(&cur).is_empty() || (!backslashes && !out.is_empty()) {
        out.push(cur);
    }
    out.into_iter()
        .map(|e| {
            let e = tikra_engine::keys::trim(&e).to_vec();
            let (options, text) = split_options(e);
            LegendEntry { options, text: tex::strip_outer_braces(text) }
        })
        .collect()
}

/// `[options]text` → (options, text).
pub fn split_options(v: Vec<Token>) -> (Vec<Token>, Vec<Token>) {
    if v.first().is_some_and(|t| t.is_char('[')) {
        let mut depth = 0usize;
        for (i, t) in v.iter().enumerate() {
            match t {
                Token::Char(_, Cat::Begin) => depth += 1,
                Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
                Token::Char(']', _) if depth == 0 => return (v[1..i].to_vec(), tikra_engine::keys::trim(&v[i + 1..]).to_vec()),
                _ => {}
            }
        }
    }
    (Vec::new(), v)
}

/// Tokens of `text`, read as TeX with `@` a letter.
pub fn toks(text: &str) -> Vec<Token> {
    tikra_tokens(text)
}

use std::collections::HashMap;
use std::rc::Rc;

use tikra_engine::engine::piece::{CommandFn, DecorationFn, EnvironmentFn, NodeAnchorFn, Piece, StatementFilterFn};
use tikra_engine::scene::{Point, Segment};

/// What pgfplots keeps for the whole figure.
#[derive(Clone, Default)]
pub struct Figure {
    pub axis: Option<Box<Axis>>,
    pub frame: Option<Rc<draw::Frame>>,
    pub plot_group: Option<Box<group::Group>>,
    pub labels: Vec<Vec<Token>>,
    pub tables: HashMap<String, (String, Vec<Token>)>,
    pub filter_rows: Option<(table::Table, Vec<usize>)>,
    pub colorbar_parent: Option<Rc<draw::Frame>>,
    pub boxplot: Option<boxplot::Geometry>,
    pub timeline: Option<attime::Timeline>,
}

pub fn fig(e: &mut Engine) -> &mut Figure {
    e.piece_state()
}

pub use tikra_engine::engine::number::{group_at, print_numbers};

fn outside_axis(e: &mut Engine, name: &str) -> Result<bool, Error> {
    if fig(e).axis.is_some() {
        return Ok(false);
    }
    let rest = e.skip_statement()?;
    e.warn(format!("\\{name} outside an axis: '{}' ignored", rest.chars().take(30).collect::<String>().trim()));
    Ok(true)
}

fn addplot_cmd(e: &mut Engine, _: &Token) -> Result<(), Error> {
    if outside_axis(e, "addplot")? {
        return Ok(());
    }
    let r = plot::addplot(e);
    if let Err(err) = r {
        if tikra_engine::engine::is_limit(&err) {
            return Err(err);
        }
        e.warn(format!("\\addplot: {err}; plot ignored"));
        let _ = e.skip_statement();
    }
    Ok(())
}

fn addlegendentry_cmd(e: &mut Engine, _: &Token) -> Result<(), Error> {
    if outside_axis(e, "addlegendentry")? {
        return Ok(());
    }
    let options = e.optional()?.unwrap_or_default();
    let text = e.argument()?;
    if let Some(a) = fig(e).axis.as_mut() {
        a.legend.push(LegendEntry { options, text });
    }
    Ok(())
}

fn addlegendentryexpanded_cmd(e: &mut Engine, _: &Token) -> Result<(), Error> {
    if outside_axis(e, "addlegendentryexpanded")? {
        return Ok(());
    }
    let options = e.optional()?.unwrap_or_default();
    let mut text = e.argument()?;
    text = e.tex.expand_fully(text)?;
    if let Some(a) = fig(e).axis.as_mut() {
        a.legend.push(LegendEntry { options, text });
    }
    Ok(())
}

fn legend_cmd(e: &mut Engine, _: &Token) -> Result<(), Error> {
    if outside_axis(e, "legend")? {
        return Ok(());
    }
    let _ = e.optional()?;
    let list = e.argument()?;
    let entries = split_legend(&list);
    if let Some(a) = fig(e).axis.as_mut() {
        a.legend = entries;
    }
    Ok(())
}

fn addlegendimage_cmd(e: &mut Engine, _: &Token) -> Result<(), Error> {
    if outside_axis(e, "addlegendimage")? {
        return Ok(());
    }
    let o = e.argument()?;
    if let Some(a) = fig(e).axis.as_mut() {
        a.images.push(o);
    }
    Ok(())
}

fn pgfplotsextra_cmd(e: &mut Engine, _: &Token) -> Result<(), Error> {
    if outside_axis(e, "pgfplotsextra")? {
        return Ok(());
    }
    let code = e.argument()?;
    if let Some(a) = fig(e).axis.as_mut() {
        a.entries.push(Entry::Tikz(code));
    }
    Ok(())
}

fn nextgroupplot_cmd(e: &mut Engine, _: &Token) -> Result<(), Error> {
    group::next_group_plot(e)
}

fn closedcycle_cmd(_: &mut Engine, _: &Token) -> Result<(), Error> {
    Ok(())
}

fn pgfplotsinvokeforeach_cmd(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let list = e.argument()?;
    let body = e.argument()?;
    let list = e.tex.expand_fully(list)?;
    let parsed = tex::parse_body(body);
    for item in tikra_engine::engine::foreach_items(&list) {
        let code = tex::substitute(&parsed, &[item]);
        e.run_tokens(code)?;
    }
    Ok(())
}

fn table_read_cmd(e: &mut Engine, _: &Token) -> Result<(), Error> {
    plot::table_read(e)
}

fn table_typeset_cmd(e: &mut Engine, _: &Token) -> Result<(), Error> {
    plot::table_typeset(e)
}

fn colormap_data_cmd(e: &mut Engine, _: &Token) -> Result<(), Error> {
    colormap::to_data_file(e)
}

fn colormap_shading_cmd(e: &mut Engine, _: &Token) -> Result<(), Error> {
    colormap::to_shading_spec(e)
}

fn tikzfillbetween_cmd(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let spec = e.optional()?.unwrap_or_default();
    let style = e.argument()?;
    let plot = Plot { style, ..Default::default() };
    e.begin_group();
    let r = fillbetween::draw(e, &plot, &spec);
    e.end_group();
    r
}

fn softpath_cmd(e: &mut Engine, t: &Token) -> Result<(), Error> {
    let name = match t {
        Token::Cs(n) => n.as_ref(),
        _ => "",
    };
    fillbetween::soft_command(e, name)
}

fn transformcoordinate_cmd(e: &mut Engine, t: &Token) -> Result<(), Error> {
    let name = match t {
        Token::Cs(n) => n.as_ref(),
        _ => "",
    };
    let a = match name.as_bytes()[name.len() - 1] {
        b'x' => 0,
        b'y' => 1,
        _ => 2,
    };
    let arg = e.argument()?;
    let text = e.tex.expand_to_string(arg)?;
    let v = e.number(&text)?;
    let transformed = match fig(e).frame.clone() {
        Some(f) => {
            let (factor, shift) = draw::data_scale_trafo(e, &f, a);
            (f.logical(a, v) - shift) * factor
        }
        None => v,
    };
    e.tex.define_string("pgfmathresult", &tikra_engine::engine::pgf_number(transformed), false);
    Ok(())
}

fn point_axis(e: &mut Engine, t: &Token) -> Result<(), Error> {
    let name = match t {
        Token::Cs(n) => n.as_ref(),
        _ => "",
    };
    let n = if name.ends_with("xyz") { 3 } else { 2 };
    let mut v = [0.0; 3];
    for x in v.iter_mut().take(n) {
        let a = e.tex.read_undelimited()?;
        *x = e.number_of(a)?;
    }
    let Some(f) = fig(e).frame.clone() else { return Err(Error::Figure(format!("\\{name} outside an axis"))) };
    let inverse = e.st.transform.inverse();
    let p = if name.contains("direction") {
        let d = (0..n).fold(Point::default(), |p, a| p.plus(f.e[a].scale(v[a])));
        inverse.apply(d).minus(inverse.apply(Point::default()))
    } else {
        let l = [f.logical(0, v[0]), f.logical(1, v[1]), if n == 3 { f.logical(2, v[2]) } else { f.min[2] }];
        inverse.apply(f.at(l))
    };
    e.set_pgf_xy(p);
    Ok(())
}

const COMMANDS: &[(&str, CommandFn)] = &[
    ("addplot", addplot_cmd),
    ("addlegendentry", addlegendentry_cmd),
    ("addlegendentryexpanded", addlegendentryexpanded_cmd),
    ("legend", legend_cmd),
    ("addlegendimage", addlegendimage_cmd),
    ("pgfplotsextra", pgfplotsextra_cmd),
    ("nextgroupplot", nextgroupplot_cmd),
    ("closedcycle", closedcycle_cmd),
    ("pgfplotsinvokeforeach", pgfplotsinvokeforeach_cmd),
    ("pgfplotstableread", table_read_cmd),
    ("pgfplotscolormaptodatafile", colormap_data_cmd),
    ("pgfplotscolormaptoshadingspec", colormap_shading_cmd),
    ("tikzfillbetween", tikzfillbetween_cmd),
    ("pgfplotstabletypeset", table_typeset_cmd),
    ("tikzgetnamedpath", softpath_cmd),
    ("pgfplotspathaxisoutline", softpath_cmd),
    ("pgfcomputeintersectionsegments", softpath_cmd),
    ("pgfgetintersectionsegmentpath", softpath_cmd),
    ("pgfsetpathandBB", softpath_cmd),
    ("pgfaddpathandBB", softpath_cmd),
    ("pgfpathreplacefirstmoveto", softpath_cmd),
    ("pgfplotstransformcoordinatex", transformcoordinate_cmd),
    ("pgfplotstransformcoordinatey", transformcoordinate_cmd),
    ("pgfplotstransformcoordinatez", transformcoordinate_cmd),
    ("pgfplotspointaxisxy", point_axis),
    ("pgfplotspointaxisxyz", point_axis),
    ("pgfplotspointaxisdirectionxy", point_axis),
    ("pgfplotspointaxisdirectionxyz", point_axis),
];

fn end_env(e: &mut Engine, _: &str) -> Result<(), Error> {
    end_axis(e)
}

fn begin_group_env(e: &mut Engine, _: &str) -> Result<(), Error> {
    group::begin_groupplot(e)
}

fn end_group_env(e: &mut Engine, _: &str) -> Result<(), Error> {
    group::end_groupplot(e)
}

fn defer_inside_axis(e: &mut Engine, name: &str, t: &Token) -> Option<Result<(), Error>> {
    if fig(e).axis.is_some() && is_deferred(name) {
        Some(defer_statement(e, t))
    } else {
        None
    }
}

fn near_ticklabel(e: &mut Engine, name: &str, local: &dyn Fn(&str) -> Option<Point>) -> Option<Point> {
    if !name.starts_with("near ") {
        return None;
    }
    let align = tikra_engine::keys::value(&e.tex, "/pgfplots/near ticklabel align").map(|v| tikra_engine::tex::tokens_to_string(&v)).unwrap_or_default();
    let frame = std::mem::take(&mut fig(e).frame);
    let near = frame.as_ref().and_then(|f| f.near_point(name, &e.st.pending, &align, local));
    fig(e).frame = frame;
    near
}

fn soft_clip(e: &mut Engine, segs: &[Segment]) -> Result<Vec<Segment>, Error> {
    let spec = tikra_engine::keys::value(&e.tex, "/pgf/decoration/soft clip path").map(|v| tikra_engine::tex::tokens_to_string(&v)).unwrap_or_default();
    Ok(match fillbetween::soft_clip_region(e, &spec) {
        Some(r) => {
            let start = segs.iter().find_map(|s| if let Segment::MoveTo(p) = s { Some(*p) } else { None });
            start.map(Segment::MoveTo).into_iter().chain(fillbetween::clip_lines(segs, r)).collect()
        }
        None => {
            e.warn(format!("soft clip path '{}' not understood: path drawn unclipped", spec.trim()));
            segs.to_vec()
        }
    })
}

const ENVIRONMENTS: &[(&str, EnvironmentFn, EnvironmentFn)] = &[
    ("axis", begin_axis, end_env),
    ("semilogxaxis", begin_axis, end_env),
    ("semilogyaxis", begin_axis, end_env),
    ("loglogaxis", begin_axis, end_env),
    ("polaraxis", begin_axis, end_env),
    ("smithchart", begin_axis, end_env),
    ("ternaryaxis", begin_axis, end_env),
    ("groupplot", begin_group_env, end_group_env),
];

const NODE_TEXT: &[(&str, tikra_engine::engine::piece::NodeTextFn)] = &[("pgfplotspointplotattime", attime::run), ("pgfplotspointgetcoordinates", draw::getcoordinates)];

const FILTERS: &[StatementFilterFn] = &[defer_inside_axis];
const ANCHORS: &[NodeAnchorFn] = &[near_ticklabel];
const DECORATIONS: &[(&str, DecorationFn)] = &[("soft clip", soft_clip)];

pub static PIECE: Piece = Piece {
    name: "pgfplots",
    commands: COMMANDS,
    after_tikz: keys::define,
    statement_filters: FILTERS,
    node_anchors: ANCHORS,
    node_text: NODE_TEXT,
    environments: ENVIRONMENTS,
    coordinate_fallbacks: &[draw::pgfplots_cs],
    plain_coordinates: &[draw::pgfplots_plain],
    decorations: DECORATIONS,
    ..Piece::EMPTY
};
