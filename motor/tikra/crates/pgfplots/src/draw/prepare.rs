use std::cell::{Cell, RefCell};
use std::rc::Rc;

use tikra_engine::scene::Point;
use tikra_engine::{Error, PT};

use crate::keys;
use crate::plot::Plot;
use crate::ticks::{self};
use crate::{Axis, Engine, Entry};

use super::frame::*;
use super::meta::*;
use super::util::*;

/// What an axis knows once its limits are final.
pub(crate) struct Setup {
    pub frame: Rc<Frame>,
    /// per plot: logical coordinates (after stacking) and the stack's base
    pub points: Vec<PlotPoints>,
    pub ticks: [Vec<f64>; 3],
    pub tick_distance: [f64; 3],
    pub hide: [bool; 3],
    pub line: [String; 3],
}

/// Limits, scaling and ticks.
pub(in crate::draw) fn setup(e: &mut Engine, axis: &Axis) -> Result<Setup, Error> {
    let mut prep = begin_prep(e, axis)?;
    collect_points(e, axis, &mut prep)?;
    finish_data_range(&mut prep);
    apply_stated_limits(e, &mut prep);
    enlarge_limits(e, &mut prep);
    scale_to_paper(e, &mut prep);
    assemble_setup(e, axis, prep)
}

/// What `setup` accumulates, in the order pgfplots reads the keys.
#[allow(clippy::struct_excessive_bools)]
pub(in crate::draw) struct Prep {
    ternary: bool,
    smith: bool,
    three_d: bool,
    polar: bool,
    log: [bool; 3],
    user: [(Option<f64>, Option<f64>); 3],
    points: Vec<PlotPoints>,
    dmin: [f64; 3],
    dmax: [f64; 3],
    data_range: [[f64; 2]; 3],
    min: [f64; 3],
    max: [f64; 3],
    auto_min: [bool; 3],
    auto_max: [bool; 3],
    dims: usize,
    invalid: bool,
    abs_dim: [Option<(f64, bool, bool)>; 3],
    width: f64,
    height: f64,
    dir: [bool; 3],
    ev: [Point; 3],
}

/// A natural logarithm on a log axis; anything else is left as given.
pub(in crate::draw) fn logged(log: &[bool; 3], a: usize, v: f64) -> f64 {
    if log[a] {
        if v > 0.0 {
            v.ln()
        } else {
            f64::NAN
        }
    } else {
        v
    }
}

/// Kind, late options, log modes and the limits the user wrote.
pub(in crate::draw) fn begin_prep(e: &mut Engine, axis: &Axis) -> Result<Prep, Error> {
    let ternary = axis.kind == "ternaryaxis";
    let smith = axis.kind == "smithchart";
    let three_d = ternary || axis.entries.iter().any(|en| matches!(en, Entry::Plot(p) if p.three_d)) || !pv(e, "zmin").is_empty() || !pv(e, "zmax").is_empty();
    if three_d {
        three_d_descriptions(e, axis)?;
    }
    // `late options` of the plots (`matrix plot`: `y dir=reverse, axis on top`)
    for en in &axis.entries {
        if let Entry::Plot(p) = en {
            if !p.late.is_empty() {
                let saved = e.st.key_path.clone();
                e.process_keys(p.late.clone(), Some("/pgfplots"))?;
                e.st.key_path = saved;
            }
        }
    }
    let log = [pv(e, "xmode") == "log", pv(e, "ymode") == "log", pv(e, "zmode") == "log"];
    let mut user: [(Option<f64>, Option<f64>); 3] = [(None, None); 3];
    for (a, u) in user.iter_mut().enumerate() {
        let c = axis_char(a);
        let lo = crate::symbolic::number(e, c, &format!("/pgfplots/{c}min")).map(|v| logged(&log, a, v));
        let hi = crate::symbolic::number(e, c, &format!("/pgfplots/{c}max")).map(|v| logged(&log, a, v));
        *u = (lo, hi);
    }
    Ok(Prep {
        ternary,
        smith,
        three_d,
        polar: axis.kind == "polaraxis",
        log,
        user,
        points: Vec::new(),
        dmin: [f64::INFINITY; 3],
        dmax: [f64::NEG_INFINITY; 3],
        data_range: [[0.0; 2]; 3],
        min: [0.0; 3],
        max: [0.0; 3],
        auto_min: [true; 3],
        auto_max: [true; 3],
        dims: if three_d { 3 } else { 2 },
        invalid: false,
        abs_dim: [None; 3],
        width: 0.0,
        height: 0.0,
        dir: [false; 3],
        ev: [Point::default(); 3],
    })
}

