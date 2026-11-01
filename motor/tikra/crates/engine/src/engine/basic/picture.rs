//! Pictures, text, nodes, transformations and plots of the basic layer.

use crate::scene::{Point, Segment, Transform};
use crate::tex::{self, Meaning, Token};
use crate::{Error, PT};

use super::super::nodes::{NodeSpec, OnPath};
use super::super::shapes::ShapeText;
use super::super::{tikra_tokens, Engine};
use super::catalog::B;

impl Engine {
    /// `\pgfpicture` … `\endpgfpicture`, and the `pgfpicture` environment.
    pub fn basic_picture(&mut self, begin: bool) -> Result<(), Error> {
        if begin {
            self.begin_picture(None)
        } else {
            self.end_picture();
            Ok(())
        }
    }
    pub(in crate::engine::basic) fn extract_coord(&mut self, cmd: B) -> Result<(), Error> {
        // `\pgfextractx{\dimen}{point}`
        let mut target = self.argument()?;
        let p = self.point_arg()?;
        let v = if matches!(cmd, B::ExtractX) { p.x } else { p.y };
        target.extend(tikra_tokens(&format!("={}pt\\relax", v / PT)));
        self.run_tokens(target)?;

        Ok(())
    }
    pub(in crate::engine::basic) fn set_colors(&mut self, cmd: B) -> Result<(), Error> {
        let c = self.argument_string()?;
        match self.color(&c) {
            Some(c) => match cmd {
                B::StrokeColor => self.st.paint.draw_color = Some(c),
                B::FillColor => {
                    self.st.paint.fill_color = Some(c);
                    self.st.paint.fill_pattern = false;
                }
                _ => {
                    self.st.paint.fill_pattern = false;
                    self.st.paint.color = c;
                    self.st.paint.draw_color = None;
                    self.st.paint.fill_color = None;
                    self.st.paint.text_color = None;
                }
            },
            None => self.warn(format!("unknown color '{c}'")),
        }

        Ok(())
    }

    pub(in crate::engine::basic) fn transform_cm(&mut self, t: Transform) -> Result<(), Error> {
        let mut n = [0.0; 4];
        for k in &mut n {
            let v = self.argument()?;
            *k = self.number_of(v)?;
        }
        let p = self.point_arg()?;
        self.st.transform = t.compose(&Transform { a: n[0], b: n[1], c: n[2], d: n[3], e: p.x, f: p.y });

        Ok(())
    }

    pub(in crate::engine::basic) fn place_text(&mut self, t: Transform) -> Result<(), Error> {
        // \pgftext[left,base,at=…,rotate=…]{text}
        let o = self.optional()?.unwrap_or_default();
        let text = self.argument()?;
        let mut anchor_x = "";
        let mut anchor_y = "";
        let mut at = Point::default();
        let mut rotate = 0.0;
        for item in crate::keys::split_list(&o) {
            match (item.key.as_str(), item.value) {
                ("left", None) => anchor_x = "west",
                ("right", None) => anchor_x = "east",
                ("top", None) => anchor_y = "north",
                ("bottom", None) => anchor_y = "south",
                ("base", None) => anchor_y = "base",
                ("at", Some(v)) => at = self.point_of(v)?,
                ("x", Some(v)) => at.x += self.length_of(v)?,
                ("y", Some(v)) => at.y += self.length_of(v)?,
                ("rotate", Some(v)) => rotate = self.number_of(v)?,
                _ => {}
            }
        }
        let anchor = match (anchor_y, anchor_x) {
            ("", "") => "center".to_string(),
            ("", x) => x.to_string(),
            (y, "") => y.to_string(),
            (y, x) => format!("{y} {x}"),
        };
        // under a nonlinear transformation, the text goes where its point goes
        let place = if self.st.nonlinear.is_empty() { t.apply(at) } else { self.nonlinear_point(t.apply(at))? };
        // a shading as the text: its box, placed by the anchor (its baseline is its bottom)
        let words: Vec<&Token> = text.iter().filter(|x| !x.is_space()).collect();
        if let [Token::Cs(cmd), ..] = words[..] {
            if &**cmd == "pgfuseshading" {
                let name = crate::tex::tokens_to_string(&text).replace("\\pgfuseshading", "").replace(['{', '}'], "");
                if let Some((w, h)) = self.shading_half_size(name.trim()) {
                    let dx = if anchor.contains("west") {
                        w
                    } else if anchor.contains("east") {
                        -w
                    } else {
                        0.0
                    };
                    let dy = if anchor.contains("north") {
                        -h
                    } else if anchor.contains("south") || anchor.contains("base") {
                        h
                    } else {
                        0.0
                    };
                    let s = Transform::translation(place.x, place.y).compose(&Transform::rotation(rotate)).compose(&Transform::translation(dx, dy));
                    return self.draw_shading(name.trim(), &s);
                }
            }
        }
        // in a declared shape's code: a text of the shape
        if let Some(c) = &mut self.shape_capture {
            use crate::extension::TextAnchor as A;
            let anchor = match anchor.as_str() {
                "west" => A::West,
                "east" => A::East,
                "north" => A::North,
                "south" => A::South,
                "base" => A::Base,
                "north west" => A::NorthWest,
                "north east" => A::NorthEast,
                "south west" => A::SouthWest,
                "south east" => A::SouthEast,
                "base west" => A::BaseWest,
                "base east" => A::BaseEast,
                _ => A::Center,
            };
            let font = Some(self.st.text.font.clone());
            c.texts.push(ShapeText { tex: tex::tokens_to_string(&text), at: place, color: None, anchor: Some(anchor), font, turn: rotate, upright: false });
            return Ok(());
        }
        // the text is scaled and turned by the transformation in force, as pgf boxes it
        let linear = t.a != 1.0 || t.b != 0.0 || t.c != 0.0 || t.d != 1.0;
        let shape = if linear { ",transform shape" } else { "" };
        let options = tikra_tokens(&format!("shape=rectangle,inner sep=0pt,outer sep=0pt,anchor={anchor},rotate={rotate}{shape}"));
        let items = self.node(NodeSpec { options, name: None, at: Some(place), text, place: OnPath::Point(place), point: place, coordinate: false, default_pos: None })?;
        self.canvas.items.extend(items);

        Ok(())
    }

