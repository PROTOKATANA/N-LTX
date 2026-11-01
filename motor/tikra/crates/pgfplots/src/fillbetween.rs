//! `\addplot fill between[of=A and B, …]` (pgflibraryfillbetween.code.tex): the area between two
//! named paths, the first one followed by the second one backwards (or forwards: `reverse`, by
//! default whether both run the same way). With `split`, the area is cut where the paths cross and
//! each piece is filled on its own, in `every segment`, the plot's options, `every segment no k`,
//! `every odd segment` or `every even segment` and `every last segment` (`\tikzfillbetween`).

use tikra_engine::scene::{Point, Segment};
use tikra_engine::tex::{self, Token};
use tikra_engine::{Error, PT};

use super::plot::Plot;
use super::{toks, Engine};

/// Pieces a curve is flattened into when the paths are cut.
const CURVE_PIECES: usize = 32;

/// A path as one polyline: inner moves become lines (`inner moveto=connect`), curves are
/// flattened.
fn polyline(segs: &[Segment]) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::new();
    let mut start = Point::default();
    for s in segs {
        match *s {
            Segment::MoveTo(p) => {
                if out.is_empty() {
                    start = p;
                }
                out.push(p);
            }
            Segment::LineTo(p) => out.push(p),
            Segment::CurveTo(c1, c2, p) => {
                let a = out.last().copied().unwrap_or(p);
                for k in 1..=CURVE_PIECES {
                    let t = k as f64 / CURVE_PIECES as f64;
                    let u = 1.0 - t;
                    out.push(a.scale(u * u * u).plus(c1.scale(3.0 * u * u * t)).plus(c2.scale(3.0 * u * t * t)).plus(p.scale(t * t * t)));
                }
            }
            Segment::Close => out.push(start),
        }
    }
    out.dedup_by(|a, b| a.minus(*b).norm() < 1e-9);
    out
}

/// Inner moves of a path joined by lines (`inner moveto=connect`), trailing moves dropped.
fn connected(segs: &[Segment]) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    // a move waits: the next one replaces it, the end of the path drops it
    let mut moved: Option<Point> = None;
    for s in segs {
        match s {
            Segment::MoveTo(p) if out.is_empty() => out.push(*s),
            Segment::MoveTo(p) => moved = Some(*p),
            _ => {
                if let Some(q) = moved.take() {
                    out.push(Segment::LineTo(q));
                }
                out.push(*s);
            }
        }
    }
    out
}

/// The way a path starts: `+` when its second point is to the right (or above, at the same x)
/// of its first (`\pgfplotsfillbetween@heuristics@get@direction`).
fn direction(segs: &[Segment]) -> bool {
    let mut pts = segs.iter().filter_map(|s| match s {
        Segment::MoveTo(p) | Segment::LineTo(p) | Segment::CurveTo(_, _, p) => Some(*p),
        Segment::Close => None,
    });
    match (pts.next(), pts.next()) {
        (Some(a), Some(b)) => a.x < b.x || (a.x == b.x && a.y < b.y),
        _ => true,
    }
}

/// Where a point is on a polyline: the piece and the position along it.
fn locate(line: &[Point], p: Point) -> (usize, f64) {
    let mut best = (0, 0.0, f64::INFINITY);
    for (i, w) in line.windows(2).enumerate() {
        let d = w[1].minus(w[0]);
        let l2 = d.x * d.x + d.y * d.y;
        let t = if l2 > 0.0 { ((p.x - w[0].x) * d.x + (p.y - w[0].y) * d.y) / l2 } else { 0.0 };
        let t = t.clamp(0.0, 1.0);
        let q = w[0].plus(d.scale(t));
        let dist = q.minus(p).norm();
        if dist < best.2 - 1e-12 {
            best = (i, t, dist);
        }
    }
    (best.0, best.1)
}

/// A polyline cut at points on it, in its own order.
fn cut(line: &[Point], points: &[Point]) -> Vec<Vec<Point>> {
    let mut at: Vec<(usize, f64, Point)> = points
        .iter()
        .map(|p| {
            let (i, t) = locate(line, *p);
            (i, t, *p)
        })
        .collect();
    at.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut out = Vec::new();
    let mut current = vec![line[0]];
    let mut next = 0;
    for i in 0..line.len().saturating_sub(1) {
        while next < at.len() && at[next].0 == i {
            current.push(at[next].2);
            out.push(std::mem::replace(&mut current, vec![at[next].2]));
            next += 1;
        }
        current.push(line[i + 1]);
    }
    out.push(current);
    out
}

