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

struct ChipBox {
    dip: bool,
    half_w: f64,
    half_h: f64,
    spacing: f64,
    count: usize,
}

/// Where each pin sits on the border.
fn chip_pins(chip: &ChipBox) -> Vec<(Point, Side)> {
    let ChipBox { dip, half_w, half_h, spacing: s, count: n } = *chip;
    let p = Point::new;
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
    pins
}

/// The chip's outline. A QFP chip's corner is cut when the orientation mark is on.
fn chip_outline(chip: &ChipBox, topmark: bool) -> Vec<PathSegment> {
    let ChipBox { dip, half_w, half_h, spacing: s, .. } = *chip;
    let p = Point::new;
    if dip {
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
    }
}

/// A DIP chip's notch: a semicircle a tenth of the width across, open at the top.
fn dip_notch(half_w: f64, half_h: f64) -> Vec<PathSegment> {
    let r = half_w / 5.0;
    let k = 0.5523 * r;
    let p = Point::new;
    vec![
        PathSegment::MoveTo(p(-r, half_h)),
        PathSegment::CurveTo(p(-r, half_h - k), p(-k, half_h - r), p(0.0, half_h - r)),
        PathSegment::CurveTo(p(k, half_h - r), p(r, half_h - k), p(r, half_h)),
    ]
}

/// One pin: its anchor, the point on the border, its lead or pad, and its number.
struct ChipPin {
    at: Point,
    side: Side,
    index: usize,
}

struct PinStyle<'a> {
    lead: f64,
    pad: f64,
    spacing: f64,
    leads: bool,
    numbers: bool,
    upright: bool,
    only: &'a Option<Vec<usize>>,
    font: &'a str,
}

struct DrawnPin {
    pin: (String, Point),
    border: (String, Point),
    leads: Vec<PathSegment>,
    text: Option<ShapeText>,
}

fn draw_chip_pin(pin: ChipPin, style: &PinStyle) -> DrawnPin {
    let ChipPin { at, side, index: k } = pin;
    let o = side.out();
    let p = Point::new;
    let drawn = style.leads && style.lead > 0.0 && style.only.as_ref().is_none_or(|set| set.contains(&k));
    // the pin is at the lead's end when drawn, and stays there without leads
    let end = p(at.x + o.x * style.lead, at.y + o.y * style.lead);
    let at_pin = if drawn || !style.leads { end } else { at };
    let mut leads = Vec::new();
    if drawn {
        // across the pin, for a pad
        let a = p(o.y.abs(), o.x.abs());
        if style.pad > 0.0 {
            let h = style.spacing / style.pad;
            leads.extend([
                PathSegment::MoveTo(p(at.x + a.x * h, at.y + a.y * h)),
                PathSegment::LineTo(p(end.x + a.x * h, end.y + a.y * h)),
                PathSegment::LineTo(p(end.x - a.x * h, end.y - a.y * h)),
                PathSegment::LineTo(p(at.x - a.x * h, at.y - a.y * h)),
            ]);
        } else {
            leads.extend([PathSegment::MoveTo(at), PathSegment::LineTo(end)]);
        }
    }
    let text = if style.numbers {
        let (point, anchor) = match side {
            Side::Left => (p(at.x + NUMBER_IN, at.y - NUMBER_DOWN), TextAnchor::BaseWest),
            Side::Right => (p(at.x - NUMBER_IN, at.y - NUMBER_DOWN), TextAnchor::BaseEast),
            Side::Bottom => (p(at.x, at.y + NUMBER_ABOVE_BOTTOM), TextAnchor::Base),
            Side::Top => (p(at.x, at.y - NUMBER_BELOW_TOP), TextAnchor::Base),
        };
        Some(ShapeText { text: k.to_string(), at: point, anchor, font: Some(style.font.to_string()), upright: style.upright, ..Default::default() })
    } else {
        None
    };
    DrawnPin { pin: (format!("pin {k}"), at_pin), border: (format!("bpin {k}"), at), leads, text }
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
    let chip = ChipBox { dip, half_w, half_h, spacing: s, count: n };
    let pins = chip_pins(&chip);

    let mut anchors = super::compass(super::Edges { left: -half_w, right: half_w, bottom: -half_h, top: half_h });
    anchors.push(("center".into(), p(0.0, 0.0)));
    anchors.push(("text".into(), p(0.0, 0.0)));
    anchors.push(("dot".into(), p(-half_w + 0.3 * s, half_h - 0.3 * s)));

    let outline = chip_outline(&chip, topmark);
    let mut paths = vec![
        ShapePath { segments: outline.clone(), paint: ShapePaint::Fill, ..Default::default() },
        ShapePath { segments: outline, paint: ShapePaint::Stroke(thick), ..Default::default() },
    ];
    // a DIP chip's mark: a notch in the top, a tenth of its width across the half
    if dip && topmark {
        paths.push(ShapePath { segments: dip_notch(half_w, half_h), paint: ShapePaint::Stroke(thick), ..Default::default() });
    }

    let style = PinStyle { lead, pad, spacing: s, leads, numbers, upright, only: &only, font: &font };
    let mut texts = Vec::new();
    let mut leads_path = Vec::new();
    for (i, &(at, side)) in pins.iter().enumerate() {
        let drawn = draw_chip_pin(ChipPin { at, side, index: i + 1 }, &style);
        anchors.push(drawn.pin);
        anchors.push(drawn.border);
        leads_path.extend(drawn.leads);
        if let Some(text) = drawn.text {
            texts.push(text);
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
