//! Applying a decoration to a path: `\tikz@finish`, pre- and postactions, and the basic layer's
//! `\pgfdecoration`.

use std::rc::Rc;

use crate::canvas::{transform_segments, Item};
use crate::keys;
use crate::scene::{Point, Segment, Transform};
use crate::tex::{Cat, Token};
use crate::{Error, PT};

use super::super::state::{GState, Mode};
use super::super::{tikra_tokens, Engine};
use super::library::Run;
use super::walker::{self, Walker};
use super::{library, sp, split_commas, take_graphics, Env, How};

impl Engine {
    /// `\tikz@finish`: decorates the path, runs the preactions, uses the path, runs the
    /// postactions. `outer` is the state before the path's options.
    pub fn finish_path(&mut self, segments: Vec<Segment>, outer: &GState, command: &str) -> Result<(), Error> {
        let mut segs = segments;
        let mut b = crate::canvas::Canvas::default();
        b.include_path(&segs);
        self.st.path_box = b.min.zip(b.max);
        if self.st.decorate && !segs.is_empty() && segs.iter().all(super::super::path::valid_segment) {
            // pgf counts a path in the picture's size as it is built, before it is decorated
            if self.counts_for_size() {
                self.canvas.include_path(&segs);
            }
            self.begin_group();
            take_graphics(&mut self.st, outer);
            let r = self.decorate_segments(&segs, None);
            self.end_group();
            segs = r?;
        }
        // a name the path still has (a pre- or postaction's `name path`) is the decorated path's
        if let Some(name) = self.st.name_path.take() {
            self.named_paths.insert(name, segs.clone());
        }
        let pre = std::mem::take(&mut self.st.preactions);
        let post = std::mem::take(&mut self.st.postactions);
        for a in pre.iter() {
            self.action(&segs, a.clone(), Some(outer), command)?;
        }
        match self.st.mode.picture.take() {
            Some(code) => self.path_picture(&segs, code, command)?,
            None => self.use_path(segs.clone(), command)?,
        }
        for a in post.iter() {
            self.action(&segs, a.clone(), None, command)?;
        }
        Ok(())
    }

    /// A path with a `path picture`, in `\tikz@finish`'s order: the fill, the picture clipped to the
    /// path (with the node `path picture bounding box` around it), then the stroke.
    fn path_picture(&mut self, segs: &[Segment], code: Vec<Token>, command: &str) -> Result<(), Error> {
        let mode = self.st.mode.clone();
        if mode.fill || mode.shade || mode.pattern {
            self.st.mode = Mode { draw: false, clip: false, ..mode.clone() };
            self.use_path(segs.to_vec(), command)?;
        }
        let mut c = crate::canvas::Canvas::default();
        c.include_path(segs);
        if let (Some(lo), Some(hi)) = (c.min, c.max) {
            self.begin_group();
            self.begin_scope();
            let r = (|| {
                self.open_in_scope(crate::canvas::Item::Clip { segments: segs.to_vec(), even_odd: self.st.paint.even_odd });
                if let Some(s) = self.scopes.last_mut() {
                    s.clipped = true;
                }
                self.st.mode = Mode::default();
                let at = lo.lerp(hi, 0.5);
                let size = format!("shape=rectangle,inner sep=0pt,outer sep=0pt,minimum width={}pt,minimum height={}pt", (hi.x - lo.x) / crate::PT, (hi.y - lo.y) / crate::PT);
                let items = self.node(super::super::nodes::NodeSpec {
                    options: tikra_tokens(&size),
                    name: Some("path picture bounding box".into()),
                    at: Some(at),
                    text: Vec::new(),
                    place: super::super::nodes::OnPath::Point(at),
                    point: at,
                    coordinate: false,
                    default_pos: None,
                })?;
                self.canvas.items.extend(items);
                self.run_tokens(code)
            })();
            self.end_scope();
            self.end_group();
            r?;
        }
        if mode.draw || mode.clip {
            self.st.mode = Mode { fill: false, shade: false, pattern: false, ..mode };
            self.use_path(segs.to_vec(), command)?;
        }
        Ok(())
    }

