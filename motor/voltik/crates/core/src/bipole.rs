//! Path-style components: `\draw (0,0) to[R=$R_1$, o-*] (2,0);`.
//!
//! A component key (`R`, `C`, …) records which component the `to` draws and sets its `to path`. When
//! the path is built, `voltik/place` reads the two ends, puts the component halfway, turned along the
//! line, draws its outline with the thick stroke of its class, and leaves the rest of the path in
//! `\voltik@path`: the leads up to the component, the poles and the labels, as nodes on the path so
//! that they are drawn over the wires.

use tikra::extension::{Context, Error, Point, Tokens, Transform, PT};

use crate::components::{self, Leads, Op, Part, Source, Target};
use crate::{label, vif};

/// The `to path` of every component: place it, then the rest of the path.
const TO_PATH: &str = "to path={\\pgfextra{\\pgfkeys{/tikz/voltik/place}}\\voltik@path}";

/// A circuitikz setting, `\ctikzvalof{key}`, as text.
pub fn setting(cx: &mut Context, key: &str) -> Result<String, Error> {
    cx.expand(Tokens::text(&format!("\\pgfkeysvalueof{{/tikz/circuitikz/{key}}}")))
}

/// A circuitikz setting read as a number; `default` when empty or not a number.
pub fn number(cx: &mut Context, key: &str, default: f64) -> f64 {
    match setting(cx, key) {
        Ok(s) if !s.is_empty() && s != "none" => cx.number(Tokens::text(&s)).unwrap_or(default),
        _ => default,
    }
}

/// Whether a key's value is missing (`to[R]`): pgfkeys passes `\pgfkeysnovalue` to a style.
fn present(value: &Option<Tokens>) -> Option<Tokens> {
    let v = value.as_ref()?.trim();
    let text = v.to_text();
    if text.is_empty() || text.trim() == "\\pgfkeysnovalue" {
        return None;
    }
    Some(v)
}

/// `voltik/component={name}{value}`: the `to` draws this component; the value, if any, is its label.
pub fn k_component(cx: &mut Context, v: Option<Tokens>) -> Result<(), Error> {
    let a = v.unwrap_or_default().arguments(3);
    let name = cx.expand(a[0].clone())?;
    let variant = cx.expand(a[2].clone())?;
    // a spark gap's circle keeps the dot given before it
    if !(name == "sparkgap/circle" && cx.macro_text("voltik@kind") == "sparkgap/dot") {
        cx.define("voltik@kind", Tokens::text(&name));
    }
    if let Some(value) = present(&Some(a[1].clone())) {
        // a source's value is its voltage or its current, with the key's variant (`I<=…`)
        let kind = match components::target(&name) {
            Some(Target::Fixed(n) | Target::Voltage(n, _) | Target::Current(n, _)) => components::find(n).map(|c| c.source()),
            _ => None,
        };
        match kind {
            Some(Source::Voltage) => vif::set(cx, "v", &variant, value)?,
            Some(Source::Current) => vif::set(cx, "i", &variant, value)?,
            // a transistor's value is its text, in its middle (and a motor's, and a path-style logic port's)
            _ if components::find(&name).is_some_and(|c| matches!(c.class, components::Class::Transistors | components::Class::LogicPorts) || c.name == "Telmech") => {
                cx.define("voltik@t", value)
            }
            _ => label::set(cx, "l", 1, value),
        }
    }
    cx.keys(Tokens::text(TO_PATH))
}

/// `voltik/pole={start}{end}`: the poles at the two ends, as `bipole nodes` gives them: a shape
/// (`circ`, `ocirc`, `diamondpole`…) and options for it (`ocirc, fill=red`), or `none`.
pub fn k_pole(cx: &mut Context, v: Option<Tokens>) -> Result<(), Error> {
    let a = v.unwrap_or_default().arguments(2);
    // (a macro's value, as a `\\foreach` gives: a pole's options are not expanded)
    for (name, pole) in [("voltik@ps", &a[0]), ("voltik@pe", &a[1])] {
        let text = pole.to_text();
        let pole = if text.trim_start().starts_with('\\') { Tokens::text(&cx.expand(pole.clone())?) } else { pole.clone() };
        cx.define(name, pole);
    }
    Ok(())
}

/// `voltik/flag=name`: `mirror` and `invert`, flipped each time they are given.
pub fn k_flag(cx: &mut Context, v: Option<Tokens>) -> Result<(), Error> {
    let name = cx.expand(v.unwrap_or_default())?;
    let macro_name = format!("voltik@{name}");
    let on = cx.macro_text(&macro_name) == "1";
    cx.define(&macro_name, Tokens::text(if on { "0" } else { "1" }));
    Ok(())
}

/// The component the `to` draws: the short names (`R`, `vR`, `L`) follow the current style.
fn component(cx: &mut Context) -> Result<Option<&'static components::Component>, Error> {
    let kind = cx.macro_text("voltik@kind");
    let name = match components::target(&kind) {
        Some(Target::Fixed(name)) => name,
        Some(Target::Resistor(american, european)) => match setting(cx, "resistor")?.as_str() {
            "european" => european,
            _ => american,
        },
        Some(Target::Inductor(cute, american, european)) => match setting(cx, "inductor")?.as_str() {
            "american" => american,
            "european" => european,
            _ => cute,
        },
        Some(Target::Voltage(european, american)) => match setting(cx, "voltage")?.as_str() {
            "american" | "raised" => american,
            _ => european,
        },
        Some(Target::Current(european, american)) => match setting(cx, "current")?.as_str() {
            "american" => american,
            _ => european,
        },
        Some(Target::Diode(empty, full, stroke)) => match setting(cx, "diode")?.as_str() {
            "full" => full,
            "stroke" => stroke,
            _ => empty,
        },
        None => {
            cx.warn(format!("voltik: unknown component {kind}; drawn as a short"));
            return Ok(None);
        }
    };
    // the other shapes of thyristors, LEDs, photodiodes and whiskered diodes, by their settings
    if setting(cx, "thyristor style")? == "compact" {
        if let Some(compact) = components::find(&format!("compact {name}")) {
            return Ok(Some(compact));
        }
    }
    for (key, value, suffix) in [("led arrows", "cathode", "@cathode"), ("pd arrows", "cathode", "@cathode"), ("diode whiskers", "straight", "@straight")] {
        let fits = match key {
            "led arrows" => name.ends_with(" led"),
            "pd arrows" => name.ends_with(" photodiode"),
            _ => name.contains("ZZener") || name.contains("TVS"),
        };
        if fits && setting(cx, key)? == value {
            return Ok(components::find(&format!("{name}{suffix}")));
        }
    }
    // a block's variants: `boxed`, `ideal filter`, a bare transmission line
    for (key, suffix) in [("blocks/boxed", "|boxed"), ("blocks/ideal filter", "|ideal filter"), ("bipoles/tline/bare", "|bare")] {
        if setting(cx, key)? == "true" {
            if let Some(variant) = components::find(&format!("{name}{suffix}")) {
                return Ok(Some(variant));
            }
        }
    }
    // a logic gate on a path in the `logic ports` style (`inline not` in IEEE style)
    if let Some(styled) = components::find(&format!("{name}|{}", setting(cx, "logic ports")?)) {
        return Ok(Some(styled));
    }
    // a three-pins jumper's connections
    if name == "three-pins jumper" {
        let connections = setting(cx, "tjumper connections")?;
        if let Some(jumper) = components::find(&format!("{name}@{connections}")) {
            return Ok(Some(jumper));
        }
    }
    Ok(components::find(name))
}

/// The middle of a path's points (a pole's center).
fn middle(ops: &[Op]) -> [f64; 2] {
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for op in ops {
        let pts: &[[f64; 2]] = match op {
            Op::Move(x, y) | Op::Line(x, y) => &[[*x, *y]],
            Op::Curve(a, b, c) => &[*a, *b, *c],
            Op::Close => &[],
        };
        for p in pts {
            for i in 0..2 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        }
    }
    [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0]
}

/// The inverse of an affine transform (the identity if it is singular).
fn inverse(t: &Transform) -> Transform {
    let det = t.a * t.d - t.b * t.c;
    if det.abs() < 1e-12 {
        return Transform::default();
    }
    let (a, b, c, d) = (t.d / det, -t.b / det, -t.c / det, t.a / det);
    Transform { a, b, c, d, e: -(a * t.e + c * t.f), f: -(b * t.e + d * t.f) }
}

/// A canvas point as a picture coordinate: `(x cm, y cm)`, which the path's transform maps back.
pub struct Frame {
    to_picture: Transform,
}

impl Frame {
    pub fn coordinate(&self, p: Point) -> String {
        let q = self.to_picture.apply(p);
        format!("({}cm,{}cm)", num(q.x), num(q.y))
    }

    /// A canvas direction as a picture coordinate.
    fn vector(&self, way: Point) -> String {
        let q = self.to_picture.apply(way).minus(self.to_picture.apply(Point::new(0.0, 0.0)));
        format!("({}cm,{}cm)", num(q.x), num(q.y))
    }

    /// The angle of a canvas direction in the picture's own coordinates (degrees), as CircuiTikZ
    /// turns labels and reports directions: before `rotate` and `transform shape` do.
    pub fn angle(&self, way: Point) -> f64 {
        let q = self.to_picture.apply(way).minus(self.to_picture.apply(Point::new(0.0, 0.0)));
        q.y.atan2(q.x).to_degrees()
    }
}

fn num(x: f64) -> String {
    let s = format!("{:.5}", x);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".into()
    } else {
        s.into()
    }
}

/// The geometry of a placed component, on the canvas (cm).
#[derive(Clone)]
pub struct Placed {
    pub mid: Point,
    /// unit vector from start to end, and its left normal
    pub along: Point,
    pub normal: Point,
    /// the direction of the line, in degrees
    pub angle: f64,
    pub width: f64,
    /// how far the component reaches above and below the line (its north and south), unmirrored
    pub top: f64,
    pub bottom: f64,
    pub mirror: bool,
    pub invert: bool,
}

impl Placed {
    /// How far the component reaches on one side of the line: `side` 1 is the side of the normal.
    pub fn reach(&self, side: f64) -> f64 {
        if (side > 0.0) != self.mirror {
            self.top
        } else {
            self.bottom
        }
    }

    /// A point of the component's frame on the canvas.
    pub fn at(&self, x: f64, y: f64) -> Point {
        let x = if self.invert { -x } else { x };
        let y = if self.mirror { -y } else { y };
        self.mid.plus(self.along.scale(x)).plus(self.normal.scale(y))
    }
}

