use tikra_engine::canvas::{Fill, Item, Paint};
use tikra_engine::keys as pk;
use tikra_engine::scene::{Point, Segment, Transform};
use tikra_engine::tex::{self, Token};
use tikra_engine::{Error, PT};

use super::super::draw::Frame;
use super::super::keys;
use super::super::marks::{self, Use};
use super::super::plot::Plot;
use super::super::{toks, Engine};

use super::{define, unit, CubeStamp, MarkStamp, Marks};

/// The ends of a plot's error bars, `[x minus, x plus, y minus, y plus]` per point (NaN: none):
/// pgfplots widens the axis limits to them.
pub fn error_ends(e: &mut Engine, p: &Plot) -> Result<Vec<[f64; 4]>, Error> {
    e.begin_group();
    let r = (|| -> Result<Vec<[f64; 4]>, Error> {
        e.process_keys(p.style.clone(), Some("/pgfplots"))?;
        let mut out = vec![[f64::NAN; 4]; p.coords.len()];
        for axis in 0..2 {
            let c = ['x', 'y'][axis];
            let dir = keys::value(e, &format!("/pgfplots/error bars/{c} dir"));
            if dir == "none" || dir.is_empty() {
                continue;
            }
            let mode = keys::value(e, &format!("/pgfplots/error bars/@{c} mode"));
            let fixed = keys::number(e, &format!("/pgfplots/error bars/@{c} value")).unwrap_or(0.0);
            for (i, co) in p.coords.iter().enumerate() {
                let (em, ep) = error_amounts(co, axis, &mode, fixed);
                let raw = [co.x, co.y][axis];
                if dir != "plus" && em != 0.0 {
                    out[i][axis * 2] = raw - em;
                }
                if dir != "minus" && ep != 0.0 {
                    out[i][axis * 2 + 1] = raw + ep;
                }
            }
        }
        Ok(out)
    })();
    e.end_group();
    r
}

/// The minus and plus error of a point on `axis`, by the error bars' mode.
pub(super) fn error_amounts(co: &super::super::plot::Coord, axis: usize, mode: &str, fixed: f64) -> (f64, f64) {
    let raw = [co.x, co.y][axis];
    let (em, ep) = match mode {
        "explicit" => (co.err[axis * 2], co.err[axis * 2 + 1]),
        "explicit relative" => (co.err[axis * 2] * raw.abs(), co.err[axis * 2 + 1] * raw.abs()),
        "fixed relative" => (fixed * raw.abs(), fixed * raw.abs()),
        _ => (fixed, fixed),
    };
    (if em.is_finite() { em.abs() } else { 0.0 }, if ep.is_finite() { ep.abs() } else { 0.0 })
}

/// Error bars: a line from minus to plus error, with `error mark` at its ends.
pub(super) fn error_bars(e: &mut Engine, p: &Plot, pts: &[[f64; 3]], f: &Frame) -> Result<(), Error> {
    let ydir = keys::value(e, "/pgfplots/error bars/y dir");
    let xdir = keys::value(e, "/pgfplots/error bars/x dir");
    if ydir == "none" && xdir == "none" {
        return Ok(());
    }
    let mut segs = Vec::new();
    let mut caps = Vec::new();
    for (axis, dir) in [(1usize, ydir), (0usize, xdir)] {
        if dir == "none" {
            continue;
        }
        let c = ['x', 'y'][axis];
        let mode = keys::value(e, &format!("/pgfplots/error bars/@{c} mode"));
        let fixed = keys::number(e, &format!("/pgfplots/error bars/@{c} value")).unwrap_or(0.0);
        for (i, l) in pts.iter().enumerate() {
            if !(l[0].is_finite() && l[1].is_finite()) {
                continue;
            }
            let Some(co) = p.coords.get(i) else { continue };
            let raw = [co.x, co.y][axis];
            let (em, ep) = error_amounts(co, axis, &mode, fixed);
            if em == 0.0 && ep == 0.0 {
                continue;
            }
            let lo = if dir == "plus" { raw } else { raw - em };
            let hi = if dir == "minus" { raw } else { raw + ep };
            let mut a = *l;
            let mut b = *l;
            a[axis] = f.logical(axis, lo);
            b[axis] = f.logical(axis, hi);
            let (pa, pb) = (f.at(a), f.at(b));
            segs.push(Segment::MoveTo(pa));
            segs.push(Segment::LineTo(pb));
            let d = unit(pb.minus(pa));
            let n = Point::new(-d.y, d.x).scale(2.0 * PT);
            for q in [pa, pb] {
                caps.push(Segment::MoveTo(q.minus(n)));
                caps.push(Segment::LineTo(q.plus(n)));
            }
        }
    }
    segs.extend(caps);
    if segs.is_empty() {
        return Ok(());
    }
    e.begin_group();
    let r = (|| {
        e.st.mode = Default::default();
        e.st.mode.draw = true;
        e.st.pen.dash = None;
        e.process_keys(toks("/pgfplots/every error bar"), Some("/pgfplots"))?;
        e.use_path(segs, "error bars")
    })();
    e.end_group();
    r
}

