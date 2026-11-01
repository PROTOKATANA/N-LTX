//! Opening and closing pictures and the environments that contain them.

use super::*;
use crate::canvas::{Canvas, Item};
use crate::scene::Point;
use crate::tex::{self, Cat, Meaning, Token};
use crate::Error;

impl Engine {
    // ---------------------------------------------------------------- pictures and environments

    pub fn begin_picture(&mut self, o: Option<Vec<Token>>) -> Result<(), Error> {
        if self.depth >= MAX_DEPTH {
            return Err(Error::Figure(format!("scope/\\foreach nesting went past {MAX_DEPTH} levels")));
        }
        self.depth += 1;
        if self.picture_depth == 0 {
            self.pictures += 1;
            // each picture is drawn on its own canvas, laid out with the others at the end
            let margin = self.canvas.margin;
            self.canvas = Canvas { margin, ..Canvas::default() };
            self.canvas_frozen = false;
            self.last_xy = Point::default();
        }
        self.picture_depth += 1;
        self.begin_group();
        self.begin_scope();
        self.run_tokens(tikra_tokens("\\tikz@picture@start"))?;
        if let Some(o) = o {
            // the picture's own options failing bring the figure down, as in TeX
            self.process_keys(o, Some("/tikz")).map_err(|e| Error::Figure(format!("picture options: {e}")))?;
        }
        self.st.apply_pending();
        self.open_transparency_group();
        // a picture is a scope too: `execute at begin scope` first, its end last
        let mut begin = std::mem::take(&mut self.st.hooks.at_begin_scope);
        begin.append(&mut self.st.hooks.at_begin_picture);
        let mut end = std::mem::take(&mut self.st.hooks.at_end_picture);
        end.append(&mut self.st.hooks.at_end_scope);
        self.scope_ends.push(end);
        self.run_tokens(begin)
    }

    /// `transparency group` given to a scope or a picture: what it draws is a group of its own.
    pub(super) fn open_transparency_group(&mut self) {
        if std::mem::take(&mut self.st.paint.transparency_group) {
            // the scope's opacity is the group's; inside, everything is opaque
            let opacity = self.st.paint.fill_opacity.min(self.st.paint.draw_opacity);
            self.open_in_scope(Item::Group { opacity });
            self.st.paint.draw_opacity = 1.0;
            self.st.paint.fill_opacity = 1.0;
            self.st.paint.text_opacity = 1.0;
        }
    }

    /// `\begin{scope}[options]` and `\scoped`: a group, a pgf scope, the options, `execute at begin scope`.
    pub fn open_tikz_scope(&mut self, o: Option<Vec<Token>>, fatal: bool) -> Result<(), Error> {
        self.begin_group();
        self.begin_scope();
        self.run_tokens(tikra_tokens("\\tikz@scope@options"))?;
        if let Some(o) = o {
            let r = self.process_keys(o, Some("/tikz"));
            if fatal {
                r.map_err(|e| Error::Figure(format!("scope options: {e}")))?;
            } else {
                r?;
            }
        }
        self.st.apply_pending();
        self.open_transparency_group();
        let begin = std::mem::take(&mut self.st.hooks.at_begin_scope);
        let end = std::mem::take(&mut self.st.hooks.at_end_scope);
        self.scope_ends.push(end);
        self.run_tokens(begin)
    }

    pub fn close_tikz_scope(&mut self) -> Result<(), Error> {
        let end = self.scope_ends.pop().unwrap_or_default();
        let r = self.run_tokens(end);
        self.end_scope();
        self.end_group();
        r
    }

    /// `\tikzfading[name=…, top color=transparent!0, bottom color=transparent!100]`: a fading from
    /// shading options, with `transparent!N` meaning N percent transparent.
    pub(super) fn tikzfading(&mut self) -> Result<(), Error> {
        let o = self.optional()?.unwrap_or_default();
        let mut name = String::new();
        let mut opts = Vec::new();
        for item in crate::keys::split_list(&o) {
            if item.key == "name" {
                name = self.tex.expand_to_string(item.value.unwrap_or_default())?.trim().to_string();
            } else {
                if !opts.is_empty() {
                    opts.push(Token::other(','));
                }
                opts.extend(tikra_tokens(&item.key));
                if let Some(v) = item.value {
                    opts.push(Token::other('='));
                    opts.push(Token::Char('{', Cat::Begin));
                    opts.extend(v);
                    opts.push(Token::Char('}', Cat::End));
                }
            }
        }
        self.begin_group();
        // transparent is black in a fading: N percent of it, the rest white
        let r = self.process_keys(opts, Some("/tikz"));
        // a unit square, so the shading's box is the identity once fitted
        let square = [crate::scene::Segment::MoveTo(Point::new(-25.0, -25.0)), crate::scene::Segment::LineTo(Point::new(25.0, 25.0))];
        let sh = self.shading_of(&square);
        self.end_group();
        r?;
        if !name.is_empty() {
            self.fadings.insert(name, sh);
        }
        Ok(())
    }

