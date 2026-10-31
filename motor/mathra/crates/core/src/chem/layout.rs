//! From the tree to a drawing (spec/rules.md): atoms placed by their bonds' directions, labels
//! typeset by mathra and attached by the right piece, bonds cut at the labels, double bonds inside
//! their ring, wedges, hashes, aromatic circles, Lewis marks; then the compounds of a scheme in a
//! row (or along each arrow's direction), the reaction arrows with their labels and the curved
//! arrows of a mechanism.

use std::collections::BTreeMap;

use super::draw::*;
use super::model::*;
use super::scheme::R;
use crate::layout::{Context, MathBox, Style};

/// One em in font units.
const EM: f32 = 1000.0;
/// C-LAY-1: a standard bond is 2.2 em (chemfig's default is 3 em; the ACS style, 1.44 em).
pub const BOND: f32 = 2.2 * EM;
/// C-LAY-2: strokes 0.055 em wide; the second line of a double bond 0.22 em away.
pub const STROKE: f32 = 0.055 * EM;
const DOUBLE: f32 = 0.22 * EM;
const TRIPLE: f32 = 0.2 * EM;
/// Half the width of a wedge at its wide end.
const WEDGE: f32 = 0.11 * EM;
/// A bond stops this far from the letters of a label.
const LABEL_MARGIN: f32 = 0.09 * EM;
/// Half the height of a label's letters, around the atom's position.
const LETTER_HALF: f32 = 0.36 * EM;

/// The equation's layout context, and the style text is set in inside the drawing.
pub struct Ctx<'a, 'r> {
    pub layout: &'a mut Context<'r>,
    pub style: Style,
}

impl Ctx<'_, '_> {
    /// LaTeX read and composed like any part of the equation, in text style.
    pub fn typeset(&mut self, tex: &str) -> R<MathBox> {
        if tex.trim().is_empty() {
            return Ok(MathBox::empty());
        }
        let node = crate::input::latex::read_with(tex, self.layout.locale, self.layout.registry, self.layout.strict)?;
        crate::layout::compose(&node, self.style, self.layout)
    }

    /// Text over or under an arrow, or a compound name: LaTeX text with `$…$` for math.
    pub fn text(&mut self, tex: &str) -> R<MathBox> {
        self.typeset(&super::scheme::text_to_tex(tex))
    }
}

/// A scheme as a box of the equation: drawn at text size in units of 1000 per em, centered on the math
/// axis, then scaled to the style (a structure in a subscript is a script-size structure).
pub fn compose(s: &Scheme, e: Style, layout: &mut Context) -> R<MathBox> {
    let font = e.font();
    let per_em = font.units_per_em() / EM;
    let text = Style { level: crate::layout::Level::Text, cramped: false, font: e.font };
    let drawing = {
        let mut ctx = Ctx { layout, style: text };
        scheme(s, &mut ctx)?
    };
    let mut b = drawing.to_box();
    let bounds = drawing.bounds();
    let axis = font.c(|c| c.axis_height()) / per_em;
    let dy = axis - (bounds.y0 + bounds.y1) / 2.0;
    b.shift_vertical(dy);
    b.height = bounds.y1 + dy;
    b.depth = -(bounds.y0 + dy);
    let f = e.scale() * per_em;
    if (f - 1.0).abs() > 1e-6 {
        b.scale_box(f);
    }
    Ok(b)
}

/// Where each atom ended up, for the mechanism arrows.
pub struct Hooks {
    pub points: BTreeMap<String, (P, Rect)>,
}

/// One label piece, typeset, with its horizontal place inside the label.
struct Piece {
    math_box: MathBox,
    /// From the left edge of the label to the piece's left edge.
    x: f32,
}

struct Label {
    pieces: Vec<Piece>,
}

impl Label {
    fn center(&self, k: usize) -> f32 {
        self.pieces.get(k).map(|p| p.x + p.math_box.width / 2.0).unwrap_or(0.0)
    }
}

