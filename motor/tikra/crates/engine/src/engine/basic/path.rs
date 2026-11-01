//! The basic layer's path, in canvas coordinates.

use crate::scene::{Point, Segment, Transform};
use crate::tex::{self, Token};
use crate::{Error, PT};

use super::super::shapes::{ShapePath, Usage};
use super::super::state::Mode;
use super::super::{tikra_tokens, Engine};
use super::catalog::B;

/// The basic layer's path being built, in canvas coordinates.
#[derive(Debug, Clone, Default)]
pub struct PgfPath {
    pub segments: Vec<Segment>,
    pub last: Option<Point>,
    pub start: Option<Point>,
    /// built by the quick commands only (`\pgfpathqmoveto`…): it does not make the picture larger
    pub quick: bool,
}

/// The soft path a macro holds, if it holds one.
pub fn softpath_of(e: &Engine, macro_name: &Token) -> Option<Vec<Segment>> {
    let tex::Meaning::Macro(m) = e.tex.meaning(macro_name) else {
        return None;
    };
    let text: String = m.body.iter().filter_map(|b| if let tex::Body::Tok(t) = b { Some(tex::tokens_to_string(std::slice::from_ref(t))) } else { None }).collect();
    text.starts_with("@softpath").then(|| e.saved_paths.get(&text).cloned()).flatten()
}

impl Engine {
    pub fn pgf_add(&mut self, s: Segment) {
        self.pgf_path.quick = false;
        let p = match s {
            Segment::MoveTo(p) | Segment::LineTo(p) | Segment::CurveTo(_, _, p) => Some(p),
            Segment::Close => self.pgf_path.start,
        };
        if let Segment::MoveTo(p) = s {
            self.pgf_path.start = Some(p);
        }
        if !matches!(s, Segment::MoveTo(_)) && self.pgf_path.segments.is_empty() {
            let at = self.pgf_path.last.unwrap_or_default();
            self.pgf_path.segments.push(Segment::MoveTo(at));
            self.pgf_path.start = Some(at);
        }
        self.pgf_path.segments.push(s);
        self.pgf_path.last = p;
    }

    pub(in crate::engine::basic) fn last_local(&self) -> Point {
        self.st.transform.inverse().apply(self.pgf_path.last.unwrap_or_default())
    }

    /// Uses the basic layer's path with `actions` (`fill`, `stroke`, `draw`, `clip`, `discard`…).
    pub(in crate::engine::basic) fn pgf_use(&mut self, actions: &str) -> Result<(), Error> {
        let segs = std::mem::take(&mut self.pgf_path.segments);
        let quick = std::mem::take(&mut self.pgf_path.quick);
        if let Some(p) = self.pgf_path.last {
            self.last_xy = p;
        }
        self.pgf_path = PgfPath::default();
        let frozen = self.canvas_frozen;
        self.canvas_frozen |= quick;
        self.begin_group();
        self.st.mode = Mode::default();
        for a in actions.split(',').map(str::trim) {
            match a {
                // after \pgfsetfillpattern, a fill is the pattern
                "fill" if self.st.paint.fill_pattern => self.st.mode.pattern = true,
                "fill" => self.st.mode.fill = true,
                "stroke" | "draw" => self.st.mode.draw = true,
                "clip" => self.st.mode.clip = true,
                "use as bounding box" => self.st.mode.bounding_box = true,
                "discard" | "" => {}
                other => self.warn(format!("\\pgfusepath{{{other}}} not supported: ignored")),
            }
        }
        // in a declared shape's code: a path of the shape
        if let Some(layer) = self.shape_capture.as_ref().and_then(|c| c.layer) {
            let mut uses = Vec::new();
            if self.st.mode.fill {
                uses.push(Usage::Fill(self.st.fill_paint_color().xcolor()));
            }
            if self.st.mode.draw {
                uses.push(Usage::Stroke);
            }
            if let Some(c) = &mut self.shape_capture {
                c.paths.extend(uses.into_iter().map(|usage| ShapePath { layer, segments: segs.clone(), usage }));
            }
            self.end_group();
            self.canvas_frozen = frozen;
            return Ok(());
        }
        // the clip outlives this group: it belongs to the pgf scope around
        let r = self.use_path(segs, "\\pgfusepath");
        self.end_group();
        if quick {
            self.canvas_frozen = frozen;
        }
        r
    }
    pub(in crate::engine::basic) fn path_arc(&mut self, cmd: B, t: Transform) -> Result<(), Error> {
        let s = self.argument()?;
        let en = self.argument()?;
        let (a0, a1) = (self.number_of(s)?, self.number_of(en)?);
        let center_from = self.last_local();
        let (segments, end) = if cmd == B::Arc {
            let r = self.argument()?;
            let text = tex::tokens_to_string(&r);
            let (rx, ry) = match text.split_once(" and ") {
                Some((x, y)) => (self.length(x)?, self.length(y)?),
                None => {
                    let v = self.length_of(r)?;
                    (v, v)
                }
            };
            let (s0, c0) = a0.to_radians().sin_cos();
            let center = center_from.minus(Point::new(rx * c0, ry * s0));
            let segs = super::super::path::arc_segments(super::super::path::EllipseArc { center: t.apply(center), rx, ry, start: a0, end: a1 }, &t);
            let (s1, c1) = a1.to_radians().sin_cos();
            (segs, t.apply(center.plus(Point::new(rx * c1, ry * s1))))
        } else {
            // the arc of the ellipse with these two axes
            let ax = self.point_arg()?;
            let ay = self.point_arg()?;
            let axes = Transform { a: ax.x, b: ax.y, c: ay.x, d: ay.y, e: 0.0, f: 0.0 };
            let (s0, c0) = a0.to_radians().sin_cos();
            let center = center_from.minus(axes.apply(Point::new(c0, s0)));
            let full = t.compose(&Transform::translation(center.x, center.y)).compose(&axes);
            let segs = super::super::path::arc_segments(
                super::super::path::EllipseArc { center: full.apply(Point::default()), rx: 1.0, ry: 1.0, start: a0, end: a1 },
                &Transform { e: 0.0, f: 0.0, ..full },
            );
            let (s1, c1) = a1.to_radians().sin_cos();
            (segs, full.apply(Point::new(c1, s1)))
        };
        for s in segments {
            self.pgf_add(s);
        }
        self.pgf_path.last = Some(end);

        Ok(())
    }

