use tikra_engine::keys;
use tikra_engine::tex::{self, Meaning, Token};
use tikra_engine::Error;

use super::emit::definitions;
use super::read::*;
use tikra_engine::engine::number::{SciStyle, Style};
use tikra_engine::engine::{options::KeyFn, Engine, NATIVE_KEY_BASE};

macro_rules! keyfn {
    ($name:ident, |$e:ident, $v:ident| $body:expr) => {
        fn $name($e: &mut Engine, $v: Option<Vec<Token>>) -> Result<(), Error> {
            $body
        }
    };
}
// `/pgfplots/table/.unknown`: a key of `table[…]` that is not the table's is the plot's
// (`\pgfplots@addplotimpl@table@installkeypath`), `table[point meta=…]` for one.
keyfn!(k_table_unknown, |e, v| {
    let name = match e.tex.meaning_of("pgfkeyscurrentname") {
        Meaning::Macro(m) => tex::tokens_to_string(&tex::substitute(&m.body, &[])),
        _ => String::new(),
    };
    let saved = e.st.key_path.clone();
    let r = e.invoke(&format!("/pgfplots/{name}"), v);
    e.st.key_path = saved;
    r
});

// `/pgfplots/.unknown`: a key pgfplots does not know is a TikZ key (`.search also=/tikz`).
keyfn!(k_unknown, |e, v| {
    let name = match e.tex.meaning_of("pgfkeyscurrentname") {
        Meaning::Macro(m) => tex::tokens_to_string(&tex::substitute(&m.body, &[])),
        _ => String::new(),
    };
    let saved = e.st.key_path.clone();
    let r = e.invoke(&format!("/tikz/{name}"), v);
    e.st.key_path = saved;
    r
});

// `compat=1.x`: the level, and the label placement it implies (`compat/labels`).
// `visualization depends on={expr \as \macro}`: one more, kept as a group
keyfn!(k_visualization_depends_on, |e, v| {
    let path = keys::macro_name("/pgfplots/@visualization depends on");
    let mut list = keys::value(&e.tex, "/pgfplots/@visualization depends on").unwrap_or_default();
    list.push(Token::Char('{', tex::Cat::Begin));
    list.extend(v.unwrap_or_default());
    list.push(Token::Char('}', tex::Cat::End));
    e.tex.set_meaning(&Token::cs(&path), tikra_engine::engine::macro_tokens(list), false);
    Ok(())
});

keyfn!(k_compat, |e, v| {
    let s = text(e, &v)?;
    let level = match s.as_str() {
        "newest" | "1.18" => 118,
        "default" | "pre 1.3" | "" => 0,
        other => {
            let mut parts = other.split('.');
            let major: u32 = parts.next().and_then(|p| p.trim().parse().ok()).unwrap_or(1);
            let minor: u32 = parts.next().and_then(|p| p.trim().parse().ok()).unwrap_or(0);
            major * 100 + minor
        }
    };
    set(e, "/pgfplots/compat", &s);
    set(e, "/pgfplots/@compat", &level.to_string());
    set(e, "/pgfplots/@substitute pgfpoint", if level >= 111 { "true" } else { "false" });
    if level >= 103 {
        run(e, "xlabel near ticks,ylabel near ticks")?;
    } else {
        run(e, "xlabel absolute,ylabel absolute")?;
    }
    Ok(())
});

// `compat/pgfpoint substitution=1.x`: from 1.11 on, `(1,2)` and `\pgfqpointxy` inside an axis
// are `axis cs`; before, they are in the units of the data scale transformation.
keyfn!(k_pgfpoint_substitution, |e, v| {
    let s = text(e, &v)?;
    let minor = s.trim().strip_prefix("1.").and_then(|m| m.parse::<u32>().ok());
    let on = s.trim() == "newest" || minor.is_some_and(|m| m >= 11);
    set(e, "/pgfplots/@substitute pgfpoint", if on { "true" } else { "false" });
    Ok(())
});