pub(super) struct MarkRun {
    name: String,
    size: f64,
    local: Transform,
    cube: bool,
    colors: Option<(super::super::mesh::Linear, super::super::mesh::Linear, tikra_engine::engine::GState)>,
    chosen: Vec<(usize, Point, f64)>,
    custom: bool,
}

pub(super) struct MarkPaint<'a> {
    marks: &'a Marks,
    frame: &'a Frame,
    run: &'a MarkRun,
}

pub(super) fn marker_text(e: &Engine, which: &str) -> String {
    match e.tex.meaning_of(&pk::macro_name(&format!("/pgfplots/scatter/{which} marker code/.@cmd"))) {
        tex::Meaning::Macro(m) => tex::tokens_to_string(&tex::substitute(&m.body, &[])),
        _ => String::new(),
    }
}

pub(super) fn bare_marker(s: &str) -> String {
    s.replace([' ', '{', '}'], "").replace("\\pgfeov", "")
}

pub(super) fn prepare_marks(e: &mut Engine, m: &Marks, f: &Frame) -> Result<Option<MarkRun>, Error> {
    // the mark size is the plot's: a `mark size` in `every mark` comes too late for pgfplots'
    // marks (as the size styles' `every mark/.append style={mark size=8}` shows)
    let plot_size = std::cell::Cell::new(2.0 * PT);
    let setup = |e: &mut Engine| -> Result<(), Error> {
        e.st.mode = Default::default();
        e.st.pending = Transform::default();
        e.process_keys(m.style.clone(), Some("/pgfplots"))?;
        e.st.apply_pending();
        e.st.pending = Transform::default();
        plot_size.set(e.st.plot.mark_size);
        e.process_keys(toks("every mark"), Some("/tikz"))?;
        if m.scatter {
            if let Some(v) = keys::value_tokens(e, "/pgfplots/scatter/use mapped color") {
                e.process_keys(v, Some("/tikz"))?;
            }
        }
        Ok(())
    };
    let colors = if m.scatter { Some(super::super::mesh::mapped_colors(e, setup)?) } else { None };
    setup(e)?;
    let Some(name) = e.st.plot.plot_mark.clone() else { return Ok(None) };
    let local = e.st.pending;
    let size = plot_size.get();
    let cube = f.three_d && (name == "cube" || name == "cube*");
    // the native keys keep these in the state
    let repeat = e.st.plot.mark_repeat.max(1);
    let phase = e.st.plot.mark_phase.max(1);
    let indices: Vec<usize> = match keys::value_tokens(e, "/tikz/mark indices") {
        Some(v) => {
            let list = e.tex.expand_fully(v)?;
            tikra_engine::engine::foreach_items(&list).into_iter().filter_map(|i| tex::tokens_to_string(&i).trim().parse().ok()).collect()
        }
        None => e.st.plot.mark_indices.clone().unwrap_or_default(),
    };
    // `scatter/@pre marker code` and `@post marker code` of the document: run around each mark,
    // with `\pgfplotspointmetatransformed` (the default code sets the mapped color)
    let (pre, post) = (marker_text(e, "@pre"), marker_text(e, "@post"));
    let custom = m.scatter && (bare_marker(&pre) != bare_marker("\\def\\tikra@scatter@usemapped{1}") || !bare_marker(&post).is_empty());
    let chosen = m
        .points
        .iter()
        .copied()
        .enumerate()
        .filter(|(i, _)| {
            let k = i + 1;
            if !indices.is_empty() {
                return indices.contains(&k);
            }
            k >= phase && (k - phase).is_multiple_of(repeat)
        })
        .map(|(i, (p, meta))| (i, p, meta))
        .collect();
    Ok(Some(MarkRun { name, size, local, cube, colors, chosen, custom }))
}

