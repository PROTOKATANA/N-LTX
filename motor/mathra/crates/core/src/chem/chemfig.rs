//! chemfig's molecule language (spec/input.md): atoms, bonds with their angles, branches, rings
//! (fused or not, aromatic or not), distant bonds (`?`), Lewis marks and hooks for mechanisms.

use std::collections::BTreeMap;

use super::model::*;
use super::scheme::{split_top, Reader, R};
use crate::rules;

/// Characters that end an atom label.
const STOPS: &[char] = &['-', '=', '~', '<', '>', '(', ')', '*', '?', '[', ']', '@'];

/// The state of the ring being read: how its bonds turn and where it closes.
#[derive(Clone, Copy)]
struct RingState {
    id: usize,
    first: f32,
    /// Degrees each bond turns from the previous one: positive counterclockwise.
    turn: f32,
    /// Bonds the ring content may have, and the atom the last one closes on.
    bonds: usize,
    close: AtomId,
}

struct Mol<'r, 'w> {
    c: Vec<char>,
    pos: usize,
    m: Molecule,
    reader: &'r mut Reader<'w>,
    /// Exterior bisector of each ring vertex: the default direction of a branch there.
    bisector: BTreeMap<AtomId, f32>,
    /// `?[name]` marks waiting for their partner: the atom and the bond kind given.
    closures: BTreeMap<String, (AtomId, Option<BondKind>)>,
    hook: Option<String>,
}

/// Reads a `\chemfig{…}` body.
pub fn molecule(src: &str, reader: &mut Reader) -> R<Molecule> {
    let mut p = Mol { c: src.chars().collect(), pos: 0, m: Molecule::default(), reader, bisector: BTreeMap::new(), closures: BTreeMap::new(), hook: None };
    let start = p.atom(None, None)?;
    p.chain(start, 0.0, None)?;
    p.skip_spaces();
    while p.pos < p.c.len() {
        let ch = p.c[p.pos];
        p.reader.tolerate(rules::E_REC_9, format!("'{ch}' with nothing to close or attach to"), "ignored")?;
        p.pos += 1;
        p.chain(start, 0.0, None)?;
    }
    for name in p.closures.keys() {
        p.reader.tolerate(rules::E_REC_9, format!("distant bond ?[{name}] opened and never closed"), "left out")?;
    }
    Ok(p.m)
}

fn is_bond_start(ch: char) -> bool {
    matches!(ch, '-' | '=' | '~' | '<' | '>')
}

impl Mol<'_, '_> {
    fn current(&self) -> Option<char> {
        self.c.get(self.pos).copied()
    }

    fn skip_spaces(&mut self) {
        while self.current().is_some_and(char::is_whitespace) {
            self.pos += 1;
        }
    }

    /// Content between `open` and its matching `close`; the current character is `open`.
    fn group(&mut self, open: char, close: char) -> R<String> {
        let mut level = 0;
        let mut s = String::new();
        while let Some(ch) = self.current() {
            self.pos += 1;
            if ch == open {
                level += 1;
                if level == 1 {
                    continue;
                }
            } else if ch == close {
                level -= 1;
                if level == 0 {
                    return Ok(s);
                }
            }
            s.push(ch);
        }
        self.reader.tolerate(rules::E_REC_4, format!("'{open}' without a matching '{close}'"), "closed at the end")?;
        Ok(s)
    }

