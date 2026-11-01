//! The logic gates of `pgflibraryshapes.gates.logic.US.code.tex` (and, nand, or, nor, xor, xnor, not
//! and buffer, drawn the American way, and the and and nand gates of the CDH style) and of
//! `pgflibraryshapes.gates.logic.IEC.code.tex` (the same gates as rectangles with a symbol), with the
//! keys they share from `pgflibraryshapes.gates.logic.code.tex`.
//!
//! A gate's size grows with its text and with its number of inputs (`/pgf/logic gate inputs`, `nin`
//! or `normal,inverted,normal`): the inputs are spread evenly along its back, `input 1` at the top,
//! and an inverted one gets a small circle of `/pgf/logic gate inverted radius`. The US gates are
//! square around their text, by their `\halfside`; their arcs and anchors are pgf's constants times
//! it. pgf computes their border piece by piece (lines and arcs, depending on the direction); here
//! the same pieces are sampled into an outline around the center.

mod and_us;
mod cdh;
mod common;
mod iec;
mod not_gate;
mod or_gate;

pub use and_us::and_gate_us;
pub use cdh::and_gate_cdh;
pub use iec::gate_iec;
pub use not_gate::not_gate_us;
pub use or_gate::or_gate_us;

/// `\pgf@lib@sh@logicgate@parseinputs`: whether each of the first `max` inputs is inverted. Without a
/// comma every letter is an input (`i` inverted); with commas every item is (`inverted`).
pub fn parse_inputs(spec: &str, max: usize) -> Vec<bool> {
    let v: Vec<bool> = if spec.contains(',') {
        spec.split(',').map(|s| s.trim_start() == "inverted").collect()
    } else {
        spec.chars().filter(|c| !c.is_whitespace() && *c != '{' && *c != '}').map(|c| c == 'i').collect()
    };
    v.into_iter().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::super::EllipseSpan;
    use super::common::{ray_arc, EllipticArc};
    use super::parse_inputs;
    use crate::scene::Point;

    #[test]
    fn inputs_short_and_long() {
        assert_eq!(parse_inputs("nin", 1024), vec![false, true, false]);
        assert_eq!(parse_inputs("inverted, normal, inverted", 1024), vec![true, false, true]);
        assert_eq!(parse_inputs("normal,normal", 1024), vec![false, false]);
        assert_eq!(parse_inputs("iii", 2), vec![true, true]);
    }

    #[test]
    fn a_ray_meets_an_arc() {
        let p = ray_arc(Point::new(0.0, 0.0), Point::new(1.0, 0.0), EllipticArc { center: Point::new(0.0, 0.0), span: EllipseSpan { a0: -90.0, a1: 90.0, rx: 2.0, ry: 1.0 } });
        assert!((p.x - 2.0).abs() < 1e-12 && p.y.abs() < 1e-12);
    }
}
