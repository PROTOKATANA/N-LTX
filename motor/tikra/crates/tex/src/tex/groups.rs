//! Meanings, groups and registers.

use super::*;
use std::collections::HashMap;
use std::rc::Rc;

impl Tex {
    // ---------------------------------------------------------------- meanings and groups

    pub fn meaning(&self, t: &Token) -> Meaning {
        match t {
            Token::Cs(n) => self.meanings.get(n).cloned().unwrap_or(Meaning::Undefined),
            Token::Char(c, Cat::Active) => self.active.get(c).cloned().unwrap_or(Meaning::Undefined),
            Token::Char(c, cat) => Meaning::Char(*c, *cat),
        }
    }

    pub fn meaning_of(&self, name: &str) -> Meaning {
        self.meanings.get(name).cloned().unwrap_or(Meaning::Undefined)
    }

    pub fn is_defined(&self, name: &str) -> bool {
        !matches!(self.meanings.get(name), None | Some(Meaning::Undefined))
    }

    /// Assigns a meaning, locally unless `\global` was given (or `global` is true).
    pub fn set_meaning(&mut self, t: &Token, m: Meaning, global: bool) {
        let global = global || std::mem::take(&mut self.global);
        match t {
            Token::Cs(n) => {
                if global {
                    self.forget_saved(|s| matches!(s, Saved::Meaning(k, _) if k == n));
                } else if let Some(level) = self.save.last_mut() {
                    level.push(Saved::Meaning(n.clone(), self.meanings.get(n).cloned()));
                }
                self.meanings.insert(n.clone(), m);
            }
            Token::Char(c, _) => {
                if global {
                    self.forget_saved(|s| matches!(s, Saved::Active(k, _) if k == c));
                } else if let Some(level) = self.save.last_mut() {
                    level.push(Saved::Active(*c, self.active.get(c).cloned()));
                }
                self.active.insert(*c, m);
            }
        }
        self.run_after_assignment();
    }

    pub fn define(&mut self, name: &str, m: Meaning) {
        self.set_meaning(&Token::cs(name), m, false);
    }

    /// A macro without parameters, from tokens.
    pub fn define_macro(&mut self, name: &str, body: Vec<Token>, global: bool) {
        let m = Macro { params: Vec::new(), body: body.into_iter().map(Body::Tok).collect(), optional: None, protected: false };
        self.set_meaning(&Token::cs(name), Meaning::Macro(Rc::new(m)), global);
    }

    /// A macro without parameters whose body is this text (read as letters and others).
    pub fn define_string(&mut self, name: &str, value: &str, global: bool) {
        self.define_macro(name, string_tokens(value), global);
    }

    fn forget_saved(&mut self, matches: impl Fn(&Saved) -> bool) {
        for level in &mut self.save {
            level.retain(|s| !matches(s));
        }
    }

    pub fn begin_group(&mut self) {
        self.save.push(Vec::new());
        self.after_group.push(Vec::new());
    }

    /// Ends a group: restores what it changed locally and runs its `\aftergroup` tokens.
    pub fn end_group(&mut self) -> bool {
        let Some(level) = self.save.pop() else { return false };
        for s in level.into_iter().rev() {
            match s {
                Saved::Meaning(n, old) => restore(&mut self.meanings, n, old),
                Saved::Active(c, old) => restore(&mut self.active, c, old),
                Saved::Count(n, old) => restore(&mut self.counts, n, old),
                Saved::Dimen(n, old) => restore(&mut self.dimens, n, old),
                Saved::Toks(n, old) => restore(&mut self.toks, n, old),
                Saved::Catcode(c, old) => restore(&mut self.catcodes, c, old),
            }
        }
        if let Some(after) = self.after_group.pop() {
            self.push_tokens(after);
        }
        true
    }

    pub fn group_depth(&self) -> usize {
        self.save.len()
    }

    pub fn set_count(&mut self, n: u32, v: i64, global: bool) {
        let global = global || std::mem::take(&mut self.global);
        if global {
            self.forget_saved(|s| matches!(s, Saved::Count(k, _) if *k == n));
        } else if let Some(level) = self.save.last_mut() {
            level.push(Saved::Count(n, self.counts.get(&n).copied()));
        }
        self.counts.insert(n, v.clamp(-2_147_483_647, 2_147_483_647));
        self.run_after_assignment();
    }

    pub fn set_dimen(&mut self, n: u32, v: i64, global: bool) {
        let global = global || std::mem::take(&mut self.global);
        if global {
            self.forget_saved(|s| matches!(s, Saved::Dimen(k, _) if *k == n));
        } else if let Some(level) = self.save.last_mut() {
            level.push(Saved::Dimen(n, self.dimens.get(&n).copied()));
        }
        self.dimens.insert(n, v.clamp(-dimen::MAX_DIMEN, dimen::MAX_DIMEN));
        self.run_after_assignment();
    }

    pub fn set_toks(&mut self, n: u32, v: Vec<Token>, global: bool) {
        let global = global || std::mem::take(&mut self.global);
        if global {
            self.forget_saved(|s| matches!(s, Saved::Toks(k, _) if *k == n));
        } else if let Some(level) = self.save.last_mut() {
            level.push(Saved::Toks(n, self.toks.get(&n).cloned()));
        }
        self.toks.insert(n, Rc::new(v));
        self.run_after_assignment();
    }

    fn run_after_assignment(&mut self) {
        if let Some(t) = self.after_assignment.take() {
            self.back(t);
        }
    }

    pub fn new_register(&mut self) -> u32 {
        self.next_register += 1;
        self.next_register
    }

    pub fn warn(&mut self, s: impl Into<String>) {
        let s = s.into();
        if !self.warnings.contains(&s) && self.warnings.len() < 1000 {
            self.warnings.push(s);
        }
    }
}

fn restore<K: std::hash::Hash + Eq, V, S: std::hash::BuildHasher>(map: &mut HashMap<K, V, S>, k: K, old: Option<V>) {
    match old {
        Some(v) => {
            map.insert(k, v);
        }
        None => {
            map.remove(&k);
        }
    }
}
