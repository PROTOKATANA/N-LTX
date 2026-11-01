//! Nodes: text in a shape, placed by an anchor, as `\tikz@fig` and pgf's shapes do.
//!
//! A node's own coordinate system has the text's origin (the left end of its first baseline) at
//! (0, 0). The shape (see [`super::shapes`]) is computed there, from the text boxes of the node's
//! parts, `inner sep`, the minimum sizes and `outer sep`; the node is then moved so that the chosen
//! anchor lands on its position. The transformations of the picture apply to that position only,
//! unless `transform shape`; those of the node's own options apply to the whole node.

use std::collections::HashMap;
use std::rc::Rc;

use crate::canvas::{Item, Stroke};
use crate::keys;
use crate::label;
use crate::scene::{Label, Point, Segment, Transform};
use crate::tex::{self, Token};
use crate::{Error, PT};

use super::shapes::{self, Border, BoxSize, Geometry, Layer, ShapeText, Usage};
use super::Engine;

mod build;
mod draw;
mod labels;
mod sets;
mod text;

pub(crate) use sets::define_sets;

pub fn is_shape(name: &str) -> bool {
    shapes::is_shape(name)
}

/// Node settings that last for one node (set by its options).
#[derive(Debug, Clone, Default)]
pub struct NodeSetup {
    pub at: Option<Point>,
    pub pos: Option<f64>,
    pub sloped: bool,
    pub transform_shape: bool,
    /// `transform shape nonlinear`: the node turned and scaled as the nonlinear transformation
    /// bends the picture where it is
    pub transform_shape_nonlinear: bool,
    pub name: Option<String>,
    /// `label=` and `pin=` values, in order; true for a pin
    pub labels: Vec<(Vec<Token>, bool)>,
    /// `fit`: the width and height of the fitted area, which become the text box
    pub fit: Option<(f64, f64)>,
    /// `rotate fit`: the fitted area is turned by this angle (and so is the node)
    pub fit_rotate: f64,
    /// `circle through`: the point on the circle (canvas)
    pub through: Option<Point>,
    /// `right of=a` and its kin: their direction, the node distance they shifted by (vertical,
    /// horizontal), and the pending transformation right after their shift
    pub old_of: Vec<(Point, (f64, f64), Transform)>,
}

/// A named node: its shape in its own coordinate system, and where that system is on the canvas.
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub shape: String,
    pub geometry: Rc<Geometry>,
    /// node coordinates → canvas
    pub transform: Transform,
}

impl NodeInfo {
    pub fn center(&self) -> Point {
        self.transform.apply(self.geometry.center)
    }

    /// The border point toward canvas point `p` (where a line to the node stops).
    pub fn border_toward(&self, p: Point) -> Point {
        let inv = self.transform.inverse();
        let q = inv.apply(p);
        let b = self.geometry.border_point(q.minus(self.geometry.center));
        self.transform.apply(b)
    }

    /// A named or angle anchor, on the canvas.
    pub fn anchor(&self, name: &str) -> Option<Point> {
        self.anchor_local(name).map(|p| self.transform.apply(p))
    }

    pub fn anchor_local(&self, name: &str) -> Option<Point> {
        self.geometry.anchor(name)
    }
}
/// The sets of nodes (`new set`, `set=`) and the count of anonymous names (`tikz@f@N`).
#[derive(Debug, Clone, Default)]
pub struct NodeSets {
    /// the nodes that joined each set
    pub sets: HashMap<String, Vec<String>>,
    /// anonymous nodes named so far: a node of a set with no name, and `/` or `""` in a graph
    pub anonymous: usize,
}

impl NodeSets {
    /// The name of a node in a set that has none (`\tikz@fig@mustbenamed`).
    pub fn set_member_name(&mut self, listed: &str) -> Option<String> {
        if listed.is_empty() {
            return None;
        }
        self.anonymous += 1;
        Some(format!("tikz@f@{}", self.anonymous))
    }

