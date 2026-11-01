use std::rc::Rc;

use crate::canvas::{transform_segments, Item};
use crate::keys;
use crate::label;
use crate::scene::{Label, Point, Transform};
use crate::tex::{self, Token};
use crate::{Error, PT};

use super::super::shapes::{self, BoxSize, Geometry, Layer, Usage};
use super::super::{tikra_tokens, Engine};
use super::labels::*;
use super::sets::*;
use super::text::*;
use super::*;

/// A node's shape, name and chain names, once its options have been read.
pub(super) struct ChosenNode {
    outer_name_path: Option<String>,
    shape: String,
    shape_name: String,
    user: Option<super::super::user_shapes::UserShape>,
    library: Option<crate::extension::ShapeFn>,
    name: Option<String>,
    chain_names: Vec<String>,
}

/// The text of a node's parts, and how it is set.
pub(super) struct NodeText {
    texts: Vec<(String, String)>,
    /// the first part before a shading or a picture clears it: the ignore warning names it
    text: String,
    text_width: Option<f64>,
    align: char,
    color: crate::color::Color,
    font: String,
}

/// The boxes and labels the shape is computed from.
pub(super) struct MeasuredParts {
    boxes: Vec<(String, BoxSize)>,
    labels: Vec<(String, Option<Label>)>,
    text_width: Option<f64>,
}

/// The separations and minima a shape is computed with.
#[derive(Clone, Copy)]
pub(super) struct SepInput {
    xsep: f64,
    ysep: f64,
    min_w: f64,
    min_h: f64,
    ox: f64,
    oy: f64,
    aspect: f64,
    ex: f64,
    line_width: f64,
}

/// What [`Engine::compute_shape`] reads.
pub(super) struct ShapeQuery<'a> {
    shape: &'a str,
    boxes: &'a [(String, BoxSize)],
    sep: SepInput,
}

/// A computed shape, still able to be computed again (a callout's pointer).
pub(super) struct BuiltShape<'a> {
    geometry: Geometry,
    boxes: &'a [(String, BoxSize)],
    sep: SepInput,
}

/// The shape and the text a node is placed with.
pub(super) struct NodeCore<'a> {
    chosen: &'a ChosenNode,
    text: &'a str,
}

/// A placed node, registered under its name.
pub(super) struct FinishNode<'a> {
    chosen: ChosenNode,
    info: NodeInfo,
    texts: &'a [(String, String)],
}

impl Engine {
    fn choose_node(&mut self, spec: &NodeSpec) -> Result<ChosenNode, Error> {
        if spec.coordinate {
            self.tex.define_string(&keys::macro_name("/tikz/shape"), "coordinate", false);
        }
        let outer_name_path = self.st.name_path.clone();
        self.process_keys(tikra_tokens("every node/.try"), Some("/tikz"))?;
        self.process_keys(spec.options.clone(), Some("/tikz"))?;
        // `right of=a` shifts by the node distance the node ends up with, given after it too: each
        // shift is redone with it, in its place among the transformations
        for (u, (v0, h0), after) in std::mem::take(&mut self.st.node.old_of).into_iter().rev() {
            let (v, h) = super::super::options::node_distance(self)?;
            let shift = Transform::translation(u.x * (h - h0), u.y * (v - v0));
            self.st.pending = after.compose(&shift).compose(&after.inverse()).compose(&self.st.pending);
        }
        // the node begin hook: a node on a chain takes its place and names there
        let (chain_names, joins) = self.chain_node_found()?;
        let mut after = std::mem::take(&mut self.st.after_node_path);
        after.extend(joins);
        self.after_node_path.extend(after);
        let shape_name = self.key_text("/tikz/shape").split_whitespace().collect::<Vec<_>>().join(" ");
        let user = self.declared.user_shapes.get(shape_name.trim()).cloned();
        let library = self.declared.extension_shapes.get(shape_name.trim()).copied();
        let shape = if let Some(u) = &user {
            u.base.clone()
        } else if library.is_some() {
            // computed as a rectangle, then replaced by what the library's shape gives
            "rectangle".to_string()
        } else if shapes::is_shape(&shape_name) {
            shape_name.clone()
        } else {
            self.warn(format!("shape '{shape_name}' not supported: drawn as a rectangle"));
            "rectangle".to_string()
        };
        self.process_keys(tikra_tokens(&format!("every {shape_name} node/.try")), Some("/tikz"))?;
        let listed = sets_macro(self);
        let name = spec.name.clone().or_else(|| self.st.node.name.clone()).or_else(|| self.node_sets.set_member_name(&listed));
        // what the system layer's animations give the node at the snapshot time
        if let Some(options) = name.as_ref().and_then(|n| self.anim.sys_animations.remove(n)) {
            self.process_keys(options, Some("/tikz"))?;
        }
        Ok(ChosenNode { outer_name_path, shape, shape_name, user, library, name, chain_names })
    }