/// The two stacks of `stack plots` (`stack negative=separate` keeps its own).
pub(in crate::draw) struct Stacks {
    base: [Vec<f64>; 2],
    neg: [Vec<f64>; 2],
    separate: bool,
}

/// One plot's coordinates while they are stacked.
pub(in crate::draw) struct Stacked<'a> {
    coords: &'a mut Vec<[f64; 3]>,
    base: &'a mut Vec<f64>,
    stacks: &'a mut Stacks,
}

/// A box plot's outliers and the coordinates they may still move.
pub(in crate::draw) struct BoxExtent<'a> {
    plot: &'a Plot,
    index: usize,
    coords: &'a mut Vec<[f64; 3]>,
}

/// Error-bar ends, next to the coordinates they hang from.
pub(in crate::draw) struct ErrorView<'a> {
    plot: &'a Plot,
    coords: &'a [[f64; 3]],
    prep: &'a mut Prep,
}

/// Quiver arrow ends, when `quiver/update limits` asks for them.
pub(in crate::draw) fn include_quiver(e: &mut Engine, p: &Plot, prep: &mut Prep) {
    if p.handler == "quiver" && keys::flag(e, "/pgfplots/quiver/update limits") {
        let ends: Vec<[f64; 3]> =
            p.coords.iter().map(|c| [logged(&prep.log, 0, c.x + c.uvw[0]), logged(&prep.log, 1, c.y + c.uvw[1]), logged(&prep.log, 2, c.z + c.uvw[2])]).collect();
        let dims = if prep.three_d { 3 } else { 2 };
        for c in ends {
            for (i, v) in c.iter().take(dims).enumerate() {
                if v.is_finite() {
                    prep.dmin[i] = prep.dmin[i].min(*v);
                    prep.dmax[i] = prep.dmax[i].max(*v);
                }
            }
        }
    }
}

/// Error-bar ends (`\pgfplots@PREPARE@errorbar@stream@it@`).
pub(in crate::draw) fn include_error_bars(e: &mut Engine, seen: &mut ErrorView<'_>) -> Result<(), Error> {
    if seen.prep.three_d || seen.plot.boxplot.is_some() {
        return Ok(());
    }
    for (k, ends) in crate::visual::error_ends(e, seen.plot)?.into_iter().enumerate() {
        let Some(c) = seen.coords.get(k).copied() else { break };
        for (j, end) in ends.into_iter().enumerate() {
            let a = j / 2;
            let end = logged(&seen.prep.log, a, end);
            if end.is_finite() && c[1 - a].is_finite() {
                seen.prep.dmin[a] = seen.prep.dmin[a].min(end);
                seen.prep.dmax[a] = seen.prep.dmax[a].max(end);
            }
        }
    }
    Ok(())
}

/// A box plot's outliers, box and whiskers.
pub(in crate::draw) fn include_boxplot(e: &mut Engine, job: &mut BoxExtent<'_>, prep: &mut Prep) {
    for c in crate::boxplot::extent(e, job.plot, job.index, job.coords) {
        for (i, v) in c.iter().enumerate() {
            if v.is_finite() {
                prep.dmin[i] = prep.dmin[i].min(*v);
                prep.dmax[i] = prep.dmax[i].max(*v);
            }
        }
    }
}

/// `stack plots`: each point stands on the previous plot's value.
pub(in crate::draw) fn stack_coordinates(p: &Plot, stacked: &mut Stacked<'_>) {
    let Some(dir) = p.stack else { return };
    let a = if dir == 'x' { 0 } else { 1 };
    let n = stacked.coords.len();
    for list in [&mut stacked.stacks.base[a], &mut stacked.stacks.neg[a]] {
        if list.len() < n {
            list.resize(n, 0.0);
        }
    }
    let separate = stacked.stacks.separate;
    for i in 0..n {
        let cur = stacked.coords[i][a];
        let val = if p.stack_minus { -cur } else { cur };
        let prev = if separate && val < 0.0 { stacked.stacks.neg[a][i] } else { stacked.stacks.base[a][i] };
        stacked.base[i] = prev;
        let next = prev + if val.is_finite() { val } else { 0.0 };
        stacked.coords[i][a] = next;
        if separate && val < 0.0 {
            stacked.stacks.neg[a][i] = next;
        } else {
            stacked.stacks.base[a][i] = next;
        }
    }
}

