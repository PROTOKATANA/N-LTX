//! Applying a decoration to a path: `\pgfdecoration` and the automaton.

use tikra_engine::canvas::{transform_segments, Item};
use tikra_engine::engine::{tikra_tokens, Engine};
use tikra_engine::keys;
use tikra_engine::scene::{Point, Segment, Transform};
use tikra_engine::tex::{Cat, Token};
use tikra_engine::{Error, PT};

use super::library::Run;
use super::walker::{self, Walker};
use super::{library, sp, split_commas, Env, How};

/// Open `\pgfdecoration`s. The stack outlives the group the decoration opens.
#[derive(Clone, Default)]
struct OpenDecorations(Vec<Env>);

fn open_decorations(e: &mut Engine) -> &mut Vec<Env> {
    &mut e.piece_state::<OpenDecorations>().0
}

/// Decorates segments (cm) with the current decoration (`\tikz@lib@dec@decorate@path` and
/// `\pgfdecoration`). `existing` is the point before them, for a path that does not start
/// with a moveto.
pub(super) fn decorate_segments(e: &mut Engine, segs: &[Segment], existing: Option<Point>) -> Result<Vec<Segment>, Error> {
    let Some(name) = e.st.decoration.name.clone() else {
        return Ok(segs.to_vec());
    };
    if let Some(decoration) = e.piece_decoration(name.trim()) {
        return decoration(e, segs);
    }
    if library::resolve_meta(e, &name).is_none() && library::resolve(e, &name).is_none() {
        e.warn(format!("decoration '{name}' not supported: path drawn undecorated"));
        return Ok(segs.to_vec());
    }
    if e.st.pen.rounded != 0.0 || e.st.pen.rounded_out != 0.0 {
        // the round corners of the path are made before it is decorated (`\pgfprocessround`)
        let rounded = tikra_engine::engine::output::round_corners(segs, e.st.pen.rounded, e.st.pen.rounded_out);
        return decorate_how(e, &rounded, existing, How::Tikz(name));
    }
    decorate_how(e, segs, existing, How::Tikz(name))
}

/// Decorates segments (cm): `\pgfdecoration` with a list, `\pgfmetadecoration`, or TikZ's way.
fn decorate_how(e: &mut Engine, segs: &[Segment], existing: Option<Point>, how: How) -> Result<Vec<Segment>, Error> {
    let existing = existing.unwrap_or_default().scale(1.0 / PT);
    let mut pts = transform_segments(segs, &Transform::scaling(1.0 / PT, 1.0 / PT));
    // `path from text`: a lone moveto becomes a line as long as the text
    let mut prepared = None;
    if let (How::Tikz(name), [Segment::MoveTo(q)]) = (&how, pts.as_slice()) {
        if name.trim() == "text effects along path" && e.st.decoration.te.path_from_text {
            let q = *q;
            let probe = Walker::new(walker::parse(&[Segment::MoveTo(q), Segment::LineTo(Point::new(q.x + 1.0, q.y))], q), q);
            let mut run = Run::new(e, probe, e.st.decoration.clone());
            run.te_prepare()?;
            let width: f64 = run.te.chars.iter().map(|c| c.pre + c.post).sum();
            let te = std::mem::take(&mut run.te);
            let a = e.st.decoration.te.path_from_text_angle.to_radians();
            pts.push(Segment::LineTo(Point::new(q.x + width * a.cos(), q.y + width * a.sin())));
            prepared = Some(te);
        }
    }
    // a lone moveto becomes a tiny line
    if let [Segment::MoveTo(q)] = pts.as_slice() {
        let q = *q;
        pts.push(Segment::LineTo(Point::new(q.x + 0.0001, q.y)));
    }
    if !pts.iter().any(|s| !matches!(s, Segment::MoveTo(_))) {
        return Ok(Vec::new());
    }
    let mut parsed = walker::parse(&pts, existing);
    e.process_keys(tikra_tokens("/pgf/every decoration/.try"), Some("/pgf/decoration"))?;
    let s = e.st.decoration.clone();
    if s.reverse {
        parsed.objs = walker::reverse(&parsed.objs);
    }
    let w = Walker::new(parsed, existing);
    let mut run = Run::new(e, w, s);
    if let Some(te) = prepared {
        run.te = te;
    }
    match how {
        How::Tikz(name) => {
            let pre = run.key_text("pre length");
            let post = run.key_text("post length");
            if let Some(m) = library::resolve_meta(run.e, &name) {
                run.run_meta(m, None)?;
            } else {
                let dec = library::resolve(run.e, &name).unwrap_or(library::Dec::LineTo);
                if pre != "0pt" || post != "0pt" {
                    run.run_meta(library::Meta::Internal, Some(dec))?;
                } else {
                    let total = run.w.total;
                    run.invoke(dec, total)?;
                }
            }
        }
        How::Meta(name) => match library::resolve_meta(run.e, &name) {
            Some(m) => run.run_meta(m, None)?,
            None => run.e.warn(format!("meta-decoration '{}' not supported: path left out", name.trim())),
        },
        How::List(list) => {
            for item in split_commas(&list) {
                let a = tikra_engine::engine::options::split_arguments(&item, 4);
                let name = run.e.tex.expand_to_string(a[0].clone())?;
                let (total, rem) = (run.w.total, run.w.rem_total);
                run.e.tex.dimens.insert(4010, sp(total));
                run.e.tex.dimens.insert(4012, sp(rem));
                let dist = run.e.length_of(a[1].clone())? / PT;
                let code = |v: &Vec<Token>| {
                    if keys::trim(v).is_empty() {
                        None
                    } else {
                        Some(v.clone())
                    }
                };
                match library::resolve(run.e, &name) {
                    Some(dec) => run.invoke_with(dec, dist, code(&a[2]), code(&a[3]))?,
                    None => run.e.warn(format!("decoration '{}' not supported: left out", name.trim())),
                }
            }
        }
    }
    let out = std::mem::take(&mut run.w.out.segs);
    Ok(transform_segments(&out, &Transform::scaling(PT, PT)))
}