    pub(in crate::engine::basic) fn pgf_node(&mut self, cmd: B, t: Transform) -> Result<(), Error> {
        // \pgfnode{shape}{anchor}{text}{name}{path usage}; \pgfmultipartnode has no text
        // (its parts are boxes, empty here)
        let shape = self.argument_string()?;
        let anchor = self.argument_string()?;
        let text = if matches!(cmd, B::Node) { self.argument()? } else { self.node_part_boxes() };
        let name = self.argument_string()?;
        let usage = self.argument()?;
        let usage = tex::tokens_to_string(&usage);
        // pgf's nodes are in the whole transformation, not only its shift
        let mut opts = format!("shape={shape},anchor={anchor},transform shape");
        if usage.contains("stroke") || usage.contains("draw") {
            opts.push_str(",draw");
        }
        if usage.contains("fill") {
            opts.push_str(",fill");
        }
        let place = t.apply(Point::default());
        let name = (!name.is_empty()).then_some(name);
        let items =
            self.node(NodeSpec { options: tikra_tokens(&opts), name, at: Some(place), text, place: OnPath::Point(place), point: place, coordinate: false, default_pos: None })?;
        self.canvas.items.extend(items);

        Ok(())
    }

    pub(in crate::engine::basic) fn place_coordinate(&mut self, t: Transform) -> Result<(), Error> {
        let name = self.argument_string()?;
        let p = self.point_arg()?;
        let place = t.apply(p);
        let items = self.node(NodeSpec {
            options: Vec::new(),
            name: Some(name),
            at: Some(place),
            text: Vec::new(),
            place: OnPath::Point(place),
            point: place,
            coordinate: true,
            default_pos: None,
        })?;
        self.canvas.items.extend(items);

        Ok(())
    }

    pub(in crate::engine::basic) fn plot_function(&mut self, t: Transform) -> Result<(), Error> {
        // \pgfplotfunction{\x}{list}{point}: a stream of the point for each item
        let var = self.argument()?;
        let list = self.argument()?;
        let point = self.argument()?;
        let list = self.tex.expand_fully(list)?;
        let name = var.into_iter().find(|t| matches!(t, Token::Cs(_)));
        self.plot_stream.clear();
        for item in super::super::foreach_items(&list) {
            if let Some(n) = &name {
                self.tex.set_meaning(n, super::super::macro_tokens(item), false);
            }
            let p = self.point_of(point.clone())?;
            self.plot_stream.push(t.apply(p));
        }
        self.plot_stream_end()?;

        Ok(())
    }

    pub(in crate::engine::basic) fn transform_triangle(&mut self, t: Transform) -> Result<(), Error> {
        // the origin goes to the first point, (1pt,0pt) to the second, (0pt,1pt) to the third
        let o = self.point_arg()?;
        let x = self.point_arg()?;
        let y = self.point_arg()?;
        let (x, y) = (x.minus(o).scale(1.0 / PT), y.minus(o).scale(1.0 / PT));
        self.st.transform = t.compose(&Transform { a: x.x, b: x.y, c: y.x, d: y.y, e: o.x, f: o.y });

        Ok(())
    }

