//! `mesh`, `surf` and `patch` plots (`pgfplotsmeshplothandler.code.tex`): a lattice or a list of
//! patches, each colored by the colormap at its point meta, drawn back to front in 3d
//! (`z buffer`), with the shaders `faceted` (fill and `faceted color` edges), `flat` (one color,
//! edges too), `flat corner` and `interp` (the point meta shaded across each triangle).
//!
//! Colors given with `mapped color` (`draw=mapped color!80!black`) are linear in it: each is found
//! by setting the style with `mapped color` black, then white.

use tikra_engine::canvas::{Fill, Item, Paint, Stroke};
use tikra_engine::color::Color;
use tikra_engine::scene::{Point, Segment};
use tikra_engine::tex::Token;
use tikra_engine::Error;

use super::draw::Frame;
use super::keys;
use super::plot::Plot;
use super::Engine;

mod color;
mod image;
mod surface;

pub use color::{explicit_color, map, mapped_colors, Linear};
pub use image::{image_corners, image_interp_metas, image_limits};

/// A patch: its corners (canvas points) and the points its colors come from.
type PatchSource = (Vec<usize>, Vec<usize>);

/// The patches of a plot: corner indices.
fn patches(p: &Plot, n: usize, kind: &str) -> Vec<Vec<usize>> {
    if let Some((rows, cols)) = p.lattice {
        let mut v = Vec::new();
        // `patch type=line`: the segments along the scanlines only
        if kind == "line" {
            for r in 0..rows {
                for c in 0..cols.saturating_sub(1) {
                    v.push(vec![r * cols + c, r * cols + c + 1]);
                }
            }
            return v;
        }
        for r in 0..rows.saturating_sub(1) {
            for c in 0..cols.saturating_sub(1) {
                let k = r * cols + c;
                v.push(vec![k, k + 1, k + cols + 1, k + cols]);
            }
        }
        return v;
    }
    if p.patch {
        let (size, corners): (usize, usize) = match kind {
            "rectangle" | "bilinear" => (4, 4),
            "biquadratic" => (9, 4),
            "quadratic triangle" => (6, 3),
            "triangle quadr" => (6, 3),
            "cubic spline" => (4, 2),
            "line" => (2, 2),
            "bicubic" => (16, 4),
            "polygon" => (4, 4),
            _ => (3, 3),
        };
        return (0..n / size).map(|i| (0..corners).map(|k| i * size + k).collect()).collect();
    }
    // a line of points: segments
    (0..n.saturating_sub(1)).map(|i| vec![i, i + 1]).collect()
}

/// A mesh, surf or patch plot to draw, in the plot's group, its style already the caller's.
#[derive(Clone, Copy)]
pub struct MeshDraw<'a> {
    pub plot: &'a Plot,
    pub pts: &'a [[f64; 3]],
    pub frame: &'a Frame,
}

/// `patch type` and the colors a patch plot is drawn with.
struct PatchPaint<'a> {
    kind: &'a str,
    look: &'a Look,
}

struct Tuned {
    look: Look,
    kind: String,
    zbuffer: String,
    explicit: bool,
    opacity: (f64, f64),
    width: f64,
}

struct MeshMaps {
    outer: Option<super::colormap::Colormap>,
    inner: Option<super::colormap::Colormap>,
    range: [f64; 2],
    interior: Interior,
}

struct CellOrder<'a> {
    pts: &'a [[f64; 3]],
    frame: &'a Frame,
    cols: Option<usize>,
    zbuffer: &'a str,
    reversed: (bool, bool),
}

struct CellCtx<'a> {
    plot: &'a Plot,
    pts: &'a [[f64; 3]],
    frame: &'a Frame,
    look: &'a Look,
    kind: &'a str,
    explicit: bool,
    opacity: (f64, f64),
    width: f64,
    reversed: (bool, bool),
    maps: &'a MeshMaps,
    image_metas: Option<&'a Vec<[f64; 4]>>,
    closes: bool,
}

struct CellIn<'a> {
    q: Vec<usize>,
    src: Vec<usize>,
    canvas: &'a [Option<Point>],
}

struct CellColor {
    corners: Vec<Point>,
    metas: Vec<f64>,
    vertex: Option<Vec<Color>>,
    mapped: Color,
    use_inner: bool,
    interp_ok: bool,
}