/// `voltik/place`, run inside the `to path`: draws the component and defines `\voltik@path`.
pub fn k_place(cx: &mut Context, _v: Option<Tokens>) -> Result<(), Error> {
    // what `vif queue add` queued needs the component's anchors: one without a name gets one
    if !cx.macro_text("voltik@vq").trim().is_empty() && cx.macro_text("voltik@name").is_empty() {
        let n = cx.macro_text("voltik@autoname").parse::<u32>().unwrap_or(0) + 1;
        cx.define_text("voltik@autoname", &n.to_string(), true);
        cx.define_text("voltik@name", &format!("ctikzauto{n}"), false);
    }
    let start_name = cx.expand(Tokens::text("\\tikztostart"))?;
    let end_name = cx.expand(Tokens::text("\\tikztotarget"))?;
    let start = cx.point(&start_name)?;
    let end = cx.point(&end_name)?;
    let frame = Frame { to_picture: inverse(&cx.transform()) };

    let length_setting = setting(cx, "bipoles/length")?;
    let unit = cx.length(Tokens::text(&length_setting)).unwrap_or(1.4);
    // with `transform shape`, the picture's scale scales the component too
    let unit = if cx.macro_text("voltik@ts") == "1" {
        let t = cx.transform();
        unit * (t.a * t.d - t.b * t.c).abs().sqrt()
    } else {
        unit
    };
    let known = component(cx)?;
    let comp = known.or_else(|| components::find("short")).expect("short is a component");
    let class = comp.class.name();
    let scale = number(cx, &format!("{class}/scale"), 1.0);
    // the transformer sources as big as a single source (`ootrans size=large`)
    let large = match comp.name {
        "oosourcetrans" | "oosourceatrans" if setting(cx, "ootrans size")? == "large" => 1.0 / 0.6,
        "ooosource" if setting(cx, "ootrans size")? == "large" => 1.0 / 0.55,
        _ => 1.0,
    };
    let scale = scale * large;
    let comp = if large != 1.0 && comp.name == "oosourcetrans" { components::find("oosourcetrans large").unwrap_or(comp) } else { comp };
    // the class's width and height settings, when given, are the same for all its components (but
    // for the ones that are wider than the others, by their ratio)
    // a bipole drawn as a node (`node[resistorshape]`) fills the node's width, whatever its size
    let unit = if cx.macro_text("voltik@fit") == "1" {
        let base = own_size(cx, comp)
            .0
            .or_else(|| components::blocks_size(cx, comp).map(|(w, _)| w))
            .or_else(|| optional(cx, &format!("{class}/width")).map(|w| w * comp.ratio))
            .unwrap_or(comp.width)
            * scale;
        end.minus(start).norm() / base.max(1e-9)
    } else {
        unit
    };
    // a component's own size (`bipoles/oscope/width`), over its class's
    let (own_width, own_height) = own_size(cx, comp);
    let width = own_width.or_else(|| optional(cx, &format!("{class}/width")).map(|w| w * comp.ratio)).unwrap_or(comp.width) * unit * scale;
    let height = own_height.or_else(|| optional(cx, &format!("{class}/height"))).unwrap_or(comp.height) * unit * scale;
    // a block's own size (`bipoles/amp/width`): square but for the generic two-ports
    let (width, height) = match crate::components::blocks_size(cx, comp) {
        Some((w, h)) => (w * unit * scale, h * unit * scale),
        None => (width, height),
    };
    let thickness = optional(cx, &format!("{class}/thickness")).unwrap_or_else(|| number(cx, "bipoles/thickness", 2.0));
    let thick = thickness * cx.line_width();

    // cm to a unit of the default size, along and across the line
    // (one with no width, a barrier, keeps its drawing at its size)
    let x = if comp.width > 0.0 { width / comp.width } else { unit * scale };
    let y = if comp.height > 0.0 { height / comp.height } else { 0.0 };

    let up = cx.macro_body("voltik@up");
    let up_is_down = if up.to_text().trim().is_empty() {
        false
    } else {
        let north = cx.expand(up)?;
        let n = cx.point(&north)?;
        let d = end.minus(start);
        let m = start.lerp(end, 0.5);
        // left of the line is up
        d.x * (n.y - m.y) - d.y * (n.x - m.x) < 0.0
    };
    let d = end.minus(start);
    let length = d.norm().max(1e-9);
    let along = d.scale(1.0 / length);
    let placed = Placed {
        mid: start.lerp(end, 0.5),
        along,
        normal: Point::new(-along.y, along.x),
        angle: d.y.atan2(d.x).to_degrees(),
        width,
        top: comp.top * y,
        bottom: comp.bottom * y,
        // a transistor inverted is turned round: its gate goes to the other side too; a bipole
        // drawn as a node goes the way the node's north is (`yscale=-1`)
        mirror: ((cx.macro_text("voltik@mirror") == "1") != (comp.class == components::Class::Transistors && cx.macro_text("voltik@invert") == "1")) != up_is_down,
        invert: cx.macro_text("voltik@invert") == "1",
    };
    let look = Look {
        placed: &placed,
        frame: &frame,
        x,
        y,
        unit: unit * scale,
        thick,
        modifier: thick * number(cx, &format!("{class}/modifier thickness"), 1.0),
        thin: cx.line_width(),
        // the `fill` of the `to`, or its class's (`\ctikzset{resistors/fill=yellow}`)
        fill: match fill(cx) {
            Some(f) => Some(f),
            None => match setting(cx, &format!("{class}/fill"))?.as_str() {
                "" | "none" => None,
                c => Some(format!("fill={{{c}}}")),
            },
        },
    };

    if !comp.parts.is_empty() {
        let mut code = String::new();
        if comp.extent > 0.0 {
            // room CircuiTikZ makes around it, beyond what it draws
            code.push_str(&format!("\\path {} {};", look.at([0.0, comp.extent]), look.at([0.0, -comp.extent])));
        }
        // the poles of jumpers and cute switches: their own shape (`bipoles/jumper/shape`), and
        // nodes of their own when the component is named (`J1-in`, `J1-tap`, `J1-out`)
        let shape_key = if comp.name.contains("jumper") { "bipoles/jumper/shape" } else { "bipoles/cuteswitch/shape" };
        let pole_shape = setting(cx, shape_key)?;
        let mut poles: Vec<[f64; 2]> = comp
            .parts
            .iter()
            .filter_map(|p| match p {
                Part::Pole(ops) => Some(middle(ops)),
                _ => None,
            })
            .collect();
        poles.sort_by(|a, b| a[0].total_cmp(&b[0]));
        // a block's inner drawing, after its box, in `blocks/inner color`
        let inner = match setting(cx, "blocks/inner color")?.as_str() {
            "" | "default" => None,
            color if comp.class == components::Class::Blocks => Some(color.to_string()),
            _ => None,
        };
        let mut boxed = false;
        let mut scoped = false;
        // an instrument's screen upright, around its middle
        // (and a split two-port's diagonal, whichever way the path goes)
        let straight = comp.name == "twoportsplit" || (STRAIGHT.contains(&comp.name) && setting(cx, "instruments/rotated")? != "true");
        let upright = Placed { along: Point::new(1.0, 0.0), normal: Point::new(0.0, 1.0), angle: 0.0, mirror: false, invert: false, ..placed.clone() };
        let upright = Look { placed: &upright, ..look.clone() };
        // a wiggly fuse's poles: none (`bipoles/wfuse/dots=false`), or other shapes, nodes of their
        // own when it is named (`F-left`, `F-right`)
        let wfuse = comp.name == "wfuse";
        let wfuse_dots = wfuse && setting(cx, "bipoles/wfuse/dots")? != "false";
        let wfuse_shape = setting(cx, "bipoles/wfuse/shape")?;
        let fuse_name = cx.macro_text("voltik@name");
        let spark = if comp.name.starts_with("sparkgap") { sparkgap_arrows(cx, &look)? } else { None };
        let oosized = matches!(comp.name, "ioosource" | "voosource") && (own_width.is_some() || own_height.is_some());
        let customs = own_arrows(cx, comp)?;
        let mut head = false;
        for (i, part) in comp.parts.iter().enumerate() {
            if std::mem::take(&mut head) {
                continue;
            }
            // the arrows of a look of their own, whole, with the tips of TikZ
            if let Some(custom) = &customs {
                let stroke = match part {
                    Part::Thin(ops) => Some((ops, look.thin)),
                    Part::Line(ops) => Some((ops, look.thick)),
                    _ => None,
                };
                let tip = match comp.parts.get(i + 1) {
                    Some(Part::Solid(h)) if h.iter().any(|o| matches!(o, Op::Curve(..))) => stroke.map(|(ops, _)| head_tip(ops, h)),
                    _ => None,
                };
                if let (Some((ops, width)), Some(tip)) = (stroke, tip) {
                    if let (true, Some(Part::Solid(h))) = (custom.own_head, comp.parts.get(i + 1)) {
                        let color = custom.options.split(',').find(|o| o.starts_with("color")).unwrap_or("color=.");
                        code.push_str(&format!(
                            "\\draw[{{{}}}-,solid,{}line width={}cm]{};\\fill[{color}]{};",
                            custom.start,
                            custom.options,
                            num(width * custom.thickness),
                            look.path(ops),
                            look.path(h)
                        ));
                    } else {
                        code.push_str(&format!("\\draw[{}]{} -- {};", custom.draw(width), look.path(ops), look.at(tip)));
                    }
                    head = true;
                    continue;
                }
                // (a GTO's gate has two lines and no heads)
                if let (true, Part::Thin(ops)) = (comp.name.contains("gto"), part) {
                    if matches!(ops[..], [Op::Move(x, y), Op::Line(x2, y2)] if (x - x2).abs() > 1e-9 && (y - y2).abs() > 1e-9) {
                        code.push_str(&format!("\\draw[{}]{};", custom.draw(look.thin), look.path(ops)));
                        continue;
                    }
                }
                if let Part::Arrow { from, to, thin } = part {
                    if comp.name.contains("light dependent") || comp.name == "photoresistor" {
                        code.push_str(&format!("\\draw[{}] {} -- {};", custom.draw(if *thin { look.thin } else { look.modifier }), look.at(*from), look.at(*to)));
                        continue;
                    }
                }
            }
            if let (Some(color), true, false) = (&inner, boxed, scoped) {
                code.push_str(&format!("\\begin{{scope}}[color={{{color}}}]"));
                scoped = true;
            }
            boxed |= matches!(part, Part::Line(_));
            let look = if straight && i >= 2 { &upright } else { &look };
            if wfuse && i >= 1 {
                if wfuse_dots && i % 2 == 1 {
                    let at = middle(match part {
                        Part::Paint(_, ops) => ops,
                        _ => &[],
                    });
                    if wfuse_shape != "ocirc" {
                        let fill = look.fill.clone().map_or(String::new(), |f| format!(",{f}"));
                        code.push_str(&format!("\\node[{wfuse_shape}{fill}] at {} {{}};", look.at(at)));
                    }
                    if !fuse_name.is_empty() {
                        let side = if at[0] < 0.0 { "left" } else { "right" };
                        code.push_str(&format!("\\node[shape=voltik pole, voltik/pole shape={wfuse_shape}, name={fuse_name}-{side}] at {} {{}};", look.at(at)));
                    }
                }
                if !wfuse_dots || wfuse_shape != "ocirc" {
                    continue;
                }
            }
            // a spark gap's arrows of other tips or gap (`sparkgap end arrow`, `sparkgap/gap`)
            if spark.is_some() && matches!(part, Part::Thin(ops) | Part::Solid(ops) if !ops.iter().any(|op| matches!(op, Op::Curve(..)))) {
                if let (Some(spark), Part::Thin(ops)) = (&spark, part) {
                    // drawn once, with the first lead
                    if ops.len() == 2 && ops[0] == Op::Move(0.3, 0.0) {
                        code.push_str(spark);
                    }
                }
                continue;
            }
            // a noise source's gray, in `bipoles/noise sources/fillcolor`, or hatched (`dashed`)
            if let (true, Part::Paint(_, ops)) = (comp.name.starts_with("noise "), part) {
                code.push_str(&noise_fill(cx, look, ops)?);
                continue;
            }
            // a double-circle source of its own size (`bipoles/oosource/width`, `height`): round
            // circles as high as it is, touching its ends
            if oosized {
                if i == 0 {
                    code.push_str(&oosource_circles(look, width, height));
                }
                continue;
            }
            // a current tap's dot, by `bipoles/currtap/…`
            if comp.name == "currtap" && i >= 1 {
                if i == 1 {
                    code.push_str(&currtap_dot(cx, look)?);
                }
                continue;
            }
            // a source's waveform, turned by `sources/symbol/rotate` (upright with `auto`)
            if let (true, Part::Line(ops)) = (i >= 2 && WAVE_SOURCES.contains(&comp.name), part) {
                code.push_str(&wave_symbol(cx, look, &upright, comp, ops)?);
                continue;
            }
            match part {
                Part::Pole(ops) if pole_shape != "ocirc" => {
                    let fill = look.fill.clone().map_or(String::new(), |f| format!(",{f}"));
                    code.push_str(&format!("\\node[{pole_shape}{fill}] at {} {{}};", look.at(middle(ops))));
                }
                _ if comp.class == components::Class::Blocks => {
                    let drawn = match block_part(cx, look, comp, *part)? {
                        Some(drawn) => drawn,
                        None => draw_part(cx, look, comp, *part),
                    };
                    code.push_str(&dashed_block(cx, *part, drawn)?);
                }
                _ => code.push_str(&draw_part(cx, look, comp, *part)),
            }
        }
        // an ideal filter's scale, left of its grid
        if comp.name.ends_with("|ideal filter") && setting(cx, "blocks/ideal numbers")? == "true" {
            for (key, y) in [("low", -0.21), ("high", 0.21)] {
                let text = setting(cx, &format!("ideal filter number {key}"))?;
                code.push_str(&format!("\\node[anchor=east,inner sep=0pt,font=\\fontsize{{3.2}}{{3.9}}\\selectfont] at {} {{{text}}};", look.at([-0.25, y])));
            }
        }
        if scoped {
            code.push_str("\\end{scope}");
        }
        code.push_str(&windings(cx, &look, comp, scale));
        if cx.macro_text("voltik@gt") == "1" {
            let tip = -comp.width / 2.0;
            let (long, half) = (BLOCK_ARROW[0] * look.unit / look.x, BLOCK_ARROW[1] * look.unit / look.y);
            code.push_str(&format!("\\fill[black] {} -- {} -- {} -- cycle;", look.at([tip, 0.0]), look.at([tip - long, half]), look.at([tip - long, -half])));
        }
        let name = cx.macro_text("voltik@name");
        if !name.is_empty() && !poles.is_empty() {
            let names: &[&str] = if poles.len() == 3 { &["in", "tap", "out"] } else { &["in", "out"] };
            for (pole, pname) in poles.iter().zip(names) {
                code.push_str(&format!("\\node[shape=voltik pole, voltik/pole shape={pole_shape}, name={name}-{pname}] at {} {{}};", look.at(*pole)));
            }
        }
        cx.run(Tokens::text(&code))?;
    }
    let name = cx.macro_text("voltik@name");
    if !name.is_empty() {
        let code = anchors(cx, &look, comp, &name);
        cx.run(Tokens::text(&code))?;
    }

    // the leads, then what goes over them
    let mut path = match comp.leads {
        Leads::Gap => "(\\tikztotarget)".to_string(),
        Leads::Wire => "-- (\\tikztotarget)".to_string(),
        // the leads stop at the component's ends, whichever way it is turned
        Leads::Ends => format!(
            "-- {} {} -- (\\tikztotarget)",
            frame.coordinate(placed.mid.minus(placed.along.scale(width / 2.0))),
            frame.coordinate(placed.mid.plus(placed.along.scale(width / 2.0)))
        ),
    };
    path.push_str(&label::nodes(cx, &placed, &frame, unit)?);
    // `t=…`: upright in the middle of the component (below it on a screen's meter)
    let t = cx.macro_body("voltik@t");
    if !t.to_text().trim().is_empty() {
        let low = if comp.name == "smeter" { 0.15 } else { 0.0 };
        // (a block puts it where it has room: left of a generator's sign, in an amplifier's triangle)
        let [dx, dy] = components::block_text(comp.name);
        let rotated = STRAIGHT.contains(&comp.name) && setting(cx, "instruments/rotated")? == "true";
        // (a screen kept upright has its bottom down the page, whichever way the line goes)
        let down = if STRAIGHT.contains(&comp.name) && !rotated { Point::new(0.0, 1.0) } else { placed.normal };
        let at = placed.mid.minus(down.scale(low * y)).plus(placed.along.scale(dx * x)).plus(placed.normal.scale(dy * y));
        let turn = if rotated { format!(", rotate={}", num(placed.angle)) } else { String::new() };
        path.push_str(&format!(" node[anchor=center{turn}] at {} {{{}}}", frame.coordinate(at), label::units(&t.to_text())));
    }
    // `t1=…`, `t2=…`: a split two-port's texts, in its halves or by its corners (`component text=border`)
    for (key, corner) in [("voltik@ta", Point::new(-1.0, 1.0)), ("voltik@tb", Point::new(1.0, -1.0))] {
        let t = cx.macro_body(key);
        if t.to_text().trim().is_empty() || !comp.name.starts_with("twoportsplit") {
            continue;
        }
        let (hx, hy) = (comp.width / 2.0 * x, comp.height / 2.0 * y);
        // (upright, as its diagonal, its first text at its top left: at its bottom right on a path
        // going left or up)
        let back = placed.along.x < -1e-6 || placed.along.y > 1e-6;
        let corner = if back { corner.scale(-1.0) } else { corner };
        let point = |cx: f64, cy: f64| placed.mid.plus(Point::new(cx, cy));
        let (at, anchor) = if setting(cx, "component text")? == "border" {
            let d = distance(cx, "border text distance", 0.0703);
            let at = point(corner.x * (hx - d), corner.y * (hy - d));
            // the text's corner that faces the block's, wherever the block turns
            let out = point(corner.x * hx, corner.y * hy).minus(placed.mid);
            let canvas = frame.to_picture.apply(out).minus(frame.to_picture.apply(Point::new(0.0, 0.0)));
            let anchor = format!("{} {}", if canvas.y > 0.0 { "north" } else { "south" }, if canvas.x > 0.0 { "east" } else { "west" });
            (at, anchor)
        } else {
            (point(corner.x * 0.4486 * hx, corner.y * 0.4486 * hy), "center".to_string())
        };
        path.push_str(&format!(" node[anchor={anchor}, inner sep=0pt] at {} {{{}}}", frame.coordinate(at), label::units(&t.to_text())));
    }
    let span = vif::Span { placed: &placed, frame: &frame, start, length, unit: unit * scale, y, half: width / 2.0, volts_half: comp.volts().half.map_or(width / 2.0, |h| h * x) };
    // a component voltik does not draw yet gets no voltage nor current: its convention is its own
    let drawn = if known.is_some() { vif::vif(cx, comp, &span, &name)? } else { vif::Drawn::default() };
    if !drawn.draw.is_empty() {
        cx.run(Tokens::text(&drawn.draw))?;
    }
    path.push_str(&drawn.nodes);
    if !name.is_empty() {
        // what `\ctikzgetanchor` and `\ctikzgetdirection` answer go by the name with its prefix
        let full = format!("{}{name}", cx.macro_text("voltik@prefix"));
        let mut code = String::new();
        for (point, at) in &drawn.points {
            code.push_str(&format!("\\coordinate ({name}-{point}) at {};", frame.coordinate(*at)));
        }
        cx.run(Tokens::text(&code))?;
        for (key, anchor) in &drawn.anchors {
            cx.define_text(&format!("voltik@anchor@{full}@{key}"), anchor, true);
        }
        // `\ctikzgetdirection{R1}`: the way the component goes, in degrees
        cx.define_text(&format!("voltik@direction@{full}"), &num(frame.angle(placed.along).rem_euclid(360.0)), true);
        // `vif queue add={macro}{a}{b}{c}`: `\macro{R1}{a}{b}{c}`, at the end of the picture
        // (or at `\ctikzprocessvif`), when the component's nodes are there too
        if !cx.macro_text("voltik@vq").trim().is_empty() {
            let calls = cx.macro_body("voltik@vq").to_text().replace("\\voltik@qn", &full);
            let queue = cx.macro_body("voltik@queue").to_text() + &calls;
            cx.define_tex("voltik@queue", &queue, true);
        }
    }
    path.push_str(&poles(cx, &frame, start, end, unit));
    path.push_str(" \\tikztonodes");
    cx.define("voltik@path", Tokens::text(&path));
    Ok(())
}