/// Points that are allowed to widen the limits (`update limits`, `clip limits`).
pub(in crate::draw) fn widen_kept(p: &Plot, coords: &[[f64; 3]], prep: &mut Prep) {
    for c in coords.iter().filter(|_| !p.keep_limits) {
        if !(c[0].is_finite() && c[1].is_finite()) {
            continue;
        }
        for i in 0..3 {
            if i == 2 && !prep.three_d {
                continue;
            }
            // `clip limits`: a point beyond another axis' given limits does not widen this one
            let outside = (0..3).any(|j| j != i && (j < 2 || prep.three_d) && (prep.user[j].0.is_some_and(|m| c[j] < m) || prep.user[j].1.is_some_and(|m| c[j] > m)));
            if c[i].is_finite() && !outside {
                prep.dmin[i] = prep.dmin[i].min(c[i]);
                prep.dmax[i] = prep.dmax[i].max(c[i]);
            }
        }
    }
}

/// A matrix plot's cells reach halfway to the next point.
pub(in crate::draw) fn include_image(p: &Plot, coords: &[[f64; 3]], prep: &mut Prep) {
    if let Some((rows, cols)) = p.lattice.filter(|_| p.image) {
        for c in crate::mesh::image_limits(coords, rows, cols, 0) {
            for (i, v) in c.iter().take(2).enumerate() {
                if v.is_finite() {
                    prep.dmin[i] = prep.dmin[i].min(*v);
                    prep.dmax[i] = prep.dmax[i].max(*v);
                }
            }
        }
    }
}

/// Every plot's logical coordinates, and how far the data reaches.
pub(in crate::draw) fn collect_points(e: &mut Engine, axis: &Axis, prep: &mut Prep) -> Result<(), Error> {
    let mut stacks = Stacks { base: [Vec::new(), Vec::new()], neg: [Vec::new(), Vec::new()], separate: pv(e, "stack negative") == "separate" };
    let mut boxplots = 0usize;
    crate::boxplot::variable_widths(&axis.entries.iter().filter_map(|en| if let Entry::Plot(p) = en { p.boxplot.as_deref() } else { None }).collect::<Vec<_>>());
    for en in &axis.entries {
        let Entry::Plot(p) = en else {
            prep.points.push(None);
            continue;
        };
        let log = prep.log;
        let mut coords: Vec<[f64; 3]> = p.coords.iter().map(|c| [logged(&log, 0, c.x), logged(&log, 1, c.y), logged(&log, 2, c.z)]).collect();
        let mut base = vec![0.0; coords.len()];
        include_quiver(e, p, prep);
        include_error_bars(e, &mut ErrorView { plot: p, coords: &coords, prep })?;
        if p.boxplot.is_some() {
            include_boxplot(e, &mut BoxExtent { plot: p, index: boxplots, coords: &mut coords }, prep);
            boxplots += 1;
        }
        if p.stack.is_some() {
            stack_coordinates(p, &mut Stacked { coords: &mut coords, base: &mut base, stacks: &mut stacks });
        }
        widen_kept(p, &coords, prep);
        include_image(p, &coords, prep);
        prep.points.push(Some((coords, base)));
    }
    Ok(())
}

/// The data range of the tick algorithm: the data, and given limits when there are any.
pub(in crate::draw) fn finish_data_range(prep: &mut Prep) {
    let any_given = prep.user.iter().take(if prep.three_d { 3 } else { 2 }).any(|(a, b)| a.is_some() || b.is_some());
    let data_range = [0, 1, 2].map(|a| {
        let (mut lo, mut hi) = (prep.dmin[a], prep.dmax[a]);
        if any_given {
            if let Some(v) = prep.user[a].0 {
                lo = lo.min(v);
            }
            if let Some(v) = prep.user[a].1 {
                hi = hi.max(v);
            }
        }
        [lo, hi]
    });
    prep.data_range = data_range;
}

/// The second read of `xmin`/`xmax` (after the plots' late options have run).
pub(in crate::draw) fn read_stated_limits(e: &mut Engine, prep: &mut Prep) {
    for a in 0..3 {
        let c = axis_char(a);
        if let Some(v) = crate::symbolic::number(e, c, &format!("/pgfplots/{c}min")) {
            let v = logged(&prep.log, a, v);
            prep.min[a] = v;
            prep.auto_min[a] = false;
        }
        if let Some(v) = crate::symbolic::number(e, c, &format!("/pgfplots/{c}max")) {
            let v = logged(&prep.log, a, v);
            prep.max[a] = v;
            prep.auto_max[a] = false;
        }
    }
}

