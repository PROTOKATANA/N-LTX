//! What visualizers draw: the lines and marks of `visualize as line`, `smooth line`, `scatter`…
//! (the plot handler visualizer's `render()`, with pgf's plot handlers and plot marks), the
//! rectangles of `visualize as rectangles`, and the legends: entries laid out in rows and columns
//! by the legend class, each a text with a sample of its visualizer, in a matrix.

use tikra_engine::color::Color;
use tikra_engine::scene::Point;
use tikra_engine::tex::{self, Meaning, Token};
use tikra_engine::{Error, PT};

use super::axes::pt;
use super::objects::{Object, Step};
use super::{attribute, group, key_text, macro_text, set_attribute, PathToken, Signal};
use tikra_engine::engine::{tikra_tokens, Engine};

/// How a visualizer joins its points (pgf's plot handlers), by the name its styling gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Handler {
    LineTo,
    CurveTo,
    ClosedCurve,
    Polygon,
    Discard,
}

fn handler_of(name: &str) -> Handler {
    match name.trim() {
        "curveto" => Handler::CurveTo,
        "closedcurve" => Handler::ClosedCurve,
        "polygon" => Handler::Polygon,
        "discard" => Handler::Discard,
        _ => Handler::LineTo,
    }
}

/// pgf's plot tension (0.5) as `\pgfsetplottension` stores it.
const TENSION: f64 = 0.5 * 0.2775;

/// A stream of points as a path with a plot handler (pt).
fn stream_path(points: &[Point], h: Handler) -> String {
    let mut out = String::new();
    if points.is_empty() || h == Handler::Discard {
        return out;
    }
    let p = |q: Point| pt(q);
    match h {
        Handler::LineTo | Handler::Polygon => {
            out.push_str(&p(points[0]));
            for q in &points[1..] {
                out.push_str(" -- ");
                out.push_str(&p(*q));
            }
            if h == Handler::Polygon {
                out.push_str(" -- cycle");
            }
        }
        Handler::CurveTo => {
            out.push_str(&p(points[0]));
            if points.len() == 1 {
                return out;
            }
            let mut first = points[0];
            let mut support = points[0];
            let mut second = points[1];
            for current in &points[2..] {
                let d = current.minus(first).scale(TENSION);
                let b = second.minus(d);
                out.push_str(&format!(" .. controls {} and {} .. {}", p(support), p(b), p(second)));
                first = second;
                support = second.plus(d);
                second = *current;
            }
            out.push_str(&format!(" .. controls {} and {} .. {}", p(support), p(second), p(second)));
        }
        Handler::ClosedCurve => {
            let n = points.len();
            if n < 3 {
                return stream_path(points, Handler::Polygon);
            }
            let d = |i: usize| points[(i + 1) % n].minus(points[(i + n - 1) % n]).scale(TENSION);
            out.push_str(&p(points[0]));
            for i in 0..n {
                let j = (i + 1) % n;
                out.push_str(&format!(" .. controls {} and {} .. {}", p(points[i].plus(d(i))), p(points[j].minus(d(j))), p(points[j])));
            }
            out.push_str(" -- cycle");
        }
        Handler::Discard => {}
    }
    out
}

/// xcolor's hsb (all in [0, 1]) as rgb.
fn hsb(h: f64, s: f64, b: f64) -> Color {
    let h = (h - h.floor()) * 6.0;
    let i = h.floor() as i32 % 6;
    let f = h - h.floor();
    let (p, q, t) = (b * (1.0 - s), b * (1.0 - s * f), b * (1.0 - s * (1.0 - f)));
    let (r, g, bl) = match i {
        0 => (b, t, p),
        1 => (q, b, p),
        2 => (p, b, t),
        3 => (p, q, b),
        4 => (t, p, b),
        _ => (b, p, q),
    };
    Color::rgb(r, g, bl)
}

/// The visualizer's color (`visualizer color`, a color name) around `f`: TeX's `\colorlet` is local.
pub(crate) fn keeping_visualizer_color<T>(e: &mut Engine, f: impl FnOnce(&mut Engine) -> Result<T, Error>) -> Result<T, Error> {
    let saved = e.colors.get("visualizer color").copied();
    let r = f(e);
    match saved {
        Some(c) => {
            e.colors.insert("visualizer color".into(), c);
        }
        None => {
            e.colors.remove("visualizer color");
        }
    }
    r
}