/// The components with a size of their own (`bipoles/oscope/width`, `height`), by its key.
const SIZES: &[(&str, &str)] = &[("oscope", "oscope"), ("ioosource", "oosource"), ("voosource", "oosource")];

/// A component's own width and height, when set (`bipoles/oscope/width`), in `bipoles/length`.
pub fn own_size(cx: &mut Context, comp: &components::Component) -> (Option<f64>, Option<f64>) {
    let Some((_, key)) = SIZES.iter().find(|(name, _)| *name == comp.name) else { return (None, None) };
    (optional(cx, &format!("bipoles/{key}/width")), optional(cx, &format!("bipoles/{key}/height")))
}

/// The sources whose waveform symbol turns by `sources/symbol/rotate` (or `csources/…`).
const WAVE_SOURCES: &[&str] = &[
    "sinusoidal voltage source",
    "sinusoidal current source",
    "square voltage source",
    "vsourcetri",
    "controlled sinusoidal voltage source",
    "controlled sinusoidal current source",
];

/// A source's waveform symbol: turned from its default (along the line, `rotate=90`) by its class's
/// `symbol/rotate`, or kept upright with `auto`, and as thick as `symbol/thickness` says.
fn wave_symbol(cx: &mut Context, look: &Look, upright: &Look, comp: &components::Component, ops: &[Op]) -> Result<String, Error> {
    let class = comp.class.name();
    let rotate = setting(cx, &format!("{class}/symbol/rotate"))?;
    let (look, turn) = match rotate.trim() {
        "auto" => (upright, -90.0f64),
        "" => (look, 0.0),
        r => (look, cx.number(Tokens::text(r)).unwrap_or(90.0) - 90.0),
    };
    let (c, sn) = (turn.to_radians().cos(), turn.to_radians().sin());
    let q = |p: [f64; 2]| [p[0] * c - p[1] * sn, p[0] * sn + p[1] * c];
    let turned: Vec<Op> = ops
        .iter()
        .map(|op| match *op {
            Op::Move(x, y) => {
                let p = q([x, y]);
                Op::Move(p[0], p[1])
            }
            Op::Line(x, y) => {
                let p = q([x, y]);
                Op::Line(p[0], p[1])
            }
            Op::Curve(a, b, e) => Op::Curve(q(a), q(b), q(e)),
            Op::Close => Op::Close,
        })
        .collect();
    let width = look.thick * number(cx, &format!("{class}/symbol/thickness"), 1.0);
    Ok(format!("\\draw[-,solid,fill=none,miter limit=10,line width={}cm]{};", num(width), look.path(&turned)))
}

