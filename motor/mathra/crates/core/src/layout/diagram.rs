//! Commutative diagrams with free arrows (tikz-cd, xymatrix): the cells sit on a grid, as in amscd,
//! and each arrow goes from the border of one cell to the border of another, in any direction.
//! Everything is drawn with straight strokes (`Item::Line`): a bent arrow is a cubic Bézier sampled
//! finely, as are the hooks and the curved barbs of the heads.

use crate::tree::{ArrowHead, ArrowLine, ArrowTail, DiagramArrow, LabelSide, Node};
use crate::Error;

use super::{em, k, Context, Item, Level, MathBox, Style};

type R<T> = Result<T, Error>;
type P = (f32, f32);

/// Points per bent arrow, per hook and per barb: enough for a smooth stroke at any display size.
const CURVE_STEPS: usize = 40;
const BARB_STEPS: usize = 8;

#[derive(Clone, Copy)]
struct Rect {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

impl Rect {
    fn contains(&self, p: P) -> bool {
        p.0 >= self.x0 && p.0 <= self.x1 && p.1 >= self.y0 && p.1 <= self.y1
    }
}

fn add(a: P, b: P) -> P {
    (a.0 + b.0, a.1 + b.1)
}

fn sub(a: P, b: P) -> P {
    (a.0 - b.0, a.1 - b.1)
}

fn mul(a: P, f: f32) -> P {
    (a.0 * f, a.1 * f)
}

fn length(a: P) -> f32 {
    (a.0 * a.0 + a.1 * a.1).sqrt()
}

fn unit(a: P) -> P {
    let l = length(a);
    if l < 1e-6 {
        (1.0, 0.0)
    } else {
        (a.0 / l, a.1 / l)
    }
}

/// The left normal of a direction (y points up).
fn left(d: P) -> P {
    (-d.1, d.0)
}

fn rotate(d: P, radians: f32) -> P {
    let (s, c) = radians.sin_cos();
    (d.0 * c - d.1 * s, d.0 * s + d.1 * c)
}

/// Where the ray from `p` (inside `r`) along `d` leaves the rectangle.
fn exit(r: Rect, p: P, d: P) -> P {
    let mut t = f32::INFINITY;
    if d.0 > 1e-6 {
        t = t.min((r.x1 - p.0) / d.0);
    } else if d.0 < -1e-6 {
        t = t.min((r.x0 - p.0) / d.0);
    }
    if d.1 > 1e-6 {
        t = t.min((r.y1 - p.1) / d.1);
    } else if d.1 < -1e-6 {
        t = t.min((r.y0 - p.1) / d.1);
    }
    if !t.is_finite() {
        t = 0.0;
    }
    add(p, mul(d, t.max(0.0)))
}

fn polyline_length(path: &[P]) -> f32 {
    path.windows(2).map(|w| length(sub(w[1], w[0]))).sum()
}

/// The point at arc length `s` along the path, and the direction there.
fn point_at(path: &[P], s: f32) -> (P, P) {
    let mut left_over = s.max(0.0);
    for w in path.windows(2) {
        let seg = length(sub(w[1], w[0]));
        if left_over <= seg || seg < 1e-6 && left_over <= 0.0 {
            let d = unit(sub(w[1], w[0]));
            return (add(w[0], mul(d, left_over)), d);
        }
        left_over -= seg;
    }
    let n = path.len();
    (path[n - 1], unit(sub(path[n - 1], path[n.saturating_sub(2)])))
}

/// The part of the path between arc lengths `a` and `b`.
fn slice(path: &[P], a: f32, b: f32) -> Vec<P> {
    let mut out = vec![point_at(path, a).0];
    let mut s = 0.0;
    for w in path.windows(2) {
        let seg = length(sub(w[1], w[0]));
        s += seg;
        if s > a && s < b {
            out.push(w[1]);
        }
    }
    out.push(point_at(path, b).0);
    out
}

/// The same path at a constant distance to its left (negative: to its right).
fn offset(path: &[P], distance: f32) -> Vec<P> {
    (0..path.len())
        .map(|i| {
            let before = if i > 0 { unit(sub(path[i], path[i - 1])) } else { unit(sub(path[1], path[0])) };
            let after = if i + 1 < path.len() { unit(sub(path[i + 1], path[i])) } else { before };
            let n = unit(add(left(before), left(after)));
            add(path[i], mul(n, distance))
        })
        .collect()
}

/// A finely sampled copy of the path, one point every `step`.
fn resample(path: &[P], step: f32) -> Vec<P> {
    let total = polyline_length(path);
    let n = ((total / step).ceil() as usize).max(1);
    (0..=n).map(|i| point_at(path, total * i as f32 / n as f32).0).collect()
}

struct Pen {
    items: Vec<Item>,
    thickness: f32,
    /// Bounding box of everything drawn so far.
    bounds: Option<Rect>,
}

impl Pen {
    fn grow(&mut self, r: Rect) {
        self.bounds = Some(match self.bounds {
            None => r,
            Some(b) => Rect { x0: b.x0.min(r.x0), y0: b.y0.min(r.y0), x1: b.x1.max(r.x1), y1: b.y1.max(r.y1) },
        });
    }