/// A visualizer's scope (its options, `every visualizer`, its style sheets' style, its styling),
/// after the style signal once, so that the visualizer's color is known when its options use it
/// (TikZ sets colors after all options).
pub(crate) fn begin_visualizer(e: &mut Engine, name: &str) -> Result<(), Error> {
    e.begin_group();
    set_attribute(e, "set", name);
    let old = std::mem::replace(&mut e.st.key_path, "/tikz".into());
    let styling = tikra_tokens(&format!("/tikz/data visualization/visualizers/{name}/styling/.try"));
    let r = crate::emit(e, Signal::Style, None).and_then(|_| e.keys_command(styling, Some("/tikz"), true));
    e.st.key_path = old;
    e.end_group();
    r?;
    set_attribute(e, "set", name);
    let base = format!("/tikz/data visualization/visualizers/{name}");
    let mut open = tikra_tokens("\\begin{scope}[");
    open.extend(tikra_engine::keys::value(&e.tex, &format!("{base}/options")).unwrap_or_default());
    open.extend(tikra_tokens(&format!(",/tikz/data visualization/every visualizer/.try,{base}/signal,{base}/styling/.try]")));
    e.run_tokens(open)
}

pub(crate) fn end_visualizer(e: &mut Engine, _name: &str) -> Result<(), Error> {
    e.run_tokens(tikra_tokens("\\end{scope}"))
}

/// The plot handler the visualizer's styling chose.
fn current_handler(e: &mut Engine) -> Handler {
    handler_of(&macro_text(e, "tikra@dv@handler"))
}

/// The path of the points with the current handler, drawn (every path is `draw` in visualizers).
pub(crate) fn draw_stream(e: &mut Engine, positions: &[Option<Point>]) -> Result<(), Error> {
    let h = current_handler(e);
    let mut path = String::new();
    for stream in positions.split(|p| p.is_none()) {
        let pts: Vec<Point> = stream.iter().flatten().copied().collect();
        let s = stream_path(&pts, h);
        if !s.is_empty() {
            path.push(' ');
            path.push_str(&s);
        }
    }
    if path.is_empty() {
        return Ok(());
    }
    e.run_tokens(tikra_tokens(&format!("\\path{path};")))
}

/// The marks at the points (`\tikra@dv@marks`: `solid`, then `every mark`).
pub(crate) fn draw_marks(e: &mut Engine, positions: &[Option<Point>]) -> Result<(), Error> {
    if e.st.plot.plot_mark.is_none() {
        return Ok(());
    }
    e.begin_group();
    let r = (|| {
        e.keys_command(tikra_tokens("solid"), Some("/tikz"), true)?;
        let t = e.st.transform;
        let points: Vec<Point> = positions.iter().flatten().map(|p| t.apply(Point::new(p.x * PT, p.y * PT))).collect();
        let items = e.plot_marks(&points)?;
        e.canvas.items.extend(items);
        Ok(())
    })();
    e.end_group();
    r
}

/// A plot handler visualizer's `render()`, at the end of the visualization: for each of its
/// handlers (the line, the marks), the visualizer's scope and what the handler draws.
pub fn render_visualizer(e: &mut Engine, id: usize) -> Result<(), Error> {
    let Some(Object::PlotVisualizer { name, handlers, positions, .. }) = crate::running(e).last().and_then(|d| d.objects.get(id)).cloned() else { return Ok(()) };
    if let Some(Object::PlotVisualizer { positions, .. }) = crate::running(e).last_mut().and_then(|d| d.objects.get_mut(id)) {
        positions.clear();
    }
    keeping_visualizer_color(e, |e| {
        e.begin_group();
        e.begin_scope();
        let r = (|| {
            for h in super::axes_split(&handlers) {
                let Some(Token::Cs(hn)) = h.iter().find(|t| !t.is_space()).cloned() else { continue };
                begin_visualizer(e, &name)?;
                if &*hn == "tikra@dv@marks" {
                    draw_marks(e, &positions)?;
                } else {
                    draw_stream(e, &positions)?;
                }
                end_visualizer(e, &name)?;
            }
            Ok(())
        })();
        e.end_scope();
        e.end_group();
        r
    })
}