    pub(in crate::engine::basic) fn transform_arrow(&mut self, t: Transform) -> Result<(), Error> {
        // at the end point, the x axis pointing away from the start
        let from = self.point_arg()?;
        let to = self.point_arg()?;
        let d = to.minus(from);
        let d = if d.norm() > 1e-12 { d.scale(1.0 / d.norm()) } else { Point::new(1.0, 0.0) };
        self.st.transform = t.compose(&Transform { a: d.x, b: d.y, c: -d.y, d: d.x, e: to.x, f: to.y });

        Ok(())
    }

    pub(in crate::engine::basic) fn transform_curve(&mut self) -> Result<(), Error> {
        let time = self.argument()?;
        let s = self.number_of(time)?;
        let mut c = [Point::default(); 4];
        for p in &mut c {
            *p = self.point_arg()?;
        }
        let u = 1.0 - s;
        let at = c[0].scale(u * u * u).plus(c[1].scale(3.0 * u * u * s)).plus(c[2].scale(3.0 * u * s * s)).plus(c[3].scale(s * s * s));
        let tangent = c[1].minus(c[0]).scale(u * u).plus(c[2].minus(c[1]).scale(2.0 * u * s)).plus(c[3].minus(c[2]).scale(s * s));
        self.transform_at_time(at, tangent);

        Ok(())
    }

    pub(in crate::engine::basic) fn write_adjustments(&mut self, t: Transform) -> Result<(), Error> {
        // how much a horizontal and a vertical length shrink to come out at their size
        let h = 1.0 / t.linear(Point::new(1.0, 0.0)).norm().max(1e-12);
        let v = 1.0 / t.linear(Point::new(0.0, 1.0)).norm().max(1e-12);
        for (name, value) in [("pgfhorizontaltransformationadjustment", h), ("pgfverticaltransformationadjustment", v)] {
            self.tex.set_meaning(&Token::cs(name), super::super::macro_tokens(tikra_tokens(&format!("{value}"))), false);
        }

        Ok(())
    }

    pub(in crate::engine::basic) fn low_level_object(&mut self) -> Result<(), Error> {
        let code = self.argument()?;
        let body = self.argument()?;
        self.begin_group();
        self.begin_scope();
        let r = self.low_level(code).and_then(|()| self.run_tokens(body));
        self.end_scope();
        self.end_group();
        r?;

        Ok(())
    }
    pub(in crate::engine::basic) fn declare_plot_handler(&mut self) -> Result<(), Error> {
        // \pgfdeclareplothandler{\macro}{#1…}{point=code}: \macro{args} makes it the
        // plot handler, the code run at each point
        let name = self.argument()?;
        let params = self.argument()?;
        let options = self.argument()?;
        let Some(Token::Cs(name)) = name.into_iter().find(|t| matches!(t, Token::Cs(_))) else {
            return Ok(());
        };
        let n = params.iter().filter(|t| matches!(t, Token::Char(_, tex::Cat::Param))).count();
        let mut point = String::new();
        for item in crate::keys::split_list(&options) {
            if item.key.trim() == "point" {
                point = tex::tokens_to_string(&item.value.unwrap_or_default());
            }
        }
        self.plot_handlers.insert(name.to_string(), point);
        let args: String = (1..=n).map(|k| format!("{{#{k}}}")).collect();
        let params: String = (1..=n).map(|k| format!("#{k}")).collect();
        self.run_tokens(tikra_tokens(&format!("\\def\\{name}{params}{{\\tikra@plothandler{{{name}}}{{{args}}}}}")))?;

        Ok(())
    }

    pub(in crate::engine::basic) fn dec_to_base(&mut self) -> Result<(), Error> {
        // \pgfmathdectobase{\macro}{number}{base}: its digits, padded with zeros to
        // \pgfmathbasenumberlength
        let target = self.argument()?;
        let n = self.argument()?;
        let base = self.argument()?;
        let (n, base) = (self.number_of(n)? as i64, (self.number_of(base)? as u32).clamp(2, 36));
        let mut digits = Vec::new();
        let mut m = n.unsigned_abs();
        while m > 0 || digits.is_empty() {
            digits.push(char::from_digit((m % u64::from(base)) as u32, base).unwrap_or('0'));
            m /= u64::from(base);
        }
        let length = self.tex.expand_to_string(tikra_tokens("\\pgfmathbasenumberlength")).ok().and_then(|s| s.trim().parse::<usize>().ok()).unwrap_or(1);
        while digits.len() < length {
            digits.push('0');
        }
        let s: String = digits.iter().rev().collect();
        if let Some(name) = target.iter().find(|t| matches!(t, Token::Cs(_))) {
            self.tex.set_meaning(name, super::super::macro_from(&s), false);
        }

        Ok(())
    }