/// Draws a mesh, surf or patch plot (in the plot's group, its style set).
pub fn draw(e: &mut Engine, d: &MeshDraw<'_>, style: &[Token]) -> Result<(), Error> {
    let tuned = tune_mesh(e, d, style)?;
    if d.plot.patch && d.plot.lattice.is_none() && !tuned.explicit {
        return surface::draw_patches(e, d, &PatchPaint { kind: &tuned.kind, look: &tuned.look });
    }
    let (canvas, mut list) = cell_sources(d, &tuned.kind);
    let reversed = view_reversal(e, d.frame);
    let cols = d.plot.lattice.filter(|_| !d.plot.image).map(|(_, c)| c);
    sort_cells(e, &mut list, &CellOrder { pts: d.pts, frame: d.frame, cols, zbuffer: &tuned.zbuffer, reversed });
    let closes = list.first().is_some_and(|(q, _)| q.len() > 2);
    let maps = mesh_maps(e, d);
    // the shadings of `interp` are drawn as PDF viewers draw them, without antialiasing: no seams
    // between patches
    let crisp = closes && (tuned.look.surf || d.plot.patch) && (tuned.look.shader == "interp" || tuned.look.shader == "faceted interp");
    if crisp {
        e.canvas.push(Item::Crisp);
    }
    // an image's cells are shaded between metas found for their corners
    let image_metas = d
        .plot
        .lattice
        .filter(|_| d.plot.image && crisp && !tuned.explicit)
        .map(|(rows, cols)| image_interp_metas(&d.plot.coords.iter().map(|c| c.meta).collect::<Vec<_>>(), rows, cols, maps.range[0]));
    let ctx = CellCtx {
        plot: d.plot,
        pts: d.pts,
        frame: d.frame,
        look: &tuned.look,
        kind: &tuned.kind,
        explicit: tuned.explicit,
        opacity: tuned.opacity,
        width: tuned.width,
        reversed,
        maps: &maps,
        image_metas: image_metas.as_ref(),
        closes,
    };
    for (q, src) in list {
        if let Some(cell) = prepare_cell(e, &ctx, CellIn { q, src, canvas: &canvas }) {
            paint_prepared(e, &ctx, &cell);
        }
    }
    if crisp {
        e.canvas.push(Item::End);
    }
    Ok(())
}

fn tune_mesh(e: &mut Engine, d: &MeshDraw<'_>, style: &[Token]) -> Result<Tuned, Error> {
    let (draw, fill, st) = mapped_colors(e, |e| {
        e.st.mode = Default::default();
        e.process_keys(style.to_vec(), Some("/pgfplots"))
    })?;
    let surf = d.plot.mesh == Some('s');
    let shader = keys::value(e, "/pgfplots/shader");
    let faceted_name = keys::value(e, "/pgfplots/faceted color");
    let faceted = color::mapped_named(e, &faceted_name).unwrap_or(draw);
    let kind = keys::value(e, "/pgfplots/patch type");
    let zbuffer = keys::value(e, "/pgfplots/z buffer");
    let width = st.pen.line_width;
    let opacity = (st.paint.draw_opacity, st.paint.fill_opacity);
    let explicit = keys::value(e, "/pgfplots/mesh/color input").starts_with("explicit");
    let look = Look { draw, fill, faceted, shader, stroke: st, surf };
    Ok(Tuned { look, kind, zbuffer, explicit, opacity, width })
}

fn cell_sources(d: &MeshDraw<'_>, kind: &str) -> (Vec<Option<Point>>, Vec<PatchSource>) {
    let f = d.frame;
    let p = d.plot;
    let pts = d.pts;
    let to_canvas = |l: &[f64; 3]| if l.iter().take(if f.three_d { 3 } else { 2 }).all(|v| v.is_finite()) { Some(f.at(*l)) } else { None };
    // each patch: its corners (into `canvas`) and the points its colors come from
    match p.lattice.filter(|_| p.image) {
        Some((rows, cols)) => {
            let corners = image_corners(pts, rows, cols);
            let mut v = Vec::new();
            for r in 0..rows {
                for c in 0..cols {
                    let k = r * (cols + 1) + c;
                    v.push((vec![k, k + 1, k + cols + 2, k + cols + 1], vec![r * cols + c; 4]));
                }
            }
            (corners.iter().map(to_canvas).collect(), v)
        }
        None => (pts.iter().map(to_canvas).collect(), patches(p, pts.len(), kind).into_iter().map(|q| (q.clone(), q)).collect()),
    }
}

/// `z buffer=auto` reverses the columns when the face at xmin is in front, the rows when the one at ymin is.
fn view_reversal(e: &mut Engine, f: &Frame) -> (bool, bool) {
    let az = keys::number(e, "/pgfplots/view/az").unwrap_or(25.0).to_radians();
    let el = keys::number(e, "/pgfplots/view/el").unwrap_or(30.0).to_radians();
    if f.three_d {
        (az.sin() * el.cos() < -1e-9, -az.cos() * el.cos() < -1e-9)
    } else {
        (false, false)
    }
}