/// A rectangle of `visualize as rectangles`, from the attributes' `min` and `max` sub keys.
pub fn render_rectangle(e: &mut Engine, name: &str, a1: &str, a2: &str) -> Result<(), Error> {
    let get = |e: &mut Engine, k: &str| key_text(e, k).filter(|s| !s.is_empty());
    let (Some(amin), Some(amax), Some(bmin), Some(bmax)) = (get(e, &format!("{a1}/min")), get(e, &format!("{a1}/max")), get(e, &format!("{a2}/min")), get(e, &format!("{a2}/max")))
    else {
        return Ok(());
    };
    let (k1, k2) = (a1.trim_start_matches("/data point/").to_string(), a2.trim_start_matches("/data point/").to_string());
    keeping_visualizer_color(e, |e| {
        e.begin_group();
        let r = (|| {
            begin_visualizer(e, name)?;
            let mut steps = Vec::new();
            for (i, (x, y)) in [(&amin, &bmin), (&amin, &bmax), (&amax, &bmax), (&amax, &bmin), (&amin, &bmin)].iter().enumerate() {
                set_attribute(e, &k1, x);
                set_attribute(e, &k2, y);
                let token = if i == 0 { PathToken::MoveTo } else { PathToken::LineTo };
                steps.push(match crate::dv_generic(e, Signal::Path, token)? {
                    Step::Point(p) => p,
                    _ => continue,
                });
            }
            let path: Vec<String> = steps.iter().map(|p| pt(*p)).collect();
            e.run_tokens(tikra_tokens(&format!("\\path {} -- cycle;", path.join(" -- "))))?;
            end_visualizer(e, name)
        })();
        e.end_group();
        r
    })
}

// ---------------------------------------------------------------- legends

/// Where the next legend entry goes, and how big the grid is.
struct Place {
    row: i64,
    col: i64,
    rows: i64,
    cols: i64,
}

/// The next cell, in the order the strategy names (`down then right`…).
fn step_entry(strategy: &str, at: Place) -> Place {
    let Place { mut row, mut col, rows, cols } = at;
    match strategy {
        "right then down" => {
            col += 1;
            if col > cols {
                col = 1;
                row += 1;
            }
        }
        "up then right" => {
            row -= 1;
            if row == 0 {
                row = rows;
                col += 1;
            }
        }
        "right then up" => {
            col += 1;
            if col > cols {
                col = 1;
                row -= 1;
            }
        }
        "left then up" => {
            col -= 1;
            if col == 0 {
                col = cols;
                row -= 1;
            }
        }
        "up then left" => {
            row -= 1;
            if row == 0 {
                row = rows;
                col -= 1;
            }
        }
        "left then down" => {
            col -= 1;
            if col == 0 {
                col = cols;
                row += 1;
            }
        }
        "down then left" => {
            row += 1;
            if row > cols {
                row = 1;
                col -= 1;
            }
        }
        _ => {
            row += 1;
            if row > rows {
                row = 1;
                col += 1;
            }
        }
    }
    Place { row, col, rows, cols }
}

