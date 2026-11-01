use std::cell::{Cell, RefCell};

use tikra_engine::scene::{Point, Transform};
use tikra_engine::tex::Token;

/// Which coordinate system a plain `(x,y)` means while the axis is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cs {
    Plain,
    Axis,
    Description,
}

/// The axis as the coordinate systems see it: where its box is and how data maps to the canvas.
#[derive(Debug)]
pub struct Frame {
    /// Canvas point of the logical lower limits.
    pub origin: Point,
    /// Canvas cm per logical unit, for x, y and z.
    pub e: [Point; 3],
    pub min: [f64; 3],
    pub max: [f64; 3],
    /// Logarithmic axes keep natural logarithms.
    pub log: [bool; 3],
    pub three_d: bool,
    pub zero: [f64; 3],
    pub cs: Cell<Cs>,
    /// The axis whose label is being placed (`ticklabel cs`, `near ticklabel`).
    pub label_axis: Cell<usize>,
    /// The largest distance from a tick to its label, outwards (cm).
    pub tick_dim: RefCell<[f64; 3]>,
    /// Where tick labels are: -1 lower, 1 upper, 0 at the other axis' zero.
    pub label_side: [i8; 3],
    /// `xlabel shift`
    pub label_shift: [f64; 3],
    /// The range of point meta (axis wide) and the colormap it maps to.
    pub meta: [f64; 2],
    pub colormap: Option<crate::colormap::Colormap>,
    /// 3d: the box's foreground faces (`[axis][lower, upper]`), the lines with the tick labels
    /// (pgfplots' three-character strings like `v00`) and those lines for a boxed axis.
    pub fg: [[bool; 2]; 3],
    pub label_line: [crate::cube::Part; 3],
    pub box_line: [crate::cube::Part; 3],
    /// The data's range (merged with given limits), logical.
    pub data_range: [[f64; 2]; 3],
    /// `polaraxis`: x is an angle in degrees, y a radius; `e[0]`, `e[1]` are the cartesian
    /// unit vectors and `origin` the center.
    pub polar: bool,
    /// `ternaryaxis`: points are placed by `ternary::at`; `e` are the triangle's edges per
    /// logical unit. Tick labels are percentages of the ranges with relative limits.
    pub ternary: bool,
    pub ternary_relative: bool,
    /// `smithchart`: points at their reflection coefficient, `origin` the center and `e` the
    /// radii (x negative for `smithchart mirrored`).
    pub smith: bool,
    /// While a tick label is placed: the direction `sloped like y axis` follows (a smith chart's
    /// circle there), and the anchor code it keeps for an upside down label.
    pub slope: Cell<Option<Point>>,
    pub upside_down: RefCell<Option<Vec<Token>>>,
    /// `x dir=reverse` and its friends.
    pub reversed: [bool; 3],
}

impl Frame {
    pub fn logical(&self, a: usize, v: f64) -> f64 {
        if self.log[a] {
            v.ln()
        } else {
            v
        }
    }

    /// The data coordinates of a canvas point of a 2d axis (`\pgfplotspointgetcoordinates`).
    pub fn data_at(&self, p: Point) -> Option<[f64; 2]> {
        if self.three_d || self.polar || self.ternary || self.smith {
            return None;
        }
        let (a, b) = (self.e[0], self.e[1]);
        let det = a.x * b.y - a.y * b.x;
        if det.abs() < 1e-12 {
            return None;
        }
        let d = p.minus(self.origin);
        let (u, v) = ((d.x * b.y - d.y * b.x) / det, (a.x * d.y - a.y * d.x) / det);
        let l = [u + self.min[0], v + self.min[1]];
        Some([0, 1].map(|k| if self.log[k] { l[k].exp() } else { l[k] }))
    }