fn sort_cells(e: &mut Engine, list: &mut [PatchSource], order: &CellOrder<'_>) {
    if !order.frame.three_d || order.zbuffer == "none" {
        return;
    }
    // a matrix is drawn in its order, the rows or columns reversed (`z buffer=auto`)
    if let Some(cols) = order.cols.filter(|c| *c > 1) {
        if matches!(order.zbuffer, "default" | "auto") {
            let reversed = order.reversed;
            let key = |q: &[usize]| {
                let (r, c) = (q[0] / cols, q[0] % cols);
                (if reversed.1 { usize::MAX - r } else { r }, if reversed.0 { usize::MAX - c } else { c })
            };
            list.sort_by_key(|(q, _)| key(q));
            return;
        }
    }
    let depth = Depth::new(e, order.frame);
    let of = |q: &[usize]| depth.of(&q.iter().map(|&k| order.pts[k]).collect::<Vec<_>>());
    list.sort_by(|(a, _), (b, _)| of(a).total_cmp(&of(b)));
}

fn mesh_maps(e: &mut Engine, d: &MeshDraw<'_>) -> MeshMaps {
    // the plot's colormap (`colormap/blackwhite` among its options), and that of the patches
    // seen from behind (`mesh/interior colormap`)
    let name = keys::value(e, "/pgfplots/colormap name");
    let outer = super::draw::colormap_named(e, &name).or_else(|| d.frame.colormap.clone());
    let inner_name = keys::value(e, "/pgfplots/mesh/interior colormap name");
    let inner = if inner_name.is_empty() || !d.frame.three_d { None } else { super::draw::colormap_named(e, &inner_name) };
    let thresh = keys::number(e, "/pgfplots/mesh/interior colormap thresh").unwrap_or(0.0);
    let interior = Interior::new(e, d.frame, thresh);
    // `point meta rel=per plot`: the colormap spans this plot's point meta only
    let range = if keys::value(e, "/pgfplots/point meta rel") == "per plot" { plot_meta_range(d.plot).unwrap_or(d.frame.meta) } else { d.frame.meta };
    MeshMaps { outer, inner, range, interior }
}

impl CellCtx<'_> {
    /// The normal at the corner the z buffer made first (`compute normal`), its sign kept.
    fn inside(&self, q: &[usize]) -> bool {
        let c: Vec<[f64; 3]> = q.iter().map(|&k| self.pts[k]).collect();
        let face = match (c.len(), self.reversed) {
            (4, (false, true)) => Face { a: c[3], b: c[2], d: c[0], sign: -1.0 },
            (4, (true, false)) => Face { a: c[1], b: c[0], d: c[2], sign: -1.0 },
            (4, (true, true)) => Face { a: c[2], b: c[3], d: c[1], sign: 1.0 },
            _ => Face { a: c[0], b: c[1], d: c[c.len() - 1], sign: 1.0 },
        };
        self.maps.interior.shows(face)
    }
}

fn cmap_of<'a>(ctx: &'a CellCtx<'_>, inner: bool) -> Option<&'a super::colormap::Colormap> {
    if inner {
        ctx.maps.inner.as_ref()
    } else {
        ctx.maps.outer.as_ref()
    }
}

fn shade_meta(ctx: &CellCtx<'_>, inner: bool, m: f64) -> Color {
    let range = ctx.maps.range;
    cmap_of(ctx, inner).map_or(Color::BLACK, |c| c.color_of(m, range[0], range[1]))
}

fn shade_at(ctx: &CellCtx<'_>, cell: &CellColor, m: f64) -> Color {
    shade_meta(ctx, cell.use_inner, m)
}