/// Angles 0 to 360 unless both are given; radii from 0 to the largest `|r|`.
pub(in crate::draw) fn polar_limits(prep: &mut Prep) {
    if prep.auto_min[0] || prep.auto_max[0] {
        (prep.min[0], prep.max[0]) = (0.0, 360.0);
    }
    if prep.auto_min[1] {
        prep.min[1] = 0.0;
    }
    if prep.auto_max[1] {
        prep.max[1] = prep.dmin[1].abs().max(prep.dmax[1].abs());
        if !prep.max[1].is_finite() || prep.max[1] == 0.0 {
            prep.max[1] = 1.0;
        }
    }
}

/// An empty window becomes 0 to 1; a reversed stated window is swapped back.
pub(in crate::draw) fn repair_limits(e: &mut Engine, prep: &mut Prep) {
    prep.invalid = (0..prep.dims).any(|a| !(prep.min[a].is_finite() && prep.max[a].is_finite()));
    if prep.invalid {
        e.warn("axis without data: drew the default window from 0 to 1");
        prep.min = [0.0, 0.0, 0.0];
        prep.max = [1.0, 1.0, 1.0];
        if prep.log[0] {
            prep.min[0] = 0.0;
        }
    }
    for a in 0..prep.dims {
        let (lo, hi) = (prep.min[a], prep.max[a]);
        if lo > hi && !(prep.auto_min[a] && prep.auto_max[a]) {
            prep.min[a] = hi;
            prep.max[a] = lo;
        }
    }
}

/// User limits over the data, then the polar window and the empty-window repair.
pub(in crate::draw) fn apply_stated_limits(e: &mut Engine, prep: &mut Prep) {
    prep.min = prep.dmin;
    prep.max = prep.dmax;
    prep.auto_min = [true; 3];
    prep.auto_max = [true; 3];
    read_stated_limits(e, prep);
    if prep.polar {
        polar_limits(prep);
    }
    repair_limits(e, prep);
}

/// One axis of `enlarge x limits`, including an empty range.
pub(in crate::draw) fn enlarge_one(e: &mut Engine, prep: &mut Prep, a: usize) {
    let c = axis_char(a);
    let v = pv(e, &format!("enlarge {c} limits"));
    // 'auto' enlarges only limits that were computed: x and y of a 2d axis, z of a 3d one
    let auto = if prep.three_d { a == 2 } else { a < 2 };
    let (lo, hi, rel, abs) = enlarge_setting(&v, prep.auto_min[a] && auto, prep.auto_max[a] && auto);
    let lo = lo && !prep.polar;
    let dim = v.trim_matches(|c| c == '{' || c == '}').split(',').find_map(|p| {
        let (k, val) = p.split_once('=')?;
        (matches!(k.trim(), "abs" | "abs value") && has_unit(val)).then(|| val.trim().to_string())
    });
    if let Some(d) = dim {
        if let Ok(len) = e.length(&d) {
            prep.abs_dim[a] = Some((len, lo, hi));
        }
        return;
    }
    // TeX multiplies the range (a dimension) by the factor read as a dimension: 0.1 is 6554 sp
    let rel = (rel * 65536.0).round() / 65536.0;
    // an absolute amount is a coordinate: logarithmic on a log axis
    let abs = abs.map(|v| if prep.log[a] { v.ln() } else { v });
    let amount = abs.unwrap_or(rel * (prep.max[a] - prep.min[a]));
    if amount > 1e-9 * (prep.max[a].abs() + 1.0) && !prep.invalid {
        if lo {
            prep.min[a] -= amount;
        }
        if hi {
            prep.max[a] += amount;
        }
    }
    // an empty range is widened
    let span = (prep.max[a] - prep.min[a]).abs();
    if span <= 1e-9 * prep.min[a].abs().max(1e-4) || span < 1e-12 {
        if prep.log[a] {
            prep.min[a] -= 1.0;
            prep.max[a] += 1.0;
        } else if prep.max[a] == 0.0 {
            prep.min[a] = -1.0;
            prep.max[a] = 1.0;
        } else if prep.max[a] > 0.0 {
            prep.min[a] *= 0.8;
            prep.max[a] *= 1.2;
        } else {
            prep.min[a] *= 1.2;
            prep.max[a] *= 0.8;
        }
    }
}

/// `enlargelimits` (a polar axis enlarges only the largest radius).
pub(in crate::draw) fn enlarge_limits(e: &mut Engine, prep: &mut Prep) {
    // `abs=1cm`: an amount on paper, applied once the axis is scaled
    prep.abs_dim = [None; 3];
    let axes = if prep.ternary || prep.smith {
        0..0
    } else if prep.polar {
        1..2
    } else {
        0..prep.dims
    };
    for a in axes {
        enlarge_one(e, prep, a);
    }
    if !prep.three_d {
        prep.min[2] = 0.0;
        prep.max[2] = 1.0;
    }
}