    /// The text boxes of the parts: expanded, and a shading, a picture or a paragraph taken out.
    fn collect_node_text(&mut self, spec: &NodeSpec) -> Result<NodeText, Error> {
        let mut texts = Vec::new();
        // expanded first: a macro may hold the `\nodepart`s
        // with what `execute at begin node` and `execute at end node` put around it
        let body = |e: &Engine, name: &str| match e.tex.meaning_of(name) {
            tex::Meaning::Macro(m) => tex::substitute(&m.body, &[]),
            _ => Vec::new(),
        };
        let mut text_tokens = body(self, "tikz@atbegin@node");
        text_tokens.extend(spec.text.clone());
        text_tokens.extend(body(self, "tikz@atend@node"));
        let expanded = self.expand_text(text_tokens)?;
        for (part, tokens) in split_parts(&expanded) {
            texts.push((part, tex::tokens_to_text(&tokens).trim().to_string()));
        }
        let text = texts[0].1.clone();
        let text_width = {
            let t = self.key_text("/tikz/text width");
            if t.trim().is_empty() {
                None
            } else {
                Some(self.length(&t)?)
            }
        };
        let align = match self.key_text("/tikz/align").trim() {
            // (the flush ones in capitals: they break lines otherwise)
            "right" => 'r',
            "flush right" => 'R',
            "center" => 'c',
            "flush center" => 'C',
            "left" => 'l',
            "flush left" => 'L',
            "justify" => 'j',
            "none" if text_width.is_some() => 'j',
            _ if text_width.is_some() => 'l',
            _ => 'c',
        };
        let color = self.st.text_paint_color();
        let font = self.st.text.font.clone();
        // a text that is a shading (`\pgfuseshading{name}`): its box, drawn where the text goes
        self.node_run.text_shading = None;
        if let Some(name) = texts.first().and_then(|(_, t)| t.strip_prefix("\\pgfuseshading")).map(|n| n.trim().trim_start_matches('{').trim_end_matches('}').trim().to_string()) {
            if let Some((w, h)) = self.shading_half_size(&name) {
                self.node_run.text_shading = Some((name, w, h));
                texts[0].1.clear();
            }
        }
        // a text that is a picture: its box (its baseline at its bottom), drawn where the text goes
        self.node_run.text_picture = None;
        if texts.len() == 1 {
            if let Some((drawn, lo, hi)) = self.text_image(&texts[0].1)? {
                self.node_run.text_picture = Some((drawn, lo, hi, 0.0));
                texts[0].1.clear();
            } else if let Some(b) = self.text_box(&texts[0].1) {
                // a saved box (`\usebox{\name}`): where it was set, its baseline where it was
                let origin = Point::new(0.0, b.baseline);
                let (lo, hi) = (b.canvas.min.unwrap_or(origin), b.canvas.max.unwrap_or(origin));
                self.node_run.text_picture = Some((b.canvas.items, lo, hi, b.baseline - lo.y));
                texts[0].1.clear();
            }
        }
        let whole = texts.first().map(|(_, t)| t.trim().to_string()).unwrap_or_default();
        let tikz = whole.strip_prefix("\\tikz").is_some_and(|r| !r.starts_with(|c: char| c.is_ascii_alphabetic()));
        if texts.len() == 1 && (tikz || (whole.starts_with("\\begin{tikzpicture}") && whole.ends_with("\\end{tikzpicture}"))) {
            if let Some((drawn, lo, hi)) = self.inner_picture(spec.text.clone())? {
                self.node_run.text_picture = Some((drawn, lo, hi, 0.0));
                texts[0].1.clear();
            }
        }
        // a text that is all a `\parbox{4cm}{…}` or a `minipage`: a paragraph that wide, as a text
        // width sets it, justified unless it says otherwise (`\centering`)
        let (mut text_width, mut align) = (text_width, align);
        if let Some((width, body)) = texts.first().filter(|_| texts.len() == 1).and_then(|(_, t)| label::paragraph_box(t)) {
            if let Ok(w) = self.length(&width) {
                let first = body.trim_start();
                align = [("\\centering", 'C'), ("\\raggedright", 'L'), ("\\raggedleft", 'R')].iter().find(|(c, _)| first.starts_with(c)).map_or('j', |(_, a)| *a);
                text_width = Some(w);
                texts[0].1 = body;
            }
        }
        Ok(NodeText { texts, text, text_width, align, color, font })
    }