fn prepare_cell(e: &mut Engine, ctx: &CellCtx<'_>, input: CellIn<'_>) -> Option<CellColor> {
    let CellIn { q, src, canvas } = input;
    let corners = q.iter().map(|&k| canvas.get(k).copied().flatten()).collect::<Option<Vec<_>>>()?;
    let use_inner = ctx.maps.inner.is_some() && q.len() > 2 && ctx.inside(&q);
    let mut metas: Vec<f64> = src.iter().map(|&k| ctx.plot.coords.get(k).map_or(f64::NAN, |c| c.meta)).filter(|m| m.is_finite()).collect();
    // a patch whose color comes from a point meta of nan is not drawn (with `flat corner`, the first corner's)
    let meta_of = |k: usize| ctx.plot.coords.get(k).map_or(f64::NAN, |c| c.meta);
    let unbounded = if ctx.look.shader == "flat corner" { src.first().is_some_and(|&k| !meta_of(k).is_finite()) } else { metas.len() < src.len() };
    if ctx.plot.has_meta && !ctx.explicit && unbounded {
        return None;
    }
    if let Some(im) = ctx.image_metas {
        if let Some(c) = im.get(src[0]) {
            metas = c.to_vec();
        }
    }
    let m = if ctx.look.shader == "flat corner" { metas.first().copied().unwrap_or(f64::NAN) } else { metas.iter().sum::<f64>() / metas.len().max(1) as f64 };
    // `mesh/color input=explicit`: each point has its color. A point without a color (`[]`) is unbounded, as a nan.
    let vertex = match ctx.explicit.then(|| src.iter().map(|&k| color::vertex_color(e, ctx.plot, k)).collect::<Option<Vec<Color>>>()) {
        Some(None) => return None,
        v => v.flatten(),
    };
    let mapped = match &vertex {
        Some(v) if ctx.look.shader == "flat corner" => v[0],
        Some(v) => color::mean(v),
        None if m.is_finite() => shade_meta(ctx, use_inner, m),
        None => Color::BLACK,
    };
    let interp_ok = vertex.is_some() || metas.len() == corners.len();
    Some(CellColor { corners, metas, vertex, mapped, use_inner, interp_ok })
}

fn paint_prepared(e: &mut Engine, ctx: &CellCtx<'_>, cell: &CellColor) {
    if paint_axial(e, ctx, cell) || paint_bands(e, ctx, cell) || paint_gouraud(e, ctx, cell) || paint_refined(e, ctx, cell) {
        return;
    }
    paint_solid(e, ctx, cell);
}

/// A parallelogram whose color changes along one side only: an axial shading, exact, without pieces.
fn paint_axial(e: &mut Engine, ctx: &CellCtx<'_>, cell: &CellColor) -> bool {
    if !(ctx.closes && ctx.look.shader == "interp" && cell.vertex.is_none() && cell.corners.len() == 4 && cell.metas.len() == 4) {
        return false;
    }
    let Some(cm) = cmap_of(ctx, cell.use_inner).filter(|c| c.access == super::colormap::Access::Map) else { return false };
    let corners = &cell.corners;
    let metas = &cell.metas;
    let same = |a: f64, b: f64| (a - b).abs() <= 1e-12 * (a.abs() + b.abs() + 1.0);
    let pick = if same(metas[0], metas[1]) && same(metas[3], metas[2]) {
        Some((corners[0], corners[1], corners[3], corners[2], metas[0], metas[3]))
    } else if same(metas[0], metas[3]) && same(metas[1], metas[2]) {
        Some((corners[0], corners[3], corners[1], corners[2], metas[0], metas[1]))
    } else {
        None
    };
    let Some((a, b, d, c, ma, mb)) = pick else { return false };
    let flat = c.minus(b.plus(d).minus(a)).norm() < 1e-6 * (b.minus(a).norm() + d.minus(a).norm() + 1e-12);
    if !flat || same(ma, mb) {
        return false;
    }
    let (u, v) = (b.minus(a), d.minus(a));
    let transform = tikra_engine::scene::Transform { a: u.x / 100.0, b: u.y / 100.0, c: v.x / 100.0, d: v.y / 100.0, e: a.x + 0.5 * (u.x + v.x), f: a.y + 0.5 * (u.y + v.y) };
    let (lo_m, hi_m) = (ctx.frame.meta[0], ctx.frame.meta[1]);
    let mut metas_at: Vec<f64> = vec![ma, mb];
    for (p, _) in cm.normalized() {
        let m = lo_m + p * (hi_m - lo_m);
        if (m - ma) * (m - mb) < 0.0 {
            metas_at.push(m);
        }
    }
    let mut stops: Vec<tikra_engine::canvas::Stop> = metas_at.iter().map(|m| ((m - ma) / (mb - ma), ctx.look.fill.at(shade_at(ctx, cell, *m)))).collect();
    stops.sort_by(|x, y| x.0.total_cmp(&y.0));
    let segs = vec![Segment::MoveTo(a), Segment::LineTo(b), Segment::LineTo(c), Segment::LineTo(d), Segment::Close];
    if !e.canvas_frozen {
        e.canvas.include_path(&segs);
    }
    let paint = Paint::Shading(tikra_engine::canvas::Shading::Axial { transform, stops });
    e.canvas.push(Item::Path { segments: segs, fill: Some(Fill { paint, opacity: ctx.opacity.1, even_odd: false }), stroke: None });
    true
}

