//! The objects of a data visualization (pgfmoduledatavisualization.code.tex and the polar library):
//! what each class keeps, its methods, and the slots its signals call.

use tikra_engine::keys::macro_name;
use tikra_engine::scene::Point;
use tikra_engine::tex::{self, Meaning, Token};
use tikra_engine::{Error, PT};

use super::{attribute, group, handle, key_text, num, parse_value, set_attribute, PathToken, Phase, Signal, Slot};
use tikra_engine::engine::{macro_from, macro_tokens, tikra_tokens, Engine};

/// The deforming function of a scaling (`function=`): none, `\pgfdvmathln` (logarithmic axes), or a
/// pgfmath function by name (`\pgfdvmathunaryop{…}{sqrt}{…}`).
#[derive(Debug, Clone, PartialEq)]
pub enum Function {
    Identity,
    Ln,
    Op(String),
}

impl Function {
    fn of(text: &str) -> Function {
        if text.trim().is_empty() {
            Function::Identity
        } else if text.contains("pgfdvmathln") {
            Function::Ln
        } else if let Some(i) = text.find("pgfdvmathunaryop") {
            let rest = &text[i..];
            let name = rest.split('{').nth(2).and_then(|s| s.split('}').next()).unwrap_or("").trim().to_string();
            Function::Op(name)
        } else {
            Function::Identity
        }
    }

    fn apply(&self, e: &mut Engine, v: f64) -> Option<f64> {
        let r = match self {
            Function::Identity => v,
            Function::Ln => v.ln(),
            Function::Op(name) => e.number(&format!("{name}({})", num(v))).ok()?,
        };
        r.is_finite().then_some(r)
    }
}

/// How an interval mapper scales, once set up at the end of the survey.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scale {
    Unset,
    /// the in interval is a single value: everything maps to the middle of the out interval
    None,
    By(f64),
}

/// A scaling mapper, with its range surveyor, its in range interval and its interval mapper.
#[derive(Debug, Clone)]
pub struct Mapper {
    pub input: String,
    pub output: String,
    pub spec: String,
    pub function: Function,
    pub range: usize,
    pub scale: Scale,
    pub trans_in_min: f64,
    pub out_min: f64,
    pub out_max: f64,
    pub maps_min: bool,
    pub maps_max: bool,
}

#[derive(Debug, Clone)]
pub enum Object {
    /// the data visualization itself
    Main,
    Interval {
        min: Option<f64>,
        max: Option<f64>,
    },
    ScalingMapper(Box<Mapper>),
    LinearTransformer {
        attribute: String,
        origin: Point,
        unit: Point,
    },
    PolarTransformer {
        angle: String,
        radius: String,
        u0: Point,
        u90: Point,
        prev_angle: Option<f64>,
        prev_radius: Option<f64>,
    },
    PlotVisualizer {
        name: String,
        handlers: Vec<Token>,
        filter: Option<Vec<Token>>,
        projection: Vec<Token>,
        /// canvas positions in pt; None where an outlier breaks the line
        positions: Vec<Option<Point>>,
    },
    RectangleVisualizer {
        name: String,
        a1: String,
        a2: String,
    },
    LabelVisualizer {
        attribute: String,
        threshold: Option<f64>,
        code: Vec<Token>,
        filter: Vec<Token>,
        before: Option<Point>,
        at: Option<Point>,
    },
    StyleAttribute {
        attribute: String,
        sheet: String,
    },
    Legend {
        columns: bool,
        ideal: usize,
        max: usize,
        strategy: String,
        entries: Vec<Vec<Token>>,
    },
    Count {
        attribute: String,
        val: f64,
        max: f64,
        step: f64,
        start: f64,
        filter: Vec<Token>,
    },
    Accumulator {
        attribute: String,
        sum: f64,
    },
}

/// What a path step becomes: a point, or an arc that a polar transformer draws instead of a line.
#[derive(Debug, Clone, Copy)]
pub enum Step {
    Point(Point),
    /// from angle to angle, on the ellipse with these axes (pt)
    Arc {
        from: f64,
        to: f64,
        a: Point,
        b: Point,
    },
    /// a direction a polar transformer computed (normalized, pt)
    Direction(Point),
}

/// `\pgfqpoint{1pt}{0pt}` (or any two lengths in braces) → the point in pt.
fn point_of(e: &mut Engine, text: &str) -> Point {
    let parts: Vec<String> = text.split('{').skip(1).map(|s| s.split('}').next().unwrap_or("").to_string()).collect();
    if parts.len() < 2 {
        return Point::new(0.0, 0.0);
    }
    let x = e.length(&parts[0]).unwrap_or(0.0) / PT;
    let y = e.length(&parts[1]).unwrap_or(0.0) / PT;
    Point::new(x, y)
}