pub fn molecule(m: &Molecule, ctx: &mut Ctx, hooks: &mut Hooks) -> R<Drawing> {
    // the labels, piece by piece, upright as chemfig prints them
    let mut labels: Vec<Option<Label>> = Vec::with_capacity(m.atoms.len());
    for atom in &m.atoms {
        if atom.pieces.is_empty() {
            labels.push(None);
            continue;
        }
        let mut pieces = Vec::new();
        let mut x = 0.0;
        for p in &atom.pieces {
            let math_box = ctx.typeset(&format!("\\mathrm{{{p}}}"))?;
            let w = math_box.width;
            pieces.push(Piece { math_box, x });
            x += w;
        }
        labels.push(Some(Label { pieces }));
    }
    // the point of each piece, relative to its atom's position (the anchor piece's center)
    let offset = |atom: AtomId, k: usize| -> f32 {
        match &labels[atom] {
            Some(l) => l.center(k) - l.center(m.atoms[atom].anchor),
            None => 0.0,
        }
    };

    // C-LAY-3: each atom from the bond that first reaches it
    let mut pos: Vec<Option<P>> = vec![None; m.atoms.len()];
    if !pos.is_empty() {
        pos[0] = Some((0.0, 0.0));
    }
    for b in &m.bonds {
        let (Some(from), None) = (pos[b.from], pos[b.to]) else { continue };
        let start = (from.0 + offset(b.from, b.departure), from.1);
        let end = add(start, mul(direction(b.angle), BOND * b.length));
        pos[b.to] = Some((end.0 - offset(b.to, b.arrival), end.1));
    }
    let pos: Vec<P> = pos.into_iter().map(|p| p.unwrap_or((0.0, 0.0))).collect();
    let point = |atom: AtomId, k: usize| -> P { (pos[atom].0 + offset(atom, k), pos[atom].1) };
    // the box a bond stops at: the letters of one piece
    let stop = |atom: AtomId, k: usize| -> Option<Rect> {
        let l = labels[atom].as_ref()?;
        let piece = l.pieces.get(k)?;
        let left = pos[atom].0 - l.center(m.atoms[atom].anchor) + piece.x;
        let y = pos[atom].1;
        Some(Rect { x0: left - LABEL_MARGIN, y0: y - LETTER_HALF - LABEL_MARGIN, x1: left + piece.math_box.width + LABEL_MARGIN, y1: y + LETTER_HALF + LABEL_MARGIN })
    };
    let centers: Vec<P> = m
        .rings
        .iter()
        .map(|r| {
            let n = r.atoms.len().max(1) as f32;
            let s = r.atoms.iter().fold((0.0, 0.0), |acc, a| add(acc, pos[*a]));
            mul(s, 1.0 / n)
        })
        .collect();

    let mut d = Drawing::default();
    for b in &m.bonds {
        let (mut a, mut z) = (point(b.from, b.departure), point(b.to, b.arrival));
        let dir = unit(sub(z, a));
        if let Some(r) = stop(b.from, b.departure) {
            a = r.exit(a, dir);
        }
        if let Some(r) = stop(b.to, b.arrival) {
            z = r.exit(z, mul(dir, -1.0));
        }
        if length(sub(z, a)) < 1.0 || dir.0 * (z.0 - a.0) + dir.1 * (z.1 - a.1) <= 0.0 {
            continue;
        }
        bond(&mut d, b, a, z, b.ring.map(|r| centers[r]));
    }
    // C-LAY-5: an aromatic ring's circle, at 0.6 of its inner radius
    for (r, c) in m.rings.iter().zip(&centers) {
        if r.aromatic && r.atoms.len() >= 3 {
            let mean = r.atoms.iter().map(|a| length(sub(pos[*a], *c))).sum::<f32>() / r.atoms.len() as f32;
            let inner = mean * (std::f32::consts::PI / r.size as f32).cos();
            d.circle(*c, inner * 0.6, STROKE, false);
        }
    }
    let half_letter = LETTER_HALF;
    for (id, atom) in m.atoms.iter().enumerate() {
        let here = pos[id];
        let mut around = Rect::around(here, 0.12 * EM);
        if let Some(l) = &labels[id] {
            let left = here.0 - l.center(atom.anchor);
            // C-LAY-4: the letters' middle on the atom's position
            let baseline = here.1 - half_letter + 0.02 * EM;
            for p in &l.pieces {
                d.math(p.math_box.clone(), (left + p.x, baseline));
            }
            let a = l.pieces.get(atom.anchor).map(|p| (left + p.x, left + p.x + p.math_box.width)).unwrap_or((here.0, here.0));
            around = Rect { x0: a.0, y0: here.1 - half_letter, x1: a.1, y1: here.1 + half_letter };
        }
        for mark in &atom.lewis {
            lewis(&mut d, around, *mark);
        }
        if let Some(h) = &atom.hook {
            hooks.points.insert(h.clone(), (here, around));
        }
    }
    Ok(d)
}

