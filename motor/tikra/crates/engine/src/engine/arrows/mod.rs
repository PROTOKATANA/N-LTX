//! Arrow tips as pgf draws them (`pgfcorearrows`, the `arrows.meta`, `arrows` and `arrows.spaced`
//! libraries): reading specifications (`->`, `<->`, `-{Latex[length=3mm]}`, `|<->|`, `<<.<-`), the
//! geometry of each tip with its keys, how far the line is pulled back into the tips, and the drawing.
//!
//! Geometry is in TeX points in the tip's own frame, as pgf writes it: the tip points right along x,
//! y is to its left. Only the placement converts to the canvas (cm).

mod keys;
mod legacy;
mod meta;
mod spec;

pub use keys::{Eval, Flex, Plain, Widths};
pub use spec::{clear_declared, declare_tip, parse_arrows, DeclaredTip, TipsMode, UserTips};

use crate::canvas::{Cap, Fill, Item, Join, Paint, Stroke};
use crate::color::Color;
use crate::scene::{Point, Segment, Transform};
use crate::PT;

use keys::{parse_opts, Keys, Opt};
use spec::{sequence, Kind, Old, Raw};

/// One path of a tip's drawing, in the tip's frame (pt).
#[derive(Debug, Clone, PartialEq)]
struct Shape {
    segs: Vec<Segment>,
    fill: bool,
    stroke: bool,
    width: f64,
    /// None: the path's
    cap: Option<Cap>,
    join: Option<Join>,
}

/// A tip as pgf instantiates it: its ends along x (tip end, back end, line end), the visual ends
/// that a flexed tip puts on the path, the convex hull that counts for the bounding box, and the
/// drawing.
#[derive(Debug, Clone, PartialEq)]
struct Geometry {
    tip: f64,
    back: f64,
    line: f64,
    visual_tip: f64,
    visual_back: f64,
    hull: Vec<Point>,
    shapes: Vec<Shape>,
}

/// One cubic of a tip, in the tip's frame (pt).
struct Cubic {
    c1: Point,
    c2: Point,
    end: Point,
}

impl Cubic {
    fn new(c1: Point, c2: Point, end: Point) -> Cubic {
        Cubic { c1, c2, end }
    }
}

/// A path being built in the tip's frame.
#[derive(Default)]
struct Pen(Vec<Segment>);

impl Pen {
    fn m(&mut self, x: f64, y: f64) -> &mut Self {
        self.0.push(Segment::MoveTo(Point::new(x, y)));
        self
    }
    fn l(&mut self, x: f64, y: f64) -> &mut Self {
        self.0.push(Segment::LineTo(Point::new(x, y)));
        self
    }
    fn c(&mut self, curve: Cubic) -> &mut Self {
        self.0.push(Segment::CurveTo(curve.c1, curve.c2, curve.end));
        self
    }
    fn z(&mut self) -> &mut Self {
        self.0.push(Segment::Close);
        self
    }
    /// `\pgfpatharc`: from the current point, around the center that puts it at angle `a0`.
    fn arc(&mut self, a0: f64, a1: f64, rx: f64, ry: f64) -> &mut Self {
        let at = self.current();
        let (s, c) = a0.to_radians().sin_cos();
        let center = Point::new(at.x - rx * c, at.y - ry * s);
        self.0.extend(super::path::arc_segments(super::path::EllipseArc { center, rx, ry, start: a0, end: a1 }, &Transform::default()));
        self
    }
    /// `\pgfpathellipse` with axes along x and y.
    fn ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64) -> &mut Self {
        self.m(cx + rx, cy);
        self.arc(0.0, 360.0, rx, ry);
        self.z()
    }
    fn current(&self) -> Point {
        match self.0.last() {
            Some(Segment::MoveTo(p) | Segment::LineTo(p) | Segment::CurveTo(_, _, p)) => *p,
            _ => Point::default(),
        }
    }
    fn take(&mut self) -> Vec<Segment> {
        std::mem::take(&mut self.0)
    }
}