pub(in crate::keys) fn text(e: &mut Engine, v: &Option<Vec<Token>>) -> Result<String, Error> {
    Ok(match v {
        Some(v) => e.tex.expand_to_string(v.clone())?.trim().to_string(),
        None => String::new(),
    })
}

/// The key's last name, for keys that share their code (`/pgfplots/axis x line`).
pub(in crate::keys) fn current_name(e: &Engine) -> String {
    keys::name(&e.current_key).to_string()
}

/// `axis x line*=bottom`: where the line is, and the ticks go with it.
pub(in crate::keys) fn line_star(e: &mut Engine, axis: char, value: &str) {
    let (line, pos) = match value {
        "box" => ("box", "both"),
        "bottom" | "left" => ("low", "left"),
        "middle" | "center" => ("center", "center"),
        "top" | "right" => ("high", "right"),
        _ => ("none", "none"),
    };
    set(e, &format!("/pgfplots/@{axis} line"), line);
    set(e, &format!("/pgfplots/{axis}tick pos"), pos);
}

keyfn!(k_line_star, |e, v| {
    let name = current_name(e);
    let axis = name.chars().nth(5).unwrap_or('x');
    let value = text(e, &v)?;
    line_star(e, axis, &value);
    Ok(())
});

// `axis x line=middle`: the line, and the styles of `compat/axis line style` for it.
keyfn!(k_line, |e, v| {
    let name = current_name(e);
    let axis = name.chars().nth(5).unwrap_or('x');
    let value = text(e, &v)?;
    line_star(e, axis, &value);
    let post18 = compat(e) >= 108;
    let other = if axis == 'x' { 'y' } else { 'x' };
    let other_line = keys::value(&e.tex, &format!("/pgfplots/@{other} line")).map(|v| tex::tokens_to_string(&v)).unwrap_or_default();
    let list = match (value.as_str(), axis) {
        ("box", _) if post18 => format!("{axis}label near ticks,every boxed {axis} axis"),
        ("box", _) => format!("every boxed {axis} axis"),
        ("none", _) => format!("hide {axis} axis"),
        ("middle" | "center", 'x') if post18 => {
            let at = if other_line == "high" { "0" } else { "1" };
            format!("every axis x label/.style={{/pgfplots/near ticklabel align=inside,at={{(ticklabel* cs:{at})}},anchor=near ticklabel opposite}},every non boxed x axis")
        }
        ("middle" | "center", 'x') => {
            if other_line == "high" {
                "every axis x label/.style={at={(current axis.left of origin)},anchor=south west},every non boxed x axis".into()
            } else {
                "every axis x label/.style={at={(current axis.right of origin)},anchor=south east},every non boxed x axis".into()
            }
        }
        ("middle" | "center", 'y') if post18 => {
            if other_line == "high" {
                "every axis y label/.style={/pgfplots/near ticklabel align=outside,at={(ticklabel* cs:0)},anchor=near ticklabel opposite},every non boxed y axis".into()
            } else {
                "every axis y label/.style={/pgfplots/near ticklabel align=inside,at={(ticklabel* cs:1)},anchor=near ticklabel opposite},every non boxed y axis".into()
            }
        }
        ("middle" | "center", 'y') => {
            if other_line == "high" {
                "every axis y label/.style={at={(current axis.below origin)},anchor=south west},every non boxed y axis".into()
            } else {
                "every axis y label/.style={at={(current axis.above origin)},anchor=north west},every non boxed y axis".into()
            }
        }
        ("top" | "right", 'x') if post18 => "every axis x label/.style={at={(ticklabel cs:0.5)},anchor=near ticklabel},every non boxed x axis".into(),
        ("top" | "right", 'x') => "every axis x label/.style={at={(0.5,1)},anchor=south,yshift=15pt},every non boxed x axis".into(),
        ("top" | "right", 'y') if post18 => "every axis y label/.style={at={(ticklabel cs:0.5)},anchor=near ticklabel,rotate=90},every non boxed y axis".into(),
        ("top" | "right", 'y') => "every axis y label/.style={at={(1,0.5)},xshift=33pt,rotate=90},every non boxed y axis".into(),
        (_, a) => format!("every non boxed {a} axis"),
    };
    run(e, &list)
});

