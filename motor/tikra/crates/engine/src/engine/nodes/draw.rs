use crate::canvas::{transform_segments, Fill, Item, Paint, Stroke};
use crate::extension::{LineCap, ShapePaint};
use crate::label;
use crate::scene::{Label, Point, Segment, Transform};
use crate::Error;

use super::super::shapes::{Geometry, Layer, Usage};
use super::super::Engine;
use super::*;

/// One layer pass of a node's paths.
struct LayerDraw<'a> {
    t: Transform,
    g: &'a Geometry,
    node_fill: Option<Fill>,
    node_stroke: Option<Stroke>,
    scope: &'a super::super::GState,
}

/// The part labels drawn inside a shape.
struct PartLabels<'a> {
    t: Transform,
    g: &'a Geometry,
    labels: Vec<(String, Option<Label>)>,
    text_width: Option<f64>,
}

impl Engine {
    /// The paths of one run of layers, in pgf's order.
    fn emit_layers(&mut self, draw: &LayerDraw<'_>, layers: &[Layer], items: &mut Vec<Item>) -> Result<(), Error> {
        for p in draw.g.paths.iter().filter(|p| layers.contains(&p.layer)) {
            let mut segs = p.segments.clone();
            if p.layer == Layer::Background && draw.g.roundable && (self.st.pen.rounded != 0.0 || self.st.pen.rounded_out != 0.0) {
                segs = super::super::output::round_corners(&segs, self.st.pen.rounded, self.st.pen.rounded_out);
            }
            let segs = transform_segments(&segs, &draw.t);
            if matches!(p.usage, Usage::Node)
                && p.layer == Layer::Background
                && (self.st.decorate
                    || self.st.mode.picture.is_some()
                    || !self.st.preactions.is_empty()
                    || !self.st.postactions.is_empty()
                    || (self.st.pen.double.is_some() && self.st.mode.draw))
            {
                // the background as a path of its own, decorated, with its actions, or double
                let saved = std::mem::take(&mut self.canvas.items);
                let r = self.finish_path(segs, draw.scope, "node");
                items.extend(std::mem::replace(&mut self.canvas.items, saved));
                r?;
                continue;
            }
            let (fill, stroke) = match &p.usage {
                Usage::Node => (draw.node_fill.clone(), draw.node_stroke.clone()),
                Usage::Fill(c) => {
                    let color = self.color(c).unwrap_or(crate::color::Color::WHITE);
                    (Some(Fill { paint: Paint::Solid(color), opacity: self.st.paint.fill_opacity, even_odd: false }), None)
                }
                Usage::Stroke => (None, Some(self.stroke())),
                Usage::TipStroke(cap, join) => (None, Some(crate::canvas::Stroke { cap: *cap, join: *join, ..self.stroke() })),
                Usage::LineFill => (Some(Fill { paint: Paint::Solid(self.st.stroke_color()), opacity: self.st.paint.fill_opacity, even_odd: false }), None),
                Usage::Library { paint, color, dash, cap, opacity: own_opacity } => {
                    let own = color.as_deref().and_then(|c| {
                        let found = self.color(c);
                        if found.is_none() {
                            self.warn(format!("shape color '{c}' not known: the node's used"));
                        }
                        found
                    });
                    let opacity = own_opacity.unwrap_or(self.st.paint.fill_opacity);
                    let solid = |c: crate::color::Color| Fill { paint: Paint::Solid(c), opacity, even_odd: false };
                    match paint {
                        ShapePaint::Stroke(w) => {
                            let base = self.stroke();
                            let stroke = crate::canvas::Stroke {
                                width: *w,
                                color: own.unwrap_or(base.color),
                                dash: dash.as_ref().map(|d| (d.clone(), 0.0)).or(base.dash.clone()),
                                cap: cap.map_or(base.cap, |c| match c {
                                    LineCap::Butt => crate::canvas::Cap::Butt,
                                    LineCap::Round => crate::canvas::Cap::Round,
                                    LineCap::Rect => crate::canvas::Cap::Square,
                                }),
                                ..base
                            };
                            (None, Some(stroke))
                        }
                        ShapePaint::Fill => (draw.node_fill.clone().map(|f| own.map_or(f, solid)), None),
                        ShapePaint::Background => (Some(own.map(solid).or_else(|| draw.node_fill.clone()).unwrap_or_else(|| solid(crate::color::Color::WHITE))), None),
                        ShapePaint::Solid => (Some(solid(own.unwrap_or_else(|| self.st.stroke_color()))), None),
                    }
                }
            };
            if self.counts_for_size() {
                self.canvas.include_path(&segs);
                if let Some(s) = &stroke {
                    // \pgfusepath adds half the line width around a stroked path
                    self.canvas.include_path_margin(&segs, s.width / 2.0);
                }
            }
            if fill.is_some() || stroke.is_some() {
                items.push(Item::Path { segments: segs, fill, stroke });
            }
        }
        Ok(())
    }

    /// A picture or a saved box, where the text part goes.
    fn place_text_picture(&mut self, t: Transform, g: &Geometry, items: &mut Vec<Item>) {
        if let Some((drawn, lo, _, depth)) = self.node_run.text_picture.take() {
            if let Some(origin) = g.parts.iter().find(|(n, _)| n == "text").map(|(_, p)| *p) {
                let place = t.compose(&Transform::translation(origin.x - lo.x, origin.y - lo.y - depth));
                items.push(Item::Transform { transform: place });
                items.extend(drawn);
                items.push(Item::End);
            }
        }
    }

