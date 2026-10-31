//! Data visualizations: `\datavisualization [options] data {…} …;`, drawn as pgf's
//! datavisualization module and TikZ's data visualization libraries draw them.
//!
//! A data visualization is made of objects that listen to signals. A data point is the current
//! value of the keys under `/data point/`; each time a data format has read one, `\pgfdatapoint`
//! emits signals: in the survey (done first) the axes see the ranges of their attributes; in the
//! visualization their mappers scale each attribute, the transformers turn the scaled attributes
//! into a position, and the visualizers remember the positions, to draw them at the end. Then the
//! grids, ticks, axes, labels and legends are drawn from what the survey found.
//!
//! The keys are the libraries' ([`keys`], [`library`]); objects ([`objects`]), formats
//! ([`formats`]) and drawing ([`axes`], [`legend`]) are native. An object's handle is the macro
//! `\tikra@dv@obj{n}`, and `\tikra@dv@obj{n}.method(args)` calls a method, so that the documented
//! object interface (`new object`, `\pgfdvmin`…) works.

mod axes;
mod formats;
mod keys;
mod legend;
mod library;
mod numbers;
mod objects;

use tikra_engine::keys::macro_name;
use tikra_engine::scene::Point;
use tikra_engine::tex::{self, Cat, Meaning, Token};
use tikra_engine::{Error, PT};

use tikra_engine::engine::piece::{CommandFn, Piece};
use tikra_engine::engine::{macro_tokens, tikra_tokens, Engine, NATIVE_KEY_BASE};

pub use objects::Object;

/// Data visualizations, plugged into an engine.
pub static PIECE: Piece = Piece {
    name: "datavis",
    commands: COMMANDS,
    coordinate_systems: &[("visualization", visualization_cs)],
    reading_input: &["datavisualization"],
    define,
    definitions,
    ..Piece::EMPTY
};

/// The native commands of data visualizations, by their TeX name.
const COMMANDS: &[(&str, CommandFn)] = &[
    ("datavisualization", |e, _| datavisualization(e)),
    ("pgfdatapoint", |e, _| data_point(e)),
    ("pgfkeysaddvalue", keys_add_value),
    ("pgfdvvisualizerfilter", visualizer_filter),
    ("pgfmathapproxequalto", |e, _| crate::numbers::approx_equal_command(e)),
    ("tikzdvdeclarestylesheetcolorseries", |e, _| crate::legend::declare_color_series(e)),
    ("tikra@dv@obj", obj_command),
    ("tikra@dv@data", data_command),
    ("tikra@dv@newobject", new_object_command),
    ("tikra@dv@make", |e, _| make_object(e)),
    ("tikra@dv@include", include_command),
    ("tikra@dv@setrange", setrange_command),
    ("tikra@dv@printnumber", |e, _| crate::numbers::print_number_command(e)),
    ("tikra@dv@colorseries", |e, _| crate::legend::color_series(e)),
    ("tikra@dv@stylesignal", |e, _| emit(e, Signal::Style, None)),
    ("tikra@dv@adddata", adddata_command),
    ("tikra@dv@about", about_command),
    ("tikra@dv@pi", |e, _| crate::axes::pi_typesetter(e)),
    ("tikra@dv@finish", |e, _| finish(e)),
    ("tikra@dv@addentry", add_entry_command),
    ("tikra@dv@legendentry", legend_entry_command),
    ("tikra@dv@sample", |e, _| crate::legend::legend_sample(e, false)),
    ("tikra@dv@rectsample", |e, _| crate::legend::legend_sample(e, true)),
    ("tikra@dv@labeldraw", label_draw_command),
];

/// The signals of a data visualization, in the order pgf makes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Prepare,
    Map,
    Transform,
    Visualize,
    Finish,
    Survey,
    Phase,
    Path,
    Direction,
    Style,
}

const SIGNALS: usize = 10;

/// What the phase signal emits (`\pgfdvbeforesurvey`…).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    BeforeSurvey,
    BeginSurvey,
    EndSurvey,
    AfterSurvey,
    BeforeVisualization,
    BeginVisualization,
    EndVisualization,
    AfterVisualization,
}

/// What a path or direction signal is about (`\pgfdvpathmovetotoken`…).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathToken {
    MoveTo,
    LineTo,
    DirectionFrom,
    DirectionTo,
    DirectionAt,
}

/// The slot of an object that a signal calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Survey,
    Map,
    Transform,
    Phase,
    Protocol,
    Visualize,
    Test,
    InvokeStyle,
    Apply,
    Path,
    Direction,
}

/// The code a data visualization runs around its phases (the attributes of pgf's class).
#[derive(Debug, Clone, Default)]
pub struct Hooks {
    pub before_survey: Vec<Token>,
    pub at_start_survey: Vec<Token>,
    pub at_end_survey: Vec<Token>,
    pub after_survey: Vec<Token>,
    pub before_visualization: Vec<Token>,
    pub at_start_visualization: Vec<Token>,
    pub at_end_visualization: Vec<Token>,
    pub after_visualization: Vec<Token>,
}