    fn point(&mut self, p: P) {
        let h = self.thickness;
        self.grow(Rect { x0: p.0 - h, y0: p.1 - h, x1: p.0 + h, y1: p.1 + h });
    }

    fn line(&mut self, a: P, b: P) {
        self.point(a);
        self.point(b);
        self.items.push(Item::Line { x1: a.0, y1: a.1, x2: b.0, y2: b.1, thickness: self.thickness, color: None });
    }

    fn stroke(&mut self, path: &[P]) {
        for w in path.windows(2) {
            self.line(w[0], w[1]);
        }
    }

    /// A round dot: a stroke shorter than its width, which the round cap fills.
    fn dot(&mut self, p: P) {
        let w = self.thickness * 0.02;
        self.line((p.0 - w, p.1), (p.0 + w, p.1));
    }

    /// One curved barb of an arrow head, from the tip out to the corner.
    fn stroke_barb(&mut self, tip: Tip, size: BarbSize) {
        let n = left(tip.direction);
        let (h, w) = (size.length, size.width);
        for side in [1.0f32, -1.0] {
            let corner = add(sub(tip.at, mul(tip.direction, h)), mul(n, w * side));
            let control = add(sub(tip.at, mul(tip.direction, h * 0.45)), mul(n, w * side * 0.18));
            let points: Vec<P> = (0..=BARB_STEPS)
                .map(|s| {
                    let t = s as f32 / BARB_STEPS as f32;
                    let v = 1.0 - t;
                    (v * v * tip.at.0 + 2.0 * v * t * control.0 + t * t * corner.0, v * v * tip.at.1 + 2.0 * v * t * control.1 + t * t * corner.1)
                })
                .collect();
            self.stroke(&points);
        }
    }
}

/// The cells and the arrows of one commutative diagram.
pub struct Diagram<'a> {
    pub cells: &'a [Vec<Node>],
    pub arrows: &'a [DiagramArrow],
}

struct Extents {
    heights: Vec<f32>,
    depths: Vec<f32>,
    widths: Vec<f32>,
}

struct Gaps {
    columns: Vec<f32>,
    rows: Vec<f32>,
}

struct Grid {
    centers: Vec<f32>,
    bases: Vec<f32>,
    /// Math axis of the cell style: where an arrow meets a cell.
    axis: f32,
}

struct GridSource<'a> {
    boxes: &'a [Vec<MathBox>],
    arrows: &'a [DiagramArrow],
    labels: &'a [Vec<MathBox>],
    cell_style: Style,
    outer: Style,
}

struct Curve {
    points: Vec<P>,
    length: f32,
}

struct Tip {
    at: P,
    direction: P,
}

#[derive(Clone, Copy)]
struct BarbSize {
    length: f32,
    width: f32,
}

struct Cubic {
    start: P,
    end: P,
    out: P,
    arrival: P,
    theta: f32,
}

struct Canvas<'a> {
    boxes: &'a [Vec<MathBox>],
    grid: &'a Grid,
    pen: Pen,
    body: MathBox,
    style: Style,
}