/// The plot handlers of TikZ that pgfplots draws (`const plot`, `ybar`…): stored as a value so
/// that groups restore them.
pub(in crate::keys) fn handler(e: &mut Engine, name: &str) {
    set(e, "/tikz/@plot handler", name);
}

keyfn!(k_handler, |e, _v| {
    let name = current_name(e);
    // TikZ's own `plot` draws with the same handler
    e.st.plot.plot_handler = name.clone();
    let h = match name.as_str() {
        "smooth" | "sharp plot" => "",
        "const plot" | "const plot mark left" => "const",
        "const plot mark right" => "const right",
        "const plot mark mid" => "const mid",
        "jump mark left" => "jump left",
        "jump mark right" => "jump right",
        "jump mark mid" => "jump mid",
        other => other,
    };
    handler(e, h);
    Ok(())
});

// `mesh`, `surf`, `patch`: the plot is a mesh colored by its point meta.
keyfn!(k_mesh, |e, v| {
    let name = current_name(e);
    let value = text(e, &v)?;
    if value == "false" {
        set(e, "/pgfplots/@mesh", "");
        return Ok(());
    }
    set(e, "/pgfplots/@mesh", if name == "mesh" { "m" } else { "s" });
    if name == "patch" {
        set(e, "/pgfplots/@patch", "true");
        set(e, "/pgfplots/mesh input", "patches");
    }
    run(e, "set point meta if empty=f(x),every mesh/.try,/tikz/color=mapped color")?;
    if name == "patch" {
        run(e, "every patch")?;
    }
    Ok(())
});

// `scatter`: marks colored by the colormap.
keyfn!(k_scatter, |e, v| {
    let value = text(e, &v)?;
    let on = value != "false";
    set(e, "/pgfplots/scatter", if on { "true" } else { "false" });
    if on {
        run(e, "set point meta if empty=f(x)")?;
    }
    Ok(())
});

// `color of colormap={300 of viridis}` (0 to 1000), `const color of colormap`, `index of
// colormap={4 of viridis}` (the colormap's 5th color): the current color, as
// `\pgfplots@color@of@colormap` sets it.
keyfn!(k_colormap_color, |e, v| {
    let name = current_name(e);
    let s = text(e, &v)?;
    // `\i of jet` reads `0of jet`: the first `of` after the number
    let at = s.char_indices().find(|&(i, _)| s[i..].starts_with("of") && s[..i].trim_end().chars().last().is_some_and(|c| c.is_ascii_digit() || c == '.' || c == ')'));
    let (value, map) = match at {
        Some((i, _)) => (s[..i].trim().to_string(), Some(s[i + 2..].trim().to_string())),
        None => (s.trim().to_string(), None),
    };
    let mut map_name = map.unwrap_or_else(|| keys::value(&e.tex, "/pgfplots/colormap name").map(|v| tex::tokens_to_string(&v)).unwrap_or_default());
    // `of colormap/cool`: the style defines the map
    let spec_of = |e: &Engine, n: &str| keys::value(&e.tex, &format!("/pgfplots/@colormaps/{n}")).map(|v| tex::tokens_to_string(&v)).unwrap_or_default();
    let spec = if let Some(style) = map_name.strip_prefix("colormap/").map(str::to_string) {
        e.begin_group();
        let r = run(e, &format!("/pgfplots/colormap/{style}"));
        let spec = spec_of(e, &style);
        e.end_group();
        r?;
        map_name = style;
        spec
    } else {
        spec_of(e, &map_name)
    };
    let Some(cm) = crate::colormap::Colormap::parse(&spec, &e.colors) else {
        e.warn(format!("{name}: colormap '{map_name}' not defined"));
        return Ok(());
    };
    let x = e.number(&value)?;
    let c = if name.starts_with("index") {
        let k = (x.max(0.0) as usize).min(cm.stops.len() - 1);
        cm.stops[k].1
    } else if name.starts_with("const") {
        cm.at_const(x / 1000.0)
    } else {
        cm.at(x / 1000.0)
    };
    let id = format!("pgfplots@colormap@{:.0}@{:.0}@{:.0}", c.r * 1e6, c.g * 1e6, c.b * 1e6);
    e.colors.insert(id.clone(), c);
    e.process_keys(crate::toks(&format!("/tikz/color={id}")), Some("/tikz"))
});