fn bond(d: &mut Drawing, b: &Bond, a: P, z: P, ring_center: Option<P>) {
    let dir = unit(sub(z, a));
    let n = left(dir);
    let len = length(sub(z, a));
    match b.kind {
        BondKind::Single => d.line(a, z, STROKE),
        BondKind::Double => match ring_center {
            // C-LAY-6: in a ring the second line goes inside, shortened at both ends
            Some(c) => {
                d.line(a, z, STROKE);
                let side = if (c.0 - a.0) * n.0 + (c.1 - a.1) * n.1 >= 0.0 { 1.0 } else { -1.0 };
                let off = mul(n, DOUBLE * side);
                let trim = (0.16 * len).min(0.4 * EM);
                d.line(add(add(a, off), mul(dir, trim)), sub(add(z, off), mul(dir, trim)), STROKE);
            }
            None => {
                let off = mul(n, DOUBLE / 2.0);
                d.line(add(a, off), add(z, off), STROKE);
                d.line(sub(a, off), sub(z, off), STROKE);
            }
        },
        BondKind::Triple => {
            let off = mul(n, TRIPLE);
            d.line(a, z, STROKE);
            d.line(add(a, off), add(z, off), STROKE);
            d.line(sub(a, off), sub(z, off), STROKE);
        }
        BondKind::Wedge | BondKind::WedgeBack | BondKind::Hollow | BondKind::HollowBack => {
            let (narrow, wide) = if matches!(b.kind, BondKind::Wedge | BondKind::Hollow) { (a, z) } else { (z, a) };
            let points = vec![narrow, add(wide, mul(n, WEDGE)), sub(wide, mul(n, WEDGE))];
            d.polygon(points, matches!(b.kind, BondKind::Wedge | BondKind::WedgeBack), STROKE * 0.8);
        }
        BondKind::Hash | BondKind::HashBack => {
            let (narrow, wide) = if b.kind == BondKind::Hash { (a, z) } else { (z, a) };
            let along = unit(sub(wide, narrow));
            let count = ((len / (0.13 * EM)).floor() as usize).max(3);
            for i in 0..=count {
                let t = i as f32 / count as f32;
                let p = add(narrow, mul(along, len * t));
                let half = (WEDGE * t).max(STROKE * 0.6);
                d.line(add(p, mul(n, half)), sub(p, mul(n, half)), STROKE * 0.8);
            }
        }
        BondKind::Dashed => d.dashed(a, z, STROKE, (0.12 * EM, 0.09 * EM)),
        BondKind::Invisible => {}
    }
}