fn path_of(line: &[Point]) -> Vec<Segment> {
    let mut segs = Vec::new();
    if let Some((first, rest)) = line.split_first() {
        segs.push(Segment::MoveTo(*first));
        segs.extend(rest.iter().map(|p| Segment::LineTo(*p)));
    }
    segs
}

/// The area between `a` and `b` (`\pgfpathfillbetween@compute`): `a`, then `b` (backwards with
/// `reverse`) joined to it by a line, closed.
fn area(a: &[Segment], b: &[Segment], reverse: bool) -> Vec<Segment> {
    let mut segs: Vec<Segment> = a.iter().copied().filter(|s| !matches!(s, Segment::Close)).collect();
    let b = if reverse { super::visual::reverse(b) } else { b.to_vec() };
    for (i, s) in b.into_iter().filter(|s| !matches!(s, Segment::Close)).enumerate() {
        segs.push(match (i, s) {
            (0, Segment::MoveTo(p)) => Segment::LineTo(p),
            (_, s) => s,
        });
    }
    segs.push(Segment::Close);
    segs
}

/// Whether a piece is empty: its bounding box narrower or lower than 1pt.
fn empty(line: &[Point]) -> bool {
    let (mut lo, mut hi) = (Point::new(f64::INFINITY, f64::INFINITY), Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY));
    for p in line {
        lo = Point::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    hi.x - lo.x < PT || hi.y - lo.y < PT
}

/// The options of `fill between=[…]`.
struct FillBetween {
    of: String,
    split: bool,
    reverse: Option<bool>,
    soft_domain: bool,
    soft: [Option<String>; 3],
    rest: Vec<Token>,
}

/// Reads `of`, `split`, `reverse`, the soft clips, and the options left for TikZ.
fn parse_fill_between(spec: &[Token]) -> FillBetween {
    let mut of = String::new();
    let mut split = false;
    let mut reverse: Option<bool> = None;
    let mut soft_domain = false;
    let mut soft: [Option<String>; 3] = [None, None, None];
    let mut rest: Vec<Token> = Vec::new();
    for item in tikra_engine::keys::split_list(spec) {
        let value = item.value.clone().map(|v| tex::tokens_to_string(&v)).unwrap_or_default();
        match item.key.trim() {
            "of" => of = value,
            "split" => split = value.trim() != "false",
            "reverse" => {
                reverse = match value.trim() {
                    "false" => Some(false),
                    "auto" => None,
                    _ => Some(true),
                }
            }
            k @ ("soft clip" | "soft clip first" | "soft clip second") => {
                soft_domain |= value.trim_start().starts_with("domain");
                soft[["soft clip", "soft clip first", "soft clip second"].iter().position(|x| *x == k).unwrap_or(0)] = Some(value);
            }
            _ => {
                rest.extend(toks(&item.key));
                if let Some(v) = item.value {
                    rest.extend(toks("={"));
                    rest.extend(v);
                    rest.extend(toks("}"));
                }
                rest.extend(toks(","));
            }
        }
    }
    FillBetween { of, split, reverse, soft_domain, soft, rest }
}

/// `soft clip`, `soft clip first`, `soft clip second`: each region, or a warning when it is not understood.
fn soft_regions(e: &mut Engine, soft: &[Option<String>; 3]) -> [Option<(Point, Point)>; 3] {
    let mut regions = [None, None, None];
    for (k, s) in soft.iter().enumerate() {
        if let Some(s) = s {
            regions[k] = soft_clip_region(e, s);
            if regions[k].is_none() {
                e.warn(format!("fill between: soft clip '{s}' not understood: ignored"));
            }
        }
    }
    regions
}

/// A path of no length (a point, as `(\pgfkeysvalueof{/pgfplots/xmin},0)` makes with xmin unset).
fn point_path(segs: &[Segment]) -> bool {
    let pts: Vec<Point> = segs
        .iter()
        .filter_map(|g| match g {
            Segment::MoveTo(p) | Segment::LineTo(p) | Segment::CurveTo(_, _, p) => Some(*p),
            _ => None,
        })
        .collect();
    pts.windows(2).all(|w| w[0].minus(w[1]).norm() < 1e-9)
}