fn map_segments(segs: &[Segment], f: impl Fn(Point) -> Point) -> Vec<Segment> {
    segs.iter()
        .map(|s| match *s {
            Segment::MoveTo(p) => Segment::MoveTo(f(p)),
            Segment::LineTo(p) => Segment::LineTo(f(p)),
            Segment::CurveTo(a, b, p) => Segment::CurveTo(f(a), f(b), f(p)),
            Segment::Close => Segment::Close,
        })
        .collect()
}

fn ratio(a: f64, b: f64) -> f64 {
    if b.abs() < 1e-9 {
        0.0
    } else {
        a / b
    }
}

/// The setup and drawing code of a tip, then its reversal (`\pgf@arrows@handle@reverse`).
fn geometry(kind: Kind, k: &Keys, lw: Widths) -> Geometry {
    let mut g = match kind {
        Kind::Old(Old::DirectionEe) => legacy::direction_ee(k.length, lw),
        Kind::Old(o) => legacy::old(o, lw, k.reversed),
        _ => meta::meta(kind, k, lw),
    };
    if k.reversed {
        let (t, b) = (g.tip, g.back);
        g.tip = -b;
        g.back = -t;
        g.line = -g.line;
        let (vt, vb) = (g.visual_tip, g.visual_back);
        g.visual_tip = -vb;
        g.visual_back = -vt;
        let flip = |p: Point| Point::new(-p.x, p.y);
        g.hull = g.hull.iter().map(|&p| flip(p)).collect();
        for s in &mut g.shapes {
            s.segs = map_segments(&s.segs, flip);
        }
    }
    g
}

// ---------------------------------------------------------------------------------------- placing

/// A tip of an end, ready to be placed.
#[derive(Debug, Clone, PartialEq)]
struct Placed {
    g: Geometry,
    sep: f64,
    slant: f64,
    swap: bool,
    color: Option<Color>,
    fill: Option<Color>,
    flex: Flex,
}

/// The tips at one end of a path, as pgf computes them before drawing (`\pgf@arrow@compute@shortening`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EndTips {
    tips: Vec<Placed>,
    /// How far the path is pulled back from its end (pt), `shorten` included.
    pub shortening: f64,
    /// From the back of the first tip to the end of the path (pt), `shorten` included.
    pub total: f64,
}

/// What [`EndTips::new`] is asked for: the specification, which end, and the line it sits on.
pub struct TipQuery<'a> {
    pub spec: &'a str,
    pub start: bool,
    pub user: &'a UserTips,
    pub widths: Widths,
    pub shorten: f64,
}

/// Where one end is drawn, and the stroke it takes its color and width from.
pub struct TipEnd<'a> {
    pub at: Point,
    pub from: Point,
    pub stroke: &'a Stroke,
    pub fill_opacity: f64,
}

/// A tip drawn for a caller without an engine: where it sits and which way it points.
pub struct TipMark<'a> {
    pub spec: &'a str,
    pub at: Point,
    pub direction: Point,
    pub stroke: &'a Stroke,
}

impl EndTips {
    /// The tips of `spec` (none if empty) at the start or the end of a path drawn with `lw`,
    /// shortened by `shorten` (pt) more. Warnings for unknown tips and keys.
    pub fn new(ev: &mut dyn Eval, query: &TipQuery<'_>, warn: &mut Vec<String>) -> EndTips {
        let spec = query.spec;
        let start = query.start;
        let user = query.user;
        let lw = query.widths;
        let shorten = query.shorten;
        let mut e = EndTips::default();
        let seq = if spec.trim().is_empty() {
            Vec::new()
        } else {
            sequence(spec, start, user, 0).unwrap_or_else(|| {
                warn.push(format!("arrow tip '{}' not supported: ignored", spec.trim()));
                Vec::new()
            })
        };
        let mut after_line_end = false;
        for r in seq {
            let (kind, texts) = match r {
                Raw::Dot => {
                    after_line_end = true;
                    continue;
                }
                Raw::Tip { kind, opts } => (kind, opts),
            };
            let opts: Vec<Opt> = texts.iter().flat_map(|t| parse_opts(ev, t, warn)).collect();
            let k = Keys::of(kind, &opts, lw);
            let g = geometry(kind, &k, lw);
            e.total += g.tip - g.back + k.sep;
            if after_line_end {
                e.shortening += g.tip + k.sep - g.back;
            } else {
                e.shortening = g.tip + k.sep - g.line;
            }
            e.tips.push(Placed { g, sep: k.sep, slant: k.slant, swap: k.swap, color: k.color, fill: k.fill, flex: k.flex });
        }
        e.shortening += shorten;
        e.total += shorten;
        e
    }

