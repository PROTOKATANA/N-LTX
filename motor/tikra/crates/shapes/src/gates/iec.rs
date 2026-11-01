//! The IEC gates: a rectangle and the gate's symbol.

use crate::scene::{Point, Segment};

use super::super::basic::ellipse_path;
use super::super::{Border, Geometry, Input, ShapeText};
use super::common::*;

/// The IEC gates: `kind` is `and`, `nand`, `or`, `nor`, `xor`, `xnor`, `not` or `buffer`. A rectangle
/// around the text and the gate's symbol (`/pgf/and gate IEC symbol`…, placed by `/pgf/logic gate
/// IEC symbol align`), with a circle at the output of the inverting gates.
pub fn gate_iec(input: &mut Input, kind: &str) -> Geometry {
    let single = kind == "not" || kind == "buffer";
    let inverted = matches!(kind, "nand" | "nor" | "xnor" | "not");
    let cm = common(
        input,
        match kind {
            "xor" => 2,
            _ if single => 1,
            _ => 1024,
        },
    );
    let c = cm.c;
    // \pgf@lib@sh@logicgates@dimensions@IEC (the buffer gate measures the xor gate's symbol)
    let sized = if kind == "buffer" { "xor" } else { kind };
    let symbol_of = |input: &mut Input, k: &str| input.keys.text(&format!("/pgf/{k} gate IEC symbol"));
    let sized_symbol = symbol_of(input, sized);
    let sb = input.keys.text_box(&sized_symbol);
    let t = input.text();
    let mut x = (0.5 * t.wd).max(0.5 * sb.wd) + input.inner_xsep;
    let mut y = (0.5 * (t.ht + t.dp)).max(0.5 * (sb.ht + sb.dp)) + input.inner_ysep;
    x = x.max(0.5 * input.minimum_width);
    y = y.max(0.5 * input.minimum_height);
    let n1 = (cm.inputs.len() + 1) as f64;
    let yb = (2.0 * cm.r + cm.lw).max(cm.sep) * n1;
    y = y.max(0.5 * yb);
    let (ihw, ihh) = (x - input.inner_xsep, y - input.inner_ysep);
    let (ox, oy) = (input.outer_xsep, input.outer_ysep);
    let (w, hh) = (x + ox, y + oy);
    let mut g = Geometry::new(c, Border::Rectangle(Point::new(w, hh)));
    text_anchors(&mut g, input);
    let p = |dx: f64, dy: f64| c.plus(Point::new(dx, dy));
    for (name, q) in [
        ("north", p(0.0, hh)),
        ("south", p(0.0, -hh)),
        ("east", p(w, 0.0)),
        ("west", p(-w, 0.0)),
        ("north east", p(w, hh)),
        ("north west", p(-w, hh)),
        ("south east", p(w, -hh)),
        ("south west", p(-w, -hh)),
    ] {
        g.anchor_at(name, q);
    }
    side_anchors(&mut g, input, "east", c.x + w);
    side_anchors(&mut g, input, "west", c.x - w);
    g.anchor_at("output", p(w + if inverted { cm.r + cm.outer_r } else { 0.0 }, 0.0));
    input_anchors(&mut g, &cm, InputSites { single, half: hh, at_x: |_, _| -w });
    // the background: the rectangle (in by the outer x sep, also vertically), the inputs' circles
    let (xc, yc) = (w - ox, hh - ox);
    let mut v = vec![Segment::MoveTo(p(-xc, -yc)), Segment::LineTo(p(xc, -yc)), Segment::LineTo(p(xc, yc)), Segment::LineTo(p(-xc, yc)), Segment::Close];
    input_circles(&mut v, &cm, hh, |_| -w - cm.r);
    if inverted {
        v.extend(ellipse_path(p(w + cm.r, 0.0), cm.r, cm.r));
    }
    g.background(v);
    // the symbol, by \pgftext at the center moved to the side of the alignment
    let symbol = symbol_of(input, kind);
    let b = input.keys.text_box(&symbol);
    let align = input.keys.text("/pgf/logic gate IEC symbol align");
    // each word moves the point to that side and puts that side of the text there, as
    // `\pgftext[bottom,right]` (`top` and `bottom` the text's top and bottom, not its baseline)
    let (mut x, mut y) = (-b.wd / 2.0, -(b.ht - b.dp) / 2.0);
    for word in align.split(',').map(str::trim) {
        match word {
            "top" => y = ihh - b.ht,
            "bottom" => y = -ihh + b.dp,
            "left" => x = -ihw,
            "right" => x = ihw - b.wd,
            _ => {}
        }
    }
    let at = p(x, y);
    let color = input.keys.text("/pgf/logic gate IEC symbol color");
    g.texts.push(ShapeText { tex: symbol, at, color: Some(color).filter(|c| !c.trim().is_empty()), anchor: None, font: None, turn: 0.0, upright: false });
    g
}
