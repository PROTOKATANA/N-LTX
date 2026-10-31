//! Voltages, currents and flows of path-style components: `v=`, `i=`, `f=` and their variants.
//!
//! A variant says on which side of the line the symbol goes (`^` the side of the path's left, `_`
//! the other), which way its arrow points (`>` toward the end of the path, `<` toward its start),
//! and, for currents and flows, at which end: the arrow sits on the lead after the component, or
//! on the one before when the direction comes before the side (`i>_`). Voltages span the
//! component, from lead to lead, in one of four styles (`voltage`): a curved arrow (European), a
//! straight one, or the American `+` and `-`, near the line or raised to the label.
//!
//! The geometry is measured on what CircuiTikZ draws with `RPvoltages`, the convention of the
//! manual: by default a passive component's voltage is below it, pointing to its start.

use tikra::extension::{Context, Error, Point, Tokens, PT};

use crate::bipole::{setting, Frame, Placed};
use crate::components::{self, Component, Leads, Source, Volts};
use crate::label;

/// Records a voltage, current or flow: `voltik/vif={kind}{variant}{text}`. Without a value
/// (`to[R, f]`) the manual only names its anchors, and nothing is drawn.
pub fn k_vif(cx: &mut Context, v: Option<Tokens>) -> Result<(), Error> {
    let a = v.unwrap_or_default().arguments(3);
    let kind = cx.expand(a[0].clone())?;
    let variant = cx.expand(a[1].clone())?;
    // a key of `\ctikzactivatevoltagedirections{vpms}` (`vpms^>`) gives the variant of its style's `v`
    let variant = if variant.is_empty() { cx.macro_text(&format!("voltik@{kind}var")) } else { variant };
    let text = if a[2].to_text().trim() == "\\pgfkeysnovalue" { Tokens::new() } else { a[2].clone() };
    cx.define(&format!("voltik@{kind}"), text);
    cx.define_text(&format!("voltik@{kind}on"), "1", false);
    cx.define_text(&format!("voltik@{kind}spec"), &variant, false);
    passive_now(cx, &kind)
}

/// Notes whether a source is passive for its voltage (`v`) or current (`i`) when that is given:
/// `bipole/is voltage=false` after `v=` does not change it, as in CircuiTikZ.
fn passive_now(cx: &mut Context, kind: &str) -> Result<(), Error> {
    let key = if kind == "i" { "bipole/is current" } else { "bipole/is voltage" };
    let overridden = setting(cx, "bipole/override source vif")? == "true";
    let passive = overridden || setting(cx, key)? == "false";
    cx.define_text(&format!("voltik@{kind}passive"), if passive { "1" } else { "0" }, false);
    cx.define_text(&format!("voltik@{kind}override"), if overridden { "1" } else { "0" }, false);
    Ok(())
}

/// What a component is for its voltage, as it was when the voltage was given.
fn voltage_source(cx: &mut Context, comp: &Component) -> Result<Source, Error> {
    match cx.macro_text("voltik@vpassive").as_str() {
        "1" => Ok(Source::Not),
        "0" => Ok(comp.source()),
        _ => source(cx, comp),
    }
}

/// What a component is for its voltages and currents: a source, unless `bipole/is voltage=false`,
/// `bipole/is current=false` or `bipole/override source vif` make it a passive one.
fn source(cx: &mut Context, comp: &Component) -> Result<Source, Error> {
    let off = |cx: &mut Context, key: &str| setting(cx, key).map(|v| v == "false");
    Ok(match comp.source() {
        _ if setting(cx, "bipole/override source vif")? == "true" => Source::Not,
        Source::Voltage if off(cx, "bipole/is voltage")? => Source::Not,
        Source::Current if off(cx, "bipole/is current")? => Source::Not,
        s => s,
    })
}

