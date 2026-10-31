//! Node-style components: `\draw (0,0) node[ground] (G) {};`.
//!
//! Each is a node shape declared to tikra (`tikra::extension::Context::declare_shape`): a node like
//! any other, with the component's anchors (`G.north`, `B.zero`), placed by them (`anchor=hot`),
//! turned with the node, and filled with its `fill` where the manual makes it fillable. The drawing
//! is measured on what CircuiTikZ draws, in `bipoles/length` (1.4 cm), with the node's point at the
//! origin; the terminals are `nodes width` (0.04) across the half.

use tikra::extension::{Context, Error, LineCap, NodeShape, PathSegment, Point, ShapeBorder, ShapeInput, ShapePaint, ShapePath, ShapeText, TextAnchor, Tokens};

use crate::bipole::setting;
pub use displays::{k_seven_segment_bits, k_seven_segment_val};

mod amplifiers;
mod blocks;
mod chips;
mod displays;
mod doubles;
mod flipflops;
mod logic;
mod muxdemux;
mod symbols;
mod transistors;
mod tubes;
use crate::components::Op;
use Op::{Close as Z, Curve as C, Line as L, Move as M};

/// A path of a node component and how it is painted.
#[derive(Clone, Copy, Debug)]
pub enum Part {
    /// Stroked as wide as the node's lines, twice, or three times.
    Thin(&'static [Op]),
    Thick(&'static [Op]),
    Thicker(&'static [Op]),
    /// Filled with the node's `fill`, if any.
    Fill(&'static [Op]),
    /// Filled with the node's line color.
    Solid(&'static [Op]),
    /// Filled with the node's `fill`, or its line color: a depletion transistor's channel.
    Channel(&'static [Op]),
    /// A pole: filled with the node's `fill`, or white.
    Pole(&'static [Op]),
    /// Stroked `k` times as wide as the node's lines (a cute switch's lever).
    Stroke(f64, &'static [Op]),
}

/// Where a node component's text goes: a point, and the side of the text's box put there.
#[derive(Clone, Copy, Debug)]
pub enum Text {
    /// Nowhere special: centered on the node's point.
    Center,
    /// Above the point, centered (its box's bottom there, past the inner sep).
    Above([f64; 2]),
    Below([f64; 2]),
    /// Above, with its right or left edge at the point.
    AboveLeft([f64; 2]),
    AboveRight([f64; 2]),
    /// Right of the point, its box's left edge there, centered on it (a transistor's).
    Right([f64; 2]),
    /// Left of the point, its box's right edge there, centered on it (a TX antenna's).
    Left([f64; 2]),
    /// Centered on the point (a magnetron's).
    Middle([f64; 2]),
    /// Above the point, centered, its baseline 0.6 of its height above it (a transformer's).
    Over([f64; 2]),
}

/// A node component: its name (`node[name]`), its drawing, its box (left, right, bottom, top) for
/// the compass anchors, its own anchors and where its text goes. Its drawing counts in the
/// picture's size, its text does not, unless `text_counts`; `extent` are points that count too.
#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub name: &'static str,
    pub parts: &'static [Part],
    pub bounds: [f64; 4],
    pub anchors: &'static [(&'static str, [f64; 2])],
    pub text: Text,
    pub text_counts: bool,
    pub extent: &'static [[f64; 2]],
    /// Its sizes that a setting changes (`tripoles/op amp/width`): it is drawn in between its
    /// default and the node measured with each of them `step` more, as it goes in a straight line.
    pub params: &'static [Param],
    /// The pairs of its sizes that change it together (width and port width: the leads are what
    /// the body leaves of the width), and the node measured with both `step` more.
    pub crosses: &'static [Cross],
}

/// Two sizes of a node component that change it together, and the node `name` drawn with both.
#[derive(Clone, Copy, Debug)]
pub struct Cross {
    pub a: &'static str,
    pub b: &'static str,
    pub name: &'static str,
}

/// A size of a node component: its setting, its default, and the node `name` drawn with it `step`
/// more than its default.
#[derive(Clone, Copy, Debug)]
pub struct Param {
    pub setting: &'static str,
    pub default: f64,
    pub step: f64,
    pub name: &'static str,
}

const fn node(name: &'static str, parts: &'static [Part], bounds: [f64; 4]) -> Node {
    Node { name, parts, bounds, anchors: &[], text: Text::Center, text_counts: false, extent: &[], params: &[], crosses: &[] }
}

/// Every size a setting changes, for its default.
pub fn params() -> impl Iterator<Item = &'static Param> {
    NODES
        .iter()
        .chain(transistors::TRANSISTORS)
        .chain(tubes::TUBES)
        .chain(amplifiers::AMPLIFIERS)
        .chain(doubles::DOUBLE_BIPOLES)
        .chain(logic::LOGIC)
        .chain(blocks::BLOCKS)
        .flat_map(|n| n.params)
}

impl Part {
    fn ops(&self) -> &'static [Op] {
        match *self {
            Part::Thin(o) | Part::Thick(o) | Part::Thicker(o) | Part::Fill(o) | Part::Solid(o) | Part::Pole(o) | Part::Channel(o) | Part::Stroke(_, o) => o,
        }
    }
}

/// `base`, moved by each `(k, other)` k times as far as `other` is from it.
fn mix_ops(base: &[Op], others: &[(f64, &[Op])]) -> Vec<Op> {
    let at = |i: usize, f: &dyn Fn(&Op) -> Vec<f64>| -> Vec<f64> {
        let mut v = f(&base[i]);
        for (k, o) in others {
            for (a, b) in v.iter_mut().zip(f(&o[i]).iter().zip(f(&base[i]))) {
                *a += k * (b.0 - b.1);
            }
        }
        v
    };
    let nums = |op: &Op| match *op {
        Op::Move(x, y) | Op::Line(x, y) => vec![x, y],
        Op::Curve(a, b, c) => vec![a[0], a[1], b[0], b[1], c[0], c[1]],
        Op::Close => vec![],
    };
    let mixed: Vec<Op> = (0..base.len())
        .map(|i| {
            let v = at(i, &nums);
            match base[i] {
                Op::Move(..) => Op::Move(v[0], v[1]),
                Op::Line(..) => Op::Line(v[0], v[1]),
                Op::Curve(..) => Op::Curve([v[0], v[1]], [v[2], v[3]], [v[4], v[5]]),
                Op::Close => Op::Close,
            }
        })
        .collect();
    // a line that shrinks to nothing is not drawn (an amplifier's leads with `port width=1`)
    let mut out = Vec::new();
    let mut i = 0;
    while i < mixed.len() {
        if let (Op::Move(x, y), Some(Op::Line(a, b))) = (mixed[i], mixed.get(i + 1)) {
            let alone = !matches!(mixed.get(i + 2), Some(Op::Line(..) | Op::Curve(..) | Op::Close));
            if alone && (x - a).hypot(y - b) < 1e-3 {
                i += 2;
                continue;
            }
        }
        out.push(mixed[i]);
        i += 1;
    }
    out
}

/// A closed path of four curves: a transistor's circle.
fn is_circle(ops: &[Op]) -> bool {
    ops.len() == 6 && matches!(ops[0], Op::Move(..)) && matches!(ops[5], Op::Close) && ops[1..5].iter().all(|o| matches!(o, Op::Curve(..)))
}

/// A transistor's circle `k` times as large, from its left side: its paths (the closed ones of four
/// curves) and its anchors (`circle C`…).
/// A dynode's arc, which its sizes do not change linearly: a circle's arc across its 0.4 wide stem
/// with its lowest point at `arc pos` of its height, meeting the stem's ends at 90° minus `arc
/// angle` from the horizontal; none at `arc angle=90` (measured).
fn dynode_arc(cx: &mut Context, sized: &mut Sized) {
    let Some((_, ops)) = sized.parts.iter_mut().find(|(part, _)| matches!(part, Part::Thin(_))) else { return };
    let angle = crate::bipole::number(cx, "monopoles/dynode/arc angle", 30.0);
    let low = crate::bipole::number(cx, "monopoles/dynode/arc pos", 0.5) * 0.8;
    let theta = (90.0 - angle).to_radians();
    let mut arc = Vec::new();
    if theta.sin() > 1e-6 {
        let r = 0.2 / theta.sin();
        let (cx0, cy0) = (0.0, low + r);
        let at = |a: f64| [cx0 + r * a.cos(), cy0 + r * a.sin()];
        let down = -std::f64::consts::FRAC_PI_2;
        // two curves, from the left end to the lowest point and on to the right end
        let k = 4.0 / 3.0 * (theta / 4.0).tan() * r;
        let start = at(down - theta);
        arc.push(Op::Move(start[0], start[1]));
        for (a0, a1) in [(down - theta, down), (down, down + theta)] {
            let (p0, p1) = (at(a0), at(a1));
            let c1 = [p0[0] - k * a0.sin(), p0[1] + k * a0.cos()];
            let c2 = [p1[0] + k * a1.sin(), p1[1] - k * a1.cos()];
            arc.push(Op::Curve(c1, c2, p1));
        }
    }
    // the stem stays: what follows the arc's two curves
    let stem: Vec<Op> = ops.iter().skip_while(|o| !matches!(o, Op::Move(x, _) if x.abs() < 1e-9)).copied().collect();
    arc.extend(stem);
    *ops = arc;
}

/// A transistor's circle: its left and right ends and the height of its center, in the node's units.
fn circle_of(sized: &Sized) -> Option<(f64, f64, f64)> {
    let (_, ops) = sized.parts.iter().find(|(_, ops)| is_circle(ops))?;
    let xs: Vec<f64> = ops
        .iter()
        .flat_map(|o| match *o {
            Op::Move(x, _) | Op::Line(x, _) => vec![x],
            Op::Curve(a, b, c) => vec![a[0], b[0], c[0]],
            Op::Close => vec![],
        })
        .collect();
    let left = xs.iter().copied().fold(f64::MAX, f64::min);
    let right = xs.iter().copied().fold(f64::MIN, f64::max);
    // (it starts at its right, level with its center)
    let cy = match ops[0] {
        Op::Move(_, y) => y,
        _ => 0.0,
    };
    Some((left, right, cy))
}

/// Moves every point of a part.
fn move_ops(ops: &mut [Op], f: impl Fn([f64; 2]) -> [f64; 2]) {
    for o in ops.iter_mut() {
        *o = match *o {
            Op::Move(x, y) => {
                let q = f([x, y]);
                Op::Move(q[0], q[1])
            }
            Op::Line(x, y) => {
                let q = f([x, y]);
                Op::Line(q[0], q[1])
            }
            Op::Curve(a, b, c) => Op::Curve(f(a), f(b), f(c)),
            Op::Close => Op::Close,
        };
    }
}

/// A supply's arrow given as one of TikZ's (`monopoles/vcc/arrow={Latex[blue]}`): noted to be drawn
/// after the node, from its center to its `arrow tip`, instead of its own lines. `\\scaledwidth`
/// in it is the plain arrow's width, scaling with the node.
fn supply_arrow(cx: &mut Context, input: &ShapeInput, n: &Node, unit: f64) -> Result<bool, Error> {
    if !matches!(n.name, "vcc" | "vee") {
        return Ok(false);
    }
    let arrow = setting(cx, &format!("monopoles/{}/arrow", n.name))?;
    let arrow = arrow.trim();
    if arrow.is_empty() || arrow == "legacy" {
        cx.run(Tokens::text("\\gdef\\voltik@sa{}"))?;
        return Ok(false);
    }
    let width = 0.2 * unit * input.transform.factor();
    cx.run(Tokens::text(&format!(
        "\\gdef\\voltik@sa{{\\pgfextra{{\\def\\scaledwidth{{{width}cm}}\\xdef\\voltik@tn{{\\tikzlastnode}}\\draw[-{{{arrow}}}] (\\voltik@tn.center) -- (\\voltik@tn.arrow tip);}}}}"
    )))?;
    Ok(true)
}

/// A glyph's box, among a part's points.
fn bounds(ops: &[Op]) -> [f64; 4] {
    let mut b = [f64::MAX, f64::MIN, f64::MAX, f64::MIN];
    let mut see = |p: [f64; 2]| b = [b[0].min(p[0]), b[1].max(p[0]), b[2].min(p[1]), b[3].max(p[1])];
    for o in ops {
        match *o {
            Op::Move(x, y) | Op::Line(x, y) => see([x, y]),
            Op::Curve(_, _, c) => see(c),
            Op::Close => {}
        }
    }
    b
}

/// `amplifiers/plus`, `amplifiers/minus` (`amp plus`, `amp minus`) and `amp symbol font`: an
/// amplifier's measured signs to leave out, and the texts instead, centered where they were.
fn amplifier_signs(cx: &mut Context, n: &Node, sized: &Sized, unit: f64) -> Result<(Vec<Vec<Op>>, Vec<ShapeText>), Error> {
    if n.class() != Some("amplifiers") {
        return Ok((Vec::new(), Vec::new()));
    }
    let (plus, minus) = (setting(cx, "amplifiers/plus")?, setting(cx, "amplifiers/minus")?);
    let font = setting(cx, "amplifiers/symbol font")?;
    if plus.trim() == "default" && minus.trim() == "default" && font.trim().is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut parts = Vec::new();
    let mut texts = Vec::new();
    for (_, ops) in sized.parts.iter().filter(|(part, _)| matches!(part, Part::Solid(_))) {
        let [left, right, low, high] = bounds(ops);
        let (w, h) = (right - left, high - low);
        let text = match (w, h) {
            (w, h) if (0.14..0.2).contains(&w) && h < 0.03 => &minus,
            (w, h) if (0.14..0.2).contains(&w) && (0.14..0.2).contains(&h) => &plus,
            _ => continue,
        };
        parts.push(ops.clone());
        let text = match text.trim() {
            "default" if std::ptr::eq(text, &plus) => "$+$",
            "default" => "$-$",
            t => t,
        };
        if !text.is_empty() {
            let at = Point::new((left + right) / 2.0 * unit, (low + high) / 2.0 * unit);
            // (a `\\color` among the font commands goes around the text, which holds it)
            let (font, text) = match font.split_once("\\color{").and_then(|(before, rest)| rest.split_once('}').map(|(color, after)| (before, color, after))) {
                Some((before, color, after)) => (format!("{before}{after}"), format!("\\textcolor{{{color}}}{{{text}}}")),
                None => (font.clone(), text.to_string()),
            };
            let (font, text) = if font.contains("\\boldmath") { (font.replace("\\boldmath", ""), format!("{{\\boldmath {text}}}")) } else { (font, text) };
            let font = (!font.trim().is_empty()).then_some(font);
            texts.push(ShapeText { text, at, font, ..Default::default() });
        }
    }
    Ok((parts, texts))
}

/// The sizes of what is measured at one size: a Schottky base's hooks, a grid node's lines, a radial
/// stub's inner radius, a proximeter's lines and a transistor's arrow (`current arrow scale`).
fn sizes(cx: &mut Context, n: &Node, sized: &mut Sized) -> Result<(), Error> {
    if n.name.contains("schottky base") {
        let k = crate::bipole::number(cx, "tripoles/schottky base size", 0.05) / 0.05;
        for (_, ops) in sized.parts.iter_mut().filter(|(part, _)| matches!(part, Part::Thick(_))) {
            for i in 0..ops.len().saturating_sub(3) {
                if let [Op::Move(x, y), Op::Line(..), Op::Line(..), Op::Line(..)] = ops[i..i + 4] {
                    move_ops(&mut ops[i + 1..i + 4], |p| [x + k * (p[0] - x), y + k * (p[1] - y)]);
                }
            }
        }
    }
    // a double bipole's source: its plus up with rising-potential voltages
    if n.name == "double bipole" && matches!(setting(cx, "voltage dir")?.as_str(), "RP" | "old") {
        for (_, ops) in sized.parts.iter_mut().filter(|(part, _)| matches!(part, Part::Solid(_))) {
            move_ops(ops, |p| [p[0], -p[1]]);
        }
    }
    if n.name == "gridnode" {
        // `gridlines` lines each way, the middle ones on the diagonal, spaced by 2/(n+1) of its side
        let count = crate::bipole::number(cx, "blocks/gridnode/gridlines", 7.0).round().max(1.0);
        let (half, step) = (0.35, 2.0 * 0.7 / (count + 1.0));
        let mut ops = Vec::new();
        for i in 0..count as i32 {
            let t = (f64::from(i) - (count - 1.0) / 2.0) * step;
            let (a, b) = if t >= 0.0 { ([-half + t, half], [half, -half + t]) } else { ([-half, half + t], [half + t, -half]) };
            ops.extend([Op::Move(a[0], a[1]), Op::Line(b[0], b[1]), Op::Move(-a[0], a[1]), Op::Line(-b[0], b[1])]);
        }
        if let Some((_, lines)) = sized.parts.iter_mut().find(|(part, _)| matches!(part, Part::Thin(_))) {
            *lines = ops;
        }
    }
    if n.name == "msrstub" {
        let k = crate::bipole::number(cx, "monopoles/msrstub/radius", 0.125) / 0.125;
        for (_, ops) in sized.parts.iter_mut() {
            move_ops(ops, |p| if p[0].hypot(p[1]) < 0.15 { [k * p[0], k * p[1]] } else { p });
        }
    }
    if n.name == "proximeter" {
        // (the lines are `hlines position` of the way to the diamond's top, as long as fits)
        let at = crate::bipole::number(cx, "proximeter/hlines position", 0.3);
        let thickness = crate::bipole::number(cx, "proximeter/hlines thickness", 0.5);
        let (x, y) = (0.3 * (1.0 - at), 0.3 * at);
        if let Some(part) = sized.parts.iter_mut().find(|(part, _)| matches!(part, Part::Thin(_))) {
            *part = (Part::Stroke(2.0 * thickness, &[]), vec![Op::Move(-x, y), Op::Line(x, y), Op::Move(-x, -y), Op::Line(x, -y)]);
        }
    }
    // a MOSFET's arrow at the end (`transistors/arrow pos=end`): bigger, its tip on the channel's end
    if n.name.contains("arrowmos") && setting(cx, "transistors/arrow pos")? == "end" {
        let tip = sized.parts.iter().find_map(|(part, ops)| match (part, &ops[..]) {
            (Part::Solid(_), [Op::Move(tail, _), _, Op::Line(tip, y), _, Op::Close]) => Some((ops.clone(), *tail, *tip, *y)),
            _ => None,
        });
        if let Some((arrow, tail, tip, y)) = tip {
            sized.parts.retain(|(part, ops)| !(matches!(part, Part::Thin(_)) && *ops == arrow));
            let (tip, back) = if tip > tail { (0.005, -0.1239) } else { (-0.35, -0.2213) };
            if let Some((_, ops)) = sized.parts.iter_mut().find(|(_, ops)| *ops == arrow) {
                *ops = vec![Op::Move(back, y), Op::Line(back, y + 0.0577), Op::Line(tip, y), Op::Line(back, y - 0.0577), Op::Close];
            }
        }
    }
    // an American or-like port rounded, its nose a quarter of an ellipse (`american or shape=roundy`)
    if n.name.starts_with("american") && ["or", "nor", "xor", "xnor"].iter().any(|k| n.name.contains(&format!(" {k} port"))) && setting(cx, "american or shape")?.trim() == "roundy"
    {
        for (_, ops) in sized.parts.iter_mut().filter(|(part, _)| matches!(part, Part::Fill(_) | Part::Thick(_))) {
            for i in 0..ops.len().saturating_sub(1) {
                if let (Op::Curve(_, _, [tip, nose]), Op::Curve(_, _, [left, top])) = (ops[i], ops[i + 1]) {
                    if nose.abs() < 1e-3 && top > 0.1 && tip > left {
                        let along = left + 0.5523 * (tip - left);
                        let round = 0.5523 * top;
                        ops[i] = Op::Curve([along, -top], [tip, -round], [tip, 0.0]);
                        ops[i + 1] = Op::Curve([tip, round], [along, top], [left, top]);
                    }
                }
            }
        }
    }
    // an IEEE port's negation circle (`ieeestd ports/not radius`), and its xor's bar and leads
    if n.name.starts_with("ieeestd") {
        let k = 0.4;
        let radius = k * crate::bipole::number(cx, "ieeestd ports/not radius", 0.154);
        let circle = sized.parts.iter().find(|(_, ops)| is_circle(ops)).map(|(_, ops)| bounds(ops));
        if let Some([left, right, low, high]) = circle.filter(|_| (radius - k * 0.154).abs() > 1e-9) {
            let (grow, cy) = (radius / ((right - left) / 2.0), (low + high) / 2.0);
            for (_, ops) in sized.parts.iter_mut() {
                if is_circle(ops) {
                    move_ops(ops, |p| [left + grow * (p[0] - left), cy + grow * (p[1] - cy)]);
                } else if matches!(ops[..], [Op::Move(x, _), Op::Line(..)] if (x - right).abs() < 1e-3) {
                    if let Op::Move(x, _) = &mut ops[0] {
                        *x = left + 2.0 * radius;
                    }
                }
            }
        }
        if n.name.contains("xor") || n.name.contains("xnor") {
            let bar = k * (crate::bipole::number(cx, "ieeestd ports/xor bar distance", 0.192) - 0.192);
            let lead = k * (crate::bipole::number(cx, "ieeestd ports/xor leads in", 0.192) - 0.192);
            for (part, ops) in sized.parts.iter_mut() {
                if matches!(part, Part::Thick(_)) && ops.len() == 2 && matches!(ops[1], Op::Curve(..)) {
                    move_ops(ops, |p| [p[0] - bar, p[1]]);
                } else if matches!(part, Part::Thin(_)) {
                    for i in 0..ops.len().saturating_sub(1) {
                        if let (Op::Move(a, _), Op::Line(b, y)) = (ops[i], ops[i + 1]) {
                            if a.min(b) < -0.3 && b > a && y.abs() > 0.1 {
                                ops[i + 1] = Op::Line(b + lead, y);
                            } else if a.min(b) < -0.3 && a > b && y.abs() > 0.1 {
                                ops[i] = Op::Move(a + lead, y);
                            }
                        }
                    }
                }
            }
        }
    }
    // a HEMT's gate split in three around its lead (`tripoles/hemt/split gate`), or whole
    if n.name.starts_with("hemt") || n.name.starts_with("GaN hemt") {
        let split = setting(cx, "tripoles/hemt/split gate")?;
        let lead = sized.anchors.iter().find(|(name, _)| *name == "G").map_or(0.0, |(_, p)| p[1]);
        if !split.trim().is_empty() {
            if let Some((_, ops)) = sized.parts.iter_mut().find(|(part, _)| matches!(part, Part::Thick(_))) {
                let [x, _, low, high] = bounds(ops);
                *ops = if split.trim() == "true" {
                    let (near, far) = (0.055, 0.11);
                    vec![M(x, high), L(x, lead + far), M(x, lead + near), L(x, lead - near), M(x, lead - far), L(x, low)]
                } else {
                    vec![M(x, high), L(x, low)]
                };
            }
        }
    }
    // a transistor's solder dot, of `transistor solderdot scale` of a pole
    if n.name.contains("solderdot") {
        let k = crate::bipole::number(cx, "transistor solderdot scale", 0.7) / 0.7;
        for (_, ops) in sized.parts.iter_mut().filter(|(part, ops)| matches!(part, Part::Solid(_) | Part::Thin(_)) && is_circle(ops)) {
            let [left, right, low, high] = bounds(ops);
            let c = [(left + right) / 2.0, (low + high) / 2.0];
            if (right - left) < 0.1 {
                move_ops(ops, |p| [c[0] + k * (p[0] - c[0]), c[1] + k * (p[1] - c[1])]);
            }
        }
    }
    let arrow = crate::bipole::number(cx, "current arrow scale", 16.0);
    if transistors::TRANSISTORS.iter().any(|t| t.name == n.name) && arrow > 0.0 && (arrow - 16.0).abs() > 1e-9 {
        let k = 16.0 / arrow;
        for (_, ops) in sized.parts.iter_mut().filter(|(part, ops)| matches!(part, Part::Solid(_)) && ops.len() == 5 && matches!(ops[4], Op::Close)) {
            if let Op::Line(x, y) = ops[2] {
                move_ops(ops, |p| [x + k * (p[0] - x), y + k * (p[1] - y)]);
            }
        }
    }
    Ok(())
}

/// A MOSFET's gate: the leftmost of its two or more upright thick bars (a BJT has one).
fn gate_bar(sized: &Sized) -> Option<Vec<Op>> {
    let upright = |ops: &[Op]| !ops.is_empty() && ops.iter().all(|o| matches!(o, Op::Move(..) | Op::Line(..))) && (bounds(ops)[1] - bounds(ops)[0]).abs() < 1e-9;
    let bars: Vec<_> = sized.parts.iter().filter(|(part, ops)| matches!(part, Part::Thick(_)) && upright(ops)).collect();
    (bars.len() > 1).then(|| bars.iter().min_by(|a, b| bounds(&a.1)[0].total_cmp(&bounds(&b.1)[0])).map(|(_, ops)| ops.clone())).flatten()
}

/// `tripoles/igbt/outer base height`: the IGBT's outer bar (its leftmost thick line), of half height
/// 0.5812 of it, with the base of an `L` one, which enters at its end, following. Returns the bar.
fn igbt_outer_base(cx: &mut Context, sized: &mut Sized) -> Result<Vec<Op>, Error> {
    let bar = |ops: &[Op]| match *ops {
        [Op::Move(x, a), Op::Line(x2, b)] if (x - x2).abs() < 1e-9 && (a - b).abs() > 1e-9 => Some((x, a, b)),
        _ => None,
    };
    let found = sized.parts.iter().enumerate().filter_map(|(i, (part, ops))| if matches!(part, Part::Thick(_)) { bar(ops).map(|b| (i, b)) } else { None });
    let Some((i, (x, a, b))) = found.min_by(|p, q| p.1 .0.total_cmp(&q.1 .0)) else { return Ok(Vec::new()) };
    let height = setting(cx, "tripoles/igbt/outer base height")?;
    if let Ok(height) = height.trim().parse::<f64>() {
        let (mid, half, was) = ((a + b) / 2.0, 0.5812 * height, (a - b).abs() / 2.0);
        sized.parts[i].1 = vec![Op::Move(x, mid + half), Op::Line(x, mid - half)];
        // the base, where it is not in the middle
        let shift = half - was;
        let moved = |y: f64| y + (y - mid).signum() * shift;
        for (_, ops) in sized.parts.iter_mut() {
            if let [Op::Move(x0, y0), Op::Line(x1, y1)] = **ops {
                if (x0 - x).abs() < 1e-3 && (y0 - y1).abs() < 1e-9 && (y0 - mid).abs() > 1e-3 && x1 < x0 {
                    *ops = vec![Op::Move(x0, moved(y0)), Op::Line(x1, moved(y1))];
                }
            }
        }
        for (_, p) in sized.anchors.iter_mut() {
            if p[0] <= x + 1e-3 && (p[1] - mid).abs() > 1e-3 {
                p[1] = moved(p[1]);
            }
        }
    }
    Ok(sized.parts[i].1.clone())
}

/// `transistor bodydiode/scale` and `dot scale`: the diode (its triangle and bar) grows around its
/// middle, drawn at 0.3 and with dots of 0.7, and its wires reach it. Returns the parts of the diode,
/// which `color`, `dash` and `relative thickness` style.
fn body_diode(cx: &mut Context, sized: &mut Sized) -> Result<Vec<Vec<Op>>, Error> {
    let triangle = |ops: &[Op]| ops.len() == 4 && matches!(ops[3], Op::Close) && ops[1..3].iter().all(|o| matches!(o, Op::Line(..)));
    let Some((_, tip)) = sized.parts.iter().find(|(part, ops)| matches!(part, Part::Thick(_)) && triangle(ops)) else { return Ok(Vec::new()) };
    let points: Vec<[f64; 2]> = tip.iter().filter_map(|o| if let Op::Move(x, y) | Op::Line(x, y) = *o { Some([x, y]) } else { None }).collect();
    let bound = |i: usize, f: fn(f64, f64) -> f64, start: f64| points.iter().map(|p| p[i]).fold(start, f);
    let (left, right) = (bound(0, f64::min, f64::MAX), bound(0, f64::max, f64::MIN));
    let (low, high) = (bound(1, f64::min, f64::MAX), bound(1, f64::max, f64::MIN));
    let (mx, my, half) = ((left + right) / 2.0, (low + high) / 2.0, (high - low) / 2.0);
    let k = crate::bipole::number(cx, "transistor bodydiode/scale", 0.3) / 0.3;
    let dots = crate::bipole::number(cx, "transistor bodydiode/dot scale", 0.7) / 0.7;
    let near = |a: f64, b: f64| (a - b).abs() < 1e-3;
    let inside = |p: [f64; 2]| p[0] >= left - 1e-3 && p[0] <= right + 1e-3 && p[1] >= low - 1e-3 && p[1] <= high + 1e-3;
    let mut diode = Vec::new();
    sized.parts.retain_mut(|(_, ops)| {
        let last = ops.iter().rev().find_map(|o| if let Op::Line(x, y) = *o { Some([x, y]) } else { None });
        let all = ops.iter().all(|o| match *o {
            Op::Move(x, y) | Op::Line(x, y) => inside([x, y]),
            Op::Curve(..) => false,
            Op::Close => true,
        });
        if all {
            move_ops(ops, |p| [mx + k * (p[0] - mx), my + k * (p[1] - my)]);
        } else if is_circle(ops) {
            let (xs, ys): (Vec<f64>, Vec<f64>) = ops.iter().filter_map(|o| if let Op::Curve(_, _, c) = *o { Some((c[0], c[1])) } else { None }).unzip();
            let (x, y) = (xs.iter().sum::<f64>() / xs.len() as f64, ys.iter().sum::<f64>() / ys.len() as f64);
            let r = xs.iter().map(|v| (v - x).abs()).fold(0.0, f64::max);
            if r > 0.05 {
                return true;
            }
            if dots <= 0.0 {
                return false;
            }
            move_ops(ops, |p| [x + dots * (p[0] - x), y + dots * (p[1] - y)]);
        } else if last.is_some_and(|p| near(p[0], mx) && near((p[1] - my).abs(), half)) {
            let end = ops.len() - 1;
            if let Op::Line(x, y) = ops[end] {
                ops[end] = Op::Line(x, my + k * (y - my));
            }
        } else {
            return true;
        }
        diode.push(ops.clone());
        true
    });
    let reach = [k * (right - mx), k * half];
    sized.extent.extend([[mx - reach[0], my - reach[1]], [mx + reach[0], my + reach[1]]]);
    Ok(diode)
}

/// `transistor circle/partial borders` and `tubes/partial borders`: each part of the border, clockwise
/// from the top (a tube wider than tall from its left), left out (0), drawn (1) or dashed (2) with
/// its `partial border dash`.
fn partial_borders(cx: &mut Context, paths: Vec<ShapePath>, circled: bool, n: &Node) -> Result<Vec<ShapePath>, Error> {
    let tube = n.class() == Some("tubes");
    if !circled && !tube {
        return Ok(paths);
    }
    let class = if tube { "tubes" } else { "transistor circle" };
    let spec = setting(cx, &format!("{class}/partial borders"))?;
    let spec = spec.trim();
    if spec.is_empty() || spec == "none" {
        return Ok(paths);
    }
    let pattern = setting(cx, &format!("{class}/partial border dash"))?;
    let pattern = dash(cx, &pattern)?;
    let mut out = Vec::new();
    for path in paths {
        let curves: Vec<[Point; 4]> = {
            let mut at = Point::new(0.0, 0.0);
            let mut found = Vec::new();
            for s in &path.segments {
                match *s {
                    PathSegment::MoveTo(p) | PathSegment::LineTo(p) => at = p,
                    PathSegment::CurveTo(a, b, c) => {
                        found.push([at, a, b, c]);
                        at = c;
                    }
                    PathSegment::Close => {}
                }
            }
            found
        };
        let border = matches!(path.paint, ShapePaint::Stroke(_)) && curves.len() == 4 && matches!(path.segments.last(), Some(PathSegment::Close));
        if !border {
            out.push(path);
            continue;
        }
        let curve = |c: [Point; 4]| vec![PathSegment::MoveTo(c[0]), PathSegment::CurveTo(c[1], c[2], c[3])];
        let back = |c: [Point; 4]| curve([c[3], c[2], c[1], c[0]]);
        let line = |a: Point, b: Point| vec![PathSegment::MoveTo(a), PathSegment::LineTo(b)];
        let [c1, c2, c3, c4] = [curves[0], curves[1], curves[2], curves[3]];
        let (side, other) = (line(c2[3], c3[0]), line(c4[3], c1[0]));
        let parts = if !tube {
            // (the circle goes the other way, from its right)
            vec![back(c1), back(c4), back(c3), back(c2)]
        } else if n.name.contains("~wide") {
            vec![curve(c4), other, curve(c1), curve(c2), side, curve(c3)]
        } else {
            vec![curve(c2), side, curve(c3), curve(c4), other, curve(c1)]
        };
        for (segments, how) in parts.into_iter().zip(spec.chars()) {
            match how {
                '1' => out.push(ShapePath { segments, ..path.clone() }),
                '2' => out.push(ShapePath { segments, dash: pattern.clone(), ..path.clone() }),
                _ => {}
            }
        }
    }
    Ok(out)
}

fn scale_circle(sized: &mut Sized, k: f64) {
    let Some((left, right, cy)) = circle_of(sized) else { return };
    let pivot = [left, cy];
    let grow = |p: [f64; 2]| [pivot[0] + k * (p[0] - pivot[0]), pivot[1] + k * (p[1] - pivot[1])];
    for (_, ops) in sized.parts.iter_mut().filter(|(_, ops)| is_circle(ops)) {
        for o in ops.iter_mut() {
            *o = match *o {
                Op::Move(x, y) => {
                    let q = grow([x, y]);
                    Op::Move(q[0], q[1])
                }
                Op::Line(x, y) => {
                    let q = grow([x, y]);
                    Op::Line(q[0], q[1])
                }
                Op::Curve(a, b, c) => Op::Curve(grow(a), grow(b), grow(c)),
                Op::Close => Op::Close,
            };
        }
    }
    for (name, p) in sized.anchors.iter_mut() {
        if name.starts_with("circle") {
            *p = grow(*p);
        }
    }
    let r = (right - left) / 2.0 * k;
    sized.extent.extend([[pivot[0], pivot[1] - r], [pivot[0] + 2.0 * r, pivot[1] + r]]);
}

/// A node drawn with its sizes: its parts, box, anchors, extent and the shift of its text.
struct Sized {
    parts: Vec<(Part, Vec<Op>)>,
    bounds: [f64; 4],
    anchors: Vec<(&'static str, [f64; 2])>,
    extent: Vec<[f64; 2]>,
    text_shift: [f64; 2],
}

/// The coefficient and the node of size `setting`, if it changes.
fn others_k(n: &Node, setting: &str, others: &[(f64, &'static Node)]) -> Option<(f64, &'static Node)> {
    let p = n.params.iter().find(|p| p.setting == setting)?;
    others.iter().find(|(_, o)| o.name == p.name).copied()
}

fn sized(cx: &mut Context, n: &'static Node) -> Result<Sized, Error> {
    let mut others: Vec<(f64, &'static Node)> = Vec::new();
    for p in n.params {
        let value = crate::bipole::number(cx, p.setting, p.default);
        if (value - p.default).abs() > 1e-9 {
            if let Some(o) = find(p.name) {
                others.push(((value - p.default) / p.step, o));
            }
        }
    }
    // a pair that changes it together adds what it does more than each on its own:
    // ka kb ((both - base) - (a - base) - (b - base))
    for x in n.crosses {
        let k = |setting: &str| others_k(n, setting, &others);
        if let (Some((ka, a)), Some((kb, b)), Some(both)) = (k(x.a), k(x.b), find(x.name)) {
            others.extend([(ka * kb, both), (-ka * kb, a), (-ka * kb, b)]);
        }
    }
    let mix2 = |base: [f64; 2], f: &dyn Fn(&Node) -> Option<[f64; 2]>| {
        let mut v = base;
        for (k, o) in &others {
            if let Some(q) = f(o) {
                v = [v[0] + k * (q[0] - base[0]), v[1] + k * (q[1] - base[1])];
            }
        }
        v
    };
    let parts = n
        .parts
        .iter()
        .enumerate()
        .map(|(i, part)| {
            let with: Vec<(f64, &[Op])> = others.iter().filter_map(|(k, o)| o.parts.get(i).map(|q| (*k, q.ops()))).filter(|(_, q)| q.len() == part.ops().len()).collect();
            (*part, mix_ops(part.ops(), &with))
        })
        .collect();
    let mut bounds = n.bounds;
    for (k, o) in &others {
        for (b, (q, p)) in bounds.iter_mut().zip(o.bounds.iter().zip(n.bounds)) {
            *b += k * (q - p);
        }
    }
    let anchor = |n: &Node, name: &str| n.anchors.iter().find(|(a, _)| *a == name).map(|(_, p)| *p);
    let anchors = n.anchors.iter().map(|(name, p)| (*name, mix2(*p, &|o: &Node| anchor(o, name)))).collect();
    let extent = n.extent.iter().enumerate().map(|(i, p)| mix2(*p, &|o: &Node| o.extent.get(i).copied())).collect();
    let text = anchor(n, "text").unwrap_or([0.0, 0.0]);
    let moved = mix2(text, &|o: &Node| anchor(o, "text"));
    Ok(Sized { parts, bounds, anchors, extent, text_shift: [moved[0] - text[0], moved[1] - text[1]] })
}

impl Node {
    /// The class whose `scale` it follows, if any.
    pub fn class(&self) -> Option<&'static str> {
        match self.name {
            "vcc" | "vee" => Some("power supplies"),
            "bnc" | "iecconnshape" | "iecsocketL" | "iecsocketR" | "iecplugL" | "iecplugR" => Some("connectors"),
            n if n.ends_with("ground") || n == "eground2" => Some("grounds"),
            n if n.contains("spdt") || n == "proximeter" => Some("switches"),
            n if transistors::TRANSISTORS.iter().any(|t| t.name == n) => Some("transistors"),
            n if tubes::TUBES.iter().any(|t| t.name == n) => Some("tubes"),
            n if amplifiers::AMPLIFIERS.iter().any(|t| t.name == n) => Some("amplifiers"),
            n if logic::LOGIC.iter().any(|t| t.name == n) => Some("logic ports"),
            n if blocks::class(n).is_some() => blocks::class(n),
            _ => None,
        }
    }
}

/// A circle as four Béziers, around `(x, y)`, `r` across the radius.
macro_rules! circle {
    ($x:expr, $y:expr, $r:expr) => {
        &[
            M($x + $r, $y),
            C([$x + $r, $y + 0.5523 * $r], [$x + 0.5523 * $r, $y + $r], [$x, $y + $r]),
            C([$x - 0.5523 * $r, $y + $r], [$x - $r, $y + 0.5523 * $r], [$x - $r, $y]),
            C([$x - $r, $y - 0.5523 * $r], [$x - 0.5523 * $r, $y - $r], [$x, $y - $r]),
            C([$x + 0.5523 * $r, $y - $r], [$x + $r, $y - 0.5523 * $r], [$x + $r, $y]),
            Z,
        ]
    };
}

/// The three lines of a ground, from `y` down.
macro_rules! ground_lines {
    ($y:expr) => {
        &[M(-0.15, $y), L(0.15, $y), M(-0.1, $y - 0.05), L(0.1, $y - 0.05), M(-0.0625, $y - 0.1), L(0.0625, $y - 0.1)]
    };
}

const STEM: &[Op] = &[M(0.0, 0.0), L(0.0, -0.25)];
const IEC_PLUG: &[Op] = &[M(0.0, 0.05), L(0.2, 0.05), L(0.2, -0.05), L(0.0, -0.05), Z];

/// Every node component voltik draws (the terminals apart, sized by `nodes width`).
pub const NODES: &[Node] = &[
    node("ground", &[Part::Thin(&[M(0.0, 0.0), L(0.0, -0.3)]), Part::Thick(ground_lines!(-0.3))], [-0.15, 0.15, -0.4, 0.0]),
    node("tlground", &[Part::Thick(ground_lines!(0.0))], [-0.15, 0.15, -0.1, 0.0]),
    node("rground", &[Part::Thin(STEM), Part::Thick(&[M(-0.15, -0.25), L(0.15, -0.25)])], [-0.15, 0.15, -0.25, 0.0]),
    node(
        "sground",
        &[Part::Thin(STEM), Part::Fill(&[M(-0.15, -0.25), L(0.15, -0.25), L(0.0, -0.45), Z]), Part::Thick(&[M(-0.15, -0.25), L(0.15, -0.25), L(0.0, -0.45), Z])],
        [-0.15, 0.15, -0.45, 0.0],
    ),
    node("tground", &[Part::Thicker(&[M(-0.15, 0.0), L(0.15, 0.0)])], [-0.15, 0.15, 0.0, 0.0]),
    node(
        "nground",
        &[
            Part::Thin(&[M(0.0, 0.0), L(0.0, -0.3)]),
            Part::Thick(ground_lines!(-0.3)),
            Part::Thick(&[M(0.225, -0.4), C([0.225, -0.2757], [0.1242, -0.175], [0.0, -0.175]), C([-0.1243, -0.175], [-0.225, -0.2757], [-0.225, -0.4])]),
        ],
        [-0.225, 0.225, -0.4, 0.0],
    ),
    node(
        "pground",
        &[Part::Fill(circle!(0.0, -0.225, 0.225)), Part::Thick(circle!(0.0, -0.225, 0.225)), Part::Thin(STEM), Part::Thick(ground_lines!(-0.25))],
        [-0.225, 0.225, -0.45, 0.0],
    ),
    node(
        "cground",
        &[
            Part::Thin(&[M(0.0, 0.0), L(0.0, -0.375)]),
            Part::Thick(&[M(-0.25, -0.525), L(-0.1875, -0.375), L(0.1875, -0.375), L(0.125, -0.525), M(0.0, -0.375), L(-0.0625, -0.525)]),
        ],
        [-0.25, 0.25, -0.5, 0.0],
    ),
    node(
        "eground",
        &[
            Part::Thin(STEM),
            Part::Thicker(&[M(-0.25, -0.25), L(0.25, -0.25)]),
            Part::Thin(&[M(-0.275, -0.425), L(-0.15, -0.25), M(-0.15, -0.425), L(-0.025, -0.25), M(-0.025, -0.425), L(0.1, -0.25), M(0.1, -0.425), L(0.225, -0.25)]),
        ],
        [-0.275, 0.275, -0.425, 0.0],
    ),
    node(
        "eground2",
        &[
            Part::Thin(STEM),
            Part::Thicker(&[M(-0.25, -0.25), L(0.25, -0.25)]),
            Part::Thin(&[M(-0.275, -0.425), L(-0.1, -0.25), M(-0.1125, -0.425), L(0.0625, -0.25), M(0.05, -0.425), L(0.225, -0.25)]),
        ],
        [-0.275, 0.275, -0.425, 0.0],
    ),
    Node {
        text: Text::Above([0.0, 0.4]),
        anchors: &[("text", [0.0, 0.4])],
        text_counts: true,
        extent: &[[0.0, 0.4]],
        ..node("vcc", &[Part::Thick(&[M(-0.1, 0.1601), L(0.0, 0.3), L(0.1, 0.1601)]), Part::Thin(&[M(0.0, 0.0), L(0.0, 0.3)])], [-0.1, 0.1, 0.0, 0.3])
    },
    Node {
        text: Text::Below([0.0, -0.4]),
        anchors: &[("text", [0.0, -0.4])],
        text_counts: true,
        extent: &[[0.0, -0.4]],
        ..node("vee", &[Part::Thick(&[M(-0.1, -0.1601), L(0.0, -0.3), L(0.1, -0.1601)]), Part::Thin(&[M(0.0, 0.0), L(0.0, -0.3)])], [-0.1, 0.1, -0.3, 0.0])
    },
    // connectors
    Node {
        text: Text::Above([-0.1, 0.1]),
        anchors: &[("text", [-0.1, 0.1]), ("hot", [0.0, 0.0]), ("zero", [-0.1, 0.0]), ("shield", [-0.1, -0.1])],
        // its picture is a square around its circle, twice as wide
        extent: &[[-0.3, -0.2], [0.1, 0.2]],
        ..node(
            "bnc",
            &[Part::Fill(circle!(-0.1, 0.0, 0.1)), Part::Thick(circle!(-0.1, 0.0, 0.1)), Part::Thick(circle!(-0.1, 0.0, 0.04)), Part::Thin(&[M(-0.06, 0.0), L(0.0, 0.0)])],
            [-0.2, 0.0, -0.1, 0.1],
        )
    },
    Node {
        text: Text::Above([0.0, 0.1]),
        anchors: &[("text", [0.0, 0.1]), ("plug center", [0.05, 0.0]), ("socket center", [-0.05, 0.0])],
        ..node(
            "iecconnshape",
            &[
                Part::Thick(&[M(-0.05, 0.1), C([-0.1052, 0.1], [-0.15, 0.0552], [-0.15, 0.0]), C([-0.15, -0.0552], [-0.1052, -0.1], [-0.05, -0.1])]),
                Part::Solid(&[M(-0.05, 0.05), L(0.15, 0.05), L(0.15, -0.05), L(-0.05, -0.05), Z]),
                Part::Thick(&[M(-0.05, 0.05), L(0.15, 0.05), L(0.15, -0.05), L(-0.05, -0.05), Z]),
            ],
            [-0.15, 0.15, -0.1, 0.1],
        )
    },
    Node {
        text: Text::AboveLeft([0.0, 0.1]),
        anchors: &[("text", [0.0, 0.1]), ("socket center", [0.1, 0.0])],
        ..node("iecsocketL", &[Part::Thick(&[M(0.1, 0.1), C([0.0448, 0.1], [0.0, 0.0552], [0.0, 0.0]), C([0.0, -0.0552], [0.0448, -0.1], [0.1, -0.1])])], [0.0, 0.1, -0.1, 0.1])
    },
    Node {
        text: Text::AboveRight([0.2, 0.1]),
        anchors: &[("text", [0.2, 0.1]), ("plug center", [0.1, 0.0])],
        ..node("iecplugR", &[Part::Solid(IEC_PLUG), Part::Thick(IEC_PLUG)], [0.0, 0.2, -0.05, 0.05])
    },
    Node {
        text: Text::AboveLeft([0.0, 0.1]),
        anchors: &[("text", [0.0, 0.1]), ("plug center", [0.1, 0.0])],
        ..node("iecplugL", &[Part::Solid(IEC_PLUG), Part::Thick(IEC_PLUG)], [0.0, 0.2, -0.05, 0.05])
    },
    Node {
        text: Text::AboveRight([0.1, 0.1]),
        anchors: &[("text", [0.1, 0.1]), ("socket center", [0.0, 0.0])],
        ..node("iecsocketR", &[Part::Thick(&[M(0.0, 0.1), C([0.0552, 0.1], [0.1, 0.0552], [0.1, 0.0]), C([0.1, -0.0552], [0.0552, -0.1], [0.0, -0.1])])], [0.0, 0.1, -0.1, 0.1])
    },
    // crossings
    node(
        "jump crossing",
        &[
            Part::Thin(&[M(-0.1, 0.0), L(-0.04, 0.0), C([-0.04, 0.0221], [-0.0221, 0.04], [0.0, 0.04]), C([0.0221, 0.04], [0.04, 0.0221], [0.04, 0.0]), L(0.1, 0.0)]),
            Part::Thin(&[M(0.0, 0.1), L(0.0, 0.05), M(0.0, 0.03), L(0.0, -0.1)]),
        ],
        [-0.1, 0.1, -0.1, 0.1],
    ),
    node("plain crossing", &[Part::Thin(&[M(-0.1, 0.0), L(0.1, 0.0)]), Part::Thin(&[M(0.0, 0.1), L(0.0, 0.0101), M(0.0, -0.0101), L(0.0, -0.1)])], [-0.1, 0.1, -0.1, 0.1]),
    // arrows
    Node { anchors: &[("tip", [0.0625, 0.0])], ..node("currarrow", &[Part::Solid(CURRARROW), Part::Thin(CURRARROW)], [-0.0312, 0.0312, -0.0312, 0.0312]) },
    node("inputarrow", &[Part::Solid(&[M(-0.1062, 0.0), L(-0.1062, -0.05), L(0.0, 0.0), L(-0.1062, 0.05), Z])], [-0.1062, 0.0, -0.0312, 0.0312]),
    Node {
        anchors: &[("tip", [0.08, 0.0]), ("btip", [0.075, 0.0])],
        ..node("trarrow", &[Part::Solid(&[M(-0.0487, 0.0), L(-0.0487, 0.0576), L(0.08, 0.0), L(-0.0487, -0.0577), Z])], [-0.0488, 0.0488, -0.0576, 0.0576])
    },
    Node {
        text: Text::Above([0.0, 0.0]),
        anchors: &[("tip", [0.0625, 0.0])],
        ..node("flowarrow", &[Part::Thin(&[M(-0.25, 0.0), L(0.25, 0.0)]), Part::Solid(FLOWHEAD), Part::Thin(FLOWHEAD)], [-0.25, 0.25, -0.0312, 0.0312])
    },
    // switches
    Node {
        params: &[Param { setting: "bipoles/cuteswitch/height", default: 0.6, step: 0.2, name: "cute spdt down@height" }],
        anchors: &[("in", [-0.2525, 0.0]), ("out 1", [0.2525, 0.3]), ("out 2", [0.2525, -0.3]), ("mid", [0.0, -0.12])],
        ..node(
            "cute spdt down",
            &[
                Part::Stroke(7.9653, &[M(-0.2125, 0.0), L(0.2125, -0.24)]),
                Part::Pole(&[
                    M(0.2525, 0.3),
                    C([0.2525, 0.3221], [0.2346, 0.34], [0.2125, 0.34]),
                    C([0.1905, 0.34], [0.1726, 0.3221], [0.1726, 0.3]),
                    C([0.1726, 0.2779], [0.1905, 0.2599], [0.2125, 0.2599]),
                    C([0.2346, 0.2599], [0.2525, 0.2779], [0.2525, 0.3]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, 0.3),
                    C([0.2525, 0.3221], [0.2346, 0.34], [0.2125, 0.34]),
                    C([0.1905, 0.34], [0.1726, 0.3221], [0.1726, 0.3]),
                    C([0.1726, 0.2779], [0.1905, 0.2599], [0.2125, 0.2599]),
                    C([0.2346, 0.2599], [0.2525, 0.2779], [0.2525, 0.3]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, -0.3),
                    C([0.2525, -0.278], [0.2346, -0.2601], [0.2125, -0.2601]),
                    C([0.1905, -0.2601], [0.1726, -0.278], [0.1726, -0.3]),
                    C([0.1726, -0.3221], [0.1905, -0.34], [0.2125, -0.34]),
                    C([0.2346, -0.34], [0.2525, -0.3221], [0.2525, -0.3]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, -0.3),
                    C([0.2525, -0.278], [0.2346, -0.2601], [0.2125, -0.2601]),
                    C([0.1905, -0.2601], [0.1726, -0.278], [0.1726, -0.3]),
                    C([0.1726, -0.3221], [0.1905, -0.34], [0.2125, -0.34]),
                    C([0.2346, -0.34], [0.2525, -0.3221], [0.2525, -0.3]),
                    Z,
                ]),
                Part::Pole(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
                Part::Thin(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
            ],
            [-0.2125, 0.2125, -0.3, 0.3],
        )
    },
    Node {
        params: &[Param { setting: "bipoles/cuteswitch/height", default: 0.6, step: 0.2, name: "cute spdt down arrow@height" }],
        anchors: &[("in", [-0.2525, 0.0]), ("out 1", [0.2525, 0.3]), ("out 2", [0.2525, -0.3]), ("mid", [0.0, -0.12])],
        ..node(
            "cute spdt down arrow",
            &[
                Part::Stroke(7.9653, &[M(-0.2125, 0.0), L(0.2125, -0.24)]),
                Part::Thick(&[M(-0.0076, -0.2442), C([0.1273, -0.131], [0.1449, 0.07], [0.0317, 0.2049]), C([0.0126, 0.2276], [-0.0094, 0.2474], [-0.0339, 0.264])]),
                Part::Solid(&[
                    M(-0.1034, 0.2995),
                    C([-0.0791, 0.2768], [-0.036, 0.2348], [-0.0182, 0.1892]),
                    C([-0.0271, 0.2369], [-0.0133, 0.2591], [0.0333, 0.2724]),
                    C([-0.0154, 0.2681], [-0.0722, 0.2879], [-0.1034, 0.2995]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, 0.3),
                    C([0.2525, 0.3221], [0.2346, 0.34], [0.2125, 0.34]),
                    C([0.1905, 0.34], [0.1726, 0.3221], [0.1726, 0.3]),
                    C([0.1726, 0.2779], [0.1905, 0.2599], [0.2125, 0.2599]),
                    C([0.2346, 0.2599], [0.2525, 0.2779], [0.2525, 0.3]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, 0.3),
                    C([0.2525, 0.3221], [0.2346, 0.34], [0.2125, 0.34]),
                    C([0.1905, 0.34], [0.1726, 0.3221], [0.1726, 0.3]),
                    C([0.1726, 0.2779], [0.1905, 0.2599], [0.2125, 0.2599]),
                    C([0.2346, 0.2599], [0.2525, 0.2779], [0.2525, 0.3]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, -0.3),
                    C([0.2525, -0.278], [0.2346, -0.2601], [0.2125, -0.2601]),
                    C([0.1905, -0.2601], [0.1726, -0.278], [0.1726, -0.3]),
                    C([0.1726, -0.3221], [0.1905, -0.34], [0.2125, -0.34]),
                    C([0.2346, -0.34], [0.2525, -0.3221], [0.2525, -0.3]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, -0.3),
                    C([0.2525, -0.278], [0.2346, -0.2601], [0.2125, -0.2601]),
                    C([0.1905, -0.2601], [0.1726, -0.278], [0.1726, -0.3]),
                    C([0.1726, -0.3221], [0.1905, -0.34], [0.2125, -0.34]),
                    C([0.2346, -0.34], [0.2525, -0.3221], [0.2525, -0.3]),
                    Z,
                ]),
                Part::Pole(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
                Part::Thin(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
            ],
            [-0.2125, 0.2125, -0.3, 0.3],
        )
    },
    Node {
        params: &[Param { setting: "bipoles/cuteswitch/height", default: 0.6, step: 0.2, name: "cute spdt mid@height" }],
        anchors: &[("in", [-0.2525, 0.0]), ("out 1", [0.2525, 0.3]), ("out 2", [0.2525, -0.3]), ("mid", [0.0, 0.0])],
        ..node(
            "cute spdt mid",
            &[
                Part::Stroke(7.9653, &[M(-0.2125, 0.0), L(0.2125, 0.0)]),
                Part::Pole(&[
                    M(0.2525, 0.3),
                    C([0.2525, 0.3221], [0.2346, 0.34], [0.2125, 0.34]),
                    C([0.1905, 0.34], [0.1726, 0.3221], [0.1726, 0.3]),
                    C([0.1726, 0.2779], [0.1905, 0.2599], [0.2125, 0.2599]),
                    C([0.2346, 0.2599], [0.2525, 0.2779], [0.2525, 0.3]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, 0.3),
                    C([0.2525, 0.3221], [0.2346, 0.34], [0.2125, 0.34]),
                    C([0.1905, 0.34], [0.1726, 0.3221], [0.1726, 0.3]),
                    C([0.1726, 0.2779], [0.1905, 0.2599], [0.2125, 0.2599]),
                    C([0.2346, 0.2599], [0.2525, 0.2779], [0.2525, 0.3]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, -0.3),
                    C([0.2525, -0.278], [0.2346, -0.2601], [0.2125, -0.2601]),
                    C([0.1905, -0.2601], [0.1726, -0.278], [0.1726, -0.3]),
                    C([0.1726, -0.3221], [0.1905, -0.34], [0.2125, -0.34]),
                    C([0.2346, -0.34], [0.2525, -0.3221], [0.2525, -0.3]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, -0.3),
                    C([0.2525, -0.278], [0.2346, -0.2601], [0.2125, -0.2601]),
                    C([0.1905, -0.2601], [0.1726, -0.278], [0.1726, -0.3]),
                    C([0.1726, -0.3221], [0.1905, -0.34], [0.2125, -0.34]),
                    C([0.2346, -0.34], [0.2525, -0.3221], [0.2525, -0.3]),
                    Z,
                ]),
                Part::Pole(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
                Part::Thin(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
            ],
            [-0.2125, 0.2125, -0.3, 0.3],
        )
    },
    Node {
        params: &[Param { setting: "bipoles/cuteswitch/height", default: 0.6, step: 0.2, name: "cute spdt mid arrow@height" }],
        anchors: &[("in", [-0.2525, 0.0]), ("out 1", [0.2525, 0.3]), ("out 2", [0.2525, -0.3]), ("mid", [0.0, 0.0])],
        ..node(
            "cute spdt mid arrow",
            &[
                Part::Stroke(7.9653, &[M(-0.2125, 0.0), L(0.2125, 0.0)]),
                Part::Thick(&[M(0.0094, -0.2289), C([0.112, -0.1296], [0.1382, 0.0301], [0.0636, 0.1594]), C([0.0487, 0.1851], [0.0304, 0.2085], [0.0093, 0.2289])]),
                Part::Solid(&[
                    M(-0.0531, -0.2761),
                    C([-0.0244, -0.2591], [0.0281, -0.2299], [0.0769, -0.2257]),
                    C([0.0286, -0.2206], [0.0112, -0.2012], [0.0117, -0.1526]),
                    C([0.0021, -0.2006], [-0.0331, -0.2494], [-0.0531, -0.2761]),
                    Z,
                ]),
                Part::Solid(&[
                    M(-0.0531, 0.276),
                    C([-0.0331, 0.2494], [0.0021, 0.2006], [0.0117, 0.1526]),
                    C([0.0112, 0.2011], [0.0286, 0.2206], [0.0769, 0.2257]),
                    C([0.0281, 0.2299], [-0.0244, 0.2591], [-0.0531, 0.276]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, 0.3),
                    C([0.2525, 0.3221], [0.2346, 0.34], [0.2125, 0.34]),
                    C([0.1905, 0.34], [0.1726, 0.3221], [0.1726, 0.3]),
                    C([0.1726, 0.2779], [0.1905, 0.2599], [0.2125, 0.2599]),
                    C([0.2346, 0.2599], [0.2525, 0.2779], [0.2525, 0.3]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, 0.3),
                    C([0.2525, 0.3221], [0.2346, 0.34], [0.2125, 0.34]),
                    C([0.1905, 0.34], [0.1726, 0.3221], [0.1726, 0.3]),
                    C([0.1726, 0.2779], [0.1905, 0.2599], [0.2125, 0.2599]),
                    C([0.2346, 0.2599], [0.2525, 0.2779], [0.2525, 0.3]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, -0.3),
                    C([0.2525, -0.278], [0.2346, -0.2601], [0.2125, -0.2601]),
                    C([0.1905, -0.2601], [0.1726, -0.278], [0.1726, -0.3]),
                    C([0.1726, -0.3221], [0.1905, -0.34], [0.2125, -0.34]),
                    C([0.2346, -0.34], [0.2525, -0.3221], [0.2525, -0.3]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, -0.3),
                    C([0.2525, -0.278], [0.2346, -0.2601], [0.2125, -0.2601]),
                    C([0.1905, -0.2601], [0.1726, -0.278], [0.1726, -0.3]),
                    C([0.1726, -0.3221], [0.1905, -0.34], [0.2125, -0.34]),
                    C([0.2346, -0.34], [0.2525, -0.3221], [0.2525, -0.3]),
                    Z,
                ]),
                Part::Pole(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
                Part::Thin(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
            ],
            [-0.2125, 0.2125, -0.3, 0.3],
        )
    },
    Node {
        params: &[Param { setting: "bipoles/cuteswitch/height", default: 0.6, step: 0.2, name: "cute spdt up@height" }],
        anchors: &[("in", [-0.2525, 0.0]), ("out 1", [0.2525, 0.3]), ("out 2", [0.2525, -0.3]), ("mid", [0.0, 0.12])],
        ..node(
            "cute spdt up",
            &[
                Part::Stroke(7.9653, &[M(-0.2125, 0.0), L(0.2125, 0.24)]),
                Part::Pole(&[
                    M(0.2525, 0.3),
                    C([0.2525, 0.3221], [0.2346, 0.34], [0.2125, 0.34]),
                    C([0.1905, 0.34], [0.1726, 0.3221], [0.1726, 0.3]),
                    C([0.1726, 0.2779], [0.1905, 0.2599], [0.2125, 0.2599]),
                    C([0.2346, 0.2599], [0.2525, 0.2779], [0.2525, 0.3]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, 0.3),
                    C([0.2525, 0.3221], [0.2346, 0.34], [0.2125, 0.34]),
                    C([0.1905, 0.34], [0.1726, 0.3221], [0.1726, 0.3]),
                    C([0.1726, 0.2779], [0.1905, 0.2599], [0.2125, 0.2599]),
                    C([0.2346, 0.2599], [0.2525, 0.2779], [0.2525, 0.3]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, -0.3),
                    C([0.2525, -0.278], [0.2346, -0.2601], [0.2125, -0.2601]),
                    C([0.1905, -0.2601], [0.1726, -0.278], [0.1726, -0.3]),
                    C([0.1726, -0.3221], [0.1905, -0.34], [0.2125, -0.34]),
                    C([0.2346, -0.34], [0.2525, -0.3221], [0.2525, -0.3]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, -0.3),
                    C([0.2525, -0.278], [0.2346, -0.2601], [0.2125, -0.2601]),
                    C([0.1905, -0.2601], [0.1726, -0.278], [0.1726, -0.3]),
                    C([0.1726, -0.3221], [0.1905, -0.34], [0.2125, -0.34]),
                    C([0.2346, -0.34], [0.2525, -0.3221], [0.2525, -0.3]),
                    Z,
                ]),
                Part::Pole(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
                Part::Thin(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
            ],
            [-0.2125, 0.2125, -0.3, 0.3],
        )
    },
    Node {
        params: &[Param { setting: "bipoles/cuteswitch/height", default: 0.6, step: 0.2, name: "cute spdt up arrow@height" }],
        anchors: &[("in", [-0.2525, 0.0]), ("out 1", [0.2525, 0.3]), ("out 2", [0.2525, -0.3]), ("mid", [0.0, 0.12])],
        ..node(
            "cute spdt up arrow",
            &[
                Part::Stroke(7.9653, &[M(-0.2125, 0.0), L(0.2125, 0.24)]),
                Part::Thick(&[M(-0.1034, 0.2995), C([0.062, 0.2393], [0.1473, 0.0564], [0.087, -0.1091]), C([0.0769, -0.1369], [0.0629, -0.1631], [0.0456, -0.187])]),
                Part::Solid(&[
                    M(-0.0076, -0.2442),
                    C([0.0177, -0.2226], [0.0644, -0.1846], [0.1117, -0.1721]),
                    C([0.0633, -0.1754], [0.0428, -0.1593], [0.0349, -0.1114]),
                    C([0.0336, -0.1604], [0.0075, -0.2145], [-0.0076, -0.2442]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, 0.3),
                    C([0.2525, 0.3221], [0.2346, 0.34], [0.2125, 0.34]),
                    C([0.1905, 0.34], [0.1726, 0.3221], [0.1726, 0.3]),
                    C([0.1726, 0.2779], [0.1905, 0.2599], [0.2125, 0.2599]),
                    C([0.2346, 0.2599], [0.2525, 0.2779], [0.2525, 0.3]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, 0.3),
                    C([0.2525, 0.3221], [0.2346, 0.34], [0.2125, 0.34]),
                    C([0.1905, 0.34], [0.1726, 0.3221], [0.1726, 0.3]),
                    C([0.1726, 0.2779], [0.1905, 0.2599], [0.2125, 0.2599]),
                    C([0.2346, 0.2599], [0.2525, 0.2779], [0.2525, 0.3]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, -0.3),
                    C([0.2525, -0.278], [0.2346, -0.2601], [0.2125, -0.2601]),
                    C([0.1905, -0.2601], [0.1726, -0.278], [0.1726, -0.3]),
                    C([0.1726, -0.3221], [0.1905, -0.34], [0.2125, -0.34]),
                    C([0.2346, -0.34], [0.2525, -0.3221], [0.2525, -0.3]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, -0.3),
                    C([0.2525, -0.278], [0.2346, -0.2601], [0.2125, -0.2601]),
                    C([0.1905, -0.2601], [0.1726, -0.278], [0.1726, -0.3]),
                    C([0.1726, -0.3221], [0.1905, -0.34], [0.2125, -0.34]),
                    C([0.2346, -0.34], [0.2525, -0.3221], [0.2525, -0.3]),
                    Z,
                ]),
                Part::Pole(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
                Part::Thin(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
            ],
            [-0.2125, 0.2125, -0.3, 0.3],
        )
    },
    Node {
        text: Text::Above([0.0, 0.3]),
        anchors: &[("text", [0.0, 0.3]), ("hlines nw", [-0.21, 0.09]), ("hlines ne", [0.21, 0.09]), ("hlines sw", [-0.21, -0.09]), ("hlines se", [0.21, -0.09])],
        ..node(
            "proximeter",
            &[
                Part::Pole(&[M(-0.3, 0.0), L(0.0, 0.3), L(0.3, 0.0), L(0.0, -0.3), Z]),
                Part::Thick(&[M(-0.3, 0.0), L(0.0, 0.3), L(0.3, 0.0), L(0.0, -0.3), Z]),
                Part::Thin(&[M(-0.2101, 0.0901), L(0.2099, 0.0901), M(-0.2101, -0.09), L(0.2099, -0.09)]),
            ],
            [-0.3, 0.3, -0.3, 0.3],
        )
    },
    Node {
        anchors: &[("in", [-0.425, 0.0]), ("out 1", [0.425, 0.225]), ("out 2", [0.425, -0.225]), ("mid", [0.0, 0.0832])],
        ..node(
            "spdt",
            &[
                Part::Thin(&[M(0.4249, 0.225), L(0.1913, 0.225), M(0.4249, -0.225), L(0.1913, -0.225), M(-0.425, 0.0), L(-0.1913, 0.0)]),
                Part::Pole(&[
                    M(0.2312, 0.225),
                    C([0.2312, 0.2471], [0.2133, 0.265], [0.1913, 0.265]),
                    C([0.1691, 0.265], [0.1512, 0.2471], [0.1512, 0.225]),
                    C([0.1512, 0.2029], [0.1691, 0.1849], [0.1913, 0.1849]),
                    C([0.2133, 0.1849], [0.2312, 0.2029], [0.2312, 0.225]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2312, 0.225),
                    C([0.2312, 0.2471], [0.2133, 0.265], [0.1913, 0.265]),
                    C([0.1691, 0.265], [0.1512, 0.2471], [0.1512, 0.225]),
                    C([0.1512, 0.2029], [0.1691, 0.1849], [0.1913, 0.1849]),
                    C([0.2133, 0.1849], [0.2312, 0.2029], [0.2312, 0.225]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2312, -0.225),
                    C([0.2312, -0.203], [0.2133, -0.1851], [0.1913, -0.1851]),
                    C([0.1691, -0.1851], [0.1512, -0.203], [0.1512, -0.225]),
                    C([0.1512, -0.2471], [0.1691, -0.265], [0.1913, -0.265]),
                    C([0.2133, -0.265], [0.2312, -0.2471], [0.2312, -0.225]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2312, -0.225),
                    C([0.2312, -0.203], [0.2133, -0.1851], [0.1913, -0.1851]),
                    C([0.1691, -0.1851], [0.1512, -0.203], [0.1512, -0.225]),
                    C([0.1512, -0.2471], [0.1691, -0.265], [0.1913, -0.265]),
                    C([0.2133, -0.265], [0.2312, -0.2471], [0.2312, -0.225]),
                    Z,
                ]),
                Part::Pole(&[
                    M(-0.1513, 0.0),
                    C([-0.1513, 0.0221], [-0.1692, 0.0399], [-0.1913, 0.0399]),
                    C([-0.2134, 0.0399], [-0.2313, 0.0221], [-0.2313, 0.0]),
                    C([-0.2313, -0.0221], [-0.2134, -0.0399], [-0.1913, -0.0399]),
                    C([-0.1692, -0.0399], [-0.1513, -0.0221], [-0.1513, 0.0]),
                    Z,
                ]),
                Part::Thin(&[
                    M(-0.1513, 0.0),
                    C([-0.1513, 0.0221], [-0.1692, 0.0399], [-0.1913, 0.0399]),
                    C([-0.2134, 0.0399], [-0.2313, 0.0221], [-0.2313, 0.0]),
                    C([-0.2313, -0.0221], [-0.2134, -0.0399], [-0.1913, -0.0399]),
                    C([-0.1692, -0.0399], [-0.1513, -0.0221], [-0.1513, 0.0]),
                    Z,
                ]),
                Part::Thick(&[M(-0.1513, 0.0), L(0.1799, 0.1737)]),
            ],
            [-0.425, 0.425, -0.225, 0.225],
        )
    },
    // the cute switches at `bipoles/cuteswitch/height=0.8`, to draw them at any height
    Node {
        anchors: &[("in", [-0.2525, 0.0]), ("out 1", [0.2525, 0.4]), ("out 2", [0.2525, -0.4]), ("mid", [0.0, 0.17])],
        ..node(
            "cute spdt up@height",
            &[
                Part::Stroke(7.9653, &[M(-0.2125, 0.0), L(0.2125, 0.34)]),
                Part::Pole(&[
                    M(0.2525, 0.4),
                    C([0.2525, 0.4221], [0.2346, 0.44], [0.2125, 0.44]),
                    C([0.1905, 0.44], [0.1726, 0.4221], [0.1726, 0.4]),
                    C([0.1726, 0.3779], [0.1905, 0.3599], [0.2125, 0.3599]),
                    C([0.2346, 0.3599], [0.2525, 0.3779], [0.2525, 0.4]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, 0.4),
                    C([0.2525, 0.4221], [0.2346, 0.44], [0.2125, 0.44]),
                    C([0.1905, 0.44], [0.1726, 0.4221], [0.1726, 0.4]),
                    C([0.1726, 0.3779], [0.1905, 0.3599], [0.2125, 0.3599]),
                    C([0.2346, 0.3599], [0.2525, 0.3779], [0.2525, 0.4]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, -0.4),
                    C([0.2525, -0.3779], [0.2346, -0.3601], [0.2125, -0.3601]),
                    C([0.1905, -0.3601], [0.1726, -0.3779], [0.1726, -0.4]),
                    C([0.1726, -0.4221], [0.1905, -0.44], [0.2125, -0.44]),
                    C([0.2346, -0.44], [0.2525, -0.4221], [0.2525, -0.4]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, -0.4),
                    C([0.2525, -0.3779], [0.2346, -0.3601], [0.2125, -0.3601]),
                    C([0.1905, -0.3601], [0.1726, -0.3779], [0.1726, -0.4]),
                    C([0.1726, -0.4221], [0.1905, -0.44], [0.2125, -0.44]),
                    C([0.2346, -0.44], [0.2525, -0.4221], [0.2525, -0.4]),
                    Z,
                ]),
                Part::Pole(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
                Part::Thin(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
            ],
            [-0.2125, 0.2125, -0.4, 0.4],
        )
    },
    Node {
        anchors: &[("in", [-0.2525, 0.0]), ("out 1", [0.2525, 0.4]), ("out 2", [0.2525, -0.4]), ("mid", [0.0, 0.0])],
        ..node(
            "cute spdt mid@height",
            &[
                Part::Stroke(7.9653, &[M(-0.2125, 0.0), L(0.2125, 0.0)]),
                Part::Pole(&[
                    M(0.2525, 0.4),
                    C([0.2525, 0.4221], [0.2346, 0.44], [0.2125, 0.44]),
                    C([0.1905, 0.44], [0.1726, 0.4221], [0.1726, 0.4]),
                    C([0.1726, 0.3779], [0.1905, 0.3599], [0.2125, 0.3599]),
                    C([0.2346, 0.3599], [0.2525, 0.3779], [0.2525, 0.4]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, 0.4),
                    C([0.2525, 0.4221], [0.2346, 0.44], [0.2125, 0.44]),
                    C([0.1905, 0.44], [0.1726, 0.4221], [0.1726, 0.4]),
                    C([0.1726, 0.3779], [0.1905, 0.3599], [0.2125, 0.3599]),
                    C([0.2346, 0.3599], [0.2525, 0.3779], [0.2525, 0.4]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, -0.4),
                    C([0.2525, -0.3779], [0.2346, -0.3601], [0.2125, -0.3601]),
                    C([0.1905, -0.3601], [0.1726, -0.3779], [0.1726, -0.4]),
                    C([0.1726, -0.4221], [0.1905, -0.44], [0.2125, -0.44]),
                    C([0.2346, -0.44], [0.2525, -0.4221], [0.2525, -0.4]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, -0.4),
                    C([0.2525, -0.3779], [0.2346, -0.3601], [0.2125, -0.3601]),
                    C([0.1905, -0.3601], [0.1726, -0.3779], [0.1726, -0.4]),
                    C([0.1726, -0.4221], [0.1905, -0.44], [0.2125, -0.44]),
                    C([0.2346, -0.44], [0.2525, -0.4221], [0.2525, -0.4]),
                    Z,
                ]),
                Part::Pole(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
                Part::Thin(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
            ],
            [-0.2125, 0.2125, -0.4, 0.4],
        )
    },
    Node {
        anchors: &[("in", [-0.2525, 0.0]), ("out 1", [0.2525, 0.4]), ("out 2", [0.2525, -0.4]), ("mid", [0.0, -0.17])],
        ..node(
            "cute spdt down@height",
            &[
                Part::Stroke(7.9653, &[M(-0.2125, 0.0), L(0.2125, -0.34)]),
                Part::Pole(&[
                    M(0.2525, 0.4),
                    C([0.2525, 0.4221], [0.2346, 0.44], [0.2125, 0.44]),
                    C([0.1905, 0.44], [0.1726, 0.4221], [0.1726, 0.4]),
                    C([0.1726, 0.3779], [0.1905, 0.3599], [0.2125, 0.3599]),
                    C([0.2346, 0.3599], [0.2525, 0.3779], [0.2525, 0.4]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, 0.4),
                    C([0.2525, 0.4221], [0.2346, 0.44], [0.2125, 0.44]),
                    C([0.1905, 0.44], [0.1726, 0.4221], [0.1726, 0.4]),
                    C([0.1726, 0.3779], [0.1905, 0.3599], [0.2125, 0.3599]),
                    C([0.2346, 0.3599], [0.2525, 0.3779], [0.2525, 0.4]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, -0.4),
                    C([0.2525, -0.3779], [0.2346, -0.3601], [0.2125, -0.3601]),
                    C([0.1905, -0.3601], [0.1726, -0.3779], [0.1726, -0.4]),
                    C([0.1726, -0.4221], [0.1905, -0.44], [0.2125, -0.44]),
                    C([0.2346, -0.44], [0.2525, -0.4221], [0.2525, -0.4]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, -0.4),
                    C([0.2525, -0.3779], [0.2346, -0.3601], [0.2125, -0.3601]),
                    C([0.1905, -0.3601], [0.1726, -0.3779], [0.1726, -0.4]),
                    C([0.1726, -0.4221], [0.1905, -0.44], [0.2125, -0.44]),
                    C([0.2346, -0.44], [0.2525, -0.4221], [0.2525, -0.4]),
                    Z,
                ]),
                Part::Pole(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
                Part::Thin(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
            ],
            [-0.2125, 0.2125, -0.4, 0.4],
        )
    },
    Node {
        anchors: &[("in", [-0.2525, 0.0]), ("out 1", [0.2525, 0.4]), ("out 2", [0.2525, -0.4]), ("mid", [0.0, 0.17])],
        ..node(
            "cute spdt up arrow@height",
            &[
                Part::Stroke(7.9653, &[M(-0.2125, 0.0), L(0.2125, 0.34)]),
                Part::Thick(&[M(-0.1034, 0.2995), C([0.062, 0.2393], [0.1473, 0.0564], [0.087, -0.1091]), C([0.0769, -0.1369], [0.0629, -0.1631], [0.0456, -0.1869])]),
                Part::Solid(&[
                    M(-0.0076, -0.2442),
                    C([0.0177, -0.2226], [0.0644, -0.1846], [0.1117, -0.1719]),
                    C([0.0633, -0.1754], [0.0428, -0.1593], [0.0349, -0.1114]),
                    C([0.0336, -0.1604], [0.0075, -0.2145], [-0.0076, -0.2442]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, 0.4),
                    C([0.2525, 0.4221], [0.2346, 0.44], [0.2125, 0.44]),
                    C([0.1905, 0.44], [0.1726, 0.4221], [0.1726, 0.4]),
                    C([0.1726, 0.3779], [0.1905, 0.3599], [0.2125, 0.3599]),
                    C([0.2346, 0.3599], [0.2525, 0.3779], [0.2525, 0.4]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, 0.4),
                    C([0.2525, 0.4221], [0.2346, 0.44], [0.2125, 0.44]),
                    C([0.1905, 0.44], [0.1726, 0.4221], [0.1726, 0.4]),
                    C([0.1726, 0.3779], [0.1905, 0.3599], [0.2125, 0.3599]),
                    C([0.2346, 0.3599], [0.2525, 0.3779], [0.2525, 0.4]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, -0.4),
                    C([0.2525, -0.3779], [0.2346, -0.3601], [0.2125, -0.3601]),
                    C([0.1905, -0.3601], [0.1726, -0.3779], [0.1726, -0.4]),
                    C([0.1726, -0.4221], [0.1905, -0.44], [0.2125, -0.44]),
                    C([0.2346, -0.44], [0.2525, -0.4221], [0.2525, -0.4]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, -0.4),
                    C([0.2525, -0.3779], [0.2346, -0.3601], [0.2125, -0.3601]),
                    C([0.1905, -0.3601], [0.1726, -0.3779], [0.1726, -0.4]),
                    C([0.1726, -0.4221], [0.1905, -0.44], [0.2125, -0.44]),
                    C([0.2346, -0.44], [0.2525, -0.4221], [0.2525, -0.4]),
                    Z,
                ]),
                Part::Pole(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
                Part::Thin(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
            ],
            [-0.2125, 0.2125, -0.4, 0.4],
        )
    },
    Node {
        anchors: &[("in", [-0.2525, 0.0]), ("out 1", [0.2525, 0.4]), ("out 2", [0.2525, -0.4]), ("mid", [0.0, 0.0])],
        ..node(
            "cute spdt mid arrow@height",
            &[
                Part::Stroke(7.9653, &[M(-0.2125, 0.0), L(0.2125, 0.0)]),
                Part::Thick(&[M(0.0094, -0.2289), C([0.112, -0.1296], [0.1382, 0.0301], [0.0636, 0.1594]), C([0.0487, 0.1851], [0.0304, 0.2085], [0.0093, 0.2289])]),
                Part::Solid(&[
                    M(-0.0531, -0.2761),
                    C([-0.0244, -0.2591], [0.0281, -0.2299], [0.0769, -0.2257]),
                    C([0.0286, -0.2206], [0.0112, -0.2012], [0.0117, -0.1526]),
                    C([0.0021, -0.2006], [-0.0331, -0.2494], [-0.0531, -0.2761]),
                    Z,
                ]),
                Part::Solid(&[
                    M(-0.0531, 0.276),
                    C([-0.0331, 0.2494], [0.0021, 0.2006], [0.0117, 0.1526]),
                    C([0.0112, 0.2011], [0.0286, 0.2206], [0.0769, 0.2257]),
                    C([0.0281, 0.2299], [-0.0244, 0.2591], [-0.0531, 0.276]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, 0.4),
                    C([0.2525, 0.4221], [0.2346, 0.44], [0.2125, 0.44]),
                    C([0.1905, 0.44], [0.1726, 0.4221], [0.1726, 0.4]),
                    C([0.1726, 0.3779], [0.1905, 0.3599], [0.2125, 0.3599]),
                    C([0.2346, 0.3599], [0.2525, 0.3779], [0.2525, 0.4]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, 0.4),
                    C([0.2525, 0.4221], [0.2346, 0.44], [0.2125, 0.44]),
                    C([0.1905, 0.44], [0.1726, 0.4221], [0.1726, 0.4]),
                    C([0.1726, 0.3779], [0.1905, 0.3599], [0.2125, 0.3599]),
                    C([0.2346, 0.3599], [0.2525, 0.3779], [0.2525, 0.4]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, -0.4),
                    C([0.2525, -0.3779], [0.2346, -0.3601], [0.2125, -0.3601]),
                    C([0.1905, -0.3601], [0.1726, -0.3779], [0.1726, -0.4]),
                    C([0.1726, -0.4221], [0.1905, -0.44], [0.2125, -0.44]),
                    C([0.2346, -0.44], [0.2525, -0.4221], [0.2525, -0.4]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, -0.4),
                    C([0.2525, -0.3779], [0.2346, -0.3601], [0.2125, -0.3601]),
                    C([0.1905, -0.3601], [0.1726, -0.3779], [0.1726, -0.4]),
                    C([0.1726, -0.4221], [0.1905, -0.44], [0.2125, -0.44]),
                    C([0.2346, -0.44], [0.2525, -0.4221], [0.2525, -0.4]),
                    Z,
                ]),
                Part::Pole(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
                Part::Thin(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
            ],
            [-0.2125, 0.2125, -0.4, 0.4],
        )
    },
    Node {
        anchors: &[("in", [-0.2525, 0.0]), ("out 1", [0.2525, 0.4]), ("out 2", [0.2525, -0.4]), ("mid", [0.0, -0.17])],
        ..node(
            "cute spdt down arrow@height",
            &[
                Part::Stroke(7.9653, &[M(-0.2125, 0.0), L(0.2125, -0.34)]),
                Part::Thick(&[M(-0.0076, -0.2442), C([0.1273, -0.131], [0.1449, 0.07], [0.0317, 0.2049]), C([0.0126, 0.2276], [-0.0094, 0.2474], [-0.0339, 0.264])]),
                Part::Solid(&[
                    M(-0.1034, 0.2995),
                    C([-0.0791, 0.2768], [-0.036, 0.2348], [-0.0182, 0.1892]),
                    C([-0.0271, 0.2369], [-0.0133, 0.2591], [0.0333, 0.2724]),
                    C([-0.0154, 0.2681], [-0.0722, 0.2879], [-0.1034, 0.2995]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, 0.4),
                    C([0.2525, 0.4221], [0.2346, 0.44], [0.2125, 0.44]),
                    C([0.1905, 0.44], [0.1726, 0.4221], [0.1726, 0.4]),
                    C([0.1726, 0.3779], [0.1905, 0.3599], [0.2125, 0.3599]),
                    C([0.2346, 0.3599], [0.2525, 0.3779], [0.2525, 0.4]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, 0.4),
                    C([0.2525, 0.4221], [0.2346, 0.44], [0.2125, 0.44]),
                    C([0.1905, 0.44], [0.1726, 0.4221], [0.1726, 0.4]),
                    C([0.1726, 0.3779], [0.1905, 0.3599], [0.2125, 0.3599]),
                    C([0.2346, 0.3599], [0.2525, 0.3779], [0.2525, 0.4]),
                    Z,
                ]),
                Part::Pole(&[
                    M(0.2525, -0.4),
                    C([0.2525, -0.3779], [0.2346, -0.3601], [0.2125, -0.3601]),
                    C([0.1905, -0.3601], [0.1726, -0.3779], [0.1726, -0.4]),
                    C([0.1726, -0.4221], [0.1905, -0.44], [0.2125, -0.44]),
                    C([0.2346, -0.44], [0.2525, -0.4221], [0.2525, -0.4]),
                    Z,
                ]),
                Part::Thin(&[
                    M(0.2525, -0.4),
                    C([0.2525, -0.3779], [0.2346, -0.3601], [0.2125, -0.3601]),
                    C([0.1905, -0.3601], [0.1726, -0.3779], [0.1726, -0.4]),
                    C([0.1726, -0.4221], [0.1905, -0.44], [0.2125, -0.44]),
                    C([0.2346, -0.44], [0.2525, -0.4221], [0.2525, -0.4]),
                    Z,
                ]),
                Part::Pole(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
                Part::Thin(&[
                    M(-0.1724, 0.0),
                    C([-0.1724, 0.0221], [-0.1904, 0.0399], [-0.2125, 0.0399]),
                    C([-0.2346, 0.0399], [-0.2525, 0.0221], [-0.2525, 0.0]),
                    C([-0.2525, -0.0221], [-0.2346, -0.0399], [-0.2125, -0.0399]),
                    C([-0.1904, -0.0399], [-0.1724, -0.0221], [-0.1724, 0.0]),
                    Z,
                ]),
            ],
            [-0.2125, 0.2125, -0.4, 0.4],
        )
    },
];

const CURRARROW: &[Op] = &[M(-0.0437, 0.0), L(-0.0437, -0.05), L(0.0625, 0.0), L(-0.0437, 0.05), Z];
const FLOWHEAD: &[Op] = &[M(0.1437, 0.0), L(0.1437, -0.05), L(0.25, 0.0), L(0.1437, 0.05), Z];

/// The terminals: drawn by their shape and filled (with the line color, or white unless filled),
/// `nodes width` across the half.
pub const TERMINALS: &[&str] = &["circ", "ocirc", "diamondpole", "odiamondpole", "squarepole", "osquarepole", "rectjoinfill"];

pub fn find(name: &str) -> Option<&'static Node> {
    NODES
        .iter()
        .chain(transistors::TRANSISTORS)
        .chain(tubes::TUBES)
        .chain(amplifiers::AMPLIFIERS)
        .chain(doubles::DOUBLE_BIPOLES)
        .chain(logic::LOGIC)
        .chain(blocks::BLOCKS)
        .find(|n| n.name == name)
}

/// The transistors', tubes' and amplifiers' options that change their drawing, in the order of their variants'
/// names (`npn|bodydiode`), and the ones with a value.
pub const VARIANT_FLAGS: &[&str] = &[
    "no input leads",
    "no output leads",
    "no leads",
    "noinv input up",
    "noinv input down",
    "noinv output up",
    "noinv output down",
    "filament",
    "nocathode",
    "fullcathode",
    "nixieanode",
    "anodedot",
    "nogrid",
    "schottky base",
    "bodydiode",
    "photo",
    "tr circle",
    "nobase",
    "bulk",
    "emptycircle",
    "nocircle",
    "arrowmos",
    "solderdot",
    "fdsoi",
    "fdsoidot",
    "doublegate",
    "ferroel gate",
];
pub const VARIANT_VALUES: &[&str] = &["number inputs", "collectors", "emitters", "bjt pins width", "bjt multi height", "inner inputs"];

/// The transistors', tubes', amplifiers' and double bipoles' shapes (`npn`, `triode`, `op amp`,
/// `transformer`…), without their variants.
pub fn variant_shapes() -> impl Iterator<Item = &'static str> {
    transistors::TRANSISTORS
        .iter()
        .chain(tubes::TUBES)
        .chain(amplifiers::AMPLIFIERS)
        .chain(doubles::DOUBLE_BIPOLES)
        .chain(logic::LOGIC)
        .map(|n| n.name)
        .filter(|n| !n.contains('|') && !n.contains('@') && !n.contains('~'))
}

/// The transistors' shapes, without their variants.
pub fn transistor_shapes() -> impl Iterator<Item = &'static str> {
    transistors::TRANSISTORS.iter().map(|n| n.name).filter(|n| !n.contains('|') && !n.contains('@'))
}

/// The variant of transistor, tube or amplifier `base` its options ask for, or the plain one (and a warning) when
/// voltik has not measured that combination.
fn variant(cx: &mut Context, base: &str) -> Result<Option<&'static Node>, Error> {
    let mut name = base.to_string();
    for flag in VARIANT_FLAGS {
        if setting(cx, &format!("tr/{flag}"))? == "true" {
            name.push('|');
            name.push_str(flag);
        }
    }
    // a transformer's coils are the inductors' style, and a double bipole can be inline
    if base.starts_with("transformer") {
        let inductor = setting(cx, "inductor")?;
        if inductor != "cute" {
            name.push_str(&format!("|inductor={inductor}"));
        }
    }
    if matches!(base, "transformer" | "transformer core" | "gyrator" | "double bipole") && setting(cx, "quadpoles style")? == "inline" {
        name.push_str("|quadpoles style=inline");
    }
    for key in VARIANT_VALUES {
        let value = setting(cx, &format!("tr/{key}"))?;
        if !value.is_empty() {
            name.push_str(&format!("|{key}={value}"));
        }
    }
    // a multi-anode tube's anodes (`tubes/anodes`, 3)
    if base == "matube" {
        let anodes = crate::bipole::number(cx, "tubes/anodes", 3.0).round();
        if anodes != 3.0 {
            name.push_str(&format!("|anodes={anodes}"));
        }
    }
    // the transistors' arrows at the end (`transistors/arrow pos=end`)
    if setting(cx, "transistors/arrow pos")? == "end" {
        name.push_str("|arrow pos=end");
    }
    // a logic port's sizes other than its default, as measured: `ieeestd or port|height=4`
    if logic::LOGIC.iter().any(|n| n.name == base) {
        let style = base.split(' ').next().unwrap_or_default();
        let sizes: &[(String, &str, f64)] = &[
            ("ieeestd ports/height".into(), "height", 2.0),
            ("ieeestd ports/pin length".into(), "pin length", 0.7),
            (format!("tripoles/{base}/height"), "height", 0.8),
            (format!("tripoles/{base}/inner"), "inner", 0.3),
            (format!("tripoles/{base}/angle"), "angle", 70.0),
        ];
        for (key, short, default) in sizes {
            if key.starts_with("ieeestd") != (style == "ieeestd") || (style != "ieeestd" && style != "american") {
                continue;
            }
            let value = crate::bipole::number(cx, key, *default);
            if (value - default).abs() > 1e-9 {
                name.push_str(&format!("|{short}={value}"));
            }
        }
    }
    // a negated European port's symbol (`tripoles/european not symbol=circle`), and its xnor's
    if matches!(base, "european nand port" | "european nor port" | "european xnor port" | "european not port" | "european blank not port") {
        let symbol = setting(cx, "tripoles/european not symbol")?;
        if !symbol.is_empty() && symbol != "triangle" {
            name.push_str(&format!("|not symbol={symbol}"));
        }
    }
    if base == "european xnor port" && setting(cx, "european xnor style")? == "direct" {
        name.push_str("|xnor style=direct");
    }
    // options that do not change this one (`arrow pos=end` on a MOSFET) are dropped
    let name = name.replace("|number inputs=2", "");
    // (and a circle on what has none: only a P-type transistor has it)
    let circleless = name.replace("|emptycircle", "").replace("|nocircle", "");
    let plain = circleless.replace("|arrow pos=end", "").replace("|ferroel gate", "").replace("|arrowmos", "");
    let bipolar = circleless.replace("|arrowmos", "");
    let found = [&name, &name.replace("|arrow pos=end", ""), &circleless, &bipolar, &circleless.replace("|arrow pos=end", ""), &plain].into_iter().find_map(|n| find(n));
    match found {
        Some(n) => Ok(Some(n)),
        None => {
            cx.warn(format!("voltik: {} not drawn yet; drawn as a plain {base}", name.replace('|', ", ")));
            Ok(find(base))
        }
    }
}

/// `voltik/class fill={class}`: the class's `fill`, if any, fills the node.
pub fn k_class_fill(cx: &mut Context, v: Option<Tokens>) -> Result<(), Error> {
    let class = cx.expand(v.unwrap_or_default())?;
    let fill = setting(cx, &format!("{class}/fill"))?;
    if !fill.is_empty() && fill != "none" {
        cx.keys(Tokens::text(&format!("fill={{{fill}}}")))?;
        cx.define_text("voltik@cfill", &class, false);
    }
    Ok(())
}

/// How far above an oscilloscope drawn as a node its text's box goes (cm).
const OSCOPE_TEXT: f64 = 0.1133;

/// A bipole as a node (`node[resistorshape]`): its anchors, and the bipole itself drawn after it
/// between its `west` and `east`, as a `to` that fills it.
fn bipole_shape(cx: &mut Context, input: &ShapeInput) -> Result<NodeShape, Error> {
    let key = cx.macro_text("voltik@bn");
    let comp = match crate::components::target(&key) {
        Some(crate::components::Target::Fixed(n)) => crate::components::find(n),
        _ => None,
    };
    let Some(comp) = comp else { return Ok(NodeShape::default()) };
    let length = setting(cx, "bipoles/length")?;
    let scale = crate::bipole::number(cx, &format!("{}/scale", comp.class.name()), 1.0);
    let u = cx.length(Tokens::text(&length)).unwrap_or(1.4) * scale;
    // a block's own size and texts, and the node's fill, go to the bipole drawn after it
    let size = crate::components::blocks_size(cx, comp);
    // stretched to its own size (`bipoles/oscope/width`), if it has one
    let (own_w, own_h) = crate::bipole::own_size(cx, comp);
    let (cw, ch) = size.unwrap_or((own_w.unwrap_or(comp.width), own_h.unwrap_or(comp.height)));
    let (sx, sy) = (cw / comp.width.max(1e-9), ch / comp.height.max(1e-9));
    let (w, top, bottom) = (cw / 2.0 * u, comp.top * ch / comp.height.max(1e-9) * u, comp.bottom * ch / comp.height.max(1e-9) * u);
    let mut options = String::new();
    if let Some((sw, sh)) = size {
        let base = crate::components::block_setting_name(comp.name.split('|').next().unwrap_or(comp.name));
        options.push_str(&format!(", bipoles/{base}/width={sw}, bipoles/{base}/height={sh}"));
    }
    for (key, macro_name) in [("t", "voltik@t"), ("t1", "voltik@ta"), ("t2", "voltik@tb")] {
        let text = cx.macro_body(macro_name).to_text();
        if !text.trim().is_empty() {
            options.push_str(&format!(", {key}={{{text}}}"));
        }
    }
    if let Some(fill) = &input.fill {
        options.push_str(&format!(", fill={{{fill}}}"));
    }
    // its ends on its line, its north and south its band's
    let mut anchors = compass(-w, w, -bottom, top);
    for (name, at) in anchors.iter_mut() {
        if matches!(name.as_str(), "west" | "w" | "left" | "east" | "e" | "right") {
            at.y = 0.0;
        }
    }
    anchors.extend(comp.anchors.iter().map(|(name, p)| (name.to_string(), Point::new(p[0] * cw / comp.width.max(1e-9) * u, p[1] * ch / comp.height.max(1e-9) * u))));
    // a block's lateral anchors, `block left anchors pos` (and right) of the way up to its top
    if comp.class == crate::components::Class::Blocks {
        let left = crate::bipole::number(cx, "block left anchors pos", 0.5);
        let right = crate::bipole::number(cx, "block right anchors pos", 0.5);
        for (name, at) in anchors.iter_mut() {
            match name.as_str() {
                "left up" => at.y = left * top,
                "left down" => at.y = -left * bottom,
                "right up" => at.y = right * top,
                "right down" => at.y = -right * bottom,
                _ => {}
            }
        }
    }
    // a potentiometer's wiper, at `wiper pos` along it, as on a path
    if comp.parts.iter().any(|p| matches!(p, crate::components::Part::Wiper { .. })) {
        let x = (crate::bipole::number(cx, "wiper pos", 0.5) - 0.5) * comp.width * u * sx;
        let top = comp.height / 2.0 * u * sy;
        anchors.extend([("wiper".to_string(), Point::new(x, top + 0.25 * u * sy)), ("tip".to_string(), Point::new(x, top))]);
    }
    anchors.push(("text".into(), Point::new(0.0, 0.0)));
    // (globally: it runs after the node, out of its options' group)
    // (`voltik/up`: the node's north, for the bipole to be mirrored with it, `yscale=-1`)
    // (in a group with a local current point: the path goes on from the node's point, not its east)
    cx.run(Tokens::text(&format!(
        "\\gdef\\voltik@bp{{\\pgfextra{{\\xdef\\voltik@bn{{\\tikzlastnode}}}} {{[current point is local] (\\voltik@bn.west) to[{key}, voltik/fit, voltik/up=\\voltik@bn.north{options}] (\\voltik@bn.east)}}}}"
    )))?;
    // an oscilloscope's text goes above it, as a label
    let text = if comp.name == "oscope" { Point::new(-input.width / 2.0, top + OSCOPE_TEXT + input.depth) } else { Point::new(0.0, 0.0) };
    Ok(NodeShape {
        center: Point::new(0.0, 0.0),
        anchors,
        border: ShapeBorder::Point,
        text,
        paths: Vec::new(),
        // (its size is the bipole's, drawn after it: its box does not count)
        extent: Vec::new(),
        ..Default::default()
    })
}

/// The bipoles a double bipole draws after it with `double bipole L` and `R` (`fulllediodeshape`…),
/// from its terminals to `inner` of the way in; none (empty) when it draws its own.
fn double_bipole(cx: &mut Context) -> Result<bool, Error> {
    let (l, r) = (setting(cx, "double bipole L")?, setting(cx, "double bipole R")?);
    let key = |shape: &str, default: &str| crate::components::bipole_shapes().find(|(s, _)| *s == shape).map_or(default.to_string(), |(_, k)| k.to_string());
    if l.is_empty() && r.is_empty() {
        cx.run(Tokens::text("\\gdef\\voltik@db{}"))?;
        return Ok(false);
    }
    // inline, the terminals are where the bipoles go
    let (fl, fr) = if setting(cx, "quadpoles style")? == "inline" {
        (0.0, 1.0)
    } else {
        let inner = crate::bipole::number(cx, "quadpoles/double bipole/inner", 0.4);
        ((1.0 - inner) / 2.0, (1.0 + inner) / 2.0)
    };
    let invert = |cx: &mut Context, side: &str| setting(cx, &format!("db/{side} invert")).map(|v| if v == "true" { ", invert" } else { "" });
    let (il, ir) = (invert(cx, "L")?, invert(cx, "R")?);
    // the bipoles' styles, kept out of the node's group where they may have been defined
    for side in ["L", "R"] {
        cx.run(Tokens::text(&format!(
            "\\global\\expandafter\\let\\csname pgfk@/tikz/voltik@db{side}/.@cmd\\expandafter\\endcsname\\csname pgfk@/tikz/circuitikz/every double bipole {side}/.@cmd\\endcsname"
        )))?;
    }
    let n = "\\tikzlastnode";
    // L goes down from A1, R up from B2 (`double bipole R invert` turns it down)
    let code = format!(
        "({n}.A1) -- ($({n}.A1)!{fl}!({n}.B1)$) to[{}{il}, voltik@dbL] ($({n}.A2)!{fl}!({n}.B2)$) -- ({n}.A2) \
         ({n}.B2) -- ($({n}.A2)!{fr}!({n}.B2)$) to[{}{ir}, voltik@dbR] ($({n}.A1)!{fr}!({n}.B1)$) -- ({n}.B1)",
        key(&l, "european resistor"),
        key(&r, "american voltage source"),
    );
    cx.run(Tokens::text(&format!("\\gdef\\voltik@db{{{code}}}")))?;
    Ok(true)
}

/// A transformer's coil as a node of its own (`T1-L1`): `a` and `b` its ends, `voltik@coil` half its
/// length (cm) and which way it goes (`-` up from `a`, `+` down).
fn coil_shape(cx: &mut Context, _input: &ShapeInput) -> Result<NodeShape, Error> {
    let spec = cx.macro_text("voltik@coil");
    let (spec, midtap) = spec.split_once(':').unwrap_or((&spec, "0"));
    let (sign, half) = spec.split_at(1);
    let h = half.parse::<f64>().unwrap_or(0.0) * if sign == "+" { 1.0 } else { -1.0 };
    let midtap = midtap.parse::<f64>().unwrap_or(0.0);
    let p = Point::new;
    let mut anchors = compass(-0.0, 0.0, -h.abs(), h.abs());
    anchors.extend([("a".to_string(), p(0.0, h)), ("b".to_string(), p(0.0, -h)), ("midtap".to_string(), p(if sign == "+" { -midtap } else { midtap }, 0.0))]);
    Ok(NodeShape { center: p(0.0, 0.0), anchors, border: ShapeBorder::Point, text: p(0.0, 0.0), paths: Vec::new(), extent: Vec::new(), ..Default::default() })
}

/// A transformer's coils: as nodes of their own (`T1-L1`, `T1-L2`) always, and drawn by the
/// inductors of `transformer L1` and `L2` when these change them (then `true`: the shape draws only
/// its leads' frame and its core).
fn transformer(cx: &mut Context, base: &str) -> Result<bool, Error> {
    let length = setting(cx, "bipoles/length")?;
    let unit = cx.length(Tokens::text(&length)).unwrap_or(1.4);
    // each coil, with its style on: its inductor and its length
    let coil = |cx: &mut Context, style: &str| -> Result<(String, f64, f64), Error> {
        cx.begin_group();
        cx.keys_at("/tikz/circuitikz", Tokens::text(style))?;
        let kind = setting(cx, "inductor")?;
        let width = crate::bipole::number(cx, "inductors/width", f64::NAN);
        let coils = setting(cx, "inductors/coils")?;
        cx.end_group();
        let default = match kind.as_str() {
            "american" => 0.8,
            "european" => 0.8,
            _ => 0.6,
        };
        let w = if width.is_nan() { default } else { width };
        Ok((format!("{kind}|{coils}"), w, default))
    };
    let plain = coil(cx, "")?;
    let (l1, l2) = (coil(cx, "transformer L1")?, coil(cx, "transformer L2")?);
    let own = l1.0 != plain.0 || l2.0 != plain.0 || (l1.1 - plain.1).abs() > 1e-9 || (l2.1 - plain.1).abs() > 1e-9;
    let inline = setting(cx, "quadpoles style")? == "inline";
    let inner = crate::bipole::number(cx, &format!("quadpoles/{base}/inner"), 0.4);
    let (fl, fr) = if inline { (0.0, 1.0) } else { ((1.0 - inner) / 2.0, (1.0 + inner) / 2.0) };
    // the transformer's name, before its coils' nodes become the last node
    let n = "\\voltik@tn";
    let mut code = String::from("\\pgfextra{\\xdef\\voltik@tn{\\tikzlastnode}} ");
    if own {
        for side in ["L1", "L2"] {
            cx.run(Tokens::text(&format!(
                "\\global\\expandafter\\let\\csname pgfk@/tikz/voltik@tr{side}/.@cmd\\expandafter\\endcsname\\csname pgfk@/tikz/circuitikz/transformer {side}/.@cmd\\endcsname"
            )))?;
        }
        // L1 goes up from A2, L2 down from B1
        code.push_str(&format!(
            "({n}.A2) -- ($({n}.A2)!{fl}!({n}.B2)$) to[L, mirror, voltik@trL1] ($({n}.A1)!{fl}!({n}.B1)$) -- ({n}.A1) \
             ({n}.B1) -- ($({n}.A1)!{fr}!({n}.B1)$) to[L, mirror, voltik@trL2] ($({n}.A2)!{fr}!({n}.B2)$) -- ({n}.B2) "
        ));
    }
    // the coils' nodes, in the middle of each side
    for (side, f, (kind, w, _), sign) in [("L1", fl, &l1, "-"), ("L2", fr, &l2, "+")] {
        let half = w / 2.0 * unit;
        let midtap = crate::bipole::coil_midtap(kind) * unit;
        code.push_str(&format!("($({n}.A1)!{f}!({n}.B1)$ |- {n}.center) node[shape=voltik coil, voltik/coil={{{sign}{half}:{midtap}}}, name={n}-{side}] {{}} "));
    }
    cx.run(Tokens::text(&format!("\\gdef\\voltik@tr{{{code}}}")))?;
    Ok(own)
}

/// A rotary switch's arrow at its default wiper (20°) and size, from its input pole's center (`cin`):
/// the arc with a head at neither end shortened, one, or both; and its heads.
const ROTARY_R: f64 = 0.5189;
const ROTARY_ARC_CW: &[Op] = &[M(0.0887, 0.2437), C([0.1758, 0.212], [0.2394, 0.1364], [0.2555, 0.045]), C([0.2615, 0.0109], [0.2606, -0.0237], [0.2531, -0.057])];
const ROTARY_ARC_CCW: &[Op] = &[M(0.1573, 0.2063), C([0.2084, 0.1674], [0.244, 0.1101], [0.2555, 0.045]), C([0.2661, -0.0151], [0.2552, -0.077], [0.2247, -0.1297])];
const ROTARY_ARC_BOTH: &[Op] = &[M(0.1573, 0.2063), C([0.2083, 0.1673], [0.244, 0.1101], [0.2555, 0.045]), C([0.2615, 0.0109], [0.2606, -0.0237], [0.2531, -0.057])];
const ROTARY_HEAD_CW: &[Op] = &[
    M(0.2247, -0.1297),
    C([0.2397, -0.1001], [0.2681, -0.0469], [0.3069, -0.0171]),
    C([0.2636, -0.039], [0.2384, -0.0319], [0.2126, 0.0091]),
    C([0.2303, -0.0364], [0.2271, -0.0966], [0.2247, -0.1297]),
    Z,
];
const ROTARY_HEAD_CCW: &[Op] = &[
    M(0.0887, 0.2437),
    C([0.112, 0.2199], [0.153, 0.1759], [0.1686, 0.1296]),
    C([0.1621, 0.1776], [0.1769, 0.1991], [0.2241, 0.2102]),
    C([0.1751, 0.2082], [0.1194, 0.2307], [0.0887, 0.2437]),
    Z,
];

/// `rotaryswitch`: `multipoles/rotary/channels` output poles on a circle around the input pole, from
/// `angle` above to `angle` below, the wiper at `wiper`, an arrow (`arrow`: `cw`, `ccw`, `both`)
/// around it; its poles are nodes of their own (`S-in`, `S-out 3`), drawn after it.
fn rotary_shape(cx: &mut Context, input: &ShapeInput) -> Result<NodeShape, Error> {
    let num = |cx: &mut Context, key: &str, default: f64| crate::bipole::number(cx, &format!("multipoles/rotary/{key}"), default);
    let n = num(cx, "channels", 5.0).round().max(1.0) as usize;
    let a = num(cx, "angle", 60.0).to_radians();
    let wiper = num(cx, "wiper", 20.0);
    let thickness = num(cx, "thickness", 1.0);
    let arrow = setting(cx, "multipoles/rotary/arrow")?;
    let pole = setting(cx, "multipoles/rotary/shape")?;
    let length = setting(cx, "bipoles/length")?;
    let u = cx.length(Tokens::text(&length)).unwrap_or(1.4) * crate::bipole::number(cx, "switches/scale", 1.0);
    let r = ROTARY_R * crate::bipole::number(cx, "tripoles/spdt/width", 0.85) / 0.85;
    let half = crate::bipole::number(cx, "nodes width", 0.04);
    // the input pole, so that the whole is centered across
    let cin = -(r + r * a.cos().min(0.0)) / 2.0;
    let p = |x: f64, y: f64| Point::new(x * u, y * u);
    let angle = |i: usize| if n == 1 { 0.0 } else { a - 2.0 * a * i as f64 / (n - 1) as f64 };
    let mut anchors: Vec<(String, Point)> = Vec::new();
    let top = if a >= std::f64::consts::FRAC_PI_2 { r } else { r * a.sin() } + half;
    let (west, east) = (cin + r * a.cos().min(0.0) - half, cin + r + half);
    anchors.extend(compass(west * u, east * u, -top * u, top * u));
    let ext = 1.25 * r;
    for (name, x, y) in [
        ("north", cin, ext),
        ("south", cin, -ext),
        ("east", cin + ext, 0.0),
        ("west", cin - ext, 0.0),
        ("north east", cin + ext, ext),
        ("north west", cin - ext, ext),
        ("south east", cin + ext, -ext),
        ("south west", cin - ext, -ext),
        ("center", cin, 0.0),
    ] {
        anchors.push((format!("ext {name}"), p(x, y)));
    }
    anchors.push(("in".into(), p(cin - half, 0.0)));
    anchors.push(("cin".into(), p(cin, 0.0)));
    let w = wiper.to_radians();
    anchors.push(("mid".into(), p(cin + r / 2.0 * w.cos(), r / 2.0 * w.sin())));
    let mut poles = vec![("in".to_string(), cin, 0.0)];
    for i in 0..n {
        let t = angle(i);
        let (x, y) = (cin + r * t.cos(), r * t.sin());
        let k = i + 1;
        anchors.push((format!("out {k}"), p(x + half, y)));
        anchors.push((format!("cout {k}"), p(x, y)));
        anchors.push((format!("aout {k}"), p(x + half * t.cos(), y + half * t.sin())));
        // on the square around the input pole, along the line from it
        let reach = (ext / t.cos().abs().max(1e-9)).min(ext / t.sin().abs().max(1e-9));
        anchors.push((format!("sqout {k}"), p(cin + reach * t.cos(), reach * t.sin())));
        poles.push((format!("out {k}"), x, y));
    }
    anchors.push(("text".into(), p(0.0, 0.0)));
    // the wiper and the arrow, turned from their default and sized
    let mut paths = Vec::new();
    let turn = |ops: &[Op]| -> Vec<Op> {
        let (c, sn, k) = ((wiper - 20.0).to_radians().cos(), (wiper - 20.0).to_radians().sin(), r / ROTARY_R);
        let q = |x: f64, y: f64| (cin + k * (x * c - y * sn), k * (x * sn + y * c));
        ops.iter()
            .map(|op| match *op {
                Op::Move(x, y) => {
                    let (a, b) = q(x, y);
                    Op::Move(a, b)
                }
                Op::Line(x, y) => {
                    let (a, b) = q(x, y);
                    Op::Line(a, b)
                }
                Op::Curve(a1, b1, c1) => {
                    let (a, b) = (q(a1[0], a1[1]), q(b1[0], b1[1]));
                    let e = q(c1[0], c1[1]);
                    Op::Curve([a.0, a.1], [b.0, b.1], [e.0, e.1])
                }
                Op::Close => Op::Close,
            })
            .collect()
    };
    let lw = input.line_width;
    if thickness >= 0.0 {
        let lever = [M(cin, 0.0), L(cin + r * w.cos(), r * w.sin())];
        paths.push(ShapePath { segments: segments(&lever, u), paint: ShapePaint::Stroke(7.965 * thickness * lw), ..Default::default() });
    }
    let (arc, heads): (&[Op], &[&[Op]]) = match arrow.as_str() {
        "cw" => (ROTARY_ARC_CW, &[ROTARY_HEAD_CW]),
        "ccw" => (ROTARY_ARC_CCW, &[ROTARY_HEAD_CCW]),
        "both" => (ROTARY_ARC_BOTH, &[ROTARY_HEAD_CW, ROTARY_HEAD_CCW]),
        _ => (&[], &[]),
    };
    let mut after = String::new();
    let custom = if arc.is_empty() { None } else { crate::bipole::arrow_look(cx, "switch", Some("switch arrows"), "latexslim")? };
    if let Some(custom) = custom.filter(|c| !c.own_head || !c.start.is_empty()) {
        // the whole arc, with the tips of the ends it has: its start the counterclockwise one
        let thick = crate::bipole::number(cx, "bipoles/thickness", 2.0);
        let (start, end) = match arrow.as_str() {
            "cw" => ("", custom.end.as_str()),
            "ccw" => (custom.start.as_str(), ""),
            _ => (custom.start.as_str(), custom.end.as_str()),
        };
        let ops = turn(&[ROTARY_ARC_CW[0], ROTARY_ARC_CW[1], ROTARY_ARC_CCW[2]]);
        let mut code = String::new();
        for op in &ops {
            let mut point = |q: [f64; 2]| {
                let name = format!("voltik arrow {}", anchors.len());
                let text = format!("(\\voltik@tn.{name})");
                anchors.push((name, p(q[0] * u, q[1] * u)));
                text
            };
            code.push_str(&match *op {
                Op::Move(x, y) => point([x, y]),
                Op::Line(x, y) => format!(" -- {}", point([x, y])),
                Op::Curve(a, b, c) => format!(" .. controls {} and {} .. {}", point(a), point(b), point(c)),
                Op::Close => String::new(),
            });
        }
        after = format!("\\draw[{{{start}}}-{{{end}}},solid,{}line width={}cm] {code};", custom.options, thick * lw * custom.thickness);
    } else if !arc.is_empty() {
        let thick = crate::bipole::number(cx, "bipoles/thickness", 2.0);
        paths.push(ShapePath { segments: segments(&turn(arc), u), paint: ShapePaint::Stroke(thick * lw), ..Default::default() });
        for head in heads {
            paths.push(ShapePath { segments: segments(&turn(head), u), paint: ShapePaint::Solid, ..Default::default() });
        }
    }
    // the poles, drawn here (in the node's colors) and nodes of their own after it, for their
    // anchors (`S-out 2.n`)
    for (_, x, y) in &poles {
        let mut t = terminal(&pole, half * u, lw);
        for path in t.paths.drain(..) {
            let segments = path.segments.iter().map(|seg| shift(*seg, Point::new(x * u, y * u))).collect();
            paths.push(ShapePath { segments, paint: path.paint, ..Default::default() });
        }
    }
    let mut code = format!("\\pgfextra{{\\xdef\\voltik@tn{{\\tikzlastnode}}{after}}} ");
    for (name, _, _) in &poles {
        let anchor = if name == "in" { "cin".to_string() } else { format!("c{name}") };
        code.push_str(&format!("(\\voltik@tn.{anchor}) node[shape=voltik pole, voltik/pole shape={pole}, name=\\voltik@tn-{name}] {{}} "));
    }
    cx.run(Tokens::text(&format!("\\gdef\\voltik@rs{{{code}}}")))?;
    Ok(NodeShape {
        center: Point::new(0.0, 0.0),
        anchors,
        border: ShapeBorder::Point,
        text: Point::new(0.0, 0.0),
        paths,
        extent: vec![p(west, -top), p(east, top)],
        ..Default::default()
    })
}

/// A segment moved by `d`.
fn shift(seg: PathSegment, d: Point) -> PathSegment {
    let m = |p: Point| Point::new(p.x + d.x, p.y + d.y);
    match seg {
        PathSegment::MoveTo(p) => PathSegment::MoveTo(m(p)),
        PathSegment::LineTo(p) => PathSegment::LineTo(m(p)),
        PathSegment::CurveTo(a, b, c) => PathSegment::CurveTo(m(a), m(b), m(c)),
        PathSegment::Close => PathSegment::Close,
    }
}

/// A pole drawn by its component, as a node with the pole's anchors and border but nothing drawn
/// (`voltik/pole shape` the terminal it is).
fn pole_shape(cx: &mut Context, input: &ShapeInput) -> Result<NodeShape, Error> {
    let length = setting(cx, "bipoles/length")?;
    let half = crate::bipole::number(cx, "nodes width", 0.04) * cx.length(Tokens::text(&length)).unwrap_or(1.4);
    let mut t = terminal(&cx.macro_text("voltik@pole"), half, input.line_width);
    t.paths.clear();
    Ok(t)
}

/// A component named by `to[R, name=R1]`: a node with its anchors (`\voltik@named`, `north=x,y;…`
/// from its center on the canvas) and nothing drawn, so that `(R1)`, `(R1.north)` and a name prefix
/// are as a node's. Notes the prefix (`\voltik@prefix`) for what `\ctikzgetanchor` answers.
fn named_shape(cx: &mut Context, input: &ShapeInput) -> Result<NodeShape, Error> {
    let back = input.transform.inverse();
    let mut anchors = Vec::new();
    for item in cx.macro_text("voltik@named").split(';') {
        let Some((name, xy)) = item.split_once('=') else { continue };
        let Some((x, y)) = xy.split_once(',') else { continue };
        if let (Ok(x), Ok(y)) = (x.trim().parse::<f64>(), y.trim().parse::<f64>()) {
            // a component's own point overrides a compass point of the same name
            anchors.retain(|(a, _): &(String, Point)| a != name.trim());
            anchors.push((name.trim().to_string(), back.linear(Point::new(x, y))));
        }
    }
    let corner = |name: &str| anchors.iter().find(|(a, _)| a == name).map(|(_, p)| *p);
    let border = match (corner("south west"), corner("south east"), corner("north east"), corner("north west")) {
        (Some(a), Some(b), Some(c), Some(d)) => ShapeBorder::Outline(vec![a, b, c, d]),
        _ => ShapeBorder::Point,
    };
    let name = cx.macro_text("voltik@name");
    let prefix = input.node.as_deref().and_then(|n| n.strip_suffix(name.as_str())).unwrap_or("").to_string();
    cx.define_text("voltik@prefix", &prefix, true);
    Ok(NodeShape { anchors, border, ..Default::default() })
}

/// `rotary switch=8 in 120 wiper 40` (and `rotary switch <-=…`, `->`, `<->`, `-`): the rotary switch
/// with these, or its defaults, and an arrow.
pub fn k_rotary(cx: &mut Context, v: Option<Tokens>) -> Result<(), Error> {
    let a = v.unwrap_or_default().arguments(2);
    let arrow = cx.expand(a[0].clone())?;
    let spec = cx.expand(a[1].clone())?;
    let words: Vec<&str> = spec.split_whitespace().collect();
    let pick = |i: usize, default: &str| words.get(i).copied().filter(|w| !w.is_empty()).unwrap_or(default).to_string();
    let (n, angle, wiper) = (pick(0, "5"), pick(2, "60"), pick(4, "20"));
    // (`rotary switch` alone keeps the arrow in force)
    let arrow = if arrow.is_empty() { String::new() } else { format!(", /tikz/circuitikz/multipoles/rotary/arrow={arrow}") };
    cx.keys(Tokens::text(&format!(
        "shape=rotaryswitch, /tikz/circuitikz/multipoles/rotary/channels={n}, /tikz/circuitikz/multipoles/rotary/angle={angle}, /tikz/circuitikz/multipoles/rotary/wiper={wiper}{arrow}, voltik/class fill=switches, append after command={{\\voltik@rs}}"
    )))
}

/// `muxdemux def={Lh=4, NL=2}`: the shape of the `muxdemux`es, as `multipoles/muxdemux/…`.
pub fn k_muxdemux_def(cx: &mut Context, v: Option<Tokens>) -> Result<(), Error> {
    let mut list = Tokens::new();
    for (key, value) in v.unwrap_or_default().key_list() {
        // a drawing on it, in pgf's own commands, kept for the shape
        if key.starts_with("bgpicture") {
            cx.define("voltik@mdbg", value.unwrap_or_default());
            continue;
        }
        list = match value {
            Some(value) => list.and_text(&format!("{key}={{")).and(value).and_text("},"),
            None => list.and_text(&format!("{key},")),
        };
    }
    cx.keys_at("/tikz/circuitikz/multipoles/muxdemux", list)
}

/// `voltik/logic port={and}`: the port of the `logic ports` style (`ieee` draws `ieeestd and port`).
pub fn k_logic_port(cx: &mut Context, v: Option<Tokens>) -> Result<(), Error> {
    let kind = cx.expand(v.unwrap_or_default())?;
    let style = match setting(cx, "logic ports")?.as_str() {
        "european" => "european",
        "ieee" => "ieeestd",
        _ => "american",
    };
    cx.keys(Tokens::text(&format!("{style} {kind} port")))
}

/// The blocks drawn as nodes (`mixer`, `bareantenna`…), each with its class.
pub fn block_shapes() -> impl Iterator<Item = (&'static str, &'static str)> {
    blocks::SHAPES.iter().map(|n| (*n, blocks::class(n).unwrap_or("blocks")))
}

/// `voltik/shapes`: declares every node component to tikra.
pub fn k_shapes(cx: &mut Context, _v: Option<Tokens>) -> Result<(), Error> {
    // the package's old-style tip, as long and wide as measured at the default line width
    cx.declare_arrow_tip("latexslim", "Latex[length=4pt,width=3pt]");
    // the jack taps' shorthands: `^` up, `v` down (swapped), `f` filled
    cx.declare_arrow_tip("^", "Jack Tap");
    cx.declare_arrow_tip("^f", "Jack Tap[fill]");
    cx.declare_arrow_tip("v", "Jack Tap[swap]");
    cx.declare_arrow_tip("vf", "Jack Tap[swap,fill]");
    for n in NODES {
        cx.declare_shape(n.name, shape);
    }
    cx.declare_shape("voltik bipole", bipole_shape);
    cx.declare_shape("voltik coil", coil_shape);
    cx.declare_shape("rotaryswitch", rotary_shape);
    cx.declare_shape("dipchip", chips::chip_shape);
    cx.declare_shape("qfpchip", chips::chip_shape);
    cx.declare_shape("muxdemux", muxdemux::muxdemux_shape);
    cx.declare_shape("dc symbol", symbols::symbol_shape);
    cx.declare_shape("ac symbol", symbols::symbol_shape);
    cx.declare_shape("wedgeinv", muxdemux::inversion_shape);
    cx.declare_shape("circleinv", muxdemux::inversion_shape);
    cx.declare_shape("flipflop", flipflops::flipflop_shape);
    cx.declare_shape("clockwedge", flipflops::clockwedge_shape);
    cx.declare_shape("bare7seg", displays::seven_segment_shape);
    cx.declare_shape("voltik pole", pole_shape);
    cx.declare_shape("voltik named", named_shape);
    for name in blocks::SHAPES {
        cx.declare_shape(name, shape);
    }
    for name in variant_shapes() {
        cx.declare_shape(name, shape);
    }
    for t in TERMINALS {
        cx.declare_shape(t, shape);
    }
    Ok(())
}

/// The shape of a node component, for one node.
fn shape(cx: &mut Context, input: &ShapeInput) -> Result<NodeShape, Error> {
    let length = setting(cx, "bipoles/length")?;
    let unit = cx.length(Tokens::text(&length)).unwrap_or(1.4);
    if TERMINALS.contains(&input.name.as_str()) {
        let half = crate::bipole::number(cx, "nodes width", 0.04) * unit;
        let mut shape = terminal(&input.name, half, input.line_width);
        open_pole(cx, input, &mut shape)?;
        return Ok(shape);
    }
    let n = if variant_shapes().any(|t| t == input.name) { variant(cx, &input.name)? } else { find(&input.name) };
    let Some(n) = n else { return Ok(NodeShape::default()) };
    // a round block in a box (`mixer, boxed`), or only the box
    let n = match find(&format!("{}|{}", n.name, if setting(cx, "blocks/box only")? == "true" { "box only" } else { "boxed" })) {
        Some(boxed) if setting(cx, "blocks/boxed")? == "true" => boxed,
        _ => n,
    };
    // a double bipole with bipoles of its own draws only its anchors here
    let own = input.name == "double bipole" && double_bipole(cx)?;
    // a transformer whose coils are drawn as inductors draws its frame's core only
    let coils = matches!(input.name.as_str(), "transformer" | "transformer core") && transformer(cx, &input.name)?;
    let unit = match n.class() {
        Some(class) => unit * crate::bipole::number(cx, &format!("{class}/scale"), 1.0),
        None => unit,
    };
    let at = |p: [f64; 2]| Point::new(p[0] * unit, p[1] * unit);
    let w = input.line_width;
    let filled = !cx.macro_text("voltik@fill").is_empty();
    // a tube wider than tall has its own measures
    let n = match find(&format!("{}~wide", n.name)) {
        Some(wide) if crate::bipole::number(cx, "tubes/width", 1.0) > crate::bipole::number(cx, "tubes/height", 1.4) => wide,
        _ => n,
    };
    let mut sized = sized(cx, n)?;
    let circled = n.name.contains("tr circle");
    let circle_thickness = crate::bipole::number(cx, "transistor circle/relative thickness", 1.0);
    // the class's `thickness` (`tubes/thickness=4`), times the line width, for the thick lines
    // (the amplifiers' and logic ports' is the tripoles', `tripoles/thickness`, unless their own is set)
    let tripole = matches!(n.class(), Some("amplifiers" | "logic ports"));
    let thick = n
        .class()
        .and_then(|c| setting(cx, &format!("{c}/thickness")).ok())
        .and_then(|t| t.parse::<f64>().ok())
        .or_else(|| tripole.then(|| crate::bipole::number(cx, "tripoles/thickness", 2.0)))
        .unwrap_or(2.0);
    let lever = crate::bipole::number(cx, "bipoles/cuteswitch/thickness", 1.0);
    // `transistor circle/scale circle radius`: a transistor's circle grows from its left side
    let radius = crate::bipole::number(cx, "transistor circle/scale circle radius", 1.0);
    let circle = if circled { circle_of(&sized) } else { None };
    if (radius - 1.0).abs() > 1e-9 && n.name.contains("tr circle") {
        scale_circle(&mut sized, radius);
    }
    if n.name == "dynode" {
        dynode_arc(cx, &mut sized);
    }
    let diode = if n.name.contains("bodydiode") { body_diode(cx, &mut sized)? } else { Vec::new() };
    sizes(cx, n, &mut sized)?;
    let (sign_parts, sign_texts) = amplifier_signs(cx, n, &sized, unit)?;
    let supply_arrow = supply_arrow(cx, input, n, unit)?;
    let outer_base = if n.name.contains("igbt") { igbt_outer_base(cx, &mut sized)? } else { Vec::new() };
    let outer_thickness = crate::bipole::number(cx, "tripoles/igbt/outer base thickness", 1.0);
    // an `en amp` with an `en amp text` does not draw its ▷∞, the solid parts at its top
    let own_text = input.name == "en amp" && !setting(cx, "en amp text")?.is_empty();
    // a transistor's circle in a color and dashes of its own (`transistor circle/color`, `dash`)
    let circle_color = match setting(cx, "transistor circle/color")?.trim() {
        "" | "default" => None,
        c => Some(c.to_string()),
    };
    let circle_dash = setting(cx, "transistor circle/dash")?;
    let circle_dash = dash(cx, &circle_dash)?;
    let diode_color = match setting(cx, "transistor bodydiode/color")?.trim() {
        "" | "default" => None,
        c => Some(c.to_string()),
    };
    let diode_dash = setting(cx, "transistor bodydiode/dash")?;
    let diode_dash = dash(cx, &diode_dash)?;
    let diode_thickness = crate::bipole::number(cx, "transistor bodydiode/relative thickness", 1.0);
    // a European port's symbol in `european ports font`: a text at its middle, upright, instead of
    // the glyphs measured in the plain font
    let symbol = match european_symbol(n.name) {
        Some(text) => {
            let font = Tokens::text("\\unexpanded\\expandafter{\\romannumeral-`0\\pgfkeysvalueof{/tikz/circuitikz/european ports font}}");
            let font = cx.expand(font)?.trim().to_string();
            (!font.is_empty()).then_some((text, font))
        }
        None => None,
    };
    // the class's fill taken back later in the node's options (`circuitikz/logic ports/fill=none`)
    let class_fill = cx.macro_text("voltik@cfill");
    let unfilled = !class_fill.is_empty() && matches!(setting(cx, &format!("{class_fill}/fill"))?.trim(), "" | "none");
    // the light of a phototransistor and the arrow of a cute switch, in their looks (`opto arrows`)
    let custom = match n.name {
        name if name.contains("photo") => crate::bipole::arrow_look(cx, "opto", Some("opto arrows"), "latexslim")?,
        name if name.contains("spdt") && name.contains("arrow") => crate::bipole::arrow_look(cx, "switch", Some("switch arrows"), "latexslim")?,
        _ => None,
    };
    let mut after = String::new();
    let mut arrow_parts: Vec<Vec<Op>> = Vec::new();
    let mut arrow_anchors: Vec<(String, Point)> = Vec::new();
    if let Some(custom) = &custom {
        let tn = "\\voltik@tn";
        for pair in sized.parts.windows(2) {
            let ((stroke, ops), (head, tip_ops)) = (&pair[0], &pair[1]);
            let width = match stroke {
                Part::Thin(_) => w,
                Part::Thick(_) => thick * w,
                Part::Stroke(k, _) => k * w,
                _ => continue,
            };
            if !matches!(head, Part::Solid(_)) || !tip_ops.iter().any(|o| matches!(o, Op::Curve(..))) {
                continue;
            }
            let mut anchor = |p: [f64; 2]| {
                let name = format!("voltik arrow {}", arrow_anchors.len());
                let code = format!("({tn}.{name})");
                arrow_anchors.push((name, at(p)));
                code
            };
            let mut trace = |ops: &[Op]| {
                let mut path = String::new();
                for op in ops {
                    path.push_str(&match *op {
                        Op::Move(x, y) => format!(" {}", anchor([x, y])),
                        Op::Line(x, y) => format!(" -- {}", anchor([x, y])),
                        Op::Curve(a, b, c) => format!(" .. controls {} and {} .. {}", anchor(a), anchor(b), anchor(c)),
                        Op::Close => " -- cycle".to_string(),
                    });
                }
                path
            };
            if custom.own_head {
                let color = custom.options.split(',').find(|o| o.starts_with("color")).unwrap_or("color=.");
                let (line, solid) = (trace(ops), trace(tip_ops));
                after.push_str(&format!("\\draw[{{{}}}-,solid,{}line width={}cm]{line};\\fill[{color}]{solid};", custom.start, custom.options, width * custom.thickness));
            } else {
                let end = crate::bipole::head_tip(ops, tip_ops);
                let (line, tip) = (trace(ops), trace(&[Op::Line(end[0], end[1])]));
                after.push_str(&format!("\\draw[{}]{line}{tip};", custom.draw(width)));
            }
            arrow_parts.extend([ops.clone(), tip_ops.clone()]);
        }
    }
    if !after.is_empty() || !cx.macro_text("voltik@na").is_empty() {
        let code = if after.is_empty() { String::new() } else { format!("\\pgfextra{{\\xdef\\voltik@tn{{\\tikzlastnode}}{after}}}") };
        cx.run(Tokens::text(&format!("\\gdef\\voltik@na{{{code}}}")))?;
    }
    // the lines that have a look of their own: a crossing's vertical one, a transformer's core
    let lined = |name: &str| {
        if name.ends_with("crossing") {
            Some("crossing vertical")
        } else if name.contains("transformer core") {
            Some("transformer core")
        } else {
            None
        }
    };
    let line_style = match lined(n.name) {
        Some(group) => {
            let color = setting(cx, &format!("{group}/color"))?;
            let line_dash = setting(cx, &format!("{group}/dash"))?;
            let thickness = crate::bipole::number(cx, &format!("{group}/relative thickness"), 1.0);
            Some((if matches!(color.trim(), "" | "default") { None } else { Some(color.trim().to_string()) }, dash(cx, &line_dash)?, thickness))
        }
        None => None,
    };
    let styled_line = |ops: &[Op]| {
        let straight = ops.chunks(2).all(|c| matches!(c, [Op::Move(x, _), Op::Line(x2, _)] if (x - x2).abs() < 1e-9));
        straight && if n.name.ends_with("crossing") { matches!(ops[0], Op::Move(x, _) if x.abs() < 1e-9) } else { matches!(ops[0], Op::Move(x, _) if x.abs() < 0.1) }
    };
    // a depletion MOSFET's channel, of `tripoles/nmosd/depletion color`
    let depletion = match n.name.split('|').next().unwrap_or("") {
        kind @ ("nmosd" | "pmosd") => match setting(cx, &format!("tripoles/{kind}/depletion color"))?.trim() {
            "" | "default" => None,
            c => Some(c.to_string()),
        },
        _ => None,
    };
    // the gap between a transistor's gate and its channel, filled
    let gap_fill = match setting(cx, "tr gap fill")?.trim() {
        "" | "none" => None,
        c => Some(c.to_string()),
    };
    // a MOSFET's gate (its leftmost bar), `transistors/gate thickness` of its channel, and their ends
    let is_transistor = transistors::TRANSISTORS.iter().any(|t| t.name == n.name);
    let gate = if is_transistor { gate_bar(&sized) } else { None };
    let gate_thickness = crate::bipole::number(cx, "transistors/gate thickness", 1.0);
    let rounded = is_transistor && setting(cx, "transistors/rounded caps")? == "true";
    // (the arrows' variants have a solid circle: `emptycircle` empties it)
    let empty_bubble = n.name.contains("|arrowmos") && setting(cx, "tr/emptycircle")? == "true";
    let paths = sized
        .parts
        .iter()
        .filter(|_| !own)
        .filter(|(part, _)| symbol.is_none() || !matches!(part, Part::Solid(_)))
        .filter(|(part, _)| !(unfilled && matches!(part, Part::Fill(_))))
        .filter(|(_, ops)| !sign_parts.contains(ops))
        .filter(|(_, ops)| !arrow_parts.contains(ops))
        .filter(|(part, ops)| !(depletion.as_deref() == Some("none") && matches!(part, Part::Channel(_)) && !is_circle(ops)))
        .filter(|_| !supply_arrow)
        .filter(|(part, ops)| !coils || (matches!(part, Part::Thick(_)) && !ops.iter().any(|op| matches!(op, Op::Curve(..) | Op::Close))))
        .filter(|(part, ops)| !(own_text && matches!(part, Part::Solid(_)) && ops.iter().all(|op| !matches!(op, Op::Move(_, y) | Op::Line(_, y) if *y < 0.5))))
        .map(|(part, ops)| {
            let paint = match *part {
                Part::Thin(_) => ShapePaint::Stroke(w),
                // a transistor's circle, `transistor circle/relative thickness` of the others
                Part::Thick(ops) if circled && is_circle(ops) => ShapePaint::Stroke(thick * w * circle_thickness),
                Part::Thick(_) => ShapePaint::Stroke(thick * w),
                Part::Thicker(_) => ShapePaint::Stroke(3.0 * w),
                // a logic port's negation circle hides its lead, as a pole does
                Part::Fill(ops) if is_circle(ops) && n.class() == Some("logic ports") => ShapePaint::Background,
                Part::Fill(_) => ShapePaint::Fill,
                Part::Solid(_) => ShapePaint::Solid,
                Part::Pole(_) => ShapePaint::Background,
                Part::Channel(_) if empty_bubble => ShapePaint::Background,
                Part::Channel(_) => {
                    if filled {
                        ShapePaint::Fill
                    } else {
                        ShapePaint::Solid
                    }
                }
                // a cute switch's lever, by `bipoles/cuteswitch/thickness`
                Part::Stroke(k, _) if n.name.starts_with("cute") => ShapePaint::Stroke(k * w * lever),
                Part::Stroke(k, _) => ShapePaint::Stroke(k * w),
            };
            if matches!(part, Part::Thick(ops) if circled && is_circle(ops)) {
                return ShapePath { segments: segments(ops, unit), paint, color: circle_color.clone(), dash: circle_dash.clone(), ..Default::default() };
            }
            if ops == &outer_base {
                return ShapePath { segments: segments(ops, unit), paint: ShapePaint::Stroke(thick * w * outer_thickness), ..Default::default() };
            }
            // what the body diode is drawn with (`transistor bodydiode/…`)
            if diode.contains(ops) {
                let paint = match paint {
                    ShapePaint::Stroke(width) => ShapePaint::Stroke(width * diode_thickness),
                    other => other,
                };
                return ShapePath { segments: segments(ops, unit), paint, color: diode_color.clone(), dash: diode_dash.clone(), ..Default::default() };
            }
            if let (Some(color), Part::Channel(_), false) = (&depletion, part, is_circle(ops)) {
                return ShapePath { segments: segments(ops, unit), paint: ShapePaint::Solid, color: Some(color.clone()), ..Default::default() };
            }
            if let Some((color, line_dash, k)) = line_style.as_ref().filter(|_| styled_line(ops)) {
                let width = if let ShapePaint::Stroke(width) = paint { width } else { w };
                return ShapePath { segments: segments(ops, unit), paint: ShapePaint::Stroke(width * k), color: color.clone(), dash: line_dash.clone(), ..Default::default() };
            }
            if rounded && matches!(part, Part::Thick(_)) {
                let paint = if gate.as_ref() == Some(ops) { ShapePaint::Stroke(thick * w * gate_thickness) } else { paint };
                return ShapePath { segments: segments(ops, unit), paint, cap: Some(LineCap::Round), ..Default::default() };
            }
            if gate.as_ref() == Some(ops) {
                return ShapePath { segments: segments(ops, unit), paint: ShapePaint::Stroke(thick * w * gate_thickness), ..Default::default() };
            }
            ShapePath { segments: segments(ops, unit), paint, ..Default::default() }
        })
        .collect();
    let mut paths: Vec<ShapePath> = paths;
    if let (Some(color), Some(gate)) = (&gap_fill, &gate) {
        // from the gate to the channel next to it, as high as the gate; under what is drawn on it
        let [g, _, low, high] = bounds(gate);
        let channel =
            sized.parts.iter().filter(|(part, ops)| matches!(part, Part::Thick(_)) && ops != gate).map(|(_, ops)| bounds(ops)[0]).filter(|x| *x > g).fold(f64::MAX, f64::min);
        if channel < f64::MAX {
            let rect = [M(g, low), L(channel, low), L(channel, high), L(g, high), Z];
            let at = paths.iter().position(|p| matches!(p.paint, ShapePaint::Stroke(_))).unwrap_or(0);
            paths.insert(at, ShapePath { segments: segments(&rect, unit), paint: ShapePaint::Solid, color: Some(color.clone()), ..Default::default() });
        }
    }
    let paths = partial_borders(cx, paths, circled, n)?;
    let paths = blocks::inner(cx, n, paths)?;
    let [left, right, bottom, top] = sized.bounds;
    let mut anchors = compass(left * unit, right * unit, bottom * unit, top * unit);
    anchors.extend(sized.anchors.iter().map(|(name, p)| (name.to_string(), at(*p))));
    anchors.extend(arrow_anchors);
    // the circle's own compass (`circle top`), wherever `scale circle radius` left it
    if let (true, Some((left, right, cy))) = (circled, circle_of(&sized)) {
        let (cx0, r) = ((left + right) / 2.0, (right - left) / 2.0);
        for (name, p) in [("circle top", [cx0, cy + r]), ("circle bottom", [cx0, cy - r]), ("circle left", [left, cy]), ("circle right", [right, cy])] {
            anchors.push((name.to_string(), at(p)));
        }
    }
    // a cute switch's poles are nodes of their own too (`S1-in`, `S1-out 2`), after it
    if n.name.starts_with("cute spdt") {
        let half = crate::bipole::number(cx, "nodes width", 0.04);
        let pole = setting(cx, "bipoles/cuteswitch/shape")?;
        let mut code = String::from("\\pgfextra{\\xdef\\voltik@tn{\\tikzlastnode}} ");
        for (name, side) in [("in", 1.0), ("out 1", -1.0), ("out 2", -1.0)] {
            if let Some((_, p)) = sized.anchors.iter().find(|(a, _)| *a == name) {
                anchors.push((format!("c{name}"), at([p[0] + side * half, p[1]])));
                code.push_str(&format!("(\\voltik@tn.c{name}) node[shape=voltik pole, voltik/pole shape={pole}, name=\\voltik@tn-{name}] {{}} "));
            }
        }
        cx.run(Tokens::text(&format!("\\gdef\\voltik@cs{{{code}}}")))?;
    }
    // the text: its box's origin (the left end of its baseline), from where its side goes
    let sy = input.inner_sep.y;
    let text = match n.text {
        Text::Center => Point::new(-input.width / 2.0, (input.depth - input.height) / 2.0),
        Text::Above(p) => Point::new(p[0] * unit - input.width / 2.0, p[1] * unit + sy + input.depth),
        Text::Below(p) => Point::new(p[0] * unit - input.width / 2.0, p[1] * unit - sy - input.height),
        Text::AboveLeft(p) => Point::new(p[0] * unit - input.width, p[1] * unit + sy + input.depth),
        Text::AboveRight(p) => Point::new(p[0] * unit, p[1] * unit + sy + input.depth),
        Text::Right(p) => Point::new(p[0] * unit, p[1] * unit + (input.depth - input.height) / 2.0),
        Text::Left(p) => Point::new(p[0] * unit - input.width, p[1] * unit + (input.depth - input.height) / 2.0),
        Text::Over(p) => Point::new(p[0] * unit - input.width / 2.0, p[1] * unit + 0.6 * input.height),
        Text::Middle(p) => Point::new(p[0] * unit - input.width / 2.0, p[1] * unit + (input.depth - input.height) / 2.0),
    };
    let text = Point::new(text.x + sized.text_shift[0] * unit, text.y + sized.text_shift[1] * unit);
    // (a microstrip stub's text is over its middle, whatever its length)
    let text = if n.name == "mslstub" { Point::new(sized.bounds[1] / 2.0 * unit - input.width / 2.0, text.y) } else { text };
    // (a generic splitter's text starts further in the wider it is, measured)
    let text = if n.name == "genericsplitter" { Point::new(text.x + input.width / 3.0, text.y) } else { text };
    // a transistor's text: up or down (`component text`), or near its collector (`legacy
    // transistors text`); measured on CircuiTikZ: 0.7 of its height away (0.95 and twice as far
    // right for the multi-terminal BJTs)
    let text = if transistors::TRANSISTORS.iter().any(|t| t.name == n.name) {
        let mode = setting(cx, "component text")?;
        let legacy = setting(cx, "tr/legacy text")? == "true";
        let anchor = sized.anchors.iter().find(|(a, _)| *a == "text").map_or([0.05, 0.0], |(_, p)| *p);
        let bjt = n.name.starts_with("bjt");
        let (k, x) = if bjt { (0.95, 2.0 * anchor[0]) } else { (0.7, anchor[0]) };
        let middle = |y: f64| Point::new(x * unit, y * unit + (input.depth - input.height) / 2.0);
        // with a circle, measured on CircuiTikZ (a `nigfetebulk`): the text up or down by the
        // circle's right side, moving with it as `scale circle radius` grows it (each time by its
        // own radius times 0.3625 right and 0.5765 away); otherwise right of it, as far as its
        // right end goes; the legacy text stays where it is without a circle
        let grown = radius - 1.0;
        match (mode.as_str(), circle) {
            ("up" | "left" | "down", Some((left, right, _))) if !bjt => {
                let r = (right - left) / 2.0;
                let x = left + grown * r + r * (1.99 + 0.3625 * grown);
                if mode == "down" {
                    Point::new(x * unit, -r * (0.889 + 0.5765 * grown) * unit - input.height)
                } else {
                    Point::new(x * unit, r * (0.91 + 0.5765 * grown) * unit + input.depth)
                }
            }
            ("up" | "left", _) => middle(k * top),
            ("down", _) => middle(k * bottom),
            _ if legacy && !bjt => {
                let plain = find(&n.name.replace("|tr circle", "")).and_then(|p| p.anchors.iter().find(|(a, _)| *a == "text").map(|(_, p)| *p)).unwrap_or(anchor);
                Point::new(2.0 * plain[0] * unit, 0.7 * top * unit)
            }
            (_, Some((left, right, _))) => Point::new(text.x + grown * (right - left) * unit, text.y),
            _ => text,
        }
    } else {
        text
    };
    // `component text=left`: an amplifier's or a logic port's text starts `left text distance` from its left edge
    let leftedge = sized.anchors.iter().find(|(a, _)| *a == "leftedge" || *a == "body left").map(|(_, p)| p[0]);
    let text = match leftedge {
        Some(x) if setting(cx, "component text")? == "left" => {
            let distance = setting(cx, "left text distance")?;
            Point::new(x * unit + cx.length(Tokens::text(&distance)).unwrap_or(0.0), text.y)
        }
        _ => text,
    };
    // (a double bipole with bipoles of its own is their size, not its default one's)
    let mut extent: Vec<Point> = if own { Vec::new() } else { sized.extent.iter().map(|p| at(*p)).collect() };
    if n.text_counts && (input.width > 0.0 || input.height + input.depth > 0.0) {
        let (sx, sy) = (input.inner_sep.x, input.inner_sep.y);
        extent.push(Point::new(text.x - sx, text.y - input.depth - sy));
        extent.push(Point::new(text.x + input.width + sx, text.y + input.height + sy));
    }
    let anchors = blocks::lateral(cx, n, anchors, unit);
    // a round block's lines stop at its circle (or its box)
    let border = match blocks::border(n) {
        Some(ops) => ShapeBorder::Outline(outline_points(&segments(ops, unit))),
        None => ShapeBorder::Point,
    };
    let mut anchors = anchors;
    if matches!(n.name, "vcc" | "vee") {
        anchors.push(("arrow tip".to_string(), Point::new(0.0, if n.name == "vcc" { 0.3 } else { -0.3 } * unit)));
    }
    let mut out = NodeShape { center: Point::new(0.0, 0.0), anchors, border, text, paths, extent, texts: sign_texts };
    // `en amp text`: where the ▷∞ goes, its baseline there, mirrored and turned with the node
    if own_text {
        let text = setting(cx, "en amp text")?;
        out.texts.push(ShapeText { text, at: Point::new(-0.0235, 0.7562), anchor: TextAnchor::Base, ..Default::default() });
    }
    if let Some((text, font)) = symbol {
        let middle = sized.anchors.iter().find(|(a, _)| *a == "text").map_or([0.0, 0.0], |(_, p)| *p);
        let (color, font) = font_color(&font);
        // (a mirrored node keeps it readable too)
        let t = input.transform;
        let text = if t.a * t.d - t.b * t.c < 0.0 { format!("\\scalebox{{-1}}[1]{{{text}}}") } else { text.to_string() };
        out.texts.push(ShapeText { text: format!("{color}{text}"), at: at(middle), font: Some(font), upright: true, ..Default::default() });
    }
    // `logic ports origin=center`: a port's point halfway between its west and east
    if n.class() == Some("logic ports") && setting(cx, "logic ports origin")? == "center" {
        let x = |name: &str| out.anchors.iter().find(|(a, _)| a == name).map(|(_, p)| p.x);
        if let (Some(w), Some(e)) = (x("west"), x("east")) {
            centered(&mut out, Point::new(-(w + e) / 2.0, 0.0));
        }
    }
    Ok(out)
}

/// Font commands split into their `\color{…}`, which a shape's text takes as its color, and the rest.
fn font_color(font: &str) -> (String, String) {
    match font.find("\\color{") {
        Some(i) => {
            let end = font[i..].find('}').map_or(font.len(), |e| i + e + 1);
            (font[i..end].to_string(), format!("{}{}", &font[..i], &font[end..]))
        }
        None => (String::new(), font.to_string()),
    }
}

/// The symbol of a European port, as TeX writes it.
fn european_symbol(name: &str) -> Option<&'static str> {
    match name.split('|').next().unwrap_or(name) {
        "european and port" | "european nand port" => Some("\\&"),
        "european or port" | "european nor port" => Some("$\\scriptstyle\\geq$\\enspace 1"),
        "european xor port" | "european xnor port" => Some("=1"),
        "european buffer port" | "european not port" => Some("1"),
        _ => None,
    }
}

/// A shape moved by `d`, its `center` staying the node's point.
fn centered(s: &mut NodeShape, d: Point) {
    for (name, p) in s.anchors.iter_mut() {
        if name != "center" {
            *p = Point::new(p.x + d.x, p.y + d.y);
        }
    }
    s.text = Point::new(s.text.x + d.x, s.text.y + d.y);
    for path in s.paths.iter_mut() {
        for seg in path.segments.iter_mut() {
            *seg = shift(*seg, d);
        }
    }
    for p in s.extent.iter_mut() {
        *p = Point::new(p.x + d.x, p.y + d.y);
    }
    for t in s.texts.iter_mut() {
        t.at = Point::new(t.at.x + d.x, t.at.y + d.y);
    }
}

/// A terminal `half` (cm) across the half, with lines `w` wide.
/// An open pole's fill: the node's, else `open poles fill`, and always `open poles opacity`.
fn open_pole(cx: &mut Context, input: &ShapeInput, shape: &mut NodeShape) -> Result<(), Error> {
    let color = setting(cx, "open poles fill")?;
    let opacity = crate::bipole::number(cx, "poles/open fill opacity", 1.0);
    for path in shape.paths.iter_mut().filter(|p| p.paint == ShapePaint::Background) {
        path.color = if input.fill.is_some() { None } else { Some(color.trim().to_string()) };
        path.opacity = Some(opacity);
    }
    Ok(())
}

fn terminal(name: &str, half: f64, w: f64) -> NodeShape {
    let p = Point::new;
    let k = 0.5523 * half;
    let circle = vec![
        PathSegment::MoveTo(p(half, 0.0)),
        PathSegment::CurveTo(p(half, k), p(k, half), p(0.0, half)),
        PathSegment::CurveTo(p(-k, half), p(-half, k), p(-half, 0.0)),
        PathSegment::CurveTo(p(-half, -k), p(-k, -half), p(0.0, -half)),
        PathSegment::CurveTo(p(k, -half), p(half, -k), p(half, 0.0)),
        PathSegment::Close,
    ];
    let d = half * std::f64::consts::SQRT_2;
    let diamond = vec![PathSegment::MoveTo(p(0.0, d)), PathSegment::LineTo(p(-d, 0.0)), PathSegment::LineTo(p(0.0, -d)), PathSegment::LineTo(p(d, 0.0)), PathSegment::Close];
    let square = vec![
        PathSegment::MoveTo(p(-half, -half)),
        PathSegment::LineTo(p(half, -half)),
        PathSegment::LineTo(p(half, half)),
        PathSegment::LineTo(p(-half, half)),
        PathSegment::Close,
    ];
    let s = w / 2.0;
    let dot = vec![PathSegment::MoveTo(p(-s, -s)), PathSegment::LineTo(p(s, -s)), PathSegment::LineTo(p(s, s)), PathSegment::LineTo(p(-s, s)), PathSegment::Close];
    let (outline, open, r) = match name {
        "circ" => (circle, false, half),
        "ocirc" => (circle, true, half),
        "diamondpole" => (diamond, false, d),
        "odiamondpole" => (diamond, true, d),
        "squarepole" => (square, false, half),
        "osquarepole" => (square, true, half),
        _ => (dot, false, s),
    };
    let mut anchors = compass(-r, r, -r, r);
    anchors.push(("text".into(), p(0.0, 0.0)));
    let border = ShapeBorder::Outline(outline_points(&outline));
    let fill = if open { ShapePaint::Background } else { ShapePaint::Solid };
    let paths = if name == "rectjoinfill" {
        vec![ShapePath { segments: outline, paint: fill, ..Default::default() }]
    } else {
        vec![ShapePath { segments: outline.clone(), paint: fill, ..Default::default() }, ShapePath { segments: outline, paint: ShapePaint::Stroke(w), ..Default::default() }]
    };
    NodeShape { center: p(0.0, 0.0), anchors, border, text: p(0.0, 0.0), paths, extent: Vec::new(), ..Default::default() }
}

/// The points of an outline, its curves by their ends (enough for a border).
fn outline_points(segments: &[PathSegment]) -> Vec<Point> {
    let mut points = Vec::new();
    for s in segments {
        match *s {
            PathSegment::MoveTo(p) | PathSegment::LineTo(p) => points.push(p),
            PathSegment::CurveTo(a, b, c) => {
                // the middle of the curve too, so a circle's border is round enough
                let last = *points.last().unwrap_or(&c);
                let m = Point::new(0.125 * last.x + 0.375 * a.x + 0.375 * b.x + 0.125 * c.x, 0.125 * last.y + 0.375 * a.y + 0.375 * b.y + 0.125 * c.y);
                points.extend([m, c]);
            }
            PathSegment::Close => {}
        }
    }
    points
}

/// A dash pattern setting (`{{4pt}{4pt}{1pt}{4pt}}`, `none` for solid lines) as on and off
/// lengths in cm; `None` for `default`, the node's own.
fn dash(cx: &mut Context, spec: &str) -> Result<Option<Vec<f64>>, Error> {
    let spec = spec.trim();
    match spec {
        "" | "default" => return Ok(None),
        "none" => return Ok(Some(Vec::new())),
        _ => {}
    }
    let mut lengths = Vec::new();
    for part in spec.split(['{', '}']).map(str::trim).filter(|p| !p.is_empty()) {
        lengths.push(cx.length(Tokens::text(part))?);
    }
    Ok(Some(lengths))
}

/// The compass anchors of a box, and their short names (`n`, `ne`, `left`…).
fn compass(left: f64, right: f64, bottom: f64, top: f64) -> Vec<(String, Point)> {
    let (x, y) = ((left + right) / 2.0, (bottom + top) / 2.0);
    let p = Point::new;
    let mut out = Vec::new();
    for (names, at) in [
        (&["north", "n"][..], p(x, top)),
        (&["south", "s"], p(x, bottom)),
        (&["east", "e", "right"], p(right, y)),
        (&["west", "w", "left"], p(left, y)),
        (&["north east", "ne"], p(right, top)),
        (&["north west", "nw"], p(left, top)),
        (&["south east", "se"], p(right, bottom)),
        (&["south west", "sw"], p(left, bottom)),
    ] {
        for n in names {
            out.push((n.to_string(), at));
        }
    }
    out
}

/// Ops in `bipoles/length` as segments in cm.
fn segments(ops: &[Op], unit: f64) -> Vec<PathSegment> {
    let p = |x: f64, y: f64| Point::new(x * unit, y * unit);
    ops.iter()
        .map(|op| match *op {
            Op::Move(x, y) => PathSegment::MoveTo(p(x, y)),
            Op::Line(x, y) => PathSegment::LineTo(p(x, y)),
            Op::Curve(a, b, c) => PathSegment::CurveTo(p(a[0], a[1]), p(b[0], b[1]), p(c[0], c[1])),
            Op::Close => PathSegment::Close,
        })
        .collect()
}