/// One `\datavisualization` being run.
#[derive(Debug, Clone)]
pub struct Dv {
    /// the objects, by handle; 0 is the data visualization itself
    pub objects: Vec<Object>,
    pub signals: [Vec<(usize, Slot)>; SIGNALS],
    pub hooks: Hooks,
    /// the code that produces the data points, run once per survey and once to visualize
    pub data: Vec<Token>,
    pub visualizing: bool,
    pub survey_count: usize,
    pub number_of_surveys: usize,
    /// the picture's bounding box before the data visualization (its own starts empty)
    pub outer_box: (Option<Point>, Option<Point>),
    /// the tick placement in effect: logarithmic steps for `about=`
    pub log_steps: bool,
    /// for a polar direction: the angle and radius at its start and end
    pub polar_from: (Option<f64>, Option<f64>),
    pub polar_to: (Option<f64>, Option<f64>),
    /// the axes made so far (`all axes`), by name
    pub axes: Vec<String>,
    /// how many visualizers there are (their numbers for style sheets)
    pub visualizers: usize,
    /// what is drawn after the data
    pub actions: Actions,
    /// `major=`, `minor=`… are ignored while tick positions are computed
    pub setters_off: bool,
}

/// What `visualize axis`, `visualize ticks`, `visualize grid`, `visualize label` and legends ask
/// for: the axis and the options.
#[derive(Debug, Clone, Default)]
pub struct Actions {
    pub grids: Vec<(String, Vec<Token>)>,
    pub ticks: Vec<(String, Vec<Token>)>,
    pub axes: Vec<(String, Vec<Token>)>,
    pub labels: Vec<(String, Vec<Token>)>,
    pub legends: Vec<String>,
}

impl Dv {
    fn new() -> Dv {
        Dv {
            objects: vec![Object::Main],
            signals: Default::default(),
            hooks: Hooks::default(),
            data: Vec::new(),
            visualizing: false,
            survey_count: 1,
            number_of_surveys: 1,
            outer_box: (None, None),
            log_steps: false,
            polar_from: (None, None),
            polar_to: (None, None),
            axes: Vec::new(),
            visualizers: 0,
            actions: Actions::default(),
            setters_off: false,
        }
    }

    pub fn connect(&mut self, id: usize, slot: Slot, signal: Signal) {
        self.signals[signal as usize].push((id, slot));
    }
}

/// The native keys and their defaults. The TeX of the library runs with TikZ's definitions.
fn define(e: &mut Engine) {
    for (path, f) in keys::NATIVE {
        let id = NATIVE_KEY_BASE + e.key_fns.len() as u16;
        e.key_fns.push(*f);
        e.tex.define(&macro_name(&format!("{path}/.@cmd")), Meaning::Native(id));
    }
    for (path, v) in keys::DEFAULTS {
        e.tex.set_meaning(&Token::cs(&macro_name(&format!("{path}/.@def"))), macro_tokens(tikra_tokens(v)), false);
    }
}

/// The library's keys, as TeX. `/utils/exec` and `\pgfkeyslet` are defined here because the
/// libraries that own them are installed after this text runs; both definitions are the same ones.
fn definitions() -> String {
    let mut text = String::from("\\pgfkeys{/utils/exec/.code={#1}}");
    text.push_str("\\def\\pgfkeyslet#1#2{\\expandafter\\let\\csname pgfk@#1\\endcsname#2}");
    text.push_str(numbers::NUMBER_FORMAT);
    text.push_str(&library::definitions());
    text
}

/// `\bgroup\pgfkeys{/data point/.cd,#1}\pgfdatapoint\egroup`
pub fn data_point_code(options: Vec<Token>) -> Vec<Token> {
    let mut code = vec![Token::cs("bgroup"), Token::cs("pgfkeys"), Token::Char('{', Cat::Begin)];
    code.extend(tikra_tokens("/data point/.cd,"));
    code.extend(options);
    code.push(Token::Char('}', Cat::End));
    code.push(Token::cs("pgfdatapoint"));
    code.push(Token::cs("egroup"));
    code
}

// ---------------------------------------------------------------- helpers

pub fn group(v: Vec<Token>) -> Vec<Token> {
    let mut out = Vec::with_capacity(v.len() + 2);
    out.push(Token::Char('{', Cat::Begin));
    out.extend(v);
    out.push(Token::Char('}', Cat::End));
    out
}

/// A macro's expansion as text (empty when undefined).
pub fn macro_text(e: &mut Engine, name: &str) -> String {
    match e.tex.meaning_of(name) {
        Meaning::Macro(m) => {
            let v = tex::substitute(&m.body, &[]);
            e.tex.expand_to_string(v).unwrap_or_default().trim().to_string()
        }
        _ => String::new(),
    }
}

/// A key's value, expanded, as text; None when the key has no value.
pub fn key_text(e: &mut Engine, path: &str) -> Option<String> {
    let v = tikra_engine::keys::value(&e.tex, path)?;
    if v.len() == 1 && v[0].is_cs("relax") {
        return None;
    }
    Some(e.tex.expand_to_string(v).unwrap_or_default().trim().to_string())
}

pub fn set_key(e: &mut Engine, path: &str, value: &str) {
    e.tex.define_string(&macro_name(path), value, false);
}

/// A number as data visualization keys store it: plain decimal, full precision.
pub fn num(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let r = (v * 1e9).round() / 1e9;
    let s = if (r - v).abs() <= 1e-12 * v.abs().max(1.0) { format!("{r}") } else { format!("{v}") };
    s
}

