//! `\addplot graphics[options] {file};` and `\addplot3 graphics`, as `pgfplotscoordprocessing.code.tex`
//! does them: a PNG image placed in the axis. In 2d it fills the box from (`xmin`, `ymin`) to
//! (`xmax`, `ymax`); with `points={(x,y,z) => (px,py) …}` the first two points with image
//! coordinates (in pt from the image's lower left corner) give its scale and position, and four of
//! them set the axis' unit vectors so that the axis projects as the image does (`auto adjust
//! axis`). The points, or the corners, count in the axis' limits.
//!
//! Only PNG files are drawn (PDF and EPS images need a renderer); the file is found as tables are
//! (`TIKRA_INPUT_DIR`).

use std::rc::Rc;

use tikra_engine::canvas::Item;
use tikra_engine::scene::{Point, Segment, Transform};
use tikra_engine::Error;
use tikra_engine::PT;

use tikra_engine::engine::graphics::{find_image, natural_size, paren_group, png_info, solve, split_top, Graphics, MapPoint, BP, MAX_IMAGE};

use super::keys;
use super::plot::{Coord, Plot};
use super::{Engine, Frame};
/// Reads `graphics[options] {file}` and adds the points that count in the limits.
pub fn survey(e: &mut Engine, plot: &mut Plot, three_d: bool) -> Result<(), Error> {
    if let Some(o) = e.optional()? {
        let saved = e.st.key_path.clone();
        let r = e.process_keys(o, Some("/pgfplots/plot graphics"));
        e.st.key_path = saved;
        r?;
    }
    let name = e.argument_string()?;
    let mut g = Graphics::default();
    match find_image(&name).and_then(|f| tikra_engine::files::read_bytes(&f, &[], MAX_IMAGE).map(|d| (f, d))) {
        Ok((file, data)) => match png_info(&data) {
            Some((w, h, xr, yr)) if w > 0 && h > 0 => {
                g.natural = natural_size(w, h, xr, yr);
                g.png = Some(Rc::new(data));
            }
            _ => e.warn(format!("\\addplot graphics: '{file}' is not a PNG image; only PNG images are drawn")),
        },
        Err(m) => e.warn(format!("\\addplot graphics: {m}")),
    }
    includegraphics_options(e, &mut g);
    let gv = |e: &mut Engine, k: &str| keys::value(e, &format!("/pgfplots/plot graphics/{k}"));
    let points = gv(e, "points");
    g.points = parse_points(e, &points)?;
    for (p, _) in &g.points {
        if !three_d || p[2].is_finite() {
            plot.coords.push(Coord::new(p[0], p[1], if three_d { p[2] } else { f64::NAN }));
        }
    }
    // the lower left and upper right corners, when given (required without `points`)
    let axes: &[&str] = if three_d { &["x", "y", "z"] } else { &["x", "y"] };
    for end in ["min", "max"] {
        let mut c = [f64::NAN; 3];
        let mut missing = None;
        for (k, a) in axes.iter().enumerate() {
            let v = gv(e, &format!("{a}{end}"));
            match e.number(&v) {
                Ok(x) if !v.trim().is_empty() => c[k] = x,
                _ => missing = missing.or(Some(format!("{a}{end}"))),
            }
        }
        match missing {
            None => plot.coords.push(Coord::new(c[0], c[1], c[2])),
            Some(k) if g.points.is_empty() => {
                e.warn(format!("\\addplot graphics: the key '{k}' is missing: no place for the image (give xmin, xmax, ymin, ymax or points)"));
                break;
            }
            Some(_) => {}
        }
    }
    if three_d && keys::value(e, "/pgfplots/plot graphics/auto adjust axis") == "true" {
        plot.late = required_view(e, &g.points);
    }
    plot.graphics = Some(Box::new(g));
    Ok(())
}