pub(crate) fn object(e: &mut Engine, id: usize) -> Option<&mut Object> {
    crate::running(e).last_mut()?.objects.get_mut(id)
}

pub(crate) fn add_object(e: &mut Engine, o: Object) -> Result<usize, Error> {
    let dv = crate::dv(e)?;
    dv.objects.push(o);
    Ok(dv.objects.len() - 1)
}

/// `new object={class=…, store=…, arg1=…, after creation=…}`, run at its time: the object is
/// created unless the key `store` already holds one.
pub fn new_object(e: &mut Engine, store: &str, spec: Vec<Token>) -> Result<(), Error> {
    if !store.is_empty() && key_text(e, store).is_some_and(|s| !s.is_empty()) {
        return Ok(());
    }
    let mut class = String::new();
    let mut args: Vec<Option<Vec<Token>>> = vec![None; 8];
    let mut after = Vec::new();
    for item in tikra_engine::keys::split_list(&spec) {
        let value = item.value.unwrap_or_default();
        let (key, handler) = match item.key.split_once("/.") {
            Some((k, h)) => (k.trim().to_string(), h.trim().to_string()),
            None => (item.key.trim().to_string(), String::new()),
        };
        let value = if handler.is_empty() { value } else { e.tex.expand_fully(value)? };
        match key.as_str() {
            "class" => class = e.tex.expand_to_string(value)?.trim().to_string(),
            "before creation" => e.run_tokens(value)?,
            "after creation" => after = value,
            k => {
                let Some(rest) = k.strip_prefix("arg") else { continue };
                let (n, from_key) = match rest.split_once(' ') {
                    Some((n, r)) => (n, r.trim() == "from key"),
                    None => (rest, false),
                };
                let Some(i) = n.parse::<usize>().ok().filter(|i| (1..=8).contains(i)) else { continue };
                args[i - 1] = Some(if from_key {
                    let key = e.tex.expand_to_string(value)?;
                    tikra_engine::keys::value(&e.tex, key.trim()).unwrap_or_default()
                } else {
                    value
                });
            }
        }
    }
    let args: Vec<Vec<Token>> = args.into_iter().map_while(|a| a).collect();
    let Some(id) = create(e, &class, &args)? else { return Ok(()) };
    if !store.is_empty() {
        e.tex.set_meaning(&Token::cs(&macro_name(store)), macro_tokens(handle(id)), false);
    }
    e.tex.set_meaning(&Token::cs("tikzdvobj"), macro_tokens(handle(id)), false);
    e.run_tokens(after)
}