/// C-LAY-7: a Lewis mark beside the atom, in its direction.
fn lewis(d: &mut Drawing, around: Rect, mark: Lewis) {
    let dir = direction(mark.angle);
    let n = left(dir);
    let c = around.center();
    let at = add(c, mul(dir, around.reach(dir) + 0.17 * EM + 0.08 * EM * dir.1.abs()));
    let dot = 0.05 * EM;
    match mark.mark {
        LewisMark::Pair => {
            d.circle(add(at, mul(n, 0.09 * EM)), dot, 0.0, true);
            d.circle(sub(at, mul(n, 0.09 * EM)), dot, 0.0, true);
        }
        LewisMark::Radical => d.circle(at, dot, 0.0, true),
        LewisMark::Line => d.line(add(at, mul(n, 0.15 * EM)), sub(at, mul(n, 0.15 * EM)), STROKE),
        LewisMark::Vacancy => {
            let h = 0.1 * EM;
            let r = |u: f32, v: f32| add(at, add(mul(n, u), mul(dir, v)));
            d.polygon(vec![r(-h, -h), r(h, -h), r(h, h), r(-h, h)], false, STROKE * 0.8);
        }
    }
}

/// Head of an arrow at `tip`, pointing along `u`: two curved barbs, or one for a half head.
fn head(d: &mut Drawing, tip: P, u: P, half: Option<f32>) {
    let (h, w) = (0.3 * EM, 0.14 * EM);
    let n = left(u);
    let sides: &[f32] = match half {
        Some(s) => &[s][..],
        None => &[1.0, -1.0][..],
    };
    for &side in sides {
        let corner = add(sub(tip, mul(u, h)), mul(n, w * side));
        let control = add(sub(tip, mul(u, h * 0.45)), mul(n, w * side * 0.18));
        // a quadratic as a cubic
        let c1 = add(tip, mul(sub(control, tip), 2.0 / 3.0));
        let c2 = add(corner, mul(sub(control, corner), 2.0 / 3.0));
        d.curve(tip, c1, c2, corner, STROKE);
    }
}

/// C-SCH-2: a reaction arrow from `a` to `b`.
fn reaction_arrow(d: &mut Drawing, kind: ArrowKind, a: P, b: P) {
    let u = unit(sub(b, a));
    let n = left(u);
    let len = length(sub(b, a));
    let gap = 0.075 * EM;
    match kind {
        ArrowKind::Forward | ArrowKind::NoReaction => {
            d.line(a, b, STROKE);
            head(d, b, u, None);
            if kind == ArrowKind::NoReaction {
                let m = mul(add(a, b), 0.5);
                let s = 0.18 * EM;
                d.line(add(m, add(mul(n, s), mul(u, -s * 0.6))), add(m, add(mul(n, -s), mul(u, s * 0.6))), STROKE);
                d.line(add(m, add(mul(n, s), mul(u, s * 0.6))), add(m, add(mul(n, -s), mul(u, -s * 0.6))), STROKE);
            }
        }
        ArrowKind::Backward => {
            d.line(a, b, STROKE);
            head(d, a, mul(u, -1.0), None);
        }
        ArrowKind::Resonance => {
            d.line(a, b, STROKE);
            head(d, b, u, None);
            head(d, a, mul(u, -1.0), None);
        }
        ArrowKind::Equilibrium | ArrowKind::EquilibriumRight | ArrowKind::EquilibriumLeft => {
            // two harpoons: the upper one forward with its barb up, the lower one back with its barb down
            let short = 0.22 * len;
            let (fa, fb) = if kind == ArrowKind::EquilibriumLeft { (add(a, mul(u, short)), sub(b, mul(u, short))) } else { (a, b) };
            let (ba, bb) = if kind == ArrowKind::EquilibriumRight { (add(a, mul(u, short)), sub(b, mul(u, short))) } else { (a, b) };
            let up = mul(n, gap);
            d.line(add(fa, up), add(fb, up), STROKE);
            head(d, add(fb, up), u, Some(1.0));
            d.line(sub(ba, up), sub(bb, up), STROKE);
            head(d, sub(ba, up), mul(u, -1.0), Some(1.0));
        }
        ArrowKind::Retro => {
            // an open double arrow: two shafts and a wide head
            let off = mul(n, 0.07 * EM);
            let end = sub(b, mul(u, 0.12 * EM));
            d.line(add(a, off), add(end, off), STROKE);
            d.line(sub(a, off), sub(end, off), STROKE);
            let (h, w) = (0.28 * EM, 0.22 * EM);
            d.line(b, add(sub(b, mul(u, h)), mul(n, w)), STROKE);
            d.line(b, sub(sub(b, mul(u, h)), mul(n, w)), STROKE);
        }
        ArrowKind::Invisible => {}
    }
}