pub fn compose(diagram: &Diagram, e: Style, ctx: &mut Context) -> R<MathBox> {
    let cell_style = e.not_display();
    let label_style = Style { level: if e.is_script() { Level::ScriptScript } else { Level::Script }, cramped: false, font: e.font };
    let boxes = compose_cells(diagram.cells, cell_style, ctx)?;
    let label_boxes = compose_labels(diagram.arrows, label_style, ctx)?;
    let grid = Grid::build(&GridSource { boxes: &boxes, arrows: diagram.arrows, labels: &label_boxes, cell_style, outer: e });
    let mut canvas = Canvas::new(&boxes, &grid, e);
    canvas.place_cells();
    for (arrow, labels) in diagram.arrows.iter().zip(&label_boxes) {
        canvas.draw_arrow(arrow, labels);
    }
    Ok(canvas.finish())
}

fn compose_cells(cells: &[Vec<Node>], style: Style, ctx: &mut Context) -> R<Vec<Vec<MathBox>>> {
    let ncol = cells.iter().map(Vec::len).max().unwrap_or(0);
    let mut boxes = Vec::with_capacity(cells.len());
    for row in cells {
        let mut out = Vec::with_capacity(ncol);
        for j in 0..ncol {
            out.push(match row.get(j) {
                Some(n) => super::compose(n, style, ctx)?,
                None => MathBox::empty(),
            });
        }
        boxes.push(out);
    }
    Ok(boxes)
}

fn compose_labels(arrows: &[DiagramArrow], style: Style, ctx: &mut Context) -> R<Vec<Vec<MathBox>>> {
    let mut label_boxes = Vec::with_capacity(arrows.len());
    for a in arrows {
        let mut out = Vec::new();
        for l in &a.labels {
            out.push(super::compose(&l.node, style, ctx)?);
        }
        label_boxes.push(out);
    }
    Ok(label_boxes)
}

fn extents(boxes: &[Vec<MathBox>], cell_style: Style) -> Extents {
    let (strut_height, strut_depth) = (em(cell_style, 0.7), em(cell_style, 0.25));
    let heights = boxes.iter().map(|r| r.iter().map(|b| b.height).fold(strut_height, f32::max)).collect();
    let depths = boxes.iter().map(|r| r.iter().map(|b| b.depth).fold(strut_depth, f32::max)).collect();
    let ncol = boxes.first().map_or(0, Vec::len);
    let widths = (0..ncol).map(|j| boxes.iter().map(|r| r[j].width).fold(0.0, f32::max)).collect();
    Extents { heights, depths, widths }
}

/// tikz-cd's default separations, widened where a label must fit beside a short arrow.
fn label_gaps(source: &GridSource) -> Gaps {
    let ncol = source.boxes.first().map_or(0, Vec::len);
    let nrow = source.boxes.len();
    let mut columns = vec![em(source.outer, 2.4); ncol.saturating_sub(1)];
    let mut rows = vec![em(source.outer, 2.1); nrow.saturating_sub(1)];
    for (a, labels) in source.arrows.iter().zip(source.labels) {
        let widest = labels.iter().map(|b| b.width).fold(0.0, f32::max);
        let tallest = labels.iter().map(MathBox::total).fold(0.0, f32::max);
        if a.from.0 == a.to.0 && a.from.1.abs_diff(a.to.1) == 1 {
            let j = a.from.1.min(a.to.1);
            columns[j] = columns[j].max(widest + em(source.outer, 1.4));
        }
        if a.from.1 == a.to.1 && a.from.0.abs_diff(a.to.0) == 1 && a.labels.iter().any(|l| l.side == LabelSide::On) {
            let i = a.from.0.min(a.to.0);
            rows[i] = rows[i].max(tallest + em(source.outer, 1.2));
        }
    }
    Gaps { columns, rows }
}