/// An object of a class, from its constructor's arguments, connected to its signals.
pub fn create(e: &mut Engine, class: &str, args: &[Vec<Token>]) -> Result<Option<usize>, Error> {
    let text = |e: &mut Engine, i: usize| -> String { args.get(i).map(|v| e.tex.expand_to_string(v.clone()).unwrap_or_default().trim().to_string()).unwrap_or_default() };
    let id = match class {
        "scaling mapper" => {
            let range = add_object(e, Object::Interval { min: None, max: None })?;
            let m = Mapper {
                input: text(e, 0),
                output: text(e, 1),
                spec: text(e, 2),
                function: Function::of(&text(e, 3)),
                range,
                scale: Scale::Unset,
                trans_in_min: 0.0,
                out_min: 0.0,
                out_max: 0.0,
                maps_min: false,
                maps_max: false,
            };
            let id = add_object(e, Object::ScalingMapper(Box::new(m)))?;
            let dv = crate::dv(e)?;
            dv.connect(id, Slot::Survey, Signal::Survey);
            dv.connect(id, Slot::Map, Signal::Map);
            dv.connect(id, Slot::Phase, Signal::Phase);
            id
        }
        "linear transformer" => {
            let attribute = text(e, 0);
            let unit = text(e, 1);
            let unit = point_of(e, &unit);
            let id = add_object(e, Object::LinearTransformer { attribute, origin: Point::new(0.0, 0.0), unit })?;
            crate::dv(e)?.connect(id, Slot::Transform, Signal::Transform);
            id
        }
        "polar transformer" => {
            let angle = text(e, 0);
            let radius = text(e, 1);
            let (a, b) = (text(e, 2), text(e, 3));
            let (u0, u90) = (point_of(e, &a), point_of(e, &b));
            let id = add_object(e, Object::PolarTransformer { angle, radius, u0, u90, prev_angle: None, prev_radius: None })?;
            let dv = crate::dv(e)?;
            dv.connect(id, Slot::Transform, Signal::Transform);
            dv.connect(id, Slot::Path, Signal::Path);
            dv.connect(id, Slot::Direction, Signal::Direction);
            id
        }
        "plot handler visualizer" => {
            let name = text(e, 0);
            let handlers = args.get(1).cloned().unwrap_or_default();
            let id = add_object(e, Object::PlotVisualizer { name, handlers, filter: None, projection: Vec::new(), positions: Vec::new() })?;
            let dv = crate::dv(e)?;
            dv.connect(id, Slot::Protocol, Signal::Visualize);
            dv.connect(id, Slot::Phase, Signal::Phase);
            id
        }
        "rectangle visualizer" => {
            let name = text(e, 0);
            let id = add_object(e, Object::RectangleVisualizer { name, a1: String::new(), a2: String::new() })?;
            let dv = crate::dv(e)?;
            dv.connect(id, Slot::Visualize, Signal::Visualize);
            dv.connect(id, Slot::Phase, Signal::Phase);
            id
        }
        "label visualizer" => {
            let attribute = text(e, 0);
            let threshold = text(e, 1);
            let threshold = parse_value(e, &threshold);
            let code = args.get(2).cloned().unwrap_or_default();
            let filter = args.get(3).cloned().unwrap_or_default();
            let id = add_object(e, Object::LabelVisualizer { attribute, threshold, code, filter, before: None, at: None })?;
            let dv = crate::dv(e)?;
            dv.connect(id, Slot::Test, Signal::Visualize);
            dv.connect(id, Slot::Phase, Signal::Phase);
            id
        }
        "count" => {
            let attribute = text(e, 0);
            let id = add_object(e, Object::Count { attribute, val: 0.0, max: 0.0, step: 1.0, start: 0.0, filter: Vec::new() })?;
            let dv = crate::dv(e)?;
            dv.connect(id, Slot::Apply, Signal::Prepare);
            dv.connect(id, Slot::Phase, Signal::Phase);
            id
        }
        "accumulator" => {
            let attribute = text(e, 0);
            let id = add_object(e, Object::Accumulator { attribute: attribute.clone(), sum: 0.0 })?;
            set_attribute(e, &format!("{attribute}/sum"), "0");
            set_attribute(e, &format!("{attribute}/prev sum"), "0");
            let dv = crate::dv(e)?;
            dv.connect(id, Slot::Apply, Signal::Prepare);
            dv.connect(id, Slot::Phase, Signal::Phase);
            id
        }
        "legend" => {
            let columns = text(e, 0) == "columns";
            let ideal = text(e, 1).parse().unwrap_or(1);
            let max = text(e, 2).parse().unwrap_or(100);
            let strategy = text(e, 3);
            add_object(e, Object::Legend { columns, ideal, max, strategy, entries: Vec::new() })?
        }
        "interval" => add_object(e, Object::Interval { min: None, max: None })?,
        "" => return Ok(None),
        other => {
            e.warn(format!("data visualization: class '{other}' not supported"));
            return Ok(None);
        }
    };
    Ok(Some(id))
}

/// `before survey`, `at end visualization` and the other phase hooks: code stored on the visualization.
fn phase_hook(e: &mut Engine, method: &str, code: Vec<Token>) -> Result<(), Error> {
    let h = &mut crate::dv(e)?.hooks;
    match method {
        "before survey" => h.before_survey.extend(code),
        "at start survey" => h.at_start_survey.extend(code),
        "before visualization" => h.before_visualization.extend(code),
        "at start visualization" => h.at_start_visualization.extend(code),
        "at end survey" => h.at_end_survey = [code, std::mem::take(&mut h.at_end_survey)].concat(),
        "after survey" => h.after_survey = [code, std::mem::take(&mut h.after_survey)].concat(),
        "at end visualization" => h.at_end_visualization = [code, std::mem::take(&mut h.at_end_visualization)].concat(),
        "after visualization" => h.after_visualization = [code, std::mem::take(&mut h.after_visualization)].concat(),
        _ => {}
    }
    Ok(())
}

/// `set min`, `set max`, `let min`, `let max` on an interval.
fn set_interval_end(e: &mut Engine, id: usize, method: &str, text: &str) {
    let v = parse_value(e, text);
    if let Some(Object::Interval { min, max }) = object(e, id) {
        if method.ends_with("min") {
            *min = v;
        } else {
            *max = v;
        }
    }
}

/// What `set filter`, `set value`, `set start value` and `set step` write on a count.
struct CountSet<'a> {
    method: &'a str,
    value: f64,
    code: Vec<Token>,
}

fn set_count(e: &mut Engine, id: usize, set: CountSet) {
    let CountSet { method, value, code } = set;
    if let Some(Object::Count { filter, val, start, step, max, attribute }) = object(e, id) {
        match method {
            "set filter" => *filter = code,
            "set value" => *val = value,
            "set start value" => {
                *start = value;
                *max = value;
            }
            _ => *step = value,
        }
        let _ = attribute;
    }
}

/// `set filter` and `set projection code` on a plot visualizer.
fn set_plot_code(e: &mut Engine, id: usize, method: &str, code: Vec<Token>) {
    if let Some(Object::PlotVisualizer { filter, projection, .. }) = object(e, id) {
        if method == "set filter" {
            *filter = Some(code);
        } else {
            *projection = code;
        }
    }
}

