//! Chips (manual 4.21): `dipchip` and `qfpchip`, with `num pins` pins, their leads or pads, the
//! orientation mark and the pins' numbers.
//!
//! Measured on what CircuiTikZ 1.8.7 draws (`tools/oracle/`): a DIP chip is `multipoles/dipchip/width`
//! wide and half its pins times their spacing high, a QFP chip a square with a quarter of its pins
//! and half a spacing on each side and its corner cut by half a spacing; the spacings are in
//! `bipoles/length`, as the leads (`external pins width`) are, and everything scales by `chips/scale`
//! but the numbers.

use tikra::extension::{Context, Error, NodeShape, PathSegment, Point, ShapeBorder, ShapeInput, ShapePaint, ShapePath, ShapeText, TextAnchor, Tokens};

use crate::bipole::{number, setting};

/// Where a pin's number goes from its pin on the border (cm, in `\tiny`): inside, by its baseline.
const NUMBER_IN: f64 = 0.0829;
const NUMBER_DOWN: f64 = 0.0718;
const NUMBER_ABOVE_BOTTOM: f64 = 0.0956;
const NUMBER_BELOW_TOP: f64 = 0.2392;

/// The side of the chip a pin is on, by the way out of it.
#[derive(Clone, Copy)]
enum Side {
    Left,
    Bottom,
    Right,
    Top,
}

impl Side {
    fn out(self) -> Point {
        match self {
            Side::Left => Point::new(-1.0, 0.0),
            Side::Bottom => Point::new(0.0, -1.0),
            Side::Right => Point::new(1.0, 0.0),
            Side::Top => Point::new(0.0, 1.0),
        }
    }
}