/// A current tap's dot: `bipoles/currtap/dot size` across (relative to its height), filled
/// (`fill`: the wire's color, `none` or a color) and stroked (`color`, `thickness`, `dash`).
fn currtap_dot(cx: &mut Context, look: &Look) -> Result<String, Error> {
    let r = 0.2 * number(cx, "bipoles/currtap/dot size", 0.5);
    let k = 0.5523 * r;
    let circle = [
        Op::Move(r, 0.0),
        Op::Curve([r, k], [k, r], [0.0, r]),
        Op::Curve([-k, r], [-r, k], [-r, 0.0]),
        Op::Curve([-r, -k], [-k, -r], [0.0, -r]),
        Op::Curve([k, -r], [r, -k], [r, 0.0]),
        Op::Close,
    ];
    let mut code = String::new();
    match setting(cx, "bipoles/currtap/fill")?.trim() {
        "none" => {}
        "" | "default" => code.push_str(&format!("\\fill[color=.]{};", look.path(&circle))),
        c => code.push_str(&format!("\\path[fill={{{c}}}]{};", look.path(&circle))),
    }
    let width = match setting(cx, "bipoles/currtap/thickness")?.trim() {
        "" | "default" => look.thin,
        t => cx.number(Tokens::text(t)).unwrap_or(1.0) * look.thin,
    };
    let mut extra = String::new();
    match setting(cx, "bipoles/currtap/color")?.trim() {
        "" | "default" => {}
        c => extra.push_str(&format!("draw={{{c}}},")),
    }
    let dash = setting(cx, "bipoles/currtap/dash")?;
    match dash.trim() {
        "" | "default" => {}
        "none" => extra.push_str("solid,"),
        d => {
            let parts: Vec<&str> = d.split(['{', '}']).map(str::trim).filter(|p| !p.is_empty()).collect();
            let pattern: Vec<String> = parts.chunks(2).map(|c| format!("on {} off {}", c[0], c.get(1).unwrap_or(&"0pt"))).collect();
            extra.push_str(&format!("dash pattern={},", pattern.join(" ")));
        }
    }
    code.push_str(&format!("\\draw[-,{extra}fill=none,miter limit=10,line width={}cm]{};", num(width), look.path(&circle)));
    Ok(code)
}

/// A noise source's inside: filled with `bipoles/noise sources/fillcolor` (gray), or with
/// `dashed` hatched at 45 degrees and filled only with the component's fill.
fn noise_fill(cx: &mut Context, look: &Look, ops: &[Op]) -> Result<String, Error> {
    let color = setting(cx, "bipoles/noise sources/fillcolor")?;
    match color.trim() {
        "dashed" => {
            let mut code = look.fill.as_ref().map_or(String::new(), |f| format!("\\path[{f}]{};", look.path(ops)));
            // the hatching: lines x + y = c, an eighth of a length apart, inside the circle
            let r: f64 = 0.3;
            let mut lines = Vec::new();
            for k in -3..=3 {
                let c = 0.125 * k as f64;
                let half = (r * r - c * c / 2.0).max(0.0).sqrt() / std::f64::consts::SQRT_2;
                lines.extend([Op::Move(c / 2.0 - half, c / 2.0 + half), Op::Line(c / 2.0 + half, c / 2.0 - half)]);
            }
            code.push_str(&format!("\\draw[-,solid,fill=none,line width={}cm]{};", num(look.thin), look.path(&lines)));
            Ok(code)
        }
        "" => Ok(format!("\\path[fill={{black!25}}]{};", look.path(ops))),
        c => Ok(format!("\\path[fill={{{c}}}]{};", look.path(ops))),
    }
}

/// The two circles of a double-circle source `width` by `height` (cm), filled and stroked.
fn oosource_circles(look: &Look, width: f64, height: f64) -> String {
    let r = height / 2.0;
    let mut code = String::new();
    for side in [-1.0, 1.0] {
        let x = side * (width / 2.0 - r);
        let k = 0.5523 * r;
        let circle = [
            Op::Move(x + r, 0.0),
            Op::Curve([x + r, k], [x + k, r], [x, r]),
            Op::Curve([x - k, r], [x - r, k], [x - r, 0.0]),
            Op::Curve([x - r, -k], [x - k, -r], [x, -r]),
            Op::Curve([x + k, -r], [x + r, -k], [x + r, 0.0]),
            Op::Close,
        ];
        if let Some(f) = &look.fill {
            code.push_str(&format!("\\path[{f}]{};", look.path_cm(&circle)));
        }
        code.push_str(&format!("\\draw[-,solid,fill=none,miter limit=10,line width={}cm]{};", num(look.thick), look.path_cm(&circle)));
    }
    code
}

/// How the arrows of a group are drawn when any of its settings is changed: their tips (`opto end
/// arrow`, `switch start arrow`…) and, if the group has them, the color, dash and relative thickness
/// of `{style}` (`opto arrows`, `switch arrows`).
pub struct ArrowLook {
    pub start: String,
    pub end: String,
    pub options: String,
    pub thickness: f64,
    /// Its end is the default tip: the head measured stays, in the arrow's color.
    pub own_head: bool,
}

impl ArrowLook {
    /// The options of a `\\draw` of such an arrow, of line width `width` cm.
    pub fn draw(&self, width: f64) -> String {
        format!("{{{}}}-{{{}}},solid,{}line width={}cm", self.start, self.end, self.options, num(width * self.thickness))
    }
}

/// The look of `group`'s arrows (of default end tip `end`), `None` when nothing changes them.
pub fn arrow_look(cx: &mut Context, group: &str, style: Option<&str>, end: &str) -> Result<Option<ArrowLook>, Error> {
    let tip = |cx: &mut Context, which: &str| -> Result<Option<String>, Error> {
        Ok(match setting(cx, &format!("{group} {which} arrow"))?.trim() {
            "" | "default" => None,
            t => Some(t.to_string()),
        })
    };
    let (start_tip, end_tip) = (tip(cx, "start")?, tip(cx, "end")?);
    let mut changed = start_tip.is_some() || end_tip.is_some();
    let (mut options, mut thickness) = (String::new(), 1.0);
    if let Some(style) = style {
        thickness = number(cx, &format!("{style}/relative thickness"), 1.0);
        changed |= (thickness - 1.0).abs() > 1e-9;
        let color = setting(cx, &format!("{style}/color"))?;
        if !matches!(color.trim(), "" | "default") {
            changed = true;
            options.push_str(&format!("color={{{}}},", color.trim()));
        }
        let dash = setting(cx, &format!("{style}/dash"))?;
        if !matches!(dash.trim(), "" | "default" | "none") {
            changed = true;
            options.push_str(&format!("dash pattern={{{}}},", dash_pattern(&dash)));
        }
    }
    let own_head = end_tip.is_none();
    Ok(changed.then(|| ArrowLook { start: start_tip.unwrap_or_default(), end: if own_head { end.to_string() } else { end_tip.unwrap_or_default() }, options, thickness, own_head }))
}

/// Where the head `head` of a stroke `stroke` points: of its corners, the farthest from the stroke's end.
pub fn head_tip(stroke: &[Op], head: &[Op]) -> [f64; 2] {
    let last = |ops: &[Op]| {
        ops.iter().rev().find_map(|o| match *o {
            Op::Move(x, y) | Op::Line(x, y) => Some([x, y]),
            Op::Curve(_, _, c) => Some(c),
            Op::Close => None,
        })
    };
    let from = last(stroke).unwrap_or([0.0, 0.0]);
    let corners = head.iter().filter_map(|o| match *o {
        Op::Move(x, y) | Op::Line(x, y) => Some([x, y]),
        Op::Curve(_, _, c) => Some(c),
        Op::Close => None,
    });
    corners.max_by(|a, b| (a[0] - from[0]).hypot(a[1] - from[1]).total_cmp(&(b[0] - from[0]).hypot(b[1] - from[1]))).unwrap_or(from)
}

/// The arrows of a component's look that are not its own: the group of the `component`'s light or
/// switching arrows, and its gate's.
fn own_arrows(cx: &mut Context, comp: &components::Component) -> Result<Option<ArrowLook>, Error> {
    let name = comp.name;
    if name.contains("gto") {
        arrow_look(cx, "gto gate", None, "")
    } else if comp.class == components::Class::Switches {
        arrow_look(cx, "switch", Some("switch arrows"), "latexslim")
    } else if name.contains("led") || name.contains("photo") || name.contains("laser") || name.contains("light dependent") || name == "solar" {
        arrow_look(cx, "opto", Some("opto arrows"), "latexslim")
    } else {
        Ok(None)
    }
}