    pub fn is_empty(&self) -> bool {
        self.tips.is_empty()
    }

    /// Draws the tips for an end at `at` reached from `from` (canvas cm): pgf's
    /// `\pgf@arrow@draw@arrow` in the frame of `\pgftransformarrow`. Returns the hull points, which
    /// count for the bounding box. On a path of no length the tips point up, as pgf's normalized
    /// null vector does.
    pub fn draw(&self, end: &TipEnd<'_>, items: &mut Vec<Item>) -> Vec<Point> {
        let at = end.at;
        let from = end.from;
        let stroke = end.stroke;
        let fill_opacity = end.fill_opacity;
        let d = at.minus(from);
        let u = if d.norm() > 1e-12 { d.scale(1.0 / d.norm()) } else { Point::new(0.0, 1.0) };
        let n = Point::new(-u.y, u.x);
        let canvas = |p: Point| at.plus(u.scale(p.x * PT)).plus(n.scale(p.y * PT));
        let mut hull = Vec::new();
        let mut pos = -self.total;
        for t in &self.tips {
            let off = pos - t.g.back;
            pos += t.g.tip + t.sep - t.g.back;
            t.paint(|p| canvas(Point::new(p.x + off, p.y)), stroke, fill_opacity, items, &mut hull);
        }
        hull
    }

    /// Whether the end is drawn along a curve, when it ends with one: some tip is flexed (pgf's
    /// precise shortening).
    pub fn flexed(&self) -> bool {
        self.tips.iter().any(|t| t.flex != Flex::Quick)
    }

    /// Draws the tips on a curve (`\pgf@do@draw@curved`), each turned so that its visual ends (or
    /// tip and back ends, for `flex'`) lie on it; the curve is measured from its point `along.0`,
    /// where the path ends. Returns the hull points.
    pub fn draw_on_curve(&self, along: &Curvilinear, stroke: &Stroke, fill_opacity: f64, items: &mut Vec<Item>) -> Vec<Point> {
        let mut hull = Vec::new();
        let mut xb = self.total;
        for t in &self.tips {
            xb += t.g.back;
            let origin = xb;
            xb -= t.g.tip + t.sep;
            let (start, end, rigidity) = match t.flex {
                Flex::FlexPrime(r) => (t.g.tip, t.g.back, r),
                Flex::Flex(r) => (t.g.visual_tip, t.g.visual_back, r),
                Flex::Quick | Flex::Bend => (t.g.visual_tip, t.g.visual_back, 1.0),
            };
            let end = rigidity * (end - start) + start;
            let (at, u) = if (start - end).abs() < 1e-9 {
                // no length to flex: along the tangent
                let time = along.time(origin - start);
                (along.at(time), along.toward_start(time).scale(-1.0))
            } else {
                let p = along.at(along.time(origin - start));
                let q = along.at(along.time(origin - end));
                let d = p.minus(q);
                (p, if d.norm() > 1e-12 { d.scale(1.0 / d.norm()) } else { Point::new(0.0, 1.0) })
            };
            let n = Point::new(-u.y, u.x);
            t.paint(|p| at.plus(u.scale((p.x - start) * PT)).plus(n.scale(p.y * PT)), stroke, fill_opacity, items, &mut hull);
        }
        hull
    }
}

impl Placed {
    /// Paints the tip through `place`, from its frame (pt) after its slant and swap to the canvas.
    fn paint(&self, place: impl Fn(Point) -> Point, stroke: &Stroke, fill_opacity: f64, items: &mut Vec<Item>, hull: &mut Vec<Point>) {
        let local = |p: Point| {
            let y = if self.swap { -p.y } else { p.y };
            place(Point::new(p.x + self.slant * y, y))
        };
        let color = self.color.unwrap_or(stroke.color);
        for s in &self.g.shapes {
            let segments = map_segments(&s.segs, local);
            let fill = s.fill.then(|| Fill { paint: Paint::Solid(self.fill.unwrap_or(color)), opacity: fill_opacity, even_odd: false });
            let line = s.stroke.then(|| Stroke { color, width: s.width * PT, dash: None, cap: s.cap.unwrap_or(stroke.cap), join: s.join.unwrap_or(stroke.join), ..stroke.clone() });
            items.push(Item::Path { segments, fill, stroke: line });
        }
        hull.extend(self.g.hull.iter().map(|&p| local(p)));
    }
}