    /// Each part's label and box, with text height, text depth, a shading, a picture or `fit`.
    fn measure_parts(&mut self, shape: &str, node_text: &NodeText) -> Result<MeasuredParts, Error> {
        let mut labels = Vec::new();
        let mut boxes = Vec::new();
        let (text_width, align, color) = (node_text.text_width, node_text.align, node_text.color);
        for (k, (part, t)) in node_text.texts.iter().enumerate() {
            let lbl = if shape == "coordinate" || t.is_empty() {
                None
            } else {
                let mut warnings = Vec::new();
                // `every lower node part/.style={font=…}`: the part's own font and color
                self.begin_group();
                let r = self.process_keys(tikra_tokens(&format!("every {part} node part/.try")), Some("/tikz"));
                let (font, color) = if self.st.text.font != node_text.font || self.st.text_paint_color() != color {
                    (self.st.text.font.clone(), self.st.text_paint_color())
                } else {
                    (node_text.font.clone(), color)
                };
                self.end_group();
                r?;
                // a text that is all `\textcolor{color}{…}` is in that color
                let (t, color) = match whole_textcolor(t).and_then(|(c, inner)| self.color(&c).map(|c| (inner, c))) {
                    Some((inner, c)) => (inner, c),
                    None => (t.to_string(), color),
                };
                let t = self.inline_colors(&t);
                let l = label::compose_label(&t, &font, text_width, align, color, &mut warnings);
                for w in warnings {
                    self.warn(w);
                }
                Some(l)
            };
            let (mut wd, mut ht, mut dp) = match &lbl {
                Some(l) => (text_width.unwrap_or_else(|| l.width()), l.height(), l.depth()),
                // an empty text is as wide as the text width all the same
                None if shape != "coordinate" => (text_width.unwrap_or(0.0), 0.0, 0.0),
                None => (0.0, 0.0, 0.0),
            };
            // text height and text depth replace the box's
            for (key, target) in [("/tikz/text height", &mut ht), ("/tikz/text depth", &mut dp)] {
                let v = self.key_text(key);
                if !v.trim().is_empty() {
                    *target = self.length(&v)?;
                }
            }
            if k == 0 {
                if let Some((_, w, h)) = &self.node_run.text_shading {
                    (wd, ht, dp) = (2.0 * w, 2.0 * h, 0.0);
                }
                if let Some((_, lo, hi, depth)) = &self.node_run.text_picture {
                    (wd, ht, dp) = (hi.x - lo.x, hi.y - lo.y - depth, *depth);
                }
                if let Some(b) = self.node_run.forced_box.take() {
                    (wd, ht, dp) = b;
                }
                if let Some((w, h)) = self.st.node.fit {
                    // fit: the box is the fitted area, centered on the baseline
                    (wd, ht, dp) = (w, h / 2.0, h / 2.0);
                }
            }
            boxes.push((part.clone(), BoxSize { wd, ht, dp }));
            labels.push((part.clone(), lbl));
        }
        Ok(MeasuredParts { boxes, labels, text_width })
    }

    /// `circle through`: as wide as twice the distance from the node's position.
    fn widen_through(&mut self, spec: &NodeSpec) {
        if let Some(p) = self.st.node.through {
            let at = self.st.node.at.or(spec.at).unwrap_or(spec.point);
            let size = format!("{}pt", 2.0 * p.minus(at).norm() / PT);
            for key in ["/pgf/minimum width", "/pgf/minimum height"] {
                self.tex.define_string(&keys::macro_name(key), &size, false);
            }
        }
    }