/// A spark gap's two arrows, from its ends to `sparkgap/gap` (of its half width) apart, with
/// `sparkgap end arrow` tips; `None` for the default ones, drawn as measured.
fn sparkgap_arrows(cx: &mut Context, look: &Look) -> Result<Option<String>, Error> {
    let tip = setting(cx, "sparkgap end arrow")?;
    let gap = number(cx, "sparkgap/gap", 0.05);
    let tip = match tip.trim() {
        "" | "default" if (gap - 0.05).abs() < 1e-9 => return Ok(None),
        "" | "default" => "Triangle[scale=2]".to_string(),
        t => t.to_string(),
    };
    let mut code = String::new();
    for side in [1.0, -1.0] {
        let ops = [Op::Move(0.3 * side, 0.0), Op::Line(0.3 * gap * side, 0.0)];
        code.push_str(&format!("\\draw[-{{{tip}}},solid,line width={}cm]{};", num(look.thin), look.path(&ops)));
    }
    Ok(Some(code))
}

/// The instruments whose screen, needle or dial stays upright however they are turned, unless
/// `rotated instruments`: what they draw past their outline (their first two parts).
const STRAIGHT: &[&str] = &["oscope", "rmeter", "rmeterwa", "smeter"];

/// A circuitikz setting read as a number, or `None` when it is not set (empty or `none`).
fn optional(cx: &mut Context, key: &str) -> Option<f64> {
    match setting(cx, key) {
        Ok(s) if !s.is_empty() && s != "none" => cx.number(Tokens::text(&s)).ok(),
        _ => None,
    }
}

/// A circuitikz setting read as a length, in cm; `default` when it is not one.
fn distance(cx: &mut Context, key: &str, default: f64) -> f64 {
    match setting(cx, key) {
        Ok(s) if !s.is_empty() => cx.length(Tokens::text(&s)).unwrap_or(default),
        _ => default,
    }
}

/// The `fill` given to the `to` (or its path), as the option of a filled part: `None` without one.
fn fill(cx: &Context) -> Option<String> {
    let f = cx.macro_text("voltik@fill");
    match f.as_str() {
        "" | "none" => None,
        "\\pgfkeysnovalue" => Some("fill".into()),
        c => Some(format!("fill={{{c}}}")),
    }
}

/// What a component's parts are drawn with: where it is, how its default size stretches (`x`, `y`)
/// and its strokes, in cm.
#[derive(Clone)]
struct Look<'a> {
    placed: &'a Placed,
    frame: &'a Frame,
    x: f64,
    y: f64,
    /// cm to `bipoles/length`, at the class's scale
    unit: f64,
    thick: f64,
    modifier: f64,
    thin: f64,
    fill: Option<String>,
}

impl Look<'_> {
    /// A point of the component's default frame, as a picture coordinate.
    fn at(&self, p: [f64; 2]) -> String {
        self.frame.coordinate(self.canvas(p))
    }

    fn canvas(&self, p: [f64; 2]) -> Point {
        self.placed.at(p[0] * self.x, p[1] * self.y)
    }

    /// Ops of the default frame as a path.
    fn path(&self, ops: &[Op]) -> String {
        self.path_scaled(ops, [self.x, self.y])
    }

    /// Ops already in cm (a zig-zag, a coil) as a path.
    fn path_cm(&self, ops: &[Op]) -> String {
        self.path_scaled(ops, [1.0, 1.0])
    }

    /// Ops of the default frame as a path that keeps upright around `center`, wherever the
    /// component turns: a three-phase symbol, which TeX does not turn with its source.
    fn path_upright(&self, ops: &[Op], center: [f64; 2]) -> String {
        let c = self.canvas(center);
        let at = |p: [f64; 2]| self.frame.coordinate(Point::new(c.x + (p[0] - center[0]) * self.x, c.y + (p[1] - center[1]) * self.y));
        let mut s = String::new();
        for op in ops {
            match *op {
                Op::Move(x, y) => s.push_str(&format!(" {}", at([x, y]))),
                Op::Line(x, y) => s.push_str(&format!(" -- {}", at([x, y]))),
                Op::Curve(c1, c2, p) => s.push_str(&format!(" .. controls {} and {} .. {}", at(c1), at(c2), at(p))),
                Op::Close => s.push_str(" -- cycle"),
            }
        }
        s
    }

    fn path_scaled(&self, ops: &[Op], k: [f64; 2]) -> String {
        let at = |p: [f64; 2]| self.frame.coordinate(self.placed.at(p[0] * k[0], p[1] * k[1]));
        let mut s = String::new();
        for op in ops {
            match *op {
                Op::Move(x, y) => s.push_str(&format!(" {}", at([x, y]))),
                Op::Line(x, y) => s.push_str(&format!(" -- {}", at([x, y]))),
                Op::Curve(c1, c2, p) => s.push_str(&format!(" .. controls {} and {} .. {}", at(c1), at(c2), at(p))),
                Op::Close => s.push_str(" -- cycle"),
            }
        }
        s
    }
}

/// A dash pattern as `\\pgfsetdash` has it (`{{1pt}{2pt}}`), as the value of `dash pattern`.
fn dash_pattern(spec: &str) -> String {
    let lengths: Vec<&str> = spec.split(['{', '}']).map(str::trim).filter(|p| !p.is_empty()).collect();
    lengths.chunks(2).map(|c| format!("on {} off {}", c[0], c.get(1).unwrap_or(&c[0]))).collect::<Vec<_>>().join(" ")
}

/// A sharp corner for each rounded one of a plot, or rounded `k` times as much.
fn rounded_plot(ops: &[Op], k: f64) -> Vec<Op> {
    let mut out: Vec<Op> = Vec::new();
    let mut last = [0.0, 0.0];
    for op in ops {
        match *op {
            Op::Move(x, y) | Op::Line(x, y) => {
                out.push(*op);
                last = [x, y];
            }
            Op::Curve(a, b, c) => {
                // where the lines into and out of the corner meet
                let (d1, d2) = ([a[0] - last[0], a[1] - last[1]], [c[0] - b[0], c[1] - b[1]]);
                let det = d1[0] * d2[1] - d1[1] * d2[0];
                let t = if det.abs() < 1e-12 { 0.5 } else { ((c[0] - last[0]) * d2[1] - (c[1] - last[1]) * d2[0]) / det };
                let corner = [last[0] + t * d1[0], last[1] + t * d1[1]];
                let near = |p: [f64; 2]| [corner[0] + k * (p[0] - corner[0]), corner[1] + k * (p[1] - corner[1])];
                if k < 1e-9 {
                    out.push(Op::Line(corner[0], corner[1]));
                } else {
                    let start = near(last);
                    if let Some(Op::Line(x, y) | Op::Move(x, y)) = out.last_mut() {
                        (*x, *y) = (start[0], start[1]);
                    }
                    out.push(Op::Curve(near(a), near(b), near(c)));
                }
                last = c;
            }
            Op::Close => out.push(Op::Close),
        }
    }
    out
}

/// A block's part when its settings change it: a filter's grid and plot, and the lower line of a
/// DC symbol, cut in `blocks dc segments` with gaps as long.
fn block_part(cx: &mut Context, look: &Look, comp: &components::Component, part: Part) -> Result<Option<String>, Error> {
    let stroke = |width: f64, extra: &str| format!("\\draw[-,solid,{extra}miter limit=10,line width={}cm]", num(width));
    let plotted = comp.parts.iter().any(|p| matches!(p, Part::Styled(_, style, _) if style.starts_with("draw opacity")));
    match part {
        Part::Styled(_, style, ops) if style.starts_with("draw opacity") => {
            let opacity = number(cx, "blocks/filter grid/opacity", 0.3);
            if opacity <= 0.0 {
                return Ok(Some(String::new()));
            }
            let mut extra = format!("draw opacity={opacity},");
            match setting(cx, "blocks/filter grid/color")?.trim() {
                "" | "default" => {}
                c => extra.push_str(&format!("color={{{c}}},")),
            }
            match setting(cx, "blocks/filter grid/dash")?.trim() {
                "none" | "default" => {}
                d => extra.push_str(&format!("dash pattern={{{}}},", dash_pattern(d))),
            }
            let width = number(cx, "blocks/filter grid/thickness", 0.5) * look.thin;
            Ok(Some(format!("{}{};", stroke(width, &format!("fill=none,{extra}")), look.path(ops))))
        }
        Part::Line(ops) if plotted && ops.len() != 5 => {
            let width = number(cx, "blocks/filter plot/relative thickness", 2.0) * look.thin;
            let k = number(cx, "blocks/filter plot/rounding", 0.15) / 0.15;
            let plot = if (k - 1.0).abs() < 1e-9 { ops.to_vec() } else { rounded_plot(ops, k) };
            Ok(Some(format!("{}{};", stroke(width, "fill=none,"), look.path(&plot))))
        }
        Part::Thin(ops) if matches!(comp.name, "sdcdc" | "sacdc" | "sdcac") => {
            let [Op::Move(a, y), Op::Line(b, _)] = *ops else { return Ok(None) };
            let side = if (y - 0.1313).abs() < 1e-3 {
                "in"
            } else if (y + 0.2187).abs() < 1e-3 {
                "out"
            } else {
                return Ok(None);
            };
            let all = number(cx, "blocks dc segments", 1.0);
            let count = optional(cx, &format!("blocks dc {side} segments")).unwrap_or(all).round().max(1.0) as usize;
            if count == 1 {
                return Ok(None);
            }
            let step = (b - a) / (2 * count - 1) as f64;
            let pieces: Vec<Op> = (0..count).flat_map(|i| [Op::Move(a + 2.0 * i as f64 * step, y), Op::Line(a + (2 * i + 1) as f64 * step, y)]).collect();
            Ok(Some(format!("{}{};", stroke(look.thin, "fill=none,"), look.path(&pieces))))
        }
        _ => Ok(None),
    }
}

/// A block on a dashed path: its box and diagonal dashed by `dashed blocks pattern`, and what is
/// inside too with `inner blocks dashed`.
fn dashed_block(cx: &mut Context, part: Part, drawn: String) -> Result<String, Error> {
    if !cx.is_dashed() {
        return Ok(drawn);
    }
    let outer = match part {
        Part::Line(ops) => ops.len() == 5 && matches!(ops[4], Op::Close),
        Part::Thin(ops) => matches!(ops[..], [Op::Move(x, _), Op::Line(x2, _)] if (x + 0.35).abs() < 1e-3 && (x2 - 0.35).abs() < 1e-3),
        _ => false,
    };
    if !outer && setting(cx, "blocks/inner dashed")? != "true" {
        return Ok(drawn);
    }
    let pattern = dash_pattern(&setting(cx, "dashed blocks pattern")?);
    Ok(drawn.replacen("-,solid,", &format!("-,dash pattern={{{pattern}}},"), 1))
}

