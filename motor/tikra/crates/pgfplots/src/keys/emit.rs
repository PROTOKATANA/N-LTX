use super::table::*;

/// A key's full path: the tables name pgfplots keys without their `/pgfplots/`.
pub(in crate::keys) fn full(name: &str) -> String {
    if name.starts_with('/') {
        name.to_string()
    } else {
        format!("/pgfplots/{name}")
    }
}

/// One definition, `name/handler={value}`, onto the text.
pub(in crate::keys) fn put(s: &mut String, name: &str, handler: &str, value: &str) {
    s.push_str(&format!("{}/{handler}={{{value}}},\n", full(name)));
}

/// The lines a table line stands for: `{a}` is each of x, y and z; `{b}` is x and y, with `{o}`
/// the other of the two.
pub(in crate::keys) fn axes<const N: usize>(parts: [&str; N]) -> Vec<[String; N]> {
    let with = |pairs: &[(&str, &str)]| parts.map(|p| pairs.iter().fold(p.to_string(), |p, (from, to)| p.replace(from, to)));
    if parts.iter().any(|p| p.contains("{a}")) {
        ["x", "y", "z"].iter().map(|a| with(&[("{a}", a)])).collect()
    } else if parts.iter().any(|p| p.contains("{b}")) {
        [("x", "y"), ("y", "x")].iter().map(|(b, o)| with(&[("{b}", b), ("{o}", o)])).collect()
    } else {
        vec![parts.map(str::to_string)]
    }
}

/// Values, switches, empty hooks and ignored keys.
pub(in crate::keys) fn emit_value_keys(d: &mut String) {
    for &(n, v) in VALUES {
        axes([n, v]).iter().for_each(|[n, v]| put(d, n, ".initial", v));
    }
    for &(n, v) in SWITCHES {
        for [n, v] in axes([n, v]) {
            put(d, &n, ".initial", &v);
            put(d, &n, ".default", "true");
        }
    }
    for &n in HOOKS {
        axes([n]).iter().for_each(|[n]| put(d, n, ".style", ""));
    }
    for &n in IGNORED {
        axes([n]).iter().for_each(|[n]| put(d, n, ".code", ""));
    }
}

/// Styles that append, alias, set or spread another key.
pub(in crate::keys) fn emit_forward_keys(d: &mut String) {
    for &(n, t) in ADDS_TO {
        axes([n, t]).iter().for_each(|[n, t]| put(d, n, ".style", &format!("{}/.append style={{#1}}", full(t))));
    }
    for &(n, t) in ALIASES {
        axes([n, t]).iter().for_each(|[n, t]| put(d, n, ".style", &format!("{}={{#1}}", full(t))));
    }
    for &(n, path, v) in SETS {
        for [n, path, v] in axes([n, path, v]) {
            put(d, &n, ".code", &format!("\\pgfkeyssetvalue{{{}}}{{{v}}}", full(&path)));
        }
    }
    for &(n, targets) in SPREADS {
        let body: Vec<String> = targets.iter().map(|t| format!("{}={{#1}}", full(t))).collect();
        put(d, n, ".style", &body.join(", "));
    }
}

/// `.is choice` and the defaults that follow.
pub(in crate::keys) fn emit_choice_defaults(d: &mut String) {
    for &n in CHOICES {
        d.push_str(&format!("{}/.is choice,\n", full(n)));
    }
    for &(n, v) in DEFAULTS {
        axes([n, v]).iter().for_each(|[n, v]| put(d, n, ".default", v));
    }
}

/// Styles and codes, including the two-argument forms.
pub(in crate::keys) fn emit_style_codes(d: &mut String) {
    for &(n, v) in STYLES.iter().chain(LONG_STYLES) {
        axes([n, v]).iter().for_each(|[n, v]| put(d, n, ".style", v));
    }
    for &(n, v) in CODES {
        axes([n, v]).iter().for_each(|[n, v]| put(d, n, ".code", v));
    }
    for &(n, v) in STYLES_2 {
        put(d, n, ".style 2 args", v);
    }
    for &(n, v) in CODES_2 {
        put(d, n, ".code 2 args", v);
    }
}

/// The dash patterns pgfplots adds to TikZ's: a dash, then one or two dots.
pub(in crate::keys) fn emit_dash_patterns(d: &mut String) {
    for (spacing, gap) in [("loosely ", 4), ("", 2), ("densely ", 1)] {
        for (name, dots) in [("dashdotted", 1), ("dashdotdotted", 2)] {
            let pattern = format!("on 3pt off {gap}pt{}", format!(" on \\the\\pgflinewidth off {gap}pt").repeat(dots));
            put(d, &format!("/tikz/{spacing}{name}"), ".style", &format!("dash pattern={pattern}"));
        }
    }
}

/// `legend pos` and `legend cell align`.
pub(in crate::keys) fn emit_legend_choices(d: &mut String) {
    for &(n, at, anchor) in LEGEND_POS {
        put(d, &format!("legend pos/{n}"), ".style", &format!("/pgfplots/legend style={{at={{({at})}}, anchor={anchor}}}"));
    }
    for &(n, anchor) in LEGEND_CELL_ALIGN {
        put(d, &format!("legend cell align/{n}"), ".style", &format!("/pgfplots/legend style={{cells={{anchor={anchor}}}}}"));
    }
}