pub(super) fn draw_custom_scatter(e: &mut Engine, paint: &MarkPaint<'_>) -> Result<(), Error> {
    let (draw, fill) = match paint.run.colors.as_ref() {
        Some((draw, fill, _)) => (*draw, *fill),
        None => return Ok(()),
    };
    let m = paint.marks;
    let f = paint.frame;
    for (i, p, meta) in &paint.run.chosen {
        e.begin_group();
        let r = (|| -> Result<(), Error> {
            let t = (meta - f.meta[0]) / (f.meta[1] - f.meta[0]) * 1000.0;
            define(e, "pgfplotspointmetatransformed", &tikra_engine::engine::pgf_number(if t.is_finite() { t } else { 0.0 }));
            define(e, "pgfplotspointmeta", &tikra_engine::engine::pgf_number(*meta));
            define(e, "tikra@scatter@usemapped", "0");
            for (name, value) in m.deps.get(*i).into_iter().flatten() {
                define(e, name, value);
            }
            e.process_keys(toks("/pgfplots/scatter/@pre marker code"), None)?;
            if e.tex.expand_to_string(vec![Token::cs("tikra@scatter@usemapped")])?.trim() == "1" {
                let mapped = super::super::mesh::map(f, *meta);
                e.st.paint.draw_color = Some(draw.at(mapped));
                e.st.paint.fill_color = Some(fill.at(mapped));
            }
            let name = e.st.plot.plot_mark.clone().unwrap_or_else(|| paint.run.name.clone());
            let local = e.st.pending;
            e.st.pending = Transform::default();
            draw_mark_set(e, &MarkStamp { name: &name, size: e.st.plot.mark_size, local: &local }, std::iter::once(*p))?;
            e.process_keys(toks("/pgfplots/scatter/@post marker code"), None)
        })();
        e.end_group();
        r?;
    }
    Ok(())
}

pub(super) fn draw_mapped_scatter(e: &mut Engine, paint: &MarkPaint<'_>) -> Result<(), Error> {
    let (draw, fill) = match paint.run.colors.as_ref() {
        Some((draw, fill, _)) => (*draw, *fill),
        None => return Ok(()),
    };
    let f = paint.frame;
    let run = paint.run;
    for (_, p, meta) in &run.chosen {
        let mapped = super::super::mesh::map(f, *meta);
        e.st.paint.draw_color = Some(draw.at(mapped));
        e.st.paint.fill_color = Some(fill.at(mapped));
        if run.name == "ball" {
            // `ball color=mapped color`: the shading's color, point by point
            e.colors.insert("mapped color".into(), mapped);
            if let Some(v) = keys::value_tokens(e, "/pgfplots/scatter/use mapped color") {
                e.process_keys(v, Some("/tikz"))?;
            }
            e.colors.insert("mapped color".into(), tikra_engine::color::Color::BLACK);
        }
        if run.cube {
            draw_cubes(e, &CubeStamp { name: &run.name, size: e.st.plot.mark_size, frame: f }, std::iter::once(*p))?;
        } else {
            draw_mark_set(e, &MarkStamp { name: &run.name, size: run.size, local: &run.local }, std::iter::once(*p))?;
        }
    }
    Ok(())
}

pub(super) fn draw_plain_marks(e: &mut Engine, paint: &MarkPaint<'_>) -> Result<(), Error> {
    let run = paint.run;
    if run.cube {
        draw_cubes(e, &CubeStamp { name: &run.name, size: e.st.plot.mark_size, frame: paint.frame }, run.chosen.iter().map(|(_, p, _)| *p))
    } else {
        draw_mark_set(e, &MarkStamp { name: &run.name, size: run.size, local: &run.local }, run.chosen.iter().map(|(_, p, _)| *p))
    }
}

pub(super) fn draw_chosen_marks(e: &mut Engine, paint: &MarkPaint<'_>) -> Result<(), Error> {
    if paint.run.colors.is_some() && paint.run.custom {
        return draw_custom_scatter(e, paint);
    }
    if paint.run.colors.is_some() {
        return draw_mapped_scatter(e, paint);
    }
    draw_plain_marks(e, paint)
}

/// `coordinate style` and `scatter/classes`: each such mark with its own style.
pub(super) fn draw_styled_marks(e: &mut Engine, paint: &MarkPaint<'_>) -> Result<(), Error> {
    let m = paint.marks;
    let f = paint.frame;
    let run = paint.run;
    // `scatter/classes`: the class' style colors the mark, not the colormap
    let classes = m.scatter && keys::value_tokens(e, "/pgfplots/scatter/classes").is_some();
    for (p, meta, style) in &m.styled {
        e.begin_group();
        let r = (|| -> Result<(), Error> {
            if let Some((draw, fill, _)) = run.colors.as_ref().filter(|_| !classes) {
                let mapped = super::super::mesh::map(f, *meta);
                e.st.paint.draw_color = Some(draw.at(mapped));
                e.st.paint.fill_color = Some(fill.at(mapped));
            }
            e.st.pending = run.local;
            e.process_keys(style.clone(), Some("/tikz"))?;
            let local = e.st.pending;
            e.st.pending = Transform::default();
            let name = e.st.plot.plot_mark.clone().unwrap_or_else(|| run.name.clone());
            // a shift in the style moves the mark
            draw_mark_set(e, &MarkStamp { name: &name, size: run.size, local: &local }, std::iter::once(p.plus(Point::new(local.e, local.f))))
        })();
        e.end_group();
        r?;
    }
    Ok(())
}