/// Whether a part goes around the middle of a source of radius 0.3: its circle.
fn is_ring(ops: &[Op]) -> bool {
    ops.iter().all(|o| match *o {
        Op::Move(x, y) | Op::Line(x, y) | Op::Curve(_, _, [x, y]) => (x.hypot(y) - 0.3).abs() < 0.01,
        Op::Close => true,
    })
}

/// A current source's circle with its gaps, `bipoles/…/angle` from its axis, when it is not as measured.
fn open_circle(cx: &mut Context, comp: &components::Component) -> Option<Vec<Op>> {
    let (key, default) = match comp.name {
        "dcisource" => ("bipoles/dcisource/angle", 80.0),
        "sinusoidal current source" => ("bipoles/isourcesin/angle", 90.0),
        _ => return None,
    };
    let angle: f64 = number(cx, key, default);
    if (angle - default).abs() < 1e-9 {
        return None;
    }
    let r = 0.3;
    let mut ops = Vec::new();
    for (from, to) in [(angle, -angle), (180.0 - angle, 180.0 + angle)] {
        let pieces = (f64::abs(to - from) / 45.0).ceil().max(1.0);
        let step = ((to - from) / pieces).to_radians();
        let k = 4.0 / 3.0 * (step / 4.0).tan() * r;
        for i in 0..pieces as usize {
            let a = from.to_radians() + i as f64 * step;
            let (p, q) = ([r * a.cos(), r * a.sin()], [r * (a + step).cos(), r * (a + step).sin()]);
            if i == 0 {
                ops.push(Op::Move(p[0], p[1]));
            }
            ops.push(Op::Curve([p[0] - k * a.sin(), p[1] + k * a.cos()], [q[0] + k * (a + step).sin(), q[1] - k * (a + step).cos()], q));
        }
    }
    Some(ops)
}

/// A square meter's index, as measured: the center it turns around, the degrees of each unit of
/// its `index position` and the key.
fn index_center(name: &str) -> Option<([f64; 2], f64, &'static str)> {
    match name {
        "smeter" => Some(([0.0, -0.5385], 0.3, "bipoles/smeter/index position")),
        "qiprobe" | "qvprobe" | "qpprobe" => Some(([0.0, -0.679], 0.26, "bipoles/qmeter/index position")),
        _ => None,
    }
}

/// Half the distance between an American voltage source's signs at `margin=1`, as measured.
const SIGN_DISTANCE: f64 = 0.18;

/// An American voltage source's sign in `text` when its look changes: its text, how far from the
/// middle it goes and its rotation; `None` to draw it as measured.
fn polarity_sign(cx: &mut Context, comp: &components::Component, text: &str) -> Option<(String, f64, String)> {
    let base = match comp.name {
        "american voltage source" => "bipoles/vsourceam",
        "american controlled voltage source" => "bipoles/cvsourceam",
        _ => return None,
    };
    let sign = match text {
        "+" => "inner plus",
        "$-$" => "inner minus",
        _ => return None,
    };
    let inner = setting(cx, &format!("{base}/{sign}")).unwrap_or_default();
    let rotation = setting(cx, "sources/symbol/sign rotation").unwrap_or_default();
    let margin = number(cx, &format!("{base}/margin"), 0.7);
    let plain = inner.trim().is_empty() && matches!(rotation.trim(), "" | "default") && (margin - 0.7).abs() < 1e-9;
    let shown = if inner.trim().is_empty() { text.to_string() } else { inner };
    (!plain).then_some((shown, SIGN_DISTANCE * margin, rotation))
}

/// The TeX that draws one part of a component.
fn draw_part(cx: &mut Context, look: &Look, comp: &components::Component, part: Part) -> String {
    let stroke = |width: f64, extra: &str| format!("\\draw[-,solid,{extra}miter limit=10,line width={}cm]", num(width));
    match part {
        Part::Fill(ops) => match &look.fill {
            Some(f) => format!("\\path[{f}]{};", look.path(ops)),
            None => String::new(),
        },
        Part::Body(ops) => {
            let fill = look.fill.clone().unwrap_or_else(|| "fill=none".into());
            format!("{}{};", stroke(look.thick, &format!("{fill},")), look.path(ops))
        }
        // a current source's circle broken by `bipoles/…/angle`
        Part::Line(ops) if is_ring(ops) && open_circle(cx, comp).is_some() => {
            let ops = open_circle(cx, comp).unwrap_or_default();
            format!("{}{};", stroke(look.thick, "fill=none,"), look.path(&ops))
        }
        Part::Line(ops) => format!("{}{};", stroke(look.thick, "fill=none,"), look.path(ops)),
        // a meter's index at `index position` of its scale
        Part::Thin(ops) if index_center(comp.name).is_some() && matches!(ops[ops.len().saturating_sub(2)..], [Op::Move(..), Op::Line(..)]) && ops.len() > 2 => {
            let (center, per_unit, key) = index_center(comp.name).unwrap_or(([0.0, 0.0], 0.0, ""));
            let turn = ((number(cx, key, 80.0) - 80.0) * per_unit).to_radians();
            let (sin, cos) = (-turn).sin_cos();
            let round = |x: f64, y: f64| {
                let (dx, dy) = (x - center[0], y - center[1]);
                (center[0] + dx * cos - dy * sin, center[1] + dx * sin + dy * cos)
            };
            let mut moved = ops.to_vec();
            let last = moved.len() - 2;
            for op in &mut moved[last..] {
                *op = match *op {
                    Op::Move(x, y) => {
                        let (x, y) = round(x, y);
                        Op::Move(x, y)
                    }
                    Op::Line(x, y) => {
                        let (x, y) = round(x, y);
                        Op::Line(x, y)
                    }
                    other => other,
                };
            }
            format!("{}{};", stroke(look.thin, "fill=none,"), look.path(&moved))
        }
        Part::ChokeCore(ops) => {
            let width = look.thick * number(cx, "bipoles/cutechoke/cthick", 1.0);
            let line = format!("{}{};", stroke(width, "fill=none,"), look.path(ops));
            if number(cx, "bipoles/cutechoke/lines", 1.0) < 2.0 {
                line
            } else {
                // (the second one 0.0627 cm, measured, above the first)
                let way = look.canvas([0.0, 1.0]).minus(look.canvas([0.0, 0.0]));
                let up = look.frame.vector(way.scale(CHOKE_LINES / way.norm().max(1e-9)));
                format!("{line}\\begin{{scope}}[shift={{{up}}}]{line}\\end{{scope}}")
            }
        }
        Part::Modifier(ops) => format!("{}{};", stroke(look.modifier, "fill=none,"), look.path(ops)),
        Part::Stroke(k, ops) => format!("{}{};", stroke(k / 2.0 * look.thick, "fill=none,"), look.path(ops)),
        Part::Faint(k, ops) => format!("{}{};", stroke(k / 2.0 * look.thick, "fill=none,draw opacity=0.5,"), look.path(ops)),
        Part::Weighted(key, ops) => format!("{}{};", stroke(look.thick * number(cx, key, 1.0), "fill=none,"), look.path(ops)),
        Part::Waveform => {
            let wave = setting(cx, "bipoles/oscope/waveform").unwrap_or_default();
            match components::WAVEFORMS.iter().find(|(name, _)| *name == wave.trim()) {
                Some((_, [])) => String::new(),
                Some((_, ops)) => format!("{}{};", stroke(look.thick, "fill=none,"), look.path(ops)),
                // one of the user's (`bipoles/oscope/waveform/mywave/.code`), of its box from -1cm to 1cm
                None if cx
                    .expand(Tokens::text(&format!("\\ifcsname pgfk@/tikz/circuitikz/bipoles/oscope/waveform/{}/.@cmd\\endcsname1\\fi", wave.trim())))
                    .unwrap_or_default()
                    .trim()
                    == "1" =>
                {
                    let rotate = if setting(cx, "instruments/rotated").unwrap_or_default() == "true" { look.placed.angle } else { 0.0 };
                    format!(
                        "\\begin{{scope}}[shift={{{}}},rotate={rotate},xscale={},yscale={},line width={}cm]\\pgfkeys{{/tikz/circuitikz/bipoles/oscope/waveform/{}}}\\end{{scope}}",
                        look.at([0.0, 0.0]),
                        num(0.3 * look.x),
                        num(0.3 * look.y),
                        num(look.thick),
                        wave.trim()
                    )
                }
                None => {
                    cx.warn(format!("voltik: oscilloscope waveform {} not drawn yet; drawn as ramps", wave.trim()));
                    format!("{}{};", stroke(look.thick, "fill=none,"), look.path(components::WAVEFORMS[0].1))
                }
            }
        }
        Part::Paint(color, ops) => format!("\\path[fill={{{color}}}]{};", look.path(ops)),
        Part::Pole(ops) => format!("\\path[{}]{};", look.fill.clone().unwrap_or_else(|| "fill=white".into()), look.path(ops)),
        Part::Channel(ops) => match &look.fill {
            Some(f) => format!("\\path[{f}]{};", look.path(ops)),
            None => format!("\\fill[color=.]{};", look.path(ops)),
        },
        Part::Lever(k, ops) => {
            let k = k * number(cx, "bipoles/cuteswitch/thickness", 1.0);
            format!("{}{};", stroke(k * look.thin, "fill=none,line cap=round,"), look.path(ops))
        }
        Part::Thin(ops) => format!("{}{};", stroke(look.thin, "fill=none,"), look.path(ops)),
        Part::Styled(k, style, ops) => format!("{}{};", stroke(k * look.thin, &format!("fill=none,{style},")), look.path(ops)),
        Part::Solid(ops) => format!("\\fill[color=.]{};", look.path(ops)),
        // a variable component's arrow, left to right across it: up it with `bipoles/fix tunable
        // direction` (the default); without, as before 1.8: the American resistor's reversed, the
        // inductors' the same, the others' down it. Its end is `tunable end arrow` when set
        Part::Arrow { from, to, thin: false } if from[0] < 0.0 && to[0] > 0.0 => {
            let (from, to) = match setting(cx, "bipoles/fix tunable direction").unwrap_or_default().as_str() {
                "false" if comp.name == "variable american resistor" => (to, from),
                "false" if comp.class != components::Class::Inductors => ([from[0], -from[1]], [to[0], -to[1]]),
                _ => (from, to),
            };
            match arrow_look(cx, "tunable", None, "latexslim").unwrap_or(None) {
                None => arrow(look, from, to, look.modifier),
                Some(tips) => format!("\\draw[{}] {} -- {};", tips.draw(look.modifier), look.at(from), look.at(to)),
            }
        }
        Part::Arrow { from, to, thin } => arrow(look, from, to, if thin { look.thin } else { look.modifier }),
        Part::Wiper { thin } => {
            let pos = number(cx, "wiper pos", 0.5);
            let x = (pos - 0.5) * comp.width;
            let top = comp.height / 2.0;
            let width = if thin { look.thin } else { look.modifier };
            match arrow_look(cx, "wiper", None, "latexslim").unwrap_or(None) {
                None => arrow(look, [x, top + 0.25], [x, top], width),
                Some(tips) => format!("\\draw[{}] {} -- {};", tips.draw(width), look.at([x, top + 0.25]), look.at([x, top])),
            }
        }
        Part::ZigZag { half, high } => {
            let count = number(cx, "resistors/zigs", 3.0).round().max(1.0) as usize;
            let ops = components::zigzag(half * look.x, high * look.y, look.thick, count);
            format!("{}{};", stroke(look.thick, "line join=bevel,fill=none,"), look.path_cm(&ops))
        }
        Part::Coil | Part::Arches => {
            let (default, least) = if part == Part::Coil { (5.0, 2.0) } else { (4.0, 1.0) };
            let count = optional(cx, "inductors/coils").unwrap_or(default).round().max(least) as usize;
            let (half, h) = (comp.width * look.x / 2.0 + look.thick / 2.0, comp.height * look.y / 2.0);
            let ops = if part == Part::Coil { components::coil(half, h, count) } else { components::arches(half, h, count) };
            format!("{}{};", stroke(look.thick, "line join=bevel,fill=none,"), look.path_cm(&ops))
        }
        // an American voltage source's signs, of other texts, margin or rotation
        Part::Text { text, .. } if polarity_sign(cx, comp, text).is_some() => {
            let (sign, distance, rotation) = polarity_sign(cx, comp, text).unwrap_or_default();
            let side = if text == "+" { 1.0 } else { -1.0 };
            let angle = look.placed.angle.rem_euclid(180.0);
            let leaning = angle.min(180.0 - angle);
            let rotate = match rotation.trim() {
                "" | "default" => look.placed.angle + 90.0,
                "straight" => 0.0,
                "auto" if leaning < 45.0 => look.placed.angle,
                "auto" => look.placed.angle + 90.0,
                n => look.placed.angle + cx.number(Tokens::text(n)).unwrap_or(90.0),
            };
            format!("\\node[anchor=center,inner sep=0pt,rotate={rotate}] at {} {{{sign}}};", look.at([side * distance, 0.0]))
        }
        Part::Text { at, shift, size, text, turn } => {
            let at = [at[0] + shift[0] * look.unit / look.x.max(1e-9), at[1] + shift[1] * look.unit / look.y.max(1e-9)];
            // turned with the line, and with the component's mirror and inversion
            let base = look.placed.angle + turn;
            // upright text keeps its middle where it is on the component, wherever that turns
            let point = if turn == 0.0 {
                let pt = tikra::extension::PT;
                let chars = text.chars().filter(|c| c.is_alphanumeric()).count().max(1) as f64;
                let half = Point { x: 0.3 * size * chars * pt, y: 0.34 * size * pt };
                let middle = look.canvas([at[0] + half.x / look.x.max(1e-9), at[1] + half.y / look.y.max(1e-9)]);
                look.frame.coordinate(middle.minus(half))
            } else if look.placed.mirror || look.placed.invert {
                // a turned text flipped with the component keeps its middle where it is flipped to
                let pt = tikra::extension::PT;
                let wide = if text.contains(['+', '-']) { 0.389 } else { 0.3 * text.chars().filter(|c| c.is_alphanumeric()).count().max(1) as f64 };
                let (sin, cos) = turn.to_radians().sin_cos();
                let (hx, hy) = (wide * size * pt, 0.25 * size * pt);
                let middle = [at[0] + (hx * cos - hy * sin) / look.x.max(1e-9), at[1] + (hx * sin + hy * cos) / look.y.max(1e-9)];
                return format!("\\node[anchor=center,inner sep=0pt,rotate={base},font=\\fontsize{{{size}}}{{{}}}\\selectfont] at {} {{{text}}};", size * 1.2, look.at(middle));
            } else {
                look.at(at)
            };
            format!(
                "\\node[anchor=base west,inner sep=0pt,rotate={},font=\\fontsize{{{size}}}{{{}}}\\selectfont] at {} {{{text}}};",
                if turn == 0.0 { 0.0 } else { base },
                size * 1.2,
                point
            )
        }
    }
}