/// `colormap access=piecewise constant`: the shading's colors are bands, cut exactly.
fn paint_bands(e: &mut Engine, ctx: &CellCtx<'_>, cell: &CellColor) -> bool {
    let Some(cm) = cmap_of(ctx, cell.use_inner)
        .filter(|c| c.access == super::colormap::Access::Const && ctx.closes && ctx.look.shader == "interp" && cell.vertex.is_none() && cell.metas.len() == cell.corners.len())
    else {
        return false;
    };
    let corners = &cell.corners;
    let metas = &cell.metas;
    let tris: Vec<[usize; 3]> = if corners.len() == 4 {
        match ctx.reversed {
            (false, false) | (true, true) => vec![[0, 1, 3], [1, 2, 3]],
            _ => vec![[0, 1, 2], [0, 2, 3]],
        }
    } else {
        vec![[0, 1, 2]]
    };
    let n = cm.stops.len();
    let range = ctx.maps.range;
    let width = (range[1] - range[0]) / n as f64;
    let st = &ctx.look.stroke;
    for t in tris {
        let tri: Vec<(Point, f64)> = t.iter().map(|&k| (corners[k], metas[k])).collect();
        for (b, (_, c)) in cm.stops.iter().enumerate() {
            let lo = if b == 0 { f64::NEG_INFINITY } else { range[0] + b as f64 * width };
            let hi = if b + 1 == n { f64::INFINITY } else { range[0] + (b + 1) as f64 * width };
            let poly = super::contour::clip(&tri, lo, hi);
            if poly.len() < 3 {
                continue;
            }
            let c = ctx.look.fill.at(*c);
            let mut segs = vec![Segment::MoveTo(poly[0])];
            segs.extend(poly[1..].iter().map(|q| Segment::LineTo(*q)));
            segs.push(Segment::Close);
            if !e.canvas_frozen {
                e.canvas.include_path(&segs);
            }
            let stroke =
                Stroke { color: c, width: 0.3 * tikra_engine::PT, dash: None, cap: st.pen.cap, join: st.pen.join, miter_limit: st.pen.miter_limit, opacity: ctx.opacity.0 };
            e.canvas.push(Item::Path { segments: segs, fill: Some(Fill { paint: Paint::Solid(c), opacity: ctx.opacity.1, even_odd: false }), stroke: Some(stroke) });
        }
    }
    true
}

/// A triangle, or a matrix cell, as a Gouraud shading (type 5): two triangles as the z buffer turned them.
fn paint_gouraud(e: &mut Engine, ctx: &CellCtx<'_>, cell: &CellColor) -> bool {
    let shaded = ctx.look.shader == "interp" || ctx.look.shader == "faceted interp";
    let Some(cm) = cmap_of(ctx, cell.use_inner)
        .filter(|c| c.access == super::colormap::Access::Map && ctx.closes && shaded && cell.vertex.is_none() && cell.metas.len() == cell.corners.len())
    else {
        return false;
    };
    let corners = &cell.corners;
    let metas = &cell.metas;
    let lattice_cell = corners.len() == 4 && ctx.plot.lattice.is_some() && !ctx.plot.image && ctx.kind != "bilinear";
    let tris: Vec<[usize; 3]> = match (corners.len(), ctx.reversed) {
        (3, _) => vec![[0, 1, 2]],
        (4, (false, false)) if lattice_cell => vec![[0, 1, 3], [1, 2, 3]],
        (4, (false, true)) if lattice_cell => vec![[3, 2, 0], [2, 1, 0]],
        (4, (true, false)) if lattice_cell => vec![[1, 0, 2], [0, 3, 2]],
        (4, (true, true)) if lattice_cell => vec![[2, 3, 1], [3, 0, 1]],
        // a patch of an image: ABD, then BCD (streamed to the shader as a lattice of two columns)
        (4, _) if ctx.image_metas.is_some() => vec![[0, 1, 3], [1, 2, 3]],
        _ => vec![],
    };
    for t in &tris {
        let paint = color::gouraud(&t.map(|k| (corners[k], metas[k])), cm, ctx.maps.range, &ctx.look.fill);
        let segs = vec![Segment::MoveTo(corners[t[0]]), Segment::LineTo(corners[t[1]]), Segment::LineTo(corners[t[2]]), Segment::Close];
        if !e.canvas_frozen {
            e.canvas.include_path(&segs);
        }
        e.canvas.push(Item::Path { segments: segs, fill: Some(Fill { paint, opacity: ctx.opacity.1, even_odd: false }), stroke: None });
    }
    if tris.is_empty() {
        return false;
    }
    // `faceted interp`: the patch's edges on top, in `faceted color` (antialiased, as lines are)
    if ctx.look.shader == "faceted interp" {
        let mut segs = vec![Segment::MoveTo(corners[0])];
        segs.extend(corners[1..].iter().map(|c| Segment::LineTo(*c)));
        segs.push(Segment::Close);
        let st = &ctx.look.stroke;
        let stroke = Stroke {
            color: ctx.look.faceted.at(cell.mapped),
            width: ctx.width,
            dash: st.pen.dash.clone().filter(|d| !d.0.is_empty()),
            cap: st.pen.cap,
            join: st.pen.join,
            miter_limit: st.pen.miter_limit,
            opacity: ctx.opacity.0,
        };
        e.canvas.push(Item::End);
        e.canvas.push(Item::Path { segments: segs, fill: None, stroke: Some(stroke) });
        e.canvas.push(Item::Crisp);
    }
    true
}