// After a colormap is defined: `\pgfplotscolormaplastindexof{name}` (the index of its last color).
keyfn!(k_colormap_defined, |e, v| {
    let name = text(e, &v)?;
    let mut spec = value(e, &format!("/pgfplots/@colormaps/{name}"));
    // `samples of colormap=(10 of viridis)`: that many colors of the other colormap, equally spaced
    if let Some(rest) = spec.trim().strip_prefix("samples of colormap") {
        let arg = rest.trim().trim_start_matches('=').trim().trim_matches(|c| matches!(c, '(' | ')' | '{' | '}'));
        if let Some((n, other)) = arg.split_once(" of ") {
            let n: usize = n.trim().parse().unwrap_or(0);
            if let Some(cm) = crate::colormap::named(e, other.split(',').next().unwrap_or("").trim()).filter(|_| n >= 2) {
                let colors: Vec<String> = (0..n).map(|k| cm.at(k as f64 / (n - 1) as f64)).map(|c| format!("rgb=({},{},{})", c.r, c.g, c.b)).collect();
                spec = colors.join(" ");
                set(e, &format!("/pgfplots/@colormaps/{name}"), &spec);
            }
        }
    }
    if let Some(cm) = crate::colormap::Colormap::parse(&spec, &e.colors) {
        e.tex.define_string(&format!("pgfpl@cm@{name}@last"), &(cm.stops.len() - 1).to_string(), true);
    }
    Ok(())
});

// `x SI prefix=kilo` (the units library): the prefix of the unit in the label; with `change x base`
// the tick labels in that unit too (`scaled x ticks=base 10:-3`, without a scale label).
keyfn!(k_si_prefix, |e, v| {
    let c = current_key_axis(e);
    let name = text(e, &v)?;
    let table: &[(&str, i32, &str)] = &[
        ("yocto", 24, "y"),
        ("zepto", 21, "z"),
        ("atto", 18, "a"),
        ("femto", 15, "f"),
        ("pico", 12, "p"),
        ("nano", 9, "n"),
        ("micro", 6, "\\mu"),
        ("milli", 3, "m"),
        ("centi", 2, "c"),
        ("deci", 1, "d"),
        ("deca", -1, "da"),
        ("hecto", -2, "h"),
        ("kilo", -3, "k"),
        ("mega", -6, "M"),
        ("giga", -9, "G"),
        ("tera", -12, "T"),
        ("peta", -15, "P"),
        ("exa", -18, "E"),
        ("zetta", -21, "Z"),
        ("yotta", -24, "Y"),
    ];
    let Some(&(_, exp, prefix)) = table.iter().find(|(n, _, _)| *n == name) else {
        e.warn(format!("{c} SI prefix={name} not supported: ignored"));
        return Ok(());
    };
    set(e, &format!("/pgfplots/{c} unit prefix"), prefix);
    if flag(e, &format!("/pgfplots/change {c} base")) {
        set(e, &format!("/pgfplots/scaled {c} ticks"), &format!("base 10:{exp}"));
        set(e, &format!("/pgfplots/@{c} scale label off"), "true");
    }
    Ok(())
});

/// The axis letter of the key being run (`/pgfplots/x SI prefix` → `x`).
pub(in crate::keys) fn current_key_axis(e: &Engine) -> char {
    current_name(e).chars().next().unwrap_or('x')
}