/// `width` and `height`, less 45pt unless `scale only axis`.
pub(in crate::draw) fn paper_size(e: &mut Engine) -> (f64, f64) {
    let default = (240.0 * PT, 207.0 * PT);
    let w = keys::length(e, "/pgfplots/width");
    let h = keys::length(e, "/pgfplots/height");
    let (mut width, mut height) = match (w, h) {
        (None, None) => default,
        (Some(w), None) => (w, w / (default.0 / default.1)),
        (None, Some(h)) => (h * (default.0 / default.1), h),
        (Some(w), Some(h)) => (w, h),
    };
    if !keys::flag(e, "/pgfplots/scale only axis") {
        width = (width - 45.0 * PT).max(0.0);
        height = (height - 45.0 * PT).max(0.0);
    }
    (width, height)
}

/// A unit vector the user wrote (`x={(1cm,1cm)}` or `x=1cm`).
pub(in crate::draw) fn user_unit(e: &mut Engine, c: char) -> Option<Point> {
    let s = keys::value(e, &format!("/pgfplots/{c}"));
    if s.is_empty() {
        return None;
    }
    let t = s.trim().trim_start_matches('{').trim_end_matches('}').trim().to_string();
    if let Some(inner) = t.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
        let parts = tikra_engine::keys::split_top_level(inner);
        let x = parts.first().and_then(|p| e.length(p).ok()).unwrap_or(0.0);
        let y = parts.get(1).and_then(|p| e.length(p).ok()).unwrap_or(0.0);
        return Some(Point::new(x, y));
    }
    let l = e.length(&t).ok()?;
    Some(match c {
        'x' => Point::new(l, 0.0),
        'y' => Point::new(0.0, l),
        _ => Point::new(0.0, l),
    })
}

/// The smith chart's diameter: `width` read again, square when only one size is given.
pub(in crate::draw) fn smith_vectors(e: &mut Engine, prep: &mut Prep) {
    let (w, h) = (keys::length(e, "/pgfplots/width"), keys::length(e, "/pgfplots/height"));
    let (mut pw, mut ph) = match (w, h) {
        (None, None) => (207.0 * PT, 207.0 * PT),
        (Some(w), None) => (w, w),
        (None, Some(h)) => (h, h),
        (Some(w), Some(h)) => (w, h),
    };
    if !keys::flag(e, "/pgfplots/scale only axis") {
        pw = (pw - 45.0 * PT).max(0.0);
        ph = (ph - 45.0 * PT).max(0.0);
    }
    let mirrored = if keys::flag(e, "/pgfplots/smithchart mirrored") { -1.0 } else { 1.0 };
    prep.ev[0] = Point::new(mirrored * pw / 2.0, 0.0);
    prep.ev[1] = Point::new(0.0, ph / 2.0);
}

/// The polar circle (or sector), stretched to width × height read again.
pub(in crate::draw) fn polar_vectors(e: &mut Engine, prep: &mut Prep) {
    let (w, h) = (keys::length(e, "/pgfplots/width"), keys::length(e, "/pgfplots/height"));
    let (mut pw, mut ph) = match (w, h) {
        (None, None) => (240.0 * PT, 240.0 * PT),
        (Some(w), None) => (w, w),
        (None, Some(h)) => (h, h),
        (Some(w), Some(h)) => (w, h),
    };
    if !keys::flag(e, "/pgfplots/scale only axis") {
        pw = (pw - 45.0 * PT).max(0.0);
        ph = (ph - 45.0 * PT).max(0.0);
    }
    // the whole circle's diameter, also for a sector (`…get@unit@scales@for@limits@polar`)
    let d = 2.0 * prep.max[1].abs();
    prep.ev[0] = Point::new(if d > 1e-12 { pw / d } else { 1.0 }, 0.0);
    prep.ev[1] = Point::new(0.0, if d > 1e-12 { ph / d } else { 1.0 });
}