/// A method call on object `id`.
pub fn call(e: &mut Engine, id: usize, method: &str, args: Vec<Vec<Token>>) -> Result<(), Error> {
    let arg = |i: usize| args.get(i).cloned().unwrap_or_default();
    let target = |v: &[Token]| v.iter().find(|t| !t.is_space()).cloned();
    let Some(o) = object(e, id).cloned() else {
        e.warn(format!("data visualization: no object {id} for .{method}()"));
        return Ok(());
    };
    match (o, method) {
        (_, "default connects") => {}
        (Object::Main, "add data") => {
            let code = tex::strip_outer_braces(tikra_engine::keys::trim(&arg(0)).to_vec());
            crate::dv(e)?.data.extend(code);
        }
        (Object::Main, m) if m.starts_with("before ") || m.starts_with("after ") || m.starts_with("at ") => phase_hook(e, m, arg(0))?,
        (Object::Main, "get survey count") => {
            let n = crate::dv(e)?.survey_count;
            if let Some(t) = target(&arg(0)) {
                e.tex.set_meaning(&t, macro_from(&n.to_string()), false);
            }
        }
        (Object::Main, "do an additional survey") => crate::dv(e)?.number_of_surveys += 1,
        (Object::Main, _) => {}
        (Object::ScalingMapper(m), "get in range interval") => {
            e.tex.set_meaning(&Token::cs("pgfdvinrangeinterval"), macro_tokens(handle(m.range)), false);
        }
        (Object::ScalingMapper(m), "get out") => e.tex.define_string("pgfdvout", &m.output, false),
        (Object::ScalingMapper(m), "get function") => {
            if let Some(t) = target(&arg(0)) {
                let v = if m.function == Function::Identity { String::new() } else { "\\pgfdvmathln".into() };
                e.tex.set_meaning(&t, macro_from(&v), false);
            }
        }
        (Object::ScalingMapper(_), "include in value") => {
            let s = e.tex.expand_to_string(arg(0))?;
            if let Some(v) = parse_value(e, &s) {
                include_value(e, id, v);
            }
        }
        (Object::ScalingMapper(_), "set in to") => {
            let s = e.tex.expand_to_string(arg(0))?;
            set_in_to(e, id, &s);
        }
        (Object::Interval { min, max }, "get min and max") => {
            e.tex.define_string("pgfdvmin", &min.map(num).unwrap_or_default(), false);
            e.tex.define_string("pgfdvmax", &max.map(num).unwrap_or_default(), false);
        }
        (Object::Interval { .. }, m @ ("set min" | "set max" | "let min" | "let max")) => {
            let s = e.tex.expand_to_string(arg(0))?;
            set_interval_end(e, id, m, &s);
        }
        (Object::Count { max, .. }, "get max") => {
            if let Some(t) = target(&arg(0)) {
                e.tex.set_meaning(&t, macro_from(&num(max)), false);
            }
        }
        (Object::Count { .. }, m @ ("set filter" | "set value" | "set start value" | "set step")) => {
            let code = arg(0);
            let value = parse_value(e, &tex::tokens_to_string(&code)).unwrap_or(0.0);
            set_count(e, id, CountSet { method: m, value, code });
        }
        (Object::Legend { .. }, "add entry") => {
            if let Some(Object::Legend { entries, .. }) = object(e, id) {
                entries.push(arg(0));
            }
        }
        (Object::Legend { .. }, "get arrangement") => {
            let v = crate::legend::legend_arrangement(e, id);
            if let Some(t) = target(&arg(0)) {
                e.tex.set_meaning(&t, macro_tokens(v), false);
            }
        }
        (Object::PlotVisualizer { .. }, m @ ("set filter" | "set projection code")) => set_plot_code(e, id, m, arg(0)),
        (Object::LinearTransformer { .. }, "set origin") => {
            let s = e.tex.expand_to_string(arg(0))?;
            let p = point_of(e, &s);
            if let Some(Object::LinearTransformer { origin, .. }) = object(e, id) {
                *origin = p;
            }
        }
        (Object::PolarTransformer { angle, radius, .. }, m @ ("get angle attribute" | "get radius attribute")) => {
            if let Some(t) = target(&arg(0)) {
                e.tex.set_meaning(&t, macro_from(if m.contains("angle") { &angle } else { &radius }), false);
            }
        }
        (Object::Accumulator { attribute, .. }, "reset") => reset_accumulator(e, id, &attribute),
        (_, m) => e.warn(format!("data visualization: method '{m}' not supported")),
    }
    Ok(())
}

// ---------------------------------------------------------------- scaling mappers

