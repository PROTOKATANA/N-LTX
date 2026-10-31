//! The circle connection bar's geometry, in pt.
//!
//! The decoration automaton draws it (`decoration=circle connection bar`) and the mindmap piece
//! paints the bar between two concepts. It stays in the engine: it builds path segments with
//! `path::arc_segments`, and tikra_tex knows nothing of drawing.

use crate::scene::{Point, Segment, Transform};

/// A cap of a circle connection bar, in pt, around the circle's center: from the circle's border
/// at ±`angle` it widens and ends in a band of width `amplitude` at 1.5 radius, facing x.
pub fn cap(r: f64, amplitude: f64, angle: f64, t: &Transform) -> Vec<Segment> {
    let polar = |a: f64, d: f64| {
        let (s, c) = a.to_radians().sin_cos();
        Point::new(d * c, d * s)
    };
    let a = amplitude;
    let start = polar(angle, r);
    let mut v = vec![Segment::MoveTo(t.apply(start))];
    v.extend(crate::canvas::transform_segments(&super::path::arc_segments(Point::new(0.0, 0.0), r, r, angle, -angle, &Transform::default()), t));
    let low = polar(-angle, r).plus(polar(90.0 - angle, 0.25 * r));
    v.push(Segment::CurveTo(t.apply(low), t.apply(Point::new(1.25 * r, -0.5 * a)), t.apply(Point::new(1.5 * r, -0.5 * a))));
    v.push(Segment::LineTo(t.apply(Point::new(1.5 * r, 0.5 * a))));
    let high = start.plus(polar(angle - 90.0, 0.25 * r));
    v.push(Segment::CurveTo(t.apply(Point::new(1.25 * r, 0.5 * a)), t.apply(high), t.apply(start)));
    v.push(Segment::Close);
    v
}

/// The whole bar, in pt, along x from the start circle's border (at 0) to the end circle's (at
/// `length`): the start cap, the band between, the end cap, as the decoration's three states draw
/// them.
pub fn bar(r1: f64, r2: f64, amplitude: f64, angle: f64, length: f64, t: &Transform) -> Vec<Segment> {
    let mut v = cap(r1, amplitude, angle, &t.compose(&Transform::translation(-r1, 0.0)));
    let (x0, x1, h) = (0.5 * r1, length - 0.5 * r2, 0.5 * amplitude);
    for (i, (x, y)) in [(x0, -h), (x1, -h), (x1, h), (x0, h)].into_iter().enumerate() {
        let p = t.apply(Point::new(x, y));
        v.push(if i == 0 { Segment::MoveTo(p) } else { Segment::LineTo(p) });
    }
    v.push(Segment::Close);
    let end = t.compose(&Transform::translation(length, 0.0)).compose(&Transform::scaling(-1.0, 1.0)).compose(&Transform::translation(-r2, 0.0));
    v.extend(cap(r2, amplitude, angle, &end));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bar_has_three_pieces() {
        let v = bar(28.0, 14.0, 4.9, 20.0, 30.0, &Transform::default());
        assert_eq!(v.iter().filter(|s| matches!(s, Segment::Close)).count(), 3);
        // the band ends where the caps begin
        assert!(v.iter().any(|s| matches!(s, Segment::LineTo(p) if (p.x - 23.0).abs() < 1e-9)));
    }
}
