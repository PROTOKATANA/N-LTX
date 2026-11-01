//! Tips declared with `\pgfarrowsdeclare` (the `arrows` library).

use crate::canvas::{Cap, Join};
use crate::scene::{Point, Segment};

use super::keys::Widths;
use super::spec::{Old, DECLARED};
use super::{Cubic, Geometry, Pen, Shape};

/// `direction ee` of `pgflibraryshapes.gates.ee`: a triangle `d` long, filled and stroked with
/// round joins, from `-d/2` to `d/2`.
pub(in crate::engine::arrows) fn direction_ee(d: f64, lw: Widths) -> Geometry {
    let mut p = Pen::default();
    let k = 0.38268343237 * d;
    p.m(-0.5 * d, k).l(0.5 * d, 0.0).l(-0.5 * d, -k).z();
    let shapes = vec![Shape { segs: p.take(), fill: true, stroke: true, width: lw.line, cap: None, join: Some(Join::Round) }];
    Geometry { tip: 0.5 * d, back: -0.5 * d, line: 0.0, visual_tip: 0.5 * d, visual_back: -0.5 * d, hull: Vec::new(), shapes }
}

/// How an `arrows`-library tip is stroked. Stack values only: the same fields the push took.
struct StrokeStyle {
    fill: bool,
    stroke: bool,
    width: f64,
    cap: Option<Cap>,
    join: Option<Join>,
}

/// One legacy tip while it is drawn.
struct OldDraw {
    o: Old,
    lw: Widths,
    t: f64,
    visible: f64,
    pen: Pen,
    shapes: Vec<Shape>,
}

fn polar(a: f64, r: f64) -> (f64, f64) {
    let (s, c) = a.to_radians().sin_cos();
    (r * c, r * s)
}