/// `\pgfdvmathenter`: a number, `(expression)`, or a stored value.
pub fn parse_value(e: &mut Engine, text: &str) -> Option<f64> {
    let t = text.trim().trim_start_matches('@');
    let t = t.strip_prefix('{').and_then(|r| r.strip_suffix('}')).unwrap_or(t).trim();
    if t.is_empty() {
        return None;
    }
    if let Ok(v) = t.parse::<f64>() {
        return Some(v);
    }
    let expr = t.strip_prefix('(').and_then(|r| r.strip_suffix(')')).unwrap_or(t);
    e.number(expr).ok()
}

/// A data point attribute's value (`\pgfkeysgetvalue{/data point/…}`), None if unset or empty.
pub fn attribute(e: &mut Engine, name: &str) -> Option<String> {
    key_text(e, &format!("/data point/{name}")).filter(|s| !s.is_empty())
}

pub fn set_attribute(e: &mut Engine, name: &str, value: &str) {
    set_key(e, &format!("/data point/{name}"), value);
}

/// The data visualizations being run, innermost last. One stack for the whole figure.
pub(crate) fn running(e: &mut Engine) -> &mut Vec<Dv> {
    e.piece_state()
}

/// The innermost data visualization.
pub fn dv(e: &mut Engine) -> Result<&mut Dv, Error> {
    crate::running(e).last_mut().ok_or_else(|| Error::Figure("data visualization keys outside \\datavisualization".into()))
}

fn obj_command(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let id = e.argument_string()?;
    let (method, args) = read_method(e)?;
    crate::objects::call(e, id.parse().unwrap_or(usize::MAX), &method, args)
}

fn data_command(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let options = e.argument()?;
    let body = e.argument()?;
    crate::formats::run_data(e, options, body)
}

fn new_object_command(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let store = e.argument_string()?;
    let spec = e.argument()?;
    crate::objects::new_object(e, &store, spec)
}

fn include_command(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let axis = e.argument_string()?;
    let values = e.argument()?;
    let values = e.tex.expand_fully(values)?;
    let mapper = object_of(e, &format!("{axis}/scaling mapper"));
    for item in tikra_engine::engine::foreach_items(&values) {
        let s = tex::tokens_to_string(&item);
        if let (Some(m), Some(v)) = (mapper, parse_value(e, &s)) {
            crate::objects::include_value(e, m, v);
        }
    }
    Ok(())
}

fn setrange_command(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let axis = e.argument_string()?;
    let value = e.argument_string()?;
    let which = e.argument_string()?;
    if let (Some(m), Some(v)) = (object_of(e, &format!("{axis}/scaling mapper")), parse_value(e, &value)) {
        crate::objects::set_range(e, m, &which, v);
    }
    Ok(())
}

fn adddata_command(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let code = e.argument()?;
    add_data_call(e, code)
}

fn keys_add_value(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let key = e.argument_string()?;
    let pre = e.argument()?;
    let post = e.argument()?;
    let old = tikra_engine::keys::value(&e.tex, &key).unwrap_or_default();
    let v = [pre, old, post].concat();
    e.tex.set_meaning(&Token::cs(&macro_name(&key)), macro_tokens(v), false);
    Ok(())
}

fn visualizer_filter(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let name = e.argument_string()?;
    if attribute(e, "set").unwrap_or_default() != name {
        e.tex.set_meaning(&Token::cs("ifpgfdvfilterpassed"), Meaning::Prim(tex::Prim::Iffalse), false);
    }
    Ok(())
}

fn about_command(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let n = e.argument_string()?;
    crate::axes::compute_about(e, &n)
}

fn add_entry_command(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let legend = e.argument_string()?;
    let options = e.argument()?;
    if let Some(id) = object_of(e, &format!("{}/{legend}/obj", keys::DV)) {
        let mut entry = vec![Token::cs("tikra@dv@legendentry")];
        entry.extend(group(options));
        crate::objects::call(e, id, "add entry", vec![entry])?;
    }
    Ok(())
}

fn legend_entry_command(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let options = e.argument()?;
    crate::legend::render_legend_entry(e, options)
}

fn label_draw_command(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let name = e.argument_string()?;
    let options = e.argument()?;
    crate::legend::draw_label_in_data(e, &name, options)
}

