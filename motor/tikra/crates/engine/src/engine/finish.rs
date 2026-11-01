//! Finishing a path (`\\tikz@finish`): decorate, preactions, the path, postactions.

use std::rc::Rc;

use crate::scene::{Point, Segment, Transform};
use crate::tex::Token;
use crate::Error;

use super::state::{GState, Mode};
use super::{tikra_tokens, Engine};

impl Engine {
    /// Applies the current decoration when a piece does that, or leaves the segments as they are.
    pub fn decorate_segments(&mut self, segs: &[Segment], existing: Option<Point>) -> Result<Vec<Segment>, Error> {
        match self.decorator() {
            Some(decorate) => decorate(self, segs, existing),
            None => Ok(segs.to_vec()),
        }
    }

    /// `\tikz@finish`: decorates the path, runs the preactions, uses the path, runs the
    /// postactions. `outer` is the state before the path's options.
    pub fn finish_path(&mut self, segments: Vec<Segment>, outer: &GState, command: &str) -> Result<(), Error> {
        let mut segs = segments;
        let mut b = crate::canvas::Canvas::default();
        b.include_path(&segs);
        self.st.path_box = b.min.zip(b.max);
        if self.st.decorate && !segs.is_empty() && segs.iter().all(super::path::valid_segment) {
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
            self.pgf.named_paths.insert(name, segs.clone());
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
                let items = self.node(super::nodes::NodeSpec {
                    options: tikra_tokens(&size),
                    name: Some("path picture bounding box".into()),
                    at: Some(at),
                    text: Vec::new(),
                    place: super::nodes::OnPath::Point(at),
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
}

/// The graphic settings that TikZ installs when a path is used (`\tikz@options`), not when its
/// options are read: decorations and preactions happen before, with the scope's.
fn take_graphics(st: &mut GState, from: &GState) {
    if st.paint.text_color.is_none() && st.paint.color != from.paint.color {
        // `color=` sets the text color at once
        st.paint.text_color = Some(st.paint.color);
    }
    st.paint.color = from.paint.color;
    st.paint.draw_color = from.paint.draw_color;
    st.paint.fill_color = from.paint.fill_color;
    st.paint.pattern_color = from.paint.pattern_color;
    st.pen.line_width = from.pen.line_width;
    st.pen.dash = from.pen.dash.clone();
    st.pen.cap = from.pen.cap;
    st.pen.join = from.pen.join;
    st.pen.miter_limit = from.pen.miter_limit;
    st.paint.draw_opacity = from.paint.draw_opacity;
    st.paint.fill_opacity = from.paint.fill_opacity;
    st.paint.even_odd = from.paint.even_odd;
}