/// Draws the marks of a plot, as `\pgfplothandlermark` does: `every mark` and `mark options`
/// apply, each mark is a path at the point.
pub fn draw_marks(e: &mut Engine, m: &Marks, f: &Frame) -> Result<(), Error> {
    e.begin_group();
    let r = (|| -> Result<(), Error> {
        let Some(run) = prepare_marks(e, m, f)? else { return Ok(()) };
        let paint = MarkPaint { marks: m, frame: f, run: &run };
        draw_chosen_marks(e, &paint)?;
        draw_styled_marks(e, &paint)
    })();
    e.end_group();
    r
}

/// `mark=cube` and `cube*` in a 3d axis: a cube with edges `size` long along the axes
/// (`cube/size x=\pgfplotmarksize`); `cube*` fills the three faces in front.
pub(super) fn draw_cubes(e: &mut Engine, stamp: &CubeStamp<'_>, points: impl Iterator<Item = Point>) -> Result<(), Error> {
    let CubeStamp { name, size, frame: f } = *stamp;
    let half = [0, 1, 2].map(|k| {
        let n = f.e[k].norm();
        if n > 0.0 {
            f.e[k].scale(0.5 * size / n)
        } else {
            Point::default()
        }
    });
    let stroke = e.stroke();
    let fill = Fill { paint: Paint::Solid(e.st.fill_paint_color()), opacity: e.st.paint.fill_opacity, even_odd: false };
    for p in points {
        let at = |s: [f64; 3]| p.plus(half[0].scale(s[0])).plus(half[1].scale(s[1])).plus(half[2].scale(s[2]));
        let face = |k: usize, side: f64| -> Vec<Segment> {
            // the four corners of the face at `side` across axis k, around
            let (a, b) = ((k + 1) % 3, (k + 2) % 3);
            let mut c = Vec::new();
            for (u, v) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, 1.0), (1.0, -1.0)] {
                let mut s = [0.0; 3];
                s[k] = side;
                s[a] = u;
                s[b] = v;
                c.push(at(s));
            }
            vec![Segment::MoveTo(c[0]), Segment::LineTo(c[1]), Segment::LineTo(c[2]), Segment::LineTo(c[3]), Segment::Close]
        };
        if name == "cube*" {
            for k in 0..3 {
                let segs = face(k, if f.fg[k][0] { -1.0 } else { 1.0 });
                if !e.canvas_frozen {
                    e.canvas.include_path(&segs);
                }
                e.canvas.push(Item::Path { segments: segs, fill: Some(fill.clone()), stroke: Some(stroke.clone()) });
            }
        } else {
            let mut segs = face(0, -1.0);
            segs.extend(face(0, 1.0));
            for (u, v) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, 1.0), (1.0, -1.0)] {
                segs.push(Segment::MoveTo(at([1.0, u, v])));
                segs.push(Segment::LineTo(at([-1.0, u, v])));
            }
            if !e.canvas_frozen {
                e.canvas.include_path(&segs);
            }
            e.canvas.push(Item::Path { segments: segs, fill: None, stroke: Some(stroke.clone()) });
        }
    }
    Ok(())
}

/// Marks named `name` of size `size` at `points`, in the current state.
pub fn draw_mark_set(e: &mut Engine, stamp: &MarkStamp<'_>, points: impl Iterator<Item = Point>) -> Result<(), Error> {
    let MarkStamp { name, size, local } = *stamp;
    // a shaded ball: the engine's mark, with its `ball color`
    if name == "ball" {
        let pts: Vec<Point> = points.collect();
        let _ = size;
        let items = e.plot_marks(&pts)?;
        e.canvas.items.extend(items);
        let _ = local;
        return Ok(());
    }
    let Some((shape, usage)) = marks::mark(name, size) else {
        if name == "text" {
            return Ok(());
        }
        e.warn(format!("mark '{name}' not supported: drawn as *"));
        return draw_mark_set(e, &MarkStamp { name: "*", size, local }, points);
    };
    let stroke = e.stroke();
    let fill = Fill { paint: Paint::Solid(e.st.fill_paint_color()), opacity: e.st.paint.fill_opacity, even_odd: false };
    let lin = Transform { e: 0.0, f: 0.0, ..*local };
    for p in points {
        let t = Transform::translation(p.x, p.y).compose(&lin);
        let segs = tikra_engine::canvas::transform_segments(&shape, &t);
        if !e.canvas_frozen {
            e.canvas.include_path(&segs);
        }
        let (f, s) = match usage {
            Use::Stroke => (None, Some(stroke.clone())),
            Use::Fill => (Some(fill.clone()), None),
            Use::FillStroke => (Some(fill.clone()), Some(stroke.clone())),
        };
        e.canvas.push(Item::Path { segments: segs, fill: f, stroke: s });
    }
    Ok(())
}