pub(crate) fn mapper(e: &mut Engine, id: usize) -> Option<Mapper> {
    match object(e, id)? {
        Object::ScalingMapper(m) => Some((**m).clone()),
        _ => None,
    }
}

/// The range an axis' survey found.
pub fn range_of(e: &mut Engine, id: usize) -> (Option<f64>, Option<f64>) {
    let Some(m) = mapper(e, id) else { return (None, None) };
    match object(e, m.range) {
        Some(Object::Interval { min, max }) => (*min, *max),
        _ => (None, None),
    }
}

pub fn include_value(e: &mut Engine, id: usize, v: f64) {
    let Some(m) = mapper(e, id) else { return };
    if let Some(Object::Interval { min, max }) = object(e, m.range) {
        *min = Some(min.map_or(v, |x| x.min(v)));
        *max = Some(max.map_or(v, |x| x.max(v)));
    }
}

/// `min value=…`, `max value=…`: the end of the range, set.
pub fn set_range(e: &mut Engine, id: usize, which: &str, v: f64) {
    let Some(m) = mapper(e, id) else { return };
    if let Some(Object::Interval { min, max }) = object(e, m.range) {
        if which == "min" {
            *min = Some(v);
        } else {
            *max = Some(v);
        }
    }
}

/// `set in to(value)`: the axis' attribute set to a value, or to the range's `min` or `max`.
pub fn set_in_to(e: &mut Engine, id: usize, value: &str) {
    let Some(m) = mapper(e, id) else { return };
    let (lo, hi) = range_of(e, id);
    let v = match value.trim() {
        "min" => {
            if m.maps_min {
                set_attribute(e, &format!("{}/is min or max", m.input), "min");
            }
            lo
        }
        "max" => {
            // pgf tests `spec maps min` here too
            if m.maps_min {
                set_attribute(e, &format!("{}/is min or max", m.input), "max");
            }
            hi
        }
        other => parse_value(e, other),
    };
    set_attribute(e, &m.input, &v.map(num).unwrap_or_default());
}

/// The end of the survey: the interval mapper is set up from the range and the scaling.
pub(crate) fn setup_mapper(e: &mut Engine, id: usize) {
    let Some(m) = mapper(e, id) else { return };
    let (Some(lo), Some(hi)) = range_of(e, id) else { return };
    // #1 at#2and #3 at#4
    let spec = m.spec.clone();
    let (first, second) = spec.split_once(" and ").unwrap_or((&spec, ""));
    let (a, c) = first.split_once(" at").unwrap_or((first, "0"));
    let (b, d) = second.split_once(" at").unwrap_or((second, "1"));
    let value = |e: &mut Engine, s: &str| parse_value(e, s).unwrap_or(0.0);
    let (maps_min, a) = if a.trim() == "min" { (true, lo) } else { (false, value(e, a)) };
    let (maps_max, b) = if b.trim() == "max" { (true, hi) } else { (false, value(e, b)) };
    let (c, d) = (value(e, c), value(e, d));
    let f = m.function.clone();
    let (fa, fb) = (f.apply(e, a), f.apply(e, b));
    let (scale, trans) = match (a < b, fa, fb) {
        (true, Some(fa), Some(fb)) if fb != fa => (Scale::By((d - c) / (fb - fa)), fa),
        (true, _, _) => (Scale::Unset, 0.0),
        (false, _, _) => (Scale::None, 0.0),
    };
    if let Some(Object::ScalingMapper(m)) = object(e, id) {
        m.out_min = c;
        m.out_max = d;
        m.maps_min = maps_min;
        m.maps_max = maps_max;
        m.scale = scale;
        m.trans_in_min = trans;
    }
}

/// The map slot: the in attribute, scaled, into the out attribute.
pub(crate) fn map_slot(e: &mut Engine, id: usize) {
    let Some(m) = mapper(e, id) else { return };
    if m.scale == Scale::Unset {
        return;
    }
    let offset = attribute(e, &format!("{}/offset", m.output)).and_then(|s| parse_value(e, &s)).unwrap_or(0.0);
    let is = attribute(e, &format!("{}/is min or max", m.input)).unwrap_or_default();
    let out = if is == "min" || is == "max" {
        set_attribute(e, &format!("{}/is min or max", m.output), &is);
        Some(if is == "min" { m.out_min } else { m.out_max } + offset)
    } else if m.scale == Scale::None {
        Some((m.out_min + m.out_max) / 2.0)
    } else {
        let v = attribute(e, &format!("{}/const", m.input)).or_else(|| attribute(e, &m.input));
        match v {
            Some(v) => {
                let Scale::By(scale) = m.scale else { return };
                let x = parse_value(e, &v);
                x.and_then(|x| m.function.apply(e, x)).map(|fx| (fx - m.trans_in_min) * scale + m.out_min + offset)
            }
            None => match attribute(e, &format!("{}/out pos", m.input)) {
                Some(p) => {
                    let p = parse_value(e, &p).unwrap_or(0.0);
                    Some((1.0 - p) * m.out_min + p * m.out_max)
                }
                None => None,
            },
        }
    };
    if let Some(v) = out {
        set_attribute(e, &m.output, &num(v));
    }
}