/// The legend's entries in rows and columns (`get arrangement`), cells split by `&` and rows
/// ended by `\\`.
pub fn legend_arrangement(e: &mut Engine, id: usize) -> Vec<Token> {
    let Some(Object::Legend { columns, ideal, max, strategy, entries }) = crate::running(e).last().and_then(|d| d.objects.get(id)).cloned() else { return Vec::new() };
    let n = entries.len();
    let (ideal, max) = (ideal.max(1), max.max(1));
    // a = number of targets, b = entries per target
    let (mut a, mut b) = if n > ideal {
        if ideal * max > n {
            (ideal, (n - 1) / ideal + 1)
        } else {
            ((n - 1) / max + 1, max)
        }
    } else {
        (n, 1)
    };
    // a = rows, b = columns
    if columns {
        std::mem::swap(&mut a, &mut b);
    }
    let (rows, cols) = (a.max(1), b.max(1));
    let mut grid = vec![vec![None; cols]; rows];
    let (r, c): (i64, i64) = match strategy.as_str() {
        "up then right" | "right then up" => (rows as i64, 1),
        "left then up" | "up then left" => (rows as i64, cols as i64),
        "left then down" | "down then left" => (1, cols as i64),
        _ => (1, 1),
    };
    let mut at = Place { row: r, col: c, rows: rows as i64, cols: cols as i64 };
    for (k, entry) in entries.iter().enumerate() {
        if at.row >= 1 && at.col >= 1 && (at.row as usize) <= rows && (at.col as usize) <= cols {
            grid[at.row as usize - 1][at.col as usize - 1] = Some(k);
        }
        let _ = entry;
        at = step_entry(strategy.as_str(), at);
    }
    let mut out = Vec::new();
    for row in grid {
        for (k, cell) in row.iter().enumerate() {
            if k > 0 {
                out.push(Token::Char('&', tikra_engine::tex::Cat::Align));
            }
            if let Some(i) = cell {
                out.extend(entries[*i].clone());
            }
        }
        out.push(Token::cs("\\"));
    }
    out
}

/// A legend, as a matrix at its `at`, anchored at its `anchor`.
pub fn show_legend(e: &mut Engine, legend: &str) -> Result<(), Error> {
    crate::update_bounding_box(e, false);
    let Some(id) = crate::object_of(e, &format!("/tikz/data visualization/{legend}/obj")) else { return Ok(()) };
    let body = legend_arrangement(e, id);
    let at = key_text(e, &format!("/tikz/data visualization/{legend}/at")).unwrap_or_default();
    let anchor = key_text(e, &format!("/tikz/data visualization/{legend}/anchor")).unwrap_or_default();
    let mut v = tikra_tokens(&format!("\\matrix[at={{{at}}},matrix anchor={anchor},/tikz/data visualization/{legend}/matrix node styling]"));
    v.extend(group(body));
    v.extend(tikra_tokens(";"));
    e.begin_group();
    let r = e.run_tokens(v);
    e.end_group();
    r
}

/// A legend entry: its text and its visualizer's sample.
pub fn render_legend_entry(e: &mut Engine, options: Vec<Token>) -> Result<(), Error> {
    keeping_visualizer_color(e, |e| {
        e.begin_group();
        let r = legend_entry_in_group(e, options);
        e.end_group();
        r
    })
}

pub(crate) fn legend_entry_in_group(e: &mut Engine, options: Vec<Token>) -> Result<(), Error> {
    // in a cell, `reset cm` keeps the cell's place
    super::keys::define_native(e, "/tikz/reset cm", super::keys::k_cell_reset_cm);
    e.keys_command(tikra_tokens("/tikz/data visualization/.cd,every data set label"), Some("/"), true)?;
    let mut keys = tikra_tokens("/tikz/data visualization/legend entry options/.cd,/tikz/data visualization/every label in legend,");
    keys.extend(options);
    e.keys_command(keys, Some("/"), true)?;
    let text = tikra_engine::keys::value(&e.tex, "/tikz/data visualization/legend entry options/text").unwrap_or_default();
    let style = tikra_engine::keys::value(&e.tex, "/tikz/data visualization/@node styling").unwrap_or_default();
    let setup = tikra_engine::keys::value(&e.tex, "/tikz/data visualization/legend entry options/setup").unwrap_or_default();
    let styling = tikra_engine::keys::value(&e.tex, "/tikz/data visualization/legend entry options/visualizer in legend styling").unwrap_or_default();
    let sample = tikra_engine::keys::value(&e.tex, "/tikz/data visualization/legend entry options/visualizer in legend").unwrap_or_default();
    let text_only = matches!(e.tex.meaning_of("iftikra@dv@textonly"), Meaning::Prim(tex::Prim::Iftrue));
    e.begin_group();
    let r = (|| {
        e.keys_command(setup, Some("/tikz"), true)?;
        e.st.key_path = "/tikz".into();
        crate::emit(e, Signal::Style, None)?;
        // the text has a strut of .75em above and .25em below its baseline (taken as its
        // height and depth: TeX's inline fractions fit in them, tikra's are taller)
        let mut node = tikra_tokens("\\node[/tikz/data visualization/node styling,text height=.75em,text depth=.25em,");
        node.extend(style);
        node.extend(tikra_tokens("]"));
        node.extend(group(text));
        node.extend(tikra_tokens(";"));
        e.run_tokens(node)?;
        if !text_only {
            e.begin_group();
            let r = e.keys_command(styling, Some("/tikz"), true).and_then(|_| e.run_tokens(sample));
            e.end_group();
            r?;
        }
        Ok(())
    })();
    e.end_group();
    r
}