    /// Reads a branch, the rest of a chain: bonds, atoms, branches, rings and `?` marks, until `)` or the end.
    /// `angle` is the default direction of the first bond (each bond's is then the previous one's).
    fn chain(&mut self, start: AtomId, angle: f32, ring: Option<RingState>) -> R<()> {
        let mut cur = start;
        let mut angle = angle;
        // in a ring, the bond count so far and the direction of the last ring bond
        let mut k = 0usize;
        let mut last_edge: Option<f32> = None;
        // the atom a nested ring already put where this ring's next vertex goes
        let mut preset: Option<AtomId> = None;
        loop {
            self.skip_spaces();
            let Some(ch) = self.current() else { break };
            match ch {
                ')' => break,
                '(' => {
                    let default = self.bisector.get(&cur).copied().unwrap_or(angle);
                    self.pos += 1;
                    self.chain(cur, default, None)?;
                    self.skip_spaces();
                    if self.current() == Some(')') {
                        self.pos += 1;
                    } else {
                        self.reader.tolerate(rules::E_REC_4, "branch '(' without its ')'", "closed at the end")?;
                    }
                }
                '*' => {
                    // a ring opened at a vertex a ring bond reached is fused to the parent's next side
                    let fused = match (ring, last_edge) {
                        (Some(r), Some(edge)) => Some((r.turn, edge)),
                        _ => None,
                    };
                    let incoming = if ring.is_some() { angle } else { self.bisector.get(&cur).copied().unwrap_or(angle) };
                    if let Some(shared) = self.ring(cur, incoming, fused)? {
                        if fused.is_some() {
                            preset = Some(shared);
                        }
                    }
                }
                '?' => {
                    self.pos += 1;
                    self.closure(cur)?;
                }
                '@' => {
                    self.pos += 1;
                    if self.current() == Some('{') {
                        let name = self.group('{', '}')?;
                        self.m.atoms[cur].hook.get_or_insert_with(|| name.split(',').next().unwrap_or("").trim().to_string());
                    }
                }
                c if is_bond_start(c) => {
                    let (kind, args) = self.bond_spec()?;
                    let mut bond_angle = resolve_angle(args.first().map(String::as_str).unwrap_or(""), angle);
                    let length = args.get(1).and_then(|l| l.trim().parse::<f32>().ok()).filter(|l| *l > 0.0).unwrap_or(1.0);
                    let explicit = |i: usize| args.get(i).and_then(|v| v.trim().parse::<usize>().ok()).filter(|v| *v > 0).map(|v| v - 1);
                    let kind = tikz_kind(args.get(4).map(String::as_str).unwrap_or(""), kind);
                    let mut target = None;
                    let mut ring_id = None;
                    if let Some(r) = ring {
                        if k >= r.bonds {
                            self.reader.tolerate(rules::E_REC_9, format!("ring of {} sides with more bonds than it can take", self.m.rings[r.id].size), "extra bond left out")?;
                            // read and drop what follows the extra bond
                            let _ = self.atom(None, None)?;
                            continue;
                        }
                        bond_angle = r.first + r.turn * k as f32;
                        ring_id = Some(r.id);
                        if k + 1 == r.bonds {
                            target = Some(r.close);
                        }
                    }
                    let departure = explicit(2).unwrap_or_else(|| self.default_departure(cur, bond_angle));
                    let to = match target {
                        Some(t) => {
                            // the label after a closing bond names the start atom again; it is not a new atom
                            let _ = self.label()?;
                            t
                        }
                        None => match preset.take() {
                            Some(p) => {
                                let (label, lewis) = self.label()?;
                                let atom = &mut self.m.atoms[p];
                                if atom.label.is_empty() && !label.is_empty() {
                                    atom.pieces = pieces(&label);
                                    atom.label = label;
                                }
                                atom.lewis.extend(lewis);
                                p
                            }
                            None => self.atom(Some(bond_angle), explicit(3))?,
                        },
                    };
                    let arrival = if target.is_some() { explicit(3).unwrap_or(self.m.atoms[to].anchor) } else { self.m.atoms[to].anchor };
                    self.add_bond(Bond { from: cur, to, kind, angle: normalize(bond_angle), length, departure, arrival, ring: ring_id });
                    if let Some(r) = ring {
                        if target.is_none() && !self.m.rings[r.id].atoms.contains(&to) {
                            self.m.rings[r.id].atoms.push(to);
                            // exterior bisector: halfway between this bond and the next, outward
                            let outward = bond_angle + r.turn / 2.0 - 90.0 * r.turn.signum();
                            self.bisector.entry(to).or_insert(normalize(outward));
                        }
                        k += 1;
                        last_edge = Some(bond_angle);
                    }
                    cur = to;
                    angle = bond_angle;
                }
                other => {
                    self.reader.tolerate(rules::E_REC_9, format!("unexpected '{other}' in a structure"), "ignored")?;
                    self.pos += 1;
                }
            }
        }
        Ok(())
    }