// `colormap/PuBu`, `colormap/RdPu-6`: the ColorBrewer colormaps (`\usepgfplotslibrary{colorbrewer}`),
// equally spaced colors.
keyfn!(k_colormap_unknown, |e, _v| {
    let name = e.tex.expand_to_string(vec![Token::cs("pgfkeyscurrentname")])?;
    let Some(colors) = tikra_engine::color::brewer::colormap(&name) else {
        e.warn(format!("option '{name}' not supported: ignored"));
        return Ok(());
    };
    let spec: Vec<String> = colors.iter().map(|c| format!("rgb=({},{},{})", c.r, c.g, c.b)).collect();
    run(e, &format!("/pgfplots/colormap={{{name}}}{{{}}}", spec.join(" ")))
});

// `contour prepared={…}`: the contour handler, its keys in /pgfplots/contour.
keyfn!(k_contour_prepared, |e, v| {
    set(e, "/tikz/@plot handler", "contour prepared");
    let saved = e.st.key_path.clone();
    let mut list = crate::toks("every contour plot,");
    list.extend(v.unwrap_or_default());
    let r = e.process_keys(list, Some("/pgfplots/contour"));
    e.st.key_path = saved;
    r
});

// `contour filled={…}`: a mesh drawn in bands of the levels, its keys in /pgfplots/contour.
keyfn!(k_contour_filled, |e, v| {
    run(e, "/pgfplots/surf")?;
    set(e, "/tikz/@plot handler", "contour filled");
    let saved = e.st.key_path.clone();
    let mut list = crate::toks("every filled contour plot,");
    list.extend(v.unwrap_or_default());
    let r = e.process_keys(list, Some("/pgfplots/contour"));
    e.st.key_path = saved;
    r
});

// `sloped like x axis`: the node turned to the axis' direction on paper, never upside down; it
// replaces rotations given before (`\pgfplotstransformtoaxisdirection`).
// Its options: `execute for upside down={…}` (kept for after the node's other options, as
// pgfplots runs it when the node is placed), `allow upside down`, `reset nontranslations=false`.
pub(in crate::keys) fn sloped_like(e: &mut Engine, a: usize, v: Option<Vec<Token>>) -> Result<(), Error> {
    let Some(f) = crate::fig(e).frame.clone() else { return Ok(()) };
    let (code, allow, reset) = crate::smith::sloped_options(&v.unwrap_or_default());
    // the direction at the tick being labelled (a smith chart's circle), else the axis'
    let d = f.slope.get().unwrap_or_else(|| f.e[a].scale(if f.max[a] < f.min[a] { -1.0 } else { 1.0 }));
    if d.norm() < 1e-12 {
        return Ok(());
    }
    let mut d = d;
    if d.x < 0.0 {
        if let Some(code) = code {
            // `\tikzset{anchor=…}`: its keys
            let t = tikra_engine::keys::trim(&code).to_vec();
            let keys = match t.split_first() {
                Some((first, rest)) if first.is_cs("tikzset") => tikra_engine::tex::strip_outer_braces(tikra_engine::keys::trim(rest).to_vec()),
                _ => t,
            };
            f.upside_down.replace(Some(keys));
        }
        if !allow {
            d = d.scale(-1.0);
        }
    }
    let p = e.st.pending;
    let base = if reset { tikra_engine::scene::Transform { a: 1.0, b: 0.0, c: 0.0, d: 1.0, ..p } } else { p };
    e.st.pending = base.compose(&tikra_engine::scene::Transform::rotation(d.y.atan2(d.x).to_degrees()));
    Ok(())
}
keyfn!(k_sloped_x, |e, v| sloped_like(e, 0, v));
keyfn!(k_sloped_y, |e, v| sloped_like(e, 1, v));
keyfn!(k_sloped_z, |e, v| sloped_like(e, 2, v));