/// Records a voltage (`v`) or a current (`i`) of a variant, as a source's value does.
pub fn set(cx: &mut Context, kind: &str, variant: &str, text: Tokens) -> Result<(), Error> {
    cx.define(&format!("voltik@{kind}"), text);
    cx.define_text(&format!("voltik@{kind}spec"), variant, false);
    cx.define_text(&format!("voltik@{kind}on"), "1", false);
    passive_now(cx, kind)
}

/// A variant read: the side (1 the left of the path), the direction (1 toward its end) and whether
/// the symbol goes at the start.
struct Variant {
    side: f64,
    forward: bool,
    at_start: bool,
}

fn variant(spec: &str, side: f64, forward: bool) -> Variant {
    let side_at = spec.find(['^', '_']);
    let dir_at = spec.find(['<', '>']);
    Variant {
        side: match spec.chars().find(|c| matches!(c, '^' | '_')) {
            Some('^') => 1.0,
            Some(_) => -1.0,
            None => side,
        },
        forward: match spec.chars().find(|c| matches!(c, '<' | '>')) {
            Some('>') => true,
            Some(_) => false,
            None => forward,
        },
        at_start: matches!((dir_at, side_at), (Some(d), Some(s)) if d < s),
    }
}

/// A current's variant: `i<` alone, unlike the others, goes at the start (as `i<^`).
fn current_variant(spec: &str) -> Variant {
    let v = variant(spec, 1.0, true);
    Variant { at_start: v.at_start || spec == "<", ..v }
}

/// The size of the arrow heads (a triangle from its tip back to its base, and half its base), and
/// how far a current's tip is past the middle of its lead, in `bipoles/length`.
/// How much lower an American source's voltage label is with American voltages (in
/// `bipoles/length`).
const AMERICAN_SOURCE_LABEL: f64 = 0.1;
const HEAD_LENGTH: f64 = 0.1064;
const HEAD_HALF: f64 = 0.05;
const CURRENT_TIP: f64 = 0.0625;
/// A flow: half its length, and how far from the line.
const FLOW_HALF: f64 = 0.25;
const FLOW_SHIFT: f64 = 0.2;
/// The labels' nodes keep TikZ's inner sep (`.3333em`), the voltages' 2pt; these are how far
/// beyond it their text starts from the arrow (currents and flows), from the European arrow
/// (`label` of the component) and from the straight arrow, in `bipoles/length`.
const INNER_SEP: f64 = 3.333 * PT;
const VOLTAGE_SEP: f64 = 2.0 * PT;
const CURRENT_LABEL: f64 = 0.0886;
const STRAIGHT_LABEL: f64 = 0.0553;
/// The voltage's ends are this far off the line; the straight arrow and the American signs are as
/// far again beyond the component's `south` times 1.1; raised signs and labels this much farther.
const VOLTAGE_SHIFT: f64 = 0.08;
/// A source's voltage moves this much out for each unit of `voltage shift` (times `voltage shift
/// sources adjust`).
const SOURCE_SHIFT: f64 = 0.33;
const RAISED: f64 = 0.117;

/// What the component's frame needs: where it is, its length along the line (cm), its unit
/// (`bipoles/length` at the class's scale) and how its default size stretches across the line.
pub struct Span<'a> {
    pub placed: &'a Placed,
    pub frame: &'a Frame,
    pub start: Point,
    pub length: f64,
    pub unit: f64,
    pub y: f64,
    /// half the drawn width (cm) and half the width its voltage is placed by
    pub half: f64,
    pub volts_half: f64,
}

impl Span<'_> {
    /// A point at `along` cm from the start of the path and `across` cm to its left.
    fn at(&self, along: f64, across: f64) -> Point {
        self.start.plus(self.placed.along.scale(along)).plus(self.placed.normal.scale(across))
    }

    pub fn coordinate(&self, along: f64, across: f64) -> String {
        self.frame.coordinate(self.at(along, across))
    }
}