    /// `*n(…)` or `**n(…)` at `at`. A ring hanging from a bond has its center in that bond's direction.
    /// A ring opened at a vertex of another ring (`fused`: the parent's turn and the side that reached
    /// `at`) turns the same way and lies outside it, sharing the parent's next side: its last bond comes
    /// back to `at` along that side. Returns the atom at the far end of the shared side.
    fn ring(&mut self, at: AtomId, incoming: f32, fused: Option<(f32, f32)>) -> R<Option<AtomId>> {
        self.pos += 1;
        let aromatic = self.current() == Some('*');
        if aromatic {
            self.pos += 1;
        }
        let mut digits = String::new();
        while let Some(d) = self.current().filter(char::is_ascii_digit) {
            digits.push(d);
            self.pos += 1;
        }
        let size = digits.parse::<usize>().unwrap_or(0);
        if !(3..=40).contains(&size) {
            self.reader.tolerate(rules::E_REC_9, format!("ring of {size} sides (from 3 to 40)"), "read as a hexagon")?;
        }
        let size = if (3..=40).contains(&size) { size } else { 6 };
        self.skip_spaces();
        if self.current() != Some('(') {
            self.reader.tolerate(rules::E_REC_4, "ring without its '(…)'", "left empty")?;
            return Ok(None);
        }
        self.pos += 1;
        let step = 360.0 / size as f32;
        let id = self.m.rings.len();
        let (first, turn) = match fused {
            Some((parent_turn, edge)) => {
                let turn = parent_turn.signum() * step;
                (edge + parent_turn + 180.0 + turn, turn)
            }
            None => (incoming - 90.0 + 180.0 / size as f32, step),
        };
        self.m.rings.push(Ring { size, atoms: vec![at], aromatic });
        let start_outward = first - turn / 2.0 - 90.0 * turn.signum();
        self.bisector.entry(at).or_insert(normalize(start_outward));
        let state = RingState { id, first, turn, bonds: size, close: at };
        self.chain(at, first, Some(state))?;
        self.skip_spaces();
        if self.current() == Some(')') {
            self.pos += 1;
        } else {
            self.reader.tolerate(rules::E_REC_4, "ring '(' without its ')'", "closed at the end")?;
        }
        // the vertex the ring's second-to-last side reaches: the far end of the side it shares
        let shared = self.m.rings[id].atoms.get(size - 1).copied();
        Ok(shared)
    }

    /// Adds a bond, or strengthens the one already between the two atoms: a side two fused rings both
    /// draw is one bond, with the higher order.
    fn add_bond(&mut self, b: Bond) {
        if let Some(old) = self.m.bonds.iter_mut().find(|o| (o.from == b.from && o.to == b.to) || (o.from == b.to && o.to == b.from)) {
            if b.kind.order() > old.kind.order() {
                old.kind = b.kind;
                old.ring = b.ring;
                if old.from != b.from {
                    std::mem::swap(&mut old.from, &mut old.to);
                    std::mem::swap(&mut old.departure, &mut old.arrival);
                    old.angle = b.angle;
                }
            }
            return;
        }
        self.m.bonds.push(b);
    }

