//! Points of the basic layer, in the current coordinate system.

use crate::scene::Point;
use crate::tex::{self, Meaning, Token};
use crate::{Error, PT};

use super::super::Engine;
use super::catalog::POINTS;

impl Engine {
    /// A point argument (`\pgfpoint{1cm}{2cm}`, `\pgfpointpolar{30}{1cm}`, a macro that holds one), in
    /// the current coordinate system (untransformed).
    /// The point in `\pgf@x`, `\pgf@y`.
    pub fn pgf_xy(&self) -> Point {
        let get = |r: u32| self.tex.dimens.get(&r).copied().unwrap_or(0) as f64 / 65536.0 * PT;
        Point::new(get(4050), get(4051))
    }

    pub fn set_pgf_xy(&mut self, p: Point) {
        self.tex.dimens.insert(4050, (p.x / PT * 65536.0).round() as i64);
        self.tex.dimens.insert(4051, (p.y / PT * 65536.0).round() as i64);
    }

    pub fn point_arg(&mut self) -> Result<Point, Error> {
        let tokens = self.argument()?;
        self.point_of(tokens)
    }

    pub fn point_of(&mut self, tokens: Vec<Token>) -> Result<Point, Error> {
        let depth = self.tex.depth();
        self.tex.push_frame(tokens);
        let r = self.point_here(depth);
        self.tex.drain_to(depth);
        r
    }

    pub(in crate::engine::basic) fn point_here(&mut self, depth: usize) -> Result<Point, Error> {
        let Some(name) = self.point_command(depth)? else {
            return Ok(self.pgf_xy());
        };
        let arg = |e: &mut Engine| e.tex.read_undelimited();
        let length = |e: &mut Engine, v: Vec<Token>| e.length_of(v);
        Ok(match name.as_str() {
            "pgfpointorigin" => Point::default(),
            "pgfpoint" | "pgfqpoint" => {
                let x = arg(self)?;
                let y = arg(self)?;
                Point::new(length(self, x)?, length(self, y)?)
            }
            "pgfpointpolar" | "pgfqpointpolar" => self.point_polar()?,
            "pgfpointxy" | "pgfqpointxy" => {
                let x = arg(self)?;
                let y = arg(self)?;
                let (x, y) = (self.number_of(x)?, self.number_of(y)?);
                self.st.x.scale(x).plus(self.st.y.scale(y))
            }
            "pgfpointxyz" | "pgfqpointxyz" => {
                let x = arg(self)?;
                let y = arg(self)?;
                let z = arg(self)?;
                let (x, y, z) = (self.number_of(x)?, self.number_of(y)?, self.number_of(z)?);
                self.st.x.scale(x).plus(self.st.y.scale(y)).plus(self.st.z.scale(z))
            }
            "pgfpointpolarxy" => {
                let a = arg(self)?;
                let r = arg(self)?;
                let (angle, radius) = (self.number_of(a)?, self.number_of(r)?);
                let (s, c) = angle.to_radians().sin_cos();
                self.st.x.scale(radius * c).plus(self.st.y.scale(radius * s))
            }
            "pgfpointcylindrical" | "pgfpointspherical" => self.point_spatial(name == "pgfpointcylindrical")?,
            "pgfpointadd" => {
                let p = arg(self)?;
                let q = arg(self)?;
                self.point_of(p)?.plus(self.point_of(q)?)
            }
            "pgfpointdiff" => {
                // the second minus the first
                let p = arg(self)?;
                let q = arg(self)?;
                self.point_of(q)?.minus(self.point_of(p)?)
            }
            "pgfpointscale" | "pgfqpointscale" => {
                let f = arg(self)?;
                let p = arg(self)?;
                let f = self.number_of(f)?;
                self.point_of(p)?.scale(f)
            }
            "pgfpointlineattime" => {
                let t = arg(self)?;
                let p = arg(self)?;
                let q = arg(self)?;
                let t = self.number_of(t)?;
                self.point_of(p)?.lerp(self.point_of(q)?, t)
            }
            "pgfpointlineatdistance" => self.point_at_distance()?,
            "pgfpointcurveattime" => self.point_on_curve()?,
            "pgfpointnormalised" => {
                let p = arg(self)?;
                let p = self.point_of(p)?;
                // pgf normalises to a length of 1pt
                p.scale(PT / p.norm().max(1e-12))
            }
            "pgfpointanchor" | "pgfpointshapeborder" => self.point_on_node(&name)?,
            "pgfpointintersectionoflines" => self.point_crossing()?,
            "pgfpointarcaxesattime" => self.arc_axes_at_time()?.0,
            "pgfpointintersectionsolution" => {
                // the solution (counted from 1) in the current coordinate system
                let k = arg(self)?;
                let k = self.number_of(k)?;
                let p = (k >= 1.0).then(|| self.pgf_solutions.get(k as usize - 1).copied()).flatten();
                let p = p.ok_or_else(|| Error::Figure(format!("no intersection {k}")))?;
                self.st.transform.inverse().apply(p)
            }
            "pgfpointtransformed" => {
                let p = arg(self)?;
                let p = self.point_of(p)?;
                self.st.transform.apply(p)
            }
            _ => Point::default(),
        })
    }