/// What the voltages, currents and flows of a component make: the TeX that draws their arrows,
/// their labels and signs as nodes on the path, their anchors (`Vfrom`…) and the anchors their
/// labels' nodes are placed by (for `\ctikzgetanchor`).
#[derive(Default)]
pub struct Drawn {
    pub draw: String,
    pub nodes: String,
    pub points: Vec<(&'static str, Point)>,
    pub anchors: Vec<(&'static str, String)>,
}

/// The voltages, currents and flows of a component named `name` (empty without a name).
pub fn vif(cx: &mut Context, comp: &Component, span: &Span, name: &str) -> Result<Drawn, Error> {
    let mut out = Drawn::default();
    let thin = cx.line_width();
    let u = span.unit;
    let mid = span.length / 2.0;
    // the leads: from each end to the component
    let leads = [(0.0, mid - span.half), (mid + span.half, span.length)];
    // (a `short` is all lead: its current and flow are on its middle, as TeX draws them)
    let leads = if comp.name == "short" { [(0.0, span.length); 2] } else { leads };

    for (kind, upper, node) in [("i", "I", "current"), ("f", "F", "flow")] {
        if cx.macro_text(&format!("voltik@{kind}on")) != "1" {
            continue;
        }
        let text = label::units(&cx.macro_body(&format!("voltik@{kind}")).to_text());
        // without a text (`to[R, i]`, `i_={}`) only the anchors are there
        let symbols = setting(cx, &format!("{kind} symbols"))? != "false" && !text.trim().is_empty();
        let mut spec = cx.macro_text(&format!("voltik@{kind}spec"));
        // a voltage source's current goes out of it the other way with electric-field voltages
        if kind == "i" && !spec.contains(['<', '>']) && source(cx, comp)? == Source::Voltage && !matches!(setting(cx, "voltage dir")?.as_str(), "RP" | "old") {
            spec.insert(0, '<');
        }
        let v = if kind == "i" { current_variant(&spec) } else { variant(&spec, 1.0, true) };
        let (a, b) = if v.at_start { leads[0] } else { leads[1] };
        // `current/distance` and `flow/distance` (0.5) of the way along the lead, in the path's way
        let distance = number(cx, if kind == "i" { "current/distance" } else { "flow/distance" }).unwrap_or(0.5);
        let lead = a + distance * (b - a);
        let dir = if v.forward { 1.0 } else { -1.0 };
        let across = if kind == "i" { 0.0 } else { v.side * FLOW_SHIFT * u };
        let (from, to) = if kind == "i" { (a, b) } else { (lead - dir * FLOW_HALF * u, lead + dir * FLOW_HALF * u) };
        let tip = if kind == "i" { lead + dir * CURRENT_TIP * u } else { to };
        if symbols {
            if kind == "f" {
                out.draw.push_str(&format!("\\draw[-,solid,line width={}cm] {} -- {};", num(thin), span.coordinate(from, across), span.coordinate(to, across)));
            }
            // (`current arrow scale`: 16 is its size; the smaller, the bigger the arrow)
            let size = 16.0 / number(cx, "current arrow scale").filter(|s| *s > 0.0).unwrap_or(16.0);
            out.draw.push_str(&head(span, span.at(tip, across), span.placed.along.scale(dir), u * size, thin));
        }
        // `\ctikzgetdirection{R1-Iarrow}`: the way the arrow points (drawn or not), in whole degrees as TeX truncates them
        if kind == "i" && !name.is_empty() {
            let way = span.frame.angle(span.placed.along.scale(dir));
            let full = format!("{}{name}-Iarrow", cx.macro_text("voltik@prefix"));
            cx.define_text(&format!("voltik@direction@{full}"), &format!("{}", way.trunc() as i64), true);
        }
        let (from, to) = if kind == "i" && !v.forward { (to, from) } else { (from, to) };
        out.points.extend([(pos_name(upper, "from"), span.at(from, across)), (pos_name(upper, "to"), span.at(to, across)), (pos_name(upper, "pos"), span.at(lead, across))]);
        let anchor = anchor(span, v.side);
        out.anchors.push((if kind == "i" { "Ilab" } else { "Flab" }, anchor.clone()));
        if !text.trim().is_empty() {
            let at = span.at(lead, across + v.side * CURRENT_LABEL * u - v.side * INNER_SEP);
            out.nodes.push_str(&label_node(span, at, &anchor, &text, &options(name, node, "outer sep=0pt,")));
        }
    }

    if cx.macro_text("voltik@von") == "1" {
        voltage(cx, comp, span, name, &mut out)?;
    }
    Ok(out)
}

/// The anchor names of currents and flows: `Ifrom`, `Fpos`…
fn pos_name(kind: &str, what: &str) -> &'static str {
    match (kind, what) {
        ("I", "from") => "Ifrom",
        ("I", "to") => "Ito",
        ("I", _) => "Ipos",
        ("F", "from") => "Ffrom",
        ("F", "to") => "Fto",
        _ => "Fpos",
    }
}

/// The options of a label's node: its name (`R1voltage`), more, and its style (`bipole voltage
/// style`), which may change what comes before it (`outer sep=5pt`).
fn options(name: &str, node: &str, more: &str) -> String {
    let named = if name.is_empty() { String::new() } else { format!("name={{{name}{node}}},") };
    format!("{named}{more}/tikz/circuitikz/bipole {node} style,")
}

/// A component's voltage.
fn voltage(cx: &mut Context, comp: &Component, span: &Span, name: &str, out: &mut Drawn) -> Result<(), Error> {
    let thin = cx.line_width();
    let u = span.unit;
    // the arrow heads keep their size when the class is scaled (`resistors/scale`)
    let class_scale = number(cx, &format!("{}/scale", comp.class.name())).filter(|s| *s > 0.0).unwrap_or(1.0);
    let head_u = u / class_scale;
    let mid = span.length / 2.0;
    let text = label::units(&cx.macro_body("voltik@v").to_text());
    let labelled = !text.trim().is_empty();
    let symbols = setting(cx, "v symbols")? != "false" && labelled;
    // the arrow points to the start by default with rising-potential voltages (`RP`, `old`), to the
    // end with electric-field ones (`EF`, `noold`, the package's default); `>` and `<` say it
    let rising = matches!(setting(cx, "voltage dir")?.as_str(), "RP" | "old");
    // a source's voltage is the other way round from a passive component's; a voltage source's
    // is a straight arrow above it, pointing to its end
    let source = voltage_source(cx, comp)?;
    let v = match source {
        Source::Voltage => variant(&cx.macro_text("voltik@vspec"), 1.0, true),
        Source::Current => variant(&cx.macro_text("voltik@vspec"), -1.0, rising),
        Source::Not => variant(&cx.macro_text("voltik@vspec"), -1.0, !rising),
    };
    // a mirrored component's top is on the other side of the line
    let volts = match (comp.volts(), (v.side > 0.0) != span.placed.mirror) {
        (Volts { above: Some([south, bump, label]), .. }, true) => Volts { south, bump, label, ..comp.volts() },
        (volts, _) => volts,
    };
    let style = setting(cx, "voltage")?;
    let from_node = match (volts.from_node, number(cx, "voltage/distance from node")) {
        (d, Some(set)) if (d - 0.5).abs() < 1e-9 => set,
        (d, _) => d,
    };
    let bump = match number(cx, "voltage/bump b") {
        Some(b) if (volts.bump - 1.5 * volts.south).abs() < 1e-3 => b * volts.south,
        _ => volts.bump,
    };
    // the European label follows a deeper bump (1.097 of it, measured)
    let european_label = volts.label + 1.0969 * (bump - volts.bump);
    // the two ends, between each end of the path (a little inside) and the component; an open
    // circuit's are on the line itself, and so is all of its American voltage
    let open = comp.leads == Leads::Gap;
    let gap = if open { 0.0 } else { components::NODE_GAP * u };
    let left = (1.0 - from_node) * gap + from_node * (mid - span.volts_half);
    let right = (1.0 - from_node) * (span.length - gap) + from_node * (mid + span.volts_half);
    let side = v.side;
    // the arrow goes from `tail` to `tip`; the plus is at its tip when the potential rises along it
    let (tail, tip) = if v.forward { (left, right) } else { (right, left) };
    let (minus, plus) = if rising { (tail, tip) } else { (tip, tail) };
    let y = span.y;
    // `voltage shift` moves everything of the voltage out by that many times 0.08
    // a source's by `voltage shift sources adjust` times that
    let adjust = if source == Source::Not { 1.0 } else { number(cx, "voltage shift sources adjust").unwrap_or(1.0) };
    // `bipoles/<shape>/voltage/additional shift`, a component's own (a developer's key)
    let own = components::bipole_shapes()
        .find(|(_, c)| *c == comp.name)
        .and_then(|(shape, _)| number(cx, &format!("bipoles/{}/voltage/additional shift", shape.trim_end_matches("shape"))));
    let push = number(cx, "voltage shift").unwrap_or(0.0) * adjust * VOLTAGE_SHIFT * u + (volts.shift + own.unwrap_or(0.0) * VOLTAGE_SHIFT) * y;
    let straight = push + if open { 0.0 } else { (1.1 * volts.south + VOLTAGE_SHIFT) * y };
    // how far the straight and American labels' nodes are from their line
    let straight_label = if open { CURRENT_LABEL } else { STRAIGHT_LABEL } * u - VOLTAGE_SEP;
    // the American label, `american label distance` (1.4) from its signs' line
    let american_label = straight_label - (1.4 - number(cx, "voltage/american label distance").unwrap_or(1.4)) * 0.15 * u;
    let anchor = anchor(span, side);
    let sep = format!("inner sep={}cm,outer sep=0pt,", num(VOLTAGE_SEP));
    let label_options = options(name, "voltage", &sep);
    let (from, to, cont, lab, lab_anchor);
    // the side and way are the ones in force when the voltage was given, the drawing the final one
    // (CircuiTikZ's mix of the two, `sV=…, bipole/is voltage=false`)
    // (`bipole/override source vif` in force when the voltage was given decides the drawing too)
    let drawn_as = if cx.macro_text("voltik@voverride") == "1" { Source::Not } else { crate::vif::source(cx, comp)? };
    let style = match (drawn_as, style.as_str()) {
        (Source::Voltage, "american" | "raised") => "source label",
        (Source::Voltage, _) => "source",
        (_, s) => s,
    };
    match style {
        "source" | "source label" => {
            // across the source, `half` each side of its middle, `above` the line
            let [half, above, gap] = comp.source_volts().unwrap_or([0.2286, 0.396, 0.1153]);
            // `voltage shift` moves a source's voltage out by `SOURCE_SHIFT` (times the adjust), its
            // ends along the lines from the source's middle: the arrow grows as it goes out
            let shift = number(cx, "voltage shift").unwrap_or(0.0) * adjust * (SOURCE_SHIFT - VOLTAGE_SHIFT) * u;
            let out_by = above * y + push + shift;
            let half = if above > 0.0 { half * out_by / (above * y) } else { half };
            let across = side * out_by;
            let (a, b) = if v.forward { (mid - half * u, mid + half * u) } else { (mid + half * u, mid - half * u) };
            from = span.at(a, across);
            to = span.at(b, across);
            cont = [from.lerp(to, 0.5), from.lerp(to, 0.5)];
            if symbols && style == "source" {
                out.draw.push_str(&format!("\\draw[-,solid,line width={}cm] {} -- {};", num(thin), span.frame.coordinate(from), span.frame.coordinate(to)));
                let dir = if v.forward { 1.0 } else { -1.0 };
                out.draw.push_str(&head(span, to, span.placed.along.scale(dir), head_u, thin));
            }
            // American signs where the arrow would be, but on the American sources, which carry theirs
            if symbols && style == "source label" && !comp.name.starts_with("american") {
                let font = setting(cx, "voltage/american font")?;
                let font = if font.is_empty() { String::new() } else { format!("font={{{font}}},") };
                let plus_sign = cx.expand(Tokens::text("\\unexpanded\\expandafter{\\romannumeral-`0\\pgfkeysvalueof{/tikz/circuitikz/voltage/american plus}}"))?;
                let minus_sign = cx.expand(Tokens::text("\\unexpanded\\expandafter{\\romannumeral-`0\\pgfkeysvalueof{/tikz/circuitikz/voltage/american minus}}"))?;
                for (at, sign) in [(to, plus_sign), (from, minus_sign)] {
                    out.nodes.push_str(&label_node(span, at, "center", &sign, &font));
                }
            }
            // an American source's label sits lower, where its arrow would have been
            let gap = if style == "source label" && comp.name.starts_with("american") { gap - AMERICAN_SOURCE_LABEL } else { gap };
            lab = span.at(mid, across + side * (gap * y - VOLTAGE_SEP));
            lab_anchor = anchor.clone();
        }
        "american" | "raised" => {
            let raised = style == "raised" || open;
            // (from `voltage shift=-1` down, as measured on TeX, the raise itself shrinks with it:
            // nothing at -1, as much the other way at -2)
            let shift = number(cx, "voltage shift").unwrap_or(0.0) * adjust;
            let raise = if shift <= -1.0 { 1.0 + shift } else { 1.0 };
            let across = if raised && !open { side * (straight + raise * (STRAIGHT_LABEL + RAISED) * head_u) } else { side * push };
            let font = setting(cx, "voltage/american font")?;
            let font = if font.is_empty() { String::new() } else { format!("font={{{font}}},") };
            let plus_sign = cx.expand(Tokens::text("\\unexpanded\\expandafter{\\romannumeral-`0\\pgfkeysvalueof{/tikz/circuitikz/voltage/american plus}}"))?;
            let minus_sign = cx.expand(Tokens::text("\\unexpanded\\expandafter{\\romannumeral-`0\\pgfkeysvalueof{/tikz/circuitikz/voltage/american minus}}"))?;
            from = span.at(minus, across);
            to = span.at(plus, across);
            cont = [from.lerp(to, 0.5), from.lerp(to, 0.5)];
            if symbols {
                for (at, sign) in [(to, plus_sign), (from, minus_sign)] {
                    out.nodes.push_str(&label_node(span, at, if raised { "center" } else { &anchor }, &sign, &font));
                }
            }
            if raised {
                lab = span.at(mid, across);
                lab_anchor = "center".to_string();
            } else {
                lab = match comp.source_volts() {
                    // an American voltage source drawn as a passive one (`bipole/override source vif`)
                    // keeps a source's label
                    Some([_, above, gap]) if comp.source() == Source::Voltage && comp.name.starts_with("american") => {
                        span.at(mid, side * ((above + gap - AMERICAN_SOURCE_LABEL) * y + push - VOLTAGE_SEP))
                    }
                    _ => span.at(mid, side * (straight + american_label)),
                };
                lab_anchor = anchor.clone();
            }
        }
        "straight" => {
            from = span.at(tail, side * straight);
            to = span.at(tip, side * straight);
            cont = [from.lerp(to, 0.5), from.lerp(to, 0.5)];
            if symbols {
                out.draw.push_str(&format!("\\draw[-,solid,line width={}cm] {} -- {};", num(thin), span.frame.coordinate(from), span.frame.coordinate(to)));
                let dir = if tip > tail { 1.0 } else { -1.0 };
                out.draw.push_str(&head(span, to, span.placed.along.scale(dir), head_u, thin));
            }
            lab = span.at(mid, side * (straight + straight_label));
            lab_anchor = anchor.clone();
        }
        _ => {
            // European: a curve below (or above) the component, bent `bump` deep
            let end = side * (push + if open { 0.0 } else { VOLTAGE_SHIFT * y });
            let deep = side * (push + (VOLTAGE_SHIFT + 1.1 * bump) * y);
            let reach = 0.4 * bump * y;
            let (c1, c2) = if tip > tail { (mid - reach, mid + reach) } else { (mid + reach, mid - reach) };
            from = span.at(tail, end);
            to = span.at(tip, end);
            cont = [span.at(c1, deep), span.at(c2, deep)];
            if symbols {
                let c = |p: Point| span.frame.coordinate(p);
                out.draw.push_str(&format!("\\draw[-,solid,line width={}cm] {} .. controls {} and {} .. {};", num(thin), c(from), c(cont[0]), c(cont[1]), c(to)));
                let d = to.minus(cont[1]);
                out.draw.push_str(&head(span, to, d.scale(1.0 / d.norm().max(1e-9)), head_u, thin));
            }
            lab = span.at(mid, side * (push + european_label * y) - side * VOLTAGE_SEP);
            lab_anchor = anchor.clone();
        }
    }
    out.points.extend([("Vfrom", from), ("Vto", to), ("Vcont1", cont[0]), ("Vcont2", cont[1]), ("Vlab", lab)]);
    // `\ctikzgetanchor{R1}{Vlab}` answers the side's anchor, even where a raised label is centered
    out.anchors.push(("Vlab", if lab_anchor == "center" { anchor.clone() } else { lab_anchor.clone() }));
    if labelled {
        out.nodes.push_str(&label_node(span, lab, &lab_anchor, &text, &label_options));
    }
    Ok(())
}

/// A filled triangle with its tip at `tip`, pointing along the unit vector `dir`.
fn head(span: &Span, tip: Point, dir: Point, u: f64, thin: f64) -> String {
    let n = Point::new(-dir.y, dir.x);
    let base = tip.minus(dir.scale(HEAD_LENGTH * u));
    let c = |p: Point| span.frame.coordinate(p);
    format!("\\filldraw[-,solid,color=.,line width={}cm] {} -- {} -- {} -- cycle;", num(thin), c(base.plus(n.scale(HEAD_HALF * u))), c(tip), c(base.minus(n.scale(HEAD_HALF * u))))
}

/// The anchor of a label on side `side` of the line: the side of its box that faces the line, an
/// edge when the way out is within 4° of an axis, else a corner.
pub fn anchor(span: &Span, side: f64) -> String {
    let out = span.placed.normal.scale(side);
    let angle = out.y.atan2(out.x).to_degrees();
    let near = |a: f64| (angle - a + 540.0).rem_euclid(360.0) - 180.0;
    let vertical = if near(90.0).abs() < 86.0 {
        "south"
    } else if near(-90.0).abs() < 86.0 {
        "north"
    } else {
        ""
    };
    let horizontal = if near(0.0).abs() < 86.0 {
        "west"
    } else if near(180.0).abs() < 86.0 {
        "east"
    } else {
        ""
    };
    match (vertical, horizontal) {
        ("", h) => h.to_string(),
        (v, "") => v.to_string(),
        (v, h) => format!("{v} {h}"),
    }
}

/// A node at `at` with `anchor` and more options.
fn label_node(span: &Span, at: Point, anchor: &str, text: &str, options: &str) -> String {
    format!(" node[{options}anchor={anchor}] at {} {{{text}}}", span.frame.coordinate(at))
}

/// A circuitikz setting read as a number, or `None` when it is not set.
fn number(cx: &mut Context, key: &str) -> Option<f64> {
    match setting(cx, key) {
        Ok(s) if !s.is_empty() => cx.number(Tokens::text(&s)).ok(),
        _ => None,
    }
}

fn num(x: f64) -> String {
    format!("{:.5}", x)
}

#[cfg(test)]
mod tests {
    use super::variant;

    #[test]
    fn variants() {
        let v = variant("", 1.0, true);
        assert!(v.side > 0.0 && v.forward && !v.at_start);
        let v = variant("_<", 1.0, true);
        assert!(v.side < 0.0 && !v.forward && !v.at_start);
        let v = variant(">_", 1.0, true);
        assert!(v.side < 0.0 && v.forward && v.at_start);
        let v = variant("^", -1.0, false);
        assert!(v.side > 0.0 && !v.forward);
    }
}
