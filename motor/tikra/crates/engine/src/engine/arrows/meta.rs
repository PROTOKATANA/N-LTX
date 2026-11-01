//! The tips of `arrows.meta`.

use crate::canvas::{Cap, Join};
use crate::scene::{Point, Segment};

use super::keys::{Keys, Widths};
use super::spec::Kind;
use super::{map_segments, ratio, Cubic, Geometry, Pen, Shape};

/// The tips of arrows.meta.
fn tip_cap(round: bool) -> Option<Cap> {
    Some(if round { Cap::Round } else { Cap::Butt })
}
fn tip_join(round: bool) -> Option<Join> {
    Some(if round { Join::Round } else { Join::Miter })
}
/// The mutable drawing of one `arrows.meta` tip. Lengths are in pt, in the tip's frame.
struct MetaDraw<'a> {
    keys: &'a Keys,
    lw: Widths,
    l: f64,
    w: f64,
    ins: f64,
    alw: f64,
    h: bool,
    rev: bool,
    rc: bool,
    rj: bool,
    hull: Vec<Point>,
    pen: Pen,
    visual_back: Option<f64>,
    shapes: Vec<Shape>,
    tip: f64,
    back: f64,
    line: f64,
}

impl MetaDraw<'_> {
    /// Both sides of the tip, unless it is a harpoon.
    fn upper(&mut self, x: f64, y: f64) {
        self.hull.push(Point::new(x, y));
        if y > 0.0 && !self.h {
            self.hull.push(Point::new(x, -y));
        }
    }

    fn hy(&self, y: f64) -> f64 {
        if self.h {
            -0.5 * self.alw
        } else {
            y
        }
    }

    fn push_barb(&mut self, segs: Vec<Segment>, width: f64, round_cap: bool) {
        let join = tip_join(self.rj);
        self.shapes.push(Shape { segs, fill: false, stroke: true, width, cap: tip_cap(round_cap), join });
    }

    fn push_closed(&mut self, segs: Vec<Segment>) {
        let alw = self.alw;
        let open = self.keys.open;
        let join = tip_join(self.rj);
        self.shapes.push(Shape { segs, fill: !open, stroke: open || alw > 0.0, width: alw, cap: None, join });
    }

    fn straight_barb(&mut self) {
        let q = ratio(self.l, self.w);
        let xc = 0.5 * (4.0 * q * q + 1.0).sqrt() * self.alw;
        let xa = q * self.alw;
        self.tip = if self.rj { self.l + 0.5 * self.alw } else { self.l + xc + if self.h { xa } else { 0.0 } };
        self.visual_back = Some(self.l + 0.5 * self.alw);
        self.back = -0.5 * self.alw;
        self.line = if self.rev { self.l + if self.h { 0.5 * self.alw } else { 0.0 } } else { self.l - 0.5 * self.alw };
        {
            let y = self.hy(0.0);
            self.hull.push(Point::new(if self.rj { self.l + 0.5 * self.alw } else { self.l + xc + xa }, y));
        }
        self.upper(0.5 * self.alw, 0.5 * self.w + 0.5 * self.alw);
        self.upper(-0.5 * self.alw, 0.5 * self.w + 0.5 * self.alw);
        if self.h {
            self.hull.push(Point::new(-0.5 * self.alw, -0.5 * self.alw));
        }
        self.pen.m(0.0, 0.5 * self.w).l(self.l, 0.0);
        if self.h {
            self.pen.l(if self.rev { self.l + self.alw } else { self.l - self.alw }, 0.0);
        } else {
            self.pen.l(0.0, -0.5 * self.w);
        }
        {
            let segs = self.pen.take();
            let width = self.alw;
            let round = self.rc;
            self.push_barb(segs, width, round);
        }
    }
    fn hook_barb(&mut self) {
        let arc = self.keys.arc;
        if self.h {
            self.hull.push(Point::new(0.5 * self.l, -0.5 * self.alw));
            self.hull.push(Point::new(self.l, -0.5 * self.alw + 0.125 * self.w));
        }
        if arc < 90.1 {
            let y = 0.25 * self.w - if self.rc { 0.0 } else { 0.25 * self.alw };
            self.upper(0.0, y);
            self.upper(self.l, y);
        } else {
            self.upper(self.l, 0.375 * self.w);
            self.upper(0.5 * self.l, 0.5 * self.w);
            if arc > 180.0 {
                self.upper(-0.5 * self.l, 0.5 * self.w);
                self.upper(-self.l, 0.375 * self.w);
                self.upper(-self.l, 0.25 * self.w);
            } else {
                let x = if self.rc { -0.5 * self.alw } else { 0.0 };
                self.upper(x, 0.25 * self.w);
                self.upper(x, 0.5 * self.w);
            }
        }
        self.l -= 0.5 * self.alw;
        self.w -= self.alw;
        let mut b = if self.rev && self.rj { -0.5 * self.alw } else { 0.0 };
        let t;
        if arc < 90.0 {
            t = arc.to_radians().sin() * self.l + 0.5 * self.alw;
        } else if arc < 180.0 {
            t = self.l + 0.5 * self.alw;
            if self.rc {
                b = -0.5 * self.alw;
            }
        } else if arc < 270.0 {
            t = self.l + 0.5 * self.alw;
            if arc > 180.0 {
                b = arc.to_radians().sin() * self.l - 0.5 * self.alw;
            } else if self.rc {
                b = -0.5 * self.alw;
            }
        } else {
            t = self.l + 0.5 * self.alw;
            b = -self.l - 0.5 * self.alw;
        }
        self.tip = t;
        self.back = b;
        self.line = if self.rev {
            0.5 * self.alw
        } else if self.h {
            0.0
        } else {
            0.25 * self.alw
        };
        let a = arc - 90.0;
        let (s, c) = a.to_radians().sin_cos();
        self.pen.m(c, s + 1.0).arc(a, -90.0, 1.0, 1.0);
        if !self.h {
            self.pen.arc(90.0, -a, 1.0, 1.0);
        }
        let mut segs = map_segments(&self.pen.take(), |q| Point::new(q.x * self.l, q.y * 0.25 * self.w));
        if self.h && self.rev {
            segs.push(Segment::LineTo(Point::new(self.alw, 0.0)));
        }
        {
            let width = self.alw;
            let round = self.rc;
            self.push_barb(segs, width, round);
        }
    }
    fn arc_barb(&mut self) {
        let ah = 0.5 * self.keys.arc;
        self.tip = self.l;
        self.visual_back = Some(self.l);
        self.line = self.l - 0.5 * self.alw;
        self.upper(self.l, 0.25 * self.w);
        if self.h {
            self.hull.push(Point::new(self.l, -0.5 * self.lw.line));
        }
        let (sn, cs) = ah.to_radians().sin_cos();
        let xa = 0.5 * self.w;
        self.upper(cs * self.l, sn * xa);
        if ah > 60.0 {
            self.upper(0.5 * self.l, if ah < 90.0 { sn * xa } else { 0.5 * self.w });
        }
        if ah > 90.0 {
            if ah < 120.0 {
                self.upper(cs * self.l, 0.5 * self.w);
            } else {
                self.upper(-0.5 * self.l, 0.5 * self.w);
                if ah > 150.0 {
                    self.upper(-self.l, 0.25 * self.w);
                }
            }
        }
        if ah < 90.0 || self.h {
            self.upper(cs * (self.l - self.alw), sn * (0.5 * self.w - self.alw));
        }
        let x = if self.rc {
            cs * (self.l - 0.5 * self.alw)
        } else if ah < 90.0 {
            cs * (self.l - self.alw)
        } else {
            cs * self.l
        };
        self.back = x - if self.rc { 0.5 * self.alw } else { 0.0 };
        self.l -= 0.5 * self.alw;
        self.w -= self.alw;
        self.pen.m(cs, sn).arc(ah, if self.h { 0.0 } else { -ah }, 1.0, 1.0);
        let mut segs = map_segments(&self.pen.take(), |q| Point::new(q.x * self.l, q.y * 0.5 * self.w));
        if self.h {
            segs.push(Segment::LineTo(Point::new(self.l + if self.rev { 0.5 } else { -0.5 } * self.alw, 0.0)));
        }
        {
            let width = self.alw;
            let round = self.rc;
            self.push_barb(segs, width, round);
        }
    }
    fn tee_barb(&mut self) {
        let (mut da, mut sa) = (self.l - self.ins, false);
        if da < 0.5 * self.alw {
            da = 0.5 * self.alw;
            sa = true;
        }
        let (mut db, mut sb) = (-self.ins, false);
        if db > -0.5 * self.alw {
            db = -0.5 * self.alw;
            sb = true;
        }
        self.tip = da + if self.rc && !sa { 0.5 * self.alw } else { 0.0 };
        self.back = db - if self.rc && !sb { 0.5 * self.alw } else { 0.0 };
        self.line = if self.rev { 0.25 * self.alw } else { -0.25 * self.alw };
        self.visual_back = Some(0.5 * self.alw);
        self.upper(self.tip, 0.5 * self.w);
        self.upper(self.back, 0.5 * self.w);
        if self.h {
            self.hull.push(Point::new(0.5 * self.alw, -0.5 * self.alw));
        }
        let yc = self.lw.line;
        let top = 0.5 * self.w - 0.5 * self.alw;
        if ((da - 0.5 * self.alw) - (db + 0.5 * self.alw)).abs() < 1e-9 {
            self.pen.m(0.0, 0.5 * self.w).l(0.0, if self.h { -0.5 * self.alw } else { -0.5 * self.w });
        } else if (da - 0.5 * self.alw).abs() < 1e-9 {
            self.pen.m(db, top).l(0.0, top);
            if self.h {
                self.pen.l(0.0, -0.5 * yc);
            } else {
                self.pen.l(0.0, -top).l(db, -top);
            }
        } else {
            self.pen.m(db, top).l(da, top).m(0.0, top);
            if self.h {
                self.pen.l(0.0, -0.5 * yc);
            } else {
                self.pen.l(0.0, -top).m(db, -top).l(da, -top);
            }
        }
        {
            let segs = self.pen.take();
            let width = self.alw;
            let round = self.rc;
            self.push_barb(segs, width, round);
        }
    }
    fn classical_arrow(&mut self) {
        if self.h {
            self.rj = true;
        }
        let da = self.l - self.alw;
        let db = self.w - self.alw;
        let xc = if self.rj { 0.0 } else { 0.5 * (40.96 * ratio(da, db).powi(2) + 1.0).sqrt() * self.alw };
        self.tip = da + if self.rj { 0.5 * self.alw } else { xc };
        self.line = if self.rev {
            if self.h {
                da + 0.5 * self.lw.line
            } else {
                da - 0.25 * self.lw.line
            }
        } else {
            da - 0.6 * self.lw.line
        };
        self.back = -0.5 * self.alw;
        self.visual_back = Some(da + 0.5 * self.alw);
        self.hull.push(Point::new(self.tip, 0.5 * self.alw));
        self.hull.push(Point::new(self.tip, -0.5 * self.alw));
        self.upper(0.5 * self.alw, 0.5 * db + 0.5 * self.alw);
        self.upper(-0.5 * self.alw, 0.5 * db + 0.5 * self.alw);
        if self.h {
            self.hull.push(Point::new(da - self.lw.line, -0.5 * self.lw.line));
            self.hull.push(Point::new(-0.5 * self.alw, 0.5 * db - 0.5 * self.lw.line));
        }
        // in the drawing, \pgflinewidth is the tip's
        let t = self.alw;
        self.pen.m(0.0, 0.5 * db);
        if self.h && self.rev {
            self.pen.c(Cubic::new(Point::new(0.066666 * da, 0.3125 * db), Point::new(0.8 * da, 0.03125 * db), Point::new(da, 0.125 * t)));
            self.pen.m(0.0, 0.5 * db).c(Cubic::new(Point::new(0.066666 * da, 0.3125 * db), Point::new(0.8 * da, 0.03125 * db), Point::new(da, -0.125 * t)));
            {
                let segs = self.pen.take();
                let width = self.alw;
                let round = self.rc;
                self.push_barb(segs, width, round);
            }
            self.pen.m(da + 0.6 * self.lw.line, 0.0).l(da, 0.0);
            {
                let segs = self.pen.take();
                let width = self.lw.line;
                self.push_barb(segs, width, true);
            }
        } else if self.h {
            self.pen.c(Cubic::new(Point::new(0.066666 * da, 0.3125 * db), Point::new(0.8 * da, 0.03125 * db), Point::new(da, 0.0)));
            self.pen.c(Cubic::new(Point::new(0.95 * da, -0.125 * t), Point::new(0.933333 * da, -0.125 * t), Point::new(0.933333 * da, -0.125 * t)));
            self.pen.l(0.8 * da - 0.5 * t, -0.125 * t);
            {
                let segs = self.pen.take();
                let width = self.alw;
                let round = self.rc;
                self.push_barb(segs, width, round);
            }
        } else {
            self.pen.c(Cubic::new(Point::new(0.066666 * da, 0.3125 * db), Point::new(0.8 * da, 0.03125 * db), Point::new(da, 0.0)));
            self.pen.c(Cubic::new(Point::new(0.8 * da, -0.03125 * db), Point::new(0.066666 * da, -0.3125 * db), Point::new(0.0, -0.5 * db)));
            {
                let segs = self.pen.take();
                let width = self.alw;
                let round = self.rc;
                self.push_barb(segs, width, round);
            }
        }
    }
    fn cm_arrow(&mut self) {
        let da = self.l - self.alw;
        let db = self.w - self.alw;
        let (xc, xa) = if self.rj {
            (0.0, 0.0)
        } else {
            let q = ratio(da, db);
            (0.5 * (49.44662 * q * q + 1.0).sqrt() * self.alw, 3.51591 * q * self.alw)
        };
        self.tip = if self.rj { 0.5 * self.alw } else { xc + if self.h { xa } else { 0.0 } };
        self.line = if self.rev && self.h { 0.5 * self.alw } else { -0.5 * self.alw };
        self.back = -da - 0.5 * self.alw;
        self.visual_back = Some(0.5 * self.alw);
        self.hull.push(Point::new(self.tip, 0.5 * self.alw));
        self.hull.push(Point::new(self.tip, -0.5 * self.alw));
        self.upper(-da + 0.5 * self.alw, 0.5 * db + 0.5 * self.alw);
        self.upper(-da - 0.5 * self.alw, 0.5 * db + 0.5 * self.alw);
        if self.h {
            self.hull.push(Point::new(-self.alw, -0.5 * self.alw));
            self.hull.push(Point::new(-da - 0.5 * self.alw, 0.5 * db - 0.5 * self.alw));
        }
        self.pen.m(-da, 0.5 * db).c(Cubic::new(Point::new(-0.81731 * da, 0.2 * db), Point::new(-0.41019 * da, 0.05833333 * db), Point::new(0.0, 0.0)));
        if self.h {
            self.pen.l(if self.rev { 0.5 * self.alw } else { -self.alw }, 0.0);
        } else {
            self.pen.c(Cubic::new(Point::new(-0.41019 * da, -0.05833333 * db), Point::new(-0.81731 * da, -0.2 * db), Point::new(-da, -0.5 * db)));
        }
        {
            let segs = self.pen.take();
            let width = self.alw;
            let round = self.rc;
            self.push_barb(segs, width, round);
        }
    }
    fn implies_arrow(&mut self) {
        let da = 0.25 * self.lw.line + 0.25 * self.lw.inner;
        let db = 0.5 * self.lw.line - 0.5 * self.lw.inner;
        self.tip = 2.06 * da + 0.5 * db;
        self.back = -1.36 * da + 0.5 * db;
        self.line = 0.0;
        self.upper(-1.4 * da, 2.8 * da);
        self.upper(2.0 * da, 0.5 * db);
        let x = 0.06 * da;
        self.pen.m(x - 1.4 * da, 2.65 * da).c(Cubic::new(Point::new(x - 0.75 * da, 1.25 * da), Point::new(x + da, 0.05 * da), Point::new(x + 2.0 * da, 0.0)));
        self.pen.c(Cubic::new(Point::new(x + da, -0.05 * da), Point::new(x - 0.75 * da, -1.25 * da), Point::new(x - 1.4 * da, -2.65 * da)));
        {
            let segs = self.pen.take();
            self.push_barb(segs, db, true);
        }
    }
    fn latex_arrow(&mut self) {
        self.alw = self.alw.min(0.2 * self.l);
        let q = ratio(self.l, self.w);
        let xc = (9.0 * q * q + 1.0).sqrt() * self.alw;
        let xa = q * self.alw;
        let da = self.l - 0.5 * xc - 0.5 * self.alw;
        self.back = -0.5 * self.alw;
        self.line = if self.rev {
            if self.lw.inner > 0.0 {
                0.0
            } else {
                da - 0.5 * self.lw.line
            }
        } else {
            0.0
        };
        self.tip = if self.rj { da + 0.5 * self.alw } else { self.l - 0.5 * self.alw + if self.h { 1.5 * xa } else { 0.0 } };
        // the miter at the back corners
        let v = Point::new(0.3 * self.l, 0.2333333 * self.w);
        let n = v.scale(1.0 / v.norm().max(1e-12));
        let db = 0.5 * self.w - 0.5 * ratio(n.y + 1.0, n.x) * self.alw;
        {
            let y = self.hy(0.0);
            self.hull.push(Point::new(self.l - 0.5 * self.alw + if !self.rj && self.h { 1.5 * xa } else { 0.0 }, y));
        }
        self.hull.push(Point::new(-0.5 * self.alw, 0.5 * self.w - if self.h { 0.5 * self.alw } else { 0.0 }));
        {
            let y = self.hy(-0.5 * self.w);
            self.hull.push(Point::new(-0.5 * self.alw, y));
        }
        self.pen.m(da, 0.0).c(Cubic::new(Point::new(0.877192 * da, 0.077922 * db), Point::new(0.337381 * da, 0.519480 * db), Point::new(0.0, db)));
        if self.h {
            self.pen.l(0.0, 0.0);
        } else {
            self.pen.l(0.0, -db).c(Cubic::new(Point::new(0.337381 * da, -0.519480 * db), Point::new(0.877192 * da, -0.077922 * db), Point::new(da, 0.0)));
        }
        self.pen.z();
        {
            let segs = self.pen.take();
            self.push_closed(segs);
        }
    }
    fn stealth_arrow(&mut self) {
        self.alw = self.alw.min(0.25 * (self.l - self.ins));
        let q = ratio(self.l, self.w);
        let xc = 0.5 * (4.0 * q * q + 1.0).sqrt() * self.alw;
        let xa = q * self.alw;
        // the miters at the back corners and on top
        let half_w = 0.5 * self.w;
        let outer = self.l.atan2(half_w).to_degrees();
        let inner = self.ins.atan2(half_w).to_degrees();
        let half = 0.5 * (outer - inner);
        let yc = 0.5 * ratio(1.0, half.to_radians().tan()) * self.alw;
        let (s, c) = (inner + half).to_radians().sin_cos();
        let mut back_miter = s * yc;
        let top_miter = c * yc;
        if self.ins == 0.0 {
            back_miter = 0.5 * self.alw;
        }
        let q2 = ratio(self.ins, self.w);
        let inset_miter = 0.5 * (4.0 * q2 * q2 + 1.0).sqrt() * self.alw;
        let da = self.l - xc - back_miter;
        let db = 0.5 * self.w - top_miter;
        self.back = if self.rj { back_miter - 0.5 * self.alw } else { 0.0 };
        self.line = if self.h {
            self.ins + inset_miter + 0.5 * self.alw
        } else if self.rev {
            if self.lw.inner > 0.0 {
                self.ins
            } else {
                da + back_miter - 0.25 * self.alw
            }
        } else {
            self.ins + inset_miter - 0.25 * self.alw
        };
        self.tip = if self.rj { da + back_miter + 0.5 * self.alw } else { self.l + if self.h { xa } else { 0.0 } };
        {
            let y = self.hy(0.0);
            self.hull.push(Point::new(self.l + if !self.rj && self.h { xa } else { 0.0 }, y));
        }
        self.upper(0.0, 0.5 * self.w);
        {
            let y = self.hy(0.0);
            self.hull.push(Point::new(self.ins, y));
        }
        self.visual_back = Some(self.ins);
        self.ins += inset_miter;
        self.pen.m(da + back_miter, 0.0).l(back_miter, db).l(self.ins, 0.0);
        if !self.h {
            self.pen.l(back_miter, -db);
        }
        self.pen.z();
        {
            let segs = self.pen.take();
            self.push_closed(segs);
        }
    }
    fn kite_arrow(&mut self) {
        self.alw = self.alw.min(0.4 * self.l).min(0.4 * self.w);
        let q = ratio(self.l - self.ins, self.w);
        let xc = 0.5 * (4.0 * q * q + 1.0).sqrt() * self.alw;
        let xa = q * self.alw;
        let q2 = ratio(self.ins, self.w);
        let yc = 0.5 * (4.0 * q2 * q2 + 1.0).sqrt() * self.alw;
        let ya = q2 * self.alw;
        // the miter at the side corners
        let tb = 0.5 * self.w;
        let front = (self.l - self.ins).atan2(tb).to_degrees();
        let rear = self.ins.atan2(tb).to_degrees();
        let half = 0.5 * (front + rear);
        let r = if (half - 45.0).abs() < 1e-9 { std::f64::consts::SQRT_2 } else { ratio(1.0, half.to_radians().sin()) };
        let m = 0.5 * r * self.alw;
        let (s, c) = (-rear - 90.0 + half).to_radians().sin_cos();
        let xb = c * m;
        let side = s * m;
        let da = self.l - xc;
        let db = 0.5 * self.w + side;
        self.back = if self.rj { ya - 0.5 * self.alw } else { 0.0 };
        self.line = if self.rev { da - self.lw.line + self.alw } else { yc + self.lw.line - self.alw };
        self.tip = if self.rj { da + 0.5 * self.alw } else { self.l + if self.h { xa } else { 0.0 } };
        {
            let y = self.hy(0.0);
            self.hull.push(Point::new(self.l + if !self.rj && self.h { xa } else { 0.0 }, y));
        }
        self.upper(self.ins, 0.5 * self.w);
        {
            let y = self.hy(0.0);
            self.hull.push(Point::new(0.0, y));
        }
        self.ins += xb;
        self.pen.m(da, 0.0).l(self.ins, db).l(yc, 0.0);
        if !self.h {
            self.pen.l(self.ins, -db);
        }
        self.pen.z();
        {
            let segs = self.pen.take();
            self.push_closed(segs);
        }
    }
    fn square_arrow(&mut self) {
        self.alw = self.alw.min(0.5 * self.l);
        self.line = if self.rev { self.l - 0.5 * self.alw } else { 0.5 * self.alw };
        self.tip = self.l;
        self.back = 0.0;
        self.hull.push(Point::new(self.l, 0.5 * self.w));
        self.hull.push(Point::new(0.0, 0.5 * self.w));
        {
            let y = self.hy(-0.5 * self.w);
            self.hull.push(Point::new(self.l, y));
        }
        {
            let y = self.hy(-0.5 * self.w);
            self.hull.push(Point::new(0.0, y));
        }
        let (x0, x1, y) = (0.5 * self.alw, self.l - 0.5 * self.alw, 0.5 * self.w - 0.5 * self.alw);
        let yb = if self.h { 0.0 } else { -y };
        self.pen.m(x1, y).l(x0, y).l(x0, yb).l(x1, yb).z();
        {
            let segs = self.pen.take();
            self.push_closed(segs);
        }
    }
    fn circle_arrow(&mut self) {
        self.alw = self.alw.min(0.5 * self.l);
        self.line = if self.rev { self.l - 0.5 * self.alw } else { 0.5 * self.alw };
        self.tip = self.l;
        self.back = 0.0;
        self.upper(self.l, 0.25 * self.w);
        self.upper(0.75 * self.l, 0.5 * self.w);
        self.upper(0.25 * self.l, 0.5 * self.w);
        self.upper(0.0, 0.25 * self.w);
        if self.h {
            self.hull.push(Point::new(0.0, -0.5 * self.alw));
            self.hull.push(Point::new(self.l, -0.5 * self.alw));
        }
        let (rx, ry) = (0.5 * self.l - 0.5 * self.alw, 0.5 * self.w - 0.5 * self.alw);
        if self.h {
            self.pen.m(self.l - 0.5 * self.alw, 0.0).arc(0.0, 180.0, rx, ry).z();
        } else {
            self.pen.ellipse(0.5 * self.l, 0.0, rx, ry);
        }
        {
            let segs = self.pen.take();
            self.push_closed(segs);
        }
    }
    fn line_cap(&mut self, kind: Kind) {
        let t = 0.5 * self.lw.line;
        self.tip = self.l;
        self.back = 0.0;
        self.line = if self.rev && kind != Kind::ButtCap { self.l + 0.5 * self.lw.line } else { -0.5 * self.lw.line };
        self.upper(0.0, t);
        if kind == Kind::TriangleCap {
            self.hull.push(Point::new(self.l, 0.0));
        } else {
            self.upper(self.l, t);
        }
        let start = -0.75 * self.lw.line;
        let r = self.rev && kind != Kind::ButtCap;
        // reversed, the cap is drawn mirrored (x ↦ l - x) but its front part is not
        let len = self.l;
        let outer = move |x: f64| if r { len - x } else { x };
        self.pen.m(outer(start), t).l(outer(0.0), t);
        if r {
            self.pen.l(outer(self.l), t);
        }
        match kind {
            Kind::RoundCap => {
                let k = 0.55228475;
                self.pen.c(Cubic::new(Point::new(k * self.l, t), Point::new(self.l, k * t), Point::new(self.l, 0.0))).c(Cubic::new(
                    Point::new(self.l, -k * t),
                    Point::new(k * self.l, -t),
                    Point::new(0.0, -t),
                ));
            }
            Kind::ButtCap => {
                self.pen.l(self.l, t).l(self.l, -t);
            }
            _ => {
                self.pen.l(self.l, 0.0);
                if r {
                    self.pen.l(outer(self.l), -t);
                }
                self.pen.l(outer(0.0), -t);
            }
        }
        self.pen.l(outer(start), -t).z();
        {
            let segs = self.pen.take();
            self.shapes.push(Shape { segs, fill: true, stroke: false, width: 0.0, cap: None, join: None });
        }
    }
    fn fast_cap(&mut self, kind: Kind) {
        let t = 0.5 * self.lw.line;
        self.tip = self.l + self.ins;
        self.back = 0.0;
        self.line = if self.rev { self.l + self.ins } else { 0.0 };
        self.upper(0.0, t);
        if kind == Kind::FastTriangle {
            self.upper(self.ins, t);
            self.hull.push(Point::new(self.l + self.ins, 0.0));
            self.pen.m(0.0, t).l(self.ins, t).l(self.ins + self.l, 0.0).l(self.ins, -t).l(0.0, -t).l(self.l, 0.0).z();
        } else {
            self.upper(self.l + self.ins, t);
            let k = 0.55228475;
            self.pen.m(0.0, t).l(self.ins, t);
            self.pen.c(Cubic::new(Point::new(self.ins + k * self.l, t), Point::new(self.ins + self.l, k * t), Point::new(self.ins + self.l, 0.0))).c(Cubic::new(
                Point::new(self.ins + self.l, -k * t),
                Point::new(self.ins + k * self.l, -t),
                Point::new(self.ins, -t),
            ));
            self.pen
                .l(0.0, -t)
                .c(Cubic::new(Point::new(k * self.l, -t), Point::new(self.l, -k * t), Point::new(self.l, 0.0)))
                .c(Cubic::new(Point::new(self.l, k * t), Point::new(k * self.l, t), Point::new(0.0, t)))
                .z();
        }
        {
            let segs = self.pen.take();
            self.shapes.push(Shape { segs, fill: true, stroke: false, width: 0.0, cap: None, join: None });
        }
    }
    fn ray_tips(&mut self) {
        let n = self.keys.n.max(1);
        let inc = 360.0 / n as f64;
        let first = if n % 2 == 1 { 0.25 * inc } else { 0.5 * inc };
        let (s, c) = first.to_radians().sin_cos();
        let mut xa = c * 0.5 * self.l;
        if self.rc {
            xa += 0.5 * self.alw;
        } else {
            let v = Point::new(c * self.l, s * self.w);
            xa += v.y / v.norm().max(1e-12) * 0.5 * self.alw;
        }
        self.tip = xa;
        self.back = -xa;
        self.line = 0.0;
        let r = if self.rc { 0.5 * self.alw } else { 0.0 };
        self.upper(xa, 0.25 * self.w);
        self.upper(0.25 * self.l, 0.5 * self.w + r);
        self.upper(-0.25 * self.l, 0.5 * self.w + r);
        self.upper(-xa, 0.25 * self.w);
        if self.h {
            self.hull.push(Point::new(-0.5 * self.l - r, -0.5 * self.alw));
            self.hull.push(Point::new(0.5 * self.l + r, -0.5 * self.alw));
        }
        let mut a = first;
        let end = if self.h { 180.0 } else { 360.0 };
        while a < end - 1e-9 {
            let (s, c) = a.to_radians().sin_cos();
            self.pen.m(c * 0.5 * self.l, s * 0.5 * self.w).l(0.0, 0.0);
            if self.h && a <= inc + 1e-9 {
                self.pen.l(-0.5 * self.alw, 0.0);
            }
            a += inc;
        }
        {
            let segs = self.pen.take();
            let width = self.alw;
            let round = self.rc;
            self.shapes.push(Shape { segs, fill: false, stroke: true, width, cap: tip_cap(round), join: None });
        }
    }
    fn jack_tap(&mut self) {
        // a tenth of the length along the line, then up the width at its middle and down to
        // the end: open, or closed and filled
        self.tip = 1.1 * self.l;
        self.back = 0.0;
        self.line = 0.0;
        self.hull.extend([Point::new(0.0, 0.0), Point::new(0.6 * self.l, self.w), Point::new(self.tip, 0.0)]);
        self.pen.m(0.0, 0.0).l(0.1 * self.l, 0.0).l(0.6 * self.l, self.w).l(self.tip, 0.0);
        if !self.keys.open {
            self.pen.z();
        }
        {
            let segs = self.pen.take();
            let width = self.alw;
            let open = self.keys.open;
            let round = self.rc;
            let join = self.rj;
            self.shapes.push(Shape { segs, fill: !open, stroke: true, width, cap: tip_cap(round), join: tip_join(join) });
        }
    }
}