/// `dipchip` and `qfpchip`.
pub fn chip_shape(cx: &mut Context, input: &ShapeInput) -> Result<NodeShape, Error> {
    let dip = input.name == "dipchip";
    let length = setting(cx, "bipoles/length")?;
    let u = cx.length(Tokens::text(&length)).unwrap_or(1.4) * number(cx, "chips/scale", 1.0);
    let n = (number(cx, "multipoles/num pins", 8.0).round().max(0.0)) as usize;
    let s = number(cx, if dip { "multipoles/dipchip/pin spacing" } else { "multipoles/qfpchip/pin spacing" }, 0.4) * u;
    let (half_w, half_h) = if dip {
        (number(cx, "multipoles/dipchip/width", 1.2) * u / 2.0, (n / 2) as f64 * s / 2.0)
    } else {
        let side = ((n / 4) as f64 + 0.5) * s / 2.0;
        (side, side)
    };
    let lw = input.line_width;
    let thick = number(cx, "multipoles/thickness", 2.0) * lw;
    let thin = number(cx, "multipoles/external pins thickness", 1.0) * lw;
    let lead = number(cx, "multipoles/external pins width", 0.2) * u;
    let pad = number(cx, "multipoles/external pad fraction", 0.0);
    let leads = setting(cx, "tr/no input leads")? != "true";
    let topmark = setting(cx, "multipoles/topmark")? != "false";
    let numbers = setting(cx, "multipoles/numbers")? != "false";
    let only = drawn_pins(&setting(cx, "multipoles/draw only pins")?);
    let upright = setting(cx, "multipoles/straight numbers")? != "false";
    let font = cx.expand(Tokens::text("\\unexpanded\\expandafter{\\romannumeral-`0\\pgfkeysvalueof{/tikz/circuitikz/multipoles/font}}"))?;
    let p = Point::new;

    // the pins, by number: where on the border, and on which side
    let mut pins: Vec<(Point, Side)> = Vec::with_capacity(n);
    if dip {
        let k = n / 2;
        for i in 0..k {
            pins.push((p(-half_w, half_h - s / 2.0 - i as f64 * s), Side::Left));
        }
        for i in 0..n - k {
            pins.push((p(half_w, -half_h + s / 2.0 + i as f64 * s), Side::Right));
        }
    } else {
        let q = n / 4;
        let at = |k: usize| (k as f64 - (q as f64 - 1.0) / 2.0) * s;
        for (side, place) in [(Side::Left, 0), (Side::Bottom, 1), (Side::Right, 2), (Side::Top, 3)] {
            for k in 0..q {
                let point = match place {
                    0 => p(-half_w, -at(k)),
                    1 => p(at(k), -half_h),
                    2 => p(half_w, at(k)),
                    _ => p(-at(k), half_h),
                };
                pins.push((point, side));
            }
        }
    }

    let mut anchors = super::compass(-half_w, half_w, -half_h, half_h);
    anchors.push(("center".into(), p(0.0, 0.0)));
    anchors.push(("text".into(), p(0.0, 0.0)));
    anchors.push(("dot".into(), p(-half_w + 0.3 * s, half_h - 0.3 * s)));

    let outline = if dip {
        vec![
            PathSegment::MoveTo(p(-half_w, -half_h)),
            PathSegment::LineTo(p(half_w, -half_h)),
            PathSegment::LineTo(p(half_w, half_h)),
            PathSegment::LineTo(p(-half_w, half_h)),
            PathSegment::Close,
        ]
    } else {
        let cut = s / 2.0;
        vec![
            PathSegment::MoveTo(p(-half_w, half_h - if topmark { cut } else { 0.0 })),
            PathSegment::LineTo(p(-half_w + if topmark { cut } else { 0.0 }, half_h)),
            PathSegment::LineTo(p(half_w, half_h)),
            PathSegment::LineTo(p(half_w, -half_h)),
            PathSegment::LineTo(p(-half_w, -half_h)),
            PathSegment::Close,
        ]
    };
    let mut paths = vec![
        ShapePath { segments: outline.clone(), paint: ShapePaint::Fill, ..Default::default() },
        ShapePath { segments: outline, paint: ShapePaint::Stroke(thick), ..Default::default() },
    ];
    // a DIP chip's mark: a notch in the top, a tenth of its width across the half
    if dip && topmark {
        let r = half_w / 5.0;
        let k = 0.5523 * r;
        paths.push(ShapePath {
            segments: vec![
                PathSegment::MoveTo(p(-r, half_h)),
                PathSegment::CurveTo(p(-r, half_h - k), p(-k, half_h - r), p(0.0, half_h - r)),
                PathSegment::CurveTo(p(k, half_h - r), p(r, half_h - k), p(r, half_h)),
            ],
            paint: ShapePaint::Stroke(thick),
            ..Default::default()
        });
    }

    let mut texts = Vec::new();
    let mut leads_path = Vec::new();
    for (i, &(at, side)) in pins.iter().enumerate() {
        let k = i + 1;
        let o = side.out();
        let drawn = leads && lead > 0.0 && only.as_ref().is_none_or(|set| set.contains(&k));
        // the pin is at the lead's end when drawn, and stays there without leads
        let end = p(at.x + o.x * lead, at.y + o.y * lead);
        let pin = if drawn || !leads { end } else { at };
        anchors.push((format!("pin {k}"), pin));
        anchors.push((format!("bpin {k}"), at));
        if drawn {
            // across the pin, for a pad
            let a = p(o.y.abs(), o.x.abs());
            if pad > 0.0 {
                let h = s / pad;
                leads_path.extend([
                    PathSegment::MoveTo(p(at.x + a.x * h, at.y + a.y * h)),
                    PathSegment::LineTo(p(end.x + a.x * h, end.y + a.y * h)),
                    PathSegment::LineTo(p(end.x - a.x * h, end.y - a.y * h)),
                    PathSegment::LineTo(p(at.x - a.x * h, at.y - a.y * h)),
                ]);
            } else {
                leads_path.extend([PathSegment::MoveTo(at), PathSegment::LineTo(end)]);
            }
        }
        if numbers {
            let (point, anchor) = match side {
                Side::Left => (p(at.x + NUMBER_IN, at.y - NUMBER_DOWN), TextAnchor::BaseWest),
                Side::Right => (p(at.x - NUMBER_IN, at.y - NUMBER_DOWN), TextAnchor::BaseEast),
                Side::Bottom => (p(at.x, at.y + NUMBER_ABOVE_BOTTOM), TextAnchor::Base),
                Side::Top => (p(at.x, at.y - NUMBER_BELOW_TOP), TextAnchor::Base),
            };
            texts.push(ShapeText { text: k.to_string(), at: point, anchor, font: Some(font.clone()), upright, ..Default::default() });
        }
    }
    if !leads_path.is_empty() {
        paths.push(ShapePath { segments: leads_path, paint: ShapePaint::Stroke(thin), ..Default::default() });
    }
    Ok(NodeShape {
        center: p(0.0, 0.0),
        anchors,
        border: ShapeBorder::Rectangle(p(half_w, half_h)),
        text: p(-input.width / 2.0, -(input.height - input.depth) / 2.0),
        paths,
        texts,
        extent: Vec::new(),
    })
}

/// The pins `multipoles/draw only pins` keeps (`1, 3, 5-8`); `None` for all of them.
pub(super) fn drawn_pins(spec: &str) -> Option<Vec<usize>> {
    let spec = spec.trim();
    if spec.is_empty() || spec == "all" {
        return None;
    }
    let mut pins = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        match part.split_once('-') {
            Some((a, b)) => {
                if let (Ok(a), Ok(b)) = (a.trim().parse::<usize>(), b.trim().parse::<usize>()) {
                    pins.extend(a..=b);
                }
            }
            None => pins.extend(part.parse::<usize>().ok()),
        }
    }
    Some(pins)
}