/// `\tikra@dv@make{kind}{…}`: one of the objects the keys ask for, made at its time.
pub(crate) fn make_object(e: &mut Engine) -> Result<(), Error> {
    let kind = e.argument_string()?;
    let dv = keys::DV;
    let arg = |e: &mut Engine| e.argument_string();
    let text = |e: &mut Engine, key: &str| key_text(e, key).unwrap_or_default();
    let made = |e: &mut Engine, key: &str| key_text(e, key).is_some_and(|s| !s.is_empty());
    let store = |e: &mut Engine, key: &str, id: Option<usize>| {
        if let Some(id) = id {
            e.tex.set_meaning(&Token::cs(&macro_name(key)), macro_tokens(handle(id)), false);
        }
    };
    let t = |s: &str| tex::string_tokens(s);
    match kind.as_str() {
        "mapper" => {
            let axis = arg(e)?;
            let key = format!("{axis}/scaling mapper");
            if made(e, &key) {
                return Ok(());
            }
            crate::axes::prepare_scaling(e, &axis)?;
            let input = text(e, &format!("{axis}/attribute"));
            let (scaling, function) = (text(e, &format!("{axis}/scaling")), text(e, &format!("{axis}/function")));
            let id = crate::objects::create(e, "scaling mapper", &[t(&input), t(&format!("{input}/scaled")), t(&scaling), t(&function)])?;
            store(e, &key, id);
        }
        "linear" => {
            let axis = arg(e)?;
            let key = format!("{axis}/linear transformer");
            if made(e, &key) {
                return Ok(());
            }
            let a = text(e, &format!("{axis}/attribute"));
            let unit = text(e, &format!("{axis}/unit vector"));
            let id = crate::objects::create(e, "linear transformer", &[t(&format!("{a}/scaled")), t(&unit)])?;
            store(e, &key, id);
        }
        "polar" => {
            let (angle, radius) = (arg(e)?, arg(e)?);
            let key = format!("{angle}/polar transformer");
            if made(e, &key) {
                return Ok(());
            }
            let (a, r) = (text(e, &format!("{angle}/attribute")), text(e, &format!("{radius}/attribute")));
            let (u0, u90) = (text(e, &format!("{angle}/unit vector 0 degrees")), text(e, &format!("{angle}/unit vector 90 degrees")));
            let id = crate::objects::create(e, "polar transformer", &[t(&format!("{a}/scaled")), t(&format!("{r}/scaled")), t(&u0), t(&u90)])?;
            store(e, &key, id);
        }
        "plot" => {
            let name = arg(e)?;
            let handlers = e.argument()?;
            let key = format!("{dv}/visualizers/{name}");
            if made(e, &key) {
                return Ok(());
            }
            let id = crate::objects::create(e, "plot handler visualizer", &[t(&name), handlers])?;
            store(e, &key, id);
        }
        "rectangle" => {
            let name = arg(e)?;
            let key = format!("{dv}/visualizers/{name}");
            if made(e, &key) {
                return Ok(());
            }
            let id = crate::objects::create(e, "rectangle visualizer", &[t(&name)])?;
            store(e, &key, id);
        }
        "count" => {
            let name = arg(e)?;
            let key = format!("{dv}/visualizers/{name}/count/obj");
            if made(e, &key) {
                return Ok(());
            }
            let id = crate::objects::create(e, "count", &[t(&format!("set/{name}/count"))])?;
            store(e, &key, id);
            if let Some(id) = id {
                let filter = tikra_tokens(&format!("\\pgfdvvisualizerfilter{{{name}}}"));
                crate::objects::call(e, id, "set filter", vec![filter])?;
            }
        }
        "label" => {
            let name = arg(e)?;
            let options = e.argument()?;
            crate::legend::make_label_in_data(e, &name, options)?;
        }
        "legend" => {
            let name = arg(e)?;
            let base = format!("{dv}/{name}");
            if made(e, &format!("{base}/obj")) {
                return Ok(());
            }
            let args: Vec<Vec<Token>> = ["@columns or rows", "@ideal", "max entries", "@strategy"].iter().map(|k| t(&text(e, &format!("{base}/{k}")))).collect();
            let id = crate::objects::create(e, "legend", &args)?;
            store(e, &format!("{base}/obj"), id);
            crate::dv(e)?.actions.legends.push(name);
        }
        other => e.warn(format!("data visualization: no object '{other}'")),
    }
    Ok(())
}

/// After the visualization: the bounding boxes, then the grids, ticks, axes, labels and legends
/// (the grids and ticks first all their positions, then subminor, minor, major).
pub(crate) fn finish(e: &mut Engine) -> Result<(), Error> {
    update_bounding_box(e, true);
    let actions = dv(e)?.actions.clone();
    for (what, list) in [("grid", &actions.grids), ("ticks", &actions.ticks)] {
        for (axis, _) in list {
            crate::axes::default_action(e, what, axis)?;
        }
        for kind in ["subminor", "minor", "major"] {
            for (axis, options) in list {
                if what == "grid" {
                    crate::axes::show_grid(e, kind, axis, options.clone())?;
                } else {
                    crate::axes::show_ticks(e, kind, axis, options.clone())?;
                }
            }
        }
    }
    for (axis, options) in &actions.axes {
        crate::axes::show_axis(e, axis, options.clone())?;
    }
    for (axis, options) in &actions.labels {
        crate::axes::show_label(e, axis, options.clone())?;
    }
    for legend in &actions.legends {
        crate::legend::show_legend(e, legend)?;
    }
    Ok(())
}

// ---------------------------------------------------------------- the \datavisualization command

/// `\datavisualization [options] data {…} …;`: a sequence of blocks up to the semicolon, then the
/// survey and the visualization.
pub fn datavisualization(e: &mut Engine) -> Result<(), Error> {
    if crate::running(e).len() >= 8 {
        return Err(Error::Figure("data visualizations nested too deeply".into()));
    }
    e.begin_group();
    let mut dv = Dv::new();
    // the data visualization's own bounding box starts empty
    dv.outer_box = (e.canvas.min.take(), e.canvas.max.take());
    crate::running(e).push(dv);
    if e.facts.mute == 0 {
        e.facts.data_visualizations += 1;
    }
    let r = datavisualization_body(e);
    let dv = crate::running(e).pop();
    if let Some(dv) = dv {
        update_bounding_box(e, false);
        let (lo, hi) = dv.outer_box;
        for p in [lo, hi].into_iter().flatten() {
            e.canvas.include(p);
        }
    }
    e.end_group();
    r
}