/// The anchors of a component named `name` (`to[R, name=R1]`), `R1.north`, `R1.wiper`…: the
/// compass points of its band, and its own named points.
fn anchors(cx: &mut Context, look: &Look, comp: &components::Component, name: &str) -> String {
    let (w, top, bottom) = (comp.width / 2.0, comp.top, -comp.bottom);
    let mut points: Vec<(&str, [f64; 2])> = vec![
        ("center", [0.0, 0.0]),
        ("north", [0.0, top]),
        ("south", [0.0, bottom]),
        ("east", [w, 0.0]),
        ("west", [-w, 0.0]),
        ("right", [w, 0.0]),
        ("left", [-w, 0.0]),
        ("north east", [w, top]),
        ("north west", [-w, top]),
        ("south east", [w, bottom]),
        ("south west", [-w, bottom]),
        ("n", [0.0, top]),
        ("s", [0.0, bottom]),
        ("e", [w, 0.0]),
        ("w", [-w, 0.0]),
        ("ne", [w, top]),
        ("nw", [-w, top]),
        ("se", [w, bottom]),
        ("sw", [-w, bottom]),
    ];
    points.extend(comp.anchors.iter().copied());
    // a block's lateral anchors, `block left anchors pos` (and right) of the way up to its top
    if comp.class == components::Class::Blocks {
        let left = number(cx, "block left anchors pos", 0.5);
        let right = number(cx, "block right anchors pos", 0.5);
        for (name, at) in points.iter_mut() {
            match *name {
                "left up" => at[1] = left * top,
                "left down" => at[1] = left * bottom,
                "right up" => at[1] = right * top,
                "right down" => at[1] = right * bottom,
                _ => {}
            }
        }
    }
    points.extend(winding_anchors(cx, comp));
    // a current tap's tap, at the bottom of its dot (`bipoles/currtap/dot size`)
    if comp.name == "currtap" {
        let r = 0.2 * number(cx, "bipoles/currtap/dot size", 0.5);
        points.retain(|(n, _)| *n != "tap");
        points.push(("tap", [0.0, -r]));
    }
    if comp.parts.iter().any(|p| matches!(p, Part::Wiper { .. })) {
        let x = (number(cx, "wiper pos", 0.5) - 0.5) * comp.width;
        points.extend([("wiper", [x, comp.height / 2.0 + 0.25]), ("tip", [x, comp.height / 2.0])]);
    }
    if comp.class == components::Class::Inductors {
        // the core lines go `core distance` above the coil; the dots of mutual inductance a little
        // beyond its corners; the middle tap is on the coil, above or below as its coils fall
        let unit = look.y.max(1e-9);
        let core = distance(cx, "bipoles/inductors/core distance", 2.0 * PT) / unit;
        let dx = distance(cx, "bipoles/inductors/dot x distance", 4.0 * PT) / unit;
        let dy = distance(cx, "bipoles/inductors/dot y distance", PT) / unit;
        let h = comp.height / 2.0;
        points.extend([("core west", [-w, h + core]), ("core east", [w, h + core])]);
        let (x, up, down) = (w * 1.1 + dx, comp.top * 1.1 + dy, -(comp.bottom * 1.1 + dy));
        points.extend([("ul dot", [-x, up]), ("ur dot", [x, up]), ("ll dot", [-x, down]), ("lr dot", [x, down])]);
        let coils = comp.parts.iter().find_map(|p| match p {
            Part::Coil => Some(optional(cx, "inductors/coils").unwrap_or(5.0).round()),
            Part::Arches => Some(optional(cx, "inductors/coils").unwrap_or(4.0).round()),
            _ => None,
        });
        points.push(("midtap", [0.0, midtap(comp, coils, h)]));
    }
    // a node of its own, as in CircuiTikZ: `(R1)` is its center, and a name prefix is its
    let center = look.canvas([0.0, 0.0]);
    let mut list = String::new();
    for (anchor, p) in points {
        let q = look.canvas(p).minus(center);
        list.push_str(&format!("{anchor}={},{};", num(q.x), num(q.y)));
    }
    format!("\\def\\voltik@named{{{list}}}\\node[shape=voltik named, name={{{name}}}, inner sep=0pt, outer sep=0pt, anchor=center] at {} {{}};", look.at([0.0, 0.0]))
}

/// An arrow from `from` to `to` (default frame) with its head at `to`, for a stroke `width` cm wide.
fn arrow(look: &Look, from: [f64; 2], to: [f64; 2], width: f64) -> String {
    let (p0, tip) = (look.canvas(from), look.canvas(to));
    let d = tip.minus(p0);
    let u = d.scale(1.0 / d.norm().max(1e-9));
    let n = Point::new(-u.y, u.x);
    let size = components::arrow_length(width);
    let head = |b: f64, s: f64| look.frame.coordinate(tip.minus(u.scale(b * size)).plus(n.scale(s * size)));
    let mut s = format!(
        "\\draw[-,solid,line width={}cm] {} -- {};\\fill[color=.] {}",
        num(width),
        look.frame.coordinate(p0),
        look.frame.coordinate(tip.minus(u.scale(components::ARROW_INSET * size))),
        look.frame.coordinate(tip)
    );
    for c in components::ARROW_HEAD.chunks(3) {
        s.push_str(&format!(" .. controls {} and {} .. {}", head(c[0][0], c[0][1]), head(c[1][0], c[1][1]), head(c[2][0], c[2][1])));
    }
    s.push_str(" -- cycle;");
    s
}

/// Where an inductor's tap is, across it: over a coil's middle turn if it has one, else between two.
fn midtap(comp: &crate::components::Component, coils: Option<f64>, h: f64) -> f64 {
    match coils {
        Some(n) if n as i64 % 2 == 1 => h,
        Some(_) if comp.parts.contains(&Part::Coil) => -comp.bottom,
        Some(_) => 0.0,
        None => comp.top,
    }
}

/// The tap of a transformer's coil, of `kind|coils` (`cute|6`, no number for the default).
pub fn coil_midtap(spec: &str) -> f64 {
    let (kind, coils) = spec.split_once('|').unwrap_or((spec, ""));
    let Some(comp) = crate::components::find(&format!("{kind} inductor")) else { return 0.0 };
    let coils = comp.parts.iter().find_map(|p| match p {
        Part::Coil => Some(coils.trim().parse().unwrap_or(5.0)),
        Part::Arches => Some(coils.trim().parse().unwrap_or(4.0)),
        _ => None,
    });
    midtap(comp, coils, comp.height / 2.0)
}

