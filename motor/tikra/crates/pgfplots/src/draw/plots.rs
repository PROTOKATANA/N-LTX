use tikra_engine::canvas::Item;
use tikra_engine::engine::nodes::{NodeSpec, OnPath};
use tikra_engine::keys as pk;
use tikra_engine::scene::Point;
use tikra_engine::tex::{self, Token};
use tikra_engine::{Error, PT};

use crate::keys;
use crate::plot::Plot;
use crate::{toks, Axis, Engine, Entry};

use super::frame::*;
use super::prepare::*;
use super::util::*;

use super::anchor::{register_axis_node, AnchorPlace};
use super::marks::{draw_axis_2d, other_axis};

/// The axis background rectangle, before the axes and the plots.
pub(in crate::draw) fn draw_axis_background(e: &mut Engine, f: &Frame) -> Result<(), Error> {
    if let Some(Meaning(code)) = style_code(e, "/pgfplots/axis background") {
        let _ = code;
        let rect = box_path(f);
        draw_with(e, "draw=none,/pgfplots/axis background", rect)?;
    }
    Ok(())
}

/// Opens the clip, then reports the picture's bounding box (the clip counts).
pub(in crate::draw) fn open_clip(e: &mut Engine, f: &Frame, clip: bool) -> (Option<Point>, Option<Point>) {
    if clip {
        let segs = if f.smith {
            crate::smith::clip_path(f)
        } else if f.ternary {
            crate::ternary::outline(f)
        } else if f.polar {
            crate::polar::clip_path(f)
        } else if f.three_d {
            hull_path(f)
        } else {
            box_path(f)
        };
        // `clip bounding box=default tikz`: the clip path counts in the picture's size
        if !e.canvas_frozen {
            e.canvas.include_path(&segs);
        }
        e.open_in_scope(Item::Clip { segments: segs, even_odd: false });
    }
    (e.canvas.min, e.canvas.max)
}

/// Before compat 1.11, TikZ's unit vectors are those of the data scale transformation.
pub(in crate::draw) fn install_plot_units(e: &mut Engine, f: &Frame) -> (Point, Point, Point) {
    let compat = keys::compat(e);
    f.cs.set(if keys::substitute_pgfpoint(e) { Cs::Axis } else { Cs::Plain });
    let saved = (e.st.x, e.st.y, e.st.z);
    if compat < 111 && !f.polar && !f.smith && !f.ternary {
        let inverse = e.st.transform.inverse();
        let unit = |e: &mut Engine, a: usize| inverse.linear(f.e[a].scale(1.0 / data_scale_trafo(e, f, a).0));
        e.st.x = unit(e, 0);
        e.st.y = unit(e, 1);
        if f.three_d {
            e.st.z = unit(e, 2);
        }
    }
    saved
}

/// One plot of the axis, with its place among plots of the same handler.
pub(in crate::draw) struct PlotJob<'a> {
    setup: &'a Setup,
    plot: &'a Plot,
    index: usize,
}

/// Plots seen so far, and where the next `fill between` region is spliced.
pub(in crate::draw) struct PlotStream<'a> {
    plots: Vec<&'a Plot>,
    seen: usize,
    pre_main_end: usize,
    marks: Vec<crate::visual::Marks>,
}

pub(in crate::draw) struct EntryJob<'a> {
    axis: &'a Axis,
    setup: &'a Setup,
}

/// One `\addplot`: draw it, and sink a `fill between` region under the plots.
pub(in crate::draw) fn draw_plot_entry(e: &mut Engine, job: &PlotJob<'_>, stream: &mut PlotStream<'_>) -> Result<(), Error> {
    let s = job.setup;
    let p = job.plot;
    let i = job.index;
    let (pts, base) = s.points[i].clone().unwrap_or_default();
    let of_type = stream.plots.iter().filter(|q| q.handler == p.handler && !q.forget).count().max(1);
    let index_of_type = stream.plots[..stream.seen].iter().filter(|q| q.handler == p.handler && !q.forget).count();
    stream.seen += 1;
    let before = e.canvas.items.len();
    let r = crate::visual::draw_plot(e, p, &pts, &base, &s.frame, of_type, index_of_type);
    if p.fill_between.is_some() && before <= e.canvas.items.len() && stream.pre_main_end <= before {
        let drawn: Vec<Item> = e.canvas.items.drain(before..).collect();
        let at = stream.pre_main_end.min(e.canvas.items.len());
        stream.pre_main_end = at + drawn.len();
        e.canvas.items.splice(at..at, drawn);
    }
    match r {
        Ok(Some(m)) => stream.marks.push(m),
        Ok(None) => {}
        Err(err) if tikra_engine::engine::is_limit(&err) => return Err(err),
        Err(err) => e.warn(format!("\\addplot: {err}; plot not drawn")),
    }
    Ok(())
}

/// A TikZ statement kept from the axis body.
pub(in crate::draw) fn draw_tikz_entry(e: &mut Engine, tokens: &[Token]) -> Result<(), Error> {
    // what the axis' body draws is the figure's
    let mute = std::mem::take(&mut e.facts.mute);
    let r = e.run_tokens(tokens.to_vec());
    e.facts.mute = mute;
    if let Err(err) = r {
        if tikra_engine::engine::is_limit(&err) {
            return Err(err);
        }
        e.warn(format!("statement in axis: {err}"));
    }
    Ok(())
}

