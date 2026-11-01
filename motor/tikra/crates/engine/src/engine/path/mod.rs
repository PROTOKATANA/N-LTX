//! Path statements: `\path[options] <operations> ;`, read the way `tikz.code.tex` reads them, token
//! by token after expansion: coordinates, `--`, `-|`, `|-`, `.. controls ..`, `to`, `edge`,
//! `rectangle`, `circle`, `ellipse`, `arc`, `grid`, `plot`, `sin`, `cos`, `parabola`, `cycle`,
//! `let`, `node`, `coordinate`, `\foreach` and `\pgfextra` inside the path, options and `{…}` groups.

use crate::canvas::Item;
use crate::scene::{Point, Segment};
use crate::tex::{self, Cat, Meaning, Token};
use crate::Error;

use super::coords::{read_coordinate_tokens, Coord};
use super::nodes::{NodeSpec, OnPath};
use super::state::Mode;
use super::{tikra_tokens, Engine};
use crate::describe::{Arrow, Link};

mod connect;
mod geom;
mod plot;
mod repeat;
mod svg;

use connect::*;
pub use geom::*;
pub use plot::*;
use repeat::*;
pub use svg::*;

/// Largest accepted coordinate, in cm (one kilometer): beyond that it is a typo or a bad computation.
pub const MAX_COORDINATE: f64 = 100_000.0;
/// Cap on `grid` lines in each direction.
pub const MAX_GRID_LINES: usize = 10_000;

pub fn valid_point(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite() && p.x.abs() <= MAX_COORDINATE && p.y.abs() <= MAX_COORDINATE
}

pub fn valid_segment(s: &Segment) -> bool {
    match s {
        Segment::MoveTo(p) | Segment::LineTo(p) => valid_point(*p),
        Segment::CurveTo(a, b, p) => valid_point(*a) && valid_point(*b) && valid_point(*p),
        Segment::Close => true,
    }
}

/// A node written between an operation and its target (`-- node {a} (B)`), placed once the segment exists.
struct MidNode {
    options: Vec<Token>,
    name: Option<String>,
    text: Vec<Token>,
    coordinate: bool,
    /// a pic (`to … pic {type} (b)`), placed as a node is
    pic: bool,
}

/// An `edge`: its segments, options, nodes and connections.
type EdgePath = (Vec<Segment>, Vec<Token>, Vec<Item>, Vec<Link>);

/// An elliptical arc in degrees, before the transform that places it on the canvas.
#[derive(Clone, Copy)]
pub struct EllipseArc {
    pub center: Point,
    pub rx: f64,
    pub ry: f64,
    pub start: f64,
    pub end: f64,
}

/// An SVG elliptical arc from `from` to `to`, in the path data's own unit.
#[derive(Clone, Copy)]
pub struct SvgArc {
    pub from: Point,
    pub to: Point,
    pub rx: f64,
    pub ry: f64,
    pub phi: f64,
    pub large: bool,
    pub sweep: bool,
}

/// The two controls and the end point of one cubic.
struct CurveEnd {
    c1: Point,
    c2: Point,
    target: Coord,
}

/// A pending path operation and the point it is applied to.
struct AppliedOp {
    op: Option<String>,
    to_options: Vec<Token>,
    target: Coord,
}

/// An `edge` and where it goes: its options, its target and the nodes on it.
struct EdgeTo {
    options: Vec<Token>,
    target: Coord,
    mids: Vec<MidNode>,
}

/// One `\foreach` inside a path: the variables, the list, the options and the body.
struct ForeachJob {
    names: Vec<Token>,
    list: Vec<Token>,
    options: super::ForeachOptions,
    body: Vec<Token>,
}

/// Whether the operation reader goes on or the path statement is finished.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PathStep {
    Next,
    Done,
}

/// Where a `to` or an `edge` was read, and the operation still waiting for its point.
struct ToEdge<'a> {
    depth: Option<usize>,
    word: &'a str,
    pending_op: &'a mut Option<String>,
    to_options: &'a mut Vec<Token>,
}

/// A `node` or `coordinate` read in the path, and whether an operation is still waiting.
struct PathNode<'a> {
    depth: Option<usize>,
    coordinate: bool,
    pending: &'a Option<String>,
}

/// A `pic` read in the path, and whether an operation is still waiting.
struct PicHere<'a> {
    depth: Option<usize>,
    pending: &'a Option<String>,
}

/// `sin` or `cos`, and where it was read.
struct TrigHere<'a> {
    depth: Option<usize>,
    kind: &'a str,
}

/// `rectangle`, and the operation still waiting for its corner.
struct RectHere<'a> {
    depth: Option<usize>,
    pending: &'a mut Option<String>,
}

struct Builder {
    segs: Vec<Segment>,
    current: Option<Coord>,
    subpath_start: Option<Point>,
    last: Option<OnPath>,
    node_items: Vec<Item>,
    mid_nodes: Vec<MidNode>,
    /// `edge` paths, drawn after the main path: (segments, options, nodes, connections)
    edges: Vec<EdgePath>,
    /// the point a `+` coordinate is relative to (set by `+`, not moved)
    relative_base: Option<Point>,
    /// where a path is before its first point: the origin of its coordinate system
    origin: Point,
    /// the operation's target was `cycle`: close the subpath once it is done
    close_after: bool,
    /// the node just made: a `to` or `edge` right after it starts there (`\tikz@to@use@last@fig@name`)
    to_start: Option<Coord>,
    /// the current point where each `{` of the path opened, for `current point is local`
    groups: Vec<Option<Coord>>,
    /// the lines from node to node, for the figure's description; the node the path last was at,
    /// and whether the last operation made a line (its nodes are its labels)
    links: Vec<Link>,
    from_node: Option<String>,
    fresh_link: bool,
}

impl Builder {
    fn point(&self) -> Point {
        self.current.as_ref().map_or(self.origin, |c| c.p)
    }

