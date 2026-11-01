//! `shape backgrounds`: a shape's background path scaled between two anchors.

use super::*;

impl Run<'_> {
    pub(super) fn shape_precompute(&mut self, state: &str) -> Result<(), Error> {
        match state {
            "initialise" => {
                let sh = &mut self.shape;
                *sh = ShapeRun::default();
                let sw = self.key_pt("shape start width", 2.5);
                let ew = self.key_pt("shape end width", 2.5);
                let hs = self.key_pt("shape start height", 2.5);
                let he = self.key_pt("shape end height", 2.5);
                let scaled = self.s.shape_scaled;
                let sep_text = self.s.shape_sep.clone();
                let spread_text = self.s.shape_spread.clone();
                let rem = self.w.remaining;
                let mut sh = ShapeRun { start_width: sw, width: sw, width_change: ew - sw, start_height: hs, height: hs, height_change: he - hs, ..Default::default() };
                if spread_text.trim().is_empty() {
                    let (amount, keyword) = split_keyword(&sep_text);
                    sh.between_borders = keyword == "between borders";
                    sh.sep = self.e.length(&amount)? / PT;
                } else {
                    sh.spread = true;
                    let (amount, keyword) = split_keyword(&spread_text);
                    sh.between_borders = keyword == "by borders";
                    let mut n = self.e.number(&amount)?.trunc() as i64;
                    let mut x;
                    if sh.between_borders {
                        if n > 1 {
                            n -= 1;
                            let ab = if scaled { sw + ew } else { 2.0 * sw };
                            x = ((rem - n as f64 * 0.5 * ab) / n as f64) * 0.9999;
                            if scaled {
                                sh.special_width = sh.width_change / n as f64;
                            }
                        } else {
                            sh.between_borders = false;
                            x = rem;
                            if n == 1 {
                                sh.initialise = 0.5 * x;
                            } else {
                                x += 5.0;
                                sh.initialise = x;
                            }
                        }
                    } else {
                        x = rem;
                        if n > 1 {
                            x /= (n - 1) as f64;
                        } else if n == 1 {
                            sh.initialise = 0.5 * x;
                        } else {
                            x += 5.0;
                            sh.initialise = x;
                        }
                    }
                    sh.sep = x;
                }
                self.shape = sh;
            }
            "before shape" => {
                let c = self.w.completed;
                let r = self.w.remaining;
                let sh = &mut self.shape;
                if self.s.shape_scaled {
                    let ratio = if sh.between_borders {
                        if !sh.spread {
                            (c + sh.start_width * 0.5) / (c + r - 0.5 * sh.width_change)
                        } else {
                            0.0
                        }
                    } else {
                        c / (c + r)
                    };
                    let ratio = if sh.spread && sh.between_borders {
                        let w = sh.width + sh.special_width;
                        (w - sh.start_width) / sh.width_change
                    } else {
                        ratio
                    };
                    sh.width = if sh.spread && sh.between_borders { sh.width + sh.special_width } else { sh.width_change * ratio + sh.start_width };
                    sh.height = sh.height_change * ratio + sh.start_height;
                }
                sh.before = if sh.between_borders { 0.5 * sh.width } else { 0.0 };
            }
            "after shape" => {
                let sh = &mut self.shape;
                sh.after = if sh.between_borders { 0.5 * sh.width } else { 0.0 };
            }
            _ => self.shape.before = 0.0,
        }
        Ok(())
    }

    /// The shape's background path, scaled to the shape's width and height, at its anchor.
    pub(super) fn shape_code(&mut self, t: &Transform) {
        let mut t = *t;
        if !self.s.shape_sloped {
            t = t.compose(&Transform::rotation(-self.w.angle));
        }
        let (w, h) = (self.shape.width / 2.0, self.shape.height / 2.0);
        let name = self.key_text("shape");
        let anchor = self.key_text("anchor");
        let off = match anchor.as_str() {
            "north" => p(0.0, h),
            "south" => p(0.0, -h),
            "east" => p(w, 0.0),
            "west" => p(-w, 0.0),
            "north east" => p(w, h),
            "north west" => p(-w, h),
            "south east" => p(w, -h),
            "south west" => p(-w, -h),
            _ => p(0.0, 0.0),
        };
        let base = t;
        let t = shifted(&t, -off.x, -off.y);
        match name.as_str() {
            "circle" | "ellipse" => {
                let k = 0.5523;
                self.mv(&t, w, 0.0);
                self.cv(&t, LocalCurve { c1: p(w, h * k), c2: p(w * k, h), end: p(0.0, h) });
                self.cv(&t, LocalCurve { c1: p(-w * k, h), c2: p(-w, h * k), end: p(-w, 0.0) });
                self.cv(&t, LocalCurve { c1: p(-w, -h * k), c2: p(-w * k, -h), end: p(0.0, -h) });
                self.cv(&t, LocalCurve { c1: p(w * k, -h), c2: p(w, -h * k), end: p(w, 0.0) });
                self.w.out.close();
            }
            "diamond" => {
                self.mv(&t, 0.0, -h);
                self.ln(&t, w, 0.0);
                self.ln(&t, 0.0, h);
                self.ln(&t, -w, 0.0);
                self.w.out.close();
            }
            other if other != "rectangle" => {
                // any other shape: its background path at the size, from the node shapes, at its anchor
                let boxes = [("text".to_string(), crate::engine::shapes::BoxSize::default())];
                let (line_width, ex) = (self.e.st.pen.line_width, 4.3 * PT);
                let mut keys = crate::engine::nodes::EngineKeys(self.e);
                let mut input = crate::engine::shapes::Input {
                    parts: &boxes,
                    // pgf's shape of an arbitrary size, then stretched to the decoration's
                    inner_xsep: 50.0 * PT,
                    inner_ysep: 50.0 * PT,
                    minimum_width: PT,
                    minimum_height: PT,
                    outer_xsep: 0.0,
                    outer_ysep: 0.0,
                    aspect: 1.0,
                    ex,
                    line_width,
                    pointer: None,
                    keys: &mut keys,
                };
                let Some(g) = crate::engine::shapes::compute(other, &mut input) else {
                    self.e.warn(format!("shape backgrounds: shape '{other}' not supported: drawn as a rectangle"));
                    self.rectangle(&t, w, h);
                    return;
                };
                // the path stretched to be as wide and high as the shape's size, as pgf scales the
                // background it measured
                let mut bounds = crate::canvas::Canvas::default();
                for path in g.paths.iter().filter(|p| matches!(p.layer, crate::engine::shapes::Layer::Background) && matches!(p.usage, crate::engine::shapes::Usage::Node)) {
                    bounds.include_path(&path.segments);
                }
                let stretch = match (bounds.min, bounds.max) {
                    (Some(lo), Some(hi)) if hi.x - lo.x > 1e-9 && hi.y - lo.y > 1e-9 => {
                        let c = g.center;
                        Transform::translation(c.x, c.y)
                            .compose(&Transform::scaling(2.0 * w * PT / (hi.x - lo.x), 2.0 * h * PT / (hi.y - lo.y)))
                            .compose(&Transform::translation(-c.x, -c.y))
                    }
                    _ => Transform::default(),
                };
                let at = stretch.apply(g.anchor(&anchor).unwrap_or(g.center));
                // node coordinates are cm, the decoration's pt
                let m = base.compose(&Transform::scaling(1.0 / PT, 1.0 / PT)).compose(&Transform::translation(-at.x, -at.y)).compose(&stretch);
                for path in g.paths.iter().filter(|p| matches!(p.layer, crate::engine::shapes::Layer::Background) && matches!(p.usage, crate::engine::shapes::Usage::Node)) {
                    for seg in &path.segments {
                        match *seg {
                            crate::scene::Segment::MoveTo(q) => self.w.out.move_to(m.apply(q)),
                            crate::scene::Segment::LineTo(q) => self.w.out.line_to(m.apply(q)),
                            crate::scene::Segment::CurveTo(a, b, q) => self.w.out.curve_to(m.apply(a), m.apply(b), m.apply(q)),
                            crate::scene::Segment::Close => self.w.out.close(),
                        }
                    }
                }
            }
            _ => self.rectangle(&t, w, h),
        }
    }

    fn rectangle(&mut self, t: &Transform, w: f64, h: f64) {
        self.mv(t, -w, -h);
        self.ln(t, w, -h);
        self.ln(t, w, h);
        self.ln(t, -w, h);
        self.w.out.close();
    }
}