    /// `?`, `?[name]` or `?[name,{=}]`: the first mark of a name waits, the second draws the bond.
    fn closure(&mut self, at: AtomId) -> R<()> {
        let mut name = "a".to_string();
        let mut kind = None;
        if self.current() == Some('[') {
            let inside = self.group('[', ']')?;
            let parts = split_top(&inside, ',');
            if let Some(n) = parts.first().map(|n| n.trim()).filter(|n| !n.is_empty()) {
                name = n.to_string();
            }
            if let Some(b) = parts.get(1) {
                kind = Some(match b.trim().trim_matches(['{', '}']) {
                    "=" => BondKind::Double,
                    "~" => BondKind::Triple,
                    _ => BondKind::Single,
                });
            }
        }
        match self.closures.remove(&name) {
            None => {
                self.closures.insert(name, (at, kind));
            }
            Some((other, first_kind)) => {
                let kind = kind.or(first_kind).unwrap_or(BondKind::Single);
                let (a, b) = (self.m.atoms[other].anchor, self.m.atoms[at].anchor);
                self.add_bond(Bond { from: other, to: at, kind, angle: 0.0, length: 1.0, departure: a, arrival: b, ring: None });
            }
        }
        Ok(())
    }

    /// A bond character and its optional `[angle,length,departure,arrival,tikz]`.
    fn bond_spec(&mut self) -> R<(BondKind, Vec<String>)> {
        let ch = self.current().unwrap_or('-');
        self.pos += 1;
        let kind = match ch {
            '=' => BondKind::Double,
            '~' => BondKind::Triple,
            '>' | '<' => {
                let back = ch == '<';
                let variant = self.current();
                let pick = |plain, back_kind| if back { back_kind } else { plain };
                match variant {
                    Some(':') => {
                        self.pos += 1;
                        pick(BondKind::Hash, BondKind::HashBack)
                    }
                    Some('|') => {
                        self.pos += 1;
                        pick(BondKind::Hollow, BondKind::HollowBack)
                    }
                    _ => pick(BondKind::Wedge, BondKind::WedgeBack),
                }
            }
            _ => BondKind::Single,
        };
        self.skip_spaces();
        let args = if self.current() == Some('[') { split_top(&self.group('[', ']')?, ',') } else { Vec::new() };
        Ok((kind, args))
    }

    /// Reads an atom label (possibly empty) and creates the atom. `arriving`: the direction of the bond
    /// that reaches it, which picks the piece it attaches to.
    fn atom(&mut self, arriving: Option<f32>, explicit: Option<usize>) -> R<AtomId> {
        let (label, lewis) = self.label()?;
        let pieces = pieces(&label);
        let anchor = match explicit {
            Some(k) => k.min(pieces.len().saturating_sub(1)),
            None => default_arrival(&pieces, arriving),
        };
        let hook = self.hook.take();
        self.m.atoms.push(Atom { label, pieces, anchor, lewis, hook });
        Ok(self.m.atoms.len() - 1)
    }

    /// The raw label up to the next bond or structure character, without spaces; `\lewis{…,X}` gives X and its marks.
    fn label(&mut self) -> R<(String, Vec<Lewis>)> {
        let mut s = String::new();
        let mut lewis = Vec::new();
        while let Some(ch) = self.current() {
            match ch {
                c if c.is_whitespace() => self.pos += 1,
                '@' => {
                    self.pos += 1;
                    if self.current() == Some('{') {
                        let name = self.group('{', '}')?;
                        self.hook = Some(name.split(',').next().unwrap_or("").trim().to_string());
                    }
                }
                c if STOPS.contains(&c) => break,
                '{' => {
                    let inner = self.group('{', '}')?;
                    s.push('{');
                    s.push_str(&inner);
                    s.push('}');
                }
                '^' | '_' => {
                    // a script takes the next character even if it is a bond sign: ^- is a charge
                    s.push(ch);
                    self.pos += 1;
                    match self.current() {
                        Some('{') => {
                            let inner = self.group('{', '}')?;
                            s.push('{');
                            s.push_str(&inner);
                            s.push('}');
                        }
                        Some(n) => {
                            s.push(n);
                            self.pos += 1;
                        }
                        None => {}
                    }
                }
                '\\' => {
                    self.pos += 1;
                    let mut name = String::new();
                    while let Some(l) = self.current().filter(char::is_ascii_alphabetic) {
                        name.push(l);
                        self.pos += 1;
                    }
                    if name.is_empty() {
                        if let Some(sym) = self.current() {
                            s.push('\\');
                            s.push(sym);
                            self.pos += 1;
                        }
                        continue;
                    }
                    match name.as_str() {
                        "lewis" | "Lewis" => {
                            self.skip_spaces();
                            let arg = if self.current() == Some('{') { self.group('{', '}')? } else { String::new() };
                            let (spec, inner) = arg.split_once(',').unwrap_or((arg.as_str(), ""));
                            lewis.extend(lewis_marks(spec));
                            s.push_str(inner.trim());
                        }
                        "chemabove" | "chembelow" => {
                            let mut args = Vec::new();
                            for _ in 0..2 {
                                self.skip_spaces();
                                args.push(if self.current() == Some('{') { self.group('{', '}')? } else { String::new() });
                            }
                            let stack = if name == "chemabove" { "overset" } else { "underset" };
                            s.push_str(&format!("\\{stack}{{{}}}{{{}}}", args[1], args[0]));
                        }
                        _ => {
                            s.push('\\');
                            s.push_str(&name);
                            // braced arguments go with the command
                            while self.current() == Some('{') {
                                let inner = self.group('{', '}')?;
                                s.push('{');
                                s.push_str(&inner);
                                s.push('}');
                            }
                        }
                    }
                }
                _ => {
                    s.push(ch);
                    self.pos += 1;
                }
            }
        }
        Ok((s, lewis))
    }

