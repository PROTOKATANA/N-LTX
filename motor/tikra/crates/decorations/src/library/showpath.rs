//! `show path construction`: one segment of the path under decoration.

use super::super::walker::Kind;
use super::*;

impl Run<'_> {
    pub(super) fn segment_code_current(&mut self) -> Result<(), Error> {
        let k = match self.w.seg.kind {
            Kind::MoveTo => 0,
            Kind::LineTo => 1,
            Kind::CurveTo => 2,
            Kind::ClosePath => 3,
            Kind::Last => return Ok(()),
        };
        self.segment_code(k)
    }

    /// Runs `moveto code`, `lineto code`… with `\tikzinputsegmentfirst`… set, in canvas coordinates.
    pub(super) fn segment_code(&mut self, k: usize) -> Result<(), Error> {
        let Some(code) = self.s.segment_code[k].clone() else {
            return Ok(());
        };
        let s = self.w.seg;
        let e = &mut *self.e;
        e.begin_group();
        for (name, q) in [("tikzinputsegmentfirst", s.first), ("tikzinputsegmentlast", s.last), ("tikzinputsegmentsupporta", s.sa), ("tikzinputsegmentsupportb", s.sb)] {
            e.tex.define_string(name, &format!("{}pt,{}pt", q.x, q.y), false);
        }
        e.st.transform = Transform::default();
        e.st.pending = Transform::default();
        let r = e.run_tokens(code);
        e.end_group();
        r
    }
}
