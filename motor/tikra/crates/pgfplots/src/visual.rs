//! Drawing one plot (`\pgfplots@stored@plotlist@visualize@selected@phases`): the plot handlers
//! (lines, `smooth`, `const plot`, `jump mark`, combs, `ybar`/`xbar`, intervals), `\closedcycle`,
//! `fill between`, error bars, and the marks drawn afterwards, as pgf's mark handler draws them.

use tikra_engine::scene::{Point, Segment, Transform};
use tikra_engine::tex::Token;

use super::draw::Frame;
use super::plot::Plot;
use super::Engine;

mod curve;
mod nodes;
mod stamps;

pub use curve::draw_plot;
pub use nodes::draw_near_nodes;
pub use stamps::{draw_mark_set, draw_marks, error_ends};

/// The marks of a plot, drawn once all plots are: its style and the canvas points.
#[derive(Debug, Clone)]
pub struct Marks {
    pub style: Vec<Token>,
    /// Canvas points and their point meta.
    pub points: Vec<(Point, f64)>,
    /// `scatter`: each mark in the colormap's color for its point meta.
    pub scatter: bool,
    /// `nodes near coords`: what the nodes say, and the nodes.
    pub node_text: Vec<Token>,
    pub nodes: Vec<NearNode>,
    /// Points with a `coordinate style` of their own, drawn after the others.
    pub styled: Vec<(Point, f64, Vec<Token>)>,
    /// The macros of `visualization depends on` at each of `points`.
    pub deps: Vec<Vec<(String, String)>>,
}

/// A node of `nodes near coords`.
#[derive(Debug, Clone)]
pub struct NearNode {
    /// The point it belongs to (kept or dropped with it), and where the node is.
    pub point: Point,
    pub at: Point,
    /// Alignment options (`above`, `anchor=center`…).
    pub align: String,
    pub meta: f64,
    /// The point meta as written, for symbolic point meta.
    pub label: Option<Vec<Token>>,
    pub index: usize,
    /// `/data point/x` and its friends.
    pub data: [f64; 3],
    /// The point's `coordinate style`.
    pub style: Vec<Token>,
    /// The point's `visualization depends on` macros and their values.
    pub deps: Vec<(String, String)>,
}

/// The canvas points of a plot (None at jumps).
fn canvas_points(f: &Frame, pts: &[[f64; 3]]) -> Vec<Option<Point>> {
    pts.iter()
        .map(|l| {
            let z = if f.three_d { l[2] } else { f.min[2] };
            if l[0].is_finite() && l[1].is_finite() && z.is_finite() {
                Some(f.at([l[0], l[1], z]))
            } else {
                None
            }
        })
        .collect()
}

/// pgf's curveto handler: support points at `tension` times the neighbours' difference.
fn smooth(points: &[Point], tension: f64, segs: &mut Vec<Segment>) {
    let t = 0.2775 * tension;
    if points.len() < 3 {
        for p in &points[1..] {
            segs.push(Segment::LineTo(*p));
        }
        return;
    }
    let mut first_support = points[0];
    for i in 1..points.len() - 1 {
        let d = points[i + 1].minus(points[i - 1]).scale(t);
        segs.push(Segment::CurveTo(first_support, points[i].minus(d), points[i]));
        first_support = points[i].plus(d);
    }
    let last = points[points.len() - 1];
    segs.push(Segment::CurveTo(first_support, last, last));
}

/// The pieces of a plot between jumps.
fn runs(points: &[Option<Point>]) -> Vec<Vec<Point>> {
    let mut out = vec![Vec::new()];
    for p in points {
        match p {
            Some(p) => out.last_mut().expect("a run").push(*p),
            None => {
                if !out.last().is_some_and(Vec::is_empty) {
                    out.push(Vec::new());
                }
            }
        }
    }
    out.into_iter().filter(|r| !r.is_empty()).collect()
}

/// One plot drawn in its axis: its points, the base stacked bars stand on, and its place among plots of its handler.
#[derive(Clone, Copy)]
pub struct PlotDraw<'a> {
    pub plot: &'a Plot,
    pub pts: &'a [[f64; 3]],
    pub base: &'a [f64],
    pub frame: &'a Frame,
    pub of_type: usize,
    pub index_of_type: usize,
}

/// Where `nodes near coords` are asked for. The base, the handler and the shift are the bar's;
/// a mesh passes none of them.
#[derive(Clone, Copy)]
struct NearQuery<'a> {
    plot: &'a Plot,
    pts: &'a [[f64; 3]],
    base: &'a [f64],
    frame: &'a Frame,
    handler: &'a str,
    shift: f64,
}

/// Nodes that follow a plot on its trailing path.
#[derive(Clone, Copy)]
struct PlacedNodes<'a> {
    plot: &'a Plot,
    pts: &'a [[f64; 3]],
    canvas: &'a [Option<Point>],
    frame: &'a Frame,
    rest: &'a [Token],
}

/// A mark stamped at several canvas points.
#[derive(Clone, Copy)]
pub struct MarkStamp<'a> {
    pub name: &'a str,
    pub size: f64,
    pub local: &'a Transform,
}

/// `mark=cube` in a 3d axis.
#[derive(Clone, Copy)]
struct CubeStamp<'a> {
    name: &'a str,
    size: f64,
    frame: &'a Frame,
}

fn define(e: &mut Engine, name: &str, value: &str) {
    e.tex.set_meaning(&Token::cs(name), tikra_engine::engine::macro_from(value), false);
}

fn unit(p: Point) -> Point {
    super::draw::unit(p)
}

fn rect(segs: &mut Vec<Segment>, a: Point, b: Point) {
    segs.push(Segment::MoveTo(a));
    segs.push(Segment::LineTo(Point::new(b.x, a.y)));
    segs.push(Segment::LineTo(b));
    segs.push(Segment::LineTo(Point::new(a.x, b.y)));
    segs.push(Segment::Close);
}

/// A path walked backwards.
pub(super) fn reverse(segs: &[Segment]) -> Vec<Segment> {
    let mut pts: Vec<(Point, Option<(Point, Point)>)> = Vec::new();
    for s in segs {
        match s {
            Segment::MoveTo(p) | Segment::LineTo(p) => pts.push((*p, None)),
            Segment::CurveTo(a, b, p) => pts.push((*p, Some((*a, *b)))),
            Segment::Close => {}
        }
    }
    let mut out = Vec::new();
    if let Some(last) = pts.last() {
        out.push(Segment::MoveTo(last.0));
    }
    for i in (1..pts.len()).rev() {
        match pts[i].1 {
            Some((a, b)) => out.push(Segment::CurveTo(b, a, pts[i - 1].0)),
            None => out.push(Segment::LineTo(pts[i - 1].0)),
        }
    }
    out
}
