use std::rc::Rc;

use tikra_engine::scene::{Point, Transform};
use tikra_engine::tex::{self};
use tikra_engine::Error;

use crate::keys;
use crate::plot::Plot;
use crate::{Axis, Engine, Entry};

use super::frame::*;
use super::prepare::*;
use super::util::*;

use super::anchor::{named_anchor, register_axis_node, AnchorPlace};
use super::plots::draw_all;

/// The axis' `at` (in the picture), and whether its own `xshift` moves it: not when a colorbar is
/// placed at plain description coordinates of its parent (as pgfplots' `colorbar top`
/// customizations show).
pub(in crate::draw) fn axis_at(e: &mut Engine) -> Result<(Point, bool), Error> {
    let mut shifts = true;
    let at = match keys::value_tokens(e, "/pgfplots/at") {
        Some(v) => {
            let s = e.tex.expand_to_string(v)?;
            let inner = s.trim().trim_start_matches('{').trim_end_matches('}').trim().trim_start_matches('(').trim_end_matches(')').to_string();
            // a colorbar's `at` is in its parent's description coordinates
            let parent = crate::colorbar::parent(e);
            if parent.is_some() && inner.chars().all(|c| c.is_ascii_digit() || ".,-+ ".contains(c)) {
                shifts = false;
            }
            if let Some(p) = &parent {
                p.cs.set(Cs::Description);
                crate::fig(e).frame = Some(p.clone());
            }
            let r = e.coordinate(&inner);
            if parent.is_some() {
                crate::fig(e).frame = None;
            }
            match r {
                Ok(c) => c.p,
                Err(err) => {
                    e.warn(format!("axis at={s}: {err}"));
                    e.st.transform.apply(Point::default())
                }
            }
        }
        None => e.st.transform.apply(Point::default()),
    };
    Ok((at, shifts))
}

/// Draws the axis collected by the survey; returns its colorbar, drawn once its group is closed.
/// A picture scaled or rotated (`\begin{tikzpicture}[scale=0.75]`) has the whole axis drawn so, its
/// text and lines too, as pgfplots does: it is drawn unscaled from the picture's origin, then
/// transformed around it.
pub fn draw_axis(e: &mut Engine, axis: &Axis) -> Result<Option<crate::colorbar::Colorbar>, Error> {
    let saved = e.st.transform;
    let linear = Transform { e: 0.0, f: 0.0, ..saved };
    let identity = Transform::default();
    if [linear.a - identity.a, linear.b, linear.c, linear.d - identity.d].iter().all(|v| v.abs() < 1e-9) {
        return draw_axis_unscaled(e, axis);
    }
    let origin = saved.apply(Point::default());
    let m = Transform::translation(origin.x, origin.y).compose(&linear).compose(&Transform::translation(-origin.x, -origin.y));
    e.st.transform = Transform::translation(origin.x, origin.y);
    let (items_before, box_before) = (e.canvas.items.len(), (e.canvas.min.take(), e.canvas.max.take()));
    let nodes_before: std::collections::HashSet<String> = e.nodes.keys().cloned().collect();
    let r = draw_axis_unscaled(e, axis);
    e.st.transform = saved;
    e.canvas.items[items_before..].iter_mut().for_each(|it| it.transform(&m));
    for (name, n) in e.nodes.iter_mut() {
        if !nodes_before.contains(name) {
            n.transform = m.compose(&n.transform);
        }
    }
    let axis_box = e.canvas.min.zip(e.canvas.max);
    (e.canvas.min, e.canvas.max) = box_before;
    if let Some((lo, hi)) = axis_box {
        for corner in [lo, Point::new(hi.x, lo.y), hi, Point::new(lo.x, hi.y)] {
            e.canvas.include(m.apply(corner));
        }
    }
    r
}