    /// The canvas point of logical coordinates.
    pub fn at(&self, l: [f64; 3]) -> Point {
        if self.ternary {
            return crate::ternary::at(self, l);
        }
        if self.smith {
            return crate::smith::at(self, l);
        }
        if self.polar {
            let (s, c) = crate::polar::sin_cos(l[0]);
            return self.origin.plus(self.e[0].scale(l[1] * c)).plus(self.e[1].scale(l[1] * s));
        }
        let mut p = self.origin;
        for (i, &v) in l.iter().enumerate() {
            if i == 2 && !self.three_d {
                continue;
            }
            p = p.plus(self.e[i].scale(v - self.min[i]));
        }
        p
    }

    pub fn data(&self, x: f64, y: f64, z: f64) -> Point {
        self.at([self.logical(0, x), self.logical(1, y), if self.three_d { self.logical(2, z) } else { self.min[2] }])
    }

    /// `rel axis cs`: fractions of the ranges.
    pub fn rel(&self, r: [f64; 3]) -> Point {
        let l = [0, 1, 2].map(|i| self.min[i] + r[i] * (self.max[i] - self.min[i]));
        self.at(l)
    }

    /// The lower left and upper right corners of the box as drawn (a 3d box's projection).
    pub fn bbox(&self) -> (Point, Point) {
        let mut lo = Point::new(f64::INFINITY, f64::INFINITY);
        let mut hi = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        if self.smith {
            return crate::smith::bbox(self);
        }
        if self.ternary {
            for p in crate::ternary::corners(self) {
                lo = Point::new(lo.x.min(p.x), lo.y.min(p.y));
                hi = Point::new(hi.x.max(p.x), hi.y.max(p.y));
            }
            return (lo, hi);
        }
        if self.polar {
            for p in crate::polar::outline(self) {
                lo = Point::new(lo.x.min(p.x), lo.y.min(p.y));
                hi = Point::new(hi.x.max(p.x), hi.y.max(p.y));
            }
            return (lo, hi);
        }
        for i in 0..8 {
            if !self.three_d && i >= 4 {
                break;
            }
            let p = self.rel([(i & 1) as f64, ((i >> 1) & 1) as f64, ((i >> 2) & 1) as f64]);
            lo = Point::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point::new(hi.x.max(p.x), hi.y.max(p.y));
        }
        (lo, hi)
    }

    /// `axis description cs`: (0,0) the lower left corner of the box as drawn, (1,1) the upper right
    /// one (`\pgfplotspointdescriptionxy`). pgfplots takes the corners of the bounding box of the
    /// axis, so with skewed unit vectors (`x={(5pt,1pt)}`) these are not the axis' corners.
    pub fn desc(&self, x: f64, y: f64) -> Point {
        let (lo, hi) = self.bbox();
        Point::new(lo.x + x * (hi.x - lo.x), lo.y + y * (hi.y - lo.y))
    }

    /// The outward unit normal of the line that holds axis `a`'s tick labels.
    pub fn normal(&self, a: usize) -> Point {
        if self.ternary {
            return crate::ternary::normal(self, a);
        }
        if self.smith {
            // at the tick being labelled: outwards of the circle
            if let (1, Some(u)) = (a, self.slope.get()) {
                return Point::new(u.y, -u.x);
            }
            return crate::smith::normal(self, a);
        }
        if self.three_d {
            return crate::cube::outer_normal(&self.e, self.label_line[a], self.box_line[a]);
        }
        let b = if a == 0 { 1 } else { 0 };
        let d = self.e[b];
        let u = unit(d);
        let side = if self.label_side[a] > 0 { 1.0 } else { -1.0 };
        // e[b] points to growing b: the lower side is outward against it
        let sign = if self.max[b] >= self.min[b] { 1.0 } else { -1.0 };
        u.scale(side * sign)
    }

    /// The direction axis `a` grows in on paper (at the tick being labelled, for a smith chart's
    /// circle).
    fn direction(&self, a: usize) -> Point {
        if self.smith && a == 1 {
            return self.slope.get().unwrap_or(Point::new(0.0, -1.0));
        }
        unit(self.e[a])
    }