    /// The last point and the point the path came to it from (`\tikz@tangent`), for `turn`.
    fn tangent(&self) -> (Point, Point) {
        let last = self.point();
        let n = self.segs.len();
        let end = |s: &Segment| match s {
            Segment::MoveTo(p) | Segment::LineTo(p) | Segment::CurveTo(_, _, p) => Some(*p),
            Segment::Close => None,
        };
        let from = match self.segs.last() {
            Some(Segment::CurveTo(_, c2, _)) => Some(*c2),
            Some(Segment::LineTo(_)) if n >= 2 => end(&self.segs[n - 2]),
            _ => None,
        };
        (last, from.unwrap_or(last))
    }

    fn move_to(&mut self, r: Coord) {
        self.from_node = r.node.clone();
        self.segs.push(Segment::MoveTo(r.p));
        self.subpath_start = Some(r.p);
        self.last = Some(OnPath::Point(r.p));
        self.current = Some(r);
    }
}

/// A path statement (`\draw`, `\fill`, `\path`, `\node`…): `defaults` are the command's options.
pub fn statement(e: &mut Engine, defaults: &str) -> Result<(), Error> {
    e.begin_group();
    // the state before the path's options: decorations and preactions draw with its graphics
    let outer = e.st.clone();
    e.st.mode = Mode::default();
    e.st.pending = crate::scene::Transform::default();
    reset_path_settings(e);
    // a `scope fading` of a path that drew nothing fades the next one
    let carried = std::mem::take(&mut e.carried_fadings);
    e.path_fadings.push(carried.into_iter().map(|(name, fit, t)| (name, e.canvas.items.len(), fit.is_some(), t, fit)).collect());
    e.path_fading_settings.push((e.st.shade.fading_transform, e.st.shade.fit_fading));
    // `transform canvas` among the path's options (or an animated `:rotate`) is for this path only;
    // a clip it makes stays for the rest of the scope
    let (opened, transforms) = (e.scopes.last().map(|s| s.ends), e.canvas_transforms);
    let r = statement_in_group(e, defaults, &outer);
    if let (Some(before), Some(s)) = (opened, e.scopes.last_mut()) {
        let mut own = e.canvas_transforms - transforms;
        while own > 0 && s.ends > before {
            s.ends -= 1;
            own -= 1;
            e.canvas.push(crate::canvas::Item::End);
        }
    }
    e.canvas_transforms = transforms;
    let fadings = e.path_fadings.pop().unwrap_or_default();
    let settings = e.path_fading_settings.pop().unwrap_or((crate::scene::Transform::default(), true));
    e.close_path_fadings(fadings, settings);
    e.end_group();
    r
}

/// `lindenmayer system [options]` and `l-system [options]`: the system's drawing, from the current
/// point.
fn lindenmayer_operation(e: &mut Engine, b: &mut Builder, depth: Option<usize>, w: &str) -> Result<(), Error> {
    let rest = if w == "l" {
        match next(e, depth)? {
            Some(t) if t.is_char('-') => match next(e, depth)? {
                Some(Token::Char(c, Cat::Letter)) => word(e, c),
                _ => String::new(),
            },
            _ => String::new(),
        }
    } else {
        match next(e, depth)? {
            Some(Token::Char(c, Cat::Letter)) => word(e, c),
            _ => String::new(),
        }
    };
    if rest != "system" {
        e.warn(format!("unrecognized path fragment: '{w}'"));
        return skip_rest(e, depth);
    }
    if let Some(t) = peek(e, depth)? {
        if t.is_char('[') {
            let _ = next(e, depth)?;
            let o = e.tex.read_until_char(']', false)?;
            e.process_keys(o, Some("/tikz"))?;
        }
    }
    let segs = e.lindenmayer_path(b.point())?;
    for s in segs {
        match s {
            Segment::MoveTo(p) => {
                b.segs.push(s);
                b.subpath_start = Some(p);
                b.current = Some(Coord { p, node: None });
            }
            Segment::LineTo(p) => {
                let from = b.point();
                b.segs.push(s);
                b.last = Some(OnPath::Segment(from, None, p));
                b.current = Some(Coord { p, node: None });
            }
            _ => b.segs.push(s),
        }
    }
    Ok(())
}

/// `insert path={…}` in options just read: the path operations go on from there.
fn insert_path(e: &mut Engine) {
    if let Meaning::Macro(m) = e.tex.meaning_of("tikra@insertpath") {
        if !m.body.is_empty() {
            let v = tex::substitute(&m.body, &[]);
            e.tex.define_string("tikra@insertpath", "", false);
            e.tex.push_tokens(v);
        }
    }
}

/// What `\path` resets: `decorate`, `preaction`, `postaction` belong to one path.
fn reset_path_settings(e: &mut Engine) {
    e.st.decorate = false;
    e.st.preactions = Default::default();
    e.st.postactions = Default::default();
}