impl OldDraw {
    fn push(&mut self, segs: Vec<Segment>, style: StrokeStyle) {
        self.shapes.push(Shape { segs, fill: style.fill, stroke: style.stroke, width: style.width, cap: style.cap, join: style.join });
    }
    fn old_stealth(&mut self) -> (f64, f64) {
        let a = 0.28 + 0.3 * self.visible;
        self.pen.m(5.0 * a, 0.0).l(-3.0 * a, 4.0 * a).l(0.0, 0.0).l(-3.0 * a, -4.0 * a);
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: true, stroke: false, width: self.t, cap: None, join: None });
        (-3.0 * a, 5.0 * a)
    }
    fn old_to(&mut self) -> (f64, f64) {
        use Old::*;
        let a = 0.28 + 0.3 * self.t;
        if self.o == To {
            self.pen.m(-3.0 * a, 4.0 * a).c(Cubic::new(Point::new(-2.75 * a, 2.5 * a), Point::new(0.0, 0.25 * a), Point::new(0.75 * a, 0.0)));
            self.pen.c(Cubic::new(Point::new(0.0, -0.25 * a), Point::new(-2.75 * a, -2.5 * a), Point::new(-3.0 * a, -4.0 * a)));
        } else {
            self.pen.m(3.5 * a, 4.0 * a).c(Cubic::new(Point::new(3.25 * a, 2.5 * a), Point::new(0.5 * a, 0.25 * a), Point::new(-0.25 * a, 0.0)));
            self.pen.c(Cubic::new(Point::new(0.5 * a, -0.25 * a), Point::new(3.25 * a, -2.5 * a), Point::new(3.5 * a, -4.0 * a)));
        }
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: false, stroke: true, width: 0.8 * self.t, cap: Some(Cap::Round), join: Some(Join::Round) });
        if self.o == To {
            (-0.84 - 1.3 * self.t, 0.21 + 0.625 * self.t)
        } else {
            (-0.21 - 0.475 * self.t, 0.98 + 1.45 * self.t)
        }
    }
    fn old_latex(&mut self) -> (f64, f64) {
        let a = 0.28 + 0.3 * self.visible;
        self.pen.m(9.0 * a, 0.0).c(Cubic::new(Point::new(6.3333 * a, 0.5 * a), Point::new(2.0 * a, 2.0 * a), Point::new(-a, 3.75 * a))).l(-a, -3.75 * a);
        self.pen.c(Cubic::new(Point::new(2.0 * a, -2.0 * a), Point::new(6.3333 * a, -0.5 * a), Point::new(9.0 * a, 0.0)));
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: true, stroke: false, width: self.t, cap: None, join: None });
        (-a, 9.0 * a)
    }
    fn old_bar(&mut self) -> (f64, f64) {
        let a = 2.0 + 1.5 * self.t;
        self.pen.m(0.25 * self.t, -a).l(0.25 * self.t, a);
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: false, stroke: true, width: self.t, cap: Some(Cap::Square), join: None });
        (-0.25 * self.t, 0.75 * self.t)
    }
    fn old_square_bracket(&mut self) -> (f64, f64) {
        let a = 2.0 + 1.5 * self.t;
        let b = a + self.t;
        self.pen.m(-0.5 * b, -a).l(0.0, -a).l(0.0, a).l(-0.5 * b, a);
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: false, stroke: true, width: self.t, cap: Some(Cap::Butt), join: Some(Join::Miter) });
        (-(1.0 + 1.25 * self.t), 0.5 * self.t)
    }
    fn old_parenthesis(&mut self) -> (f64, f64) {
        let a = 2.0 + 1.5 * self.t;
        self.pen.m(-0.5 * a, -a).c(Cubic::new(Point::new(0.25 * a, -0.5 * a), Point::new(0.25 * a, 0.5 * a), Point::new(-0.5 * a, a)));
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: false, stroke: true, width: self.t, cap: Some(Cap::Round), join: None });
        (-(0.5 * a + 0.5 * self.t), 0.0625 * a + 0.5 * self.t)
    }
    fn old_angle(&mut self, deg: u8) -> (f64, f64) {
        use Old::*;
        let a = if matches!(self.o, Angle(_)) { 0.3 + 0.25 * self.t } else { 0.5 + 0.25 * self.t };
        let x = 0.5 * a;
        let (ext, front) = match deg {
            90 => {
                self.pen.m(-5.5 * a, -6.0 * a).l(x, 0.0).l(-5.5 * a, 6.0 * a);
                (5.5, 0.707)
            }
            60 => {
                let (dx, dy) = polar(150.0, 9.0 * a);
                self.pen.m(x + dx, dy).l(x, 0.0).l(x + dx, -dy);
                (7.29, 1.0)
            }
            _ => {
                let (dx, dy) = polar(157.0, 10.0 * a);
                self.pen.m(x + dx, dy).l(x, 0.0).l(x + dx, -dy);
                (8.705, 1.28)
            }
        };
        if matches!(self.o, Angle(_)) {
            let segs = self.pen.take();
            self.push(segs, StrokeStyle { fill: false, stroke: true, width: self.t, cap: Some(Cap::Round), join: Some(Join::Miter) });
        } else {
            self.pen.z();
            let segs = self.pen.take();
            self.push(segs, StrokeStyle { fill: true, stroke: true, width: self.t, cap: None, join: Some(Join::Miter) });
        }
        (-(ext * a + 0.5 * self.t), 0.5 * a + front * self.t)
    }
    fn old_open_triangle(&mut self, deg: u8) -> (f64, f64) {
        use Old::*;
        let a = 0.5 + 0.25 * self.t;
        let rev = matches!(self.o, OpenTriangleReversed(_));
        let ext = match (deg, rev) {
            (90, false) => {
                self.pen.m(0.0, -6.0 * a).l(6.0 * a, 0.0).l(0.0, 6.0 * a);
                (-0.5 * self.t, 6.0 * a + 0.707 * self.t)
            }
            (90, true) => {
                self.pen.m(6.0 * a, -6.0 * a).l(0.0, 0.0).l(6.0 * a, 6.0 * a);
                (-0.707 * self.t, 6.0 * a + 0.5 * self.t)
            }
            (60, false) | (45, false) => {
                let (x, ang, r, f) = if deg == 60 { (7.794 * a, 150.0, 9.0 * a, 1.0) } else { (9.205 * a, 157.0, 10.0 * a, 1.28) };
                let (dx, dy) = polar(ang, r);
                self.pen.m(x + dx, dy).l(x, 0.0).l(x + dx, -dy);
                (-0.5 * self.t, x + f * self.t)
            }
            _ => {
                let (x, ang, r, f) = if deg == 60 { (7.794 * a, 30.0, 9.0 * a, 1.0) } else { (9.205 * a, 23.0, 10.0 * a, 1.28) };
                let (dx, dy) = polar(ang, r);
                self.pen.m(dx, dy).l(0.0, 0.0).l(dx, -dy);
                (-f * self.t, x + 0.5 * self.t)
            }
        };
        self.pen.z();
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: false, stroke: true, width: self.t, cap: None, join: Some(Join::Miter) });
        ext
    }
    fn old_dot(&mut self) -> (f64, f64) {
        use Old::*;
        let a = 0.4 + 0.2 * self.t;
        if self.o == Dot {
            self.pen.ellipse(-3.0 * a, 0.0, 4.5 * a, 4.5 * a);
            let segs = self.pen.take();
            self.push(segs, StrokeStyle { fill: true, stroke: true, width: self.t, cap: None, join: None });
            (-(7.5 * a + 0.5 * self.t), 1.5 * a + 0.5 * self.t)
        } else {
            self.pen.ellipse(4.5 * a, 0.0, 4.5 * a, 4.5 * a);
            let segs = self.pen.take();
            self.push(segs, StrokeStyle { fill: false, stroke: true, width: self.t, cap: None, join: None });
            (-0.5 * self.t, 9.0 * a + 0.5 * self.t)
        }
    }
    fn old_diamond(&mut self) -> (f64, f64) {
        use Old::*;
        let a = 0.4 + 0.275 * self.t;
        if self.o == Diamond {
            self.pen.m(a, 0.0).l(-6.0 * a, 4.0 * a).l(-13.0 * a, 0.0).l(-6.0 * a, -4.0 * a).z();
            let segs = self.pen.take();
            self.push(segs, StrokeStyle { fill: true, stroke: true, width: self.t, cap: None, join: Some(Join::Round) });
            (-(13.0 * a + 0.5 * self.t), a + 0.5 * self.t)
        } else {
            self.pen.m(14.0 * a, 0.0).l(7.0 * a, 4.0 * a).l(0.0, 0.0).l(7.0 * a, -4.0 * a).z();
            let segs = self.pen.take();
            self.push(segs, StrokeStyle { fill: false, stroke: true, width: self.t, cap: None, join: Some(Join::Round) });
            (-0.5 * self.t, 14.0 * a + 0.5 * self.t)
        }
    }
    fn old_square(&mut self) -> (f64, f64) {
        use Old::*;
        let a = 0.4 + 0.275 * self.t;
        if self.o == Square {
            self.pen.m(a, 4.0 * a).l(-7.0 * a, 4.0 * a).l(-7.0 * a, -4.0 * a).l(a, -4.0 * a).z();
            let segs = self.pen.take();
            self.push(segs, StrokeStyle { fill: true, stroke: true, width: self.t, cap: None, join: Some(Join::Round) });
            (-7.0 * a - 0.5 * self.t, a + 0.5 * self.t)
        } else {
            self.pen.m(8.0 * a, 4.0 * a).l(0.0, 4.0 * a).l(0.0, -4.0 * a).l(8.0 * a, -4.0 * a).z();
            let segs = self.pen.take();
            self.push(segs, StrokeStyle { fill: false, stroke: true, width: self.t, cap: None, join: Some(Join::Round) });
            (-0.5 * self.t, 8.0 * a + 0.5 * self.t)
        }
    }
    fn old_latex_prime(&mut self) -> (f64, f64) {
        let a = 0.28 + 0.3 * self.t;
        self.pen.m(6.0 * a, 0.0).c(Cubic::new(Point::new(3.5 * a, 0.5 * a), Point::new(-a, 1.5 * a), Point::new(-4.0 * a, 3.75 * a)));
        self.pen
            .c(Cubic::new(Point::new(-1.5 * a, a), Point::new(-1.5 * a, -a), Point::new(-4.0 * a, -3.75 * a)))
            .c(Cubic::new(Point::new(-a, -1.5 * a), Point::new(3.5 * a, -0.5 * a), Point::new(6.0 * a, 0.0)))
            .z();
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: true, stroke: false, width: self.t, cap: None, join: None });
        (-4.0 * a, 6.0 * a)
    }
    fn old_stealth_prime(&mut self) -> (f64, f64) {
        let a = 0.28 + 0.3 * self.t;
        self.pen.m(2.0 * a, 0.0).c(Cubic::new(Point::new(-0.5 * a, 0.5 * a), Point::new(-3.0 * a, 1.5 * a), Point::new(-6.0 * a, 3.25 * a)));
        self.pen
            .c(Cubic::new(Point::new(-3.0 * a, a), Point::new(-3.0 * a, -a), Point::new(-6.0 * a, -3.25 * a)))
            .c(Cubic::new(Point::new(-3.0 * a, -1.5 * a), Point::new(-0.5 * a, -0.5 * a), Point::new(2.0 * a, 0.0)))
            .z();
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: true, stroke: true, width: self.t, cap: None, join: Some(Join::Round) });
        (-(6.0 * a + 0.5 * self.t), 2.0 * a + 0.5 * self.t)
    }
    fn old_side_to(&mut self) -> (f64, f64) {
        use Old::*;
        let a = 0.28 + 0.3 * self.t;
        let s = if self.o == LeftTo { 1.0 } else { -1.0 };
        let u = 0.8 * self.t;
        self.pen.m(-3.0 * a, s * 4.0 * a).c(Cubic::new(Point::new(-2.75 * a, s * 2.5 * a), Point::new(0.0, s * 0.25 * a), Point::new(0.75 * a, 0.0)));
        self.pen.c(Cubic::new(Point::new(0.55 * a, -s * 0.125 * u), Point::new(0.5 * a, -s * 0.125 * u), Point::new(0.5 * a, -s * 0.125 * u))).l(0.0, -s * 0.125 * u);
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: false, stroke: true, width: u, cap: Some(Cap::Round), join: Some(Join::Round) });
        (-0.84 - 1.3 * self.t, 0.21 + 0.625 * self.t)
    }
    fn old_side_to_reversed(&mut self) -> (f64, f64) {
        use Old::*;
        let a = 0.28 + 0.3 * self.t;
        let s = if self.o == LeftToReversed { 1.0 } else { -1.0 };
        self.pen.m(0.5 * self.t, 0.0).l(-0.1 * self.t, 0.0);
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: false, stroke: true, width: self.t, cap: Some(Cap::Butt), join: Some(Join::Round) });
        let u = 0.8 * self.t;
        let x = 0.625 * u;
        self.pen.m(x + 3.75 * a, s * 4.0 * a).c(Cubic::new(Point::new(x + 3.5 * a, s * 2.5 * a), Point::new(x + 0.75 * a, s * 0.25 * a), Point::new(x, s * 0.125 * u)));
        self.pen.m(x + 3.75 * a, s * 4.0 * a).c(Cubic::new(Point::new(x + 3.5 * a, s * 2.5 * a), Point::new(x + 0.75 * a, s * 0.25 * a), Point::new(x, -s * 0.125 * u)));
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: false, stroke: true, width: u, cap: Some(Cap::Round), join: Some(Join::Round) });
        (-0.1 * self.t, 3.75 * a + 0.9 * self.t)
    }
    fn old_serif_cm(&mut self) -> (f64, f64) {
        let a = 0.4 + 0.45 * self.t;
        let x = 0.04 * self.t;
        self.pen
            .m(x - 0.75 * a, 0.5 * self.t)
            .c(Cubic::new(Point::new(x - 0.375 * a, 0.5 * self.t), Point::new(x - 0.375 * a, 0.7 * self.t), Point::new(x - 0.375 * a, 1.95 * a)))
            .l(x, 1.95 * a);
        self.pen.c(Cubic::new(Point::new(x - 0.04 * self.t, 0.5 * a), Point::new(x - 0.04 * self.t, -0.5 * a), Point::new(x, -1.95 * a))).l(x - 0.375 * a, -1.95 * a);
        self.pen.c(Cubic::new(Point::new(x - 0.375 * a, -0.7 * self.t), Point::new(x - 0.375 * a, -0.5 * self.t), Point::new(x - 0.75 * a, -0.5 * self.t))).z();
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: true, stroke: false, width: self.t, cap: None, join: None });
        (-0.75 * a, 0.04 * self.t)
    }
    fn old_round_cap(&mut self) -> (f64, f64) {
        self.pen.m(0.0, 0.0).l(0.5 * self.t, 0.0);
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: false, stroke: true, width: self.t, cap: Some(Cap::Round), join: None });
        (0.0, self.t)
    }
    fn old_butt_cap(&mut self) -> (f64, f64) {
        self.pen.m(-0.1 * self.t, 0.0).l(0.5 * self.t, 0.0);
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: false, stroke: true, width: self.t, cap: Some(Cap::Butt), join: None });
        (-0.1 * self.t, 0.5 * self.t)
    }
    fn old_triangle_cap(&mut self) -> (f64, f64) {
        self.pen.m(-0.1 * self.t, 0.5 * self.t).l(0.5 * self.t, 0.5 * self.t).l(self.t, 0.0).l(0.5 * self.t, -0.5 * self.t).l(-0.1 * self.t, -0.5 * self.t).z();
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: true, stroke: false, width: self.t, cap: None, join: None });
        (-0.1 * self.t, self.t)
    }
    fn old_triangle_cap_reversed(&mut self) -> (f64, f64) {
        self.pen.m(self.t, 0.5 * self.t).l(-0.1 * self.t, 0.5 * self.t).l(-0.1 * self.t, -0.5 * self.t).l(self.t, -0.5 * self.t).l(0.5 * self.t, 0.0).z();
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: true, stroke: false, width: self.t, cap: None, join: None });
        (-0.1 * self.t, self.t)
    }
    fn old_fast_cap(&mut self) -> (f64, f64) {
        self.pen.m(-0.1 * self.t, 0.5 * self.t).l(0.5 * self.t, 0.5 * self.t).l(self.t, 0.0).l(0.5 * self.t, -0.5 * self.t).l(-0.1 * self.t, -0.5 * self.t).z();
        self.pen.m(self.t, 0.5 * self.t).l(1.5 * self.t, 0.5 * self.t).l(2.0 * self.t, 0.0).l(1.5 * self.t, -0.5 * self.t).l(self.t, -0.5 * self.t).l(1.5 * self.t, 0.0).z();
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: true, stroke: false, width: self.t, cap: None, join: None });
        (-0.1 * self.t, 2.0 * self.t)
    }
    fn old_fast_cap_reversed(&mut self) -> (f64, f64) {
        self.pen.m(-0.1 * self.t, 0.5 * self.t).l(self.t, 0.5 * self.t).l(0.5 * self.t, 0.0).l(self.t, -0.5 * self.t).l(-0.1 * self.t, -0.5 * self.t).z();
        self.pen.m(1.5 * self.t, 0.5 * self.t).l(2.0 * self.t, 0.5 * self.t).l(1.5 * self.t, 0.0).l(2.0 * self.t, -0.5 * self.t).l(1.5 * self.t, -0.5 * self.t).l(self.t, 0.0).z();
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: true, stroke: false, width: self.t, cap: None, join: None });
        (-0.1 * self.t, 2.0 * self.t)
    }
    fn old_implies(&mut self) -> (f64, f64) {
        let a = 0.25 * self.t + 0.25 * self.lw.inner;
        let b = 0.5 * self.t - 0.5 * self.lw.inner;
        let x = 0.06 * a;
        self.pen.m(x - 1.4 * a, 2.65 * a).c(Cubic::new(Point::new(x - 0.75 * a, 1.25 * a), Point::new(x + a, 0.05 * a), Point::new(x + 2.0 * a, 0.0)));
        self.pen.c(Cubic::new(Point::new(x + a, -0.05 * a), Point::new(x - 0.75 * a, -1.25 * a), Point::new(x - 1.4 * a, -2.65 * a)));
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: false, stroke: true, width: b, cap: Some(Cap::Round), join: Some(Join::Round) });
        (-1.36 * a - 0.5 * b, 2.06 * a + 0.5 * b)
    }

    fn old_hook(&mut self, a: f64, s: f64) {
        self.pen.c(Cubic::new(Point::new(2.415 * a, 0.0), Point::new(3.75 * a, s * 1.665 * a), Point::new(3.75 * a, s * 3.0 * a)));
        self.pen.c(Cubic::new(Point::new(3.75 * a, s * 4.665 * a), Point::new(2.415 * a, s * 6.0 * a), Point::new(0.75 * a, s * 6.0 * a)));
    }

    fn old_hooks(&mut self) -> (f64, f64) {
        use Old::*;
        let t = self.t;
        let a = 0.4 + 0.2 * t;
        self.pen.m(0.0, 0.0).l(0.75 * a, 0.0);
        self.old_hook(a, if self.o == RightHook { -1.0 } else { 1.0 });
        if self.o == Hooks {
            self.pen.m(0.75 * a, 0.0);
            self.old_hook(a, -1.0);
        }
        let segs = self.pen.take();
        self.push(segs, StrokeStyle { fill: false, stroke: true, width: t, cap: Some(Cap::Round), join: None });
        (-0.5 * t, 3.75 * a + 0.5 * t)
    }
}