/// A legend entry's sample (a line through the legend's line coordinates, marks at
/// its mark coordinates) or `…@as@rectangle`.
pub fn legend_sample(e: &mut Engine, rectangle: bool) -> Result<(), Error> {
    let name = attribute(e, "set").unwrap_or_default();
    let coords = |e: &mut Engine, key: &str| -> Result<Vec<Option<Point>>, Error> {
        let v = tikra_engine::keys::value(&e.tex, &format!("/tikz/data visualization/legend entry options/{key}")).unwrap_or_default();
        let v = e.tex.expand_fully(v)?;
        let mut out = Vec::new();
        for item in super::point_split(&v) {
            if tikra_engine::keys::trim(&item).is_empty() {
                continue;
            }
            out.push(Some(super::keys::scan_point(e, item)?));
        }
        Ok(out)
    };
    if rectangle {
        let pts = coords(e, "label in legend rectangle coordinates")?;
        begin_visualizer(e, &name)?;
        let p: Vec<Point> = pts.iter().flatten().copied().collect();
        let path = stream_path(&p, Handler::Polygon);
        if !path.is_empty() {
            e.run_tokens(tikra_tokens(&format!("\\path {path};")))?;
        }
        return end_visualizer(e, &name);
    }
    let line = coords(e, "label in legend line coordinates")?;
    begin_visualizer(e, &name)?;
    draw_stream(e, &line)?;
    end_visualizer(e, &name)?;
    let marks = coords(e, "label in legend mark coordinates")?;
    begin_visualizer(e, &name)?;
    draw_marks(e, &marks)?;
    end_visualizer(e, &name)
}

/// `\tikzdvdeclarestylesheetcolorseries{name}{model}{start}{step}`: a style sheet whose n-th
/// style is the n-th color of the series (`start` + n `step`, each part modulo 1).
pub fn declare_color_series(e: &mut Engine) -> Result<(), Error> {
    let name = e.argument_string()?;
    let model = e.argument_string()?;
    let start = e.argument_string()?;
    let step = e.argument_string()?;
    let code = format!(
            "\\pgfdvdeclarestylesheet{{{name}}}{{default style/.style={{/utils/exec=\\tikra@dv@colorseries{{{model}}}{{{start}}}{{{step}}}{{#1}},visualizer color=tikra@dv@seriescolor}}}}"
        );
    e.run_tokens(tikra_tokens(&code))
}

/// `\tikra@dv@colorseries{model}{start}{step}{n}`: the color `tikra@dv@seriescolor`.
pub fn color_series(e: &mut Engine) -> Result<(), Error> {
    let model = e.argument_string()?;
    let start = e.argument_string()?;
    let step = e.argument_string()?;
    let n = e.argument_string()?;
    let n = super::parse_value(e, &n).unwrap_or(0.0);
    let parse = |s: &str| -> Vec<f64> { s.split(',').map(|x| x.trim().parse().unwrap_or(0.0)).collect() };
    let (a, d) = (parse(&start), parse(&step));
    let part = |i: usize| -> f64 {
        let v = a.get(i).copied().unwrap_or(0.0) + n * d.get(i).copied().unwrap_or(0.0);
        if (0.0..=1.0).contains(&v) {
            v
        } else {
            v - v.floor()
        }
    };
    let c = match model.as_str() {
        "hsb" => hsb(part(0), part(1), part(2)),
        "rgb" => Color::rgb(part(0), part(1), part(2)),
        "gray" => Color::rgb(part(0), part(0), part(0)),
        _ => Color::BLACK,
    };
    e.colors.insert("tikra@dv@seriescolor".into(), c);
    Ok(())
}