/// Plots and TikZ statements, in the order they were given.
pub(in crate::draw) fn draw_entries(e: &mut Engine, job: &EntryJob<'_>, stream: &mut PlotStream<'_>) -> Result<(), Error> {
    for (i, en) in job.axis.entries.iter().enumerate() {
        match en {
            Entry::Plot(p) => draw_plot_entry(e, &PlotJob { setup: job.setup, plot: p.as_ref(), index: i }, stream)?,
            Entry::Tikz(tokens) => draw_tikz_entry(e, tokens)?,
        }
    }
    Ok(())
}

/// `before end axis/.code`, at the end of the axis body.
pub(in crate::draw) fn run_before_end(e: &mut Engine, f: &Frame) -> Result<(), Error> {
    if e.key_exists("/pgfplots/before end axis") {
        let r = in_plain_units(e, f, |e| e.process_keys(toks("/pgfplots/before end axis"), None));
        if let Err(err) = r {
            if tikra_engine::engine::is_limit(&err) {
                return Err(err);
            }
            e.warn(format!("before end axis: {err}"));
        }
    }
    Ok(())
}

/// Marks kept to draw after the clip closes.
pub(in crate::draw) struct KeptMarks {
    marks: Vec<crate::visual::Marks>,
    clip: bool,
}

/// Marks are not clipped (`clip marker paths=false`); those outside the axis are dropped.
pub(in crate::draw) fn draw_mark_list(e: &mut Engine, f: &Frame, kept: KeptMarks) -> Result<(), Error> {
    // inside the box: by the point's axis coordinates (the box is a parallelogram with unit
    // vectors of any direction)
    let inside = |p: &Point| match f.data_at(*p) {
        Some(v) => (0..2).all(|k| {
            let (l, tol) = (f.logical(k, v[k]), 0.05 * PT / f.e[k].norm().max(f64::MIN_POSITIVE));
            l >= f.min[k].min(f.max[k]) - tol && l <= f.min[k].max(f.max[k]) + tol
        }),
        None => true,
    };
    for mut m in kept.marks {
        // marks outside the axis are not drawn, those on its border are drawn whole
        if kept.clip && !f.three_d && !f.polar && !f.smith {
            m.points.retain(|(p, _)| inside(p));
            m.styled.retain(|(p, _, _)| inside(p));
            m.nodes.retain(|n| inside(&n.point));
        } else if kept.clip && f.polar {
            m.points.retain(|(p, _)| crate::polar::contains(f, *p));
            m.nodes.retain(|n| crate::polar::contains(f, n.point));
        }
        crate::visual::draw_marks(e, &m, f)?;
        crate::visual::draw_near_nodes(e, &m)?;
    }
    Ok(())
}

/// `xmin`/`xmax` as computed, for a title that reads them.
pub(in crate::draw) fn publish_limits(e: &mut Engine, f: &Frame) {
    for a in 0..if f.three_d { 3 } else { 2 } {
        let c = axis_char(a);
        for (k, v) in [("min", f.min[a]), ("max", f.max[a])] {
            let v = if f.log[a] { v.exp() } else { v };
            if v.is_finite() && !f.polar && !f.ternary && !f.smith {
                keys::set(e, &format!("/pgfplots/{c}{k}"), &tikra_engine::engine::pgf_number(v));
            }
        }
    }
}

/// `current axis`, the descriptions, and a smith chart's origin.
pub(in crate::draw) fn finish_descriptions(e: &mut Engine, axis: &Axis, s: &Setup) -> Result<(), Error> {
    let f = s.frame.as_ref();
    // `current axis` exists for the descriptions (`current axis.right of origin`)
    let (lo, hi) = f.bbox();
    let (blo, bhi) = e.canvas.min.zip(e.canvas.max).unwrap_or((lo, hi));
    register_axis_node(e, "current axis", &AnchorPlace { lo, hi, outer_lo: blo, outer_hi: bhi, frame: f });
    // descriptions, in description coordinates
    f.cs.set(Cs::Description);
    // the descriptions see the limits as computed (`\pgfkeysvalueof{/pgfplots/xmin}` in a title)
    e.begin_group();
    publish_limits(e, f);
    let r = descriptions(e, axis, s);
    e.end_group();
    r?;
    f.cs.set(Cs::Plain);
    if f.smith && keys::flag(e, "/pgfplots/show origin") {
        crate::smith::show_origin(e, f)?;
    }
    Ok(())
}