    /// `\pgfonlayer{name}`: what is drawn until `\endpgfonlayer` goes to that layer.
    pub(super) fn begin_on_layer(&mut self, name: String) {
        self.begin_group();
        let main = std::mem::take(&mut self.canvas.items);
        self.on_layer.push((name, main));
    }

    pub(super) fn end_on_layer(&mut self) {
        let Some((name, main)) = self.on_layer.pop() else {
            self.warn("\\endpgfonlayer without \\pgfonlayer");
            return;
        };
        let drawn = std::mem::replace(&mut self.canvas.items, main);
        self.layers.entry(name).or_default().extend(drawn);
        self.end_group();
    }

    /// `\tikra@frame`: the picture's box, and with the frame seps around it, as macros in pt
    /// (`\tikra@xa`… inner, `\tikra@oxa`… outer) for the backgrounds library.
    pub(super) fn frame_macros(&mut self) -> Result<(), Error> {
        // once per picture: the frame is around what the picture drew, not around another frame
        if matches!(self.tex.meaning_of("tikra@xa"), Meaning::Macro(_)) {
            return Ok(());
        }
        let (Some(lo), Some(hi)) = (self.canvas.min, self.canvas.max) else { return Ok(()) };
        let sep = |e: &mut Engine, name: &str| -> f64 {
            let v = match e.tex.meaning_of(name) {
                Meaning::Macro(m) => tex::tokens_to_string(&tex::substitute(&m.body, &[])),
                _ => "0pt".into(),
            };
            e.length(&v).unwrap_or(0.0)
        };
        let (fx, fy) = (sep(self, "tikz@framexsep"), sep(self, "tikz@frameysep"));
        let (ox, oy) = (sep(self, "tikz@outerframexsep"), sep(self, "tikz@outerframeysep"));
        let pt = |v: f64| format!("{}pt", v / crate::PT);
        for (name, v) in [
            ("tikra@xa", lo.x - fx),
            ("tikra@ya", lo.y - fy),
            ("tikra@xb", hi.x + fx),
            ("tikra@yb", hi.y + fy),
            ("tikra@oxa", lo.x - fx - ox),
            ("tikra@oya", lo.y - fy - oy),
            ("tikra@oxb", hi.x + fx + ox),
            ("tikra@oyb", hi.y + fy + oy),
        ] {
            self.tex.define_string(name, &pt(v), false);
        }
        Ok(())
    }

    pub fn end_picture(&mut self) {
        // execute at end picture, while the picture is still open
        let end = self.scope_ends.pop().unwrap_or_default();
        if let Err(e) = self.run_tokens(end) {
            self.warn(format!("execute at end picture: {e}"));
        }
        while !self.on_layer.is_empty() {
            self.end_on_layer();
        }
        self.end_scope();
        let outermost = self.picture_depth == 1;
        if outermost && !self.layers.is_empty() {
            // the layers, in the order of \pgfsetlayers (main where it is listed)
            let main = std::mem::take(&mut self.canvas.items);
            let mut layers = std::mem::take(&mut self.layers);
            let mut order = self.layer_order.clone();
            if !order.iter().any(|l| l == "main") {
                order.push("main".into());
            }
            let mut main = Some(main);
            for name in order {
                if name == "main" {
                    self.canvas.items.extend(main.take().unwrap_or_default());
                } else if let Some(v) = layers.remove(&name) {
                    self.canvas.items.extend(v);
                }
            }
            for (_, v) in layers {
                self.canvas.items.extend(v);
            }
        }
        let baseline = if outermost { self.picture_baseline() } else { 0.0 };
        if outermost {
            self.trim_picture();
        }
        self.end_group();
        self.picture_depth = self.picture_depth.saturating_sub(1);
        self.depth = self.depth.saturating_sub(1);
        if outermost {
            let margin = self.canvas.margin;
            let canvas = std::mem::replace(&mut self.canvas, Canvas { margin, ..Canvas::default() });
            // in a `\vbox` of another `\hsize`, the line is that wide
            let hsize = self.tex.dimens.get(&255).copied().unwrap_or(0);
            if hsize != 345 * 65536 && hsize > 0 {
                self.layout.push(Layout::Width(hsize as f64 / 65536.0 * crate::PT));
                if !self.tex.env_stack.iter().any(|n| n == "minipage") {
                    self.layout.push(Layout::Vbox);
                }
            }
            self.layout.push(Layout::Picture(self.boxes.len()));
            self.boxes.push(PictureBox { canvas, baseline });
        }
    }