/// `interp` of a large patch: small pieces interpolated between its corners.
fn paint_refined(e: &mut Engine, ctx: &CellCtx<'_>, cell: &CellColor) -> bool {
    let corners = &cell.corners;
    if !(ctx.closes && ctx.look.shader == "interp" && cell.interp_ok && (corners.len() == 4 || corners.len() == 3)) {
        return false;
    }
    let size = (0..corners.len()).map(|i| corners[i].minus(corners[(i + 1) % corners.len()]).norm()).fold(0.0, f64::max);
    let n = if size > 12.0 * tikra_engine::PT { ((size / (3.0 * tikra_engine::PT)).ceil() as usize).clamp(1, 64) } else { 1 };
    if n <= 1 {
        return false;
    }
    let metas = &cell.metas;
    let vertex = &cell.vertex;
    let at = |w: &[f64]| -> (Point, Color) {
        let mut pt = Point::default();
        for k in 0..corners.len() {
            pt = pt.plus(corners[k].scale(w[k]));
        }
        let c = match vertex {
            Some(v) => color::blend(v, w),
            None => shade_at(ctx, cell, (0..corners.len()).map(|k| metas[k] * w[k]).sum()),
        };
        (pt, c)
    };
    let mut pieces: Vec<Vec<(Point, Color)>> = Vec::new();
    let nf = n as f64;
    if corners.len() == 4 && ctx.plot.lattice.is_some() && !ctx.plot.image && ctx.kind != "bilinear" {
        // a cell of a matrix is shaded as two triangles (a shading of type 5), ABD then BCD of its
        // corners as the z buffer has turned them (reversing the rows or the columns)
        let tris: [[usize; 3]; 2] = match ctx.reversed {
            (false, false) => [[0, 1, 3], [1, 2, 3]],
            (false, true) => [[3, 2, 0], [2, 1, 0]],
            (true, false) => [[1, 0, 2], [0, 3, 2]],
            (true, true) => [[2, 3, 1], [3, 0, 1]],
        };
        for t in tris {
            let bar = |i: usize, j: usize| {
                let (a, b) = (i as f64 / nf, j as f64 / nf);
                let mut w = [0.0; 4];
                w[t[0]] = 1.0 - a - b;
                w[t[1]] = a;
                w[t[2]] = b;
                w
            };
            for i in 0..n {
                for j in 0..n - i {
                    pieces.push([bar(i, j), bar(i + 1, j), bar(i, j + 1)].iter().map(|w| at(w)).collect());
                    if i + j + 1 < n {
                        pieces.push([bar(i + 1, j), bar(i + 1, j + 1), bar(i, j + 1)].iter().map(|w| at(w)).collect());
                    }
                }
            }
        }
    } else if corners.len() == 4 {
        let bil = |u: f64, v: f64| [(1.0 - u) * (1.0 - v), u * (1.0 - v), u * v, (1.0 - u) * v];
        for i in 0..n {
            for j in 0..n {
                let (u0, u1, v0, v1) = (i as f64 / nf, (i + 1) as f64 / nf, j as f64 / nf, (j + 1) as f64 / nf);
                pieces.push([bil(u0, v0), bil(u1, v0), bil(u1, v1), bil(u0, v1)].iter().map(|w| at(w)).collect());
            }
        }
    } else {
        let bar = |i: usize, j: usize| {
            let (a, b) = (i as f64 / nf, j as f64 / nf);
            [1.0 - a - b, a, b]
        };
        for i in 0..n {
            for j in 0..n - i {
                pieces.push([bar(i, j), bar(i + 1, j), bar(i, j + 1)].iter().map(|w| at(w)).collect());
                if i + j + 1 < n {
                    pieces.push([bar(i + 1, j), bar(i + 1, j + 1), bar(i, j + 1)].iter().map(|w| at(w)).collect());
                }
            }
        }
    }
    let st = &ctx.look.stroke;
    for piece in pieces {
        let c = ctx.look.fill.at(color::mean(&piece.iter().map(|q| q.1).collect::<Vec<_>>()));
        let mut segs = vec![Segment::MoveTo(piece[0].0)];
        segs.extend(piece[1..].iter().map(|q| Segment::LineTo(q.0)));
        segs.push(Segment::Close);
        if !e.canvas_frozen {
            e.canvas.include_path(&segs);
        }
        let stroke = Stroke { color: c, width: 0.5 * tikra_engine::PT, dash: None, cap: st.pen.cap, join: st.pen.join, miter_limit: st.pen.miter_limit, opacity: ctx.opacity.0 };
        e.canvas.push(Item::Path { segments: segs, fill: Some(Fill { paint: Paint::Solid(c), opacity: ctx.opacity.1, even_odd: false }), stroke: Some(stroke) });
    }
    true
}