fn statement_in_group(e: &mut Engine, defaults: &str, outer: &super::GState) -> Result<(), Error> {
    e.tikz_keys("every path/.try")?;
    if !defaults.is_empty() {
        e.tikz_keys(defaults)?;
    }
    // options right after the command, possibly several groups
    loop {
        match e.tex.get_x_token()? {
            Some(t) if t.is_space() => continue,
            Some(t) if t.is_char('[') => {
                let o = e.tex.read_until_char(']', false)?;
                e.process_keys(o, Some("/tikz"))?;
            }
            Some(t) => {
                e.tex.back(t);
                break;
            }
            None => break,
        }
    }
    e.st.apply_pending();
    insert_path(e);
    let mut b = Builder {
        segs: Vec::new(),
        current: None,
        subpath_start: None,
        last: None,
        node_items: Vec::new(),
        mid_nodes: Vec::new(),
        edges: Vec::new(),
        relative_base: None,
        origin: e.st.transform.apply(Point::default()),
        close_after: false,
        to_start: None,
        groups: Vec::new(),
        links: Vec::new(),
        from_node: None,
        fresh_link: false,
    };
    let (color, dash, start) = (e.st.stroke_color(), e.st.pen.dash.clone(), e.canvas.items.len());
    operations(e, &mut b, None)?;
    if b.current.is_some() {
        e.last_xy = b.point();
    }
    // a node left between an operation and a target that never came
    place_mid_nodes(e, &mut b, None)?;
    // options in the middle of the path (`[dashed, red]`) are the whole path's: TeX draws its nodes
    // after it, with its final color and dashes where they have their own none
    if (e.st.stroke_color(), &e.st.pen.dash) != (color, &dash) {
        let (to_color, to_dash) = (e.st.stroke_color(), e.st.pen.dash.clone());
        for item in e.canvas.items[start..].iter_mut().chain(b.node_items.iter_mut()) {
            if let crate::canvas::Item::Path { stroke: Some(st), .. } = item {
                if st.color == color {
                    st.color = to_color;
                }
                if st.dash == dash {
                    st.dash = to_dash.clone();
                }
            }
        }
    }
    let mut segs = std::mem::take(&mut b.segs);
    // a move to a node waits for the next point, which tells it where on the border to start
    // (`\tikz@moveto@waiting`): alone (`\draw (a) edge (b);`), it never reaches the path
    if matches!(segs[..], [Segment::MoveTo(_)]) && b.current.as_ref().is_some_and(|c| c.node.is_some()) {
        segs.clear();
    }
    if let Some(name) = e.st.name_path.take() {
        e.named_paths.insert(name, segs.clone());
    }
    let behind = std::mem::take(&mut e.behind_path);
    e.canvas.items.extend(behind);
    // `current path bounding box`, and what a `scope fading` of the path is fitted to
    let mut bounds = crate::canvas::Canvas::default();
    bounds.include_path(&segs);
    e.st.path_box = bounds.min.zip(bounds.max);
    let m = &e.st.mode;
    if !segs.is_empty() && e.facts.mute == 0 && (m.draw || m.fill || m.shade || m.pattern) {
        e.facts.shapes += 1;
    }
    e.note_links(std::mem::take(&mut b.links));
    e.finish_path(segs, outer, "path")?;
    for (segs, options, items, links) in std::mem::take(&mut b.edges) {
        edge_path(e, segs, options, links)?;
        e.canvas.items.extend(items);
    }
    let items = std::mem::take(&mut b.node_items);
    e.canvas.items.extend(items);
    Ok(())
}

/// The next token that is not a space, expanded.
fn next(e: &mut Engine, depth: Option<usize>) -> Result<Option<Token>, Error> {
    loop {
        let t = match depth {
            Some(d) => e.tex.get_x_token_until(d)?,
            None => e.tex.get_x_token()?,
        };
        match t {
            Some(t) if t.is_space() || t.is_cs("par") || t.is_cs("relax") => continue,
            other => return Ok(other),
        }
    }
}

fn peek(e: &mut Engine, depth: Option<usize>) -> Result<Option<Token>, Error> {
    let t = next(e, depth)?;
    if let Some(t) = &t {
        e.tex.back(t.clone());
    }
    Ok(t)
}

/// Reads a word of letters (`rectangle`, `node`), the first one already read.
fn word(e: &mut Engine, first: char) -> String {
    let mut w = String::new();
    w.push(first);
    while let Some(t) = e.tex.get_token() {
        match t {
            Token::Char(c, Cat::Letter) => w.push(c),
            other => {
                e.tex.back(other);
                break;
            }
        }
        if w.len() > 64 {
            break;
        }
    }
    w
}

/// `(…)` read and evaluated (the `(` already read).
fn coordinate(e: &mut Engine) -> Result<Coord, Error> {
    let tokens = read_coordinate_tokens(e)?;
    let text = e.math_string(tokens)?;
    let c = e.coordinate(&text)?;
    e.facts.touched = named_node(e, &text);
    Ok(c)
}

/// The node a coordinate's text names, with or without an anchor: `a`, `a.east`, `a.30`.
fn named_node(e: &Engine, text: &str) -> Option<String> {
    let t = text.trim();
    let name = if e.lookup_node(t).is_some() { t } else { t.rsplit_once('.').map(|(n, _)| n.trim()).filter(|n| e.lookup_node(n).is_some())? };
    Some(e.node_key(name))
}

/// A node that a connection can join: a coordinate is a point the line goes through.
fn joinable(e: &Engine, name: &str) -> bool {
    e.nodes.get(name).is_some_and(|n| n.shape != "coordinate")
}

/// A line just made to `r`: a connection when it goes from a node to a node.
fn note_link(e: &mut Engine, b: &mut Builder, r: &Coord) {
    let touched = e.facts.touched.take();
    let Some(to) = r.node.clone().or(touched).filter(|n| joinable(e, n)) else { return };
    if let Some(from) = b.from_node.clone().filter(|n| joinable(e, n)) {
        b.links.push(Link { from, to: to.clone(), arrow: Arrow::None, labels: Vec::new() });
        b.fresh_link = true;
    }
    b.from_node = Some(to);
}

/// A point that must follow: skips spaces and reads `(…)`, with an optional `+`/`++`.
fn target(e: &mut Engine, b: &mut Builder, depth: Option<usize>, what: &str) -> Result<Coord, Error> {
    e.turn_from = b.tangent();
    e.subpath_start = b.subpath_start.unwrap_or_else(|| b.point());
    e.facts.touched = None;
    match next(e, depth)? {
        Some(t) if t.is_char('(') => coordinate(e),
        Some(t) if t.is_char('+') => relative(e, b, depth),
        // `cycle`: back to where the subpath started, closed once the operation is done
        Some(Token::Char('c', Cat::Letter)) => {
            let w = word(e, 'c');
            if w != "cycle" {
                e.tex.push_tokens(word_tokens(&w));
                return Err(Error::Figure(format!("{what} without a point")));
            }
            b.close_after = true;
            Ok(Coord { p: b.subpath_start.unwrap_or_else(|| b.point()), node: None })
        }
        _ => Err(Error::Figure(format!("{what} without a point"))),
    }
}

/// The point a `+(x,y)` is relative to: the last point not given with `+`.
fn relative_base(b: &Builder) -> Point {
    b.relative_base.unwrap_or_else(|| b.point())
}