/// The survey slot: the attribute (and its sub attributes) widen the range.
pub(crate) fn survey_slot(e: &mut Engine, id: usize) {
    let Some(m) = mapper(e, id) else { return };
    let mut subs = vec![String::new()];
    if let Some(s) = key_text(e, &format!("/data point/{}/sub attributes", m.input)) {
        for part in s.split("\\pgfdvsub").skip(1) {
            subs.push(part.trim().trim_start_matches('{').split('}').next().unwrap_or("").to_string());
        }
    }
    for sub in subs {
        if let Some(v) = attribute(e, &format!("{}{sub}", m.input)) {
            if let Some(x) = parse_value(e, &v) {
                include_value(e, id, x);
            }
        }
    }
}

// ---------------------------------------------------------------- transformers

/// The transform slot of a transformer: `p` shifted for the current data point.
pub fn transform_point(e: &mut Engine, id: usize, p: Point) -> Point {
    match object(e, id).cloned() {
        Some(Object::LinearTransformer { attribute: a, origin, unit }) => {
            let Some(v) = attribute(e, &a).and_then(|s| parse_value(e, &s)) else { return p };
            let p = p.plus(origin);
            // values below 10^-3 are not shifted by (pgf's TeX dimensions would overflow)
            if v != 0.0 && v.abs() < 1e-3 {
                return p;
            }
            p.plus(unit.scale(v))
        }
        Some(Object::PolarTransformer { angle, radius, u0, u90, .. }) => {
            let get = |e: &mut Engine, a: &str| attribute(e, &format!("{a}/const")).or_else(|| attribute(e, a)).and_then(|s| parse_value(e, &s));
            let (Some(a), Some(r)) = (get(e, &angle), get(e, &radius)) else { return p };
            let (s, c) = a.to_radians().sin_cos();
            p.plus(u0.scale(c * r)).plus(u90.scale(s * r))
        }
        _ => p,
    }
}

/// The path and direction slots of a polar transformer; Some when it handles the step.
pub fn path_slot(e: &mut Engine, id: usize, token: PathToken, handled: bool) -> Result<Option<Step>, Error> {
    let Some(Object::PolarTransformer { angle, radius, u0, u90, prev_angle, prev_radius }) = object(e, id).cloned() else { return Ok(None) };
    let a = attribute(e, &angle).and_then(|s| parse_value(e, &s));
    let r = attribute(e, &radius).and_then(|s| parse_value(e, &s));
    let same = |x: Option<f64>, y: Option<f64>| match (x, y) {
        (Some(x), Some(y)) => (x - y).abs() <= 1e-9 * x.abs().max(y.abs()).max(1.0),
        _ => false,
    };
    let mut step = None;
    match token {
        PathToken::MoveTo | PathToken::LineTo => {
            if !handled && token == PathToken::LineTo && same(r, prev_radius) && !same(a, prev_angle) {
                if let (Some(from), Some(to), Some(r)) = (prev_angle, a, r) {
                    step = Some(Step::Arc { from, to, a: u0.scale(r), b: u90.scale(r) });
                }
            }
            if let Some(Object::PolarTransformer { prev_angle, prev_radius, .. }) = object(e, id) {
                *prev_angle = a;
                *prev_radius = r;
            }
        }
        PathToken::DirectionFrom => crate::dv(e)?.polar_from = (a, r),
        PathToken::DirectionTo => crate::dv(e)?.polar_to = (a, r),
        PathToken::DirectionAt => {
            let (from, to) = (crate::dv(e)?.polar_from, crate::dv(e)?.polar_to);
            if same(from.1, to.1) {
                if let Some(a) = a {
                    let sign = if from.0.unwrap_or(0.0) < to.0.unwrap_or(0.0) { 90.0 } else { -90.0 };
                    let (s, c) = (a + sign).to_radians().sin_cos();
                    step = Some(Step::Direction(Point::new(c, s)));
                }
            }
        }
    }
    Ok(step)
}

// ---------------------------------------------------------------- slots

/// Runs filter code (`\pgfdvfilterpassedtrue`, the code, then `\ifpgfdvfilterpassed`).
pub(crate) fn filter_passes(e: &mut Engine, code: &[Token]) -> Result<bool, Error> {
    if code.is_empty() {
        return Ok(true);
    }
    e.tex.set_meaning(&Token::cs("ifpgfdvfilterpassed"), Meaning::Prim(tex::Prim::Iftrue), false);
    e.run_tokens(code.to_vec())?;
    Ok(matches!(e.tex.meaning_of("ifpgfdvfilterpassed"), Meaning::Prim(tex::Prim::Iftrue)))
}

