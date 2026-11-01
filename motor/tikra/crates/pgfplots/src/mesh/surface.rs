use tikra_engine::canvas::{Fill, Item, Paint, Stroke};
use tikra_engine::color::Color;
use tikra_engine::scene::{Point, Segment};
use tikra_engine::Error;

use super::super::draw::Frame;
use super::super::keys;
use super::super::Engine;

use super::color::gouraud;
use super::{map, Depth, Look, MeshDraw, PatchPaint};
pub(super) fn stroke_for(look: &Look, color: Color, width: f64) -> Stroke {
    let st = &look.stroke;
    Stroke {
        color,
        width,
        dash: st.pen.dash.clone().filter(|d| !d.0.is_empty()),
        cap: st.pen.cap,
        join: st.pen.join,
        miter_limit: st.pen.miter_limit,
        opacity: st.paint.draw_opacity,
    }
}

pub(super) fn segments_of(frame: &Frame, edges: &[super::super::patch::Edge], close: bool) -> Vec<Segment> {
    use super::super::patch::Edge;
    let mut segs = Vec::new();
    for (k, edge) in edges.iter().enumerate() {
        let (a, rest): (_, Vec<Point>) = match *edge {
            Edge::Line(a, b) => (a, vec![frame.at(b)]),
            Edge::Curve(a, c1, c2, b) => (a, vec![frame.at(c1), frame.at(c2), frame.at(b)]),
        };
        if k == 0 {
            segs.push(Segment::MoveTo(frame.at(a)));
        }
        segs.push(if rest.len() == 1 { Segment::LineTo(rest[0]) } else { Segment::CurveTo(rest[0], rest[1], rest[2]) });
    }
    if close && !segs.is_empty() {
        segs.push(Segment::Close);
    }
    segs
}

pub(super) struct PatchJob<'a> {
    mesh: &'a MeshDraw<'a>,
    look: &'a Look,
    kind: super::super::patch::Kind,
    depth: &'a Depth,
}

pub(super) struct PatchList {
    patches: Vec<super::super::patch::Patch>,
    kind: super::super::patch::Kind,
    depth: Depth,
}

pub(super) fn patch_list(e: &mut Engine, d: &MeshDraw<'_>, kind_name: &str) -> PatchList {
    use super::super::patch::{Kind, Patch};
    let count = keys::number(e, "/pgfplots/vertex count").unwrap_or(4.0).max(1.0) as usize;
    let kind = Kind::from_name(kind_name, count);
    let refines = keys::number(e, "/pgfplots/patch refines").unwrap_or(0.0).clamp(0.0, 6.0) as usize;
    let to_triangles = keys::flag(e, "/pgfplots/patch to triangles");
    let dims = if d.frame.three_d { 3 } else { 2 };
    let mut list: Vec<Patch> = Vec::new();
    for (i, chunk) in d.pts.chunks(kind.size()).enumerate() {
        if chunk.len() < kind.size() || chunk.iter().any(|l| l.iter().take(dims).any(|v| !v.is_finite())) {
            continue;
        }
        let metas: Vec<f64> = (0..kind.size()).map(|k| d.plot.coords.get(i * kind.size() + k).map_or(f64::NAN, |c| c.meta)).collect();
        list.push(Patch { kind, pts: chunk.to_vec(), metas });
    }
    for _ in 0..refines {
        list = list.iter().flat_map(Patch::refine).collect();
    }
    if to_triangles {
        list = list.iter().flat_map(Patch::triangulate).collect();
    }
    let zbuffer = keys::value(e, "/pgfplots/z buffer");
    let depth = Depth::new(e, d.frame);
    if d.frame.three_d && zbuffer != "none" {
        list.sort_by(|a, b| depth.of(&a.pts).total_cmp(&depth.of(&b.pts)));
    }
    PatchList { patches: list, kind, depth }
}

pub(super) struct ShadeWork<'a> {
    patch: &'a super::super::patch::Patch,
    mapped: Color,
    outline: Vec<Segment>,
}

pub(super) struct FillWork {
    mapped: Color,
    outline: Vec<Segment>,
}