/// `trim` and `clip` of `includegraphics={…}` (numbers in bp, as graphicx reads them).
fn includegraphics_options(e: &mut Engine, g: &mut Graphics) {
    let text = keys::value(e, "/pgfplots/plot graphics/includegraphics");
    for part in text.split(',') {
        let (k, v) = part.split_once('=').map_or((part.trim(), ""), |(k, v)| (k.trim(), v.trim()));
        match k {
            "clip" => g.clip = v.is_empty() || v == "true",
            "trim" => {
                for (i, x) in v.split_whitespace().take(4).enumerate() {
                    g.trim[i] = match x.parse::<f64>() {
                        Ok(n) => n * BP,
                        Err(_) => e.length(x).unwrap_or(0.0),
                    };
                }
            }
            "" => {}
            other => e.warn(format!("\\addplot graphics: includegraphics option '{other}' ignored")),
        }
    }
}

/// `(x,y[,z]) [=> (px,py)] …`: data points, with image coordinates if mapped.
fn parse_points(e: &mut Engine, text: &str) -> Result<Vec<MapPoint>, Error> {
    let mut out = Vec::new();
    let mut rest = text.trim();
    while let Some(start) = rest.find('(') {
        let Some((inner, after)) = paren_group(&rest[start..]) else { break };
        let parts = split_top(inner);
        let num = |e: &mut Engine, s: &str| e.number(s.trim());
        let x = num(e, parts.first().copied().unwrap_or(""))?;
        let y = num(e, parts.get(1).copied().unwrap_or(""))?;
        let z = match parts.get(2) {
            Some(s) if !s.trim().is_empty() => num(e, s)?,
            _ => f64::NAN,
        };
        rest = after.trim_start();
        let mut img = None;
        if let Some(r) = rest.strip_prefix("=>") {
            let r = r.trim_start();
            if let Some((inner, after)) = paren_group(r).filter(|_| r.starts_with('(')) {
                let parts = split_top(inner);
                if parts.len() == 2 && !parts[1].trim().is_empty() {
                    img = Some((num(e, parts[0])?, num(e, parts[1])?));
                }
                rest = after;
            }
        }
        out.push(([x, y, z], img));
    }
    Ok(out)
}
/// `\pgfplotsplothandlergraphicspointmapcomputerequiredview`: the projection T(x,y,z) = o + x e_x +
/// y e_y + z e_z of the image, solved from four points with image coordinates, as the axis' unit
/// vectors, scaled uniformly. Empty if there are not four such 3d points.
fn required_view(e: &mut Engine, points: &[MapPoint]) -> Vec<tikra_engine::tex::Token> {
    let mapped: Vec<([f64; 3], (f64, f64))> = points.iter().filter_map(|(p, i)| i.map(|i| (*p, i))).take(4).collect();
    if mapped.len() < 4 {
        return Vec::new();
    }
    if mapped.iter().any(|(p, _)| !p[2].is_finite()) {
        return Vec::new();
    }
    // unknowns: exx exy eyx eyy ezx ezy ox oy
    let mut m = [[0.0f64; 9]; 8];
    for (k, (p, (ix, iy))) in mapped.iter().enumerate() {
        m[2 * k] = [p[0], 0.0, p[1], 0.0, p[2], 0.0, 1.0, 0.0, *ix];
        m[2 * k + 1] = [0.0, p[0], 0.0, p[1], 0.0, p[2], 0.0, 1.0, *iy];
    }
    let Some(s) = solve(m) else {
        e.warn("\\addplot graphics: the points are not linearly independent; the axis is not adjusted to the image");
        return Vec::new();
    };
    let (mut ex, mut ey, mut ez) = ((s[0], s[1]), (s[2], s[3]), (s[4], s[5]));
    // `snap z`: a z unit vector that is nearly vertical is made vertical
    let snap = keys::value(e, "/pgfplots/plot graphics/snap z");
    if let Ok(snap) = e.number(&snap) {
        if ez.0.abs() < snap * ez.0.hypot(ez.1) {
            ez.0 = 0.0;
        }
    }
    // reversed axes reverse their unit vectors again later
    for (a, v) in [("x", &mut ex), ("y", &mut ey), ("z", &mut ez)] {
        if super::draw::pv(e, &format!("{a} dir")) == "reverse" {
            *v = (-v.0, -v.1);
        }
    }
    let f = |v: f64| tikra_engine::engine::pgf_number(v);
    let mut s = format!("x={{({},{})}},y={{({},{})}},z={{({},{})}},scale mode=scale uniformly,", f(ex.0), f(ex.1), f(ey.0), f(ey.1), f(ez.0), f(ez.1));
    // before compat 1.6, the only strategy
    if keys::compat(e) < 106 {
        s.push_str("scale uniformly strategy=change vertical limits,");
    }
    super::toks(&s)
}
/// Draws the image: placed by the first two mapped points, else filling the box of the plot's
/// points (`\pgfplots@plot@handler@graphics@usebb`), as `\node[anchor=south west, inner sep=0pt]
/// {\includegraphics[width=w, height=h]{file}}`.
pub fn draw(e: &mut Engine, g: &Graphics, pts: &[[f64; 3]], f: &Frame) {
    let Some(png) = g.png.clone() else { return };
    let (tw, th) = ((g.natural.0 - g.trim[0] - g.trim[2]).max(0.0), (g.natural.1 - g.trim[1] - g.trim[3]).max(0.0));
    let mapped: Vec<([f64; 3], (f64, f64))> = g.points.iter().filter_map(|(p, i)| i.map(|i| (*p, i))).take(2).collect();
    let (q, w, h) = if let [(pa, ia), (pb, ib)] = mapped[..] {
        let at = |p: [f64; 3]| f.data(p[0], p[1], if p[2].is_finite() { p[2] } else { f.min[2] });
        let (a, b) = (at(pa), at(pb));
        let (ddx, ddy) = ((ib.0 - ia.0) * PT, (ib.1 - ia.1) * PT);
        if ddx == 0.0 || ddy == 0.0 {
            e.warn("\\addplot graphics: the first two points with '=>' need different image x and y coordinates");
            return;
        }
        // canvas length per image length, and the lower left corner where image (0,0) goes
        let (rx, ry) = ((b.x - a.x) / ddx, (b.y - a.y) / ddy);
        (Point::new(a.x - ia.0 * PT * rx, a.y - ia.1 * PT * ry), rx * tw, ry * th)
    } else {
        let mut lo = Point::new(f64::INFINITY, f64::INFINITY);
        let mut hi = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for l in pts.iter().filter(|l| l.iter().take(if f.three_d { 3 } else { 2 }).all(|v| v.is_finite())) {
            let p = f.at(*l);
            lo = Point::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point::new(hi.x.max(p.x), hi.y.max(p.y));
        }
        if !(hi.x - lo.x > 0.0 && hi.y - lo.y > 0.0) {
            e.warn("\\addplot graphics: too few coordinates; the lower left and upper right corners are needed");
            return;
        }
        (lo, hi.x - lo.x, hi.y - lo.y)
    };
    if !(w.is_finite() && h.is_finite() && tw > 0.0 && th > 0.0) {
        return;
    }
    // the whole image, scaled as its trimmed part is to w × h
    let (sx, sy) = (w / tw, h / th);
    let place = Transform { a: g.natural.0 * sx, b: 0.0, c: 0.0, d: g.natural.1 * sy, e: q.x - g.trim[0] * sx, f: q.y - g.trim[1] * sy };
    let corners = [q, Point::new(q.x + w, q.y), Point::new(q.x + w, q.y + h), Point::new(q.x, q.y + h)];
    if e.counts_for_size() {
        corners.iter().for_each(|c| e.canvas.include(*c));
    }
    if g.clip {
        let mut segments = vec![Segment::MoveTo(corners[0])];
        segments.extend(corners[1..].iter().map(|c| Segment::LineTo(*c)));
        segments.push(Segment::Close);
        e.canvas.push(Item::Clip { segments, even_odd: false });
    }
    e.canvas.push(Item::Image { png, place });
    if g.clip {
        e.canvas.push(Item::End);
    }
}
