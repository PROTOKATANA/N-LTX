//! `markings`: positions along the path and the code of each mark.

use super::super::Pos;
use super::*;

impl Run<'_> {
    pub(super) fn set_mark_info(&mut self) -> Result<(), Error> {
        let seq = self.seq;
        // a dimension, as `\the` writes it (`120.0pt`)
        let at = if seq == 0 { String::new() } else { tikra_engine::tex::dimen::show((self.mark_at * 65536.0).round() as i64) };
        let e = &mut *self.e;
        e.tex.define_string(&tikra_engine::keys::macro_name("/pgf/decoration/mark info/sequence number"), &seq.to_string(), false);
        e.tex.define_string(&tikra_engine::keys::macro_name("/pgf/decoration/mark info/distance from start"), &at, false);
        Ok(())
    }

    /// `\pgf@lib@dec@parsenum`: a position in pt (a fraction of the path without a unit; from the
    /// end if negative).
    fn position(&mut self, pos: &Pos) -> Result<f64, Error> {
        let l = self.w.section;
        match pos {
            Pos::Pt(v) => Ok(if *v < 0.0 { l + v } else { *v }),
            Pos::Tokens(t) => {
                super::super::set_registers(self.e, &self.w);
                let s = self.e.math_string(t.clone())?;
                let (v, unit) = tikra_engine::expr::eval_length(&s, &self.e.vars())?;
                Ok(if unit {
                    let v = v / PT;
                    if v < 0.0 {
                        l + v
                    } else {
                        v
                    }
                } else if v < 0.0 {
                    l + v * l
                } else {
                    v * l
                })
            }
        }
    }

    /// The `pre` state: the next mark of the list, or the end.
    pub(super) fn next_mark(&mut self) -> Result<(), Error> {
        self.mark_at = 0.0;
        self.mark_next = "final";
        while let Some(m) = self.marks.pop_front() {
            match m {
                Mark::At { pos, action } => {
                    self.mark_at = self.position(&pos)?;
                    self.mark_action = action;
                    self.mark_next = "main";
                    return Ok(());
                }
                Mark::Between { start, end, step, action } => {
                    let a = self.position(&start)?;
                    let b = self.position(&end)?;
                    if b < a - EPS {
                        continue;
                    }
                    let st = self.position(&step)?;
                    if st <= 0.0 {
                        self.e.warn("mark between positions: the step must be positive: mark ignored");
                        continue;
                    }
                    self.marks.push_front(Mark::Between { start: Pos::Pt(a + st), end: Pos::Pt(b), step: Pos::Pt(st), action: action.clone() });
                    self.mark_at = a;
                    self.mark_action = action;
                    self.mark_next = "main";
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    /// The `main` state: the mark's code, in a scope at the mark.
    pub(super) fn mark(&mut self, t: &Transform) -> Result<(), Error> {
        let action = self.mark_action.clone();
        super::super::set_registers(self.e, &self.w);
        let e = &mut *self.e;
        e.begin_group();
        e.begin_scope();
        e.st.transform = to_cm(t);
        e.st.pending = Transform::default();
        e.st.decoration.marks = Default::default();
        e.st.decorate = false;
        e.tex.set_meaning(&Token::cs("arrow"), e.tex.meaning_of("tikra@arrow"), false);
        e.tex.set_meaning(&Token::cs("arrowreversed"), e.tex.meaning_of("tikra@arrowreversed"), false);
        let r = e.run_tokens(action);
        e.end_scope();
        e.end_group();
        r?;
        let node = self.s.connection_node.clone();
        if !node.is_empty() {
            match self.e.nodes.get(&node).cloned() {
                Some(n) => {
                    let tc = to_cm(t);
                    let a = n.border_toward(tc.apply(p(-PT, 0.0)));
                    let b = n.border_toward(tc.apply(p(PT, 0.0)));
                    self.w.out.line_to(a.scale(1.0 / PT));
                    self.w.out.move_to(b.scale(1.0 / PT));
                }
                None => self.e.warn(format!("mark connection node: no node named '{node}'")),
            }
        }
        Ok(())
    }
}
