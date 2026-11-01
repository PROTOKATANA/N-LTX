use tikra_engine::canvas::Paint;
use tikra_engine::color::Color;
use tikra_engine::scene::Point;
use tikra_engine::Error;

use super::super::draw::Frame;
use super::super::plot::Plot;
use super::super::Engine;

/// A color made from `mapped color`: `base + mapped × slope`, channel by channel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Linear {
    pub base: Color,
    pub slope: Color,
}

impl Linear {
    pub fn from(black: Color, white: Color) -> Linear {
        Linear { base: black, slope: Color::rgb(white.r - black.r, white.g - black.g, white.b - black.b) }
    }
    pub fn at(&self, m: Color) -> Color {
        let c = |b: f64, s: f64, m: f64| (b + s * m).clamp(0.0, 1.0);
        Color::rgb(c(self.base.r, self.slope.r, m.r), c(self.base.g, self.slope.g, m.g), c(self.base.b, self.slope.b, m.b))
    }
}

/// The state's colors as functions of `mapped color`: (draw, fill) after `f` runs in a group.
pub fn mapped_colors(e: &mut Engine, mut f: impl FnMut(&mut Engine) -> Result<(), Error>) -> Result<(Linear, Linear, tikra_engine::engine::GState), Error> {
    let mut states = Vec::new();
    // (pgfplots's `mapped color` is an rgb color)
    for m in [Color::rgb(0.0, 0.0, 0.0), Color::rgb(1.0, 1.0, 1.0)] {
        e.colors.insert("mapped color".into(), m);
        e.begin_group();
        let r = f(e);
        let st = e.st.clone();
        e.end_group();
        r?;
        states.push(st);
    }
    // what is drawn later with `mapped color` (nodes near coords) gets TeX's black
    e.colors.insert("mapped color".into(), Color::rgb(0.0, 0.0, 0.0));
    let draw = Linear::from(states[0].stroke_color(), states[1].stroke_color());
    let fill = Linear::from(states[0].fill_paint_color(), states[1].fill_paint_color());
    Ok((draw, fill, (*states.swap_remove(1)).clone()))
}

/// A color as a function of `mapped color`.
pub(super) fn mapped_named(e: &mut Engine, name: &str) -> Option<Linear> {
    e.colors.insert("mapped color".into(), Color::rgb(0.0, 0.0, 0.0));
    let a = e.color(name)?;
    e.colors.insert("mapped color".into(), Color::rgb(1.0, 1.0, 1.0));
    let b = e.color(name);
    e.colors.insert("mapped color".into(), Color::rgb(0.0, 0.0, 0.0));
    Some(Linear::from(a, b?))
}

/// The colormap's color for meta value `m`.
pub fn map(f: &Frame, m: f64) -> Color {
    match &f.colormap {
        Some(c) => c.color_of(m, f.meta[0], f.meta[1]),
        None => Color::BLACK,
    }
}

pub fn explicit_color(e: &Engine, s: &str) -> Option<Color> {
    let s = s.trim();
    let (model, rest) = match s.split_once('=') {
        Some((m, r)) => (m.trim(), r.trim()),
        None => ("rgb", s),
    };
    // (values of \thisrow come in parentheses)
    let nums: Vec<f64> = rest.split(',').filter_map(|x| x.trim().trim_start_matches('(').trim_end_matches(')').trim().parse().ok()).collect();
    let n = |i: usize| nums.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0);
    Some(match model {
        "color" => return e.color(rest).map(|c| c.to_rgb()),
        "rgb" => Color::rgb(n(0), n(1), n(2)),
        "rgb255" => Color::rgb(nums.first()? / 255.0, nums.get(1)? / 255.0, nums.get(2)? / 255.0),
        "gray" => Color::rgb(n(0), n(0), n(0)),
        "cmyk" => Color::rgb((1.0 - n(0)) * (1.0 - n(3)), (1.0 - n(1)) * (1.0 - n(3)), (1.0 - n(2)) * (1.0 - n(3))),
        "cmy" => Color::rgb(1.0 - n(0), 1.0 - n(1), 1.0 - n(2)),
        "wave" => wave(*nums.first()?),
        _ => return None,
    })
}