    /// A shading used as the text, where the text part goes.
    fn place_text_shading(&mut self, t: Transform, g: &Geometry, items: &mut Vec<Item>) -> Result<(), Error> {
        if let Some((name, w, h)) = self.node_run.text_shading.take() {
            if let Some(origin) = g.parts.iter().find(|(n, _)| n == "text").map(|(_, p)| *p) {
                let saved = std::mem::take(&mut self.canvas.items);
                let r = self.draw_shading(&name, &t.compose(&Transform::translation(origin.x + w, origin.y + h)));
                items.extend(std::mem::replace(&mut self.canvas.items, saved));
                r?;
            }
        }
        Ok(())
    }

    /// The labels of the parts, each at its part's origin.
    fn place_part_labels(&mut self, parts: PartLabels<'_>, items: &mut Vec<Item>) {
        let PartLabels { t, g, labels, text_width } = parts;
        for (part, lbl) in labels {
            let (Some(l), Some(origin)) = (lbl, g.parts.iter().find(|(n, _)| *n == part).map(|(_, p)| *p)) else { continue };
            let offset = match (text_width, l.align) {
                (Some(tw), 'c') => (tw - l.width()) / 2.0,
                (Some(tw), 'r') => tw - l.width(),
                _ => 0.0,
            };
            let place = t.compose(&Transform::translation(origin.x + offset, origin.y));
            items.push(Item::Text { label: l, place, opacity: self.st.paint.text_opacity });
        }
    }

    /// The texts a library's shape draws.
    fn place_library_texts(&mut self, t: Transform, g: &Geometry, items: &mut Vec<Item>) {
        for st in &g.texts {
            let (text, own) = self.shape_text(&st.tex);
            let color = own.or(st.color.clone()).and_then(|c| self.color(&c)).unwrap_or_else(|| self.st.text_paint_color());
            if text.trim().is_empty() {
                continue;
            }
            let mut warnings = Vec::new();
            let text = self.inline_colors(text.trim());
            let font = st.font.clone().unwrap_or_else(|| self.st.text.font.clone());
            let l = label::compose_label(&text, &font, None, 'c', color, &mut warnings);
            for w in warnings {
                self.warn(w);
            }
            let origin = match st.anchor {
                Some(a) => st.at.minus(text_anchor(a, &l, &font)),
                None => st.at,
            };
            // turned around its point, less the node's own turn when it stays upright
            let mut turn = st.turn;
            if st.upright && t.a * t.d - t.b * t.c > 0.0 {
                turn -= t.b.atan2(t.a).to_degrees();
            }
            let place = if turn.abs() < 1e-9 {
                t.compose(&Transform::translation(origin.x, origin.y))
            } else {
                let d = origin.minus(st.at);
                t.compose(&Transform::translation(st.at.x, st.at.y)).compose(&Transform::rotation(turn)).compose(&Transform::translation(d.x, d.y))
            };
            // a library's texts count in the picture's size, as TeX's boxes do
            if st.anchor.is_some() && self.counts_for_size() {
                for (x, y) in [(0.0, -l.depth()), (l.width(), -l.depth()), (0.0, l.height()), (l.width(), l.height())] {
                    self.canvas.include(place.apply(Point::new(x, y)));
                }
            }
            items.push(Item::Text { label: l, place, opacity: self.st.paint.text_opacity });
        }
    }

    /// What a node draws, in pgf's order: behind the background, the background (as the node's
    /// options say), before it, the texts of the parts, behind the foreground, the foreground and
    /// before it. Every path counts in the bounding box, as pgf protocols paths when they are made.
    pub(super) fn draw_shape(&mut self, info: &NodeInfo, labels: Vec<(String, Option<Label>)>, text_width: Option<f64>, scope: &super::super::GState) -> Result<Vec<Item>, Error> {
        let t = info.transform;
        let g = info.geometry.clone();
        let mut items = Vec::new();
        // a shaded node: its shading fitted to the background path, as \pgfshadepath does
        let background: Vec<Segment> =
            g.paths.iter().filter(|p| p.layer == Layer::Background && matches!(p.usage, Usage::Node)).flat_map(|p| transform_segments(&p.segments, &t)).collect();
        let node_fill = if self.st.mode.shade && !background.is_empty() {
            Some(Fill { paint: Paint::Shading(self.shading_of(&background)), opacity: self.st.paint.fill_opacity, even_odd: self.st.paint.even_odd })
        } else if self.st.mode.fill {
            Some(Fill { paint: Paint::Solid(self.st.fill_paint_color()), opacity: self.st.paint.fill_opacity, even_odd: self.st.paint.even_odd })
        } else if self.st.mode.pattern {
            Some(Fill {
                paint: {
                    let (name, color) = (self.st.paint.pattern_name.clone().unwrap_or_default(), self.st.paint.pattern_color.unwrap_or(crate::color::Color::BLACK));
                    self.pattern_paint(&name, color).unwrap_or(Paint::Pattern { name, color })
                },
                opacity: self.st.paint.fill_opacity,
                even_odd: false,
            })
        } else {
            None
        };
        let node_stroke = self.st.mode.draw.then(|| self.stroke());
        let draw = LayerDraw { t, g: &g, node_fill, node_stroke, scope };
        self.emit_layers(&draw, &[Layer::BehindBackground, Layer::Background, Layer::BeforeBackground], &mut items)?;
        if self.counts_for_size() {
            for p in &g.extent {
                self.canvas.include(t.apply(*p));
            }
        }
        self.place_text_picture(t, &g, &mut items);
        self.place_text_shading(t, &g, &mut items)?;
        self.place_part_labels(PartLabels { t, g: &g, labels, text_width }, &mut items);
        self.place_library_texts(t, &g, &mut items);
        self.emit_layers(&draw, &[Layer::BehindForeground, Layer::Foreground, Layer::BeforeForeground], &mut items)?;
        Ok(items)
    }
}