/// Everything of the axis, in pgfplots' order.
pub(in crate::draw) fn draw_all(e: &mut Engine, axis: &Axis, s: &Setup) -> Result<(), Error> {
    let f = s.frame.clone();
    let on_top = keys::flag(e, "/pgfplots/axis on top");
    draw_axis_background(e, &f)?;
    if f.ternary || f.smith {
        if !on_top {
            other_axis(e, s)?;
        }
    } else if f.polar && !on_top {
        crate::polar::draw_axis(e, s)?;
    } else if !f.three_d && !on_top {
        draw_axis_2d(e, s)?;
    }
    if f.three_d && !f.ternary && !on_top {
        crate::cube::draw(e, s, false)?;
    }
    // the plots, clipped
    let clip = keys::flag(e, "/pgfplots/clip");
    e.begin_scope();
    let saved_bbox = open_clip(e, &f, clip);
    // the layer `pre main`: fill between regions go below the other plots
    let pre_main = e.canvas.items.len();
    let units = install_plot_units(e, &f);
    let plots = axis.entries.iter().filter_map(|en| if let Entry::Plot(p) = en { Some(p.as_ref()) } else { None }).collect();
    let mut stream = PlotStream { plots, seen: 0, pre_main_end: pre_main, marks: Vec::new() };
    draw_entries(e, &EntryJob { axis, setup: s }, &mut stream)?;
    run_before_end(e, &f)?;
    e.end_scope();
    (e.st.x, e.st.y, e.st.z) = units;
    if clip {
        (e.canvas.min, e.canvas.max) = saved_bbox;
    }
    // `3d box=complete`: the foreground edges, on top of the plots
    if f.three_d && !f.ternary && pv(e, "3d box") != "background" {
        crate::cube::draw(e, s, true)?;
    }
    if on_top {
        if f.ternary || f.smith {
            other_axis(e, s)?;
        } else if f.polar {
            crate::polar::draw_axis(e, s)?;
        } else if f.three_d {
            crate::cube::draw(e, s, false)?;
        } else {
            draw_axis_2d(e, s)?;
        }
    }
    draw_mark_list(e, &f, KeptMarks { marks: stream.marks, clip })?;
    finish_descriptions(e, axis, s)
}
/// Labels, title and legend.
pub(in crate::draw) fn descriptions(e: &mut Engine, axis: &Axis, s: &Setup) -> Result<(), Error> {
    let f = s.frame.clone();
    let labels: &[(usize, &str)] = if f.three_d { &[(0, "x"), (1, "y"), (2, "z")] } else { &[(0, "x"), (1, "y")] };
    for &(a, c) in labels {
        // an empty label is still a node in TeX's drawing: it takes room at its place
        let mut text = keys::value_tokens(&*e, &format!("/pgfplots/{c}label")).unwrap_or_default();
        if s.hide[a] {
            continue;
        }
        // (but not in a colorbar, which TeX's drawing shows without them)
        if pk::trim(&text).is_empty() && keys::flag(e, "/pgfplots/@colorbar axis") {
            continue;
        }
        // the units library: ` [km]` after the label (`x unit=m`, `x unit prefix=k`)
        let unit = keys::value(e, &format!("/pgfplots/{c} unit"));
        if !unit.trim().is_empty() && keys::flag(e, "/pgfplots/use units") {
            let prefix = keys::value(e, &format!("/pgfplots/{c} unit prefix"));
            text.extend(tex::string_tokens(" "));
            text.extend(toks(&format!("$\\left[\\mathrm{{{prefix}{unit}}}\\right]$")));
        }
        f.label_axis.set(a);
        let p = f.desc(0.0, 0.0);
        let options = toks(&format!("/pgfplots/every axis label,/pgfplots/every axis {c} label"));
        let text = crate::print_numbers(e, &text)?;
        // `sloped` in a label is `sloped like x axis`
        e.begin_group();
        let slope = e.tex.meaning_of(&pk::macro_name(&format!("/tikz/sloped like {c} axis/.@cmd")));
        e.tex.set_meaning(&Token::cs(&pk::macro_name("/tikz/sloped/.@cmd")), slope, false);
        let items = e.node(NodeSpec { options, name: None, at: None, text, place: OnPath::Point(p), point: p, coordinate: false, default_pos: None });
        e.end_group();
        e.canvas.items.extend(items?);
    }
    if let Some(text) = keys::value_tokens(&*e, "/pgfplots/title") {
        let p = f.desc(0.0, 0.0);
        let text = crate::print_numbers(e, &text)?;
        let items =
            e.node(NodeSpec { options: toks("/pgfplots/every axis title"), name: None, at: None, text, place: OnPath::Point(p), point: p, coordinate: false, default_pos: None })?;
        e.canvas.items.extend(items);
    }
    crate::legend::legend(e, axis, &f)?;
    // `extra description/.code`, in description coordinates too
    if e.key_exists("/pgfplots/extra description") {
        e.begin_group();
        let r = e.process_keys(toks("/pgfplots/extra description"), None);
        e.end_group();
        r?;
    }
    // `after end axis/.code`: plain coordinates again, nothing clipped
    if e.key_exists("/pgfplots/after end axis") {
        let cs = f.cs.get();
        f.cs.set(if keys::substitute_pgfpoint(e) { Cs::Axis } else { Cs::Plain });
        e.begin_group();
        let r = in_plain_units(e, &f, |e| e.process_keys(toks("/pgfplots/after end axis"), None));
        e.end_group();
        f.cs.set(cs);
        r?;
    }
    Ok(())
}