/// `\pgfdecoration{list}` and `\pgfmetadecoration{name}` (`meta`): the path built until the end
/// is decorated, then added to the path there was.
pub(super) fn decoration_begin(e: &mut Engine) -> Result<(), Error> {
    let list = e.argument()?;
    let meta = e.argument_string()? == "1";
    let existing = std::mem::take(&mut e.pgf.pgf_path);
    e.pgf.pgf_path = tikra_engine::engine::basic::PgfPath { segments: Vec::new(), last: existing.last, start: existing.start, quick: false };
    open_decorations(e).push(Env { list, meta, existing });
    e.begin_group();
    Ok(())
}

pub(super) fn decoration_end(e: &mut Engine) -> Result<(), Error> {
    e.end_group();
    let Some(env) = open_decorations(e).pop() else {
        e.warn("\\endpgfdecoration without \\pgfdecoration: ignored");
        return Ok(());
    };
    let decorated = std::mem::take(&mut e.pgf.pgf_path.segments);
    let last = env.existing.last;
    e.pgf.pgf_path = env.existing;
    if decorated.is_empty() {
        e.warn("I cannot decorate an empty path");
        return Ok(());
    }
    let how = if env.meta { How::Meta(e.tex.expand_to_string(env.list)?) } else { How::List(env.list) };
    e.begin_group();
    let r = decorate_how(e, &decorated, last, how);
    e.end_group();
    add_to_pgf_path(e, r?);
    Ok(())
}

/// `\pgfdecoratecurrentpath{name}`: the basic layer's path, decorated.
pub(super) fn decorate_current_path(e: &mut Engine) -> Result<(), Error> {
    let name = e.argument()?;
    let path = std::mem::take(&mut e.pgf.pgf_path);
    let mut list = vec![Token::Char('{', Cat::Begin)];
    list.extend(name);
    list.push(Token::Char('}', Cat::End));
    list.extend(tikra_tokens("{\\pgfdecoratedpathlength}"));
    e.begin_group();
    let r = decorate_how(e, &path.segments, None, How::List(list));
    e.end_group();
    add_to_pgf_path(e, r?);
    Ok(())
}

fn add_to_pgf_path(e: &mut Engine, segs: Vec<Segment>) {
    let path = &mut e.pgf.pgf_path;
    for s in segs {
        match s {
            Segment::MoveTo(q) => {
                path.start = Some(q);
                path.last = Some(q);
            }
            Segment::LineTo(q) | Segment::CurveTo(_, _, q) => path.last = Some(q),
            Segment::Close => path.last = path.start,
        }
        path.segments.push(s);
    }
}

/// `\arrow[options]{tip}` and `\arrowreversed` in a mark: the tip at the origin, pointing right.
pub(super) fn arrow_command(e: &mut Engine, reversed: bool) -> Result<(), Error> {
    let options = e.optional()?;
    let spec = e.argument_string()?;
    e.begin_group();
    let r = (|| {
        if let Some(o) = options {
            e.process_keys(o, Some("/tikz"))?;
            e.st.apply_pending();
        }
        let stroke = e.stroke();
        let d = Point::new(if reversed { -1.0 } else { 1.0 }, 0.0);
        let mut tips = Vec::new();
        tikra_engine::engine::arrows::tip_items(&tikra_engine::engine::arrows::TipMark { spec: &spec, at: Point::default(), direction: d, stroke: &stroke }, &mut tips);
        let t = e.st.transform;
        for item in tips {
            if let Item::Path { segments, fill, stroke } = item {
                let segments = transform_segments(&segments, &t);
                if e.counts_for_size() {
                    e.canvas.include_path(&segments);
                }
                e.canvas.push(Item::Path { segments, fill, stroke });
            }
        }
        Ok(())
    })();
    e.end_group();
    r
}