    pub(in crate::engine::basic) fn get_last_xy(&mut self) -> Result<(), Error> {
        // \pgfgetlastxy{\x}{\y}: the last point of the path, on the canvas, in pt
        let p = self.pgf_path.last.unwrap_or(self.last_xy);
        for v in [p.x, p.y] {
            let target = self.argument()?;
            if let Some(name) = target.iter().find(|t| matches!(t, Token::Cs(_))) {
                let sp = (v / crate::PT * 65536.0).round() as i64;
                self.tex.set_meaning(name, super::super::macro_from(&crate::tex::dimen::show(sp)), false);
            }
        }

        Ok(())
    }

    pub(in crate::engine::basic) fn write_digit_list(&mut self) -> Result<(), Error> {
        // \pgfmathtodigitlist{\macro}{number}: its digits as a list
        let target = self.argument()?;
        let n = self.argument_string()?;
        let list = n.trim().chars().map(String::from).collect::<Vec<_>>().join(",");
        if let Some(name) = target.iter().find(|t| matches!(t, Token::Cs(_))) {
            self.tex.set_meaning(name, super::super::macro_from(&list), false);
        }

        Ok(())
    }

    pub(in crate::engine::basic) fn set_key_family(&mut self) -> Result<(), Error> {
        // keyval's \setkeys{family}{key=value,…}: the code \define@key gave each key
        let family = self.argument_string()?;
        let list = self.argument()?;
        for item in crate::keys::split_list(&list) {
            let mut code = vec![Token::cs(&format!("KV@{family}@{}", item.key.trim()))];
            code.push(Token::Char('{', tex::Cat::Begin));
            code.extend(item.value.unwrap_or_default());
            code.push(Token::Char('}', tex::Cat::End));
            self.run_tokens(code)?;
        }

        Ok(())
    }
    /// `\pgfpointarcaxesattime{time}{center}{0° axis}{90° axis}{start}{end}`: the point of the arc at
    /// that time, and the direction the arc goes there.
    pub(in crate::engine::basic) fn arc_axes_at_time(&mut self) -> Result<(Point, Point), Error> {
        let time = self.argument()?;
        let time = self.number_of(time)?;
        let c = self.point_arg()?;
        let x = self.point_arg()?;
        let y = self.point_arg()?;
        let a0 = self.argument()?;
        let a1 = self.argument()?;
        let (a0, a1) = (self.number_of(a0)?, self.number_of(a1)?);
        let (s, co) = (a0 + (a1 - a0) * time).to_radians().sin_cos();
        let tangent = y.scale(co).minus(x.scale(s));
        let tangent = if a0 > a1 { tangent.scale(-1.0) } else { tangent };
        Ok((c.plus(x.scale(co)).plus(y.scale(s)), tangent))
    }

    /// The coordinate system of a point on a line or curve: shifted there; with `\pgfslopedattimetrue`
    /// turned along `tangent` (kept upright unless `\pgfallowupsidedownattimetrue`); with
    /// `\pgfresetnontranslationattimetrue` without the rotations and scalings before.
    pub(in crate::engine::basic) fn transform_at_time(&mut self, at: Point, tangent: Point) {
        let on = |e: &Engine, name: &str| e.tex.meaning_of(name) == Meaning::Prim(tex::Prim::Iftrue);
        let mut t = self.st.transform.compose(&Transform::translation(at.x, at.y));
        if on(self, "ifpgfresetnontranslationattime") {
            t = Transform { a: 1.0, b: 0.0, c: 0.0, d: 1.0, ..t };
        }
        if on(self, "ifpgfslopedattime") {
            let mut d = tangent;
            if !on(self, "ifpgfallowupsidedownattime") && d.x < 0.0 {
                d = d.scale(-1.0);
            }
            let d = if d.norm() > 1e-12 { d.scale(1.0 / d.norm()) } else { Point::new(1.0, 0.0) };
            t = t.compose(&Transform { a: d.x, b: d.y, c: -d.y, d: d.x, e: 0.0, f: 0.0 });
        }
        self.st.transform = t;
    }