/// `+(x,y)` or `++(x,y)` (the first `+` already read).
fn relative(e: &mut Engine, b: &mut Builder, depth: Option<usize>) -> Result<Coord, Error> {
    let double = match next(e, depth)? {
        Some(t) if t.is_char('+') => true,
        Some(t) => {
            e.tex.back(t);
            false
        }
        None => false,
    };
    match next(e, depth)? {
        Some(t) if t.is_char('(') => {}
        _ => return Err(Error::Figure("'+' without a point".into())),
    }
    let r = coordinate(e)?;
    // the offset uses only the linear part of the transformation
    let origin = e.st.transform.apply(Point::default());
    let base = relative_base(b);
    let p = base.plus(r.p.minus(origin));
    if double {
        b.relative_base = None;
    } else {
        b.relative_base = Some(base);
    }
    Ok(Coord { p, node: None })
}

/// The direction a `turn` is measured from, and where this subpath began.
fn remember_turn(e: &mut Engine, b: &Builder) {
    e.turn_from = b.tangent();
    e.subpath_start = b.subpath_start.unwrap_or_else(|| b.point());
}

/// `--` or `-|` after a `-`. Done when the fragment is not one of those.
fn pending_dash(e: &mut Engine, depth: Option<usize>, pending: &mut Option<String>) -> Result<PathStep, Error> {
    match e.tex.get_token() {
        Some(n) if n.is_char('-') => *pending = Some("--".into()),
        Some(n) if n.is_char('|') => *pending = Some("-|".into()),
        other => {
            if let Some(o) = other {
                e.tex.back(o);
            }
            e.warn("unrecognized path fragment: '-'");
            skip_rest(e, depth)?;
            return Ok(PathStep::Done);
        }
    }
    Ok(PathStep::Next)
}

/// `|-` after a `|`. Done when the next character is not `-`.
fn pending_bar(e: &mut Engine, depth: Option<usize>, pending: &mut Option<String>) -> Result<PathStep, Error> {
    match e.tex.get_token() {
        Some(n) if n.is_char('-') => *pending = Some("|-".into()),
        other => {
            if let Some(o) = other {
                e.tex.back(o);
            }
            e.warn("unrecognized path fragment: '|'");
            skip_rest(e, depth)?;
            return Ok(PathStep::Done);
        }
    }
    Ok(PathStep::Next)
}

/// The word already read, and how far the path reader may look.
struct LeadingWord {
    depth: Option<usize>,
    word: String,
}

/// Nodes written before the next word (`.. node {a} controls`). Returns that word.
fn nodes_before_word(e: &mut Engine, b: &mut Builder, leading: LeadingWord) -> Result<String, Error> {
    let LeadingWord { depth, word: mut w } = leading;
    while w == "node" || w == "coordinate" {
        let coordinate = w == "coordinate";
        let (options, name, _, text) = read_node(e, depth, coordinate, b.point())?;
        b.mid_nodes.push(MidNode { options, name, text, coordinate, pic: false });
        w = match next(e, depth)? {
            Some(Token::Char(c, Cat::Letter)) => word(e, c),
            _ => String::new(),
        };
    }
    Ok(w)
}

/// The first control, and where the second one is read.
struct FirstControl {
    depth: Option<usize>,
    point: Point,
}

/// The second control: its point, or an offset from the end when it was relative.
struct SecondControl {
    point: Point,
    offset: Option<Point>,
}

/// The second control of `.. controls (a) and (b) ..`, absolute or relative to the end.
fn second_control(e: &mut Engine, b: &mut Builder, first: FirstControl) -> Result<SecondControl, Error> {
    let FirstControl { depth, point: c1 } = first;
    let mut c2 = c1;
    let mut c2_offset = None;
    if let Some(Token::Char('a', Cat::Letter)) = peek(e, depth)? {
        let _ = next(e, depth)?;
        let w = word(e, 'a');
        if w == "and" {
            if matches!(peek(e, depth)?, Some(t) if t.is_char('+')) {
                let saved = b.relative_base;
                let base = saved.unwrap_or_else(|| b.point());
                c2_offset = Some(target(e, b, depth, "controls")?.p.minus(base));
                b.relative_base = saved;
            } else {
                c2 = target(e, b, depth, "controls")?.p;
            }
        }
    }
    Ok(SecondControl { point: c2, offset: c2_offset })
}

/// The `..` that closes `controls`. A missing dot stops the scan and is put back.
fn closing_dots(e: &mut Engine, depth: Option<usize>) -> Result<(), Error> {
    for _ in 0..2 {
        match next(e, depth)? {
            Some(t) if t.is_char('.') => {}
            Some(t) => {
                e.tex.back(t);
                break;
            }
            None => break,
        }
    }
    Ok(())
}

/// `.. controls (a) and (b) .. (c)`, with the nodes that sit on the curve.
fn controls_curve(e: &mut Engine, b: &mut Builder, depth: Option<usize>) -> Result<PathStep, Error> {
    match e.tex.get_token() {
        Some(n) if n.is_char('.') => {}
        _ => {
            e.warn("unrecognized path fragment: '.'");
            skip_rest(e, depth)?;
            return Ok(PathStep::Done);
        }
    }
    let w = match next(e, depth)? {
        Some(Token::Char(c, Cat::Letter)) => word(e, c),
        _ => String::new(),
    };
    let w = nodes_before_word(e, b, LeadingWord { depth, word: w })?;
    if w != "controls" {
        e.warn("'..' without 'controls'");
        skip_rest(e, depth)?;
        return Ok(PathStep::Done);
    }
    let c1 = target(e, b, depth, "controls")?.p;
    let SecondControl { point: mut c2, offset: c2_offset } = second_control(e, b, FirstControl { depth, point: c1 })?;
    closing_dots(e, depth)?;
    let mids = read_mid_nodes(e, depth, b.point())?;
    b.mid_nodes.extend(mids);
    let r = target(e, b, depth, "controls")?;
    if let Some(o) = c2_offset {
        c2 = r.p.plus(o);
    }
    curve_to(e, b, CurveEnd { c1, c2, target: r });
    place_mid_nodes(e, b, Some(0.5))?;
    Ok(PathStep::Next)
}