// `intersection segments={of=A and B, sequence=…}` (the fillbetween library)
keyfn!(k_intersection_segments, |e, v| crate::fillbetween::intersection_segments(e, v));

// `tieline={…}`: the curve through the ties' ends, and the ties.
keyfn!(k_tieline, |e, v| {
    set(e, "/tikz/@plot handler", "tieline");
    let saved = e.st.key_path.clone();
    let r = e.process_keys(v.unwrap_or_default(), Some("/pgfplots/tieline"));
    e.st.key_path = saved;
    r
});

// `quiver={u=…, v=…}`: arrows from each point by (u, v, w).
keyfn!(k_quiver, |e, v| {
    set(e, "/pgfplots/@quiver", "true");
    set(e, "/tikz/@plot handler", "quiver");
    let saved = e.st.key_path.clone();
    let r = e.process_keys(v.unwrap_or_default(), Some("/pgfplots/quiver"));
    e.st.key_path = saved;
    r
});

// `hist={bins=…}`: the plot's values are counted in bins, then drawn with `hist/handler`.
keyfn!(k_hist, |e, v| {
    set(e, "/pgfplots/@hist", "true");
    let saved = e.st.key_path.clone();
    let r = e.process_keys(v.unwrap_or_default(), Some("/pgfplots/hist"));
    e.st.key_path = saved;
    r?;
    run(e, "/pgfplots/hist/handler")
});

// `boxplot={…}`, `boxplot prepared={…}`: the box plot handler, its keys in /pgfplots/boxplot.
keyfn!(k_boxplot, |e, v| {
    let name = current_name(e);
    set(e, "/pgfplots/@boxplot", if name.ends_with("prepared") { "prepared" } else { "raw" });
    set(e, "/tikz/@plot handler", "boxplot");
    run(e, "/pgfplots/every boxplot")?;
    let saved = e.st.key_path.clone();
    let r = e.process_keys(v.unwrap_or_default(), Some("/pgfplots/boxplot"));
    e.st.key_path = saved;
    r?;
    if flag(e, "/pgfplots/boxplot/ensure mark") && e.st.plot.plot_mark.is_none() {
        run(e, "/tikz/mark=*")?;
    }
    Ok(())
});

// `group style={…}`: keys of /pgfplots/group.
keyfn!(k_group_style, |e, v| {
    let saved = e.st.key_path.clone();
    let r = e.process_keys(v.unwrap_or_default(), Some("/pgfplots/group"));
    e.st.key_path = saved;
    r
});

// `group size=2 by 3`: columns by rows.
keyfn!(k_group_size, |e, v| {
    let s = text(e, &v)?;
    let (c, r) = s.split_once("by").unwrap_or((s.as_str(), "1"));
    set(e, "/pgfplots/group/columns", c.trim());
    set(e, "/pgfplots/group/rows", r.trim());
    Ok(())
});

keyfn!(k_set_meta, |e, v| {
    if keys::value(&e.tex, "/pgfplots/point meta").is_none_or(|m| keys::trim(&m).is_empty() || tex::tokens_to_string(&m).trim() == "none") {
        let value = v.unwrap_or_default();
        e.tex.set_meaning(&Token::cs(&keys::macro_name("/pgfplots/point meta")), tikra_engine::engine::macro_tokens(value), false);
    }
    Ok(())
});

// ------------------------------------------------------------------ /pgf/number format

pub(in crate::keys) fn nf(e: &mut Engine) -> &mut tikra_engine::engine::number::NumberFormat {
    &mut e.st.number_format
}