/// A cubic curve measured by length from its first point, the way pgf's curvilinear library does
/// it: lengths at four times near the start (about 1, 2, 4 and 8 pt along), straight between them.
#[derive(Debug, Clone, PartialEq)]
pub struct Curvilinear {
    pub points: [Point; 4],
    tau: f64,
    lengths: [f64; 4],
}

impl Curvilinear {
    /// `\pgfsetcurvilinearbeziercurve` (canvas cm).
    pub fn new(points: [Point; 4]) -> Curvilinear {
        let mut c = Curvilinear { points, tau: 0.0, lengths: [0.0; 4] };
        let pt = |a: Point, b: Point| b.minus(a).norm() / PT;
        let polygon = pt(points[0], points[1]) + pt(points[1], points[2]) + pt(points[2], points[3]);
        c.tau = ratio(1.0, polygon);
        let mut first = pt(points[0], c.at(c.tau));
        if first > 1.0 {
            c.tau = ratio(c.tau, first);
            first = pt(points[0], c.at(c.tau));
        }
        let mut sum = first;
        c.lengths[0] = sum;
        for (k, f) in [2.0, 4.0, 8.0].into_iter().enumerate() {
            sum += pt(c.at(c.tau * f / 2.0), c.at(c.tau * f));
            c.lengths[k + 1] = sum;
        }
        c
    }

    /// The point at `t` (beyond 1, the cubic goes on).
    pub fn at(&self, t: f64) -> Point {
        let [a, b, c, d] = self.points;
        let s = 1.0 - t;
        a.scale(s * s * s).plus(b.scale(3.0 * s * s * t)).plus(c.scale(3.0 * s * t * t)).plus(d.scale(t * t * t))
    }

    /// The unit direction at `t` toward increasing `t`, away from the first point.
    fn toward_start(&self, t: f64) -> Point {
        let [a, b, c, d] = self.points;
        let s = 1.0 - t;
        let mut v = b.minus(a).scale(3.0 * s * s).plus(c.minus(b).scale(6.0 * s * t)).plus(d.minus(c).scale(3.0 * t * t));
        if v.norm() < 1e-12 {
            // a degenerate start: toward the first control point that differs
            v = self.points.iter().find(|p| p.minus(a).norm() > 1e-12).map(|p| p.minus(a)).unwrap_or(Point::new(0.0, -1.0));
        }
        v.scale(1.0 / v.norm())
    }

    /// `\pgfcurvilineardistancetotime`: the time at `distance` (pt) from the first point.
    pub fn time(&self, distance: f64) -> f64 {
        let [la, lb, lc, ld] = self.lengths;
        let tau = self.tau;
        // straight through (l0, t0) and (l1, t1)
        let line = |l0: f64, t0: f64, l1: f64, t1: f64| t0 + ratio(t1 - t0, l1 - l0) * (distance - l0);
        if distance < la {
            ratio(tau, la) * distance
        } else if distance < lb {
            line(la, tau, lb, 2.0 * tau)
        } else if distance < lc {
            line(lb, 2.0 * tau, lc, 4.0 * tau)
        } else if distance < ld {
            line(lc, 4.0 * tau, ld, 8.0 * tau)
        } else {
            ratio(8.0 * tau, ld) * distance
        }
    }

    /// The curve from time `t` to the end, as a cubic: what is left once the start is cut off.
    pub fn after(&self, t: f64) -> [Point; 4] {
        let [a, b, c, d] = self.points;
        let ab = a.lerp(b, t);
        let bc = b.lerp(c, t);
        let cd = c.lerp(d, t);
        let abc = ab.lerp(bc, t);
        let bcd = bc.lerp(cd, t);
        [abc.lerp(bcd, t), bcd, cd, d]
    }
}

/// A path of a tip: its segments, whether it is filled, and its stroke's caps and joins if stroked.
pub type TipPath = (Vec<Segment>, bool, Option<(Cap, Join)>);