/// Where a compound sits in a scheme, and what it holds.
struct Placed {
    drawing: Drawing,
}

pub fn scheme(s: &Scheme, ctx: &mut Ctx) -> R<Drawing> {
    let mut hooks = Hooks { points: BTreeMap::new() };
    let mut out = Drawing::default();
    // the cursor: the box of the last thing placed, and the direction the next one goes in
    let mut last: Option<Rect> = None;
    let mut pending: Option<(&Arrow, Drawing, Drawing)> = None;
    let mut plus_pending = false;
    let gap = 0.45 * EM;
    for item in &s.items {
        match item {
            Item::Compound(c) => {
                let mut local = Hooks { points: BTreeMap::new() };
                let placed = compound(c, ctx, &mut local)?;
                let b = placed.drawing.bounds();
                let target = match (&pending, last, plus_pending) {
                    (Some((arrow, above, below)), Some(prev), _) => {
                        // C-SCH-1: the arrow leaves the previous compound along its direction
                        let u = direction(arrow.angle);
                        let base = (2.6 * EM).max(above.bounds().width().max(below.bounds().width()) + 1.0 * EM);
                        let len = base * arrow.length;
                        let start = add(prev.center(), mul(u, prev.reach(u) + gap));
                        let end = add(start, mul(u, len));
                        reaction_arrow(&mut out, arrow.kind, start, end);
                        arrow_labels(&mut out, start, end, above, below);
                        let center = add(end, mul(u, gap + b.reach(u)));
                        sub(center, b.center())
                    }
                    (None, Some(prev), true) => {
                        let plus = ctx.typeset("+")?;
                        let space = 0.35 * EM;
                        let x = prev.x1 + space;
                        let mid = prev.center().1;
                        let baseline = mid - (plus.height - plus.depth) / 2.0;
                        let w = plus.width;
                        out.math(plus, (x, baseline));
                        (x + w + space - b.x0, mid - b.center().1)
                    }
                    (None, Some(prev), false) => (prev.x1 + 0.5 * EM - b.x0, prev.center().1 - b.center().1),
                    _ => (-b.x0, -b.center().1),
                };
                out.place(&placed.drawing, target);
                for (k, (p, r)) in local.points {
                    hooks.points.insert(k, (add(p, target), r.shifted(target)));
                }
                last = Some(b.shifted(target));
                pending = None;
                plus_pending = false;
            }
            Item::Plus => plus_pending = true,
            Item::Arrow(a) => {
                let label = |l: &Option<super::model::Label>, ctx: &mut Ctx| -> R<Drawing> {
                    let mut d = Drawing::default();
                    match l {
                        None => {}
                        Some(super::model::Label::Text { tex }) => {
                            let m = ctx.text(tex)?;
                            d.math(m, (0.0, 0.0));
                        }
                        Some(super::model::Label::Structure(m)) => {
                            let mut ignored = Hooks { points: BTreeMap::new() };
                            d = molecule(m, ctx, &mut ignored)?.scaled(0.85);
                        }
                    }
                    Ok(d)
                };
                let above = label(&a.above, ctx)?;
                let below = label(&a.below, ctx)?;
                if let (Some((arrow, ab, be)), Some(prev)) = (pending.take(), last) {
                    // two arrows in a row: the first one ends in empty space
                    let u = direction(arrow.angle);
                    let start = add(prev.center(), mul(u, prev.reach(u) + gap));
                    let end = add(start, mul(u, 2.6 * EM * arrow.length));
                    reaction_arrow(&mut out, arrow.kind, start, end);
                    arrow_labels(&mut out, start, end, &ab, &be);
                    last = Some(Rect::around(end, 0.0));
                }
                pending = Some((a, above, below));
            }
        }
    }
    if let (Some((arrow, above, below)), Some(prev)) = (pending, last) {
        let u = direction(arrow.angle);
        let start = add(prev.center(), mul(u, prev.reach(u) + gap));
        let end = add(start, mul(u, (2.6 * EM).max(above.bounds().width() + EM) * arrow.length));
        reaction_arrow(&mut out, arrow.kind, start, end);
        arrow_labels(&mut out, start, end, &above, &below);
    }
    for m in &s.moves {
        electron_move(&mut out, m, &hooks)?;
    }
    Ok(out)
}