/// What a plot visualizer saw of this data point: its name, its filter and its projection code.
struct PlotSeen {
    name: String,
    filter: Option<Vec<Token>>,
    projection: Vec<Token>,
}

/// The plot visualizer's protocol: keep the canvas position when the filter passes.
fn plot_protocol(e: &mut Engine, id: usize, seen: PlotSeen) -> Result<(), Error> {
    let PlotSeen { name, filter, projection } = seen;
    let passes = match filter {
        Some(code) => filter_passes(e, &code)?,
        None => attribute(e, "set").unwrap_or_default() == name,
    };
    if passes {
        let p = if attribute(e, "outlier").is_some() {
            None
        } else {
            e.begin_group();
            let r = e.run_tokens(projection);
            let x = attribute(e, "canvas x").map(|s| e.length(&s).unwrap_or(0.0) / PT);
            let y = attribute(e, "canvas y").map(|s| e.length(&s).unwrap_or(0.0) / PT);
            e.end_group();
            r?;
            Some(Point::new(x.unwrap_or(0.0), y.unwrap_or(0.0)))
        };
        if let Some(Object::PlotVisualizer { positions, .. }) = object(e, id) {
            positions.push(p);
        }
    }
    Ok(())
}

/// At the start of the survey, the rectangle's two attributes and their min and max.
fn rectangle_attributes(e: &mut Engine, id: usize, name: &str) {
    let a1 = format!("/data point/{}", key_text(e, &format!("/data point/{name}/attribute 1")).unwrap_or_default());
    let a2 = format!("/data point/{}", key_text(e, &format!("/data point/{name}/attribute 2")).unwrap_or_default());
    for a in [&a1, &a2] {
        e.tex.set_meaning(&Token::cs(&macro_name(&format!("{a}/sub attributes"))), macro_tokens(tikra_tokens("\\pgfdvsub{/min}\\pgfdvsub{/max}")), false);
    }
    if let Some(Object::RectangleVisualizer { a1: x, a2: y, .. }) = object(e, id) {
        *x = a1;
        *y = a2;
    }
}

/// What a label visualizer tests a data point with.
struct LabelSeen {
    attribute: String,
    threshold: Option<f64>,
    filter: Vec<Token>,
    at: Option<Point>,
}

fn label_test(e: &mut Engine, id: usize, seen: LabelSeen) -> Result<(), Error> {
    let LabelSeen { attribute: a, threshold, filter, at } = seen;
    if !a.is_empty() && filter_passes(e, &filter)? {
        let x = attribute(e, "canvas x").map(|s| e.length(&s).unwrap_or(0.0) / PT).unwrap_or(0.0);
        let y = attribute(e, "canvas y").map(|s| e.length(&s).unwrap_or(0.0) / PT).unwrap_or(0.0);
        let mut stop = false;
        if let Some(t) = threshold {
            if let Some(v) = attribute(e, &a).and_then(|s| parse_value(e, &s)) {
                stop = v >= t;
            }
        }
        if let Some(Object::LabelVisualizer { before, at: here, attribute, .. }) = object(e, id) {
            *before = at;
            *here = Some(Point::new(x, y));
            if stop {
                attribute.clear();
            }
        }
    }
    Ok(())
}

/// The label visualizer at the end of the visualization: the two coordinates and its code.
struct LabelEnd {
    code: Vec<Token>,
    before: Option<Point>,
    at: Option<Point>,
}

fn label_phase(e: &mut Engine, phase: Option<Phase>, end: LabelEnd) -> Result<(), Error> {
    if phase == Some(Phase::EndVisualization) {
        let LabelEnd { code, before, at } = end;
        let (Some(at), before) = (at, before) else { return Ok(()) };
        let before = before.unwrap_or(at);
        let mut v = tikra_tokens("\\begin{scope}");
        v.extend(tikra_tokens(&format!("\\pgfcoordinate{{label visualizer coordinate}}{{{}}}", super::keys::qpoint(at))));
        v.extend(tikra_tokens(&format!("\\pgfcoordinate{{label visualizer coordinate'}}{{{}}}", super::keys::qpoint(before))));
        v.extend(code);
        v.extend(tikra_tokens("\\end{scope}"));
        e.run_tokens(v)?;
    }
    Ok(())
}

/// A count stepping on one data point.
struct CountSeen {
    attribute: String,
    val: f64,
    max: f64,
    step: f64,
    filter: Vec<Token>,
}