/// The two paths, and whether `split` cuts them where they cross.
struct AreaPair<'a> {
    left: &'a [Segment],
    right: &'a [Segment],
    split: bool,
    reverse: bool,
}

/// One area, or one per piece when `split` cuts the paths where they cross.
fn fill_areas(e: &mut Engine, pair: &AreaPair<'_>) -> Vec<Vec<Segment>> {
    let mut pieces: Vec<Vec<Segment>> = Vec::new();
    if pair.split {
        // both run the same way, then are cut where they cross: the kth pieces make the kth segment
        let la = polyline(pair.left);
        let lb = if pair.reverse { polyline(pair.right) } else { polyline(&super::visual::reverse(pair.right)) };
        let crossings = tikra_engine::engine::intersections::intersections(&path_of(&la), &path_of(&lb));
        let (ca, cb) = (cut(&la, &crossings), cut(&lb, &crossings));
        if ca.len() != cb.len() {
            e.warn("fill between: the paths are cut into different numbers of pieces: filled as one");
            pieces.push(area(pair.left, pair.right, pair.reverse));
        } else {
            for (x, y) in ca.iter().zip(cb.iter()) {
                if empty(x) && empty(y) {
                    continue;
                }
                pieces.push(area(&path_of(x), &path_of(y), true));
            }
        }
    } else {
        pieces.push(area(pair.left, pair.right, pair.reverse));
    }
    pieces
}

/// One filled segment: its style (`every segment`, odd or even, last) and its path.
struct FillSegment<'a> {
    plot: &'a Plot,
    segs: Vec<Segment>,
    index: usize,
    last: usize,
}

fn paint_fill_segment(e: &mut Engine, seg: FillSegment<'_>) -> Result<(), Error> {
    e.begin_group();
    let r = (|| -> Result<(), Error> {
        e.st.mode = Default::default();
        e.st.mode.fill = true;
        let mut style = toks("/tikz/fill between/every segment,");
        style.extend(seg.plot.style.clone());
        style.extend(toks(&format!(
            ",/tikz/fill between/every segment no {}/.try,/tikz/fill between/every {} segment{}",
            seg.index,
            if seg.index % 2 == 1 { "odd" } else { "even" },
            if seg.index == seg.last { ",/tikz/fill between/every last segment" } else { "" }
        )));
        e.process_keys(style, Some("/pgfplots"))?;
        e.st.apply_pending();
        // with the segment's decoration, preactions and postactions
        let outer = e.st.clone();
        e.finish_path(seg.segs, &outer, "fill between")
    })();
    e.end_group();
    r
}

/// Draws the plot: its segments, each filled in its styles.
pub fn draw(e: &mut Engine, p: &Plot, spec: &[Token]) -> Result<(), Error> {
    let spec = parse_fill_between(spec);
    let Some((a, b)) = spec.of.split_once(" and ") else {
        e.warn("fill between: 'of=A and B' missing");
        return Ok(());
    };
    let (Some(pa), Some(pb)) = (e.pgf.named_paths.get(a.trim()).cloned(), e.pgf.named_paths.get(b.trim()).cloned()) else {
        e.warn(format!("fill between: no path named '{}' or '{}'", a.trim(), b.trim()));
        return Ok(());
    };
    // `every segment no 1/.style=…` and the other options
    if !spec.rest.is_empty() {
        let saved = e.st.key_path.clone();
        let r = e.process_keys(spec.rest, Some("/tikz/fill between"));
        e.st.key_path = saved;
        r?;
    }
    let (mut pa, mut pb) = (connected(&pa), connected(&pb));
    // `soft clip first`, `soft clip second`: the paths cut to a region before the area is made
    let mut regions = soft_regions(e, &spec.soft);
    if let Some(r) = regions[1] {
        pa = clip_lines(&pa, r);
    }
    if let Some(r) = regions[2] {
        pb = clip_lines(&pb, r);
    }
    // a path of no length has no intersections to split at or to clip: pgfplots fills nothing then
    if (spec.split || spec.soft_domain) && (point_path(&pa) || point_path(&pb)) {
        return Ok(());
    }
    // nor a region to clip the area to: it is filled whole
    if point_path(&pa) || point_path(&pb) {
        regions[0] = None;
    }
    let reverse = spec.reverse.unwrap_or_else(|| direction(&pa) == direction(&pb));
    let mut pieces = fill_areas(e, &AreaPair { left: &pa, right: &pb, split: spec.split, reverse });
    // `soft clip`: each area cut to the region
    if let Some(r) = regions[0] {
        pieces = pieces.iter().map(|s| clip_area(s, r)).filter(|s| !s.is_empty()).collect();
    }
    let last = pieces.len().saturating_sub(1);
    for (k, segs) in pieces.into_iter().enumerate() {
        paint_fill_segment(e, FillSegment { plot: p, segs, index: k, last })?;
    }
    Ok(())
}