fn paint_solid(e: &mut Engine, ctx: &CellCtx<'_>, cell: &CellColor) {
    let corners = &cell.corners;
    let mut segs = vec![Segment::MoveTo(corners[0])];
    segs.extend(corners[1..].iter().map(|c| Segment::LineTo(*c)));
    if ctx.closes {
        segs.push(Segment::Close);
    }
    if !e.canvas_frozen {
        e.canvas.include_path(&segs);
    }
    let st = &ctx.look.stroke;
    let width = ctx.width;
    let opacity = ctx.opacity;
    let stroke = |c: Color| Stroke {
        color: c,
        width,
        dash: st.pen.dash.clone().filter(|d| !d.0.is_empty()),
        cap: st.pen.cap,
        join: st.pen.join,
        miter_limit: st.pen.miter_limit,
        opacity: opacity.0,
    };
    let mapped = cell.mapped;
    let (fill_c, stroke_c) = if ctx.closes && (ctx.look.surf || ctx.plot.patch) {
        match ctx.look.shader.as_str() {
            // pgfplots shades each patch; flat patches with a hairline of their color have no seams between them either
            "interp" => (Some(ctx.look.fill.at(mapped)), Some(ctx.look.fill.at(mapped))),
            "flat" | "flat mean" | "flat corner" => (Some(ctx.look.fill.at(mapped)), Some(ctx.look.draw.at(mapped))),
            _ => (Some(ctx.look.fill.at(mapped)), Some(ctx.look.faceted.at(mapped))),
        }
    } else {
        (None, Some(ctx.look.draw.at(mapped)))
    };
    let fill_item = fill_c.map(|c| Fill { paint: Paint::Solid(c), opacity: opacity.1, even_odd: false });
    let mut stroke_item = stroke_c.map(stroke);
    if let (Some(st), true) = (stroke_item.as_mut(), ctx.look.surf && ctx.closes && ctx.look.shader == "interp") {
        st.width = 0.2 * tikra_engine::PT;
    }
    e.canvas.push(Item::Path { segments: segs, fill: fill_item, stroke: stroke_item });
}

/// The single path of one dimensional patches without point meta (`supports global path`):
/// their curves one after the other, joined where they meet. None for other plots.
pub fn global_path(e: &mut Engine, p: &Plot, pts: &[[f64; 3]], f: &Frame) -> Option<Vec<Segment>> {
    use super::patch::{Edge, Kind, Patch};
    if !p.patch || p.lattice.is_some() || p.has_meta {
        return None;
    }
    let kind = Kind::from_name(&keys::value(e, "/pgfplots/patch type"), 4);
    if !kind.is_curve() {
        return None;
    }
    let mut segs = Vec::new();
    let mut last: Option<Point> = None;
    for chunk in pts.chunks(kind.size()).filter(|c| c.len() == kind.size()) {
        let patch = Patch { kind, pts: chunk.to_vec(), metas: vec![f64::NAN; kind.size()] };
        for edge in patch.outline() {
            let (a, seg) = match edge {
                Edge::Line(a, b) => (f.at(a), Segment::LineTo(f.at(b))),
                Edge::Curve(a, c1, c2, b) => (f.at(a), Segment::CurveTo(f.at(c1), f.at(c2), f.at(b))),
            };
            if last.is_none_or(|q| q.minus(a).norm() > 1e-9) {
                segs.push(Segment::MoveTo(a));
            }
            last = Some(match seg {
                Segment::LineTo(b) | Segment::CurveTo(_, _, b) => b,
                _ => a,
            });
            segs.push(seg);
        }
    }
    Some(segs)
}