/// A 3d view with no unit vectors of its own: the plot box, then one scale or a stretch.
pub(in crate::draw) fn view_vectors(e: &mut Engine, prep: &mut Prep) {
    let (az, el) = view_angles(e);
    let (saz, caz) = az.to_radians().sin_cos();
    let (sel, cel) = (-el).to_radians().sin_cos();
    let ratio: Vec<f64> = pv(e, "plot box ratio").replace(['{', '}'], " ").split_whitespace().filter_map(|x| x.parse().ok()).collect();
    let r = |k: usize| ratio.get(k).copied().unwrap_or(1.0);
    let dir = prep.dir;
    let base = [Point::new(caz, saz * sel).scale(r(0)), Point::new(saz, -sel * caz).scale(r(1)), Point::new(0.0, cel).scale(r(2))];
    // stretch to fill: scale x and y components so that the box's projection is W × H
    let mut corners = Vec::new();
    for i in 0..8 {
        let r = [(i & 1) as f64, ((i >> 1) & 1) as f64, ((i >> 2) & 1) as f64];
        let mut p = Point::default();
        for k in 0..3 {
            p = p.plus(base[k].scale(r[k] * if dir[k] { -1.0 } else { 1.0 }));
        }
        corners.push(p);
    }
    let (bw, bh) = bbox_size(&corners);
    let width = prep.width;
    let height = prep.height;
    let (sx, sy) = (if bw > 1e-9 { width / bw } else { 1.0 }, if bh > 1e-9 { height / bh } else { 1.0 });
    let ratio = pv(e, "unit vector ratio");
    let mode = pv(e, "scale mode");
    // `unit vector ratio` asks for one scale for all unit vectors
    let uniform = mode == "scale uniformly" || (mode == "auto" && !ratio.trim().is_empty());
    if uniform {
        let strategy = pv(e, "scale uniformly strategy");
        let ratio_axis = pv(e, "unit vector ratio axis");
        let keep = pv(e, "unit rescale keep size");
        let inp = crate::scaling::Input {
            e: [0, 1, 2].map(|k| base[k].scale(if dir[k] { -1.0 } else { 1.0 })),
            width,
            height,
            unit_box: true,
            three_d: true,
            strategy: crate::scaling::Strategy::parse(&strategy),
            ratio: &ratio,
            ratio_axis: &ratio_axis,
            keep: crate::scaling::KeepSize::parse(&keep),
            auto_min: prep.auto_min,
            auto_max: prep.auto_max,
        };
        let mut min = prep.min;
        let mut max = prep.max;
        let ev = crate::scaling::uniform(&inp, &mut min, &mut max);
        prep.min = min;
        prep.max = max;
        prep.ev = ev;
    }
    for k in (0..3).filter(|_| !uniform) {
        let range = (prep.max[k] - prep.min[k]).abs().max(1e-12);
        let s = if dir[k] { -1.0 } else { 1.0 };
        // 1/range as pgfplots multiplies it (compat before 1.3's pgfbasic scaling): a TeX
        // dimension, in sp, of the range in the data scale transformation's units
        let [lo, hi] = prep.data_range[k];
        let f = if prep.log[k] || keys::flag(e, "/pgfplots/@disabledatascaling") || !(lo.is_finite() && hi.is_finite()) {
            1.0
        } else {
            let size = keys::length(e, if k == 0 { "/pgfplots/width" } else { "/pgfplots/height" }).unwrap_or(if k == 0 { 240.0 * PT } else { 207.0 * PT }) / PT;
            ticks::data_scale(lo, hi, size)
        };
        let inverse = ((65536.0 / (range * f)).round() / 65536.0 * f).max(1e-300);
        let inverse = if inverse.is_finite() && inverse > 0.0 { inverse } else { 1.0 / range };
        prep.ev[k] = Point::new(base[k].x * sx * s * inverse, base[k].y * sy * s * inverse);
    }
}

/// Unit vectors the user gave, or width and height spread over the ranges.
pub(in crate::draw) struct Units {
    v: [Option<Point>; 3],
}