    fn shape_seps(&mut self) -> SepInput {
        SepInput {
            xsep: self.key_length("/pgf/inner xsep", 0.3333 * 0.3515),
            ysep: self.key_length("/pgf/inner ysep", 0.3333 * 0.3515),
            min_w: self.key_length("/pgf/minimum width", PT),
            min_h: self.key_length("/pgf/minimum height", PT),
            ox: self.key_length("/pgf/outer xsep", 0.5 * self.st.pen.line_width),
            oy: self.key_length("/pgf/outer ysep", 0.5 * self.st.pen.line_width),
            aspect: self.key_text("/pgf/aspect").trim().parse::<f64>().ok().filter(|a| *a > 0.0).unwrap_or(1.0),
            ex: 0.43 * self.st.text.font_size * PT,
            line_width: self.st.pen.line_width,
        }
    }

    /// The geometry of `shape` from these boxes. `pointer` is a callout's, in node coordinates.
    fn compute_shape(&mut self, q: &ShapeQuery<'_>, pointer: Option<Point>) -> Geometry {
        let mut keys = EngineKeys(self);
        let mut input = shapes::Input {
            parts: q.boxes,
            inner_xsep: q.sep.xsep,
            inner_ysep: q.sep.ysep,
            minimum_width: q.sep.min_w,
            minimum_height: q.sep.min_h,
            outer_xsep: q.sep.ox,
            outer_ysep: q.sep.oy,
            aspect: q.sep.aspect,
            ex: q.sep.ex,
            line_width: q.sep.line_width,
            pointer,
            keys: &mut keys,
        };
        shapes::compute(q.shape, &mut input).expect("a known shape")
    }

    /// The shape's geometry: the built-in one, a user's, or a library's.
    fn node_geometry(&mut self, chosen: &ChosenNode, boxes: &[(String, BoxSize)], sep: SepInput) -> Result<Geometry, Error> {
        let q = ShapeQuery { shape: &chosen.shape, boxes, sep };
        let mut geometry = self.compute_shape(&q, None);
        // a logic gate's anchors with the other `logic gate anchors use bounding box`, which
        // pgf reads when an anchor is asked for: under `@gatebox=true name` or `@gatebox=false name`
        if chosen.shape.contains(" gate ") {
            let flag = Token::cs("iftikra@gatebox");
            let saved = self.tex.meaning(&flag);
            let on = saved == self.tex.meaning_of("iftrue");
            let other = self.tex.meaning_of(if on { "iffalse" } else { "iftrue" });
            self.tex.set_meaning(&flag, other, false);
            let g = self.compute_shape(&q, None);
            self.tex.set_meaning(&flag, saved, false);
            geometry.anchors.extend(g.anchors.into_iter().map(|(n, p)| (format!("@gatebox={} {n}", !on), p)));
        }
        if let Some(u) = &chosen.user {
            let (wd, ht, dp) = boxes.first().map(|(_, b)| (b.wd, b.ht, b.dp)).unwrap_or_default();
            let input = super::super::user_shapes::ShapeInput {
                text: (wd, ht, dp),
                minimum: Point::new(sep.min_w, sep.min_h),
                inner: Point::new(sep.xsep, sep.ysep),
                outer: Point::new(sep.ox, sep.oy),
            };
            geometry = self.user_shape_geometry(chosen.shape_name.trim(), u, geometry, input)?;
        }
        if let Some(f) = chosen.library {
            let text = boxes.first().map(|(_, b)| *b).unwrap_or_default();
            // the node's own transformation, as it is placed below (without where it goes)
            let picture = if self.st.node.transform_shape { Transform { e: 0.0, f: 0.0, ..self.st.transform } } else { Transform::default() };
            let input = crate::extension::ShapeInput {
                name: chosen.shape_name.trim().to_string(),
                width: text.wd,
                height: text.ht,
                depth: text.dp,
                inner_sep: Point::new(sep.xsep, sep.ysep),
                line_width: sep.line_width,
                node: chosen.name.as_ref().map(|n| format!("{}{n}", self.st.name_prefix)),
                fill: self.st.mode.fill.then(|| self.st.fill_paint_color().xcolor()),
                draw: self.st.mode.draw.then(|| self.st.stroke_color().xcolor()),
                transform: picture.compose(&self.st.pending),
            };
            geometry = library_geometry(f(&mut crate::extension::Context { e: self }, &input)?);
        }
        Ok(geometry)
    }