fn compound(c: &Compound, ctx: &mut Ctx, hooks: &mut Hooks) -> R<Placed> {
    let mut drawing = match &c.body {
        Body::Structure(m) => molecule(m, ctx, hooks)?,
        Body::Math { tex } => {
            let mut d = Drawing::default();
            let m = ctx.typeset(tex)?;
            d.math(m, (0.0, 0.0));
            d
        }
    };
    if let Some(name) = &c.name {
        let m = ctx.text(name)?;
        let b = drawing.bounds();
        let x = b.center().0 - m.width / 2.0;
        let y = b.y0 - 0.35 * EM - m.height;
        drawing.math(m, (x, y));
    }
    Ok(Placed { drawing })
}

/// The label above an arrow goes on its left side, the one below on its right (C-SCH-3).
fn arrow_labels(out: &mut Drawing, a: P, b: P, above: &Drawing, below: &Drawing) {
    let u = unit(sub(b, a));
    let n = left(u);
    let middle = mul(add(a, b), 0.5);
    for (label, side) in [(above, 1.0f32), (below, -1.0)] {
        if label.bounds.is_none() {
            continue;
        }
        let r = label.bounds();
        let nn = mul(n, side);
        let center = add(middle, mul(nn, 0.24 * EM + r.reach(nn)));
        out.place(label, sub(center, r.center()));
    }
}

/// C-SCH-4: a curved arrow from one hooked atom to another.
fn electron_move(out: &mut Drawing, m: &ElectronMove, hooks: &Hooks) -> R<()> {
    // the reader already dropped arrows to unknown hooks; a hook inside an arrow's label is not drawn
    let (Some((p0, r0)), Some((p3, r3))) = (hooks.points.get(&m.from), hooks.points.get(&m.to)) else {
        return Ok(());
    };
    let chord = sub(*p3, *p0);
    let dist = length(chord).max(EM);
    let out_dir = m.out_angle.map(direction).unwrap_or_else(|| rotate(unit(chord), m.bend));
    let in_dir = m.in_angle.map(direction).unwrap_or_else(|| rotate(unit(mul(chord, -1.0)), -m.bend));
    let (r1, r2) = m.reach.map(|(a, b)| (a * EM, b * EM)).unwrap_or((0.45 * dist, 0.45 * dist));
    // the arrow starts and ends just outside the atoms' letters
    let start = add(*p0, mul(out_dir, r0.reach(out_dir) + 0.12 * EM));
    let end = add(*p3, mul(in_dir, r3.reach(in_dir) + 0.12 * EM));
    let c1 = add(start, mul(out_dir, r1));
    let c2 = add(end, mul(in_dir, r2));
    out.curve(start, c1, c2, end, STROKE);
    let tangent = unit(sub(end, c2));
    head(out, end, tangent, if m.single { Some(1.0) } else { None });
    Ok(())
}