    /// `\pgflowlevel{code}`: the transformation the code makes (from none) applies to the canvas, up
    /// to the end of the scope, line widths and text included.
    pub(in crate::engine::basic) fn low_level(&mut self, code: Vec<Token>) -> Result<(), Error> {
        self.begin_group();
        self.st.transform = Transform::default();
        let r = self.run_tokens(code);
        let t = self.st.transform;
        self.end_group();
        r?;
        self.open_in_scope(crate::canvas::Item::Transform { transform: t });
        Ok(())
    }

    /// A multipart node's text from the boxes `\setbox\pgfnodepart<name>box=\hbox{…}` filled: the
    /// text part first, then the others with `\nodepart{name}`.
    fn node_part_boxes(&mut self) -> Vec<Token> {
        let boxes = std::mem::take(&mut self.tex.named_boxes);
        let mut parts: Vec<(String, Vec<Token>)> = boxes.iter().filter_map(|(n, b)| Some((n.strip_prefix("pgfnodepart")?.strip_suffix("box")?.to_string(), b.clone()))).collect();
        parts.sort_by_key(|(n, _)| n != "text");
        let mut text = Vec::new();
        for (name, body) in parts {
            if name != "text" {
                text.extend(tikra_tokens(&format!("\\nodepart{{{name}}}")));
            }
            text.extend(body);
        }
        self.tex.named_boxes = boxes;
        text
    }
    /// `\pgfplotstreamend`: the points streamed go into the path as the plot handler makes it, or
    /// each gets the mark handler's code, shifted there.
    pub(in crate::engine::basic) fn plot_stream_end(&mut self) -> Result<(), Error> {
        let points = std::mem::take(&mut self.plot_stream);
        let (handler, code) = self.st.plot.basic_plot.clone();
        match handler.as_str() {
            "discard" => {}
            "record" => {
                // the stream as commands in the macro, its points in the coordinate system
                let inv = self.st.transform.inverse();
                let mut v = tikra_tokens("\\pgfplotstreamstart");
                for p in points {
                    let q = inv.apply(p);
                    v.extend(tikra_tokens(&format!("\\pgfplotstreampoint{{\\pgfqpoint{{{}pt}}{{{}pt}}}}", q.x / PT, q.y / PT)));
                }
                v.extend(tikra_tokens("\\pgfplotstreamend"));
                if let Some(m) = code.iter().find(|t| matches!(t, Token::Cs(_))) {
                    self.tex.set_meaning(m, super::super::macro_tokens(v), true);
                }
            }
            h if h.starts_with("custom:") => {
                // a declared handler: its code at each point, `##1` the point, `#k` its arguments
                let code = self.plot_handlers.get(&h["custom:".len()..]).cloned().unwrap_or_default();
                let inv = self.st.transform.inverse();
                let params = super::super::options::split_arguments(&self.st.plot.basic_plot.1, 9);
                for p in points {
                    let q = inv.apply(p);
                    let mut text = code.replace("##1", &format!("\\pgfqpoint{{{}pt}}{{{}pt}}", q.x / PT, q.y / PT));
                    for (k, a) in params.iter().enumerate() {
                        text = text.replace(&format!("#{}", k + 1), &tex::tokens_to_string(a));
                    }
                    self.run_tokens(tikra_tokens(&text))?;
                }
            }
            h if h == "mark" || h.starts_with("mark listed:") => {
                let listed: Option<Vec<usize>> = h.strip_prefix("mark listed:").map(|l| l.split(',').filter_map(|i| i.trim().parse().ok()).collect());
                let t = self.st.transform;
                for (i, p) in points.into_iter().enumerate() {
                    if listed.as_ref().is_some_and(|l| !l.contains(&(i + 1))) {
                        continue;
                    }
                    self.begin_group();
                    self.begin_scope();
                    self.st.transform = Transform::translation(p.x, p.y).compose(&Transform { e: 0.0, f: 0.0, ..t });
                    let r = self.run_tokens(code.clone());
                    self.end_scope();
                    self.end_group();
                    r?;
                }
            }
            h => {
                let saved = std::mem::replace(&mut self.st.plot.plot_handler, h.to_string());
                let r = super::super::path::plot_segments(self, &points);
                self.st.plot.plot_handler = saved;
                let mut segs = r?;
                // \pgfsetlinetofirstplotpoint: the plot goes on from the path's last point
                if self.st.plot.plot_first_lineto && self.pgf_path.last.is_some() {
                    if let Some(Segment::MoveTo(p)) = segs.first().copied() {
                        segs[0] = Segment::LineTo(p);
                    }
                }
                for s in segs {
                    self.pgf_add(s);
                }
            }
        }
        Ok(())
    }
}