pub(crate) fn datavisualization_body(e: &mut Engine) -> Result<(), Error> {
    let mut handle = vec![Token::cs("tikra@dv@obj")];
    handle.extend(group(tex::string_tokens("0")));
    e.tex.set_meaning(&Token::cs("tikra@dv@main"), macro_tokens(handle), false);
    // \pgfmathprintnumber prints as pgf does, with /pgf/number format, inside data visualizations.
    // The binding is the piece's own command, copied into this group so the group's end restores it.
    let meaning = e.tex.meaning_of("tikra@dv@printnumber");
    e.tex.set_meaning(&Token::cs("pgfmathprintnumber"), meaning, false);
    let after = vec![Token::cs("tikra@dv@finish")];
    dv(e)?.hooks.after_visualization = after;
    parse_blocks(e)?;
    survey(e)?;
    visualize(e)
}

/// The blocks up to the semicolon.
pub(crate) fn parse_blocks(e: &mut Engine) -> Result<(), Error> {
    let mut scopes = 0usize;
    loop {
        let Some(t) = e.tex.next_nonspace() else { return Err(Error::Figure("\\datavisualization: ';' missing".into())) };
        match &t {
            Token::Char(';', _) => {
                if scopes > 0 {
                    e.warn("\\datavisualization: a scope or data group is not closed");
                }
                for _ in 0..scopes {
                    e.end_group();
                }
                return Ok(());
            }
            Token::Cs(n) if &**n == "par" => {}
            Token::Char('[', _) => {
                let o = e.tex.read_until_char(']', false)?;
                e.keys_command(o, Some("/tikz/data visualization"), true)?;
            }
            Token::Char(_, Cat::End) if scopes > 0 => {
                // the end of a scope or of a data group's definition
                do_add_data(e, vec![Token::cs("endgroup")])?;
                e.end_group();
                scopes -= 1;
            }
            Token::Char(c, Cat::Letter) => {
                e.tex.back(t.clone());
                let w = word(e);
                match (*c, w.as_str()) {
                    (_, "data") => scopes += data_block(e)?,
                    (_, "scope") => {
                        let o = e.optional()?.unwrap_or_default();
                        e.begin_group();
                        e.keys_command(data_keys(o.clone()), Some("/"), true)?;
                        let mut code = vec![Token::cs("begingroup"), Token::cs("pgfkeys")];
                        code.extend(group(data_keys(o)));
                        do_add_data(e, code)?;
                        match e.tex.next_nonspace() {
                            Some(Token::Char(_, Cat::Begin)) => scopes += 1,
                            other => {
                                if let Some(o) = other {
                                    e.tex.back(o);
                                }
                                e.warn("\\datavisualization: scope without '{'");
                                e.end_group();
                            }
                        }
                    }
                    (_, "info") => {
                        let prime = match e.tex.get_token() {
                            Some(x) if x.is_char('\'') => true,
                            Some(x) => {
                                e.tex.back(x);
                                false
                            }
                            None => false,
                        };
                        let o = e.optional()?.unwrap_or_default();
                        let body = e.argument()?;
                        let mut code = tikra_tokens("{\\begin{scope}[");
                        code.extend(o);
                        code.push(Token::other(']'));
                        code.extend(body);
                        code.extend(tikra_tokens("\\end{scope}}"));
                        let dv = dv(e)?;
                        if prime {
                            dv.hooks.before_visualization.extend(code);
                        } else {
                            let old = std::mem::take(&mut dv.hooks.after_visualization);
                            dv.hooks.after_visualization = [code, old].concat();
                        }
                    }
                    _ => return Err(Error::Figure(format!("\\datavisualization: semicolon expected before '{w}'"))),
                }
            }
            _ => return Err(Error::Figure(format!("\\datavisualization: semicolon expected before '{}'", tex::show_token(&t)))),
        }
    }
}

/// The letters that follow, as a word.
pub(crate) fn word(e: &mut Engine) -> String {
    let mut w = String::new();
    while let Some(t) = e.tex.get_token() {
        match t {
            Token::Char(c, Cat::Letter) => w.push(c),
            other => {
                e.tex.back(other);
                break;
            }
        }
    }
    w
}