    /// The piece a bond leaves from by default: the last one going right, the first going left, and for
    /// a vertical bond the heavy atom (`H_3C` leaves from C).
    fn default_departure(&self, at: AtomId, angle: f32) -> usize {
        let pieces = &self.m.atoms[at].pieces;
        if pieces.len() <= 1 {
            return 0;
        }
        let x = angle.to_radians().cos();
        if x > 0.1 {
            pieces.len() - 1
        } else if x < -0.1 {
            0
        } else {
            heavy(pieces)
        }
    }
}

/// The piece a bond arrives at by default: the first coming from the left, the last coming from the
/// right, the heavy atom coming vertically or for the first atom.
fn default_arrival(pieces: &[String], arriving: Option<f32>) -> usize {
    if pieces.len() <= 1 {
        return 0;
    }
    match arriving.map(|a| a.to_radians().cos()) {
        Some(x) if x > 0.1 => 0,
        Some(x) if x < -0.1 => pieces.len() - 1,
        _ => heavy(pieces),
    }
}

/// The first piece that is not hydrogen (or the first piece).
fn heavy(pieces: &[String]) -> usize {
    pieces.iter().position(|p| !p.trim_start_matches('{').starts_with('H')).unwrap_or(0)
}

/// Splits a label into atoms as chemfig does: a capital letter starts a new one, lowercase letters,
/// digits, scripts, braces and symbol commands stay with the current one.
pub fn pieces(label: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let c: Vec<char> = label.chars().collect();
    let mut i = 0;
    let mut level = 0;
    while i < c.len() {
        let ch = c[i];
        let starts = level == 0 && (ch.is_ascii_uppercase() || (ch == '\\' && starts_piece(&c[i + 1..])));
        // a script's argument never starts a piece: H_2, ^{+}
        let after_script = i > 0 && matches!(c[i - 1], '_' | '^');
        if (starts && !after_script) || out.is_empty() {
            out.push(String::new());
        }
        match ch {
            '{' => level += 1,
            '}' => level -= 1,
            '\\' => {
                // the whole command name goes at once
                let last = out.last_mut().expect("a piece was just opened");
                last.push('\\');
                i += 1;
                while i < c.len() && c[i].is_ascii_alphabetic() {
                    last.push(c[i]);
                    i += 1;
                }
                continue;
            }
            _ => {}
        }
        out.last_mut().expect("a piece was just opened").push(ch);
        i += 1;
    }
    out.retain(|p| !p.is_empty());
    out
}

