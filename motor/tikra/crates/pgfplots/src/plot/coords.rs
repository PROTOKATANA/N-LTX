use tikra_engine::tex::Token;
use tikra_engine::Error;

use super::super::keys;
use super::super::Engine;
use super::sample::{axis_value, split_commas, value};
use super::{add_label, Coord};

/// One `(…)` group, from the `(` at `i`. `i` ends just after the closing `)`.
pub(super) fn paren_group(list: &[Token], i: &mut usize) -> Option<Vec<Token>> {
    let mut depth = 0i32;
    let start = *i + 1;
    while *i < list.len() {
        match &list[*i] {
            Token::Char('(', _) => depth += 1,
            Token::Char(')', _) => {
                depth -= 1;
                if depth == 0 {
                    let v = list[start..*i].to_vec();
                    *i += 1;
                    return Some(v);
                }
            }
            _ => {}
        }
        *i += 1;
    }
    None
}

/// `(x,y)` or `(x,y,z)` of a coordinate list.
pub(super) fn coord_parts(e: &mut Engine, parts: &[Vec<Token>]) -> Coord {
    let x = parts.first().map_or(f64::NAN, |p| axis_value(e, 'x', p));
    let y = parts.get(1).map_or(f64::NAN, |p| axis_value(e, 'y', p));
    let z = parts.get(2).map_or(0.0, |p| value(e, p));
    Coord::new(x, y, z)
}

/// Where a coordinate's error bars and `[meta]` are read from.
pub(super) struct CoordTail<'a> {
    list: &'a [Token],
    at: usize,
    /// `point meta=explicit symbolic`: the bracket is text, never a number.
    symbolic: bool,
}

/// `+- (…)`, or `+= (…) -= (…)`: the point's error bars.
pub(super) fn read_error_bars(e: &mut Engine, tail: &mut CoordTail<'_>, c: &mut Coord) {
    let list = tail.list;
    let mut j = tail.at;
    loop {
        while j < list.len() && list[j].is_space() {
            j += 1;
        }
        if !(j + 1 < list.len() && (list[j].is_char('+') || list[j].is_char('-')) && (list[j + 1].is_char('-') || list[j + 1].is_char('='))) {
            break;
        }
        let before = j;
        {
            let kind = (list[j].is_char('+'), list[j + 1].is_char('-'));
            let mut k = j + 2;
            while k < list.len() && list[k].is_space() {
                k += 1;
            }
            if k < list.len() && list[k].is_char('(') {
                let mut kk = k;
                if let Some(inside) = paren_group(list, &mut kk) {
                    let p = split_commas(&inside);
                    let ex = p.first().map_or(f64::NAN, |v| value(e, v));
                    let ey = p.get(1).map_or(f64::NAN, |v| value(e, v));
                    match kind {
                        (true, true) => c.err = [ex, ex, ey, ey],
                        (true, false) => {
                            c.err[1] = ex;
                            c.err[3] = ey;
                        }
                        _ => {
                            c.err[0] = ex;
                            c.err[2] = ey;
                        }
                    }
                    j = kk;
                }
            }
        }
        if j == before {
            break;
        }
    }
    tail.at = j;
}

/// `[meta]` after a coordinate.
pub(super) fn read_bracket_meta(e: &mut Engine, tail: &mut CoordTail<'_>, c: &mut Coord) {
    let list = tail.list;
    let mut j = tail.at;
    while j < list.len() && list[j].is_space() {
        j += 1;
    }
    if j < list.len() && list[j].is_char('[') {
        let mut k = j + 1;
        let start = k;
        while k < list.len() && !list[k].is_char(']') {
            k += 1;
        }
        let written = &list[start..k.min(list.len())];
        // `explicit symbolic`: the text as written, never a number
        if !tail.symbolic {
            c.meta = value(e, written);
        }
        c.label = add_label(e, tikra_engine::keys::trim(written).to_vec());
        j = k + 1;
    }
    tail.at = j;
}

/// `coordinates {(x,y) (x,y) +- (ex,ey) [meta] …}`.
/// The points of `coordinates {…}`, and where its scanlines (ended by empty lines) end.
pub(super) fn coordinates(e: &mut Engine, list: Vec<Token>) -> Result<(Vec<Coord>, Vec<usize>), Error> {
    let explicit_symbolic = keys::value(e, "/pgfplots/point meta").trim() == "explicit symbolic";
    let list = e.tex.expand_fully(list)?;
    let mut out = Vec::new();
    let mut scanlines: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < list.len() {
        let t = &list[i];
        if t.is_cs("par") {
            if !out.is_empty() && scanlines.last() != Some(&out.len()) {
                scanlines.push(out.len());
            }
            i += 1;
            continue;
        }
        if t.is_char('(') {
            let Some(inside) = paren_group(&list, &mut i) else { break };
            let parts = split_commas(&inside);
            if parts.iter().all(|p| p.is_empty()) {
                // () is a jump
                out.push(Coord::new(f64::NAN, f64::NAN, f64::NAN));
                continue;
            }
            let mut c = coord_parts(e, &parts);
            // error bars (`+- (…)`, or `+= (…) -= (…)`), then [meta]
            let mut tail = CoordTail { list: &list, at: i, symbolic: explicit_symbolic };
            read_error_bars(e, &mut tail, &mut c);
            read_bracket_meta(e, &mut tail, &mut c);
            i = tail.at;
            out.push(c);
            continue;
        }
        i += 1;
    }
    Ok((out, scanlines))
}