    /// `trim left=x`, `trim right=x`: the picture's box starts or ends there (what is drawn beyond
    /// it sticks out, cut off with the page).
    pub(super) fn trim_picture(&mut self) {
        for (spec, left) in [(self.st.trim_left.clone(), true), (self.st.trim_right.clone(), false)] {
            let Some(spec) = spec else { continue };
            let s = spec.trim();
            let x = match s.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
                Some(inner) => self.coordinate(inner).map(|c| c.p.x),
                None if s.is_empty() => Ok(0.0),
                None => self.length(s),
            };
            let Ok(x) = x else { continue };
            match (left, self.canvas.min.as_mut(), self.canvas.max.as_mut()) {
                (true, Some(m), _) => m.x = x,
                (false, _, Some(m)) => m.x = x,
                _ => {}
            }
        }
    }

    /// Where the picture's baseline is (`baseline=`): by default its lower edge, as TikZ sets it.
    pub(super) fn picture_baseline(&mut self) -> f64 {
        let lower = self.canvas.min.map_or(0.0, |m| m.y);
        let Some(spec) = self.st.baseline.clone() else { return lower };
        let s = spec.trim();
        if s.is_empty() {
            return 0.0;
        }
        if let Some(inner) = s.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
            return self.coordinate(inner).map(|c| c.p.y).unwrap_or(lower);
        }
        // \pgfsetbaselinepointlater{point}: the point's height once the picture is drawn
        if let Some(point) = s.strip_prefix("tikra@point:") {
            let t = self.st.transform;
            return self.point_of(tikra_tokens(point)).map(|p| t.apply(p).y).unwrap_or(lower);
        }
        self.length(s).unwrap_or(lower)
    }

    /// The pictures from `start` on scaled around their origins, by the factors `factors` gives
    /// for the size of all of them together: its width, and its height and depth at the first
    /// one's baseline.
    pub(super) fn scale_boxes(&mut self, start: usize, factors: impl Fn(f64, f64, f64) -> (f64, f64)) {
        let corners: Vec<(Point, Point)> = self.boxes[start..].iter().filter_map(|b| b.canvas.min.zip(b.canvas.max)).collect();
        let Some(&(first_lo, first_hi)) = corners.first() else { return };
        let (lo, hi) = corners.iter().fold((first_lo, first_hi), |(a, b), (l, h)| (Point::new(a.x.min(l.x), a.y.min(l.y)), Point::new(b.x.max(h.x), b.y.max(h.y))));
        let baseline = self.boxes[start].baseline;
        let (fx, fy) = factors(hi.x - lo.x, hi.y - baseline, baseline - lo.y);
        let t = crate::scene::Transform::scaling(fx, fy);
        for b in &mut self.boxes[start..] {
            b.canvas.items.iter_mut().for_each(|i| i.transform(&t));
            let corners = [b.canvas.min, b.canvas.max];
            if let [Some(lo), Some(hi)] = corners {
                let (a, c) = (t.apply(lo), t.apply(hi));
                b.canvas.min = Some(Point::new(a.x.min(c.x), a.y.min(c.y)));
                b.canvas.max = Some(Point::new(a.x.max(c.x), a.y.max(c.y)));
            }
            b.baseline *= fy;
        }
    }

    /// `\tikz[options]{…}` or `\tikz[options] statement;`
    pub(super) fn tikz_command(&mut self) -> Result<(), Error> {
        let o = self.optional()?;
        // a command that reads its own input is run as it comes, so its catcodes see the lines
        if let Some(t) = self.tex.next_nonspace() {
            if self.reads_its_input(&t) {
                self.begin_picture(o)?;
                let r = self.execute(t);
                self.end_picture();
                return r;
            }
            self.tex.back(t);
        }
        let body = self.next_statement()?;
        self.begin_picture(o)?;
        self.run_tokens(body)?;
        self.end_picture();
        Ok(())
    }

    pub(super) fn begin_environment(&mut self) -> Result<(), Error> {
        let name = self.argument_string()?;
        self.tex.env_stack.push(name.clone());
        match name.as_str() {
            "tikzpicture" => {
                let o = self.optional()?;
                self.begin_picture(o)
            }
            "pgfpicture" => self.begin_picture(None),
            "pgfscope" => {
                self.begin_group();
                self.begin_scope();
                Ok(())
            }
            "scope" => {
                if self.depth >= MAX_DEPTH {
                    return Err(Error::Figure(format!("scope/\\foreach nesting went past {MAX_DEPTH} levels")));
                }
                self.depth += 1;
                let o = self.optional()?;
                self.open_tikz_scope(o, true)
            }
            n if self.piece_environment(n).is_some() => match self.piece_environment(n) {
                Some((begin, _)) => begin(self, n),
                None => Ok(()),
            },
            "pgfonlayer" => {
                let layer = self.argument_string()?;
                self.begin_on_layer(layer);
                Ok(())
            }
            "pgfviewboxscope" => self.begin_view_box_scope(),
            // what is drawn inside does not make the picture larger
            "pgfinterruptboundingbox" => {
                self.interrupted_boxes.push((self.canvas.min, self.canvas.max));
                self.begin_group();
                Ok(())
            }
            "tikzfadingfrompicture" => {
                // its \end is read with its body
                self.tex.env_stack.pop();
                self.fading_from_picture()
            }
            "lrbox" => {
                self.tex.env_stack.pop();
                self.lrbox()
            }
            "tabular" if self.picture_depth == 0 => {
                // `\begin{tabular}[pos]{rl}`: the pictures in the cells, set as TeX sets an \halign
                let _ = self.optional()?;
                let spec = self.argument_string()?;
                let cols: Vec<u32> = spec
                    .chars()
                    .filter_map(|c| match c {
                        'l' => Some(0),
                        'c' => Some(1),
                        'r' => Some(2),
                        _ => None,
                    })
                    .take(12)
                    .collect();
                let code = cols.iter().enumerate().fold(cols.len() as u32, |acc, (k, a)| acc | (a << (8 + 2 * k)));
                self.layout.push(Layout::TabStart(code));
                self.begin_group();
                Ok(())
            }
            "document" | "center" | "figure" | "flushleft" | "flushright" | "minipage" => {
                if name == "center" && self.picture_depth == 0 {
                    self.layout.extend([Layout::Par, Layout::Center(true)]);
                }
                if name == "minipage" {
                    let _ = self.optional()?;
                    let width = self.argument()?;
                    // the page is as wide as the minipage
                    if self.picture_depth == 0 {
                        let w = self.length_of(width)?;
                        self.layout.push(Layout::Width(w));
                    }
                } else if name == "figure" {
                    let _ = self.optional()?;
                }
                self.begin_group();
                if name == "document" {
                    self.document_depth = Some(self.tex.group_depth());
                }
                Ok(())
            }
            _ => {
                if let Some(code) = self.tex.environment_begin(&name)? {
                    self.begin_group();
                    self.tex.push_tokens(code);
                    return Ok(());
                }
                self.warn(format!("environment {name} not supported: content read as statements"));
                self.begin_group();
                Ok(())
            }
        }
    }

    pub(super) fn end_environment(&mut self) -> Result<(), Error> {
        let name = self.argument_string()?;
        let Some(pos) = self.tex.env_stack.iter().rposition(|n| *n == name) else {
            self.warn(format!("\\end{{{name}}} without a matching \\begin: ignored"));
            return Ok(());
        };
        // environments left open inside are closed with it
        while self.tex.env_stack.len() > pos + 1 {
            let inner = self.tex.env_stack.pop().unwrap_or_default();
            self.warn(format!("\\begin{{{inner}}} without \\end: closed at \\end{{{name}}}"));
            self.close_environment(&inner)?;
        }
        self.tex.env_stack.pop();
        self.close_environment(&name)
    }

    pub(super) fn close_environment(&mut self, name: &str) -> Result<(), Error> {
        match name {
            "tikzpicture" | "pgfpicture" => self.end_picture(),
            n if self.piece_environment(n).is_some() => {
                if let Some((_, end)) = self.piece_environment(n) {
                    end(self, n)?;
                }
            }
            "pgfscope" | "pgfviewboxscope" => {
                self.end_scope();
                self.end_group();
            }
            "pgfonlayer" => self.end_on_layer(),
            "pgfinterruptboundingbox" => {
                self.end_group();
                if let Some((min, max)) = self.interrupted_boxes.pop() {
                    (self.canvas.min, self.canvas.max) = (min, max);
                }
            }
            "scope" => {
                self.close_tikz_scope()?;
                self.depth = self.depth.saturating_sub(1);
            }
            _ => {
                if let Some((_, end)) = self.tex.environments.get(name).cloned() {
                    self.run_tokens(end)?;
                }
                self.end_group();
                if name == "center" && self.picture_depth == 0 {
                    self.layout.extend([Layout::Par, Layout::Center(false)]);
                }
                if name == "tabular" && self.picture_depth == 0 {
                    self.layout.push(Layout::TabEnd);
                }
                // what follows `\end{document}` is never typeset
                if name == "document" {
                    self.tex.clear_input();
                }
            }
        }
        Ok(())
    }
}
