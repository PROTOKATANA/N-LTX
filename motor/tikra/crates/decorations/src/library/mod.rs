//! The decorations of pgf's libraries, state by state as they are declared there
//! (`pgflibrarydecorations.*.code.tex` and the module's `lineto`, `moveto`, `curveto`), and the
//! automaton that runs them (`\pgf@decorate@run`, `\pgf@metadecoration@run`).
//!
//! A state has options (its width, switches, next state, repeat, auto end and corner, persistent
//! computations) and code that draws in the coordinate system of the current position: x along
//! the path, y to its left, in pt.

use std::collections::VecDeque;

use tikra_engine::scene::{Point, Transform};
use tikra_engine::tex::Token;
use tikra_engine::{Error, PT};

use super::walker::{Walker, EPS, SP};
use super::{Mark, Settings};
use tikra_engine::engine::Engine;

mod catalog;
mod draw;
mod markings;
mod meta;
mod shapebg;
mod showpath;
mod textpath;

use catalog::initial;
use catalog::macro_body;
pub use catalog::{intern, resolve, resolve_meta, Dec, Meta};

/// The options of a state (`/pgf/decoration automaton/…`), in their order.
#[derive(Debug, Clone, Copy)]
enum Opt {
    Width(f64),
    Switch(f64, &'static str),
    SwitchInput(f64, &'static str),
    Next(&'static str),
    AutoEnd(f64),
    AutoCorner(f64),
    /// `persistent precomputation`, `persistent postcomputation`
    Pre,
    Post,
    Repeat(i64),
}

/// TeX code to run, if any.
type Code = Option<Vec<Token>>;

/// A path command of a foot: moveto, lineto, curveto, closepath (pt).
#[derive(Debug, Clone, Copy)]
pub enum FootOp {
    M(f64, f64),
    L(f64, f64),
    C(f64, f64, f64, f64, f64, f64),
    Z,
}

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// A cubic's two controls and its end, in the decoration's local frame (pt).
struct LocalCurve {
    c1: Point,
    c2: Point,
    end: Point,
}

fn polar(angle: f64, r: f64) -> Point {
    let (s, c) = angle.to_radians().sin_cos();
    p(r * c, r * s)
}

fn shifted(t: &Transform, x: f64, y: f64) -> Transform {
    t.compose(&Transform::translation(x, y))
}

/// Canvas pt → cm for a transformation from local pt.
pub fn to_cm(t: &Transform) -> Transform {
    Transform { e: t.e * PT, f: t.f * PT, ..*t }
}

/// What text along path keeps between states.
#[derive(Debug, Clone, Default)]
struct TextState {
    rest: VecDeque<Token>,
    format: String,
    math: bool,
    indent_left: f64,
    char_shift: f64,
    space_shift: f64,
    fit: bool,
    stretch: bool,
    is_space: bool,
    /// the character's box: its text, format, width (pt); None at the end of the text
    current: Option<(String, String, f64)>,
    shift: f64,
}

/// One run of decorations along one path.
pub struct Run<'a> {
    pub e: &'a mut Engine,
    pub w: Walker,
    pub s: Settings,
    /// `\pgf@decorate@additionaltransform` (raise, mirror, transform), pt
    pub extra: Transform,
    has_corners: bool,
    state: &'static str,
    next_state: &'static str,
    repeat: i64,
    width: f64,
    jump: bool,
    // triangles and crosses
    ssw: f64,
    ssh: f64,
    // markings
    marks: VecDeque<Mark>,
    mark_at: f64,
    mark_action: Vec<Token>,
    mark_next: &'static str,
    seq: i64,
    // shape backgrounds
    shape: ShapeRun,
    text: TextState,
    pub(super) te: super::textfx::State,
    // declared decorations: the persistent computations of the state
    user_pre: Vec<Token>,
    user_post: Vec<Token>,
}

#[derive(Debug, Clone, Default)]
struct ShapeRun {
    initialise: f64,
    before: f64,
    after: f64,
    start_width: f64,
    width: f64,
    width_change: f64,
    start_height: f64,
    height: f64,
    height_change: f64,
    sep: f64,
    between_borders: bool,
    spread: bool,
    special_width: f64,
}

impl<'a> Run<'a> {
    pub fn new(e: &'a mut Engine, w: Walker, s: Settings) -> Run<'a> {
        let extra = s.extra_transform();
        let has_corners = s.has_corners;
        // \pgfdecorationsegmentlength and \pgfdecorationsegmentamplitude, which code may set
        e.tex.dimens.insert(4016, (s.segment_length * 65536.0).round() as i64);
        e.tex.dimens.insert(4017, (s.amplitude * 65536.0).round() as i64);
        Run {
            e,
            w,
            s,
            extra,
            has_corners,
            state: "",
            next_state: "",
            repeat: -1,
            width: 0.0,
            jump: false,
            ssw: 0.0,
            ssh: 0.0,
            marks: VecDeque::new(),
            mark_at: 0.0,
            mark_action: Vec::new(),
            mark_next: "final",
            seq: 0,
            shape: ShapeRun::default(),
            text: TextState::default(),
            user_pre: Vec::new(),
            user_post: Vec::new(),
            te: Default::default(),
        }
    }

    fn amp(&self) -> f64 {
        self.s.amplitude
    }
    fn len(&self) -> f64 {
        self.s.segment_length
    }

    /// A length stored in a key (`/pgf/decoration/pre length`…), in pt.
    pub fn key_pt(&mut self, key: &str, default: f64) -> f64 {
        match tikra_engine::keys::value(&self.e.tex, &format!("/pgf/decoration/{key}")) {
            Some(v) if !tikra_engine::keys::trim(&v).is_empty() => match self.e.length_of(v) {
                Ok(l) => l / PT,
                Err(err) => {
                    self.e.warn(format!("decoration: {key}: {err}"));
                    default
                }
            },
            _ => default,
        }
    }

    pub fn key_text(&mut self, key: &str) -> String {
        let v = tikra_engine::keys::value(&self.e.tex, &format!("/pgf/decoration/{key}")).unwrap_or_default();
        self.e.tex.expand_to_string(v).unwrap_or_default().trim().to_string()
    }

    fn key_number(&mut self, key: &str, default: f64) -> f64 {
        let t = self.key_text(key);
        if t.is_empty() {
            return default;
        }
        self.e.number(&t).unwrap_or(default)
    }

    /// pgf's `rand`, from the one generator pgfmath has (`\pgfmathsetseed` sets it).
    fn rand(&mut self) -> f64 {
        tikra_engine::expr::pseudo_random_signed()
    }

    /// The transformation of the code: the input segment's, then the additional one.
    fn t(&self) -> Transform {
        self.w.transform().compose(&self.extra)
    }
    fn mv(&mut self, t: &Transform, x: f64, y: f64) {
        self.w.out.move_to(t.apply(p(x, y)));
    }
    fn ln(&mut self, t: &Transform, x: f64, y: f64) {
        self.w.out.line_to(t.apply(p(x, y)));
    }
    fn cv(&mut self, t: &Transform, curve: LocalCurve) {
        self.w.out.curve_to(t.apply(curve.c1), t.apply(curve.c2), t.apply(curve.end));
    }
    /// `\pgfpathsine` and `\pgfpathcosine`: from the current point, by `(x, y)`.
    fn wave(&mut self, t: &Transform, x: f64, y: f64, sine: bool) {
        let last = self.w.out.last;
        let q = |a: f64, b: f64| last.plus(t.linear(p(a, b)));
        let (c1, c2) = if sine { (q(0.3260 * x, 0.5120 * y), q(0.6380 * x, y)) } else { (q(0.3620 * x, 0.0), q(0.6740 * x, 0.4880 * y)) };
        self.w.out.curve_to(c1, c2, q(x, y));
    }
    /// `\pgfpatharc{a0}{a1}{r}` from the current point.
    fn arc(&mut self, t: &Transform, a0: f64, a1: f64, r: f64) {
        let c = self.w.out.last.minus(t.linear(polar(a0, r)));
        for s in tikra_engine::engine::path::arc_segments(tikra_engine::engine::path::EllipseArc { center: c, rx: r, ry: r, start: a0, end: a1 }, t) {
            if let tikra_engine::scene::Segment::CurveTo(a, b, q) = s {
                self.w.out.curve_to(a, b, q);
            }
        }
    }

    /// `\pgf@decorate@invoke`: decorates the next `distance` of the path with `dec`.
    pub fn invoke(&mut self, dec: Dec, distance: f64) -> Result<(), Error> {
        self.invoke_with(dec, distance, None, None)
    }

    /// `\pgf@decorate@invoke` with the code to run before and after (`\pgfdecoration`'s list).
    pub fn invoke_with(&mut self, dec: Dec, distance: f64, before: Option<Vec<Token>>, after: Option<Vec<Token>>) -> Result<(), Error> {
        let w = &mut self.w;
        let len = super::walker::trunc_sp(distance).min(w.rem_total);
        w.section = len;
        w.rem_total -= len;
        w.remaining = len;
        w.completed = 0.0;
        if let Some(code) = before {
            self.tex_code(code, &Transform::default(), false, true)?;
            self.after_user_code();
        }
        self.run(dec)?;
        let w = &mut self.w;
        w.path_last = if w.rem_total < 1.0 { w.total_last } else { w.point_ahead(w.remaining) };
        let t = self.t();
        self.final_code(dec, &t)?;
        let rem = self.w.remaining;
        self.w.move_along_input_segment(rem);
        if let Some(code) = after {
            self.tex_code(code, &Transform::default(), false, true)?;
        }
        self.extra = Transform::default();
        self.w.path_first = self.w.path_last;
        Ok(())
    }

    /// What code of the document may have changed: the segment length and amplitude registers,
    /// and `\pgfsetdecorationsegmenttransformation`.
    fn after_user_code(&mut self) {
        let get = |e: &Engine, r: u32| e.tex.dimens.get(&r).map(|v| *v as f64 / 65536.0);
        if let Some(v) = get(self.e, 4016) {
            self.s.segment_length = v;
        }
        if let Some(v) = get(self.e, 4017) {
            self.s.amplitude = v;
        }
        if let Some(code) = macro_body(self.e, "pgf@decorate@additionaltransform") {
            if !tikra_engine::keys::trim(&code).is_empty() {
                let e = &mut *self.e;
                e.begin_group();
                e.st.transform = Transform::default();
                let r = e.run_tokens(code);
                let t = e.st.transform;
                e.end_group();
                match r {
                    Ok(()) => self.extra = Transform { e: t.e / PT, f: t.f / PT, ..t },
                    Err(err) => self.e.warn(format!("decoration segment transformation: {err}")),
                }
            }
        }
    }

    /// Defines what decoration code reads: the registers, `\pgfpointdecoratedpathfirst`… (in the
    /// coordinates of `t`), `\pgfmetadecorated…`.
    fn install_points(&mut self, t: &Transform) {
        super::set_registers(self.e, &self.w);
        let inv = t.inverse();
        let w = &self.w;
        let points = [
            ("pgfpointdecoratedpathfirst", w.path_first),
            ("pgfpointdecoratedpathlast", w.path_last),
            ("pgfpointmetadecoratedpathfirst", w.path_first),
            ("pgfpointmetadecoratedpathlast", w.path_last),
            ("pgfpointdecoratedinputsegmentfirst", w.seg.first),
            ("pgfpointdecoratedinputsegmentlast", w.seg.last),
        ];
        let defs: Vec<(String, String)> = points
            .iter()
            .map(|(n, q)| {
                let l = inv.apply(*q);
                (n.to_string(), format!("\\pgfqpoint{{{}pt}}{{{}pt}}", l.x, l.y))
            })
            .collect();
        let total = w.total;
        let rem = w.rem_total;
        for (n, v) in defs {
            let tokens = tikra_engine::engine::tikra_tokens(&v);
            self.e.tex.set_meaning(&Token::cs(&n), tikra_engine::engine::macro_tokens(tokens), false);
        }
        self.e.tex.define_string("pgfmetadecoratedpathlength", &format!("{total}pt"), false);
        let (ml, ma) = (self.s.meta_segment_length, self.s.meta_amplitude);
        self.e.tex.define_string("pgfmetadecorationsegmentlength", &format!("{ml}pt"), false);
        self.e.tex.define_string("pgfmetadecorationsegmentamplitude", &format!("{ma}pt"), false);
        self.e.tex.define_string("pgfmetadecoratedremainingdistance", &format!("{rem}pt"), false);
        self.e.tex.define_string("pgfmetadecoratedcompleteddistance", &format!("{}pt", total - rem), false);
    }

    /// Runs decoration code written in TeX: `t` (pt) is the transformation, the basic layer's
    /// path is the decoration's (all of it when `full`, else what the code adds).
    pub fn tex_code(&mut self, code: Vec<Token>, t: &Transform, grouped: bool, full: bool) -> Result<(), Error> {
        self.install_points(t);
        let out = &mut self.w.out;
        let scale = Transform::scaling(PT, PT);
        let mut path = tikra_engine::engine::basic::PgfPath { segments: Vec::new(), last: Some(out.last.scale(PT)), start: Some(out.start.scale(PT)), quick: false };
        let had = !out.segs.is_empty();
        if full {
            path.segments = tikra_engine::canvas::transform_segments(&std::mem::take(&mut out.segs), &scale);
        }
        let e = &mut *self.e;
        let saved = std::mem::replace(&mut e.pgf.pgf_path, path);
        let saved_t = (e.st.transform, e.st.pending);
        if grouped {
            e.begin_group();
        }
        e.st.transform = to_cm(t);
        e.st.pending = Transform::default();
        let r = e.run_tokens(code);
        if grouped {
            e.end_group();
        } else {
            (e.st.transform, e.st.pending) = saved_t;
        }
        let path = std::mem::replace(&mut e.pgf.pgf_path, saved);
        let mut segs = tikra_engine::canvas::transform_segments(&path.segments, &Transform::scaling(1.0 / PT, 1.0 / PT));
        let out = &mut self.w.out;
        if !full && had {
            // the basic layer starts a path with a moveto to its current point: the decoration's path goes on
            if let Some(tikra_engine::scene::Segment::MoveTo(q)) = segs.first() {
                if q.minus(out.last).norm() < 1e-6 {
                    segs.remove(0);
                }
            }
        }
        out.segs.extend(segs);
        if let Some(l) = path.last {
            out.last = l.scale(1.0 / PT);
        }
        if let Some(st) = path.start {
            out.start = st.scale(1.0 / PT);
        }
        r
    }

    /// `\pgf@decorate@run`: states until `final`.
    fn run(&mut self, dec: Dec) -> Result<(), Error> {
        self.state = match dec {
            Dec::User(n) => intern(macro_body(self.e, &format!("pgf@decorate@@{n}@initial")).map(|v| tikra_engine::tex::tokens_to_string(&v)).unwrap_or_default().trim()),
            _ => initial(dec),
        };
        self.repeat = -1;
        let mut guard = 0usize;
        loop {
            if self.state == "final" {
                return Ok(());
            }
            guard += 1;
            if guard > tikra_engine::engine::MAX_DECORATION_STEPS {
                self.e.warn("decoration: too many steps (segment length too small?): stopped");
                return Ok(());
            }
            let state = self.state;
            self.next_state = state;
            self.jump = false;
            let opts = self.options(dec, state);
            let (mut pre, mut post) = (false, false);
            for o in &opts {
                self.option(*o, &mut pre, &mut post);
            }
            if self.jump {
                continue;
            }
            let (user_pre, user_post) = (std::mem::take(&mut self.user_pre), std::mem::take(&mut self.user_post));
            if pre {
                if matches!(dec, Dec::User(_)) {
                    self.tex_code(user_pre, &Transform::default(), false, false)?;
                } else {
                    self.precompute(dec, state)?;
                }
            }
            let t = self.t();
            self.code(dec, state, &t)?;
            if post {
                self.tex_code(user_post, &Transform::default(), false, false)?;
            }
            // the width is read when the automaton moves (after the precomputation)
            if let Some(w) = self.options(dec, state).iter().rev().find_map(|o| if let Opt::Width(w) = o { Some(*w) } else { None }) {
                self.width = w;
            }
            let w = self.width;
            self.w.move_along_path(w);
            if self.repeat > 0 {
                self.repeat -= 1;
            } else {
                self.repeat = -1;
                self.state = self.next_state;
            }
        }
    }

    pub(super) fn set_next_state(&mut self, state: &'static str) {
        self.next_state = state;
    }

    fn switch_to(&mut self, state: &'static str) {
        self.state = state;
        self.repeat = -1;
        self.jump = true;
    }

    fn option(&mut self, o: Opt, pre: &mut bool, post: &mut bool) {
        match o {
            Opt::Width(w) => {
                self.width = w;
                self.switch_if(w, "final");
            }
            Opt::Switch(x, s) => self.switch_if(x, s),
            Opt::SwitchInput(x, s) => {
                if !self.jump && self.w.seg_remaining < x - EPS {
                    self.switch_to(s);
                }
            }
            Opt::Next(s) => self.next_state = s,
            Opt::Pre => *pre = true,
            Opt::Post => *post = true,
            Opt::Repeat(n) => {
                if self.repeat == -1 {
                    self.repeat = n;
                }
            }
            Opt::AutoEnd(x) => {
                if self.jump {
                    return;
                }
                if self.w.is_close {
                    if self.w.remaining > x + EPS {
                        if self.w.seg_remaining <= x + EPS {
                            self.w.out.close();
                            let d = self.w.seg_remaining;
                            self.w.move_along_path(d);
                            self.repeat = -1;
                            self.jump = true;
                        }
                    } else {
                        self.w.out.close();
                        self.width = self.w.remaining;
                        self.switch_to("final");
                    }
                } else if self.w.remaining <= x + EPS {
                    let t = self.t();
                    let r = self.w.remaining;
                    self.ln(&t, r, 0.0);
                    self.width = r;
                    self.switch_to("final");
                }
            }
            Opt::AutoCorner(x) => {
                if self.has_corners && !self.jump && self.w.seg_remaining <= x + EPS {
                    let t = self.t();
                    let r = self.w.seg_remaining;
                    self.ln(&t, r, 0.0);
                    self.w.move_along_path(r);
                    self.repeat = -1;
                    self.jump = true;
                }
            }
        }
    }

    fn switch_if(&mut self, x: f64, state: &'static str) {
        if self.jump {
            return;
        }
        if self.w.remaining.abs() < EPS || self.w.remaining < x - EPS {
            self.switch_to(state);
        }
    }

    fn options(&mut self, dec: Dec, state: &str) -> Vec<Opt> {
        use Opt::*;
        let (a, l) = (self.amp(), self.len());
        let rem = self.w.remaining;
        let seg_rem = self.w.seg_remaining;
        let aspect = self.s.aspect;
        match (dec, state) {
            (Dec::User(n), s) => self.user_options(n, s),
            (Dec::LineTo, "initial") => vec![SwitchInput(rem, "input segment"), Width(rem)],
            (Dec::LineTo, _) => vec![Width(self.w.seg.len), Next("initial")],
            (Dec::MoveTo, _) => vec![Width(rem)],
            (Dec::CurveTo, _) => vec![Width(self.w.seg.len / 100.0)],
            (Dec::Zigzag, "up from center") => vec![Width(0.5 * l), Next("big down")],
            (Dec::Zigzag, "big down") => vec![Switch(0.5 * l, "center finish"), Width(0.5 * l), Next("big up")],
            (Dec::Zigzag, "big up") => vec![Switch(0.5 * l, "center finish"), Width(0.5 * l), Next("big down")],
            (Dec::Zigzag, _) => vec![Width(0.0), Next("final")],
            (Dec::Saw, _) => vec![AutoEnd(l), AutoCorner(l), Width(l)],
            (Dec::RandomSteps, "start") => vec![Width(0.0), Next("step"), Pre],
            (Dec::RandomSteps, _) => vec![AutoEnd(1.5 * l), AutoCorner(1.5 * l), Width(l)],
            (Dec::Bent, _) => vec![Width(seg_rem)],
            (Dec::Snake, "initial") => vec![Switch(0.625 * l, "final"), Width(0.3125 * l), Next("down")],
            (Dec::Snake, "down") => vec![Switch(0.8125 * l, "end down"), Width(0.5 * l), Next("up")],
            (Dec::Snake, "up") => vec![Switch(0.8125 * l, "end up"), Width(0.5 * l), Next("down")],
            (Dec::Snake, _) => vec![Width(0.3125 * l), Next("final")],
            (Dec::Coil, "coil") => vec![Switch(1.5 * l + 2.0 * aspect * a, "last"), Width(l)],
            (Dec::Coil, _) => vec![Width(0.5 * l + 2.0 * aspect * a), Next("final")],
            (Dec::Bumps, _) => vec![AutoEnd(0.51 * l), AutoCorner(0.51 * l), Width(0.5 * l)],
            (Dec::Ticks, _) => vec![Width(l)],
            (Dec::ExpandingWaves, "initial") => vec![Width(l), Next("wave")],
            (Dec::ExpandingWaves, "wave") => vec![Switch(l, "last"), Width(l)],
            (Dec::ExpandingWaves, _) => vec![Width(0.0), Next("final")],
            (Dec::Waves, _) => vec![Width(l)],
            (Dec::Border, "tick") => vec![Switch(l, "last"), Width(l)],
            (Dec::Border, _) => vec![Width(a), Next("final")],
            (Dec::Brace, _) => vec![Width(rem), Next("final")],
            (Dec::ShowPathConstruction, "check for moveto") => vec![Width(0.0), Next("segment"), Pre],
            (Dec::ShowPathConstruction, _) => vec![Width(seg_rem), Next("check for moveto"), Pre],
            (Dec::Markings, "init") => vec![Width(0.0), Pre, Next("pre")],
            (Dec::Markings, "pre") => vec![Width(0.0), Pre, Next("skipper")],
            (Dec::Markings, "skipper") => vec![Width(self.mark_at - self.w.completed - SP), Next(self.mark_next)],
            (Dec::Markings, _) => vec![Width(SP), Next("pre"), Pre],
            (Dec::Triangles | Dec::Crosses, "init") => vec![Width(0.0), Next(if dec == Dec::Triangles { "triangle" } else { "crosses" }), Pre],
            (Dec::Triangles, "triangle") | (Dec::Crosses, "crosses") => vec![Switch(l, "last"), Width(l)],
            (Dec::Triangles, "last") => vec![Switch(self.ssw, "skip"), Width(rem), Next("final")],
            (Dec::Triangles | Dec::Crosses, _) => vec![Width(rem)],
            (Dec::Koch1 | Dec::Koch2 | Dec::KochSnowflake | Dec::Cantor, _) => vec![Width(seg_rem)],
            (Dec::CircleConnectionBar, "initial") => vec![Width(0.0), Next("bar")],
            (Dec::CircleConnectionBar, "bar") => vec![Width(0.0), Next("end")],
            (Dec::CircleConnectionBar, _) => vec![Width(0.0), Next("final")],
            (Dec::Footprints, "left") => {
                let w = self.key_pt("stride length", 30.0) / 2.0;
                vec![Width(w), Next("right")]
            }
            (Dec::Footprints, _) => {
                let w = self.key_pt("stride length", 30.0) / 2.0;
                vec![Width(w), Next("left")]
            }
            (Dec::ShapeBackgrounds, "initialise") => vec![Width(self.shape.initialise), Next("shape"), Pre],
            (Dec::ShapeBackgrounds, "before shape") => vec![Width(self.shape.before - SP), Next("shape"), Pre],
            (Dec::ShapeBackgrounds, "shape") => vec![Width(0.0), Next("after shape")],
            (Dec::ShapeBackgrounds, "after shape") => vec![Width(self.shape.after - SP), Next("sep"), Pre],
            (Dec::ShapeBackgrounds, _) => vec![Width(self.shape.sep), Next("before shape"), Pre],
            (Dec::TextEffects, "setup") => vec![Width(0.0), Next("scan"), Pre],
            (Dec::TextEffects, "scan") => vec![Width(0.0), Next("pre token"), Pre],
            (Dec::TextEffects, "pre token") => vec![Width(self.te_current().map_or(0.0, |c| c.pre)), Next("token")],
            (Dec::TextEffects, "token") => vec![Width(0.0), Next("post token")],
            (Dec::TextEffects, _) => vec![Width(self.te_current().map_or(0.0, |c| c.post)), Next("scan")],
            (Dec::TextAlongPath, "initial") => vec![Width(0.0), Next("left indent"), Pre],
            (Dec::TextAlongPath, "left indent") => vec![Width(self.text.indent_left), Next("scan")],
            (Dec::TextAlongPath, "scan") => vec![Width(0.0), Next("before typeset"), Pre],
            (Dec::TextAlongPath, "before typeset") => vec![Width(0.5 * self.text_width()), Next("typeset")],
            (Dec::TextAlongPath, "typeset") => vec![Width(0.0), Next("after typeset")],
            (Dec::TextAlongPath, "after typeset") => vec![Width(0.5 * self.text_width()), Next("shift"), Pre],
            (Dec::TextAlongPath, _) => vec![Width(self.text.shift), Next("scan")],
        }
    }

    /// The persistent precomputation of a state.
    fn precompute(&mut self, dec: Dec, state: &str) -> Result<(), Error> {
        match (dec, state) {
            (Dec::RandomSteps, _) => self.has_corners = true,
            (Dec::Triangles, _) => {
                self.ssw = self.key_pt("shape start width", 2.5);
                self.ssh = self.key_pt("shape start height", 2.5) / 2.0;
            }
            (Dec::Crosses, _) => {
                self.ssw = self.key_pt("shape start width", 2.5) / 2.0;
                self.ssh = self.key_pt("shape start height", 2.5) / 2.0;
            }
            (Dec::ShowPathConstruction, "check for moveto") => {
                if self.w.prev_is_moveto {
                    self.segment_code(0)?;
                }
            }
            (Dec::ShowPathConstruction, _) => self.segment_code_current()?,
            (Dec::Markings, "init") => {
                self.seq = 0;
                self.marks = self.s.marks.iter().cloned().collect();
                self.set_mark_info()?;
            }
            (Dec::Markings, "pre") => self.next_mark()?,
            (Dec::Markings, _) => {
                self.seq += 1;
                self.set_mark_info()?;
            }
            (Dec::ShapeBackgrounds, s) => self.shape_precompute(s)?,
            (Dec::TextEffects, "setup") => self.te_setup()?,
            (Dec::TextEffects, _) => self.te_scan(),
            (Dec::TextAlongPath, "initial") => self.text_initial()?,
            (Dec::TextAlongPath, "scan") => self.text_scan()?,
            (Dec::TextAlongPath, _) => {
                let tx = &mut self.text;
                tx.shift = if tx.fit {
                    if tx.stretch {
                        if tx.is_space {
                            tx.space_shift
                        } else {
                            0.0
                        }
                    } else {
                        tx.char_shift
                    }
                } else {
                    0.0
                };
            }
            _ => {}
        }
        Ok(())
    }
}

/// `.25cm, between centers` → ("0.25cm", "between centers").
fn split_keyword(s: &str) -> (String, String) {
    match s.split_once(',') {
        Some((a, b)) => (a.trim().to_string(), b.trim().to_string()),
        None => (s.trim().to_string(), String::new()),
    }
}

/// A text format without its `\color{…}`, and the color.
fn split_color(format: &str) -> (String, Option<String>) {
    let mut font = String::new();
    let mut color = None;
    let mut rest = format;
    while let Some(i) = rest.find("\\color") {
        font.push_str(&rest[..i]);
        let after = rest[i + 6..].trim_start();
        if let Some(b) = after.strip_prefix('{') {
            if let Some(j) = b.find('}') {
                color = Some(b[..j].to_string());
                rest = &b[j + 1..];
                continue;
            }
        }
        rest = after;
    }
    font.push_str(rest);
    (font, color)
}