/// `of`, `sequence` and `reverse` of `intersection segments`.
struct IntersectionSpec {
    of: String,
    sequence: String,
    reverse_all: bool,
}

fn parse_intersection(e: &mut Engine, v: Option<Vec<Token>>) -> IntersectionSpec {
    let mut of = String::new();
    let mut sequence = "A0 -- B1".to_string();
    let mut reverse_all = false;
    for item in tikra_engine::keys::split_list(&v.unwrap_or_default()) {
        let value = item.value.map(|v| tex::tokens_to_string(&v)).unwrap_or_default();
        match item.key.trim() {
            "of" => of = value,
            "sequence" => sequence = value,
            "reverse" => reverse_all = value.trim() != "false",
            other => e.warn(format!("intersection segments: option '{other}' not supported: ignored")),
        }
    }
    IntersectionSpec { of, sequence, reverse_all }
}

/// Two named paths cut where they cross.
struct CutPaths {
    whole: [Vec<Point>; 2],
    pieces: [Vec<Vec<Point>>; 2],
    crossed: bool,
}

/// The paths named by `of=A and B`, cut at their crossings. None when the names are missing.
fn cut_paths(e: &mut Engine, of: &str) -> Option<CutPaths> {
    let Some((a, b)) = of.split_once(" and ") else {
        e.warn("intersection segments: 'of=A and B' missing");
        return None;
    };
    let (Some(pa), Some(pb)) = (e.pgf.named_paths.get(a.trim()).cloned(), e.pgf.named_paths.get(b.trim()).cloned()) else {
        e.warn(format!("intersection segments: no path named '{}' or '{}'", a.trim(), b.trim()));
        return None;
    };
    let (la, lb) = (polyline(&pa), polyline(&pb));
    let crossings = tikra_engine::engine::intersections::intersections(&path_of(&la), &path_of(&lb));
    let crossed = !crossings.is_empty();
    let pieces = [cut(&la, &crossings), cut(&lb, &crossings)];
    Some(CutPaths { whole: [la, lb], pieces, crossed })
}

/// One piece named in the sequence: which path, its index text, and whether to walk it backwards.
struct PieceQuery<'a> {
    which: usize,
    letter: char,
    index: &'a str,
    one_based: bool,
    reverse: bool,
}

/// The points of one sequence reference (`A0`, `L1`, `*`, `[reverse]`). None when that piece does not exist.
fn piece_points(e: &mut Engine, paths: &CutPaths, query: &PieceQuery<'_>) -> Option<Vec<Point>> {
    let line = if query.index == "*" || !paths.crossed {
        if query.index != "*" {
            e.warn("intersection segments: the paths do not cross: the whole path is used");
        }
        paths.whole[query.which].clone()
    } else {
        let n = paths.pieces[query.which].len() as i64;
        let mut k: i64 = query.index.parse().unwrap_or(0);
        if query.one_based && k >= 0 {
            k -= 1;
        }
        if k < 0 {
            k += n;
        }
        match usize::try_from(k).ok().and_then(|k| paths.pieces[query.which].get(k)) {
            Some(p) => p.clone(),
            None => {
                e.warn(format!("intersection segments: there is no piece '{}{}'", query.letter, query.index));
                return None;
            }
        }
    };
    Some(if query.reverse { line.into_iter().rev().collect() } else { line })
}

/// `[reverse]` and `reverse=false` on one piece.
fn segment_reverse(e: &mut Engine, opts: &str, reverse_all: bool) -> bool {
    let mut reverse = reverse_all;
    for o in opts.split(',') {
        match o.trim() {
            "reverse" | "reverse=true" => reverse = true,
            "reverse=false" => reverse = false,
            "" => {}
            other => e.warn(format!("intersection segments: option '{other}' not supported: ignored")),
        }
    }
    reverse
}