/// After `data`: `point [options]`, `group [options] {name} = {…}` (or `+=`, or a use), or a data
/// set `[options] {…}`. Returns the scopes it opened (a group being defined).
pub(crate) fn data_block(e: &mut Engine) -> Result<usize, Error> {
    let next = e.tex.next_nonspace();
    match next {
        Some(Token::Char('p', Cat::Letter)) => {
            e.tex.back(Token::Char('p', Cat::Letter));
            let w = word(e);
            if w != "point" {
                return Err(Error::Figure(format!("\\datavisualization: 'data {w}' not understood")));
            }
            let o = e.optional()?.unwrap_or_default();
            do_add_data(e, data_point_code(o))?;
            Ok(0)
        }
        Some(Token::Char('g', Cat::Letter)) => {
            e.tex.back(Token::Char('g', Cat::Letter));
            let w = word(e);
            if w != "group" {
                return Err(Error::Figure(format!("\\datavisualization: 'data {w}' not understood")));
            }
            let o = e.optional()?.unwrap_or_default();
            let name = e.argument_string()?;
            let mut define = false;
            match e.tex.next_nonspace() {
                Some(x) if x.is_char('=') => {
                    // a new group
                    e.tex.set_meaning(&Token::cs(&format!("tikra@dv@group@{name}")), macro_tokens(Vec::new()), true);
                    define = true;
                }
                Some(x) if x.is_char('+') => {
                    if let Some(y) = e.tex.get_token() {
                        if !y.is_char('=') {
                            e.tex.back(y);
                        }
                    }
                    define = true;
                }
                Some(x) => e.tex.back(x),
                None => {}
            }
            if define {
                match e.tex.next_nonspace() {
                    Some(Token::Char(_, Cat::Begin)) => {
                        e.begin_group();
                        set_key(e, "/pgf/data/store in group", &name);
                        let mut code = vec![Token::cs("begingroup"), Token::cs("pgfkeys")];
                        code.extend(group(data_keys(o)));
                        do_add_data(e, code)?;
                        return Ok(1);
                    }
                    other => {
                        if let Some(o) = other {
                            e.tex.back(o);
                        }
                        e.warn("\\datavisualization: data group without '{'");
                        return Ok(0);
                    }
                }
            }
            // a use: the group's code adds its data
            e.begin_group();
            let r = (|| {
                e.keys_command(data_keys(o), Some("/"), true)?;
                let name = format!("tikra@dv@group@{name}");
                match e.tex.meaning_of(&name) {
                    Meaning::Macro(m) => {
                        let code = tex::substitute(&m.body, &[]);
                        e.begin_group();
                        set_key(e, "/pgf/data/store in group", "");
                        let r = e.run_tokens(code);
                        e.end_group();
                        r
                    }
                    _ => {
                        e.warn(format!("undefined data group '{}'", name.trim_start_matches("tikra@dv@group@")));
                        Ok(())
                    }
                }
            })();
            e.end_group();
            r.map(|_| 0)
        }
        other => {
            if let Some(o) = other {
                e.tex.back(o);
            }
            data_set(e)?;
            Ok(0)
        }
    }
}

/// `data [options] {…}` (`\pgfdata`): the options are read, then the inline data, line by line.
pub(crate) fn data_set(e: &mut Engine) -> Result<(), Error> {
    let o = e.optional()?.unwrap_or_default();
    e.begin_group();
    let r = (|| {
        e.keys_command(data_keys(o.clone()), Some("/"), true)?;
        if let Some(group_name) = key_text(e, "/pgf/data/use group").filter(|s| !s.is_empty()) {
            let name = format!("tikra@dv@group@{group_name}");
            if let Meaning::Macro(m) = e.tex.meaning_of(&name) {
                let code = tex::substitute(&m.body, &[]);
                return e.run_tokens(code);
            }
            e.warn(format!("undefined data group '{group_name}'"));
            return Ok(());
        }
        if key_text(e, "/pgf/data/read from file").is_some_and(|s| !s.is_empty()) {
            e.warn("data [read from file=…]: external data files are not read");
            return Ok(());
        }
        // the lines matter: the ends of lines are read as characters
        match e.tex.next_nonspace() {
            Some(Token::Char(_, Cat::Begin)) => {}
            other => {
                if let Some(o) = other {
                    e.tex.back(o);
                }
                return Err(Error::Figure("data: opening brace expected".into()));
            }
        }
        e.tex.begin_group();
        e.tex.set_catcode('\n', Cat::Other);
        e.tex.set_catcode('\r', Cat::Other);
        let body = e.tex.read_group_body();
        e.tex.end_group();
        let body = body?;
        let mut code = vec![Token::cs("tikra@dv@data")];
        code.extend(group(o));
        code.extend(group(body));
        do_add_data(e, code)
    })();
    e.end_group();
    r
}

/// Data for the data visualization, or for the group being defined.
pub fn do_add_data(e: &mut Engine, code: Vec<Token>) -> Result<(), Error> {
    let mut call = vec![Token::cs("tikra@dv@main")];
    call.extend(tikra_tokens(".add data"));
    call.push(Token::other('('));
    call.extend(group(code));
    call.push(Token::other(')'));
    add_data_call(e, call)
}

/// `\tikra@dv@adddata{call}`: the call adding data is run, or kept in the group being defined.
pub fn add_data_call(e: &mut Engine, call: Vec<Token>) -> Result<(), Error> {
    match key_text(e, "/pgf/data/store in group").filter(|s| !s.is_empty()) {
        Some(name) => {
            let name = format!("tikra@dv@group@{name}");
            let mut old = match e.tex.meaning_of(&name) {
                Meaning::Macro(m) => tex::substitute(&m.body, &[]),
                _ => Vec::new(),
            };
            old.extend(call);
            e.tex.set_meaning(&Token::cs(&name), macro_tokens(old), true);
            Ok(())
        }
        None => e.run_tokens(call),
    }
}

// ---------------------------------------------------------------- phases

pub(crate) fn run_hook(e: &mut Engine, which: fn(&Hooks) -> &Vec<Token>) -> Result<(), Error> {
    let code = which(&dv(e)?.hooks).clone();
    e.run_tokens(code)
}