/// `[options]` in a path. Inside a `{…}` group a color changes only the text.
fn path_bracket(e: &mut Engine, b: &Builder) -> Result<(), Error> {
    let o = e.tex.read_until_char(']', false)?;
    let before = (e.st.paint.color, e.st.paint.draw_color, e.st.paint.fill_color, e.st.text_paint_color());
    e.process_keys(o, Some("/tikz"))?;
    // in a `{…}` group of the path, a color is only the text's: the path, and the nodes'
    // lines and fills, keep the path's (`{[red] to[V=1] (0,2)}` has its label red)
    if !b.groups.is_empty() && (e.st.paint.color, e.st.paint.draw_color, e.st.paint.fill_color) != (before.0, before.1, before.2) {
        let text = e.st.text_paint_color();
        (e.st.paint.color, e.st.paint.draw_color, e.st.paint.fill_color) = (before.0, before.1, before.2);
        if text != before.3 {
            e.st.paint.text_color = Some(text);
        }
    }
    e.st.apply_pending();
    insert_path(e);
    Ok(())
}

/// `}` of a path group. `current point is local` restores the point where the group opened.
fn end_path_group(e: &mut Engine, b: &mut Builder) {
    let local = e.st.current_point_local;
    e.end_group();
    if let Some(c) = b.groups.pop() {
        if local {
            b.current = c;
        }
    }
}

/// The nodes of a tree's `edge from parent`, kept to be drawn after the child.
fn tree_edge_labels(e: &mut Engine, b: &Builder, depth: Option<usize>) -> Result<(), Error> {
    let o = match peek(e, depth)? {
        Some(t) if t.is_char('[') => {
            let _ = next(e, depth)?;
            e.tex.read_until_char(']', false)?
        }
        _ => Vec::new(),
    };
    let mut labels = Vec::new();
    while let Some(Token::Char(c @ ('n' | 'c'), Cat::Letter)) = peek(e, depth)? {
        let _ = next(e, depth)?;
        let w = word(e, c);
        if w != "node" && w != "coordinate" {
            e.tex.push_tokens(word_tokens(&w));
            break;
        }
        let (options, name, _, text) = read_node(e, depth, w == "coordinate", b.point())?;
        labels.extend(tikra_tokens(&format!("{w}[")));
        labels.extend(options);
        labels.extend(tikra_tokens("]"));
        if let Some(n) = name {
            labels.extend(tikra_tokens(&format!("({n})")));
        }
        if w == "node" {
            labels.push(Token::Char('{', Cat::Begin));
            labels.extend(text);
            labels.push(Token::Char('}', Cat::End));
        }
    }
    e.st.tree.tree_edge = Some((o, labels));
    e.tree_edge_out = e.st.tree.tree_edge.clone();
    Ok(())
}

/// `to[options]` (the point comes later) or `edge[options] … (point)`.
fn read_to_or_edge(e: &mut Engine, b: &mut Builder, at: ToEdge<'_>) -> Result<(), Error> {
    let opts = match peek(e, at.depth)? {
        Some(t) if t.is_char('[') => {
            let _ = next(e, at.depth)?;
            e.tex.read_until_char(']', false)?
        }
        _ => Vec::new(),
    };
    if at.word == "edge" && opts.is_empty() && peek_words(e, at.depth, &["from", "parent"])? {
        tree_edge_labels(e, b, at.depth)?;
    } else if at.word == "edge" {
        let mids = read_mid_nodes(e, at.depth, b.point())?;
        let r = target(e, b, at.depth, "edge")?;
        edge(e, b, EdgeTo { options: opts, target: r, mids })?;
    } else {
        *at.to_options = opts;
        *at.pending_op = Some("to".into());
    }
    Ok(())
}

/// `node` or `coordinate` in the path: placed now, or kept until the segment exists.
fn place_path_node(e: &mut Engine, b: &mut Builder, node: PathNode<'_>) -> Result<(), Error> {
    let (options, name, at, text) = read_node(e, node.depth, node.coordinate, relative_base(b))?;
    if at.is_some() || node.pending.is_none() {
        let point = b.point();
        let place = b.last.unwrap_or(OnPath::Point(point));
        let items = e.node(NodeSpec { options, name, at, text, place, point, coordinate: node.coordinate, default_pos: None })?;
        b.node_items.extend(items);
        if e.last_node_name.is_none() {
            // what follows may refer to the node (an edge, `after node path`): it gets a
            // name, as `\tikz@fig@mustbenamed` gives one
            if let Some(info) = e.last_node.clone() {
                let n = format!("tikra@fig@{}", e.steps);
                e.alias_last_node(&n);
                e.nodes.insert(n.clone(), info);
                e.tex.define_string("tikzlastnode", &n, false);
                e.last_node_name = Some(n);
            }
        }
        b.to_start = e.last_node_name.clone().and_then(|n| e.nodes.get(&n).map(|info| Coord { p: info.center(), node: Some(n) }));
        // `after node path` and the chains' joins continue the path
        let after = std::mem::take(&mut e.after_node_path);
        e.tex.push_tokens(after);
    } else {
        b.mid_nodes.push(MidNode { options, name, text, coordinate: node.coordinate, pic: false });
    }
    Ok(())
}

/// `cycle`: close the subpath, going by the corner first when the operation is `-|` or `|-`.
fn close_cycle(e: &mut Engine, b: &mut Builder, pending: &mut Option<String>) -> Result<(), Error> {
    let Some(p0) = b.subpath_start else { return Ok(()) };
    let Some(op) = pending.take() else { return Ok(()) };
    let start = b.segs.len();
    if op == "-|" || op == "|-" {
        let a = b.point();
        let corner = if op == "-|" { Point::new(p0.x, a.y) } else { Point::new(a.x, p0.y) };
        b.segs.push(Segment::LineTo(corner));
        b.current = Some(Coord { p: corner, node: None });
    }
    let from = b.point();
    b.segs.push(Segment::Close);
    snake(e, b, start)?;
    b.last = Some(OnPath::Segment(from, None, p0));
    b.current = Some(Coord { p: p0, node: None });
    place_mid_nodes(e, b, Some(0.5))?;
    Ok(())
}

/// `rectangle (corner)`, or `rectangle node {…}` whose corner comes after the nodes.
fn rectangle_op(e: &mut Engine, b: &mut Builder, here: RectHere<'_>) -> Result<(), Error> {
    if matches!(peek(e, here.depth)?, Some(Token::Char('n', Cat::Letter))) {
        *here.pending = Some("rectangle".into());
        return Ok(());
    }
    let r = target(e, b, here.depth, "rectangle")?;
    let start = b.segs.len();
    rectangle_to(e, b, r);
    snake(e, b, start)?;
    *here.pending = None;
    place_mid_nodes(e, b, Some(0.5))?;
    Ok(())
}