    /// The point command's name, or `None` when the code already left it in `\pgf@x` and `\pgf@y`.
    fn point_command(&mut self, depth: usize) -> Result<Option<String>, Error> {
        // the first token, expanding macros until a point command appears
        let mut guard = 0;
        loop {
            let Some(t) = self.tex.get_token_until(depth) else {
                return Err(Error::Figure("a point is missing".into()));
            };
            if t.is_space() {
                continue;
            }
            match &t {
                Token::Cs(n) if POINTS.contains(&&**n) => return Ok(Some(n.to_string())),
                _ => {
                    let m = self.tex.meaning(&t);
                    if m.is_expandable() && guard < 1000 {
                        guard += 1;
                        self.tex.expand(&t, &m)?;
                        continue;
                    }
                    // pgfmath's computations before the point run first
                    if let Meaning::Native(id) = m {
                        if Engine::is_pgfmath(id) {
                            self.command(id, &t)?;
                            continue;
                        }
                    }
                    // code that leaves the point in `\pgf@x`, `\pgf@y` (a shape's saved anchor)
                    if matches!(m, Meaning::Prim(_) | Meaning::Dimen(_) | Meaning::Native(_)) {
                        let mut code = vec![t];
                        while let Some(x) = self.tex.get_token_until(depth) {
                            code.push(x);
                        }
                        code.push(Token::cs("relax"));
                        self.run_tokens(code)?;
                        return Ok(None);
                    }
                    return Err(Error::Figure(format!("not a point: {}", tex::show_token(&t))));
                }
            }
        }
    }

    /// `\pgfpointpolar`: one radius, or two joined by `and`.
    fn point_polar(&mut self) -> Result<Point, Error> {
        let a = self.tex.read_undelimited()?;
        let r = self.tex.read_undelimited()?;
        let angle = self.number_of(a)?;
        let text = tex::tokens_to_string(&r);
        let (rx, ry) = match text.split_once(" and ") {
            Some((x, y)) => (self.length(x)?, self.length(y)?),
            None => {
                let v = self.length_of(r)?;
                (v, v)
            }
        };
        let (s, c) = angle.to_radians().sin_cos();
        Ok(Point::new(rx * c, ry * s))
    }

    /// `\pgfpointcylindrical` (angle, radius, z) or `\pgfpointspherical` (longitude, latitude, radius).
    fn point_spatial(&mut self, cylindrical: bool) -> Result<Point, Error> {
        let (a, b, c) = (self.tex.read_undelimited()?, self.tex.read_undelimited()?, self.tex.read_undelimited()?);
        let (a, b, c) = (self.number_of(a)?, self.number_of(b)?, self.number_of(c)?);
        let p = if cylindrical {
            let (s, co) = a.to_radians().sin_cos();
            self.st.x.scale(b * co).plus(self.st.y.scale(b * s)).plus(self.st.z.scale(c))
        } else {
            let ((ls, lc), (ts, tc)) = (a.to_radians().sin_cos(), b.to_radians().sin_cos());
            self.st.x.scale(ls).plus(self.st.y.scale(lc)).scale(tc).plus(self.st.z.scale(ts)).scale(c)
        };
        Ok(p)
    }

    /// `\pgfpointlineatdistance`: `d` along the line from the first point to the second.
    fn point_at_distance(&mut self) -> Result<Point, Error> {
        let d = self.tex.read_undelimited()?;
        let p = self.tex.read_undelimited()?;
        let q = self.tex.read_undelimited()?;
        let d = self.length_of(d)?;
        let (p, q) = (self.point_of(p)?, self.point_of(q)?);
        let v = q.minus(p);
        Ok(p.plus(v.scale(d / v.norm().max(1e-12))))
    }

    /// `\pgfpointcurveattime`: the cubic through the next four points, at time `t`.
    fn point_on_curve(&mut self) -> Result<Point, Error> {
        let t = self.tex.read_undelimited()?;
        let t = self.number_of(t)?;
        let mut pts = Vec::new();
        for _ in 0..4 {
            let a = self.tex.read_undelimited()?;
            pts.push(self.point_of(a)?);
        }
        let u = 1.0 - t;
        Ok(pts[0].scale(u * u * u).plus(pts[1].scale(3.0 * u * u * t)).plus(pts[2].scale(3.0 * u * t * t)).plus(pts[3].scale(t * t * t)))
    }

    /// `\pgfpointanchor` or `\pgfpointshapeborder`, brought back into the current coordinate system.
    fn point_on_node(&mut self, name_cmd: &str) -> Result<Point, Error> {
        let name = self.tex.read_undelimited()?;
        let second = self.tex.read_undelimited()?;
        let name = self.tex.expand_to_string(name)?.trim().to_string();
        let canvas = {
            let node = self.nodes.get(&name).cloned().ok_or_else(|| Error::Figure(format!("unknown node: {name}")))?;
            if name.is_empty() {
                Point::default()
            } else if name_cmd == "pgfpointshapeborder" {
                let p = self.point_of(second)?;
                node.border_toward(self.st.transform.apply(p))
            } else {
                let a = self.tex.expand_to_string(second)?;
                self.node_anchor(&node, a.trim()).ok_or_else(|| Error::Figure(format!("unknown anchor: {name}.{}", a.trim())))?
            }
        };
        Ok(self.st.transform.inverse().apply(canvas))
    }

    /// `\pgfpointintersectionoflines`: where the line through the first pair meets the second.
    fn point_crossing(&mut self) -> Result<Point, Error> {
        let mut p = Vec::new();
        for _ in 0..4 {
            let a = self.tex.read_undelimited()?;
            p.push(self.point_of(a)?);
        }
        let (r, s) = (p[1].minus(p[0]), p[3].minus(p[2]));
        let den = r.x * s.y - r.y * s.x;
        if den.abs() < 1e-12 {
            return Err(Error::Figure("intersection of parallel lines".into()));
        }
        let t = ((p[2].x - p[0].x) * s.y - (p[2].y - p[0].y) * s.x) / den;
        Ok(p[0].plus(r.scale(t)))
    }
}