    /// Where the node goes, and its border named, when the position is finite.
    fn place_node(&mut self, spec: &NodeSpec, core: NodeCore<'_>, built: BuiltShape<'_>) -> Result<Option<NodeInfo>, Error> {
        let mut info = NodeInfo { shape: core.chosen.shape.clone(), geometry: Rc::new(built.geometry), transform: Transform::default() };
        // where: `at`, positioning, the path's point or a point along its last segment
        let pos = self.st.node.pos.or(spec.default_pos);
        let (target, direction) = match (self.st.node.at.or(spec.at), pos) {
            (Some(p), _) => (p, Point::new(1.0, 0.0)),
            (None, Some(t)) => spec.place.at(t),
            (None, None) => (spec.point, Point::new(1.0, 0.0)),
        };
        let mut anchor = self.key_text("/tikz/anchor");
        if let (Some(side), true, Some(_)) = (self.st.auto, anchor.trim() == "center", pos) {
            // auto: to the left of the segment (right with swap), by its direction
            anchor = auto_anchor(direction, (side == 'r') != self.st.swap).to_string();
        }
        let anchor = anchor.trim();
        let near = self.piece_anchor(anchor, &|n| info.anchor_local(n));
        let a_local = near.or_else(|| info.anchor_local(anchor)).unwrap_or_else(|| {
            self.warn(format!("unknown anchor '{anchor}': center used"));
            info.geometry.center
        });
        // the picture's transformation keeps only its translation, unless transform shape
        let mut linear = Transform::default();
        if self.st.node.transform_shape {
            linear = Transform { e: 0.0, f: 0.0, ..self.st.transform };
        }
        // under a nonlinear transformation the node goes where its point goes; with `transform
        // shape nonlinear` it is also turned and scaled as the transformation bends the picture there
        let mut target = target;
        if !self.st.nonlinear.is_empty() {
            let moved = self.nonlinear_point(target)?;
            if self.st.node.transform_shape_nonlinear {
                let h = 0.01;
                let dx = self.nonlinear_point(target.plus(Point::new(h, 0.0)))?.minus(moved).scale(1.0 / h);
                let dy = self.nonlinear_point(target.plus(Point::new(0.0, h)))?.minus(moved).scale(1.0 / h);
                linear = Transform { a: dx.x, b: dx.y, c: dy.x, d: dy.y, e: 0.0, f: 0.0 }.compose(&linear);
            }
            target = moved;
        }
        if self.st.node.sloped {
            let angle = direction.y.atan2(direction.x).to_degrees();
            let angle = if !self.st.allow_upside_down && !(-90.0 - 1e-9..=90.0 + 1e-9).contains(&angle) { angle + 180.0 } else { angle };
            linear = linear.compose(&Transform::rotation(angle));
        }
        let t = Transform::translation(target.x, target.y).compose(&linear).compose(&self.st.pending).compose(&Transform::translation(-a_local.x, -a_local.y));
        info.transform = t;
        // a callout's absolute pointer is in the picture: once the node is placed, the shape is
        // computed again with it in node coordinates (its other anchors do not depend on it)
        if core.chosen.shape.ends_with("callout") {
            if let Some(p) = shapes::Keys::point(&mut EngineKeys(self), "/pgf/callout pointer@absolute") {
                let local = t.inverse().apply(p);
                let again = ShapeQuery { shape: &core.chosen.shape, boxes: built.boxes, sep: built.sep };
                info.geometry = Rc::new(self.compute_shape(&again, Some(local)));
            }
        }
        let limit = super::super::path::MAX_COORDINATE;
        let fine = |p: Point| p.x.is_finite() && p.y.is_finite() && p.x.abs() <= limit && p.y.abs() <= limit;
        if !fine(info.center()) || !info.geometry.anchors.iter().all(|(_, p)| fine(t.apply(*p))) {
            self.warn(format!("node '{}': position or size is not finite or beyond ±{limit} cm: ignored", core.text));
            return Ok(None);
        }
        // `name path` among the node's own options names its border
        if let Some(n) = self.st.name_path.clone().filter(|_| self.st.name_path != core.chosen.outer_name_path) {
            let g = &info.geometry;
            let border =
                g.paths.iter().filter(|p| p.layer == Layer::Background && matches!(p.usage, Usage::Node)).flat_map(|p| transform_segments(&p.segments, &info.transform)).collect();
            self.pgf.named_paths.insert(n, border);
        }
        Ok(Some(info))
    }