pub(super) fn shade_patch(e: &mut Engine, job: &PatchJob<'_>, work: ShadeWork<'_>) {
    use super::super::patch::Kind;
    let f = job.mesh.frame;
    let look = job.look;
    let st = &look.stroke;
    // the shading: small pieces of the patch, each of the color at its middle; a rectangle is shaded as two triangles
    let parts = if job.kind == Kind::Rectangle { work.patch.triangulate() } else { vec![work.patch.clone()] };
    e.canvas.push(Item::Crisp);
    for part in parts {
        // a triangle: its linear gradient, exact
        if let Some(cm) = f.colormap.as_ref().filter(|c| c.access == super::super::colormap::Access::Map && part.kind == Kind::Triangle) {
            let c: Vec<Point> = part.pts.iter().map(|l| f.at(*l)).collect();
            let paint = gouraud(&[(c[0], part.metas[0]), (c[1], part.metas[1]), (c[2], part.metas[2])], cm, f.meta, &look.fill);
            let segs = vec![Segment::MoveTo(c[0]), Segment::LineTo(c[1]), Segment::LineTo(c[2]), Segment::Close];
            e.canvas.push(Item::Path { segments: segs, fill: Some(Fill { paint, opacity: st.paint.fill_opacity, even_odd: false }), stroke: None });
            continue;
        }
        let size = {
            let c: Vec<Point> = part.pts.iter().map(|l| f.at(*l)).collect();
            let (lo, hi) = c.iter().fold((c[0], c[0]), |(lo, hi), q| (Point::new(lo.x.min(q.x), lo.y.min(q.y)), Point::new(hi.x.max(q.x), hi.y.max(q.y))));
            (hi.x - lo.x).max(hi.y - lo.y)
        };
        let curved = !matches!(part.kind, Kind::Triangle | Kind::Bilinear | Kind::Rectangle | Kind::Polygon(_));
        let n = ((size / (3.0 * tikra_engine::PT)).ceil() as usize).clamp(if curved { 6 } else { 1 }, 48);
        let mut pieces = part.pieces(n);
        if f.three_d {
            pieces.sort_by(|a, b| job.depth.of(&a.0).total_cmp(&job.depth.of(&b.0)));
        }
        for (q, meta) in pieces {
            let c = look.fill.at(map(f, meta));
            let mut segs = vec![Segment::MoveTo(f.at(q[0]))];
            segs.extend(q[1..].iter().map(|l| Segment::LineTo(f.at(*l))));
            segs.push(Segment::Close);
            let stroke =
                Stroke { color: c, width: 0.5 * tikra_engine::PT, dash: None, cap: st.pen.cap, join: st.pen.join, miter_limit: st.pen.miter_limit, opacity: st.paint.draw_opacity };
            e.canvas.push(Item::Path { segments: segs, fill: Some(Fill { paint: Paint::Solid(c), opacity: st.paint.fill_opacity, even_odd: false }), stroke: Some(stroke) });
        }
    }
    e.canvas.push(Item::End);
    if look.shader == "faceted interp" {
        let width = st.pen.line_width;
        e.canvas.push(Item::Path { segments: work.outline, fill: None, stroke: Some(stroke_for(look, look.faceted.at(work.mapped), width)) });
    }
}

pub(super) fn fill_patch(e: &mut Engine, job: &PatchJob<'_>, work: FillWork) {
    let look = job.look;
    let interp = look.shader.contains("interp");
    let edge = if look.shader.starts_with("flat") {
        look.draw.at(work.mapped)
    } else if interp {
        look.fill.at(work.mapped)
    } else {
        look.faceted.at(work.mapped)
    };
    let width = look.stroke.pen.line_width;
    e.canvas.push(Item::Path {
        segments: work.outline,
        fill: Some(Fill { paint: Paint::Solid(look.fill.at(work.mapped)), opacity: look.stroke.paint.fill_opacity, even_odd: false }),
        stroke: Some(stroke_for(look, edge, width)),
    });
}

pub(super) fn paint_one_patch(e: &mut Engine, job: &PatchJob<'_>, patch: super::super::patch::Patch) {
    let m = patch.flat_meta(job.look.shader == "flat corner");
    // a patch whose color comes from a point meta of nan is not drawn
    if job.mesh.plot.has_meta && !m.is_finite() {
        return;
    }
    let mapped = if m.is_finite() { map(job.mesh.frame, m) } else { Color::BLACK };
    let curve = job.kind.is_curve();
    let outline = segments_of(job.mesh.frame, &patch.outline(), !curve);
    if !e.canvas_frozen {
        e.canvas.include_path(&outline);
    }
    if curve || !job.look.surf {
        let width = job.look.stroke.pen.line_width;
        e.canvas.push(Item::Path { segments: outline, fill: None, stroke: Some(stroke_for(job.look, job.look.draw.at(mapped), width)) });
        return;
    }
    if job.look.shader.contains("interp") && patch.metas.iter().all(|meta| meta.is_finite()) {
        shade_patch(e, job, ShadeWork { patch: &patch, mapped, outline });
        return;
    }
    fill_patch(e, job, FillWork { mapped, outline });
}

/// `\addplot [patch, patch type=…]` from a list of patches (`mesh input=patches`): each patch
/// refined (`patch refines`) and cut in triangles (`patch to triangles`) as asked, sorted back to
/// front in 3d, then filled with one color (`flat`, `faceted`) or shaded (`interp`), its outline
/// drawn (`faceted`, `faceted interp`, a `mesh`); a curve (`line`, the splines) is drawn.
pub(super) fn draw_patches(e: &mut Engine, d: &MeshDraw<'_>, paint: &PatchPaint<'_>) -> Result<(), Error> {
    let PatchList { patches, kind, depth } = patch_list(e, d, paint.kind);
    let job = PatchJob { mesh: d, look: paint.look, kind, depth: &depth };
    for patch in patches {
        paint_one_patch(e, &job, patch);
    }
    Ok(())
}