    /// A node just named joins the sets named in `listed` (comma-separated).
    pub fn join_sets(&mut self, listed: &str, name: &str) {
        for s in listed.split(',').filter(|s| !s.is_empty()) {
            if let Some(members) = self.sets.get_mut(s) {
                members.push(name.to_string());
            }
        }
    }
}
/// A library's shape as the geometry of a node.
fn library_geometry(shape: crate::extension::NodeShape) -> Geometry {
    use crate::extension::{PathSegment, ShapeBorder};
    let border = match shape.border {
        ShapeBorder::Point => Border::Point,
        ShapeBorder::Rectangle(half) => Border::Rectangle(half),
        ShapeBorder::Outline(points) => Border::Outline(points),
    };
    let mut g = Geometry::new(shape.center, border);
    g.anchors = shape.anchors;
    g.parts = vec![("text".into(), shape.text)];
    g.extent = shape.extent;
    for path in shape.paths {
        let segments = path
            .segments
            .into_iter()
            .map(|s| match s {
                PathSegment::MoveTo(p) => Segment::MoveTo(p),
                PathSegment::LineTo(p) => Segment::LineTo(p),
                PathSegment::CurveTo(a, b, c) => Segment::CurveTo(a, b, c),
                PathSegment::Close => Segment::Close,
            })
            .collect();
        g.path(Layer::Background, segments, Usage::Library { paint: path.paint, color: path.color, dash: path.dash, cap: path.cap, opacity: path.opacity });
    }
    g.texts = shape.texts.into_iter().map(|t| ShapeText { tex: t.text, at: t.at, color: None, anchor: Some(t.anchor), font: t.font, turn: t.turn, upright: t.upright }).collect();
    g
}

/// Where the point `a` of a text's box is, from the left end of its baseline (as a node's anchor
/// with no inner sep: `mid` half an ex of its font above the baseline).
fn text_anchor(a: crate::extension::TextAnchor, l: &Label, font: &str) -> Point {
    use crate::extension::TextAnchor::*;
    let (w, h, d) = (l.width(), l.height(), l.depth());
    let x = match a {
        West | NorthWest | SouthWest | BaseWest | MidWest => 0.0,
        East | NorthEast | SouthEast | BaseEast | MidEast => w,
        Center | North | South | Base | Mid => w / 2.0,
    };
    let y = match a {
        North | NorthEast | NorthWest => h,
        South | SouthEast | SouthWest => -d,
        Center | East | West => (h - d) / 2.0,
        Base | BaseEast | BaseWest => 0.0,
        Mid | MidEast | MidWest => 0.5 * label::x_height(false) * 10.0 * label::font_size(font) * PT,
    };
    Point::new(x, y)
}

/// The node's text split at `\nodepart[options]{name}`: the parts in order, `text` first.
pub fn split_parts(tokens: &[Token]) -> Vec<(String, Vec<Token>)> {
    let mut parts = vec![("text".to_string(), Vec::new())];
    let mut current = 0usize;
    let mut depth = 0usize;
    let mut i = 0;
    let skip_spaces = |j: &mut usize| {
        while tokens.get(*j).is_some_and(|t| t.is_space()) {
            *j += 1;
        }
    };
    while i < tokens.len() {
        let t = &tokens[i];
        match t {
            Token::Char(_, tex::Cat::Begin) => depth += 1,
            Token::Char(_, tex::Cat::End) => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth == 0 && t.is_cs("nodepart") {
            let mut j = i + 1;
            skip_spaces(&mut j);
            // the part's options are not used
            if tokens.get(j).is_some_and(|t| t.is_char('[')) {
                while j < tokens.len() && !tokens[j].is_char(']') {
                    j += 1;
                }
                j += 1;
                skip_spaces(&mut j);
            }
            let mut name = Vec::new();
            if let Some(Token::Char(_, tex::Cat::Begin)) = tokens.get(j) {
                let mut d = 0usize;
                while j < tokens.len() {
                    match &tokens[j] {
                        Token::Char(_, tex::Cat::Begin) => d += 1,
                        Token::Char(_, tex::Cat::End) => d -= 1,
                        _ => {}
                    }
                    if d == 0 {
                        break;
                    }
                    if !(d == 1 && matches!(tokens[j], Token::Char(_, tex::Cat::Begin))) {
                        name.push(tokens[j].clone());
                    }
                    j += 1;
                }
                j += 1;
            } else if let Some(t) = tokens.get(j) {
                name.push(t.clone());
                j += 1;
            }
            let name = tex::tokens_to_string(&name).split_whitespace().collect::<Vec<_>>().join(" ");
            // pgf's other names for the boxes of the rectangle split
            let name = match name.as_str() {
                "one" => "text".to_string(),
                "second" => "two".to_string(),
                "third" => "three".to_string(),
                "fourth" => "four".to_string(),
                _ => name,
            };
            // a part named again starts its box anew, as `\setbox` does (`\nodepart{text}text`)
            match parts.iter().position(|(n, _)| *n == name) {
                Some(k) => {
                    parts[k].1.clear();
                    current = k;
                }
                None => {
                    parts.push((name, Vec::new()));
                    current = parts.len() - 1;
                }
            }
            i = j;
            continue;
        }
        parts[current].1.push(t.clone());
        i += 1;
    }
    parts
}

/// The shape keys, read from the engine's pgfkeys.
pub(super) struct EngineKeys<'a>(pub(super) &'a mut Engine);