/// The tip `spec` as `\pgfarrowdraw` draws it with lines `line_width` wide (cm), its end at the
/// origin pointing right: its total length (cm) and its paths, each with whether it is filled and
/// its stroke's caps and joins if it is stroked. None for an unknown tip.
pub fn tip_shapes(spec: &str, line_width: f64) -> Option<(f64, Vec<TipPath>)> {
    let mut warn = Vec::new();
    let e = EndTips::new(&mut Plain, &TipQuery { spec, start: false, user: &UserTips::default(), widths: Widths { line: line_width / PT, inner: 0.0 }, shorten: 0.0 }, &mut warn);
    if !warn.is_empty() || e.is_empty() {
        return None;
    }
    let stroke = Stroke { color: Color::BLACK, width: line_width, dash: None, cap: Cap::Butt, join: Join::Miter, miter_limit: 10.0, opacity: 1.0 };
    let mut items = Vec::new();
    e.draw(&TipEnd { at: Point::default(), from: Point::new(-1.0, 0.0), stroke: &stroke, fill_opacity: 1.0 }, &mut items);
    let paths = items
        .into_iter()
        .filter_map(|i| match i {
            Item::Path { segments, fill, stroke } => Some((segments, fill.is_some(), stroke.map(|s| (s.cap, s.join)))),
            _ => None,
        })
        .collect();
    Some((e.total * PT, paths))
}

/// Draws the tip `spec` at `p`, pointing along `d`, for a caller without an engine (the pgfplots
/// bridge). Returns how far to pull the line back, in cm.
pub fn tip_items(mark: &TipMark<'_>, items: &mut Vec<Item>) -> f64 {
    let mut warn = Vec::new();
    let e = EndTips::new(
        &mut Plain,
        &TipQuery { spec: mark.spec, start: false, user: &UserTips::default(), widths: Widths { line: mark.stroke.width / PT, inner: 0.0 }, shorten: 0.0 },
        &mut warn,
    );
    e.draw(&TipEnd { at: mark.at, from: mark.at.minus(mark.direction), stroke: mark.stroke, fill_opacity: mark.stroke.opacity }, items);
    e.shortening * PT
}

#[cfg(test)]
mod tests {
    use super::keys::Dep;
    use super::*;

    fn names(spec: &str, start: bool) -> Vec<String> {
        sequence(spec, start, &UserTips::default(), 0)
            .unwrap()
            .iter()
            .map(|r| match r {
                Raw::Tip { kind, opts } => format!("{kind:?}{opts:?}"),
                Raw::Dot => ".".into(),
            })
            .collect()
    }