keyfn!(k_nf, |e, v| {
    let name = current_name(e);
    let value = text(e, &v)?;
    let on = |s: &str| s != "false";
    let f = nf(e);
    match name.as_str() {
        "fixed" => f.style = Style::Fixed,
        "frac" => f.style = Style::Frac,
        "sci" => f.style = Style::Sci,
        "std" => {
            f.style = match value.split_once(':') {
                Some((a, b)) => Style::Std { lower: a.trim().parse().ok(), upper: b.trim().parse().unwrap_or(4) },
                None if value.is_empty() => Style::Std { lower: None, upper: 4 },
                None => Style::Std { lower: value.trim().parse().ok(), upper: 4 },
            }
        }
        "int detect" => f.style = Style::IntDetect,
        "int trunc" => f.style = Style::IntTrunc,
        "relative" | "relative*" => f.style = Style::Relative(value.trim().parse().unwrap_or(0)),
        "@fixed relative" => f.style = Style::FixedRelative,
        "precision" => f.precision = value.trim().parse::<f64>().map_or(2, |p| p.max(0.0) as usize),
        "sci precision" => f.sci_precision = value.trim().parse().ok(),
        "fixed zerofill" => f.fixed_zerofill = on(&value),
        "sci zerofill" => f.sci_zerofill = on(&value),
        "zerofill" => {
            f.fixed_zerofill = on(&value);
            f.sci_zerofill = on(&value);
        }
        "set thousands separator" | "1000 sep" => f.thousands_sep = value,
        "set decimal separator" | "dec sep" => f.dec_sep = value,
        "use comma" => {
            f.dec_sep = "{,}".into();
            f.thousands_sep = ".".into();
        }
        "use period" => {
            f.dec_sep = ".".into();
            f.thousands_sep = "{,}".into();
        }
        "min exponent for 1000 sep" => f.min_exponent_1000 = value.trim().parse().unwrap_or(0),
        "showpos" | "print sign" => f.showpos = on(&value),
        "skip 0." => f.skip_zero = on(&value),
        "sci 10^e" | "sci 10e" => f.sci_style = SciStyle::TenE,
        "sci e" => f.sci_style = SciStyle::LowerE,
        "sci E" => f.sci_style = SciStyle::UpperE,
        "sci subscript" => f.sci_style = SciStyle::Subscript,
        "sci superscript" => f.sci_style = SciStyle::Superscript,
        "retain unit mantissa" => f.unit_mantissa = on(&value),
        _ => {}
    }
    Ok(())
});

pub(in crate::keys) const NATIVE: &[(&str, KeyFn)] = &[
    ("/pgfplots/.unknown", k_unknown),
    ("/pgfplots/table/.unknown", k_table_unknown),
    ("/pgfplots/compat", k_compat),
    ("/pgfplots/compat/pgfpoint substitution", k_pgfpoint_substitution),
    ("/pgfplots/visualization depends on", k_visualization_depends_on),
    ("/pgfplots/mesh", k_mesh),
    ("/pgfplots/surf", k_mesh),
    ("/pgfplots/patch", k_mesh),
    ("/pgfplots/scatter", k_scatter),
    ("/pgfplots/set point meta if empty", k_set_meta),
    ("/pgfplots/axis x line", k_line),
    ("/pgfplots/axis y line", k_line),
    ("/pgfplots/axis z line", k_line),
    ("/pgfplots/axis x line*", k_line_star),
    ("/pgfplots/axis y line*", k_line_star),
    ("/pgfplots/axis z line*", k_line_star),
    ("/pgfplots/group style", k_group_style),
    ("/pgfplots/hist", k_hist),
    ("/pgfplots/quiver", k_quiver),
    ("/pgfplots/tieline", k_tieline),
    ("/tikz/intersection segments", k_intersection_segments),
    ("/tikz/sloped like x axis", k_sloped_x),
    ("/tikz/sloped like y axis", k_sloped_y),
    ("/tikz/sloped like z axis", k_sloped_z),
    ("/pgfplots/contour prepared", k_contour_prepared),
    ("/pgfplots/contour filled", k_contour_filled),
    ("/pgfplots/boxplot", k_boxplot),
    ("/pgfplots/boxplot prepared", k_boxplot),
    ("/pgfplots/colormap/.unknown", k_colormap_unknown),
    ("/pgfplots/x SI prefix", k_si_prefix),
    ("/pgfplots/y SI prefix", k_si_prefix),
    ("/pgfplots/z SI prefix", k_si_prefix),
    ("/pgfplots/@colormap defined", k_colormap_defined),
    ("/pgfplots/color of colormap", k_colormap_color),
    ("/pgfplots/const color of colormap", k_colormap_color),
    ("/pgfplots/index of colormap", k_colormap_color),
    ("/tikz/color of colormap", k_colormap_color),
    ("/tikz/const color of colormap", k_colormap_color),
    ("/tikz/index of colormap", k_colormap_color),
    ("/pgfplots/group/group size", k_group_size),
    ("/tikz/smooth", k_handler),
    ("/tikz/sharp plot", k_handler),
    ("/tikz/const plot", k_handler),
    ("/tikz/const plot mark left", k_handler),
    ("/tikz/const plot mark right", k_handler),
    ("/tikz/const plot mark mid", k_handler),
    ("/tikz/jump mark left", k_handler),
    ("/tikz/jump mark right", k_handler),
    ("/tikz/jump mark mid", k_handler),
    ("/tikz/ycomb", k_handler),
    ("/tikz/xcomb", k_handler),
    ("/tikz/ybar", k_handler),
    ("/tikz/xbar", k_handler),
    ("/tikz/ybar interval", k_handler),
    ("/tikz/xbar interval", k_handler),
    ("/tikz/polar comb", k_handler),
];