    pub(in crate::engine::basic) fn path_ellipse(&mut self, cmd: B, t: Transform) -> Result<(), Error> {
        let c = self.point_arg()?;
        let (a, b) = if cmd == B::Circle {
            let r = self.argument()?;
            let r = self.length_of(r)?;
            (Point::new(r, 0.0), Point::new(0.0, r))
        } else {
            (self.point_arg()?, self.point_arg()?)
        };
        let axes = Transform { a: a.x, b: a.y, c: b.x, d: b.y, e: 0.0, f: 0.0 };
        let full = t.compose(&Transform::translation(c.x, c.y)).compose(&axes);
        self.pgf_add(Segment::MoveTo(full.apply(Point::new(1.0, 0.0))));
        for s in super::super::path::arc_segments(
            super::super::path::EllipseArc { center: full.apply(Point::default()), rx: 1.0, ry: 1.0, start: 0.0, end: 360.0 },
            &Transform { e: 0.0, f: 0.0, ..full },
        ) {
            self.pgf_add(s);
        }
        self.pgf_add(Segment::Close);
        self.pgf_path.last = Some(t.apply(c));

        Ok(())
    }

    pub(in crate::engine::basic) fn path_rectangle(&mut self, cmd: B, t: Transform) -> Result<(), Error> {
        let a = self.point_arg()?;
        let b = self.point_arg()?;
        let b = if cmd == B::Rectangle { a.plus(b) } else { b };
        self.pgf_add(Segment::MoveTo(t.apply(a)));
        self.pgf_add(Segment::LineTo(t.apply(Point::new(a.x, b.y))));
        self.pgf_add(Segment::LineTo(t.apply(b)));
        self.pgf_add(Segment::LineTo(t.apply(Point::new(b.x, a.y))));
        self.pgf_add(Segment::Close);
        self.pgf_path.last = Some(t.apply(a));

        Ok(())
    }