pub(in crate::draw) fn flat_vectors(e: &mut Engine, units: &Units, prep: &mut Prep) {
    let (width, height) = (prep.width, prep.height);
    let (min, max, dir) = (prep.min, prep.max, prep.dir);
    for a in 0..prep.dims {
        let range = (max[a] - min[a]).abs().max(1e-300);
        let s = if dir[a] { -1.0 } else { 1.0 };
        prep.ev[a] = match units.v[a] {
            Some(u) => u.scale(s),
            None => {
                if a == 0 {
                    Point::new(s * width / range, 0.0)
                } else {
                    Point::new(0.0, s * height / range)
                }
            }
        };
    }
    // a 3d axis with given unit vectors, scaled uniformly to width × height
    let ratio = pv(e, "unit vector ratio");
    let mode = pv(e, "scale mode");
    if prep.three_d && units.v.iter().all(Option::is_some) && (mode == "scale uniformly" || (mode == "auto" && !ratio.trim().is_empty())) {
        let strategy = pv(e, "scale uniformly strategy");
        let ratio_axis = pv(e, "unit vector ratio axis");
        let keep = pv(e, "unit rescale keep size");
        let width = prep.width;
        let height = prep.height;
        let ev = prep.ev;
        let auto_min = prep.auto_min;
        let auto_max = prep.auto_max;
        let inp = crate::scaling::Input {
            e: ev,
            width,
            height,
            unit_box: false,
            three_d: true,
            strategy: crate::scaling::Strategy::parse(&strategy),
            ratio: &ratio,
            ratio_axis: &ratio_axis,
            keep: crate::scaling::KeepSize::parse(&keep),
            auto_min,
            auto_max,
        };
        let mut min = prep.min;
        let mut max = prep.max;
        let ev = crate::scaling::uniform(&inp, &mut min, &mut max);
        prep.min = min;
        prep.max = max;
        prep.ev = ev;
    }
    // unit vector ratio (axis equal): one scale for both, the limits enlarged to keep the size
    if !ratio.is_empty() && !prep.three_d && units.v[0].is_none() && units.v[1].is_none() {
        let r: Vec<f64> = ratio.split_whitespace().filter_map(|x| x.parse().ok()).collect();
        let (rx, ry) = (r.first().copied().unwrap_or(1.0), r.get(1).copied().unwrap_or(1.0));
        let keep = pv(e, "unit rescale keep size");
        let ux = prep.ev[0].x.abs() / rx;
        let uy = prep.ev[1].y.abs() / ry;
        let u = ux.min(uy);
        let declared = !prep.auto_min[0] && !prep.auto_max[0] && !prep.auto_min[1] && !prep.auto_max[1];
        let keep_size = keep == "true" || (keep == "unless limits declared" && !declared);
        for (a, k) in [(0usize, rx), (1usize, ry)] {
            let target = u * k;
            let current = if a == 0 { prep.ev[0].x.abs() } else { prep.ev[1].y.abs() };
            if keep_size && current > target + 1e-12 {
                // enlarge the range around its center
                let range = prep.max[a] - prep.min[a];
                let new = range * current / target;
                let c = (prep.max[a] + prep.min[a]) / 2.0;
                prep.min[a] = c - new / 2.0;
                prep.max[a] = c + new / 2.0;
            }
            let s = if prep.dir[a] { -1.0 } else { 1.0 };
            if a == 0 {
                prep.ev[0] = Point::new(s * target, 0.0);
            } else {
                prep.ev[1] = Point::new(0.0, s * target);
            }
        }
    }
}

/// `enlarge x limits={abs=1cm}`: the axis keeps its length, the data leave that room.
pub(in crate::draw) fn apply_abs_room(prep: &mut Prep) {
    for a in 0..2 {
        let Some((d, lo, hi)) = prep.abs_dim[a] else { continue };
        let range = (prep.max[a] - prep.min[a]).abs();
        let unit = prep.ev[a].norm();
        let length = unit * range;
        let room = d * (f64::from(u8::from(lo)) + f64::from(u8::from(hi)));
        if range > 0.0 && length > room {
            let u2 = (length - room) / range;
            if lo {
                prep.min[a] -= d / u2;
            }
            if hi {
                prep.max[a] += d / u2;
            }
            let scaled = prep.ev[a].scale(u2 / unit);
            prep.ev[a] = scaled;
        }
    }
}

/// `x post scale` and its siblings, after the unit vectors exist.
pub(in crate::draw) fn apply_post_scale(e: &mut Engine, prep: &mut Prep) {
    for (a, v) in prep.ev.iter_mut().enumerate() {
        if let Some(f) = keys::number(e, &format!("/pgfplots/{} post scale", axis_char(a))) {
            *v = v.scale(f);
        }
    }
}

/// Paper size, unit vectors, absolute enlargement and post scale.
pub(in crate::draw) fn scale_to_paper(e: &mut Engine, prep: &mut Prep) {
    let (width, height) = paper_size(e);
    prep.width = width;
    prep.height = height;
    prep.dir = [pv(e, "x dir") == "reverse", pv(e, "y dir") == "reverse", pv(e, "z dir") == "reverse"];
    let units = Units { v: [user_unit(e, 'x'), user_unit(e, 'y'), user_unit(e, 'z')] };
    if prep.smith {
        smith_vectors(e, prep);
    } else if prep.ternary {
        // a triangle of side `width` (less the labels' room)
        let v = crate::ternary::vectors(prep.width);
        let (min, max) = (prep.min, prep.max);
        for a in 0..3 {
            prep.ev[a] = v[a].scale(1.0 / (max[a] - min[a]).abs().max(1e-300));
        }
    } else if prep.polar {
        polar_vectors(e, prep);
    } else if prep.three_d && units.v.iter().all(Option::is_none) {
        view_vectors(e, prep);
    } else {
        flat_vectors(e, &units, prep);
    }
    if !prep.three_d {
        apply_abs_room(prep);
    }
    apply_post_scale(e, prep);
}