impl shapes::Keys for EngineKeys<'_> {
    fn length(&mut self, key: &str, default: f64) -> f64 {
        self.0.key_length(key, default)
    }

    fn length_of(&mut self, text: &str, default: f64) -> f64 {
        self.0.length(text).unwrap_or(default)
    }

    fn number(&mut self, key: &str, default: f64) -> f64 {
        match keys::value(&self.0.tex, key) {
            Some(v) if !keys::trim(&v).is_empty() => self.0.number_of(v).unwrap_or(default),
            _ => default,
        }
    }

    fn text(&mut self, key: &str) -> String {
        match keys::value(&self.0.tex, key) {
            Some(v) => self.0.tex.expand_to_string(v).unwrap_or_default(),
            None => String::new(),
        }
    }

    fn flag(&mut self, name: &str) -> bool {
        let m = self.0.tex.meaning_of(&format!("if{name}"));
        m == self.0.tex.meaning_of("iftrue")
    }

    fn point(&mut self, key: &str) -> Option<Point> {
        let text = self.text(key);
        let text = crate::keys::strip_braces(text.trim()).trim().to_string();
        if text.is_empty() {
            return None;
        }
        // `(a)+(b)`: the second relative to the first
        let mut total: Option<Point> = None;
        for part in text.split(")+(").map(|s| s.trim_start_matches('+')) {
            let inner = part.trim().trim_start_matches('(').trim_end_matches(')').to_string();
            let p = self.0.coordinate(&inner).ok()?.p;
            total = Some(match total {
                None => p,
                Some(t) => t.plus(p.minus(self.0.coordinate("0,0").ok()?.p)),
            });
        }
        total
    }

    fn text_box(&mut self, tex: &str) -> BoxSize {
        let (text, _) = self.0.shape_text(tex);
        if text.trim().is_empty() {
            return BoxSize::default();
        }
        let mut warnings = Vec::new();
        let l = label::compose_label(text.trim(), &self.0.st.text.font, None, 'c', self.0.st.text_paint_color(), &mut warnings);
        BoxSize { wd: l.width(), ht: l.height(), dp: l.depth() }
    }

    fn code_paths(&mut self, key: &str, t: Transform) -> Vec<Vec<Segment>> {
        let e = &mut *self.0;
        let Some(code) = keys::value(&e.tex, key).filter(|v| !keys::trim(v).is_empty()) else { return Vec::new() };
        // run where nothing it draws stays or counts for the picture's size
        let n = e.canvas.items.len();
        let frozen = e.canvas_frozen;
        let path = std::mem::take(&mut e.pgf.pgf_path);
        e.begin_group();
        e.canvas_frozen = true;
        e.st.transform = t;
        e.st.mode = Default::default();
        e.st.mode.draw = true;
        let r = e.run_tokens(code);
        e.end_group();
        e.canvas_frozen = frozen;
        e.pgf.pgf_path = path;
        if let Err(err) = r {
            e.warn(format!("{key}: {err}"));
        }
        e.canvas.items.split_off(n).into_iter().filter_map(|i| if let Item::Path { segments, stroke: Some(_), .. } = i { Some(segments) } else { None }).collect()
    }
}
/// A node's labels to make: the node, their specifications (true for a pin), the last node.
/// and the values of [`LABEL_KEYS`] in the node.
type NodeLabels = (NodeInfo, Vec<(Vec<Token>, bool)>, (Option<NodeInfo>, Option<String>), Vec<(&'static str, tex::Meaning)>);

/// The keys of a node's options that its labels read.
const LABEL_KEYS: [&str; 3] = ["/tikz/label distance", "/tikz/label position", "/tikz/pin distance"];

/// Where a node on a path goes: the path's current point, or along its last segment.
#[derive(Debug, Clone, Copy)]
pub enum OnPath {
    Point(Point),
    /// a line or a curve: start, controls, end
    Segment(Point, Option<(Point, Point)>, Point),
    /// `-|` and `|-`: two legs through a corner (pos 0.5 is the corner)
    Corner(Point, Point, Point),
    /// an arc: its center, the ends of its axes at 0° and 90°, and its angles (pos goes along the
    /// angle, as `\pgftransformarcaxesattime`)
    Arc {
        center: Point,
        x: Point,
        y: Point,
        from: f64,
        to: f64,
    },
}

impl OnPath {
    pub fn at(&self, t: f64) -> (Point, Point) {
        match *self {
            OnPath::Point(p) => (p, Point::new(1.0, 0.0)),
            OnPath::Segment(a, None, b) => (a.lerp(b, t), b.minus(a)),
            OnPath::Segment(a, Some((c1, c2)), b) => {
                let u = 1.0 - t;
                let p = a.scale(u * u * u).plus(c1.scale(3.0 * u * u * t)).plus(c2.scale(3.0 * u * t * t)).plus(b.scale(t * t * t));
                let d = c1.minus(a).scale(3.0 * u * u).plus(c2.minus(c1).scale(6.0 * u * t)).plus(b.minus(c2).scale(3.0 * t * t));
                (p, d)
            }
            OnPath::Arc { center, x, y, from, to } => {
                let angle = (from + t * (to - from)).to_radians();
                let (s, c) = angle.sin_cos();
                let p = center.plus(x.scale(c)).plus(y.scale(s));
                let d = y.scale(c).minus(x.scale(s)).scale(if to < from { -1.0 } else { 1.0 });
                (p, d)
            }
            OnPath::Corner(a, c, b) => {
                if t <= 0.5 {
                    (a.lerp(c, 2.0 * t), c.minus(a))
                } else {
                    (c.lerp(b, 2.0 * t - 1.0), b.minus(c))
                }
            }
        }
    }
}
pub struct NodeSpec {
    pub options: Vec<Token>,
    pub name: Option<String>,
    pub at: Option<Point>,
    pub text: Vec<Token>,
    pub place: OnPath,
    /// the path's current point, where a node without `pos` goes
    pub point: Point,
    pub coordinate: bool,
    /// the default `pos` for a node between an operation and its target
    pub default_pos: Option<f64>,
}

impl Engine {
    /// A node's anchor; a name it does not have is an angle given by an expression, as pgf
    /// evaluates it (`(a.200+360/5)`).
    pub fn node_anchor(&mut self, node: &NodeInfo, a: &str) -> Option<Point> {
        if let Some(p) = node.anchor(a) {
            return Some(p);
        }
        let angle = self.number(a).ok()?;
        node.anchor(&angle.to_string())
    }
}
impl Engine {
    pub fn st_node(&mut self) -> &mut NodeSetup {
        &mut self.st.node
    }

    pub fn key_text(&mut self, path: &str) -> String {
        keys::value(&self.tex, path).map(|v| tex::tokens_to_string(&v)).unwrap_or_default()
    }

    fn key_length(&mut self, path: &str, default: f64) -> f64 {
        match keys::value(&self.tex, path) {
            Some(v) if !keys::trim(&v).is_empty() => self.length_of(v).unwrap_or(default),
            _ => default,
        }
    }

    /// `\tikzlastnode` is also known as `alias` (an unnamed node that edges need to name): the
    /// description finds it by either.
    pub fn alias_last_node(&mut self, alias: &str) {
        let last = self.tex.expand_to_string(vec![Token::cs("tikzlastnode")]).unwrap_or_default();
        let last = last.trim();
        if let Some(f) = self.facts.nodes.iter_mut().rev().find(|f| f.key == last) {
            f.key = alias.to_string();
        }
    }

    /// Creates a node: returns what it draws (drawn after its path) and registers its name.
    pub fn node(&mut self, spec: NodeSpec) -> Result<Vec<Item>, Error> {
        // `\pgfmathprintnumber{…}` in a node's text is printed as pgf prints numbers
        let mut spec = spec;
        if spec.text.iter().any(|t| t.is_cs("pgfmathprintnumber") || matches!(t, Token::Cs(name) if self.node_text_command(name).is_some())) {
            spec.text = super::number::print_numbers(self, &spec.text)?;
        }
        self.begin_group();
        // `transform shape` given to a scope or a path holds for its nodes
        self.st.node = NodeSetup { transform_shape: self.st.node.transform_shape, ..NodeSetup::default() };
        self.st.mode = Default::default();
        self.st.pending = Transform::default();
        self.st.chain.on_chain = None;
        self.st.chain.pending_joins.clear();
        self.st.after_node_path.clear();
        self.st.aliases.clear();
        self.st.pen.start_arrow = None;
        self.st.pen.end_arrow = None;
        // a node's background is decorated, and has actions, by its own options only
        let outer = self.st.clone();
        self.st.decorate = false;
        self.st.preactions = Default::default();
        self.st.postactions = Default::default();
        let shifts_before = self.anim.anim_shifts.len();
        let r = self.node_in_group(spec, &outer);
        // `behind path`: drawn before the path the node is on
        let behind = matches!(self.tex.meaning_of("tikra@behindpath"), tex::Meaning::Macro(m) if tex::tokens_to_string(&tex::substitute(&m.body, &[])).trim() == "1");
        self.end_group();
        let (mut items, labels) = r?;
        // animated shifts of the node: the picture makes room for it at each
        let shifts: Vec<Point> = self.anim.anim_shifts.drain(shifts_before..).collect();
        if let (false, Some(info)) = (shifts.is_empty(), self.last_node.clone()) {
            let ends: Vec<Point> = ["north", "south", "east", "west"].iter().filter_map(|a| info.anchor(a)).collect();
            if self.counts_for_size() {
                for d in shifts {
                    for p in &ends {
                        self.canvas.include(p.plus(d));
                    }
                }
            }
        }
        if behind {
            self.node_run.behind_path.append(&mut items);
        }
        // labels and pins: made after the node, out of its options (TikZ's `append after command`)
        if let Some((info, labels, last, kept)) = labels {
            // the label distances and position that the node's options set still hold
            self.begin_group();
            for (key, value) in kept {
                self.tex.set_meaning(&Token::cs(&keys::macro_name(key)), value, false);
            }
            let r = (|| {
                for (spec_tokens, pin) in labels {
                    items.extend(self.label(&info, spec_tokens, pin)?);
                }
                Ok::<(), Error>(())
            })();
            self.end_group();
            r?;
            (self.last_node, self.last_node_name) = last;
        }
        Ok(items)
    }

    /// `node also [options] (name)`, `late options={name=…, …}`: the labels and pins of the
    /// options, for a node that exists.
    pub fn late_options(&mut self, name: &str, options: Vec<Token>) -> Result<Vec<Item>, Error> {
        let Some(info) = self.lookup_node(name) else {
            self.warn(format!("late options: no node named '{name}'"));
            return Ok(Vec::new());
        };
        self.begin_group();
        self.st.node = NodeSetup::default();
        let r = (|| {
            self.process_keys(options, Some("/tikz"))?;
            let mut items = Vec::new();
            for (spec, pin) in std::mem::take(&mut self.st.node.labels) {
                items.extend(self.label(&info, spec, pin)?);
            }
            Ok(items)
        })();
        self.end_group();
        r
    }

    /// `\pgfpositionnodenow{point}`: the node `\pgfpositionnodelater` kept, moved from the origin
    /// it was made at to the point, drawn and known by its name.
    pub fn position_node_now(&mut self) -> Result<(), Error> {
        let p = self.point_arg()?;
        let d = self.st.transform.apply(p);
        let Some((items, mut info, name)) = self.node_run.later_node.take() else {
            self.warn("\\pgfpositionnodenow: no node kept by \\pgfpositionnodelater".to_string());
            return Ok(());
        };
        info.transform = Transform::translation(d.x, d.y).compose(&info.transform);
        for mut item in items {
            item.translate(d);
            if let (Item::Path { segments, .. }, true) = (&item, self.counts_for_size()) {
                self.canvas.include_path(segments);
            }
            self.canvas.items.push(item);
        }
        if let Some(n) = name {
            self.nodes.insert(n, info);
        }
        Ok(())
    }

    /// The stroke of the current state.
    pub fn stroke(&self) -> Stroke {
        Stroke {
            color: self.st.stroke_color(),
            width: self.st.pen.line_width,
            dash: self.st.pen.dash.clone().filter(|d| !d.0.is_empty()),
            cap: self.st.pen.cap,
            join: self.st.pen.join,
            miter_limit: self.st.pen.miter_limit,
            opacity: self.st.paint.draw_opacity,
        }
    }
}