/// The canvas coordinates of a piece, joined with `--` when the sequence said so.
fn path_tokens(line: &[Point], joined: bool) -> Vec<Token> {
    let mut out = Vec::new();
    for (k, p) in line.iter().enumerate() {
        let at = format!("(tikra cs:{}pt,{}pt)", p.x / PT, p.y / PT);
        if k == 0 && !joined {
            out.extend(toks(&format!(" {at}")));
        } else {
            out.extend(toks(&format!(" -- {at}")));
        }
    }
    out
}

/// A sequence walked against the two cut paths.
struct Sequence<'a> {
    text: &'a str,
    paths: &'a CutPaths,
    reverse_all: bool,
}

/// Walks the sequence and builds the path tokens of the pieces it names.
fn sequence_tokens(e: &mut Engine, seq: &Sequence<'_>) -> Vec<Token> {
    let (sequence, paths, reverse_all) = (seq.text, seq.paths, seq.reverse_all);
    let mut out: Vec<Token> = Vec::new();
    let mut joined = false;
    let text: Vec<char> = sequence.chars().collect();
    let mut i = 0;
    while i < text.len() {
        let c = text[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '-' && text.get(i + 1) == Some(&'-') {
            joined = true;
            i += 2;
            continue;
        }
        let which = match c {
            'A' | 'L' => 0,
            'B' | 'R' => 1,
            _ => {
                e.warn(format!("intersection segments: unexpected '{}' in the sequence: the rest ignored", text[i..].iter().collect::<String>()));
                break;
            }
        };
        let one_based = matches!(c, 'L' | 'R');
        i += 1;
        let start = i;
        while i < text.len() && (text[i].is_ascii_digit() || (text[i] == '-' && i == start) || text[i] == '*') {
            i += 1;
        }
        let index: String = text[start..i].iter().collect();
        // `[reverse]`
        while i < text.len() && text[i].is_whitespace() {
            i += 1;
        }
        let mut reverse = reverse_all;
        if text.get(i) == Some(&'[') {
            let close = text[i..].iter().position(|ch| *ch == ']').map_or(text.len(), |p| i + p);
            let opts: String = text[i + 1..close].iter().collect();
            reverse = segment_reverse(e, &opts, reverse_all);
            i = close + 1;
        }
        let Some(line) = piece_points(e, paths, &PieceQuery { which, letter: c, index: &index, one_based, reverse }) else {
            joined = false;
            continue;
        };
        out.extend(path_tokens(&line, joined));
        joined = false;
    }
    out
}

/// As `insert path` does: the path goes on with these tokens (`\tikra@insertpath`).
fn append_insert_path(e: &mut Engine, out: Vec<Token>) {
    let mut body = match e.tex.meaning_of("tikra@insertpath") {
        tikra_engine::tex::Meaning::Macro(m) => tex::substitute(&m.body, &[]),
        _ => Vec::new(),
    };
    body.extend(out);
    e.tex.set_meaning(&Token::cs("tikra@insertpath"), tikra_engine::engine::macro_tokens(body), false);
}

/// `intersection segments={of=A and B, sequence={L1 -- R2[reverse]}}`: pieces of two named paths,
/// cut where they cross, put into the path being built (`\tikzpathintersectionsegments`). `A0`,
/// `B1` count from 0, `L1`, `R1` from 1, negative numbers from the end, `*` is the whole path;
/// `--` joins a piece to the one before, `[reverse]` walks it backwards.
pub fn intersection_segments(e: &mut Engine, v: Option<Vec<Token>>) -> Result<(), Error> {
    let spec = parse_intersection(e, v);
    let Some(paths) = cut_paths(e, &spec.of) else { return Ok(()) };
    let out = sequence_tokens(e, &Sequence { text: &spec.sequence, paths: &paths, reverse_all: spec.reverse_all });
    append_insert_path(e, out);
    Ok(())
}

/// A soft path in a macro (`\pgfretval`): the macro holds a key into the engine's saved paths.
pub fn softpath(e: &mut Engine, segs: Vec<Segment>) -> tex::Meaning {
    let key = format!("@softpath{}", e.pgf.saved_paths.len());
    e.pgf.saved_paths.insert(key.clone(), segs);
    tikra_engine::engine::macro_from(&key)
}

/// The first control sequence of an argument (`\pgfsetpath\A`, `{\A}`).
fn macro_arg(e: &mut Engine) -> Result<Token, Error> {
    let arg = e.argument()?;
    Ok(arg.into_iter().find(|t| matches!(t, Token::Cs(_))).unwrap_or_else(|| Token::cs("relax")))
}

/// The low-level commands of the fillbetween library on soft paths: `\tikzgetnamedpath{name}`,
/// `\pgfcomputeintersectionsegments{1}` (the paths of the last `\pgfintersectionofpaths` cut where
/// they cross), `\pgfgetintersectionsegmentpath{1}{0}`, `\pgfsetpathandBB\A`,
/// `\pgfpathreplacefirstmoveto\A` and `\pgfaddpathandBB\A`; and pgfplots'
/// `\pgfplotspathaxisoutline`.
pub fn soft_command(e: &mut Engine, name: &str) -> Result<(), Error> {
    let retval = Token::cs("pgfretval");
    match name {
        "tikzgetnamedpath" => {
            let arg = e.argument()?;
            let n = e.tex.expand_to_string(arg)?;
            let Some(segs) = e.pgf.named_paths.get(n.trim()).cloned() else {
                e.warn(format!("\\tikzgetnamedpath: no path named '{}'", n.trim()));
                return Ok(());
            };
            let m = softpath(e, segs);
            e.tex.set_meaning(&retval, m, false);
        }
        "pgfcomputeintersectionsegments" => {
            let arg = e.argument()?;
            let k = e.tex.expand_to_string(arg)?;
            let k = k.trim().to_string();
            let path = e.pgf.saved_paths.get(&format!("@intersection path {k}")).cloned().unwrap_or_default();
            let line = polyline(&path);
            let pieces = if line.is_empty() { Vec::new() } else { cut(&line, &e.pgf.pgf_solutions.clone()) };
            for (j, piece) in pieces.iter().enumerate() {
                e.pgf.saved_paths.insert(format!("@intersection segment {k} {j}"), path_of(piece));
            }
        }
        "pgfgetintersectionsegmentpath" => {
            let (a, b) = (e.argument()?, e.argument()?);
            let (k, j) = (e.tex.expand_to_string(a)?, e.tex.expand_to_string(b)?);
            let segs = e.pgf.saved_paths.get(&format!("@intersection segment {} {}", k.trim(), j.trim())).cloned().unwrap_or_default();
            let m = softpath(e, segs);
            e.tex.set_meaning(&retval, m, false);
        }
        "pgfsetpathandBB" | "pgfaddpathandBB" => {
            let t = macro_arg(e)?;
            let segs = tikra_engine::engine::basic::softpath_of(e, &t).unwrap_or_default();
            if name == "pgfsetpathandBB" {
                e.pgf.pgf_path = tikra_engine::engine::basic::PgfPath::default();
            }
            for s in segs {
                e.pgf_add(s);
            }
        }
        // the outline of the axis being drawn, added to the path being built
        "pgfplotspathaxisoutline" => {
            let Some(f) = super::fig(e).frame.clone() else { return Ok(()) };
            let (lo, hi) = (f.min, f.max);
            let corners = [[lo[0], lo[1]], [hi[0], lo[1]], [hi[0], hi[1]], [lo[0], hi[1]]].map(|[x, y]| f.at([x, y, lo[2]]));
            e.pgf_add(Segment::MoveTo(corners[0]));
            for c in &corners[1..] {
                e.pgf_add(Segment::LineTo(*c));
            }
            e.pgf_add(Segment::Close);
        }
        "pgfpathreplacefirstmoveto" => {
            let t = macro_arg(e)?;
            let mut segs = tikra_engine::engine::basic::softpath_of(e, &t).unwrap_or_default();
            if let Some(Segment::MoveTo(p)) = segs.first().copied() {
                segs[0] = Segment::LineTo(p);
            }
            let m = softpath(e, segs);
            e.tex.set_meaning(&t, m, false);
        }
        _ => {}
    }
    Ok(())
}

/// The region of `soft clip={(3,-1) rectangle (4,100)}` or `soft clip={domain=3:4}` (the x range of
/// an axis, or `domain y=` its y range), on the canvas: its lower left and upper right corners.
pub fn soft_clip_region(e: &mut Engine, spec: &str) -> Option<(Point, Point)> {
    let spec = tikra_engine::keys::strip_braces(spec.trim()).trim().to_string();
    let far = 1e9;
    if let Some((key, range)) = spec.split_once('=') {
        let (a, b) = range.split_once(':')?;
        let (a, b) = (e.number(a.trim()).ok()?, e.number(b.trim()).ok()?);
        let f = super::fig(e).frame.clone()?;
        let (p, q) = match key.trim() {
            "domain" => (f.at([f.logical(0, a), f.min[1], f.min[2]]), f.at([f.logical(0, b), f.min[1], f.min[2]])),
            "domain y" => (f.at([f.min[0], f.logical(1, a), f.min[2]]), f.at([f.min[0], f.logical(1, b), f.min[2]])),
            _ => return None,
        };
        let (lo, hi) = (p.x.min(q.x), p.x.max(q.x));
        let (bot, top) = (p.y.min(q.y), p.y.max(q.y));
        return Some(if key.trim() == "domain" { (Point::new(lo, -far), Point::new(hi, far)) } else { (Point::new(-far, bot), Point::new(far, top)) });
    }
    // `(a) rectangle (b)`
    let (a, b) = spec.split_once("rectangle")?;
    let inner = |s: &str| s.trim().strip_prefix('(').and_then(|r| r.strip_suffix(')')).map(str::to_string);
    let (a, b) = (inner(a)?, inner(b)?);
    let a = e.tex.expand_to_string(tikra_engine::engine::tikra_tokens(&a)).ok()?;
    let b = e.tex.expand_to_string(tikra_engine::engine::tikra_tokens(&b)).ok()?;
    let (p, q) = (e.coordinate(&a).ok()?.p, e.coordinate(&b).ok()?.p);
    Some((Point::new(p.x.min(q.x), p.y.min(q.y)), Point::new(p.x.max(q.x), p.y.max(q.y))))
}

/// A closed area clipped to a rectangle (Sutherland–Hodgman): the part of it inside, closed.
pub fn clip_area(segs: &[Segment], (lo, hi): (Point, Point)) -> Vec<Segment> {
    let mut poly = polyline(segs);
    if poly.len() > 1 && poly.first().map(|a| a.minus(*poly.last().unwrap()).norm() < 1e-9) == Some(true) {
        poly.pop();
    }
    // each edge of the rectangle: inside when `side(p) >= 0`
    let edges: [(usize, f64, f64); 4] = [(0, lo.x, 1.0), (0, hi.x, -1.0), (1, lo.y, 1.0), (1, hi.y, -1.0)];
    for (axis, at, sign) in edges {
        let side = |p: &Point| sign * (if axis == 0 { p.x } else { p.y } - at);
        let mut out = Vec::new();
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            let (sa, sb) = (side(&a), side(&b));
            if sa >= 0.0 {
                out.push(a);
            }
            if (sa >= 0.0) != (sb >= 0.0) {
                let t = sa / (sa - sb);
                out.push(a.plus(b.minus(a).scale(t)));
            }
        }
        poly = out;
        if poly.is_empty() {
            return Vec::new();
        }
    }
    let mut out = path_of(&poly);
    out.push(Segment::Close);
    out
}