/// `sin (point)` and `cos (point)`, as `\pgfpathsine` and `\pgfpathcosine`.
fn trig_curve(e: &mut Engine, b: &mut Builder, trig: TrigHere<'_>) -> Result<(), Error> {
    let r = target(e, b, trig.depth, trig.kind)?;
    let a = b.point();
    let d = r.p.minus(a);
    let (c1, c2) = if trig.kind == "sin" {
        (Point::new(0.3260 * d.x, 0.5120 * d.y), Point::new(0.6380 * d.x, d.y))
    } else {
        (Point::new(0.3620 * d.x, 0.0), Point::new(0.6740 * d.x, 0.4880 * d.y))
    };
    curve_to(e, b, CurveEnd { c1: a.plus(c1), c2: a.plus(c2), target: r });
    Ok(())
}

/// `pic` in the path: on the segment that is waiting, or at the current point.
fn place_pic(e: &mut Engine, b: &mut Builder, here: PicHere<'_>) -> Result<(), Error> {
    let (options, name, at, content) = read_node(e, here.depth, false, relative_base(b))?;
    if at.is_none() && here.pending.is_some() {
        b.mid_nodes.push(MidNode { options, name, text: content, coordinate: false, pic: true });
    } else {
        let place = if at.is_none() { Some((b.last.unwrap_or(OnPath::Point(b.point())), None)) } else { None };
        let at = at.unwrap_or_else(|| b.point());
        e.pic(super::pics::PicSpec { options, name, at, content, place })?;
    }
    Ok(())
}

/// `child`: the nodes made so far are drawn, then the children of the last node.
fn path_child(e: &mut Engine, b: &mut Builder) -> Result<(), Error> {
    if b.segs.is_empty() {
        // a path that only makes nodes: they are drawn before the children (the
        // petri library's tokens lie on their place)
        e.canvas.items.append(&mut b.node_items);
    }
    let parent = e.last_node_name.clone();
    e.children(parent)
}

/// `\pgfextra{code}` or `\pgfextra code \endpgfextra`. A basic-layer path replaces this one.
fn pgfextra(e: &mut Engine, b: &mut Builder) -> Result<(), Error> {
    let code = match e.tex.next_nonspace() {
        Some(Token::Char(_, Cat::Begin)) => e.tex.read_group_body()?,
        Some(t) => {
            let mut v = vec![t];
            while let Some(x) = e.tex.get_token() {
                if x.is_cs("endpgfextra") {
                    break;
                }
                v.push(x);
            }
            v
        }
        None => Vec::new(),
    };
    let outer = std::mem::take(&mut e.pgf_path);
    let r = e.run_tokens(code);
    let built = std::mem::replace(&mut e.pgf_path, outer);
    r?;
    if !built.segments.is_empty() {
        b.segs = built.segments;
    }
    Ok(())
}

/// Runs path operations until `;` (or the end of the frame above `depth`).
fn operations(e: &mut Engine, b: &mut Builder, depth: Option<usize>) -> Result<(), Error> {
    // the operation waiting for its target: "--", "-|", "|-", or "to" with its options
    let mut pending_op: Option<String> = None;
    let mut to_options: Vec<Token> = Vec::new();
    loop {
        if std::mem::take(&mut b.close_after) {
            b.segs.push(Segment::Close);
            if let Some(p0) = b.subpath_start {
                b.current = Some(Coord { p: p0, node: None });
            }
        }
        let Some(t) = next(e, depth)? else {
            if depth.is_none() {
                e.warn("path without ';' at the end");
            }
            return Ok(());
        };
        e.steps += 1;
        if e.steps > super::MAX_STEPS {
            return Err(Error::Figure(format!("figure too large: more than {} statements", super::MAX_STEPS)));
        }
        match &t {
            Token::Char(';', _) => return Ok(()),
            Token::Char('(', _) => {
                remember_turn(e, b);
                let r = coordinate(e)?;
                b.relative_base = None;
                b.to_start = None;
                apply_operation(e, b, AppliedOp { op: pending_op.take(), to_options: std::mem::take(&mut to_options), target: r })?;
            }
            Token::Char('+', _) => {
                remember_turn(e, b);
                let r = relative(e, b, depth)?;
                b.to_start = None;
                apply_operation(e, b, AppliedOp { op: pending_op.take(), to_options: std::mem::take(&mut to_options), target: r })?;
            }
            Token::Char('-', _) => {
                if pending_dash(e, depth, &mut pending_op)? == PathStep::Done {
                    return Ok(());
                }
            }
            Token::Char('|', _) => {
                if pending_bar(e, depth, &mut pending_op)? == PathStep::Done {
                    return Ok(());
                }
            }
            Token::Char('.', _) => {
                if controls_curve(e, b, depth)? == PathStep::Done {
                    return Ok(());
                }
            }
            Token::Char('[', _) => path_bracket(e, b)?,
            Token::Char(':', _) => {
                if let Some(o) = e.read_animation()? {
                    e.process_keys(o, Some("/tikz"))?;
                    e.st.apply_pending();
                }
            }
            Token::Char(_, Cat::Begin) => {
                e.begin_group();
                e.st.current_point_local = false;
                b.groups.push(b.current.clone());
            }
            Token::Char(_, Cat::End) => end_path_group(e, b),
            Token::Char(c, Cat::Letter) => {
                let w = word(e, *c);
                match w.as_str() {
                    "to" | "edge" => read_to_or_edge(e, b, ToEdge { depth, word: &w, pending_op: &mut pending_op, to_options: &mut to_options })?,
                    "node" | "coordinate" | "pic" if peek_words(e, depth, &["foreach"])? => node_foreach(e, b, &w)?,
                    // `node also [label=…] (a)`: more options for a node that exists
                    "node" if peek_words(e, depth, &["also"])? => {
                        let (options, name, _, _) = read_node(e, depth, true, b.point())?;
                        let items = e.late_options(name.as_deref().unwrap_or_default(), options)?;
                        b.node_items.extend(items);
                    }
                    "node" | "coordinate" => {
                        place_path_node(e, b, PathNode { depth, coordinate: w == "coordinate", pending: &pending_op })?;
                    }
                    "cycle" => close_cycle(e, b, &mut pending_op)?,
                    "rectangle" => rectangle_op(e, b, RectHere { depth, pending: &mut pending_op })?,
                    "circle" | "ellipse" => circle(e, b, depth)?,
                    "arc" => arc(e, b, depth)?,
                    "grid" => grid(e, b, depth)?,
                    "plot" => plot(e, b, depth, pending_op.take().is_some())?,
                    "sin" | "cos" => trig_curve(e, b, TrigHere { depth, kind: &w })?,
                    "parabola" => parabola(e, b, depth)?,
                    "let" => let_operation(e, depth)?,
                    "in" => {}
                    "pic" => place_pic(e, b, PicHere { depth, pending: &pending_op })?,
                    "child" => path_child(e, b)?,
                    "decorate" => decorate_operation(e, b, depth)?,
                    "lindenmayer" | "l" => lindenmayer_operation(e, b, depth, &w)?,
                    "foreach" => path_foreach(e, b)?,
                    "svg" => svg_operation(e, b, depth)?,
                    other if e.path_operation(other).is_some() => {
                        if let Some(operation) = e.path_operation(other) {
                            operation(e)?;
                        }
                    }
                    other => {
                        e.warn(format!("unrecognized path fragment: '{other}'"));
                        return skip_rest(e, depth);
                    }
                }
            }
            Token::Cs(name) if &**name == "foreach" => path_foreach(e, b)?,
            Token::Cs(name) if &**name == "pgfextra" => pgfextra(e, b)?,
            Token::Cs(name) if matches!(e.tex.meaning(&t), Meaning::Native(_)) && matches!(&**name, "node" | "coordinate") => {
                // `\node` inside a path reads as `node`
                e.tex.push_tokens(tikra_tokens(&format!("{name} ")));
            }
            other => {
                let s = tex::show_token(other);
                e.warn(format!("unrecognized path fragment: '{s}'"));
                return skip_rest(e, depth);
            }
        }
    }
}