/// The survey: the data is produced once (or more, if objects ask for it), for the ranges.
pub(crate) fn survey(e: &mut Engine) -> Result<(), Error> {
    dv(e)?.visualizing = false;
    dv(e)?.survey_count = 1;
    loop {
        emit(e, Signal::Phase, Some(Phase::BeforeSurvey))?;
        run_hook(e, |h| &h.before_survey)?;
        emit(e, Signal::Phase, Some(Phase::BeginSurvey))?;
        run_hook(e, |h| &h.at_start_survey)?;
        let data = dv(e)?.data.clone();
        e.run_tokens(data)?;
        run_hook(e, |h| &h.at_end_survey)?;
        emit(e, Signal::Phase, Some(Phase::EndSurvey))?;
        run_hook(e, |h| &h.after_survey)?;
        emit(e, Signal::Phase, Some(Phase::AfterSurvey))?;
        let dv = dv(e)?;
        if dv.survey_count >= dv.number_of_surveys || dv.survey_count > 10 {
            return Ok(());
        }
        dv.survey_count += 1;
    }
}

pub(crate) fn visualize(e: &mut Engine) -> Result<(), Error> {
    dv(e)?.visualizing = true;
    emit(e, Signal::Phase, Some(Phase::BeforeVisualization))?;
    run_hook(e, |h| &h.before_visualization)?;
    emit(e, Signal::Phase, Some(Phase::BeginVisualization))?;
    run_hook(e, |h| &h.at_start_visualization)?;
    let data = dv(e)?.data.clone();
    e.run_tokens(data)?;
    run_hook(e, |h| &h.at_end_visualization)?;
    emit(e, Signal::Phase, Some(Phase::EndVisualization))?;
    run_hook(e, |h| &h.after_visualization)?;
    emit(e, Signal::Phase, Some(Phase::AfterVisualization))
}

/// `\pgfdatapoint`: the signals for the data point now in `/data point/`.
pub(crate) fn data_point(e: &mut Engine) -> Result<(), Error> {
    emit(e, Signal::Prepare, None)?;
    if !dv(e)?.visualizing {
        emit(e, Signal::Map, None)?;
        return emit(e, Signal::Survey, None);
    }
    let p = local_data_point(e)?;
    set_attribute(e, "canvas x", &format!("{}pt", num(p.x)));
    set_attribute(e, "canvas y", &format!("{}pt", num(p.y)));
    if let Some(name) = key_text(e, "/data point/name") {
        e.run_tokens(tikra_tokens(&format!("\\pgfcoordinate{{{name}}}{{{}}}", keys::qpoint(p))))?;
    }
    emit(e, Signal::Visualize, None)?;
    emit(e, Signal::Finish, None)
}

/// Emits a signal: each connected object's slot, in the order they connected.
pub fn emit(e: &mut Engine, signal: Signal, phase: Option<Phase>) -> Result<(), Error> {
    let slots = dv(e)?.signals[signal as usize].clone();
    for (id, slot) in slots {
        crate::objects::slot(e, id, slot, phase)?;
    }
    Ok(())
}

/// `\pgfpointdvlocaldatapoint`: where the current data point is, in pt, with the transformation
/// reset (the picture's applies when it is drawn).
pub fn local_data_point(e: &mut Engine) -> Result<Point, Error> {
    e.begin_group();
    let r = (|| {
        emit(e, Signal::Map, None)?;
        transformed(e)
    })();
    e.end_group();
    r
}

/// The transform signal applied to the origin.
pub(crate) fn transformed(e: &mut Engine) -> Result<Point, Error> {
    let slots = dv(e)?.signals[Signal::Transform as usize].clone();
    let mut p = Point::new(0.0, 0.0);
    for (id, _) in slots {
        p = crate::objects::transform_point(e, id, p);
    }
    Ok(p)
}

/// `\pgfdv@generic`: a path or direction step, which a polar transformer may take over.
pub fn dv_generic(e: &mut Engine, signal: Signal, token: PathToken) -> Result<objects::Step, Error> {
    e.begin_group();
    let r = (|| {
        emit(e, Signal::Map, None)?;
        let slots = dv(e)?.signals[signal as usize].clone();
        let mut handled = None;
        for (id, _) in slots {
            if let Some(s) = crate::objects::path_slot(e, id, token, handled.is_some())? {
                handled = Some(s);
            }
        }
        match handled {
            Some(s) => Ok(s),
            None => Ok(objects::Step::Point(transformed(e)?)),
        }
    })();
    e.end_group();
    r
}