pub(in crate::draw) fn draw_axis_unscaled(e: &mut Engine, axis: &Axis) -> Result<Option<crate::colorbar::Colorbar>, Error> {
    let mut setup = setup(e, axis)?;
    let anchor = keys::value(e, "/pgfplots/anchor");
    // an anchor of the box alone (`origin`, `south west`…) places the axis before it is drawn, as
    // pgfplots does: what it draws at named coordinates of the picture stays where they are
    let early = {
        let (lo, hi) = setup.frame.bbox();
        let a = anchor.trim();
        let box_only = !a.starts_with("outer")
            && !["above", "below", "left of", "right of"].iter().any(|w| a.starts_with(w))
            && named_anchor(&anchor, &AnchorPlace { lo, hi, outer_lo: lo, outer_hi: hi, frame: &setup.frame }).is_some();
        if box_only && crate::colorbar::parent(e).is_none() {
            let anchor_point = named_anchor(&anchor, &AnchorPlace { lo, hi, outer_lo: lo, outer_hi: hi, frame: &setup.frame }).unwrap_or(lo);
            let (at, shifts) = axis_at(e)?;
            let p = e.st.pending;
            Some(at.minus(anchor_point).plus(if shifts { Point::new(p.e, p.f) } else { Point::default() }))
        } else {
            None
        }
    };
    if let Some(d) = early {
        setup.frame = Rc::new(Frame { origin: setup.frame.origin.plus(d), ..clone_frame(&setup.frame) });
    }
    let frame = setup.frame.clone();
    crate::fig(e).frame = Some(frame.clone());
    // the canvas items and bounding box of the axis alone, then moved by `at` and `anchor`
    let nodes_before: std::collections::HashSet<String> = e.nodes.keys().cloned().collect();
    if e.facts.mute == 0 && crate::colorbar::parent(e).is_none() {
        note_axis(e, axis, &frame);
    }
    // its ticks, labels and legend are not the figure's text
    e.facts.mute += 1;
    let r = capture(e, |e| draw_all(e, axis, &setup));
    e.facts.mute -= 1;
    let ((), items, bbox) = match r {
        Ok(v) => v,
        Err(err) => {
            crate::fig(e).frame = None;
            return Err(err);
        }
    };
    // where the axis goes: `at` (in the picture), by `anchor`
    let (lo, hi) = frame.bbox();
    let (blo, bhi) = bbox.unwrap_or((lo, hi));
    // `anchor=mynode.center`: a node drawn in the axis (`title style={name=…}`)
    let node_anchor = || {
        let (n, a) = anchor.trim().rsplit_once('.')?;
        e.nodes.get(n.trim()).filter(|_| !nodes_before.contains(n.trim())).and_then(|info| info.anchor(a.trim()))
    };
    let anchor_point = named_anchor(&anchor, &AnchorPlace { lo, hi, outer_lo: blo, outer_hi: bhi, frame: &frame }).or_else(node_anchor).unwrap_or(lo);
    crate::fig(e).frame = None;
    let d = match early {
        Some(_) => Point::default(),
        None => {
            let (at, shifts) = axis_at(e)?;
            // `xshift`, `yshift` and `shift` in the axis options move it (`colorbar shift`)
            let p = e.st.pending;
            at.minus(anchor_point).plus(if shifts { Point::new(p.e, p.f) } else { Point::default() })
        }
    };
    for mut it in items {
        it.translate(d);
        e.canvas.items.push(it);
    }
    if !e.canvas_frozen {
        e.canvas.include(blo.plus(d));
        e.canvas.include(bhi.plus(d));
    }
    // nodes made inside the axis move with it
    for (name, n) in e.nodes.iter_mut() {
        if !nodes_before.contains(name) {
            n.transform = Transform::translation(d.x, d.y).compose(&n.transform);
        }
    }
    // the axis as a node: `current axis` and its name, with pgfplots' extra anchors
    let name = keys::value(e, "/pgfplots/name");
    let (lo, hi, blo, bhi) = (lo.plus(d), hi.plus(d), blo.plus(d), bhi.plus(d));
    let shifted = Frame { origin: frame.origin.plus(d), ..clone_frame(&frame) };
    let colorbar = crate::colorbar::snapshot(e, frame.meta, (hi.x - lo.x, hi.y - lo.y), clone_frame(&shifted)).map(|mut cb| {
        // a filled contour's colorbar shows its bands
        cb.bands = axis.entries.iter().find_map(|en| match en {
            Entry::Plot(p) if p.handler == "contour filled" => {
                tikra_engine::keys::split_list(&p.style).into_iter().find(|i| i.key.trim_start_matches("/pgfplots/") == "contour filled").map(|i| i.value.unwrap_or_default())
            }
            _ => None,
        });
        cb
    });
    let parent = if colorbar.is_some() { "parent axis".to_string() } else { String::new() };
    for n in ["current axis".to_string(), name, parent].into_iter().filter(|n| !n.is_empty()) {
        register_axis_node(e, &n, &AnchorPlace { lo, hi, outer_lo: blo, outer_hi: bhi, frame: &shifted });
    }
    Ok(colorbar)
}