    /// `\tikz@extra@preaction`, `\tikz@extra@postaction`: `\path[options]` on the same path.
    fn action(&mut self, segs: &[Segment], options: Vec<Token>, graphics: Option<&GState>, command: &str) -> Result<(), Error> {
        self.begin_group();
        self.begin_scope();
        let r = (|| {
            if let Some(g) = graphics {
                // the path's own colors stay known as `\tikz@fillcolor` and `\tikz@strokecolor` (the shadows library copies them)
                let (fill, stroke) = (self.st.mode.fill.then(|| self.st.fill_paint_color()), self.st.mode.draw.then(|| self.st.stroke_color()));
                for (name, c) in [("fillcolor", fill), ("strokecolor", stroke)] {
                    match c {
                        Some(c) => {
                            self.colors.insert(format!("tikra@{name}"), c);
                            self.tex.define_string(&format!("tikz@{name}"), &format!("tikra@{name}"), false);
                        }
                        None => self.tex.define_string(&format!("tikz@{name}"), "none", false),
                    }
                }
                take_graphics(&mut self.st, g);
            }
            let outer = self.st.clone();
            self.st.mode = Mode::default();
            self.st.decorate = false;
            self.st.preactions = Rc::default();
            self.st.postactions = Rc::default();
            self.st.pending = Transform::default();
            self.process_keys(tikra_tokens("every path/.try"), Some("/tikz"))?;
            self.process_keys(options, Some("/tikz"))?;
            self.finish_path(segs.to_vec(), &outer, command)
        })();
        self.end_scope();
        self.end_group();
        r
    }

    /// Decorates segments (cm) with the current decoration (`\tikz@lib@dec@decorate@path` and
    /// `\pgfdecoration`). `existing` is the point before them, for a path that does not start
    /// with a moveto.
    pub fn decorate_segments(&mut self, segs: &[Segment], existing: Option<Point>) -> Result<Vec<Segment>, Error> {
        let Some(name) = self.st.decoration.name.clone() else {
            return Ok(segs.to_vec());
        };
        if let Some(decoration) = self.piece_decoration(name.trim()) {
            return decoration(self, segs);
        }
        if library::resolve_meta(self, &name).is_none() && library::resolve(self, &name).is_none() {
            self.warn(format!("decoration '{name}' not supported: path drawn undecorated"));
            return Ok(segs.to_vec());
        }
        if self.st.pen.rounded != 0.0 || self.st.pen.rounded_out != 0.0 {
            // the round corners of the path are made before it is decorated (`\pgfprocessround`)
            let rounded = super::super::output::round_corners(segs, self.st.pen.rounded, self.st.pen.rounded_out);
            return self.decorate_how(&rounded, existing, How::Tikz(name));
        }
        self.decorate_how(segs, existing, How::Tikz(name))
    }

    /// Decorates segments (cm): `\pgfdecoration` with a list, `\pgfmetadecoration`, or TikZ's way.
    fn decorate_how(&mut self, segs: &[Segment], existing: Option<Point>, how: How) -> Result<Vec<Segment>, Error> {
        let existing = existing.unwrap_or_default().scale(1.0 / PT);
        let mut pts = transform_segments(segs, &Transform::scaling(1.0 / PT, 1.0 / PT));
        // `path from text`: a lone moveto becomes a line as long as the text
        let mut prepared = None;
        if let (How::Tikz(name), [Segment::MoveTo(q)]) = (&how, pts.as_slice()) {
            if name.trim() == "text effects along path" && self.st.decoration.te.path_from_text {
                let q = *q;
                let probe = Walker::new(walker::parse(&[Segment::MoveTo(q), Segment::LineTo(Point::new(q.x + 1.0, q.y))], q), q);
                let mut run = Run::new(self, probe, self.st.decoration.clone());
                run.te_prepare()?;
                let width: f64 = run.te.chars.iter().map(|c| c.pre + c.post).sum();
                let te = std::mem::take(&mut run.te);
                let a = self.st.decoration.te.path_from_text_angle.to_radians();
                pts.push(Segment::LineTo(Point::new(q.x + width * a.cos(), q.y + width * a.sin())));
                prepared = Some(te);
            }
        }
        // a lone moveto becomes a tiny line
        if let [Segment::MoveTo(q)] = pts.as_slice() {
            let q = *q;
            pts.push(Segment::LineTo(Point::new(q.x + 0.0001, q.y)));
        }
        if !pts.iter().any(|s| !matches!(s, Segment::MoveTo(_))) {
            return Ok(Vec::new());
        }
        let mut parsed = walker::parse(&pts, existing);
        self.process_keys(tikra_tokens("/pgf/every decoration/.try"), Some("/pgf/decoration"))?;
        let s = self.st.decoration.clone();
        if s.reverse {
            parsed.objs = walker::reverse(&parsed.objs);
        }
        let w = Walker::new(parsed, existing);
        let mut run = Run::new(self, w, s);
        if let Some(te) = prepared {
            run.te = te;
        }
        match how {
            How::Tikz(name) => {
                let pre = run.key_text("pre length");
                let post = run.key_text("post length");
                if let Some(m) = library::resolve_meta(run.e, &name) {
                    run.run_meta(m, None)?;
                } else {
                    let dec = library::resolve(run.e, &name).unwrap_or(library::Dec::LineTo);
                    if pre != "0pt" || post != "0pt" {
                        run.run_meta(library::Meta::Internal, Some(dec))?;
                    } else {
                        let total = run.w.total;
                        run.invoke(dec, total)?;
                    }
                }
            }
            How::Meta(name) => match library::resolve_meta(run.e, &name) {
                Some(m) => run.run_meta(m, None)?,
                None => run.e.warn(format!("meta-decoration '{}' not supported: path left out", name.trim())),
            },
            How::List(list) => {
                for item in split_commas(&list) {
                    let a = super::super::options::split_arguments(&item, 4);
                    let name = run.e.tex.expand_to_string(a[0].clone())?;
                    let (total, rem) = (run.w.total, run.w.rem_total);
                    run.e.tex.dimens.insert(4010, sp(total));
                    run.e.tex.dimens.insert(4012, sp(rem));
                    let dist = run.e.length_of(a[1].clone())? / PT;
                    let code = |v: &Vec<Token>| {
                        if keys::trim(v).is_empty() {
                            None
                        } else {
                            Some(v.clone())
                        }
                    };
                    match library::resolve(run.e, &name) {
                        Some(dec) => run.invoke_with(dec, dist, code(&a[2]), code(&a[3]))?,
                        None => run.e.warn(format!("decoration '{}' not supported: left out", name.trim())),
                    }
                }
            }
        }
        let out = std::mem::take(&mut run.w.out.segs);
        Ok(transform_segments(&out, &Transform::scaling(PT, PT)))
    }