/// `.method(args)`: the method's name and its arguments, split at commas.
pub(crate) fn read_method(e: &mut Engine) -> Result<(String, Vec<Vec<Token>>), Error> {
    match e.tex.next_nonspace() {
        Some(t) if t.is_char('.') => {}
        Some(t) => {
            e.tex.back(t);
            return Ok((String::new(), Vec::new()));
        }
        None => return Ok((String::new(), Vec::new())),
    }
    let mut name = String::new();
    while let Some(t) = e.tex.get_token() {
        if t.is_char('(') {
            break;
        }
        name.push_str(&tex::tokens_to_string(&[t]));
        if name.len() > 100 {
            return Err(Error::Figure("data visualization: method call without '('".into()));
        }
    }
    let inner = e.tex.read_until_char(')', true)?;
    let mut args = Vec::new();
    let mut current = Vec::new();
    let mut depth = 0usize;
    for t in inner {
        match t {
            Token::Char(_, Cat::Begin) => depth += 1,
            Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
            Token::Char(',', _) if depth == 0 => {
                args.push(tex::strip_outer_braces(tikra_engine::keys::trim(&current).to_vec()));
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(t);
    }
    args.push(tex::strip_outer_braces(tikra_engine::keys::trim(&current).to_vec()));
    Ok((name.trim().to_string(), args))
}

/// The object whose handle a key holds (`\tikra@dv@obj{n}`).
pub fn object_of(e: &mut Engine, key: &str) -> Option<usize> {
    let v = tikra_engine::keys::value(&e.tex, key)?;
    handle_id(&v)
}

/// The object a macro holds.
pub fn object_in(e: &mut Engine, name: &str) -> Option<usize> {
    match e.tex.meaning_of(name) {
        Meaning::Macro(m) => handle_id(&tex::substitute(&m.body, &[])),
        _ => None,
    }
}

// ---------------------------------------------------------------- bounding boxes

/// The nodes `data visualization bounding box` (what was drawn so far) and, when `freeze`, `data
/// bounding box` (frozen after the data, before the axes).
pub(crate) fn update_bounding_box(e: &mut Engine, freeze: bool) {
    // nothing drawn yet: a box of no size at the origin
    let origin = e.st.transform.apply(Point::new(0.0, 0.0));
    let (lo, hi) = match (e.canvas.min, e.canvas.max) {
        (Some(lo), Some(hi)) => (lo, hi),
        _ => (origin, origin),
    };
    let c = Point::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
    let mut g = tikra_engine::engine::shapes::Geometry::new(c, tikra_engine::engine::shapes::Border::Rectangle(Point::new((hi.x - lo.x) / 2.0, (hi.y - lo.y) / 2.0)));
    for (name, x, y) in [
        ("north", c.x, hi.y),
        ("south", c.x, lo.y),
        ("east", hi.x, c.y),
        ("west", lo.x, c.y),
        ("north east", hi.x, hi.y),
        ("north west", lo.x, hi.y),
        ("south east", hi.x, lo.y),
        ("south west", lo.x, lo.y),
    ] {
        g.anchor_at(name, Point::new(x, y));
    }
    let info = tikra_engine::engine::nodes::NodeInfo { shape: "rectangle".into(), geometry: std::rc::Rc::new(g), transform: tikra_engine::scene::Transform::default() };
    if freeze {
        e.nodes.insert("data bounding box".into(), info.clone());
    }
    e.nodes.insert("data visualization bounding box".into(), info);
}

/// `(visualization cs:x=1,y=2)`: where that data point would be.
pub fn visualization_cs(e: &mut Engine, args: &str) -> Result<Point, Error> {
    if crate::running(e).is_empty() {
        return Err(Error::Figure("visualization cs outside a data visualization".into()));
    }
    e.begin_group();
    let r = (|| {
        e.keys_command(tikra_tokens(&format!("/data point/.cd,{args}")), Some("/"), true)?;
        local_data_point(e)
    })();
    e.end_group();
    let p = r?;
    Ok(e.st.transform.apply(Point::new(p.x * PT, p.y * PT)))
}

/// `/pgf/data/.cd,/pgf/every data/.try,options`
fn data_keys(o: Vec<Token>) -> Vec<Token> {
    let mut v = tikra_tokens("/pgf/data/.cd,/pgf/every data/.try,");
    v.extend(o);
    v
}

/// The id in `\tikra@dv@obj{n}`.
fn handle_id(v: &[Token]) -> Option<usize> {
    let i = v.iter().position(|t| t.is_cs("tikra@dv@obj"))?;
    let s = tex::tokens_to_string(&v[i + 1..]);
    s.trim().trim_start_matches('{').split('}').next()?.trim().parse().ok()
}

/// `\tikra@dv@obj{n}` as tokens.
pub fn handle(id: usize) -> Vec<Token> {
    let mut v = vec![Token::cs("tikra@dv@obj")];
    v.extend(group(tex::string_tokens(&id.to_string())));
    v
}

/// Items at commas outside braces and parentheses (`(-2em,0),(0,0)`), as `\foreach` reads points.
pub fn point_split(v: &[Token]) -> Vec<Vec<Token>> {
    let mut out = vec![Vec::new()];
    let mut depth = 0i32;
    for t in v {
        match t {
            Token::Char(_, Cat::Begin) | Token::Char('(' | '[', _) => depth += 1,
            Token::Char(_, Cat::End) | Token::Char(')' | ']', _) => depth -= 1,
            Token::Char(',', _) if depth == 0 => {
                out.push(Vec::new());
                continue;
            }
            _ => {}
        }
        if let Some(l) = out.last_mut() {
            l.push(t.clone());
        }
    }
    out
}

/// Items at top-level commas (a list of handlers).
pub fn axes_split(v: &[Token]) -> Vec<Vec<Token>> {
    let mut out = vec![Vec::new()];
    let mut depth = 0usize;
    for t in v {
        match t {
            Token::Char(_, Cat::Begin) => depth += 1,
            Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
            Token::Char(',', _) if depth == 0 => {
                out.push(Vec::new());
                continue;
            }
            _ => {}
        }
        if let Some(l) = out.last_mut() {
            l.push(t.clone());
        }
    }
    out
}