/// The arrow into a block (`to[amp, >]`): its length and half its width, in `bipoles/length`, a
/// plain triangle with its tip on the block.
const BLOCK_ARROW: [f64; 2] = [0.1057, 0.0498];
/// How far a cute choke's second core line is from the first, in cm.
const CHOKE_LINES: f64 = 0.0627;

/// The poles at the ends: circles, diamonds or squares `nodes width` (of `bipoles/length`) across
/// the half, drawn with the wire's stroke and filled with its color, or the open ones (`ocirc`) with
/// the component's fill (white without one).
fn poles(cx: &mut Context, frame: &Frame, start: Point, end: Point, unit: f64) -> String {
    // (with `transform shape` the node scales by itself)
    let unit = if cx.macro_text("voltik@ts") == "1" { unit / cx.transform().factor() } else { unit };
    let half = number(cx, "nodes width", 0.04) * unit;
    // the filled poles take the stroke's color; the open ones the component's fill, or white
    let closed = "color=.,fill";
    let open_fill = setting(cx, "open poles fill").unwrap_or_default();
    let opacity = number(cx, "poles/open fill opacity", 1.0);
    let open = format!("{},fill opacity={}", fill(cx).unwrap_or_else(|| format!("fill={{{}}}", open_fill.trim())), num(opacity));
    let mut s = String::new();
    for (which, at) in [("voltik@ps", start), ("voltik@pe", end)] {
        let spec = cx.macro_text(which);
        let (name, options) = spec.split_once(',').map_or((spec.trim(), ""), |(n, o)| (n.trim(), o.trim()));
        let (shape, size, fill) = match name {
            "circ" => ("circle", 2.0 * half, closed),
            "ocirc" => ("circle", 2.0 * half, open.as_str()),
            "squarepole" => ("rectangle", 2.0 * half, closed),
            "osquarepole" => ("rectangle", 2.0 * half, open.as_str()),
            "diamondpole" => ("rectangle,rotate=45", 2.0 * half, closed),
            "odiamondpole" => ("rectangle,rotate=45", 2.0 * half, open.as_str()),
            "" | "none" => continue,
            // a style of the user's, or a node's shape: a node of it
            other => {
                let options = if options.is_empty() { String::new() } else { format!(",{options}") };
                s.push_str(&format!(" node[{other},draw,solid,inner sep=0pt,outer sep=0pt,minimum size={}cm{options}] at {} {{}}", num(2.0 * half), frame.coordinate(at)));
                continue;
            }
        };
        let options = if options.is_empty() { String::new() } else { format!(",{options}") };
        s.push_str(&format!(" node[{shape},draw,solid,{fill},inner sep=0pt,outer sep=0pt,minimum size={}cm{options}] at {} {{}}", num(size), frame.coordinate(at)));
    }
    s
}

/// A winding: its name, its symbol's point and how much bigger its symbol is.
type Winding = (&'static str, [f64; 2], f64);

/// The windings of the transformer sources (`oosourcetrans`, `oosourceatrans`, `ooosource`): their
/// symbol's point and how much bigger than the others it is drawn.
const WINDINGS: &[(&str, &[Winding])] = &[
    ("oosourcetrans", &[("prim", [-0.18, 0.0], 1.0), ("sec", [0.18, 0.0], 1.0)]),
    ("oosourcetrans large", &[("prim", [-0.1494, 0.0], 1.0), ("sec", [0.1494, 0.0], 1.0)]),
    ("oosourceatrans", &[("prim", [0.07, 0.0], 1.2)]),
    ("ooosource", &[("prim", [-0.1537, 0.0], 1.0), ("sec", [0.135, 0.1667], 1.0), ("tert", [0.135, -0.1667], 1.0)]),
];

/// The extra winding above a transformer source (`oo upper winding`): its circle's center, and its
/// radius at size 1, in `bipoles/length`.
const UPPER: &[(&str, [f64; 2])] = &[("oosourcetrans", [0.0, 0.234]), ("oosourcetrans large", [0.0, 0.234]), ("oosourceatrans", [0.07, 0.234])];
const UPPER_RADIUS: f64 = 0.18;

/// A three-phase symbol's size: the radius it reaches, in `bipoles/length`.
const SYMBOL: f64 = 0.0537;

/// The inner triangle of `xdelta`, as a share of the symbol.
const XDELTA_SPLIT: f64 = 0.5811;

/// The size of the extra winding (`oo upper winding=…`, or its default), 0 without one.
fn upper_size(cx: &mut Context) -> f64 {
    match setting(cx, "bipoles/oosourcetrans/upper").unwrap_or_default().trim() {
        "" | "none" => 0.0,
        "default" => number(cx, "bipoles/oosourcetrans/upper circle size default", 0.7),
        v => cx.number(Tokens::text(v)).unwrap_or(0.7),
    }
}

/// The three-phase symbols of a transformer source's windings (`prim=delta`, `sec=wye`,
/// `tert=zig`, `upper=eyw`), and its extra winding's circle.
fn windings(cx: &mut Context, look: &Look, comp: &components::Component, scale: f64) -> String {
    let mut code = String::new();
    let mut symbols: Vec<Winding> = WINDINGS.iter().find(|(n, _)| *n == comp.name).map_or(Vec::new(), |(_, w)| w.to_vec());
    if let Some((_, c)) = UPPER.iter().find(|(n, _)| *n == comp.name) {
        let size = upper_size(cx);
        if size > 0.0 {
            let r = UPPER_RADIUS * size;
            let k = 0.5523 * r;
            let circle = [
                Op::Move(c[0] + r, c[1]),
                Op::Curve([c[0] + r, c[1] + k], [c[0] + k, c[1] + r], [c[0], c[1] + r]),
                Op::Curve([c[0] - k, c[1] + r], [c[0] - r, c[1] + k], [c[0] - r, c[1]]),
                Op::Curve([c[0] - r, c[1] - k], [c[0] - k, c[1] - r], [c[0], c[1] - r]),
                Op::Curve([c[0] + k, c[1] - r], [c[0] + r, c[1] - k], [c[0] + r, c[1]]),
                Op::Close,
            ];
            if let Some(f) = &look.fill {
                code.push_str(&format!("\\path[{f}]{};", look.path(&circle)));
            }
            code.push_str(&format!("\\draw[-,solid,fill=none,miter limit=10,line width={}cm]{};", num(look.thick), look.path(&circle)));
            symbols.push(("upper", [c[0], c[1] + SYMBOL * (2.0 * size - 1.0)], 1.0));
        }
    }
    let width = look.thick * number(cx, "sources/symbol/thickness", 1.0);
    for (winding, at, big) in symbols {
        let kind = setting(cx, &format!("bipoles/oosource/{winding}")).unwrap_or_default();
        let kind = kind.trim();
        if kind.is_empty() || kind == "none" {
            continue;
        }
        if !matches!(kind, "delta" | "wye" | "eyw" | "xdelta" | "zig") {
            cx.warn(format!("voltik: three-phase symbol {kind} not drawn"));
            continue;
        }
        let r = SYMBOL * big * scale * number(cx, &format!("sources/symbol/{kind} scale"), 1.0);
        let turn = number(cx, &format!("sources/symbol/{kind} rot"), 0.0) + number(cx, &format!("sources/symbol/{winding} rot"), 0.0);
        let dir = |a: f64, d: f64| {
            let a = (a + turn).to_radians();
            [at[0] + d * a.cos(), at[1] + d * a.sin()]
        };
        let mut ops: Vec<Op> = Vec::new();
        let line = |ops: &mut Vec<Op>, a: [f64; 2], b: [f64; 2]| ops.extend([Op::Move(a[0], a[1]), Op::Line(b[0], b[1])]);
        match kind {
            "delta" | "xdelta" => {
                let s = if kind == "xdelta" { XDELTA_SPLIT } else { 1.0 };
                let v: Vec<[f64; 2]> = [-90.0, 150.0, 30.0].iter().map(|&a| dir(a, r * s)).collect();
                ops.extend([Op::Move(v[0][0], v[0][1]), Op::Line(v[1][0], v[1][1]), Op::Line(v[2][0], v[2][1]), Op::Close]);
                if kind == "xdelta" {
                    // a tick from each corner, on along the next side
                    for (i, a) in [-90.0f64, 150.0, 30.0].iter().enumerate() {
                        let t = (a + 30.0 + turn).to_radians();
                        let d = r * (1.0 - s);
                        line(&mut ops, v[i], [v[i][0] + d * t.cos(), v[i][1] + d * t.sin()]);
                    }
                }
            }
            "wye" | "eyw" => {
                let arms = if kind == "wye" { [90.0, -30.0, 210.0] } else { [-90.0, 30.0, 150.0] };
                for a in arms {
                    line(&mut ops, at, dir(a, r));
                }
            }
            _ => {
                for a in [-90.0, 30.0, 150.0] {
                    let (m, e) = (dir(a, r / 2.0), dir(a - 30.0, r));
                    ops.extend([Op::Move(at[0], at[1]), Op::Line(m[0], m[1]), Op::Line(e[0], e[1])]);
                }
            }
        }
        code.push_str(&format!("\\draw[-,solid,fill=none,miter limit=10,line width={}cm]{};", num(width), look.path_upright(&ops, at)));
    }
    code
}

/// The anchors of a transformer source's windings: their symbols' points (`symbolprim`) and its
/// extra winding's (`centerupper`, `uppertop`, `topleft`, `topright`, `symbolupper`).
fn winding_anchors(cx: &mut Context, comp: &components::Component) -> Vec<(&'static str, [f64; 2])> {
    let mut points = Vec::new();
    if let Some((_, w)) = WINDINGS.iter().find(|(n, _)| *n == comp.name) {
        for (winding, at, _) in w.iter() {
            let name = match *winding {
                "prim" => "symbolprim",
                "sec" => "symbolsec",
                _ => "symboltert",
            };
            points.push((name, *at));
        }
    }
    if let Some((_, c)) = UPPER.iter().find(|(n, _)| *n == comp.name) {
        let size = match upper_size(cx) {
            0.0 => number(cx, "bipoles/oosourcetrans/upper circle size default", 0.7),
            s => s,
        };
        let r = UPPER_RADIUS * size;
        let d = r * std::f64::consts::FRAC_1_SQRT_2;
        points.extend([
            ("centerupper", *c),
            ("uppertop", [c[0], c[1] + r]),
            ("topright", [c[0] + d, c[1] + d]),
            ("topleft", [c[0] - d, c[1] + d]),
            ("symbolupper", [c[0], c[1] + SYMBOL * (2.0 * size - 1.0)]),
        ]);
    }
    points
}