/// A word read too far, as the letters it was read as.
fn word_tokens(w: &str) -> Vec<Token> {
    w.chars().map(|c| Token::Char(c, Cat::Letter)).collect()
}

/// Whether the next words are `words` (read if so, left if not).
fn peek_words(e: &mut Engine, depth: Option<usize>, words: &[&str]) -> Result<bool, Error> {
    let mut read: Vec<Token> = Vec::new();
    for w in words {
        let first = match next(e, depth)? {
            Some(t) => t,
            None => {
                e.tex.push_tokens(read);
                return Ok(false);
            }
        };
        let ok = matches!(&first, Token::Char(c, Cat::Letter) if w.starts_with(*c));
        if !ok {
            e.tex.back(first);
            e.tex.push_tokens(read);
            return Ok(false);
        }
        let Token::Char(c, _) = first else { unreachable!() };
        let got = word(e, c);
        // given back as they were read: letters (a word that is not the one may be `at`)
        read.extend(word_tokens(&got));
        read.push(Token::Char(' ', Cat::Space));
        if got != *w {
            e.tex.push_tokens(read);
            return Ok(false);
        }
    }
    Ok(true)
}

/// `decorate[options]{subpath}`: the subpath, built with the options, is replaced by its
/// decoration (`\tikz@lib@decoration`).
fn decorate_operation(e: &mut Engine, b: &mut Builder, depth: Option<usize>) -> Result<(), Error> {
    let options = match peek(e, depth)? {
        Some(t) if t.is_char('[') => {
            let _ = next(e, depth)?;
            e.tex.read_until_char(']', false)?
        }
        _ => Vec::new(),
    };
    let body = match next(e, depth)? {
        Some(Token::Char(_, Cat::Begin)) => e.tex.read_group_body()?,
        other => {
            if let Some(t) = other {
                e.tex.back(t);
            }
            e.warn("a decoration must begin with a brace: decorate ignored");
            return Ok(());
        }
    };
    let start = b.segs.len();
    let existing = b.current.as_ref().map(|c| c.p);
    e.begin_group();
    let r = (|| {
        e.process_keys(options, Some("/tikz"))?;
        e.st.apply_pending();
        let d = e.tex.depth();
        e.tex.push_frame(body);
        let r = operations(e, b, Some(d));
        e.tex.drain_to(d);
        r?;
        let sub: Vec<Segment> = b.segs.split_off(start);
        if sub.is_empty() || !sub.iter().all(valid_segment) {
            b.segs.extend(sub);
            return Ok(());
        }
        if e.counts_for_size() {
            e.canvas.include_path(&sub);
        }
        let decorated = e.decorate_segments(&sub, if start > 0 { existing } else { None })?;
        b.segs.extend(decorated);
        Ok(())
    })();
    e.end_group();
    r
}

/// After an error inside a path: skips to its `;`.
fn skip_rest(e: &mut Engine, depth: Option<usize>) -> Result<(), Error> {
    let mut level = 0usize;
    loop {
        let t = match depth {
            Some(d) => e.tex.get_token_until(d),
            None => e.tex.get_token(),
        };
        let Some(t) = t else { return Ok(()) };
        match t {
            Token::Char(_, Cat::Begin) => level += 1,
            Token::Char(_, Cat::End) => level = level.saturating_sub(1),
            Token::Char(';', _) if level == 0 => return Ok(()),
            _ => {}
        }
    }
}

/// Where `at` put the node, or that the word was not `at`.
enum AtRead {
    Point(Point),
    NotAt,
}