pub(in crate::keys) const NUMBER_FORMAT: &[&str] = &[
    "frac",
    "fixed",
    "sci",
    "std",
    "int detect",
    "int trunc",
    "relative",
    "relative*",
    "@fixed relative",
    "precision",
    "sci precision",
    "fixed zerofill",
    "sci zerofill",
    "zerofill",
    "set thousands separator",
    "1000 sep",
    "set decimal separator",
    "dec sep",
    "use comma",
    "use period",
    "min exponent for 1000 sep",
    "showpos",
    "print sign",
    "skip 0.",
    "sci 10^e",
    "sci 10e",
    "sci e",
    "sci E",
    "sci subscript",
    "sci superscript",
    "retain unit mantissa",
];

pub(in crate::keys) fn native(e: &mut Engine, path: &str, f: KeyFn) {
    let id = NATIVE_KEY_BASE + e.key_fns.len() as u16;
    e.key_fns.push(f);
    e.tex.define(&keys::macro_name(&format!("{path}/.@cmd")), Meaning::Native(id));
}

/// Defines the pgfplots keys (after TikZ's).
pub fn define(e: &mut Engine) {
    // what the colormap gives each point; set while a plot is drawn
    e.colors.insert("mapped color".into(), tikra_engine::color::Color::rgb(0.0, 0.0, 0.0));
    for (path, f) in NATIVE {
        native(e, path, *f);
    }
    for name in NUMBER_FORMAT {
        native(e, &format!("/pgf/number format/{name}"), k_nf);
    }
    for (k, v) in [
        ("fixed zerofill", "true"),
        ("sci zerofill", "true"),
        ("zerofill", "true"),
        ("showpos", "true"),
        ("print sign", "true"),
        ("skip 0.", "true"),
        ("retain unit mantissa", "true"),
    ] {
        e.tex.set_meaning(&Token::cs(&keys::macro_name(&format!("/pgf/number format/{k}/.@def"))), tikra_engine::engine::macro_from(v), false);
    }
    // after TikZ's definitions and under its own path: as `mark options`, these replace TikZ's keys
    let saved = std::mem::replace(&mut e.st.key_path, "/pgfplots".into());
    let mut lexer = tikra_engine::tex::Tex::new_lexer();
    lexer.push_text(definitions());
    let mut tokens = Vec::new();
    while let Some(t) = lexer.get_token() {
        tokens.push(t);
    }
    if let Err(err) = e.run_tokens(tokens) {
        e.warn(format!("internal: pgfplots definitions failed: {err}"));
    }
    e.tex.warnings.clear();
    e.st.key_path = saved;
}