/// The frame, its origin, and the ticks that fall inside the window.
pub(in crate::draw) fn assemble_setup(e: &mut Engine, axis: &Axis, prep: Prep) -> Result<Setup, Error> {
    let Prep { ternary, smith, three_d, polar, log, points, data_range, min, max, dims, dir, ev, .. } = prep;
    // the origin: the box's lower left corner at the picture's origin
    let origin_local = Point::default();
    let mut zero = [0.0; 3];
    for a in 0..3 {
        zero[a] = if log[a] { 0.0 } else { 0.0f64.clamp(min[a].min(max[a]), max[a].max(min[a])) };
    }
    let lines = [pv(e, "@x line"), pv(e, "@y line"), pv(e, "@z line")];
    let mut label_side = [-1i8; 3];
    for a in 0..3 {
        let c = axis_char(a);
        let pos = pv(e, &format!("{c}ticklabel pos"));
        let tickpos = pv(e, &format!("{c}tick pos"));
        label_side[a] = match lines[a].as_str() {
            "center" => 0,
            "high" => 1,
            "low" => -1,
            _ => match pos.as_str() {
                "right" | "top" | "upper" => 1,
                "left" | "bottom" | "lower" => -1,
                _ => {
                    if tickpos == "right" {
                        1
                    } else {
                        -1
                    }
                }
            },
        };
    }
    let label_shift = [0, 1, 2].map(|a| keys::length(e, &format!("/pgfplots/{}label shift", axis_char(a))).unwrap_or(0.0));
    // the box with its lower left corner at the origin of the picture's coordinates
    let base = e.st.transform.apply(origin_local);
    let mut frame = Frame {
        origin: base,
        e: ev,
        min,
        max,
        log,
        three_d,
        zero,
        cs: Cell::new(Cs::Plain),
        label_axis: Cell::new(0),
        tick_dim: RefCell::new([0.0; 3]),
        label_side,
        label_shift,
        meta: meta_range(e, axis),
        colormap: colormap(e),
        fg: [[false, true]; 3],
        label_line: [*b"v00", *b"0v0", *b"00v"],
        box_line: [*b"v00", *b"0v0", *b"00v"],
        data_range,
        polar,
        ternary,
        ternary_relative: ternary && keys::flag(e, "/pgfplots/ternary limits relative"),
        smith,
        slope: Cell::new(None),
        upside_down: RefCell::new(None),
        reversed: dir,
    };
    if smith {
        // the chart's lower left corner at the origin
        frame.origin = base.plus(Point::new(frame.e[0].norm(), frame.e[1].norm()));
    } else if ternary {
        // the lower left corner is at the origin already
    } else if three_d {
        crate::cube::prepare(e, &mut frame, dir, &lines);
    } else {
        // `lower` is lower on paper (left for y): with `y dir=reverse` the x tick labels go to
        // ymax (`\pgfplots@init@ticklabelaxisspec@twodim@for`)
        for a in 0..2 {
            let lower_is_min = if a == 0 { frame.e[1].y >= 0.0 } else { frame.e[0].x >= 0.0 };
            if !lower_is_min && frame.label_side[a] != 0 {
                frame.label_side[a] = -frame.label_side[a];
            }
        }
    }
    // the box's lowest corner goes to the origin
    if polar {
        let (lo, _) = frame.bbox();
        frame.origin = frame.origin.plus(base.minus(lo));
    } else if !three_d {
        let a = frame.at(frame.min);
        let b = frame.at(frame.max);
        let lo = Point::new(a.x.min(b.x), a.y.min(b.y));
        frame.origin = frame.origin.plus(base.minus(lo));
    }
    let hide = hidden_axes(e, &frame, dims);
    let mut ticks: [Vec<f64>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    let mut dist = [0.0; 3];
    for a in 0..dims {
        let (t, d) = clipped_ticks(e, &TickWindow { frame: &frame, axis: a, plot: axis })?;
        ticks[a] = t;
        dist[a] = d;
    }
    Ok(Setup { frame: Rc::new(frame), points, ticks, tick_distance: dist, hide, line: lines })
}
