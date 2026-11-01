//! The path a state draws: motifs, and the dispatch over the decoration.

use super::super::feet;
use super::*;

impl Run<'_> {
    /// One coil: the control points pgf lists, two quadrants or all four.
    fn coil_code(&mut self, t: &Transform, state: &str) {
        let (a, l) = (self.amp(), self.len());
        let asp = self.s.aspect;
        let on = |x: f64, y: f64, k: f64| p(x * a * asp + k * 0.083333333333 * l, y * a);
        let half = [[(0.0, 0.555, 1.0), (0.445, 1.0, 2.0), (1.0, 1.0, 3.0)], [(1.555, 1.0, 4.0), (2.0, 0.555, 5.0), (2.0, 0.0, 6.0)]];
        let rest = [[(2.0, -0.555, 7.0), (1.555, -1.0, 8.0), (1.0, -1.0, 9.0)], [(0.445, -1.0, 10.0), (0.0, -0.555, 11.0), (0.0, 0.0, 12.0)]];
        let n = if state == "coil" { 4 } else { 2 };
        for c in half.iter().chain(rest.iter()).take(n) {
            let q: Vec<Point> = c.iter().map(|&(x, y, k)| t.apply(on(x, y, k))).collect();
            self.w.out.curve_to(q[0], q[1], q[2]);
        }
    }

    /// A brace across the remaining path: the two bumps and the point in the middle.
    fn brace_code(&mut self, t: &Transform) {
        let (a, r) = (self.amp(), self.w.remaining);
        let asp = self.s.aspect;
        let mut yc = asp * r;
        yc = if 2.0 * a > yc { 0.5 * yc } else { a };
        let mut xc = asp * r - r;
        xc = if -2.0 * a < xc { -0.5 * xc } else { a };
        self.mv(t, 0.0, 0.0);
        self.cv(t, LocalCurve { c1: p(0.15 * yc, 0.3 * a), c2: p(0.5 * yc, 0.5 * a), end: p(yc, 0.5 * a) });
        let t2 = shifted(t, asp * r, 0.0);
        self.ln(&t2, -yc, 0.5 * a);
        self.cv(&t2, LocalCurve { c1: p(-0.5 * yc, 0.5 * a), c2: p(-0.15 * yc, 0.7 * a), end: p(0.0, a) });
        self.cv(&t2, LocalCurve { c1: p(0.15 * xc, 0.7 * a), c2: p(0.5 * xc, 0.5 * a), end: p(xc, 0.5 * a) });
        let t3 = shifted(t, r, 0.0);
        self.ln(&t3, -xc, 0.5 * a);
        self.cv(&t3, LocalCurve { c1: p(-0.5 * xc, 0.5 * a), c2: p(-0.15 * xc, 0.3 * a), end: p(0.0, 0.0) });
    }

    /// The start cap, the band or the end cap of a circle connection bar. All three are drawn from
    /// the start, because each state's width is 0.
    fn circle_bar(&mut self, t: &Transform, state: &str) {
        let a = self.amp();
        let (r1, r2) = (self.key_pt("start radius", 2.5), self.key_pt("end radius", 2.5));
        let (ang, rem) = (self.s.angle, self.w.remaining);
        let all = tikra_engine::engine::mindmap::bar(r1, r2, a, ang, rem, t);
        let mut pieces = all.split_inclusive(|s| matches!(s, tikra_engine::scene::Segment::Close));
        let piece = match state {
            "initial" => pieces.next(),
            "bar" => pieces.nth(1),
            _ => pieces.nth(2),
        };
        for seg in piece.unwrap_or_default() {
            match *seg {
                tikra_engine::scene::Segment::MoveTo(q) => self.w.out.move_to(q),
                tikra_engine::scene::Segment::LineTo(q) => self.w.out.line_to(q),
                tikra_engine::scene::Segment::CurveTo(c1, c2, q) => self.w.out.curve_to(c1, c2, q),
                tikra_engine::scene::Segment::Close => self.w.out.close(),
            }
        }
    }

    /// The points of one Koch or Cantor step, as fractions of the segment: `(x, y, draw a line)`.
    fn koch_points(dec: Dec) -> &'static [(f64, f64, bool)] {
        match dec {
            Dec::Koch1 => &[(0.33333, 0.0, true), (0.33333, 0.33333, true), (0.66666, 0.33333, true), (0.66666, 0.0, true), (1.0, 0.0, true)],
            Dec::Koch2 => {
                &[(0.25, 0.0, true), (0.25, 0.25, true), (0.5, 0.25, true), (0.5, 0.0, true), (0.5, -0.25, true), (0.75, -0.25, true), (0.75, 0.0, true), (1.0, 0.0, true)]
            }
            Dec::KochSnowflake => &[(0.3333, 0.0, true), (0.5, 0.2886751347, true), (0.6666, 0.0, true), (1.0, 0.0, true)],
            _ => &[(0.3333, 0.0, true), (0.6666, 0.0, false), (1.0, 0.0, true)],
        }
    }

    /// One step of a Koch curve or of the Cantor set, along the segment that is left.
    fn koch_code(&mut self, t: &Transform, dec: Dec) {
        let r = self.w.seg_remaining;
        for &(x, y, line) in Self::koch_points(dec) {
            if line {
                self.ln(t, x * r, y * r);
            } else {
                self.mv(t, x * r, y * r);
            }
        }
    }

    /// A footprint at this state: mirrored for the right foot, shifted, scaled and turned.
    fn place_foot(&mut self, t: &Transform, side: &str) {
        let t = if side == "right" { t.compose(&Transform::scaling(1.0, -1.0)) } else { *t };
        let t = shifted(&t, 0.0, self.key_pt("foot sep", 4.0) / 2.0);
        let size = self.key_pt("foot length", 10.0);
        let t = t.compose(&Transform::scaling(size, size)).compose(&Transform::rotation(self.key_number("foot angle", 10.0)));
        self.foot(&t);
    }

    /// The code of a state, drawn with `t`.
    pub(super) fn code(&mut self, dec: Dec, state: &str, t: &Transform) -> Result<(), Error> {
        let (a, l) = (self.amp(), self.len());
        match (dec, state) {
            (Dec::User(n), s) => {
                if let Some(code) = macro_body(self.e, &format!("pgf@decorate@@{n}@{s}@code")) {
                    self.tex_code(code, t, true, false)?;
                }
            }
            (Dec::LineTo, "input segment") => {
                let q = self.w.seg.last;
                self.w.out.line_to(q);
            }
            (Dec::CurveTo, _) => self.ln(t, 0.0, 0.0),
            (Dec::Zigzag, "up from center") | (Dec::Zigzag, "big up") => self.ln(t, 0.25 * l, a),
            (Dec::Zigzag, "big down") => self.ln(t, 0.25 * l, -a),
            (Dec::Zigzag, "center finish") => self.ln(t, 0.0, 0.0),
            (Dec::Saw, _) => {
                self.ln(t, l, a);
                self.ln(t, l, 0.0);
            }
            (Dec::RandomSteps, "step") => {
                let x = self.rand() * a;
                let y = self.rand() * a;
                self.ln(t, l + x, y);
            }
            (Dec::Bent, _) => {
                let r = self.w.seg_remaining;
                let k = self.s.aspect * r;
                self.cv(t, LocalCurve { c1: p(k, a), c2: p(r - k, a), end: p(r, 0.0) });
            }
            (Dec::Snake, "initial") => self.cv(t, LocalCurve { c1: p(0.125 * l, 0.0), c2: p(0.1875 * l, a), end: p(0.3125 * l, a) }),
            (Dec::Snake, "down") | (Dec::Snake, "up") => {
                let y = if state == "down" { -a } else { a };
                self.wave(t, 0.25 * l, y, false);
                self.wave(t, 0.25 * l, y, true);
            }
            (Dec::Snake, "end down") => self.cv(t, LocalCurve { c1: p(0.125 * l, a), c2: p(0.1875 * l, 0.0), end: p(0.3125 * l, 0.0) }),
            (Dec::Snake, "end up") => self.cv(t, LocalCurve { c1: p(0.125 * l, -a), c2: p(0.1875 * l, 0.0), end: p(0.3125 * l, 0.0) }),
            (Dec::Coil, _) => self.coil_code(t, state),
            (Dec::Bumps, _) => {
                self.cv(t, LocalCurve { c1: p(0.0, 0.555 * a), c2: p(0.11125 * l, a), end: p(0.25 * l, a) });
                self.cv(t, LocalCurve { c1: p(0.38875 * l, a), c2: p(0.5 * l, 0.5 * a), end: p(0.5 * l, 0.0) });
            }
            (Dec::Ticks, _) => {
                self.mv(t, 0.0, a);
                self.ln(t, 0.0, -a);
            }
            (Dec::ExpandingWaves, "wave") | (Dec::ExpandingWaves, "last") => {
                let c = self.w.completed;
                let ang = self.s.angle;
                let q = p(-c, 0.0).plus(polar(ang, c));
                self.mv(t, q.x, q.y);
                self.arc(t, ang, -ang, c);
            }
            (Dec::Waves, _) => {
                let t = shifted(t, l, 0.0);
                let r = self.key_pt("start radius", 2.5);
                let ang = self.s.angle;
                let q = p(-r, 0.0).plus(polar(ang, r));
                self.mv(&t, q.x, q.y);
                self.arc(&t, ang, -ang, r);
            }
            (Dec::Border, _) => {
                self.mv(t, 0.0, 0.0);
                let q = polar(self.s.angle, a);
                self.ln(t, q.x, q.y);
            }
            (Dec::Brace, _) => self.brace_code(t),
            (Dec::Markings, "main") => self.mark(t)?,
            (Dec::Triangles, "triangle") | (Dec::Triangles, "last") => {
                let (w, h) = (self.ssw, self.ssh);
                self.mv(t, 0.0, h);
                self.ln(t, w, 0.0);
                self.ln(t, 0.0, -h);
                self.w.out.close();
            }
            (Dec::Crosses, "crosses") | (Dec::Crosses, "last") => self.cross(t),
            (Dec::CircleConnectionBar, state) => self.circle_bar(t, state),
            (Dec::Koch1 | Dec::Koch2 | Dec::KochSnowflake | Dec::Cantor, _) => self.koch_code(t, dec),
            (Dec::Footprints, s) => self.place_foot(t, s),
            (Dec::ShapeBackgrounds, "shape") => self.shape_code(t),
            (Dec::TextAlongPath, "typeset") => self.text_typeset(t),
            (Dec::TextEffects, "token") => {
                if let Some(i) = self.te.count.checked_sub(1).filter(|i| *i < self.te.chars.len()) {
                    self.te_node(i, Some(t))?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn final_code(&mut self, dec: Dec, t: &Transform) -> Result<(), Error> {
        let last = self.w.path_last;
        match dec {
            Dec::LineTo | Dec::CurveTo | Dec::Zigzag | Dec::Snake | Dec::Coil | Dec::Bumps => self.w.out.line_to(last),
            Dec::MoveTo | Dec::ExpandingWaves | Dec::Waves | Dec::Border | Dec::Triangles | Dec::Crosses | Dec::ShapeBackgrounds => self.w.out.move_to(last),
            Dec::Ticks => {
                let a = self.amp();
                self.mv(t, 0.0, a);
                self.ln(t, 0.0, -a);
                self.w.out.move_to(last);
            }
            Dec::Markings => {
                if self.s.connection_node.is_empty() {
                    self.w.out.move_to(last);
                } else {
                    self.w.out.line_to(last);
                }
            }
            Dec::ShowPathConstruction => self.segment_code_current()?,
            Dec::User(n) => {
                if let Some(code) = macro_body(self.e, &format!("pgf@decorate@@{n}@final@code")) {
                    self.tex_code(code, t, true, false)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn cross(&mut self, t: &Transform) {
        let (w, h) = (self.ssw, self.ssh);
        self.mv(t, -w, h);
        self.ln(t, w, -h);
        self.mv(t, -w, -h);
        self.ln(t, w, h);
    }

    fn foot(&mut self, t: &Transform) {
        let foot = match self.key_text("foot of").as_str() {
            "gnome" => &feet::GNOME,
            "bird" => &feet::BIRD,
            "felis silvestris" => &feet::FELIS_SILVESTRIS,
            "human" => &feet::HUMAN,
            other => {
                self.e.warn(format!("footprints of '{other}' not known: drawn as human"));
                &feet::HUMAN
            }
        };
        let [a, b, c, d, e, f] = foot.cm;
        let t = t.compose(&Transform { a, b, c, d, e, f }).compose(&Transform::scaling(foot.scale, foot.scale));
        for op in foot.ops {
            match *op {
                FootOp::M(x, y) => self.mv(&t, x, y),
                FootOp::L(x, y) => self.ln(&t, x, y),
                FootOp::C(x1, y1, x2, y2, x, y) => self.cv(&t, LocalCurve { c1: p(x1, y1), c2: p(x2, y2), end: p(x, y) }),
                FootOp::Z => self.w.out.close(),
            }
        }
    }
}
