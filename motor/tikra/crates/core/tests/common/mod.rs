//! Helpers shared by the integration tests.
#![allow(dead_code)]

use tikra::canvas::{Canvas, Fill, Item, Stroke};
use tikra::scene::{Label, Point, Segment, Transform};

/// Converts and fails with the figure in the error text.
pub fn figure(tikz: &str) -> tikra::Output {
    tikra::convert(tikz).unwrap_or_else(|e| panic!("{e}\n{tikz}"))
}

/// Converts and requires zero warnings.
pub fn no_warnings(tikz: &str) -> tikra::Output {
    let r = figure(tikz);
    assert!(r.warnings.is_empty(), "warnings: {:?}\n{tikz}", r.warnings);
    r
}

/// Converts and requires a warning containing `fragment`.
pub fn with_warning(tikz: &str, fragment: &str) -> tikra::Output {
    let r = figure(tikz);
    assert!(r.warnings.iter().any(|a| a.contains(fragment)), "expected a warning with '{fragment}', got {:?}\n{tikz}", r.warnings);
    r
}

/// The canvas and warnings of a tikzpicture body (the environment is put around it), as drawn: moved
/// so that its bounding box starts at (0, 0).
pub fn drawn_and_warnings(body: &str) -> (Canvas, Vec<String>) {
    tikra::engine::run_document(&format!("\\begin{{tikzpicture}}{body}\\end{{tikzpicture}}")).unwrap_or_else(|e| panic!("{e}\n{body}"))
}

/// Half the side of the bounding box the picture is drawn in by `canvas_and_warnings`, in cm.
const FRAME: f64 = 500.0;

/// The canvas and warnings of a tikzpicture body, in the picture's own coordinates (cm, the origin
/// where TikZ has it). The figure is drawn inside a fixed bounding box that is moved back after
/// pgf's shift of the picture to its corner; `min` and `max` are that box, so use `drawn_and_warnings`
/// or `size` for the figure's own size.
pub fn canvas_and_warnings(body: &str) -> (Canvas, Vec<String>) {
    let (mut c, w) = drawn_and_warnings(&format!("\\useasboundingbox (-{FRAME},-{FRAME}) rectangle ({FRAME},{FRAME}); {body}"));
    let back = Point::new(-FRAME, -FRAME);
    for i in &mut c.items {
        i.translate(back);
    }
    c.min = c.min.map(|p| p.plus(back));
    c.max = c.max.map(|p| p.plus(back));
    (c, w)
}

/// The canvas of a tikzpicture body.
pub fn canvas(body: &str) -> Canvas {
    canvas_and_warnings(body).0
}

/// A path of the canvas, as pgf's `\pgfusepath` leaves it.
#[derive(Debug, Clone)]
pub struct Drawn {
    pub segments: Vec<Segment>,
    pub fill: Option<Fill>,
    pub stroke: Option<Stroke>,
}

/// The paths of a canvas, in drawing order (arrow tips and node borders are paths too).
pub fn paths(c: &Canvas) -> Vec<Drawn> {
    c.items
        .iter()
        .filter_map(|i| match i {
            Item::Path { segments, fill, stroke } => Some(Drawn { segments: segments.clone(), fill: fill.clone(), stroke: stroke.clone() }),
            _ => None,
        })
        .collect()
}

/// The texts of a canvas: each label and where its first baseline starts.
pub fn texts(c: &Canvas) -> Vec<(Label, Transform)> {
    c.items
        .iter()
        .filter_map(|i| match i {
            Item::Text { label, place, .. } => Some((label.clone(), *place)),
            _ => None,
        })
        .collect()
}

/// The first path of a body.
pub fn first_path(body: &str) -> Drawn {
    paths(&canvas(body)).into_iter().next().unwrap_or_else(|| panic!("no path in {body}"))
}

/// The point, in cm, that TikZ gives the coordinate `coordinate` (`a.north`, `1,2`, `$(a)+(1,0)$`)
/// after `body`: a path is drawn from it and its start is read back.
pub fn point(body: &str, coordinate: &str) -> Point {
    let c = canvas(&format!("{body} \\draw ({coordinate}) -- +(0,0);"));
    match paths(&c).last().map(|d| d.segments[0]) {
        Some(Segment::MoveTo(p)) => p,
        other => panic!("no point for ({coordinate}): {other:?}\n{body}"),
    }
}

/// Width and height of a body's bounding box, in cm.
pub fn size(body: &str) -> (f64, f64) {
    let c = drawn_and_warnings(body).0;
    let (a, b) = (c.min.expect("an empty picture"), c.max.unwrap());
    (b.x - a.x, b.y - a.y)
}

/// End points of each segment (Close has none).
pub fn points(segs: &[Segment]) -> Vec<(f64, f64)> {
    segs.iter()
        .filter_map(|s| match s {
            Segment::MoveTo(p) | Segment::LineTo(p) | Segment::CurveTo(_, _, p) => Some((p.x, p.y)),
            Segment::Close => None,
        })
        .collect()
}

/// The bounding box of segments (control points included): min and max.
pub fn bounds(segs: &[Segment]) -> (Point, Point) {
    let mut c = Canvas::default();
    c.include_path(segs);
    (c.min.expect("no points"), c.max.unwrap())
}

pub fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

pub fn close_to(p: (f64, f64), q: (f64, f64)) -> bool {
    close(p.0, q.0) && close(p.1, q.1)
}