// ---------------------------------------------------------------- labels in data

/// The label visualizer of `label in data={options}`: the options say which attribute it
/// watches and when to stop (its count, by default), it remembers the last two places.
pub fn make_label_in_data(e: &mut Engine, name: &str, options: Vec<Token>) -> Result<(), Error> {
    e.begin_group();
    let r = (|| {
        e.tex.define_string("tikra@dv@visualizer", name, false);
        e.tex.define_string("tikra@dv@labelattr", "", false);
        e.tex.define_string("tikra@dv@labelat", "", false);
        e.keys_command(options.clone(), Some("/tikz/data visualization/visualizer label options"), true)?;
        Ok((macro_text(e, "tikra@dv@labelattr"), macro_text(e, "tikra@dv@labelat")))
    })();
    e.end_group();
    let (attr, at) = r?;
    let mut code = vec![Token::cs("tikra@dv@labeldraw")];
    code.extend(group(tex::string_tokens(name)));
    code.extend(group(options));
    let filter = tikra_tokens(&format!("\\pgfdvvisualizerfilter{{{name}}}"));
    crate::objects::create(e, "label visualizer", &[tex::string_tokens(&attr), tex::string_tokens(&at), code, filter])?;
    Ok(())
}

/// A label in data, at the end of the visualization: a node beside the line from the place
/// before to the place where the label goes, in the visualizer's style.
pub fn draw_label_in_data(e: &mut Engine, name: &str, options: Vec<Token>) -> Result<(), Error> {
    let mut open = tikra_tokens(&format!("\\begin{{scope}}[/utils/exec={{\\def\\tikra@dv@visualizer{{{name}}}}},/tikz/data visualization/visualizer label options/.cd,"));
    open.extend(options);
    open.extend(tikra_tokens("]\\tikzdatavisualizationset{every data set label/.try}"));
    e.run_tokens(open)?;
    let text = tikra_engine::keys::value(&e.tex, "/tikz/data visualization/visualizer label options/text").unwrap_or_default();
    let option = |e: &mut Engine, key: &str| e.key_text(&format!("/tikz/data visualization/visualizer label options/{key}")).trim().to_string();
    // `pin in data`: the label moved a pin's distance off the line, to its left (right with
    // `text'`) or at `pin angle`, with a pin's edge to the place
    let mut shift = String::new();
    let mut pin_edge = String::new();
    if !option(e, "tikra pin").is_empty() {
        let length = option(e, "pin length");
        let sign = if option(e, "tikra pin sign") == "-" { -1.0 } else { 1.0 };
        let distance = e.length(&length)? * sign;
        let at = e.lookup_node("label visualizer coordinate").map(|n| n.center());
        let before = e.lookup_node("label visualizer coordinate'").map(|n| n.center());
        let angle = option(e, "pin angle");
        let d = if !angle.is_empty() {
            let a = e.number(&angle)?.to_radians();
            Some(Point::new(a.cos(), a.sin()))
        } else if let (Some(a), Some(b)) = (at, before) {
            let v = b.minus(a);
            let n = v.norm();
            (n > 1e-9).then(|| Point::new(v.y / n, -v.x / n))
        } else {
            None
        };
        if let Some(d) = d {
            shift = format!(",shift={{({}cm,{}cm)}},name=tikra@dv@pin", d.x * distance, d.y * distance);
            pin_edge = "\\path (tikra@dv@pin) edge[solid,every pin edge] (label visualizer coordinate);".into();
        }
    }
    let mut path = tikra_tokens(&format!(
            "\\path[/data point/set={{{name}}},/utils/exec=\\tikra@dv@stylesignal,/tikz/data visualization/.cd,every label in data/.try] (label visualizer coordinate') -- (label visualizer coordinate) node[auto,at end,/tikz/data visualization/node styling{shift}]"
        ));
    path.extend(group(text));
    path.extend(tikra_tokens(&format!(";{pin_edge}\\end{{scope}}")));
    e.run_tokens(path)
}