    pub(in crate::engine::basic) fn path_grid(&mut self, t: Transform) -> Result<(), Error> {
        // /pgf/stepx and /pgf/stepy are 1cm
        let mut step = (1.0, 1.0);
        if let Some(o) = self.optional()? {
            for item in crate::keys::split_list(&o) {
                let Some(v) = item.value else { continue };
                match item.key.as_str() {
                    // a point: its x and y
                    "step" => {
                        let p = self.point_of(v)?;
                        step = (p.x.abs(), p.y.abs());
                    }
                    "stepx" => step.0 = self.length_of(v)?,
                    "stepy" => step.1 = self.length_of(v)?,
                    _ => {}
                }
            }
        }
        let ll = self.point_arg()?;
        let ur = self.point_arg()?;
        let (x0, x1) = (ll.x.min(ur.x), ll.x.max(ur.x));
        let (y0, y1) = (ll.y.min(ur.y), ll.y.max(ur.y));
        let lines = |a: f64, b: f64, s: f64| -> Vec<f64> {
            if s <= 0.0 {
                return Vec::new();
            }
            let first = (a / s).ceil();
            let n = ((b / s).floor() - first + 1.0).clamp(0.0, 10_000.0) as usize;
            (0..n).map(|k| (first + k as f64) * s).collect()
        };
        for y in lines(y0, y1, step.1) {
            self.pgf_add(Segment::MoveTo(t.apply(Point::new(x0, y))));
            self.pgf_add(Segment::LineTo(t.apply(Point::new(x1, y))));
        }
        for x in lines(x0, x1, step.0) {
            self.pgf_add(Segment::MoveTo(t.apply(Point::new(x, y0))));
            self.pgf_add(Segment::LineTo(t.apply(Point::new(x, y1))));
        }

        Ok(())
    }

    pub(in crate::engine::basic) fn path_parabola(&mut self, t: Transform) -> Result<(), Error> {
        let bend = self.point_arg()?;
        let end = self.point_arg()?;
        let mut from = self.last_local();
        if bend.norm() > 1e-12 {
            let c = bend;
            self.pgf_add(Segment::CurveTo(t.apply(from.plus(Point::new(0.1125 * c.x, 0.225 * c.y))), t.apply(from.plus(Point::new(0.5 * c.x, c.y))), t.apply(from.plus(c))));
            from = from.plus(c);
        }
        if end.norm() > 1e-12 {
            let c = end;
            self.pgf_add(Segment::CurveTo(t.apply(from.plus(Point::new(0.5 * c.x, 0.0))), t.apply(from.plus(Point::new(0.8875 * c.x, 0.775 * c.y))), t.apply(from.plus(c))));
        }

        Ok(())
    }

    pub(in crate::engine::basic) fn path_trig(&mut self, cmd: B, t: Transform) -> Result<(), Error> {
        let d = self.point_arg()?;
        let a = self.last_local();
        let (c1, c2) = if cmd == B::Sine {
            (Point::new(0.3260 * d.x, 0.5120 * d.y), Point::new(0.6380 * d.x, d.y))
        } else {
            (Point::new(0.3620 * d.x, 0.0), Point::new(0.6740 * d.x, 0.4880 * d.y))
        };
        self.pgf_add(Segment::CurveTo(t.apply(a.plus(c1)), t.apply(a.plus(c2)), t.apply(a.plus(d))));

        Ok(())
    }

    pub(in crate::engine::basic) fn set_dash(&mut self) -> Result<(), Error> {
        // \pgfsetdash{{3pt}{2pt}}{0pt}
        let pattern = self.argument()?;
        let phase = self.argument()?;
        let parts = super::super::options::split_arguments(&pattern, 16);
        let mut v = Vec::new();
        for p in parts.into_iter().filter(|p| !crate::keys::trim(p).is_empty()) {
            v.push(self.length_of(p)?);
        }
        let phase = self.length_of(phase).unwrap_or(0.0);
        self.st.pen.dash = if v.is_empty() { None } else { Some((v, phase)) };

        Ok(())
    }
    pub(in crate::engine::basic) fn path_svg_arc(&mut self, t: Transform) -> Result<(), Error> {
        // {x radius}{y radius}{rotation}{large arc}{counterclockwise}{target}, as SVG's arc
        let rx = self.argument()?;
        let ry = self.argument()?;
        let rot = self.argument()?;
        let large = self.argument()?;
        let ccw = self.argument()?;
        let (rx, ry, rot) = (self.length_of(rx)?.abs(), self.length_of(ry)?.abs(), self.number_of(rot)?);
        let (large, ccw) = (self.number_of(large)? != 0.0, self.number_of(ccw)? != 0.0);
        let to = self.point_arg()?;
        let from = self.last_local();
        for s in super::super::path::svg_arc_segments(
            super::super::path::SvgArc { from: from.scale(1.0 / PT), to: to.scale(1.0 / PT), rx: rx / PT, ry: ry / PT, phi: rot, large, sweep: ccw },
            &t,
        ) {
            self.pgf_add(s);
        }
        self.pgf_path.last = Some(t.apply(to));

        Ok(())
    }
    pub(in crate::engine::basic) fn recall_path(&mut self, cmd: B) -> Result<(), Error> {
        // \pgfgetpath\macro keeps the path being built under the macro's name;
        // \pgfsetpath\macro makes it the path being built again
        let m = self.argument()?;
        let name = m.iter().find_map(|t| if let Token::Cs(n) = t { Some(n.to_string()) } else { None }).unwrap_or_default();
        if cmd == B::GetPath {
            self.saved_paths.insert(format!("pgf@{name}"), self.pgf_path.segments.clone());
        } else if let Some(segs) = m.iter().find(|t| matches!(t, Token::Cs(_))).and_then(|t| softpath_of(self, t)).or_else(|| self.saved_paths.get(&format!("pgf@{name}")).cloned())
        {
            self.pgf_path = PgfPath::default();
            for s in segs {
                self.pgf_add(s);
            }
        }

        Ok(())
    }