/// xcolor's `wave` model: a wavelength in nm to RGB (Bruton's approximation, as xcolor does).
pub(super) fn wave(l: f64) -> Color {
    let (r, g, b) = if l < 440.0 {
        ((440.0 - l) / 60.0, 0.0, 1.0)
    } else if l < 490.0 {
        (0.0, (l - 440.0) / 50.0, 1.0)
    } else if l < 510.0 {
        (0.0, 1.0, (510.0 - l) / 20.0)
    } else if l < 580.0 {
        ((l - 510.0) / 70.0, 1.0, 0.0)
    } else if l < 645.0 {
        (1.0, (645.0 - l) / 65.0, 0.0)
    } else {
        (1.0, 0.0, 0.0)
    };
    let f = if l < 420.0 {
        0.3 + 0.7 * (l - 380.0) / 40.0
    } else if l > 700.0 {
        0.3 + 0.7 * (780.0 - l) / 80.0
    } else {
        1.0
    };
    let c = |v: f64| (v * f).clamp(0.0, 1.0).powf(0.8);
    Color::rgb(c(r), c(g), c(b))
}

/// A triangle of `shader=interp`: pgfplots hands the point metas to a Gouraud shading (type 4)
/// whose function is the colormap, so the meta is affine across the triangle and the color
/// changes along the meta's gradient only. That is a linear gradient, exact: the colormap's own
/// stops between the triangle's lowest and highest meta.
pub(super) fn gouraud(tri: &[(Point, f64); 3], cm: &super::super::colormap::Colormap, range: [f64; 2], fill: &Linear) -> Paint {
    let color = |m: f64| fill.at(cm.color_of(m, range[0], range[1]));
    // a meta out of the range is taken into it at the corner (the shading's domain), before the
    // shading interpolates
    let tri = &tri.map(|(q, m)| (q, m.clamp(range[0].min(range[1]), range[1].max(range[0]))));
    let (p0, m0) = tri[0];
    let (lo, hi) = tri.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), t| (lo.min(t.1), hi.max(t.1)));
    let (e1, e2) = (tri[1].0.minus(p0), tri[2].0.minus(p0));
    let det = e1.x * e2.y - e1.y * e2.x;
    if det.abs() < 1e-12 || hi - lo <= 1e-12 * (lo.abs() + hi.abs() + 1.0) {
        return Paint::Solid(color((tri[0].1 + tri[1].1 + tri[2].1) / 3.0));
    }
    // the gradient g of the meta on the canvas: e1·g = d1, e2·g = d2
    let (d1, d2) = (tri[1].1 - m0, tri[2].1 - m0);
    let g = Point::new((d1 * e2.y - d2 * e1.y) / det, (e1.x * d2 - e2.x * d1) / det);
    let g2 = g.x * g.x + g.y * g.y;
    // from the point of the lowest meta to that of the highest, along g
    let a = p0.plus(g.scale((lo - m0) / g2));
    let u = g.scale((hi - lo) / g2);
    let transform = tikra_engine::scene::Transform { a: -u.y / 100.0, b: u.x / 100.0, c: u.x / 100.0, d: u.y / 100.0, e: a.x + 0.5 * u.x, f: a.y + 0.5 * u.y };
    let mut metas = vec![lo, hi];
    for (t, _) in cm.normalized() {
        let m = range[0] + t * (range[1] - range[0]);
        if m > lo && m < hi {
            metas.push(m);
        }
    }
    metas.sort_by(f64::total_cmp);
    let stops = metas.iter().map(|&m| ((m - lo) / (hi - lo), color(m))).collect();
    Paint::Shading(tikra_engine::canvas::Shading::Axial { transform, stops })
}

pub(super) fn mean(v: &[Color]) -> Color {
    let n = v.len().max(1) as f64;
    Color::rgb(v.iter().map(|c| c.r).sum::<f64>() / n, v.iter().map(|c| c.g).sum::<f64>() / n, v.iter().map(|c| c.b).sum::<f64>() / n)
}

pub(super) fn blend(v: &[Color], w: &[f64]) -> Color {
    let mut c = Color::rgb(0.0, 0.0, 0.0);
    for (k, x) in v.iter().zip(w) {
        c.r += k.r * x;
        c.g += k.g * x;
        c.b += k.b * x;
    }
    c
}

/// The color of point `k` of an explicit color input: `color=red`, `rgb=…`, `R,G,B`, `rgb255=`,
/// `cmyk=`, `gray=`, `wave=` (nanometres), as written or computed by the survey; None for `[]`.
pub(super) fn vertex_color(e: &Engine, p: &Plot, k: usize) -> Option<Color> {
    let Some(label) = p.coords.get(k).filter(|c| c.label != super::super::plot::NO_LABEL).and_then(|c| p.labels.get(c.label as usize)) else { return Some(Color::BLACK) };
    let s = tikra_engine::tex::tokens_to_string(label);
    if s.trim().is_empty() {
        return None;
    }
    Some(explicit_color(e, &s).unwrap_or(Color::BLACK))
}