    /// pgfplots' `near ticklabel` anchors (`\pgfplots@borderanchor@for@axis@`): in the node's own
    /// (possibly rotated) coordinates, the compass anchor that faces the axis is snapped to, then
    /// moved along the line through it parallel to the axis to where the line from the node's
    /// `near ticklabel align` anchor (center, by default) towards the axis meets it.
    /// `local` gives the node's named anchors.
    pub fn near_point(&self, name: &str, rotation: &Transform, align: &str, local: &dyn Fn(&str) -> Option<Point>) -> Option<Point> {
        let rest = name.strip_prefix("near ")?;
        let (axis_part, opposite) = match rest.strip_suffix(" opposite") {
            Some(r) => (r, true),
            None => (rest, false),
        };
        let a = match axis_part {
            "ticklabel" => self.label_axis.get(),
            "xticklabel" => 0,
            "yticklabel" => 1,
            "zticklabel" => 2,
            _ => return None,
        };
        let lin = Transform { e: 0.0, f: 0.0, ..*rotation }.inverse();
        // towards the axis (away from it for `opposite`)
        let n = lin.apply(self.normal(a).scale(if opposite { 1.0 } else { -1.0 }));
        let d = lin.apply(self.direction(a));
        let snap = |v: Point| -> &'static str {
            const T: f64 = 0.17;
            match (v.y > T, v.y < -T, v.x > T, v.x < -T) {
                (true, _, true, _) => "north east",
                (true, _, _, true) => "north west",
                (true, ..) => "north",
                (_, true, true, _) => "south east",
                (_, true, _, true) => "south west",
                (_, true, ..) => "south",
                (.., true, _) => "east",
                (.., true) => "west",
                _ => "center",
            }
        };
        let reversed = self.reversed[a];
        let inner = match align.trim() {
            "inside" => snap(if reversed { d.scale(-1.0) } else { d }),
            "outside" => snap(if reversed { d } else { d.scale(-1.0) }),
            _ => "center",
        };
        let at = local(snap(n))?;
        let from = local(inner)?;
        let det = n.x * d.y - n.y * d.x;
        if (n.x * d.x + n.y * d.y).abs() >= 0.8 || det.abs() < 1e-12 {
            return Some(at);
        }
        // from + r·n = at + s·d
        let rhs = at.minus(from);
        let r = (rhs.x * d.y - rhs.y * d.x) / det;
        Some(from.plus(n.scale(r)))
    }
}

/// A plot's logical coordinates (after stacking) and its stack's base; None for TikZ statements.
pub type PlotPoints = Option<(Vec<[f64; 3]>, Vec<f64>)>;

pub(in crate::draw) fn clone_frame(f: &Frame) -> Frame {
    Frame {
        origin: f.origin,
        e: f.e,
        min: f.min,
        max: f.max,
        log: f.log,
        three_d: f.three_d,
        zero: f.zero,
        cs: Cell::new(f.cs.get()),
        label_axis: Cell::new(f.label_axis.get()),
        tick_dim: RefCell::new(*f.tick_dim.borrow()),
        label_side: f.label_side,
        fg: f.fg,
        label_line: f.label_line,
        box_line: f.box_line,
        data_range: f.data_range,
        polar: f.polar,
        ternary: f.ternary,
        ternary_relative: f.ternary_relative,
        smith: f.smith,
        slope: Cell::new(f.slope.get()),
        upside_down: RefCell::new(f.upside_down.borrow().clone()),
        reversed: f.reversed,
        label_shift: f.label_shift,
        meta: f.meta,
        colormap: f.colormap.clone(),
    }
}

pub(crate) fn unit(p: Point) -> Point {
    // tiny vectors are not zero: an axis up to 10^21 has a unit vector of 10^-20 pt
    let n = p.norm();
    if n > 0.0 {
        p.scale(1.0 / n)
    } else {
        p
    }
}