    fn end(spec: &str, lw: f64) -> EndTips {
        let mut w = Vec::new();
        let e = EndTips::new(&mut Plain, &TipQuery { spec, start: false, user: &UserTips::default(), widths: Widths { line: lw, inner: 0.0 }, shorten: 0.0 }, &mut w);
        assert!(w.is_empty(), "{w:?}");
        e
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn specifications() {
        let u = UserTips::default();
        assert_eq!(parse_arrows("->", &u), Some((None, Some(">".into()))));
        assert_eq!(parse_arrows("<->", &u), Some((Some("<".into()), Some(">".into()))));
        assert_eq!(parse_arrows("-{Latex[length=3mm]}", &u), Some((None, Some("{Latex[length=3mm]}".into()))));
        assert_eq!(parse_arrows("|-|", &u), Some((Some("|".into()), Some("|".into()))));
        assert_eq!(parse_arrows("-", &u), Some((None, None)));
        assert_eq!(parse_arrows("<<.<->_>", &u), Some((Some("<<.<".into()), Some(">_>".into()))));
        assert_eq!(parse_arrows("red", &u), None);
        assert_eq!(parse_arrows("north east", &u), None);
        assert_eq!(parse_arrows("x-y", &u), None);
        assert_eq!(parse_arrows("-foo", &u), None);
        let mut user = UserTips::default();
        user.declare("foo", "Stealth[red]");
        assert!(parse_arrows("-foo", &user).is_some());
    }

    #[test]
    fn sequences_from_the_back_to_the_point() {
        // `>` is To, Computer Modern Rightarrow
        assert_eq!(names(">", false), ["CmRightarrow[]"]);
        assert_eq!(names(">>", false).len(), 2);
        // `<` at the end is `>` reversed; at the start it is `>`
        assert_eq!(names("<", false), ["CmRightarrow[\"reversed\"]"]);
        assert_eq!(names("<", true), ["CmRightarrow[]"]);
        // a shorthand's options come after the tip's own
        assert_eq!(names("Bar[red]", false), ["TeeBarb[\"length=+0pt\", \"red\"]"]);
        // the start reverses the order: the first tip read is at the point
        assert_eq!(names("Latex[] Stealth[]", false), ["Latex[]", "Stealth[]"]);
        assert_eq!(names("{Latex[] Stealth[]}", true), ["Stealth[]", "Latex[]"]);
        assert_eq!(names("|<", true), ["CmRightarrow[\"sep=0pt\"]", ".", "TeeBarb[\"length=+0pt\"]"]);
        // initial options for every tip
        assert_eq!(names("[red]>>", false), ["CmRightarrow[\"red\"]", "CmRightarrow[\"red\"]"]);
        assert_eq!(names("spaced to", false), ["Old(To)[\"sep=0pt\"]", ".", "Old(Space)[]"]);
        assert_eq!(names("]", false), ["Old(SquareBracket)[]"]);
        assert_eq!(names("[", true), ["Old(SquareBracket)[]"]);
        assert_eq!(names("stealth reversed", false), ["Old(Stealth)[\"reversed\"]"]);
        assert!(sequence("Nothing", false, &UserTips::default(), 0).is_none());
    }

    #[test]
    fn keys_in_pgf_order() {
        let lw = Widths { line: 0.4, inner: 0.0 };
        // width' is late: it reads the length given after it
        let k = Keys::of(Kind::Latex, &[Opt::WidthOfLength(Dep(0.0, 1.0, 0.0)), Opt::Length(Dep(10.0, 0.0, 0.0))], lw);
        assert!(close(k.length, 10.0) && close(k.width, 10.0));
        let k = Keys::of(Kind::Latex, &[Opt::Scale(2.0)], lw);
        assert!(close(k.length, 2.0 * (3.0 + 4.5 * 0.4)) && close(k.width, 0.75 * k.length));
        // a 60° angle: equal sides
        let k = Keys::of(Kind::Stealth, &[Opt::Angle(60.0, Dep(2.0, 0.0, 0.0))], lw);
        assert!(close(k.width, 2.0) && close(k.length, 3f64.sqrt()));
        let k = Keys::of(Kind::Latex, &[Opt::Reversed, Opt::Reversed, Opt::Swap], lw);
        assert!(!k.reversed && k.swap);
        // the line width of a double line's visible part
        let d = Widths { line: 2.0, inner: 1.0 };
        assert!(close(d.dep(Dep(0.0, 1.0, 1.0)), 0.5));
    }

    #[test]
    fn options_are_read() {
        let mut w = Vec::new();
        let o = parse_opts(&mut Plain, "length=3pt 2 1, width'=0pt 1, red, open, sep, harpoon, right, angle=60:2pt, n=3", &mut w);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(o[0], Opt::Length(Dep(3.0, 2.0, 1.0)));
        assert_eq!(o[1], Opt::WidthOfLength(Dep(0.0, 1.0, 0.0)));
        assert!(matches!(o[2], Opt::Color(_)));
        assert_eq!(o[4], Opt::Sep(Dep(0.88, 0.3, 1.0)));
        assert!(o.contains(&Opt::Swap) && o.contains(&Opt::Angle(60.0, Dep(2.0, 0.0, 0.0))) && o.contains(&Opt::N(3)));
        parse_opts(&mut Plain, "no such key=1", &mut w);
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn default_tip_is_computer_modern_rightarrow() {
        // length 1.6pt + 2.2 lw, line ends half a line width behind the point
        let e = end(">", 0.4);
        let l = 1.6 + 2.2 * 0.4;
        assert!(close(e.total, 0.2 + (l - 0.4) + 0.2), "{e:?}");
        assert!(close(e.shortening, 0.2 + 0.2));
        // the legacy `to`: the line stops at its back
        let e = end("to", 0.4);
        assert!(close(e.shortening, 0.21 + 0.625 * 0.4) && close(e.total, 0.21 + 0.625 * 0.4 + 0.84 + 1.3 * 0.4));
    }

    #[test]
    fn several_tips_and_the_line_end() {
        let one = end("Latex", 0.4);
        let two = end("Latex[] Latex[]", 0.4);
        assert!(close(two.total, 2.0 * one.total));
        // the line runs to the last tip
        assert!(close(two.shortening, one.shortening));
        // after `.` the tips add to the shortening
        let dot = end("Latex[] . Latex[]", 0.4);
        assert!(close(dot.shortening, one.shortening + one.total));
        let sep = end("Latex[sep=1pt]", 0.4);
        assert!(close(sep.total, one.total + 1.0) && close(sep.shortening, one.shortening + 1.0));
    }

    #[test]
    fn latex_geometry() {
        let e = end("Latex", 0.4);
        let g = &e.tips[0].g;
        let l = 3.0 + 4.5 * 0.4;
        // the point a miter short of the length, the line stopping at the back
        assert!(close(g.tip, l - 0.2) && close(g.back, -0.2) && close(g.line, 0.0), "{g:?}");
        assert!(g.shapes[0].fill && g.shapes[0].stroke);
        assert_eq!(g.hull.len(), 3);
        // reversed: the ends swap sides
        let r = end("Latex[reversed]", 0.4);
        let rg = &r.tips[0].g;
        assert!(close(rg.tip, 0.2) && close(rg.back, -(l - 0.2)));
        let open = end("Latex[open]", 0.4);
        assert!(!open.tips[0].g.shapes[0].fill);
    }

    #[test]
    fn stealth_and_kite_and_bar() {
        let e = end("Stealth", 0.0);
        let g = &e.tips[0].g;
        let l = 3.0;
        assert!(close(g.tip, l) && close(g.back, 0.0) && close(g.line, 0.325 * l), "{g:?}");
        let b = end("|", 0.4);
        let g = &b.tips[0].g;
        assert!(close(g.tip, 0.2) && close(g.back, -0.2) && close(g.line, -0.1));
        let k = end("Kite", 0.0);
        assert!(close(k.tips[0].g.tip, 3.6));
        // a harpoon has only the upper half
        let h = end("Straight Barb[harpoon]", 0.4);
        assert_eq!(h.tips[0].g.shapes[0].segs.len(), 3);
    }

    #[test]
    fn jack_tap() {
        // measured on CircuiTikZ 1.8.7: 0.3cm long and 0.15cm high whatever the line width, the
        // line pulled back a tenth more
        let l = 8.53583;
        for lw in [0.4, 2.0] {
            let e = end("Jack Tap", lw);
            let g = &e.tips[0].g;
            assert!(close(g.tip, 1.1 * l) && close(g.back, 0.0) && close(g.line, 0.0), "{g:?}");
            let s = &g.shapes[0];
            assert!(!s.fill && s.stroke && close(s.width, lw));
            assert_eq!(s.segs[2], Segment::LineTo(Point::new(0.6 * l, 4.26791)));
        }
        let wide = end("Jack Tap[width=.3cm,fill]", 0.4);
        let s = &wide.tips[0].g.shapes[0];
        assert!(s.fill && matches!(s.segs.last(), Some(Segment::Close)));
        assert!(end("Jack Tap[swap]", 0.4).tips[0].swap);
    }

    #[test]
    fn every_kind_draws() {
        for spec in [
            "Straight Barb",
            "Hooks",
            "Hooks[arc=270]",
            "Hooks[arc=60]",
            "Arc Barb",
            "Tee Barb",
            "Classical TikZ Rightarrow",
            "Classical TikZ Rightarrow[harpoon,reversed]",
            "Computer Modern Rightarrow[sharp]",
            "Implies",
            "Latex[harpoon]",
            "Stealth[round]",
            "Kite[left]",
            "Square",
            "Circle[harpoon]",
            "Round Cap",
            "Round Cap[reversed]",
            "Butt Cap",
            "Triangle Cap[reversed]",
            "Fast Triangle",
            "Fast Round",
            "Rays[n=5]",
            "Jack Tap[reversed,fill=red]",
            "Triangle",
            "Rectangle",
            "Ellipse",
            "Diamond",
            "Turned Square",
            "Bracket",
            "Parenthesis",
            "LaTeX",
            "_",
            "stealth",
            "to reversed",
            "latex reversed",
            "[",
            "]",
            "(",
            ")",
            "angle 90",
            "angle 60",
            "angle 45 reversed",
            "*",
            "o",
            "diamond",
            "open diamond",
            "square",
            "open square",
            "triangle 60",
            "open triangle 90 reversed",
            "open triangle 45",
            "latex'",
            "stealth' reversed",
            "left to",
            "right to reversed",
            "hooks",
            "left hook",
            "serif cm",
            "round cap",
            "butt cap",
            "triangle 90 cap reversed",
            "fast cap",
            "fast cap reversed",
            "implies",
            "spaced latex",
            "spaced ]",
        ] {
            let e = end(spec, 0.4);
            assert!(!e.tips.is_empty(), "{spec}");
            let mut items = Vec::new();
            let s = Stroke { color: Color::BLACK, width: 0.4 * PT, dash: Some((vec![1.0], 0.0)), cap: Cap::Butt, join: Join::Miter, miter_limit: 10.0, opacity: 1.0 };
            e.draw(&TipEnd { at: Point::new(1.0, 0.0), from: Point::new(0.0, 0.0), stroke: &s, fill_opacity: 1.0 }, &mut items);
            assert!(items.iter().all(|i| matches!(i, Item::Path { stroke, .. } if stroke.as_ref().is_none_or(|s| s.dash.is_none()))), "{spec}");
        }
    }

    #[test]
    fn curves_measured_from_their_start() {
        // a straight cubic 10 pt long, evenly parametrized: time is distance over length
        let p = |x: f64| Point::new(x * PT, 0.0);
        let c = Curvilinear::new([p(0.0), p(10.0 / 3.0), p(20.0 / 3.0), p(10.0)]);
        for d in [0.5, 1.0, 3.0, 7.0, 12.0] {
            assert!((c.time(d) - d / 10.0).abs() < 1e-6, "{d}: {}", c.time(d));
        }
        let [a, _, _, d] = c.after(0.5);
        assert!((a.x - 5.0 * PT).abs() < 1e-9 && d == p(10.0));
        // tips laid on a quarter circle stay near it
        let r = 1.0;
        let k = 0.5523 * r;
        let arc = Curvilinear::new([Point::new(r, 0.0), Point::new(r, k), Point::new(k, r), Point::new(0.0, r)]);
        let e = end("Latex[length=10pt]", 0.0);
        let mut items = Vec::new();
        let s = Stroke { color: Color::BLACK, width: 0.0, dash: None, cap: Cap::Butt, join: Join::Miter, miter_limit: 10.0, opacity: 1.0 };
        let hull = e.draw_on_curve(&arc, &s, 1.0, &mut items);
        // the point at the end, the back corners about 10 pt down the arc, both sides of it
        assert!(hull[0].minus(Point::new(r, 0.0)).norm() < 1e-6, "{hull:?}");
        let back = hull[1].lerp(hull[2], 0.5);
        assert!((back.norm() - r).abs() < 0.005 && back.y > 0.0, "{back:?}");
    }

    #[test]
    fn placement_along_the_path() {
        // pointing up: the tip of a Latex ends at the end point (lw 0)
        let e = end("Latex", 0.0);
        let mut items = Vec::new();
        let s = Stroke { color: Color::BLACK, width: 0.0, dash: None, cap: Cap::Butt, join: Join::Miter, miter_limit: 10.0, opacity: 1.0 };
        let hull = e.draw(&TipEnd { at: Point::new(0.0, 1.0), from: Point::new(0.0, 0.0), stroke: &s, fill_opacity: 1.0 }, &mut items);
        let top = hull.iter().map(|p| p.y).fold(f64::MIN, f64::max);
        assert!((top - 1.0).abs() < 1e-9, "{hull:?}");
        let low = hull.iter().map(|p| p.y).fold(f64::MAX, f64::min);
        assert!((low - (1.0 - 3.0 * PT)).abs() < 1e-9);
    }
}
