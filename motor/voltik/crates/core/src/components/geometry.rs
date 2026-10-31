//! Zig-zags, coils, arrow heads and the traces of an oscilloscope.

use super::Op;

/// A zig-zag of `count` teeth from `-half` to `half`, `high` on each side, the first tooth up, from
/// half a stroke (`thick`) before `-half` to as much after `half`.
pub struct Zigzag {
    pub half: f64,
    pub high: f64,
    pub thick: f64,
    pub count: usize,
}

pub fn zigzag(zag: Zigzag) -> Vec<Op> {
    let Zigzag { half, high, thick, count } = zag;
    let n = 2 * count.max(1);
    let step = 2.0 * half / n as f64;
    let mut ops = vec![Op::Move(-half - thick / 2.0, 0.0)];
    for i in 0..n {
        let y = if i % 2 == 0 { high } else { -high };
        ops.push(Op::Line(-half + (i as f64 + 0.5) * step, y));
    }
    ops.push(Op::Line(half + thick / 2.0, 0.0));
    ops
}

/// `arches` arches joined by small loops below the line, from `-half` to `half`: each arch is half an
/// ellipse `h` high, each loop half an ellipse half as high and as wide as half the arch's height.
pub fn coil(half: f64, h: f64, arches: usize) -> Vec<Op> {
    let back = h / 2.0;
    let arch = (2.0 * half + (arches - 1) as f64 * back) / arches as f64;
    let mut ops = vec![Op::Move(-half, 0.0)];
    let mut x = -half;
    for i in 0..arches {
        half_ellipse(&mut ops, x, arch / 2.0, h);
        x += arch;
        if i + 1 < arches {
            half_ellipse(&mut ops, x, -back / 2.0, -h / 2.0);
            x -= back;
        }
    }
    ops
}

/// `arches` arches side by side from `-half` to `half`, each half an ellipse `h` high.
pub fn arches(half: f64, h: f64, arches: usize) -> Vec<Op> {
    let arch = 2.0 * half / arches as f64;
    let mut ops = vec![Op::Move(-half, 0.0)];
    for i in 0..arches {
        half_ellipse(&mut ops, -half + i as f64 * arch, arch / 2.0, h);
    }
    ops
}

/// Half an ellipse from `(x, 0)` to `(x + 2rx, 0)` through `(x + rx, ry)`, as two quarter Béziers.
fn half_ellipse(ops: &mut Vec<Op>, x: f64, rx: f64, ry: f64) {
    const K: f64 = 0.552_284_75;
    let c = x + rx;
    ops.push(Op::Curve([x, K * ry], [c - K * rx, ry], [c, ry]));
    ops.push(Op::Curve([c + K * rx, ry], [c + rx, K * ry], [c + rx, 0.0]));
}

/// The head of the arrows of tunable components and potentiometers (CircuiTikZ's "latexslim"), for
/// a stroke `width` cm wide: its outline from the tip, as (back, side) pairs in units of its length,
/// and its length. The line stops `ARROW_INSET` of the length before the tip, inside the head.
pub const ARROW_HEAD: [[f64; 2]; 9] = [[0.25, 0.05], [0.70, 0.15], [1.0, 0.375], [0.75, 0.1], [0.75, -0.1], [1.0, -0.375], [0.70, -0.15], [0.25, -0.05], [0.0, 0.0]];
pub const ARROW_INSET: f64 = 0.6;

/// The length of an arrow head for a stroke `width` cm wide (measured: 2.79 bp and three widths).
pub fn arrow_length(width: f64) -> f64 {
    2.79 / 72.0 * 2.54 + 3.0 * width
}

/// The traces of an oscilloscope's screen (`bipoles/oscope/waveform`), measured at its default size.
pub const WAVEFORMS: &[(&str, &[Op])] = &[
    ("ramps", &[Op::Move(-0.225, -0.075), Op::Line(-0.015, 0.075), Op::Line(-0.015, -0.075), Op::Line(0.195, 0.075), Op::Line(0.195, -0.075)]),
    (
        "sin",
        &[
            Op::Move(-0.18, 0.0),
            Op::Curve([-0.1507, 0.0614], [-0.1226, 0.12], [-0.0901, 0.12]),
            Op::Curve([-0.0575, 0.12], [-0.0294, 0.0614], [0.0, 0.0]),
            Op::Curve([0.0294, -0.0614], [0.0574, -0.12], [0.09, -0.12]),
            Op::Curve([0.1226, -0.12], [0.1506, -0.0614], [0.18, 0.0]),
        ],
    ),
    (
        "square",
        &[
            Op::Move(-0.225, -0.075),
            Op::Line(-0.18, -0.075),
            Op::Line(-0.18, 0.075),
            Op::Line(0.0, 0.075),
            Op::Line(0.0, -0.075),
            Op::Line(0.18, -0.075),
            Op::Line(0.18, 0.075),
            Op::Line(0.225, 0.075),
        ],
    ),
    (
        "triangle",
        &[Op::Move(-0.225, 0.0), Op::Line(-0.18, -0.075), Op::Line(-0.09, 0.075), Op::Line(0.0, -0.075), Op::Line(0.09, 0.075), Op::Line(0.18, -0.075), Op::Line(0.225, 0.0)],
    ),
    (
        "lissajous",
        &[
            Op::Move(0.15, 0.105),
            Op::Curve([0.1003, 0.1382], [-0.0072, 0.118], [-0.09, 0.0601]),
            Op::Curve([-0.1729, 0.0021], [-0.1997, -0.0719], [-0.15, -0.105]),
            Op::Curve([-0.1003, -0.1381], [0.0072, -0.118], [0.09, -0.0599]),
            Op::Curve([0.1729, -0.002], [0.1997, 0.0719], [0.15, 0.105]),
            Op::Close,
        ],
    ),
    ("zero", &[Op::Move(-0.225, 0.0), Op::Line(0.225, 0.0)]),
    ("none", &[]),
];