/// `ticks` and `grid`, for each of major, minor, both and none.
pub(in crate::keys) fn emit_axis_choices(d: &mut String) {
    for (key, kind) in [("ticks", "ticks"), ("grid", "grids")] {
        for &(choice, major, minor) in MAJOR_MINOR {
            let body: Vec<String> = ["x", "y", "z"].iter().map(|a| format!("{a}major{kind}={major}, {a}minor{kind}={minor}")).collect();
            put(d, &format!("{key}/{choice}"), ".style", &body.join(", "));
        }
    }
}

/// Tick labels that follow a circle.
pub(in crate::keys) fn emit_circle_labels(d: &mut String) {
    for &(n, align, anchor, upside_down) in CIRCLE_LABELS {
        let labels = format!("rotate=90, sloped like y axis={{execute for upside down={{\\tikzset{{anchor={upside_down}}}}}, reset nontranslations=false}}, anchor={anchor}");
        put(d, n, ".style", &format!("ytick align={align}, yticklabel style={{{labels}}}"));
    }
}

/// Keys that install a named cycle list.
pub(in crate::keys) fn emit_cycle_list_keys(d: &mut String) {
    for &(n, slot) in CYCLE_LIST_KEYS {
        put(d, n, ".code", &format!("\\pgfplotscreateplotcyclelist{{{slot}}}{{#1}}\\pgfkeyssetvalue{{/pgfplots/cycle list name}}{{{slot}}}"));
    }
}

/// `bar cycle list`.
pub(in crate::keys) fn emit_bar_cycle(d: &mut String) {
    let bars: Vec<String> = BAR_CYCLE.iter().map(|(line, fill)| format!("{{{line}, fill={fill}, mark=none}}")).collect();
    put(d, "bar cycle list", ".style", &format!("/pgfplots/cycle list={{{}}}", bars.join(", ")));
}

/// Colormap styles, sampled or written out.
pub(in crate::keys) fn emit_colormap_styles(d: &mut String) {
    for &(n, spec) in COLORMAPS {
        put(d, &format!("colormap/{n}"), ".style", &format!("/pgfplots/colormap={{{n}}}{{{spec}}}"));
    }
    for &(n, stops) in SAMPLED_COLORMAPS {
        let spec: Vec<String> = stops.iter().map(|rgb| format!("rgb=({rgb})")).collect();
        put(d, &format!("colormap/{n}"), ".style", &format!("/pgfplots/colormap={{{n}}}{{{}}}", spec.join(" ")));
    }
}

/// One entry of a cycle list, as TeX sees it.
pub(in crate::keys) fn cycle_entry(entry: Entry) -> String {
    let (line, dashed, mark_style, mark) = entry;
    let mut options = vec![];
    options.extend((!line.is_empty()).then(|| line.to_string()));
    options.extend(dashed.then(|| "densely dashed".to_string()));
    options.extend((!mark_style.is_empty()).then(|| format!("every mark/.append style={{{mark_style}}}")));
    options.push(format!("mark={mark}"));
    format!("{{{}}}", options.join(","))
}

/// The cycle lists built from line, dash and mark.
pub(in crate::keys) fn emit_cycle_lists(s: &mut String) {
    for &(name, entries) in CYCLE_LISTS {
        let entries: Vec<String> = entries.iter().map(|entry| cycle_entry(*entry)).collect();
        s.push_str(&format!("\\pgfplotscreateplotcyclelist{{{name}}}{{{}\\\\}}\n", entries.join("\\\\")));
    }
}

/// Cycle lists that are already a list of option strings.
pub(in crate::keys) fn emit_plain_cycle_lists(s: &mut String) {
    for &(name, options) in PLAIN_CYCLE_LISTS {
        s.push_str(&format!("\\pgfplotscreateplotcyclelist{{{name}}}{{{}\\\\}}\n", options.join("\\\\")));
    }
}

/// The pgfplots keys as TeX, in the order the tables declare them.
pub(in crate::keys) fn assemble_definitions() -> String {
    let mut s = String::from(PRELUDE);
    s.push_str("\\pgfkeys{\n");
    emit_value_keys(&mut s);
    emit_forward_keys(&mut s);
    emit_choice_defaults(&mut s);
    emit_style_codes(&mut s);
    emit_dash_patterns(&mut s);
    emit_legend_choices(&mut s);
    emit_axis_choices(&mut s);
    emit_circle_labels(&mut s);
    emit_cycle_list_keys(&mut s);
    emit_bar_cycle(&mut s);
    emit_colormap_styles(&mut s);
    s.push_str("}\n");
    emit_cycle_lists(&mut s);
    emit_plain_cycle_lists(&mut s);
    s.push_str(SETUP);
    s
}

/// The pgfplots keys as TeX, built once from the tables.
pub(crate) fn definitions() -> &'static str {
    static TEXT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    TEXT.get_or_init(assemble_definitions)
}