    /// `\pgfdecoration{list}` and `\pgfmetadecoration{name}` (`meta`): the path built until the end
    /// is decorated, then added to the path there was.
    pub fn decoration_begin(&mut self) -> Result<(), Error> {
        let list = self.argument()?;
        let meta = self.argument_string()? == "1";
        let existing = std::mem::take(&mut self.pgf_path);
        self.pgf_path = super::super::basic::PgfPath { segments: Vec::new(), last: existing.last, start: existing.start, quick: false };
        self.decoration_envs.push(Env { list, meta, existing });
        self.begin_group();
        Ok(())
    }

    pub fn decoration_end(&mut self) -> Result<(), Error> {
        self.end_group();
        let Some(env) = self.decoration_envs.pop() else {
            self.warn("\\endpgfdecoration without \\pgfdecoration: ignored");
            return Ok(());
        };
        let decorated = std::mem::take(&mut self.pgf_path.segments);
        let last = env.existing.last;
        self.pgf_path = env.existing;
        if decorated.is_empty() {
            self.warn("I cannot decorate an empty path");
            return Ok(());
        }
        let how = if env.meta { How::Meta(self.tex.expand_to_string(env.list)?) } else { How::List(env.list) };
        self.begin_group();
        let r = self.decorate_how(&decorated, last, how);
        self.end_group();
        self.add_to_pgf_path(r?);
        Ok(())
    }

    /// `\pgfdecoratecurrentpath{name}`: the basic layer's path, decorated.
    pub fn decorate_current_path(&mut self) -> Result<(), Error> {
        let name = self.argument()?;
        let path = std::mem::take(&mut self.pgf_path);
        let mut list = vec![Token::Char('{', Cat::Begin)];
        list.extend(name);
        list.push(Token::Char('}', Cat::End));
        list.extend(tikra_tokens("{\\pgfdecoratedpathlength}"));
        self.begin_group();
        let r = self.decorate_how(&path.segments, None, How::List(list));
        self.end_group();
        self.add_to_pgf_path(r?);
        Ok(())
    }

    fn add_to_pgf_path(&mut self, segs: Vec<Segment>) {
        let path = &mut self.pgf_path;
        for s in segs {
            match s {
                Segment::MoveTo(q) => {
                    path.start = Some(q);
                    path.last = Some(q);
                }
                Segment::LineTo(q) | Segment::CurveTo(_, _, q) => path.last = Some(q),
                Segment::Close => path.last = path.start,
            }
            path.segments.push(s);
        }
    }

    /// `\arrow[options]{tip}` and `\arrowreversed` in a mark: the tip at the origin, pointing right.
    pub fn arrow_command(&mut self, reversed: bool) -> Result<(), Error> {
        let options = self.optional()?;
        let spec = self.argument_string()?;
        self.begin_group();
        let r = (|| {
            if let Some(o) = options {
                self.process_keys(o, Some("/tikz"))?;
                self.st.apply_pending();
            }
            let stroke = self.stroke();
            let d = Point::new(if reversed { -1.0 } else { 1.0 }, 0.0);
            let mut tips = Vec::new();
            super::super::arrows::tip_items(&super::super::arrows::TipMark { spec: &spec, at: Point::default(), direction: d, stroke: &stroke }, &mut tips);
            let t = self.st.transform;
            for item in tips {
                if let Item::Path { segments, fill, stroke } = item {
                    let segments = transform_segments(&segments, &t);
                    if self.counts_for_size() {
                        self.canvas.include_path(&segments);
                    }
                    self.canvas.push(Item::Path { segments, fill, stroke });
                }
            }
            Ok(())
        })();
        self.end_group();
        r
    }
}
