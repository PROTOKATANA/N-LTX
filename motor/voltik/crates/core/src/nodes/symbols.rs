//! The dc and ac symbols (manual 4.24.2): `dc symbol`, two lines, the lower in `dc symbol/segments`
//! dashes, and `ac symbol`, a wave; `…/width` by `…/height` in `bipoles/length`, in the `misc`
//! class, with their text centered `…/text offset` above their height.
//!
//! Measured on what CircuiTikZ 1.8.7 draws (`tools/oracle/`).

use tikra::extension::{Context, Error, NodeShape, PathSegment, Point, ShapeBorder, ShapeInput, ShapePaint, ShapePath, Tokens};

use crate::bipole::{number, setting};

/// The ac symbol's wave, for a symbol 1 wide and 1 high.
const WAVE: [[f64; 2]; 13] = [
    [-0.5, 0.0],
    [-0.4186, 0.2557],
    [-0.3405, 0.5],
    [-0.25, 0.5],
    [-0.1595, 0.5],
    [-0.0814, 0.2557],
    [0.0, 0.0],
    [0.0814, -0.2557],
    [0.1595, -0.5],
    [0.25, -0.5],
    [0.3405, -0.5],
    [0.4183, -0.2557],
    [0.5, 0.0],
];

/// `dc symbol` and `ac symbol`.
pub fn symbol_shape(cx: &mut Context, input: &ShapeInput) -> Result<NodeShape, Error> {
    let kind = input.name.as_str();
    let length = setting(cx, "bipoles/length")?;
    let u = cx.length(Tokens::text(&length)).unwrap_or(1.4) * number(cx, "misc/scale", 1.0);
    let w = number(cx, &format!("{kind}/width"), 0.3) * u;
    let h = number(cx, &format!("{kind}/height"), 0.15) * u;
    let offset = setting(cx, &format!("{kind}/text offset"))?;
    let offset = cx.length(Tokens::text(&offset)).unwrap_or(6.0 * tikra::extension::PT);
    let thick = number(cx, "misc/thickness", 2.0) * input.line_width;
    let p = Point::new;
    let (x, y) = (w / 2.0, h / 2.0);
    let mut paths = Vec::new();
    if kind == "dc symbol" {
        paths.push(ShapePath { segments: vec![PathSegment::MoveTo(p(-x, y)), PathSegment::LineTo(p(x, y))], paint: ShapePaint::Stroke(thick), ..Default::default() });
        // the lower line in dashes as long as the gaps between them
        let n = number(cx, "dc symbol/segments", 2.0).round().max(1.0) as usize;
        let dash = w / (2 * n - 1) as f64;
        let mut lower = Vec::new();
        for i in 0..n {
            let a = -x + 2.0 * i as f64 * dash;
            lower.extend([PathSegment::MoveTo(p(a, -y)), PathSegment::LineTo(p(a + dash, -y))]);
        }
        let k = number(cx, "dc symbol/relative thickness", 1.0);
        paths.push(ShapePath { segments: lower, paint: ShapePaint::Stroke(thick * k), ..Default::default() });
    } else {
        let q = |i: usize| p(WAVE[i][0] * w, WAVE[i][1] * h);
        let mut wave = vec![PathSegment::MoveTo(q(0))];
        for i in (1..13).step_by(3) {
            wave.push(PathSegment::CurveTo(q(i), q(i + 1), q(i + 2)));
        }
        paths.push(ShapePath { segments: wave, paint: ShapePaint::Stroke(thick), ..Default::default() });
    }
    let mut anchors = super::compass(-x, x, -y, y);
    anchors.push(("center".into(), p(0.0, 0.0)));
    anchors.push(("text".into(), p(0.0, h + offset)));
    Ok(NodeShape {
        center: p(0.0, 0.0),
        anchors,
        border: ShapeBorder::Rectangle(p(x, y)),
        text: p(-input.width / 2.0, h + offset - (input.height - input.depth) / 2.0),
        paths,
        ..Default::default()
    })
}