/// A path clipped to a rectangle: its pieces inside, each a subpath.
pub fn clip_lines(segs: &[Segment], (lo, hi): (Point, Point)) -> Vec<Segment> {
    let line = polyline(segs);
    let inside = |p: Point| p.x >= lo.x - 1e-9 && p.x <= hi.x + 1e-9 && p.y >= lo.y - 1e-9 && p.y <= hi.y + 1e-9;
    let mut out = Vec::new();
    let mut open = false;
    for w in line.windows(2) {
        let (a, b) = (w[0], w[1]);
        // Liang–Barsky: the part of a→b inside
        let d = b.minus(a);
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        let mut keep = true;
        for (p, q) in [(-d.x, a.x - lo.x), (d.x, hi.x - a.x), (-d.y, a.y - lo.y), (d.y, hi.y - a.y)] {
            if p == 0.0 {
                if q < 0.0 {
                    keep = false;
                }
            } else {
                let r = q / p;
                if p < 0.0 {
                    t0 = t0.max(r);
                } else {
                    t1 = t1.min(r);
                }
            }
        }
        if !keep || t0 > t1 {
            open = false;
            continue;
        }
        let (p, q) = (a.plus(d.scale(t0)), a.plus(d.scale(t1)));
        if !open || t0 > 0.0 {
            out.push(Segment::MoveTo(p));
        }
        out.push(Segment::LineTo(q));
        open = t1 >= 1.0 && inside(b);
    }
    out
}