fn count_apply(e: &mut Engine, id: usize, seen: CountSeen) -> Result<(), Error> {
    let CountSeen { attribute: a, val, max, step, filter } = seen;
    if filter_passes(e, &filter)? {
        let v = val + step;
        let m = if v > max { v } else { max };
        set_attribute(e, &a, &num(v));
        set_attribute(e, &format!("{a}/max"), &num(m));
        if let Some(Object::Count { val, max, .. }) = object(e, id) {
            *val = v;
            *max = m;
        }
    }
    Ok(())
}

/// An accumulator adding the attribute's value to its sum.
fn accumulate(e: &mut Engine, id: usize, attribute_name: &str, sum: f64) {
    let prev = attribute(e, &format!("{attribute_name}/sum")).unwrap_or_else(|| "0".into());
    set_attribute(e, &format!("{attribute_name}/prev sum"), &prev);
    let v = attribute(e, attribute_name).and_then(|s| parse_value(e, &s)).unwrap_or(0.0);
    let s = sum + v;
    set_attribute(e, &format!("{attribute_name}/sum"), &num(s));
    if let Some(Object::Accumulator { sum, .. }) = object(e, id) {
        *sum = s;
    }
}

pub fn slot(e: &mut Engine, id: usize, slot: Slot, phase: Option<Phase>) -> Result<(), Error> {
    let Some(o) = object(e, id).cloned() else { return Ok(()) };
    match (o, slot) {
        (Object::ScalingMapper(_), Slot::Survey) => survey_slot(e, id),
        (Object::ScalingMapper(_), Slot::Map) => map_slot(e, id),
        (Object::ScalingMapper(_), Slot::Phase) => {
            if phase == Some(Phase::EndSurvey) {
                setup_mapper(e, id);
            }
        }
        (Object::PlotVisualizer { name, filter, projection, .. }, Slot::Protocol) => plot_protocol(e, id, PlotSeen { name, filter, projection })?,
        (Object::PlotVisualizer { .. }, Slot::Phase) => {
            if phase == Some(Phase::EndVisualization) {
                crate::legend::render_visualizer(e, id)?;
            }
        }
        (Object::RectangleVisualizer { name, .. }, Slot::Phase) => {
            if phase == Some(Phase::BeginSurvey) {
                rectangle_attributes(e, id, &name);
            }
        }
        (Object::RectangleVisualizer { name, a1, a2 }, Slot::Visualize) => {
            if attribute(e, "set").unwrap_or_default() == name {
                crate::legend::render_rectangle(e, &name, &a1, &a2)?;
            }
        }
        (Object::LabelVisualizer { attribute, threshold, filter, at, .. }, Slot::Test) => label_test(e, id, LabelSeen { attribute, threshold, filter, at })?,
        (Object::LabelVisualizer { code, before, at, .. }, Slot::Phase) => label_phase(e, phase, LabelEnd { code, before, at })?,
        (Object::StyleAttribute { attribute: a, sheet }, Slot::InvokeStyle) => invoke_style(e, &a, &sheet)?,
        (Object::Count { attribute, val, max, step, filter, .. }, Slot::Apply) => count_apply(e, id, CountSeen { attribute, val, max, step, filter })?,
        (Object::Count { start, .. }, Slot::Phase) => {
            if matches!(phase, Some(Phase::BeginSurvey | Phase::BeginVisualization)) {
                if let Some(Object::Count { val, .. }) = object(e, id) {
                    *val = start;
                }
            }
        }
        (Object::Accumulator { attribute: a, sum }, Slot::Apply) => accumulate(e, id, &a, sum),
        (Object::Accumulator { attribute: a, .. }, Slot::Phase) => {
            if matches!(phase, Some(Phase::BeginSurvey | Phase::BeginVisualization)) {
                reset_accumulator(e, id, &a);
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn reset_accumulator(e: &mut Engine, id: usize, a: &str) {
    if let Some(Object::Accumulator { sum, .. }) = object(e, id) {
        *sum = 0.0;
    }
    set_attribute(e, &format!("{a}/sum"), "0");
    set_attribute(e, &format!("{a}/prev sum"), "0");
}

/// A style attribute: the style sheet's style for the attribute's value (or its default style).
pub(crate) fn invoke_style(e: &mut Engine, attr: &str, sheet: &str) -> Result<(), Error> {
    let Some(mut value) = key_text(e, attr).filter(|s| !s.is_empty()) else { return Ok(()) };
    if let Some(redirect) = key_text(e, &format!("{attr}/{value}")) {
        value = redirect;
    }
    let base = format!("/pgf/data visualization/style sheets/{sheet}");
    let key = format!("{base}/{value}");
    let assignable = e.tex.is_defined(&macro_name(&format!("{key}/.@cmd"))) || e.tex.is_defined(&macro_name(&key));
    let default = format!("{base}/default style");
    let mut list = tex::string_tokens(if assignable { &key } else { &default });
    if !assignable {
        list.push(Token::other('='));
        list.extend(group(tex::string_tokens(&value)));
    }
    e.keys_command(list, None, false)
}