/// The tips of arrows.meta.
pub(in crate::engine::arrows) fn meta(kind: Kind, k: &Keys, lw: Widths) -> Geometry {
    let mut draw = MetaDraw {
        keys: k,
        lw,
        l: k.length,
        w: k.width,
        ins: k.inset,
        alw: k.line_width,
        h: k.harpoon,
        rev: k.reversed,
        rc: k.round_cap,
        rj: k.round_join,
        hull: Vec::new(),
        pen: Pen::default(),
        visual_back: None,
        shapes: Vec::new(),
        tip: 0.0,
        back: 0.0,
        line: 0.0,
    };
    match kind {
        Kind::StraightBarb => draw.straight_barb(),
        Kind::Hooks => draw.hook_barb(),
        Kind::ArcBarb => draw.arc_barb(),
        Kind::TeeBarb => draw.tee_barb(),
        Kind::ClassicalRightarrow => draw.classical_arrow(),
        Kind::CmRightarrow => draw.cm_arrow(),
        Kind::Implies => draw.implies_arrow(),
        Kind::Latex => draw.latex_arrow(),
        Kind::Stealth => draw.stealth_arrow(),
        Kind::Kite => draw.kite_arrow(),
        Kind::Square => draw.square_arrow(),
        Kind::Circle => draw.circle_arrow(),
        Kind::RoundCap | Kind::ButtCap | Kind::TriangleCap => draw.line_cap(kind),
        Kind::FastTriangle | Kind::FastRound => draw.fast_cap(kind),
        Kind::Rays => draw.ray_tips(),
        Kind::JackTap => draw.jack_tap(),
        Kind::Underscore | Kind::Old(_) => {
            draw.tip = 0.0;
            draw.back = 0.0;
            draw.line = 0.0;
        }
    }
    Geometry { tip: draw.tip, back: draw.back, line: draw.line, visual_tip: draw.tip, visual_back: draw.visual_back.unwrap_or(draw.back), hull: draw.hull, shapes: draw.shapes }
}