impl Grid {
    fn build(source: &GridSource) -> Grid {
        let ext = extents(source.boxes, source.cell_style);
        let gaps = label_gaps(source);
        let mut centers = Vec::with_capacity(ext.widths.len());
        let mut x = 0.0;
        for (j, w) in ext.widths.iter().enumerate() {
            centers.push(x + w / 2.0);
            x += w + gaps.columns.get(j).copied().unwrap_or(0.0);
        }
        let mut bases = Vec::with_capacity(ext.heights.len());
        let mut y = 0.0;
        for i in 0..ext.heights.len() {
            y -= ext.heights[i];
            bases.push(y);
            y -= ext.depths[i] + gaps.rows.get(i).copied().unwrap_or(0.0);
        }
        Grid { centers, bases, axis: k(source.cell_style, |c| c.axis_height()) }
    }
}

impl<'a> Canvas<'a> {
    fn new(boxes: &'a [Vec<MathBox>], grid: &'a Grid, style: Style) -> Canvas<'a> {
        let thickness = k(style, |c| c.fraction_rule_thickness()).max(em(style, 0.04));
        Canvas { boxes, grid, pen: Pen { items: Vec::new(), thickness, bounds: None }, body: MathBox::empty(), style }
    }

    fn place_cells(&mut self) {
        for (i, row) in self.boxes.iter().enumerate() {
            for (j, b) in row.iter().enumerate() {
                let x0 = self.grid.centers[j] - b.width / 2.0;
                self.body.place(b, x0, self.grid.bases[i]);
                if b.width > 0.0 {
                    self.pen.grow(Rect { x0, y0: self.grid.bases[i] - b.depth, x1: x0 + b.width, y1: self.grid.bases[i] + b.height });
                }
            }
        }
    }

    fn anchor(&self, cell: (usize, usize)) -> P {
        (self.grid.centers[cell.1], self.grid.bases[cell.0] + self.grid.axis)
    }

    fn border(&self, cell: (usize, usize)) -> Rect {
        let b = &self.boxes[cell.0][cell.1];
        let (cx, cy) = self.anchor(cell);
        if b.width == 0.0 && b.items.is_empty() {
            let p = em(self.style, 0.1);
            return Rect { x0: cx - p, y0: cy - p, x1: cx + p, y1: cy + p };
        }
        let p = em(self.style, 0.22);
        Rect { x0: cx - b.width / 2.0 - p, y0: self.grid.bases[cell.0] - b.depth - p, x1: cx + b.width / 2.0 + p, y1: self.grid.bases[cell.0] + b.height + p }
    }

    fn draw_arrow(&mut self, arrow: &DiagramArrow, labels: &[MathBox]) {
        let Some(curve) = self.arrow_path(arrow) else {
            return;
        };
        let breaks = self.place_labels(&curve, arrow, labels);
        let shaft = self.trimmed_shaft(&curve, arrow);
        self.draw_shaft(&shaft, arrow, &breaks);
        self.draw_head(&curve, arrow);
        self.draw_tail(&curve, arrow);
    }

    fn arrow_path(&self, arrow: &DiagramArrow) -> Option<Curve> {
        let (p, q) = (self.anchor(arrow.from), self.anchor(arrow.to));
        let d = unit(sub(q, p));
        let shift = mul(left(d), em(self.style, arrow.shift));
        let (p, q) = (add(p, shift), add(q, shift));
        let theta = (arrow.bend as f32).to_radians();
        let out = rotate(d, theta);
        let arrival = rotate(d, -theta);
        let start = exit(self.border(arrow.from), p, out);
        let end = exit(self.border(arrow.to), q, mul(arrival, -1.0));
        let points = if arrow.bend == 0 { vec![start, end] } else { cubic(Cubic { start, end, out, arrival, theta }) };
        let length = polyline_length(&points);
        (length >= 1e-3).then_some(Curve { points, length })
    }

    fn place_labels(&mut self, curve: &Curve, arrow: &DiagramArrow, labels: &[MathBox]) -> Vec<Rect> {
        let mut breaks = Vec::new();
        for (l, b) in arrow.labels.iter().zip(labels) {
            let (middle, along) = point_at(&curve.points, curve.length * l.position.clamp(0.0, 1.0));
            let normal = left(along);
            let reach = normal.0.abs() * b.width / 2.0 + normal.1.abs() * b.total() / 2.0;
            let gap = em(self.style, 0.16);
            let center = match l.side {
                LabelSide::Left => add(middle, mul(normal, gap + reach)),
                LabelSide::Right => sub(middle, mul(normal, gap + reach)),
                LabelSide::On => middle,
            };
            let x0 = center.0 - b.width / 2.0;
            let base = center.1 - (b.height - b.depth) / 2.0;
            self.body.place(b, x0, base);
            let r = Rect { x0, y0: base - b.depth, x1: x0 + b.width, y1: base + b.height };
            self.pen.grow(r);
            if l.side == LabelSide::On {
                let p = em(self.style, 0.12);
                breaks.push(Rect { x0: r.x0 - p, y0: r.y0 - p, x1: r.x1 + p, y1: r.y1 + p });
            }
        }
        breaks
    }

    fn trimmed_shaft(&self, curve: &Curve, arrow: &DiagramArrow) -> Vec<P> {
        let double = arrow.line == ArrowLine::Double;
        let barb = barb_size(double, self.style);
        let trim = |has: bool| {
            if !has {
                0.0
            } else if double {
                barb.length * 0.55
            } else {
                self.pen.thickness / 2.0
            }
        };
        let head_trim = trim(arrow.head != ArrowHead::None);
        let tail_trim = trim(arrow.tail == ArrowTail::Head);
        slice(&curve.points, tail_trim, (curve.length - head_trim).max(tail_trim))
    }

    fn draw_shaft(&mut self, path: &[P], arrow: &DiagramArrow, breaks: &[Rect]) {
        match arrow.line {
            ArrowLine::Solid => self.draw_visible(path, breaks),
            ArrowLine::Double => self.draw_double(path, breaks),
            ArrowLine::Dashed => self.draw_dashed(path, breaks),
            ArrowLine::Dotted => self.draw_dotted(path, breaks),
            ArrowLine::Squiggly => self.draw_squiggly(path, breaks),
            ArrowLine::None => {}
        }
    }

    fn draw_visible(&mut self, line: &[P], breaks: &[Rect]) {
        let sampled = if breaks.is_empty() { line.to_vec() } else { resample(line, em(self.style, 0.04)) };
        for w in sampled.windows(2) {
            if !breaks.iter().any(|r| r.contains(mul(add(w[0], w[1]), 0.5))) {
                self.pen.line(w[0], w[1]);
            }
        }
    }

    fn draw_double(&mut self, path: &[P], breaks: &[Rect]) {
        let gap = em(self.style, 0.09);
        self.draw_visible(&offset(path, gap), breaks);
        self.draw_visible(&offset(path, -gap), breaks);
    }

    fn draw_dashed(&mut self, path: &[P], breaks: &[Rect]) {
        let (on, off) = (em(self.style, 0.26), em(self.style, 0.16));
        let span = polyline_length(path);
        let mut s = 0.0;
        while s < span {
            self.draw_visible(&slice(path, s, (s + on).min(span)), breaks);
            s += on + off;
        }
    }

    fn draw_dotted(&mut self, path: &[P], breaks: &[Rect]) {
        let span = polyline_length(path);
        let n = ((span / em(self.style, 0.2)).round() as usize).max(1);
        for s in 0..=n {
            let (pt, _) = point_at(path, span * s as f32 / n as f32);
            if !breaks.iter().any(|r| r.contains(pt)) {
                self.pen.dot(pt);
            }
        }
    }

    fn draw_squiggly(&mut self, path: &[P], breaks: &[Rect]) {
        let span = polyline_length(path);
        let (amplitude, wave) = (em(self.style, 0.07), em(self.style, 0.3));
        let lead = em(self.style, 0.12).min(span / 4.0);
        let fine = resample(path, em(self.style, 0.03));
        let mut s = 0.0;
        let mut wavy = Vec::with_capacity(fine.len());
        for (n, pt) in fine.iter().enumerate() {
            if n > 0 {
                s += length(sub(*pt, fine[n - 1]));
            }
            let (_, dir) = point_at(path, s);
            let k = if s < lead || s > span - lead { 0.0 } else { ((s - lead) / wave * std::f32::consts::TAU).sin() * amplitude };
            wavy.push(add(*pt, mul(left(dir), k)));
        }
        self.draw_visible(&wavy, breaks);
    }

    fn draw_head(&mut self, curve: &Curve, arrow: &DiagramArrow) {
        let (at, direction) = point_at(&curve.points, curve.length);
        let size = barb_size(arrow.line == ArrowLine::Double, self.style);
        match arrow.head {
            ArrowHead::Normal => self.pen.stroke_barb(Tip { at, direction }, size),
            ArrowHead::TwoHeads => {
                self.pen.stroke_barb(Tip { at, direction }, size);
                self.pen.stroke_barb(Tip { at: sub(at, mul(direction, em(self.style, 0.14))), direction }, size);
            }
            ArrowHead::None => {}
        }
    }

    fn draw_tail(&mut self, curve: &Curve, arrow: &DiagramArrow) {
        let (at, direction) = point_at(&curve.points, 0.0);
        let double = arrow.line == ArrowLine::Double;
        let size = barb_size(double, self.style);
        match arrow.tail {
            ArrowTail::None => {}
            ArrowTail::Head => self.pen.stroke_barb(Tip { at, direction: mul(direction, -1.0) }, size),
            ArrowTail::Tail => self.pen.stroke_barb(Tip { at: add(at, mul(direction, size.length * 0.8)), direction }, size),
            ArrowTail::Bar => {
                let n = left(direction);
                self.pen.line(add(at, mul(n, size.width)), sub(at, mul(n, size.width)));
            }
            ArrowTail::Hook | ArrowTail::HookRight => self.stroke_hook(at, direction, arrow.tail),
        }
    }

    fn stroke_hook(&mut self, root: P, direction: P, tail: ArrowTail) {
        let side = if tail == ArrowTail::Hook { 1.0 } else { -1.0 };
        let n = mul(left(direction), side);
        let r = em(self.style, 0.1);
        let center = add(root, mul(n, r));
        let points: Vec<P> = (0..=CURVE_STEPS / 2)
            .map(|s| {
                let phi = std::f32::consts::PI * s as f32 / (CURVE_STEPS / 2) as f32;
                add(center, add(mul(n, -r * phi.cos()), mul(direction, -r * phi.sin())))
            })
            .collect();
        self.pen.stroke(&points);
    }

    /// The whole diagram centered on the math axis, its left edge at x = 0.
    fn finish(mut self) -> MathBox {
        let bounds = self.pen.bounds.unwrap_or(Rect { x0: 0.0, y0: 0.0, x1: 0.0, y1: 0.0 });
        self.body.items.extend(self.pen.items);
        let dy = k(self.style, |c| c.axis_height()) - (bounds.y0 + bounds.y1) / 2.0;
        let mut math_box = MathBox::space(bounds.x1 - bounds.x0);
        math_box.place(&self.body, -bounds.x0, dy);
        math_box.height = bounds.y1 + dy;
        math_box.depth = -(bounds.y0 + dy);
        math_box
    }
}

fn barb_size(double: bool, e: Style) -> BarbSize {
    if double {
        BarbSize { length: em(e, 0.34), width: em(e, 0.24) }
    } else {
        BarbSize { length: em(e, 0.3), width: em(e, 0.15) }
    }
}

fn cubic(curve: Cubic) -> Vec<P> {
    let Cubic { start, end, out, arrival, theta } = curve;
    let reach = length(sub(end, start)) * 0.3915 / theta.cos().max(0.2);
    let (c1, c2) = (add(start, mul(out, reach)), sub(end, mul(arrival, reach)));
    (0..=CURVE_STEPS)
        .map(|s| {
            let t = s as f32 / CURVE_STEPS as f32;
            let u = 1.0 - t;
            let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            (w[0] * start.0 + w[1] * c1.0 + w[2] * c2.0 + w[3] * end.0, w[0] * start.1 + w[1] * c1.1 + w[2] * c2.1 + w[3] * end.1)
        })
        .collect()
}