/// The axis for the figure's description: its kind, title, labels, ranges and plots.
pub(in crate::draw) fn note_axis(e: &mut Engine, axis: &Axis, f: &Frame) {
    use tikra_engine::describe::{AxisFact, Series};
    let mut names = crate::legend::entries(e, axis).into_iter().map(|l| tex::tokens_to_string(&l.text));
    let mut series = Vec::new();
    let mut colored = false;
    let marks_only = tex::tokens_to_string(&axis.options).contains("only marks");
    for en in &axis.entries {
        let Entry::Plot(p) = en else { continue };
        let name = if p.forget { String::new() } else { names.next().unwrap_or_default() };
        colored |= p.mesh.is_some() || p.patch || p.scatter || p.handler.starts_with("contour");
        let points = p.coords.iter().map(|c| [c.x, c.y, c.z]).collect();
        let boxplot = p.boxplot.as_ref().map(|b| [b.lower_whisker, b.lower_quartile, b.median, b.upper_quartile, b.upper_whisker]);
        series.push(Series { kind: plot_kind(p, marks_only), name, expression: p.expression.clone(), points, boxplot });
    }
    let kind = if f.polar {
        "polar"
    } else if f.ternary {
        "ternary"
    } else if f.smith {
        "smith"
    } else {
        ""
    };
    // bars stand on zero: their value axis shows it
    let base = |k: usize| series.iter().any(|s| s.kind == ["horizontal bar", "bar"][k.min(1)]) && k < 2;
    let range = |k: usize| {
        let [mut lo, mut hi] = f.data_range[k];
        if base(k) && !f.log[k] {
            (lo, hi) = (lo.min(0.0), hi.max(0.0));
        }
        let (lo, hi) = if f.log[k] { (lo.exp(), hi.exp()) } else { (lo, hi) };
        (lo.is_finite() && hi.is_finite() && lo <= hi).then_some([lo, hi])
    };
    let fact = AxisFact {
        kind,
        three_d: f.three_d,
        title: keys::value(e, "/pgfplots/title"),
        labels: [keys::value(e, "/pgfplots/xlabel"), keys::value(e, "/pgfplots/ylabel"), keys::value(e, "/pgfplots/zlabel")],
        range: [range(0), range(1), range(2)],
        log: f.log,
        symbolic: [crate::symbolic::names(e, 'x'), crate::symbolic::names(e, 'y'), crate::symbolic::names(e, 'z')],
        series,
        colors: (colored && f.colormap.is_some() && f.meta.iter().all(|v| v.is_finite())).then_some(f.meta),
    };
    e.facts.axes.push(fact);
}

/// What kind of plot it is, in words.
pub(in crate::draw) fn plot_kind(p: &Plot, marks_only: bool) -> &'static str {
    let style = tex::tokens_to_string(&p.style);
    let h = p.handler.as_str();
    if p.boxplot.is_some() || h == "boxplot" {
        "box plot"
    } else if h.starts_with("contour") {
        "contour"
    } else if h == "quiver" {
        "vector field"
    } else if h.ends_with("interval") {
        "histogram"
    } else if h.starts_with("ybar") {
        "bar"
    } else if h.starts_with("xbar") {
        "horizontal bar"
    } else if h.contains("comb") {
        "stem"
    } else if p.mesh == Some('s') || p.patch {
        "surface"
    } else if p.mesh == Some('m') {
        "mesh"
    } else if p.scatter || marks_only || style.contains("only marks") {
        "scatter"
    } else if p.fill_between.is_some() || p.post.iter().any(|t| t.is_cs("closedcycle")) {
        "area"
    } else {
        "line"
    }
}