/// The tips declared with `\pgfarrowsdeclare`: their extents (left and right) and drawing; the line
/// ends at 0 and they have no hull, so they leave the bounding box alone.
pub(in crate::engine::arrows) fn old(o: Old, lw: Widths, _reversed: bool) -> Geometry {
    use Old::*;
    if let User(i) = o {
        let tip = DECLARED.with(|d| d.borrow().get(i as usize).cloned());
        let Some(tip) = tip else {
            return Geometry { tip: 0.0, back: 0.0, line: 0.0, visual_tip: 0.0, visual_back: 0.0, hull: Vec::new(), shapes: Vec::new() };
        };
        let shapes = tip.paths.into_iter().map(|(segs, fill, stroke, width)| Shape { segs, fill, stroke, width, cap: None, join: None }).collect();
        return Geometry { tip: tip.right, back: tip.left, line: 0.0, visual_tip: tip.right, visual_back: tip.left, hull: Vec::new(), shapes };
    }
    let t = lw.line;
    let visible = if lw.inner > 0.0 { 0.6 * t - 0.4 * lw.inner } else { t };
    let mut d = OldDraw { o, lw, t, visible, pen: Pen::default(), shapes: Vec::new() };
    let (back, tip) = match o {
        Stealth => d.old_stealth(),
        To | ToReversed => d.old_to(),
        Latex => d.old_latex(),
        Bar => d.old_bar(),
        SquareBracket => d.old_square_bracket(),
        Parenthesis => d.old_parenthesis(),
        Angle(deg) | Triangle(deg) => d.old_angle(deg),
        OpenTriangle(deg) | OpenTriangleReversed(deg) => d.old_open_triangle(deg),
        Dot | Ring => d.old_dot(),
        Diamond | OpenDiamond => d.old_diamond(),
        Square | OpenSquare => d.old_square(),
        LatexPrime => d.old_latex_prime(),
        StealthPrime => d.old_stealth_prime(),
        LeftTo | RightTo => d.old_side_to(),
        LeftToReversed | RightToReversed => d.old_side_to_reversed(),
        LeftHook | RightHook | Hooks => d.old_hooks(),
        SerifCm => d.old_serif_cm(),
        RoundCap => d.old_round_cap(),
        ButtCap => d.old_butt_cap(),
        Triangle90Cap => d.old_triangle_cap(),
        Triangle90CapReversed => d.old_triangle_cap_reversed(),
        FastCap => d.old_fast_cap(),
        FastCapReversed => d.old_fast_cap_reversed(),
        Implies => d.old_implies(),
        Space => (0.0, 0.88 + 0.3 * t),
        DirectionEe | User(_) => (0.0, 0.0),
    };
    Geometry { tip, back, line: 0.0, visual_tip: tip, visual_back: back, hull: Vec::new(), shapes: d.shapes }
}