/// The range of a plot's point meta (`point meta rel=per plot`).
pub fn plot_meta_range(p: &Plot) -> Option<[f64; 2]> {
    let (lo, hi) = p.coords.iter().map(|c| c.meta).filter(|m| m.is_finite()).fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), m| (lo.min(m), hi.max(m)));
    (lo.is_finite() && hi > lo).then_some([lo, hi])
}

/// How a patch plot looks: its colors as functions of `mapped color`, its shader and stroke.
struct Look {
    draw: Linear,
    fill: Linear,
    faceted: Linear,
    shader: String,
    stroke: tikra_engine::engine::GState,
    /// `surf` or `patch` (filled), not `mesh`.
    surf: bool,
}

/// The depth of logical points along the view (larger is nearer), as the z buffer sorts: the
/// scalar product with the direction the axis' unit vectors flatten
/// (`\pgfplotsgetnormalforcurrentview`), in the coordinates of the data scale transformation.
struct Depth {
    c: [f64; 3],
}

impl Depth {
    fn new(e: &mut Engine, f: &Frame) -> Depth {
        let (n, s) = toward_viewer(e, f);
        Depth { c: [0, 1, 2].map(|i| n[i] * s[i]) }
    }

    fn of(&self, pts: &[[f64; 3]]) -> f64 {
        pts.iter().map(|l| (0..3).map(|i| if self.c[i] == 0.0 { 0.0 } else { self.c[i] * l[i] }).sum::<f64>()).sum::<f64>() / pts.len().max(1) as f64
    }
}

/// The direction toward the viewer in the coordinates of the data scale transformation, and the
/// transformation's factors.
fn toward_viewer(e: &mut Engine, f: &Frame) -> ([f64; 3], [f64; 3]) {
    let az = keys::number(e, "/pgfplots/view/az").unwrap_or(25.0).to_radians();
    let el = keys::number(e, "/pgfplots/view/el").unwrap_or(30.0).to_radians();
    let d = [az.sin() * el.cos(), -az.cos() * el.cos(), el.sin()];
    let s = [0, 1, 2].map(|i| super::draw::data_scale_trafo(e, f, i).0);
    let cross = |a: Point, b: Point| a.x * b.y - a.y * b.x;
    // the unit vectors of the transformed coordinates, and the direction they flatten
    let t = [0, 1, 2].map(|i| f.e[i].scale(1.0 / s[i]));
    let mut n = [cross(t[1], t[2]), cross(t[2], t[0]), cross(t[0], t[1])];
    // nearer is where `view={az}{el}` looks from: the sign of the flattened logical direction
    let u = [cross(f.e[1], f.e[2]), cross(f.e[2], f.e[0]), cross(f.e[0], f.e[1])];
    let toward: f64 = (0..3).map(|i| d[i] * u[i] / (f.max[i] - f.min[i]).abs().max(1e-300)).sum();
    let along: f64 = (0..3).map(|i| n[i] * s[i] * u[i]).sum();
    if toward * along < 0.0 {
        n = n.map(|v| -v);
    }
    (n, s)
}

/// One patch face for `mesh/interior colormap`: corners `a`, `b` (next in x) and `d` (next in y),
/// and the sign the z buffer kept on the normal.
struct Face {
    a: [f64; 3],
    b: [f64; 3],
    d: [f64; 3],
    sign: f64,
}

/// Whether a patch shows its inside: its normal (times `sign`) at least `thresh` (a cosine) away from the viewer.
struct Interior {
    n: [f64; 3],
    s: [f64; 3],
    thresh: f64,
}

impl Interior {
    fn new(e: &mut Engine, f: &Frame, thresh: f64) -> Interior {
        let (n, s) = toward_viewer(e, f);
        Interior { n, s, thresh }
    }

    fn shows(&self, face: Face) -> bool {
        let (n, s, thresh) = (self.n, self.s, self.thresh);
        let (a, b, d, sign) = (face.a, face.b, face.d, face.sign);
        let u = [0, 1, 2].map(|i| (b[i] - a[i]) * s[i]);
        let v = [0, 1, 2].map(|i| (d[i] - a[i]) * s[i]);
        let normal = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        let dot: f64 = (0..3).map(|i| -n[i] * normal[i] * sign).sum();
        let len = (n.iter().map(|x| x * x).sum::<f64>() * normal.iter().map(|x| x * x).sum::<f64>()).sqrt();
        if thresh == 0.0 {
            dot < 0.0
        } else {
            dot / len.max(1e-300) < thresh
        }
    }
}