    /// The node known by its name, and what its labels need.
    fn finish_node(&mut self, reg: FinishNode<'_>, items: Vec<Item>) -> Result<(Vec<Item>, Option<NodeLabels>), Error> {
        let FinishNode { chosen, info, texts } = reg;
        let ChosenNode { shape, name, chain_names, .. } = chosen;
        let key = if let Some(n) = &name {
            // inside a pic, names get its prefix
            let n = format!("{}{n}", self.st.name_prefix);
            self.tex.define_string("tikzlastnode", &n, true);
            let listed = sets_macro(self);
            self.node_sets.join_sets(&listed, &n);
            self.nodes.insert(n.clone(), info.clone());
            n
        } else {
            // an unnamed node is `\tikzlastnode` by an internal name, as TikZ's `tikz@f@…`
            let n = format!("tikz@f@{}", self.nodes.len());
            self.tex.define_string("tikzlastnode", &n, true);
            self.nodes.insert(n.clone(), info.clone());
            n
        };
        for n in chain_names.into_iter().chain(std::mem::take(&mut self.st.aliases)) {
            self.nodes.insert(n, info.clone());
        }
        // the last node, for trees (labels below do not count)
        let registered = name.as_ref().map(|n| format!("{}{n}", self.st.name_prefix));
        if let Some(n) = &registered {
            self.canvas.named.push((n.clone(), info.center()));
        }
        if self.facts.mute == 0 && shape != "coordinate" {
            // with the texts a library's shape draws (a chip's pin names)
            let own = info.geometry.texts.iter().filter(|t| t.anchor.is_some()).map(|t| t.tex.as_str());
            let all: Vec<&str> = texts.iter().map(|(_, t)| t.as_str()).chain(own).filter(|t| !t.trim().is_empty()).collect();
            self.facts.nodes.push(crate::describe::NodeFact { key, named: registered.is_some(), text: all.join(" "), at: info.center(), on_link: false });
        }
        if let Some(slot @ None) = self.tree.tree_child_names.last_mut() {
            *slot = Some(registered.clone().unwrap_or_default());
        }
        let last = (Some(info.clone()), registered);
        (self.last_node, self.last_node_name) = last.clone();
        let labels = std::mem::take(&mut self.st.node.labels);
        let kept = LABEL_KEYS.iter().map(|k| (*k, self.tex.meaning_of(&keys::macro_name(k)))).collect();
        Ok((items, Some((info, labels, last, kept))))
    }

    pub(super) fn node_in_group(&mut self, spec: NodeSpec, scope: &super::super::GState) -> Result<(Vec<Item>, Option<NodeLabels>), Error> {
        let chosen = self.choose_node(&spec)?;
        let node_text = self.collect_node_text(&spec)?;
        let measured = self.measure_parts(&chosen.shape, &node_text)?;
        self.widen_through(&spec);
        let sep = self.shape_seps();
        let geometry = self.node_geometry(&chosen, &measured.boxes, sep)?;
        let built = BuiltShape { geometry, boxes: &measured.boxes, sep };
        let Some(info) = self.place_node(&spec, NodeCore { chosen: &chosen, text: &node_text.text }, built)? else {
            return Ok((Vec::new(), None));
        };
        let size = (self.canvas.min, self.canvas.max);
        let items = self.draw_shape(&info, measured.labels, measured.text_width, scope)?;
        if self.st.position_later {
            // kept for `\pgfpositionnodenow`: not in the picture, nor its size, nor known by name yet
            (self.canvas.min, self.canvas.max) = size;
            self.node_run.later_node = Some((items, info, chosen.name.map(|n| format!("{}{n}", self.st.name_prefix))));
            return Ok((Vec::new(), None));
        }
        self.finish_node(FinishNode { chosen, info, texts: &node_text.texts }, items)
    }
}