    pub(in crate::engine::basic) fn intersect_paths(&mut self) -> Result<(), Error> {
        // each code builds a path (what it draws is drawn); where the two paths cross
        let mut paths = Vec::new();
        for _ in 0..2 {
            let code = self.argument()?;
            self.begin_group();
            self.begin_scope();
            self.pgf_path = PgfPath::default();
            let r = self.run_tokens(code);
            paths.push(std::mem::take(&mut self.pgf_path.segments));
            self.pgf_path = PgfPath::default();
            self.end_scope();
            self.end_group();
            r?;
        }
        self.pgf_solutions = super::super::intersections::intersections(&paths[0], &paths[1]);
        // kept for `\pgfcomputeintersectionsegments`
        for (k, p) in paths.iter().enumerate() {
            self.saved_paths.insert(format!("@intersection path {}", k + 1), p.clone());
        }
        let n = self.pgf_solutions.len();
        self.tex.set_meaning(&Token::cs("pgfintersectionsolutions"), super::super::macro_tokens(tikra_tokens(&n.to_string())), false);

        Ok(())
    }

    pub(in crate::engine::basic) fn path_arc_precomputed(&mut self, t: Transform) -> Result<(), Error> {
        // {center}{start angle}{end angle}{end point}{x radius}{y radius}{the radii's ratios}: the
        // arc of the axis-parallel ellipse
        let c = self.point_arg()?;
        let a0 = self.argument()?;
        let a1 = self.argument()?;
        let (a0, a1) = (self.number_of(a0)?, self.number_of(a1)?);
        let end = self.point_arg()?;
        let rx = self.argument()?;
        let ry = self.argument()?;
        let (rx, ry) = (self.length_of(rx)?, self.length_of(ry)?);
        let _ = self.argument()?;
        let _ = self.argument()?;
        for s in super::super::path::arc_segments(super::super::path::EllipseArc { center: t.apply(c), rx, ry, start: a0, end: a1 }, &t) {
            self.pgf_add(s);
        }
        self.pgf_path.last = Some(t.apply(end));

        Ok(())
    }
    /// `\pgfpathqmoveto{x}{y}`, `\pgfpathqlineto`, `\pgfpathqcurveto` (six lengths) and
    /// `\pgfpathqcircle{radius}`: canvas lengths, untransformed.
    pub(in crate::engine::basic) fn quick_path(&mut self, cmd: B) -> Result<(), Error> {
        match cmd {
            B::QMoveTo | B::QLineTo => {
                let x = self.argument()?;
                let y = self.argument()?;
                let p = Point::new(self.length_of(x)?, self.length_of(y)?);
                self.pgf_add(if cmd == B::QMoveTo { Segment::MoveTo(p) } else { Segment::LineTo(p) });
            }
            B::QCurveTo => {
                let mut v = [0.0; 6];
                for k in &mut v {
                    let a = self.argument()?;
                    *k = self.length_of(a)?;
                }
                self.pgf_add(Segment::CurveTo(Point::new(v[0], v[1]), Point::new(v[2], v[3]), Point::new(v[4], v[5])));
            }
            _ => {
                let r = self.argument()?;
                let r = self.length_of(r)?;
                self.pgf_add(Segment::MoveTo(Point::new(r, 0.0)));
                for s in super::super::path::arc_segments(super::super::path::EllipseArc { center: Point::default(), rx: r, ry: r, start: 0.0, end: 360.0 }, &Transform::default())
                {
                    self.pgf_add(s);
                }
                self.pgf_add(Segment::Close);
            }
        }
        Ok(())
    }
}