/// Commands that hold a whole atom (`\mathrm{Br}`, `\overset{+}{N}`) start a piece; symbols (`\oplus`) do not.
fn starts_piece(rest: &[char]) -> bool {
    let name: String = rest.iter().take_while(|c| c.is_ascii_alphabetic()).collect();
    matches!(name.as_str(), "mathrm" | "text" | "textrm" | "mathbf" | "textbf" | "overset" | "underset" | "mathit")
}

/// `\lewis{0:2.4|6o,…}`: a direction digit (0 to 7, in steps of 45°) and its mark.
fn lewis_marks(spec: &str) -> Vec<Lewis> {
    let mut out = Vec::new();
    let c: Vec<char> = spec.chars().collect();
    let mut i = 0;
    while i < c.len() {
        if let Some(d) = c[i].to_digit(10).filter(|d| *d < 8) {
            let mark = match c.get(i + 1) {
                Some('.') => LewisMark::Radical,
                Some('|') => LewisMark::Line,
                Some('o') => LewisMark::Vacancy,
                _ => LewisMark::Pair,
            };
            if matches!(c.get(i + 1), Some(':' | '.' | '|' | 'o')) {
                i += 1;
            }
            out.push(Lewis { angle: d as f32 * 45.0, mark });
        }
        i += 1;
    }
    out
}

/// chemfig's angle argument: empty keeps the previous direction, `n` is n × 45°, `:x` is absolute,
/// `::x` is relative to the previous bond.
fn resolve_angle(arg: &str, previous: f32) -> f32 {
    let a = arg.trim();
    if a.is_empty() {
        return previous;
    }
    if let Some(r) = a.strip_prefix("::") {
        return previous + r.trim().parse::<f32>().unwrap_or(0.0);
    }
    if let Some(x) = a.strip_prefix(':') {
        return x.trim().parse::<f32>().unwrap_or(previous);
    }
    a.parse::<f32>().map(|n| n * 45.0).unwrap_or(previous)
}

/// The fifth argument, TikZ options: `draw=none` hides the bond, a dash pattern dashes it.
fn tikz_kind(tikz: &str, kind: BondKind) -> BondKind {
    if tikz.contains("draw=none") {
        BondKind::Invisible
    } else if tikz.contains("dash") || tikz.contains("dotted") {
        BondKind::Dashed
    } else {
        kind
    }
}

/// An angle in (-180, 180].
pub fn normalize(a: f32) -> f32 {
    let mut a = a % 360.0;
    if a > 180.0 {
        a -= 360.0;
    } else if a <= -180.0 {
        a += 360.0;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_split_into_atoms() {
        assert_eq!(pieces("H_3C"), ["H_3", "C"]);
        assert_eq!(pieces("CH_2OH"), ["C", "H_2", "O", "H"]);
        assert_eq!(pieces("NH_3^+"), ["N", "H_3^+"]);
        assert_eq!(pieces("O\\ominus"), ["O\\ominus"]);
        assert_eq!(pieces("{CH_3}"), ["{CH_3}"]);
        assert_eq!(pieces("\\mathrm{Br}"), ["\\mathrm{Br}"]);
        assert_eq!(pieces("SO_4^{2-}"), ["S", "O_4^{2-}"]);
        assert!(pieces("").is_empty());
    }

    #[test]
    fn angles() {
        assert_eq!(resolve_angle("", 30.0), 30.0);
        assert_eq!(resolve_angle("2", 0.0), 90.0);
        assert_eq!(resolve_angle(":-30", 0.0), -30.0);
        assert_eq!(resolve_angle("::60", 30.0), 90.0);
        assert_eq!(normalize(270.0), -90.0);
        assert_eq!(normalize(-180.0), 180.0);
    }

    #[test]
    fn lewis_specs() {
        let marks = lewis_marks("0:2.4|6o");
        let kinds: Vec<_> = marks.iter().map(|l| (l.angle, l.mark)).collect();
        assert_eq!(kinds, [(0.0, LewisMark::Pair), (90.0, LewisMark::Radical), (180.0, LewisMark::Line), (270.0, LewisMark::Vacancy)]);
    }
}
