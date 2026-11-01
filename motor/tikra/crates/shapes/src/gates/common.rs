//! Measures, rays and anchors shared by the logic gates.

use crate::scene::{Point, Segment};
use crate::PT;

use super::super::basic::ellipse_path;
use super::super::{intersect_lines, polar, Border, EllipseSpan, Geometry, Input};

/// What every gate saves: its inputs, the inverted radius, `\outerinvertedradius` (with half the
/// line width) and the center of its text.
pub(super) struct Common {
    pub(super) inputs: Vec<bool>,
    pub(super) r: f64,
    pub(super) outer_r: f64,
    pub(super) sep: f64,
    pub(super) lw: f64,
    pub(super) c: Point,
}

pub(super) fn common(input: &mut Input, max: usize) -> Common {
    let spec = input.keys.text("/pgf/logic gate inputs");
    let inputs = super::parse_inputs(&spec, max);
    let r = input.length("/pgf/logic gate inverted radius", 2.0 * PT);
    let sep = input.length("/pgf/logic gate input sep", 0.125);
    Common { inputs, r, outer_r: r + 0.5 * input.line_width, sep, lw: input.line_width, c: input.center_point() }
}

/// The anchors every gate has from its text: `center`, `mid`, `base`, `text`.
pub(super) fn text_anchors(g: &mut Geometry, input: &Input) {
    g.anchor_at("center", input.center_point());
    g.anchor_at("mid", input.mid_point());
    g.anchor_at("base", input.base_point());
}

/// `mid east`, `base east`… : the x of an anchor at the height of `mid` and `base`.
pub(super) fn side_anchors(g: &mut Geometry, input: &Input, side: &str, x: f64) {
    g.anchor_at(&format!("mid {side}"), Point::new(x, input.mid_point().y));
    g.anchor_at(&format!("base {side}"), Point::new(x, 0.0));
}

/// An elliptical arc around `center`: the piece a ray is tested against.
pub(super) struct EllipticArc {
    pub(super) center: Point,
    pub(super) span: EllipseSpan,
}

/// Where the ray from `origin` along `direction` meets `arc` (`\pgfmathpointintersectionoflineandarc`,
/// for a line through the center).
pub(super) fn ray_arc(origin: Point, direction: Point, arc: EllipticArc) -> Point {
    let EllipticArc { center: m, span: EllipseSpan { a0, a1, rx, ry } } = arc;
    let (o, d) = (origin, direction);
    // o + t d on the ellipse: a t² + b t + c = 0
    let w = o.minus(m);
    let (ax, ay) = (d.x / rx, d.y / ry);
    let (wx, wy) = (w.x / rx, w.y / ry);
    let a = ax * ax + ay * ay;
    let b = 2.0 * (ax * wx + ay * wy);
    let c = wx * wx + wy * wy - 1.0;
    let disc = b * b - 4.0 * a * c;
    if a < 1e-30 || disc < 0.0 {
        return o;
    }
    let (lo, hi) = (a0.min(a1), a0.max(a1));
    let within = |p: Point| {
        let q = p.minus(m);
        let ang = (q.y / ry).atan2(q.x / rx).to_degrees();
        (-1..=1).map(|k| ang + 360.0 * k as f64).any(|x| x >= lo - 1e-6 && x <= hi + 1e-6)
    };
    let mut roots: Vec<f64> = [(-b - disc.sqrt()) / (2.0 * a), (-b + disc.sqrt()) / (2.0 * a)].into_iter().filter(|t| *t > 0.0).collect();
    roots.sort_by(|x, y| x.total_cmp(y));
    let pts: Vec<Point> = roots.iter().map(|t| o.plus(d.scale(*t))).collect();
    pts.iter().copied().find(|p| within(*p)).or_else(|| pts.last().copied()).unwrap_or(o)
}

/// The ray from `o` along `d` against the line through `a` and `b`.
pub(super) fn ray_line(o: Point, d: Point, a: Point, b: Point) -> Point {
    intersect_lines(o, o.plus(d), a, b).unwrap_or(o)
}

/// The border as pgf's border code gives it for each direction, sampled every half degree.
pub(super) fn sampled(f: impl Fn(Point) -> Point) -> Border {
    Border::Outline((0..720).map(|k| f(polar(k as f64 * 0.5, 1.0))).collect())
}

/// The angle of `q` seen from `p`, in [0, 360).
pub(super) fn angle(p: Point, q: Point) -> f64 {
    super::super::angle_between(p, q)
}

/// Circles of the inverted inputs: at height `top − k · 2·top/(n+1)`, at the x `x(y)`.
pub(super) fn input_circles(v: &mut Vec<Segment>, cm: &Common, top: f64, x: impl Fn(f64) -> f64) {
    let n = cm.inputs.len();
    let step = 2.0 * top / (n + 1) as f64;
    for (k, inv) in cm.inputs.iter().enumerate() {
        let y = top - (k + 1) as f64 * step;
        if *inv {
            v.extend(ellipse_path(cm.c.plus(Point::new(x(y), y)), cm.r, cm.r));
        }
    }
}

/// Where a gate's inputs sit: one name if `single`, else `input 1`…, at height `half` and the x
/// `at_x(inverted, y)` gives.
pub(super) struct InputSites<F> {
    pub(super) single: bool,
    pub(super) half: f64,
    pub(super) at_x: F,
}

/// The input anchors `input 1`… (`input` alone for the gates with one).
pub(super) fn input_anchors<F>(g: &mut Geometry, cm: &Common, sites: InputSites<F>)
where
    F: Fn(bool, f64) -> f64,
{
    let InputSites { single, half, at_x } = sites;
    let n = cm.inputs.len();
    let step = 2.0 * half / (n + 1) as f64;
    for (k, inv) in cm.inputs.iter().enumerate() {
        let y = half - (k + 1) as f64 * step;
        let mut dx = at_x(*inv, y);
        if *inv {
            dx -= cm.r + cm.outer_r;
        }
        let name = if single { "input".to_string() } else { format!("input {}", k + 1) };
        g.anchor_at(&name, cm.c.plus(Point::new(dx, y)));
    }
}

// ------------------------------------------------------------------------------------------ US

/// The US gates' `\dimensions`: `\halfside`, `\halfwidth`, `\halfheight` and (but for the and gates)
/// `\tipanchor`, relative to the center.
pub(super) struct Us {
    pub(super) h: f64,
    pub(super) w: f64,
    pub(super) hh: f64,
    pub(super) ox: f64,
    pub(super) oy: f64,
    pub(super) tip: Point,
}

pub(super) fn us_dimensions(input: &Input, cm: &Common, and: bool) -> Us {
    let t = input.text();
    let mut x = input.inner_xsep + 0.5 * t.wd;
    let mut y = input.inner_ysep + 0.5 * (t.ht + t.dp);
    let n1 = (cm.inputs.len() + 1) as f64;
    let ya = (cm.r + cm.lw).max(0.5 * cm.sep) * n1;
    y = y.max(ya);
    x = x.max(y);
    y = x;
    y = y.max(0.5 * input.minimum_height);
    if and {
        if 2.5 * x < input.minimum_width {
            x = 0.4 * input.minimum_width;
        }
    } else if x + 1.732051 * y < input.minimum_width {
        x = 0.366025 * input.minimum_width;
    }
    let h = x.max(y);
    let (ox, oy) = (input.outer_xsep, input.outer_ysep);
    let tip = Point::new(-0.16666 * h + 0.866025 * 2.0 * h + 1.154701 * ox, 0.0);
    Us { h, w: h + ox, hh: h + oy, ox, oy, tip }
}