/// `at (x,y)`, `at +(x,y)` and `at ++(x,y)`. The leading `a` is already read.
fn read_node_at(e: &mut Engine, depth: Option<usize>, base: Point) -> Result<AtRead, Error> {
    match e.tex.get_token() {
        Some(Token::Char('t', Cat::Letter)) => match next(e, depth)? {
            Some(t) if t.is_char('(') => Ok(AtRead::Point(coordinate_point(e)?)),
            // `at +(x,y)` and `at ++(x,y)`: from the path's last point, which stays
            Some(t) if t.is_char('+') => {
                let mut t = next(e, depth)?;
                if matches!(&t, Some(t) if t.is_char('+')) {
                    t = next(e, depth)?;
                }
                if !matches!(t, Some(t) if t.is_char('(')) {
                    return Err(Error::Figure("at without a point".into()));
                }
                let r = coordinate_point(e)?;
                Ok(AtRead::Point(base.plus(r.minus(e.st.transform.apply(Point::default())))))
            }
            _ => Err(Error::Figure("at without a point".into())),
        },
        other => {
            if let Some(o) = other {
                e.tex.back(o);
            }
            e.tex.back(Token::Char('a', Cat::Letter));
            Ok(AtRead::NotAt)
        }
    }
}

/// `\node\bgroup text\egroup`: the text up to the matching `\egroup`.
fn read_bgroup_text(e: &mut Engine) -> Vec<Token> {
    let mut text = Vec::new();
    let mut depth = 0usize;
    loop {
        match e.tex.get_token() {
            None => break,
            Some(t) if t.is_cs("egroup") && depth == 0 => break,
            Some(t) => {
                if t.is_cs("bgroup") || matches!(t, Token::Char(_, Cat::Begin)) {
                    depth += 1;
                } else if t.is_cs("egroup") || matches!(t, Token::Char(_, Cat::End)) {
                    depth = depth.saturating_sub(1);
                }
                text.push(t);
            }
        }
    }
    text
}

/// One `[…]` of a node, appended to `options`. True when `node contents` or `pic type` ends the node.
fn append_node_options(e: &mut Engine, options: &mut Vec<Token>) -> Result<bool, Error> {
    if !options.is_empty() {
        options.push(Token::other(','));
    }
    options.extend(e.tex.read_until_char(']', false)?);
    Ok(crate::keys::split_list(options).iter().any(|i| i.key == "node contents" || i.key == "pic type"))
}

/// `node contents=…` (`pic type=…` for a pic) stands for the text, which is then not given.
fn node_contents_text(options: &[Token]) -> Option<Vec<Token>> {
    crate::keys::split_list(options).into_iter().find(|i| i.key == "node contents" || i.key == "pic type").and_then(|i| i.value).map(tex::strip_outer_braces)
}

/// `node[options] (name) at (x,y) {text}`, the word already read; `base` is the point an
/// `at +(x,y)` is relative to.
#[allow(clippy::type_complexity)]
fn read_node(e: &mut Engine, depth: Option<usize>, coordinate: bool, base: Point) -> Result<(Vec<Token>, Option<String>, Option<Point>, Vec<Token>), Error> {
    let mut options = Vec::new();
    let mut name = None;
    let mut at = None;
    loop {
        match next(e, depth)? {
            // options after a coordinate's name are the path's (`\coordinate (root) [grow cyclic] child …`)
            Some(t) if t.is_char('[') && coordinate && name.is_some() => {
                e.tex.back(t);
                break;
            }
            Some(t) if t.is_char('[') => {
                if append_node_options(e, &mut options)? {
                    break;
                }
            }
            // a second point after a coordinate's name is the path's next (`coordinate (a) (1,0)`)
            Some(t) if t.is_char('(') && coordinate && name.is_some() => {
                e.tex.back(t);
                break;
            }
            // a node's name; a second one replaces the first (`node (a) (b) {}` is named b)
            Some(t) if t.is_char('(') => {
                let n = read_coordinate_tokens(e)?;
                name = Some(e.tex.expand_to_string(n)?.trim().to_string());
            }
            Some(t) if t.is_char(':') => {
                if let Some(o) = e.read_animation()? {
                    if !options.is_empty() {
                        options.push(Token::other(','));
                    }
                    options.extend(o);
                }
            }
            Some(Token::Char('a', Cat::Letter)) => match read_node_at(e, depth, base)? {
                AtRead::Point(p) => at = Some(p),
                AtRead::NotAt => break,
            },
            Some(Token::Char(_, Cat::Begin)) if !coordinate => {
                let text = e.tex.read_group_body()?;
                return Ok((options, name, at, text));
            }
            // `\node\bgroup text\egroup` (as `execute at begin cell` writes it)
            Some(t) if t.is_cs("bgroup") && !coordinate => return Ok((options, name, at, read_bgroup_text(e))),
            Some(t) => {
                e.tex.back(t);
                break;
            }
            None => break,
        }
    }
    if let Some(text) = node_contents_text(&options) {
        return Ok((options, name, at, text));
    }
    if !coordinate {
        e.warn("node without text: '{…}' missing");
    }
    Ok((options, name, at, Vec::new()))
}

fn coordinate_point(e: &mut Engine) -> Result<Point, Error> {
    Ok(coordinate(e)?.p)
}

impl Engine {
    /// The connections of a path, kept for the description when the path is drawn, with its tips.
    fn note_links(&mut self, links: Vec<Link>) {
        if links.is_empty() {
            return;
        }
        if self.facts.mute > 0 || !self.st.mode.draw {
            // not drawn: its labels are nodes like any other
            for l in links {
                for i in l.labels {
                    self.facts.nodes[i].on_link = false;
                }
            }
            return;
        }
        let tip = |a: &Option<String>| a.as_ref().is_some_and(|s| !s.trim().is_empty());
        let arrow = match (tip(&self.st.pen.start_arrow), tip(&self.st.pen.end_arrow)) {
            (false, false) => Arrow::None,
            (false, true) => Arrow::Forward,
            (true, false) => Arrow::Backward,
            (true, true) => Arrow::Both,
        };
        self.facts.links.extend(links.into_iter().map(|l| Link { arrow, ..l }));
    }

    /// A radius: with a unit in cm, without one in the x unit (as TikZ scales circle radii).
    fn radius_length(&mut self, text: &str) -> Result<f64, Error> {
        let (v, unit) = crate::expr::eval_length(crate::keys::strip_braces(text.trim()), &self.vars())?;
        Ok(if unit { v } else { v * self.st.x.norm() })
    }
}
