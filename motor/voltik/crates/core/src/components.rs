//! The path-style components: their size and their drawing, in a frame of their own.
//!
//! A component is drawn horizontally, centered at the origin, in units of `bipoles/length` (1.4 cm by
//! default, the length of a resistor with its leads); the path turns it. Its *width* is where the
//! leads stop, its *height* is the band that labels keep out of. Its drawing is a list of [`Part`]s
//! at the component's default size, measured on what CircuiTikZ draws; a component made wider or
//! taller (`resistors/width`, `capacitors/height`) stretches them. Thick strokes may go half their
//! width beyond the leads' ends, so that the joint is covered.

/// A segment of a component's outline, in the component's frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Move(f64, f64),
    Line(f64, f64),
    /// A cubic Bézier: two control points, then the end.
    Curve([f64; 2], [f64; 2], [f64; 2]),
    Close,
}

use Op::{Close as Z, Curve as C, Line as L, Move as M};

/// A piece of a component's drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Part {
    /// Filled with the `fill` of the `to`, not stroked (the space between a capacitor's plates).
    Fill(&'static [Op]),
    /// Stroked thick and filled with the `fill` of the `to` (a generic bipole's box).
    Body(&'static [Op]),
    /// Stroked thick, never filled.
    Line(&'static [Op]),
    /// Stroked thick times the class's `modifier thickness`: the marks that tell a component's kind
    /// (a sensor's bar).
    Modifier(&'static [Op]),
    /// Stroked with the path's own line width.
    Thin(&'static [Op]),
    /// Filled with the color of the stroke.
    Solid(&'static [Op]),
    /// An arrow from the first point to the second, with its head there: a modifier stroke, or the
    /// path's own line width when `thin`.
    Arrow { from: [f64; 2], to: [f64; 2], thin: bool },
    /// A potentiometer's wiper: an arrow down to the top of the component, at `wiper pos` along it.
    Wiper { thin: bool },
    /// A zig-zag of `resistors/zigs` teeth, `half` wide and `high` high on each side of the line.
    ZigZag { half: f64, high: f64 },
    /// Arches with small loops between them (the cute inductor), `inductors/coils` of them.
    Coil,
    /// Plain arches side by side (the American inductor), `inductors/coils` of them.
    Arches,
    /// Stroked `k` times as wide as the component's outline (a battery's thick plate, 3).
    Stroke(f64, &'static [Op]),
    /// Filled with a color of its own (a noise source's gray, a fuse's white).
    Paint(&'static str, &'static [Op]),
    /// A pole of the component (a jumper's, a cute switch's): filled with the `fill` of the `to`, or
    /// white.
    Pole(&'static [Op]),
    /// Filled with the `fill` of the `to`, or the line color: a depletion transistor's channel.
    Channel(&'static [Op]),
    /// Stroked `k` times the path's line width with round caps: a cute switch's lever.
    Lever(f64, &'static [Op]),
    /// A cute choke's core: `bipoles/cutechoke/cthick` times the outline's width, doubled by
    /// `twolineschoke`.
    ChokeCore(&'static [Op]),
    /// Stroked `k` times as wide as the component's outline, half transparent (an oscilloscope's
    /// grid).
    Faint(f64, &'static [Op]),
    /// Stroked thick times the setting it names: a microphone's bar (`bipoles/mic/bar thickness`).
    Weighted(&'static str, &'static [Op]),
    /// An oscilloscope's trace, as `bipoles/oscope/waveform` says.
    Waveform,
    /// Text in a font of `size` pt, its baseline starting at the point, `shift` away from it, turned
    /// by `turn` degrees: the point stretches with the component, the shift does not.
    Text { at: [f64; 2], shift: [f64; 2], size: f64, text: &'static str, turn: f64 },
    /// Stroked `k` times the path's line width, with TikZ options of its own (a filter's grid:
    /// `draw opacity=.3, dash pattern=…`).
    Styled(f64, &'static str, &'static [Op]),
}

use Part::{Arrow, Body, Fill, Line, Modifier, Solid, Text, Thin};

/// The classes of the manual: a class gathers components that share `scale`, `width` and `thickness`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Default,
    Resistors,
    Capacitors,
    Inductors,
    Connectors,
    Batteries,
    Sources,
    CSources,
    Instruments,
    Mechanicals,
    Misc,
    Diodes,
    Switches,
    Transistors,
    LogicPorts,
    Blocks,
}

impl Class {
    pub fn name(self) -> &'static str {
        match self {
            Class::Default => "default",
            Class::Resistors => "resistors",
            Class::Capacitors => "capacitors",
            Class::Inductors => "inductors",
            Class::Connectors => "connectors",
            Class::Batteries => "batteries",
            Class::Sources => "sources",
            Class::CSources => "csources",
            Class::Instruments => "instruments",
            Class::Mechanicals => "mechanicals",
            Class::Misc => "misc",
            Class::Diodes => "diodes",
            Class::Switches => "switches",
            Class::Transistors => "transistors",
            Class::LogicPorts => "logic ports",
            Class::Blocks => "blocks",
        }
    }
}

/// How a component meets its leads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leads {
    /// The leads stop at the component's width.
    Ends,
    /// Nothing between the ends: the leads join (`short`).
    Wire,
    /// No leads at all (`open`).
    Gap,
}

/// A component: its name (as in `to[name]`), class, width and height in `bipoles/length`, and its
/// drawing. `ratio` is its width over its class's `width` setting (a piezoelectric element is twice
/// as wide as the other capacitors). `top` and `bottom` are how far it reaches above and below the
/// line, its `north` and `south` (the band that labels keep out of, which a variable resistor's
/// arrow widens); `anchors` are its named points (`wiper`, `tip`, `label`…), all at the default size.
#[derive(Clone, Copy, Debug)]
pub struct Component {
    pub name: &'static str,
    pub class: Class,
    pub width: f64,
    pub height: f64,
    pub ratio: f64,
    pub top: f64,
    pub bottom: f64,
    pub leads: Leads,
    pub parts: &'static [Part],
    pub anchors: &'static [(&'static str, [f64; 2])],
    /// how far above and below the line it counts in the picture's size beyond its drawing
    pub extent: f64,
}

pub(crate) const fn bipole(name: &'static str, class: Class, width: f64, height: f64, parts: &'static [Part]) -> Component {
    Component { name, class, width, height, ratio: 1.0, top: height / 2.0, bottom: height / 2.0, leads: Leads::Ends, parts, anchors: &[], extent: 0.0 }
}

impl Component {
    /// How far it reaches above and below the line, when that is not its height.
    pub(crate) const fn band(self, top: f64, bottom: f64) -> Component {
        Component { top, bottom, ..self }
    }

    pub(crate) const fn anchors(self, anchors: &'static [(&'static str, [f64; 2])]) -> Component {
        Component { anchors, ..self }
    }

    /// How far above and below the line it counts in the picture's size.
    pub(crate) const fn extent(self, extent: f64) -> Component {
        Component { extent, ..self }
    }

    /// The width over a `width` setting.
    const fn ratio(self, ratio: f64) -> Component {
        Component { ratio, ..self }
    }
}

const fn resistor(name: &'static str, parts: &'static [Part]) -> Component {
    bipole(name, Class::Resistors, 0.8, 0.3, parts)
}

const fn capacitor(name: &'static str, parts: &'static [Part]) -> Component {
    bipole(name, Class::Capacitors, 0.2, 0.6, parts)
}

const fn inductor(name: &'static str, width: f64, parts: &'static [Part]) -> Component {
    bipole(name, Class::Inductors, width, 0.3, parts)
}

/// A box `w` wide and `h` high, centered.
macro_rules! rect {
    ($w:expr, $h:expr) => {
        &[M(-$w, $h), L($w, $h), L($w, -$h), L(-$w, -$h), Z]
    };
}

const BOX: &[Op] = rect!(0.4, 0.15);
/// The box of the tunable generic bipole, a little lower than the others.
const TBOX: &[Op] = rect!(0.4, 0.14);
const PLATES: &[Op] = &[M(-0.1, 0.3), L(-0.1, -0.3), M(0.1, 0.3), L(0.1, -0.3)];
const BETWEEN_PLATES: &[Op] = rect!(0.1, 0.3);
/// The dark end of the asymmetric bipoles.
const DARK_END: &[Op] = &[M(0.28, 0.15), L(0.4, 0.15), L(0.4, -0.15), L(0.28, -0.15), Z];
const TUNABLE: Part = Arrow { from: [-0.2, -0.35], to: [0.2, 0.35], thin: false };
/// The bar of a thermistor (and of the European resistive sensor).
const THERMAL_BAR: &[Op] = &[M(0.4, 0.214), L(-0.16, -0.2568), L(-0.4, -0.2568)];
/// The bar of a varistor and of the thermistors with a coefficient.
const VARISTOR_BAR: &[Op] = &[M(0.4, 0.214), L(-0.16, -0.214), L(-0.4, -0.214)];
/// A circle 0.4 across the radius.
const CIRCLE: &[Op] = &[
    M(0.0, 0.4),
    C([-0.2209, 0.4], [-0.4, 0.2209], [-0.4, 0.0]),
    C([-0.4, -0.2209], [-0.2209, -0.4], [0.0, -0.4]),
    C([0.2209, -0.4], [0.4, -0.2209], [0.4, 0.0]),
    C([0.4, 0.2209], [0.2209, 0.4], [0.0, 0.4]),
    Z,
];
/// The light falling on a light-dependent resistor.
const LIGHT: [Part; 2] = [Arrow { from: [0.56, 0.56], to: [0.32, 0.32], thin: false }, Arrow { from: [0.64, 0.48], to: [0.4, 0.24], thin: false }];
/// The wire inside a light-dependent resistor, from its circle to the resistor inside.
const INNER_LEADS: &[Op] = &[M(-0.408, 0.0), L(-0.328, 0.0), M(0.408, 0.0), L(0.328, 0.0)];
const THETA: Part = Text { at: [-0.3937, -0.3684], shift: [0.0, 0.0], size: 5.0, text: "$\\vartheta$", turn: 0.0 };
const EURO_INDUCTOR: [Part; 2] = [Solid(BOX), Line(BOX)];
const EURO_VARIABLE_INDUCTOR: [Part; 2] = [Solid(TBOX), Line(TBOX)];
/// The plates of the electrolytic capacitor: the positive one open, the negative one solid.
const ELKO_PLUS: &[Op] = &[M(-0.1, 0.25), L(-0.04, 0.25), L(-0.04, -0.25), L(-0.1, -0.25), Z];
const ELKO_MINUS: &[Op] = &[M(0.1, 0.25), L(0.04, 0.25), L(0.04, -0.25), L(0.1, -0.25), Z];
const ZIGZAG: Part = Part::ZigZag { half: 0.4, high: 0.15 };

/// Every component voltik draws, by its main name.
pub const COMPONENTS: &[Component] = &[
    Component { leads: Leads::Wire, ..bipole("short", Class::Default, 0.0, 0.1, &[]) },
    Component { leads: Leads::Gap, ..bipole("open", Class::Default, 0.3, 0.3, &[]) },
    // resistive bipoles
    resistor("generic", &[Body(BOX)]),
    resistor("xgeneric", &[Body(BOX), Line(&[M(-0.4, 0.15), L(0.4, -0.15), M(0.4, 0.15), L(-0.4, -0.15)])]),
    resistor("sgeneric", &[Body(BOX), Line(&[M(0.4, 0.15), L(-0.4, -0.15)])]),
    resistor(
        "singeneric",
        &[
            Body(BOX),
            Line(&[
                M(-0.32, 0.0),
                C([-0.2678, 0.0384], [-0.2179, 0.075], [-0.16, 0.075]),
                C([-0.102, 0.075], [-0.0521, 0.0384], [0.0, 0.0]),
                C([0.0522, -0.0384], [0.1021, -0.075], [0.16, -0.075]),
                C([0.218, -0.075], [0.2679, -0.0384], [0.32, 0.0]),
            ]),
        ],
    ),
    bipole("tgeneric", Class::Resistors, 0.8, 0.28, &[Body(TBOX), TUNABLE]).band(0.35, 0.35).anchors(&[("wiper", [-0.2, -0.35]), ("tip", [0.2, 0.35])]),
    // the generic ones filled with the color they are drawn in
    resistor("fullgeneric", &[Solid(BOX), Line(BOX)]),
    bipole("tfullgeneric", Class::Resistors, 0.8, 0.28, &[Solid(TBOX), Line(TBOX), TUNABLE]).band(0.35, 0.35).anchors(&[("wiper", [-0.2, -0.35]), ("tip", [0.2, 0.35])]),
    resistor("ageneric", &[Body(BOX), Solid(DARK_END), Thin(DARK_END)]),
    resistor(
        "memristor",
        &[
            Body(BOX),
            Thin(&[
                M(-0.4, 0.0),
                L(-0.288, 0.0),
                L(-0.288, 0.075),
                L(-0.14, 0.075),
                L(-0.14, -0.075),
                L(0.02, -0.075),
                L(0.02, 0.075),
                L(0.168, 0.075),
                L(0.168, 0.0),
                L(0.32, 0.0),
            ]),
            Solid(DARK_END),
            Thin(DARK_END),
        ],
    ),
    resistor("american resistor", &[ZIGZAG]),
    resistor("variable american resistor", &[ZIGZAG, Arrow { from: [-0.16, -0.3], to: [0.16, 0.3], thin: false }])
        .band(0.3, 0.3)
        .anchors(&[("wiper", [-0.16, -0.3]), ("tip", [0.16, 0.3])]),
    resistor("american potentiometer", &[ZIGZAG, Part::Wiper { thin: false }]).band(0.4, 0.15),
    resistor("american resistive sensor", &[ZIGZAG, Modifier(&[M(0.16, 0.3), L(-0.16, -0.3), L(-0.36, -0.3)])]).band(0.3, 0.3).anchors(&[
        ("wiper", [-0.36, -0.3]),
        ("tip", [0.16, 0.3]),
        ("label", [-0.16, -0.3]),
    ]),
    bipole("american light dependent resistor", Class::Resistors, 0.8, 0.8, &[Body(CIRCLE), LIGHT[0], LIGHT[1], Line(INNER_LEADS), Part::ZigZag { half: 0.32, high: 0.128 }]),
    bipole("variable european resistor", Class::Resistors, 0.8, 0.28, &[Body(TBOX), TUNABLE]).band(0.35, 0.35).anchors(&[("wiper", [-0.2, -0.35]), ("tip", [0.2, 0.35])]),
    resistor("european potentiometer", &[Body(BOX), Part::Wiper { thin: true }]).band(0.4, 0.15),
    bipole("european light dependent resistor", Class::Resistors, 0.8, 0.8, &[Body(CIRCLE), LIGHT[0], LIGHT[1], Line(INNER_LEADS), Line(rect!(0.32, 0.12))]),
    resistor("varistor", &[Body(BOX), Modifier(VARISTOR_BAR), Text { at: [-0.3058, -0.344], shift: [0.0, 0.0], size: 5.0, text: "U", turn: 0.0 }]).band(0.214, 0.214),
    resistor("mov", &[Body(BOX), Modifier(&[M(0.4, 0.2353), L(0.28, 0.2353), L(-0.28, -0.2354), L(-0.4, -0.2354)])]).band(0.214, 0.214),
    resistor("photoresistor", &[Body(BOX), Arrow { from: [0.28, 0.3], to: [0.12, 0.18], thin: true }, Arrow { from: [0.18, 0.3], to: [0.02, 0.18], thin: true }]).band(0.3, 0.15),
    resistor("thermistor", &[Body(BOX), Modifier(THERMAL_BAR)]).band(0.214, 0.214).anchors(&[("wiper", [-0.4, -0.2568]), ("tip", [0.4, 0.214]), ("label", [-0.16, -0.2568])]),
    // drawn as the thermistor, but its voltages are placed apart
    resistor("european resistive sensor", &[Body(BOX), Modifier(THERMAL_BAR)]).band(0.214, 0.214).anchors(&[
        ("wiper", [-0.4, -0.2568]),
        ("tip", [0.4, 0.214]),
        ("label", [-0.16, -0.2568]),
    ]),
    resistor(
        "thermistor ptc",
        &[
            Body(BOX),
            Modifier(VARISTOR_BAR),
            Arrow { from: [-0.248, -0.375], to: [-0.248, -0.2625], thin: true },
            Arrow { from: [-0.18, -0.375], to: [-0.18, -0.2625], thin: true },
            THETA,
        ],
    )
    .band(0.214, 0.375),
    resistor(
        "thermistor ntc",
        &[
            Body(BOX),
            Modifier(VARISTOR_BAR),
            Arrow { from: [-0.248, -0.375], to: [-0.248, -0.2625], thin: true },
            Arrow { from: [-0.18, -0.2625], to: [-0.18, -0.375], thin: true },
            THETA,
        ],
    )
    .band(0.214, 0.375),
    // capacitors
    capacitor("capacitor", &[Fill(BETWEEN_PLATES), Line(PLATES)]),
    capacitor(
        "curved capacitor",
        &[
            Fill(&[M(0.1, 0.3), C([0.0436, 0.2022], [-0.01, 0.1086], [-0.01, 0.0]), C([-0.01, -0.1086], [0.0436, -0.2022], [0.1, -0.3]), L(-0.1, -0.3), L(-0.1, 0.3), Z]),
            Line(&[M(0.1, 0.3), C([0.0436, 0.2022], [-0.01, 0.1086], [-0.01, 0.0]), C([-0.01, -0.1086], [0.0436, -0.2022], [0.1, -0.3])]),
            Line(&[M(-0.1, 0.3), L(-0.1, -0.3)]),
            Thin(&[M(-0.01, 0.0), L(0.1, 0.0)]),
        ],
    ),
    bipole(
        "ecapacitor",
        Class::Capacitors,
        0.2,
        0.5,
        &[Body(ELKO_PLUS), Solid(ELKO_MINUS), Line(ELKO_MINUS), Text { at: [-0.12, 0.15], shift: [-0.1609, -0.0376], size: 6.0, text: "+", turn: 0.0 }],
    ),
    capacitor("variable capacitor", &[Fill(BETWEEN_PLATES), Line(PLATES), Arrow { from: [-0.3, -0.3], to: [0.3, 0.3], thin: false }])
        .anchors(&[("wiper", [-0.3, -0.3]), ("tip", [0.3, 0.3])]),
    capacitor("capacitive sensor", &[Fill(BETWEEN_PLATES), Line(PLATES), Modifier(&[M(0.26, 0.3), L(-0.26, -0.36), L(-0.44, -0.36)])]).anchors(&[
        ("wiper", [-0.44, -0.36]),
        ("tip", [0.26, 0.3]),
        ("label", [-0.26, -0.36]),
    ]),
    bipole("piezoelectric", Class::Capacitors, 0.4, 0.7, &[Line(&[M(-0.2, 0.35), L(-0.2, -0.35), M(0.2, 0.35), L(0.2, -0.35)]), Body(rect!(0.12, 0.33))]).ratio(2.0),
    bipole(
        "cpe",
        Class::Capacitors,
        0.4,
        0.6,
        &[
            Fill(&[M(-0.2, 0.3), L(0.0, 0.0), L(-0.2, -0.3), L(0.0, -0.3), L(0.2, 0.0), L(0.0, 0.3), Z]),
            Thin(&[M(-0.2, 0.0), L(0.0, 0.0)]),
            Line(&[M(-0.2, 0.3), L(0.0, 0.0), L(-0.2, -0.3), M(0.0, 0.3), L(0.2, 0.0), L(0.0, -0.3)]),
        ],
    )
    .ratio(2.0),
    capacitor(
        "feC",
        &[
            Fill(BETWEEN_PLATES),
            Line(PLATES),
            Modifier(&[
                M(-0.18, 0.3),
                L(-0.18, 0.17),
                C([-0.18, 0.159], [-0.1593, 0.1328], [-0.1339, 0.1116]),
                L(0.1646, -0.1372),
                C([0.1731, -0.1443], [0.18, -0.1769], [0.18, -0.21]),
                L(0.18, -0.3),
            ]),
        ],
    )
    .anchors(&[("kink left", [-0.18, 0.15]), ("kink right", [0.18, -0.15]), ("curve left", [-0.18, 0.3]), ("curve right", [0.18, -0.3])]),
    // buses, crossings and connectors
    bipole("multiwire", Class::Default, 0.2, 0.4, &[Thin(&[M(-0.1, -0.2), L(0.0, 0.2), M(-0.1, 0.0), L(0.1, 0.0)])]),
    bipole("bmultiwire", Class::Default, 0.2, 0.4, &[Thin(&[M(-0.1, -0.2), L(0.0, 0.2), M(-0.05, -0.2), L(0.05, 0.2), M(-0.1, 0.0), L(0.1, 0.0)])]),
    bipole("tmultiwire", Class::Default, 0.2, 0.4, &[Thin(&[M(-0.1, -0.2), L(0.0, 0.2), M(-0.05, -0.2), L(0.05, 0.2), M(0.0, -0.2), L(0.1, 0.2), M(-0.1, 0.0), L(0.1, 0.0)])]),
    bipole(
        "crossing",
        Class::Default,
        0.2,
        0.08,
        &[Thin(&[M(-0.1, 0.0), L(-0.04, 0.0), C([-0.04, 0.0221], [-0.0221, 0.04], [0.0, 0.04]), C([0.0221, 0.04], [0.04, 0.0221], [0.04, 0.0]), L(0.1, 0.0)])],
    ),
    bipole(
        "iec connector",
        Class::Connectors,
        0.3,
        0.2,
        &[
            Line(&[M(-0.05, 0.1), C([-0.1052, 0.1], [-0.15, 0.0552], [-0.15, 0.0]), C([-0.15, -0.0552], [-0.1052, -0.1], [-0.05, -0.1])]),
            Solid(&[M(-0.05, 0.05), L(0.15, 0.05), L(0.15, -0.05), L(-0.05, -0.05), Z]),
            Line(&[M(-0.05, 0.05), L(0.15, 0.05), L(0.15, -0.05), L(-0.05, -0.05), Z]),
        ],
    )
    .anchors(&[("plug center", [0.05, 0.0]), ("socket center", [-0.05, 0.0])]),
    // inductors
    inductor("cute inductor", 0.6, &[Part::Coil]).band(0.15, 0.075),
    inductor("variable cute inductor", 0.6, &[Arrow { from: [-0.12, -0.15], to: [0.12, 0.3], thin: false }, Part::Coil])
        .band(0.3, 0.15)
        .anchors(&[("wiper", [-0.12, -0.15]), ("tip", [0.12, 0.3])]),
    inductor("cute inductive sensor", 0.6, &[Part::Coil, Modifier(&[M(0.24, 0.3), L(-0.24, -0.195), L(-0.48, -0.195)])]).band(0.3, 0.195).anchors(&[
        ("wiper", [-0.48, -0.195]),
        ("tip", [0.24, 0.3]),
        ("label", [-0.24, -0.195]),
    ]),
    inductor("cute choke", 0.6, &[Part::Coil, Part::ChokeCore(&[M(-0.3, 0.191), L(0.3, 0.191)])]).band(0.15, 0.075),
    inductor("american inductor", 0.8, &[Part::Arches]).band(0.15, 0.05),
    inductor("variable american inductor", 0.8, &[Part::Arches, Arrow { from: [-0.16, -0.1], to: [0.16, 0.3], thin: false }])
        .band(0.3, 0.1)
        .anchors(&[("wiper", [-0.16, -0.1]), ("tip", [0.16, 0.3])]),
    inductor("american inductive sensor", 0.8, &[Part::Arches, Modifier(&[M(0.32, 0.3), L(-0.32, -0.13), L(-0.64, -0.13)])]).band(0.3, 0.13).anchors(&[
        ("wiper", [-0.64, -0.13]),
        ("tip", [0.32, 0.3]),
        ("label", [-0.32, -0.13]),
    ]),
    inductor("european inductor", 0.8, &EURO_INDUCTOR),
    bipole("variable european inductor", Class::Inductors, 0.8, 0.28, &[EURO_VARIABLE_INDUCTOR[0], EURO_VARIABLE_INDUCTOR[1], TUNABLE])
        .band(0.35, 0.35)
        .anchors(&[("wiper", [-0.2, -0.35]), ("tip", [0.2, 0.35])]),
    inductor("european inductive sensor", 0.8, &[EURO_INDUCTOR[0], EURO_INDUCTOR[1], Modifier(&[M(0.4, 0.3), L(-0.16, -0.3), L(-0.4, -0.3)])]).band(0.3, 0.3).anchors(&[
        ("wiper", [-0.4, -0.3]),
        ("tip", [0.4, 0.3]),
        ("label", [-0.16, -0.3]),
    ]),
];

/// What a key names: a component, or one that follows the `resistor` or `inductor` style.
#[derive(Clone, Copy, Debug)]
pub enum Target {
    Fixed(&'static str),
    /// American, European
    Resistor(&'static str, &'static str),
    /// cute, American, European
    Inductor(&'static str, &'static str, &'static str),
    /// European, American, by the `voltage` style
    Voltage(&'static str, &'static str),
    /// European, American, by the `current` style
    Current(&'static str, &'static str),
    /// empty, full, stroke, by the `diode` setting
    Diode(&'static str, &'static str, &'static str),
}

use Target::{Current, Diode, Fixed, Inductor, Resistor, Voltage};

/// The keys that name a component, and what they name: the short names (`R`, `vR`, `L`) follow the
/// `resistor` and `inductor` styles (`\ctikzset{resistor=european}`, the `european` style).
pub const ALIASES: &[(&str, Target)] = &[
    ("short", Fixed("short")),
    ("open", Fixed("open")),
    ("generic", Fixed("generic")),
    ("xgeneric", Fixed("xgeneric")),
    ("sgeneric", Fixed("sgeneric")),
    ("singeneric", Fixed("singeneric")),
    ("tgeneric", Fixed("tgeneric")),
    ("fullgeneric", Fixed("fullgeneric")),
    ("tfullgeneric", Fixed("tfullgeneric")),
    ("ageneric", Fixed("ageneric")),
    ("memristor", Fixed("memristor")),
    ("Mr", Fixed("memristor")),
    ("R", Resistor("american resistor", "generic")),
    ("resistor", Resistor("american resistor", "generic")),
    ("american resistor", Fixed("american resistor")),
    ("european resistor", Fixed("generic")),
    ("vR", Resistor("variable american resistor", "variable european resistor")),
    ("variable american resistor", Fixed("variable american resistor")),
    ("variable european resistor", Fixed("variable european resistor")),
    ("pR", Resistor("american potentiometer", "european potentiometer")),
    ("potentiometer", Resistor("american potentiometer", "european potentiometer")),
    ("american potentiometer", Fixed("american potentiometer")),
    ("european potentiometer", Fixed("european potentiometer")),
    ("sR", Resistor("american resistive sensor", "european resistive sensor")),
    ("american resistive sensor", Fixed("american resistive sensor")),
    ("european resistive sensor", Fixed("european resistive sensor")),
    ("ldR", Resistor("american light dependent resistor", "european light dependent resistor")),
    ("american light dependent resistor", Fixed("american light dependent resistor")),
    ("european light dependent resistor", Fixed("european light dependent resistor")),
    ("varistor", Fixed("varistor")),
    ("mov", Fixed("mov")),
    ("phR", Fixed("photoresistor")),
    ("photoresistor", Fixed("photoresistor")),
    ("thR", Fixed("thermistor")),
    ("thermistor", Fixed("thermistor")),
    ("thRp", Fixed("thermistor ptc")),
    ("thermistor ptc", Fixed("thermistor ptc")),
    ("thRn", Fixed("thermistor ntc")),
    ("thermistor ntc", Fixed("thermistor ntc")),
    ("C", Fixed("capacitor")),
    ("capacitor", Fixed("capacitor")),
    ("cC", Fixed("curved capacitor")),
    ("curved capacitor", Fixed("curved capacitor")),
    ("eC", Fixed("ecapacitor")),
    ("elko", Fixed("ecapacitor")),
    ("ecapacitor", Fixed("ecapacitor")),
    ("vC", Fixed("variable capacitor")),
    ("variable capacitor", Fixed("variable capacitor")),
    ("sC", Fixed("capacitive sensor")),
    ("capacitive sensor", Fixed("capacitive sensor")),
    ("PZ", Fixed("piezoelectric")),
    ("piezoelectric", Fixed("piezoelectric")),
    ("cpe", Fixed("cpe")),
    ("feC", Fixed("feC")),
    ("ferrocap", Fixed("feC")),
    ("L", Inductor("cute inductor", "american inductor", "european inductor")),
    ("inductor", Inductor("cute inductor", "american inductor", "european inductor")),
    ("cute inductor", Fixed("cute inductor")),
    ("american inductor", Fixed("american inductor")),
    ("european inductor", Fixed("european inductor")),
    ("vL", Inductor("variable cute inductor", "variable american inductor", "variable european inductor")),
    ("variable cute inductor", Fixed("variable cute inductor")),
    ("variable american inductor", Fixed("variable american inductor")),
    ("variable european inductor", Fixed("variable european inductor")),
    ("sL", Inductor("cute inductive sensor", "american inductive sensor", "european inductive sensor")),
    ("cute inductive sensor", Fixed("cute inductive sensor")),
    ("american inductive sensor", Fixed("american inductive sensor")),
    ("european inductive sensor", Fixed("european inductive sensor")),
    ("cute choke", Fixed("cute choke")),
    ("multiwire", Fixed("multiwire")),
    ("bmultiwire", Fixed("bmultiwire")),
    ("tmultiwire", Fixed("tmultiwire")),
    ("crossing", Fixed("crossing")),
    ("xing", Fixed("crossing")),
    ("iec connector", Fixed("iec connector")),
    ("iecconn", Fixed("iec connector")),
];

/// Where a component's voltage goes, in `bipoles/length` at its default size: the voltage spans
/// from `from node` of the way between the end (`NODE_GAP` inside it) and the component, which is
/// `half` wide on each side; `south` is how far its straight arrow and American signs keep from
/// the line, `bump` how deep the European arrow bends, `label` how far the European label is.
/// Measured on what CircuiTikZ draws, component by component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Volts {
    pub half: Option<f64>,
    pub from_node: f64,
    pub south: f64,
    pub bump: f64,
    pub label: f64,
    /// a `voltage shift` of its own, in `bipoles/length` (a square meter's, 0.08)
    pub shift: f64,
    /// `south`, `bump` and `label` on the other side (`v^`), when they differ: an LED's arrows
    pub above: Option<[f64; 3]>,
}

const fn volts(south: f64, bump: f64, label: f64) -> Volts {
    Volts { half: None, from_node: 0.5, south, bump, label, shift: 0.0, above: None }
}

/// The voltages of the generic bipoles start nearer their ends.
const fn generic_volts(south: f64, bump: f64, label: f64) -> Volts {
    Volts { from_node: 0.4, ..volts(south, bump, label) }
}

/// The voltages of every inductor are placed as for the cute one, 0.6 wide.
const fn inductor_volts(south: f64, bump: f64, label: f64) -> Volts {
    Volts { half: Some(0.3), ..volts(south, bump, label) }
}

/// How far inside the end a voltage starts from (2.5 bp).
pub const NODE_GAP: f64 = 0.0629;

const VOLTS: &[(&str, Volts)] = &[
    ("short", volts(0.0909, 0.1364, 0.2853)),
    // an open circuit's voltage is on the line, from 15% to 85% of the way
    ("open", Volts { half: Some(0.0), from_node: 0.3, ..volts(0.15, 0.225, 0.3828) }),
    ("generic", generic_volts(0.15, 0.3, 0.4653)),
    ("xgeneric", volts(0.15, 0.225, 0.3828)),
    ("sgeneric", volts(0.15, 0.225, 0.3828)),
    ("singeneric", volts(0.15, 0.225, 0.3828)),
    ("tgeneric", volts(0.35, 0.525, 0.7128)),
    ("fullgeneric", generic_volts(0.15, 0.3, 0.4653)),
    ("tfullgeneric", volts(0.35, 0.525, 0.7128)),
    ("ageneric", generic_volts(0.15, 0.3, 0.4653)),
    ("memristor", generic_volts(0.15, 0.3, 0.4653)),
    ("american resistor", volts(0.15, 0.225, 0.3828)),
    ("variable american resistor", volts(0.3, 0.45, 0.6303)),
    ("american potentiometer", volts(0.15, 0.225, 0.3828)),
    ("american resistive sensor", volts(0.3, 0.45, 0.6303)),
    ("american light dependent resistor", volts(0.4, 0.6, 0.7953)),
    ("variable european resistor", volts(0.3, 0.45, 0.6303)),
    ("european potentiometer", volts(0.15, 0.225, 0.3828)),
    ("european resistive sensor", volts(0.3, 0.45, 0.6303)),
    ("european light dependent resistor", volts(0.4, 0.6, 0.7953)),
    ("varistor", volts(0.3, 0.719, 0.9262)),
    ("mov", volts(0.2568, 0.3852, 0.5591)),
    ("photoresistor", volts(0.15, 0.24, 0.3993)),
    ("thermistor", volts(0.2568, 0.6163, 0.6849)),
    ("thermistor ptc", volts(0.375, 0.6, 0.6669)),
    ("thermistor ntc", volts(0.375, 0.6, 0.6669)),
    ("capacitor", volts(0.3, 0.4125, 0.5888)),
    ("curved capacitor", volts(0.3, 0.6046, 0.8004)),
    ("ecapacitor", volts(0.25, 0.375, 0.5478)),
    ("variable capacitor", volts(0.3, 0.4125, 0.5888)),
    ("capacitive sensor", volts(0.36, 0.54, 0.7293)),
    ("piezoelectric", volts(0.35, 0.525, 0.7128)),
    ("cpe", volts(0.3, 0.45, 0.6303)),
    ("feC", volts(0.3, 0.45, 0.6303)),
    ("cute inductor", inductor_volts(0.075, 0.1125, 0.2591)),
    ("variable cute inductor", inductor_volts(0.15, 0.225, 0.3828)),
    ("cute inductive sensor", inductor_volts(0.195, 0.2925, 0.457)),
    ("cute choke", inductor_volts(0.075, 0.1125, 0.2591)),
    ("american inductor", inductor_volts(0.075, 0.1125, 0.2591)),
    ("variable american inductor", inductor_volts(0.15, 0.225, 0.3828)),
    ("american inductive sensor", inductor_volts(0.195, 0.2925, 0.457)),
    ("european inductor", inductor_volts(0.075, 0.1125, 0.2591)),
    ("variable european inductor", inductor_volts(0.15, 0.225, 0.3828)),
    ("european inductive sensor", inductor_volts(0.195, 0.2925, 0.457)),
    ("european current source", volts(0.3, 0.45, 0.6303)),
    ("cute european current source", volts(0.3, 0.45, 0.6303)),
    ("american current source", volts(0.3, 0.45, 0.6303)),
    ("sinusoidal current source", volts(0.3, 0.45, 0.6303)),
    ("european controlled current source", volts(0.35, 0.525, 0.7128)),
    ("cute european controlled current source", volts(0.35, 0.525, 0.7128)),
    ("american controlled current source", volts(0.35, 0.525, 0.7128)),
    ("controlled sinusoidal current source", volts(0.35, 0.525, 0.7128)),
    ("noise current source", volts(0.3, 0.45, 0.6303)),
    ("ioosource", volts(0.195, 0.2925, 0.4571)),
    ("nullator", volts(0.15, 0.225, 0.3828)),
    ("norator", volts(0.125, 0.1875, 0.3416)),
    ("dcisource", volts(0.3001, 0.45, 0.6304)),
    ("rmeter", volts(0.3, 0.45, 0.6303)),
    ("rmeterwa", volts(0.3001, 0.45, 0.6304)),
    ("ammeter", volts(0.3, 0.45, 0.6303)),
    ("voltmeter", volts(0.3, 0.45, 0.6303)),
    ("ohmmeter", volts(0.3, 0.45, 0.6303)),
    ("smeter", Volts { shift: 0.08, ..volts(0.3, 0.45, 0.6303) }),
    ("qiprobe", Volts { shift: 0.04, ..volts(0.2, 0.2999, 0.4653) }),
    ("qvprobe", Volts { shift: 0.04, ..volts(0.2, 0.2999, 0.4653) }),
    ("qpprobe", Volts { shift: 0.04, ..volts(0.2, 0.2999, 0.4653) }),
    ("oscope", Volts { shift: 0.08, ..volts(0.3, 0.45, 0.6303) }),
    ("iloop", volts(0.3001, 0.45, 0.6304)),
    ("iloop2", volts(0.3001, 0.45, 0.6304)),
    ("currtap", volts(0.2, 0.3001, 0.4653)),
    ("damper", volts(0.175, 0.2625, 0.4241)),
    ("inerter", volts(0.35, 0.3607, 0.5321)),
    ("spring", volts(0.2499, 0.375, 0.5478)),
    ("viscoe", volts(0.175, 0.2625, 0.4241)),
    ("mass", volts(0.2, 0.3, 0.4653)),
    ("thermocouple", volts(0.3, 0.2886, 0.4527)),
    ("fuse", volts(0.1, 0.15, 0.3003)),
    ("afuse", volts(0.1, 0.15, 0.3003)),
    ("wfuse", volts(0.1, 0.15, 0.3003)),
    ("relais", volts(0.4, 0.6, 0.7953)),
    ("squid", volts(0.3001, 0.45, 0.6304)),
    ("barrier", volts(0.105, 0.0, 0.1353)),
    ("openbarrier", volts(0.105, 0.1575, 0.3085)),
    ("european gas filled surge arrester", volts(0.15, 0.225, 0.3828)),
    ("american gas filled surge arrester", volts(0.3, 0.45, 0.6303)),
    ("lamp", volts(0.3, 0.45, 0.6303)),
    ("bulb", volts(0.4, 0.6, 0.7953)),
    ("wbulb", volts(0.4, 0.6, 0.7953)),
    ("neonlampcc", volts(0.3, 0.45, 0.6303)),
    ("neonlampac", volts(0.3, 0.45, 0.6303)),
    ("sparkgap", volts(0.15, 0.225, 0.3828)),
    ("sparkgap circle", volts(0.3, 0.45, 0.6303)),
    ("sparkgap dot circle", volts(0.3, 0.45, 0.6303)),
    ("loudspeaker", volts(0.15, 0.225, 0.3828)),
    ("mic", volts(0.05, 0.075, 0.2178)),
    ("tlmic", volts(0.25, 0.375, 0.5479)),
    ("buzzer", volts(0.0, 0.0, 0.1353)),
    ("rbuzzer", volts(0.0, 0.0, 0.1353)),
    ("empty diode", volts(0.2499, 0.375, 0.5478)),
    ("empty Schottky diode", volts(0.2499, 0.375, 0.5478)),
    ("empty Zener diode", volts(0.2499, 0.375, 0.5478)),
    ("empty ZZener diode", volts(0.325, 0.8124, 0.829)),
    ("empty tunnel diode", volts(0.2499, 0.375, 0.5478)),
    ("empty photodiode", Volts { above: Some([0.45, 0.675, 0.8777]), ..volts(0.25, 0.375, 0.5479) }),
    ("empty led", Volts { above: Some([0.45, 0.675, 0.8777]), ..volts(0.25, 0.375, 0.5479) }),
    ("empty laser diode", Volts { above: Some([0.45, 0.675, 0.8777]), ..volts(0.25, 0.375, 0.5479) }),
    ("empty varcap", volts(0.2499, 0.375, 0.5478)),
    ("empty TVS diode", volts(0.325, 0.4875, 0.6715)),
    ("empty Shockley diode", volts(0.2499, 0.375, 0.5478)),
    ("empty bidirectionaldiode", volts(0.5501, 0.8251, 1.0429)),
    ("full diode", volts(0.2499, 0.375, 0.5478)),
    ("full Schottky diode", volts(0.2499, 0.375, 0.5478)),
    ("full Zener diode", volts(0.2499, 0.375, 0.5478)),
    ("full ZZener diode", volts(0.325, 0.8124, 0.829)),
    ("full tunnel diode", volts(0.2499, 0.375, 0.5478)),
    ("full photodiode", Volts { above: Some([0.45, 0.675, 0.8777]), ..volts(0.25, 0.375, 0.5479) }),
    ("full led", Volts { above: Some([0.45, 0.675, 0.8777]), ..volts(0.25, 0.375, 0.5479) }),
    ("full laser diode", Volts { above: Some([0.45, 0.675, 0.8777]), ..volts(0.25, 0.375, 0.5479) }),
    ("full varcap", volts(0.2499, 0.375, 0.5478)),
    ("full TVS diode", volts(0.325, 0.4875, 0.6715)),
    ("full Shockley diode", volts(0.2499, 0.375, 0.5478)),
    ("full bidirectionaldiode", volts(0.5501, 0.8251, 1.0429)),
    ("stroke diode", volts(0.2499, 0.375, 0.5478)),
    ("stroke Schottky diode", volts(0.2499, 0.375, 0.5478)),
    ("stroke Zener diode", volts(0.2499, 0.375, 0.5478)),
    ("stroke ZZener diode", volts(0.325, 0.8124, 0.829)),
    ("stroke tunnel diode", volts(0.2499, 0.375, 0.5478)),
    ("stroke photodiode", Volts { above: Some([0.45, 0.675, 0.8777]), ..volts(0.25, 0.375, 0.5479) }),
    ("stroke led", Volts { above: Some([0.45, 0.675, 0.8777]), ..volts(0.25, 0.375, 0.5479) }),
    ("stroke laser diode", Volts { above: Some([0.45, 0.675, 0.8777]), ..volts(0.25, 0.375, 0.5479) }),
    ("stroke varcap", volts(0.2499, 0.375, 0.5478)),
    ("stroke TVS diode", volts(0.325, 0.4875, 0.6715)),
    ("empty triac", volts(0.5501, 0.9901, 0.9244)),
    ("full triac", volts(0.5501, 0.9901, 0.9244)),
    ("empty thyristor", Volts { above: Some([0.55, 1.1, 1.2453]), ..volts(0.25, 0.5001, 0.5854) }),
    ("full thyristor", Volts { above: Some([0.55, 1.1, 1.2453]), ..volts(0.25, 0.5001, 0.5854) }),
    ("stroke thyristor", Volts { above: Some([0.55, 1.1, 1.2453]), ..volts(0.25, 0.5001, 0.5854) }),
    ("empty put", Volts { above: Some([0.5501, 0.8251, 1.0428]), ..volts(0.25, 0.375, 0.5479) }),
    ("full put", Volts { above: Some([0.5501, 0.8251, 1.0428]), ..volts(0.25, 0.375, 0.5479) }),
    ("stroke put", Volts { above: Some([0.5501, 0.8251, 1.0428]), ..volts(0.25, 0.375, 0.5479) }),
    ("empty gto", Volts { above: Some([0.5501, 0.8251, 1.0428]), ..volts(0.25, 0.375, 0.5479) }),
    ("full gto", Volts { above: Some([0.5501, 0.8251, 1.0428]), ..volts(0.25, 0.375, 0.5479) }),
    ("stroke gto", Volts { above: Some([0.5501, 0.8251, 1.0428]), ..volts(0.25, 0.375, 0.5479) }),
    ("empty gtobar", Volts { above: Some([0.5501, 0.8251, 1.0428]), ..volts(0.25, 0.375, 0.5479) }),
    ("full gtobar", Volts { above: Some([0.5501, 0.8251, 1.0428]), ..volts(0.25, 0.375, 0.5479) }),
    ("stroke gtobar", Volts { above: Some([0.5501, 0.8251, 1.0428]), ..volts(0.25, 0.375, 0.5479) }),
    ("empty agtobar", Volts { above: Some([0.5501, 0.8251, 1.0428]), ..volts(0.25, 0.375, 0.5479) }),
    ("full agtobar", Volts { above: Some([0.5501, 0.8251, 1.0428]), ..volts(0.25, 0.375, 0.5479) }),
    ("stroke agtobar", Volts { above: Some([0.5501, 0.8251, 1.0428]), ..volts(0.25, 0.375, 0.5479) }),
    ("empty igct", Volts { above: Some([0.5501, 0.8251, 1.0428]), ..volts(0.25, 0.375, 0.5479) }),
    ("full igct", Volts { above: Some([0.5501, 0.8251, 1.0428]), ..volts(0.25, 0.375, 0.5479) }),
    ("stroke igct", Volts { above: Some([0.5501, 0.8251, 1.0428]), ..volts(0.25, 0.375, 0.5479) }),
    ("compact empty triac", volts(0.5501, 0.9901, 0.9244)),
    ("compact full triac", volts(0.5501, 0.9901, 0.9244)),
    ("compact empty thyristor", Volts { above: Some([0.4, 0.8, 0.9152]), ..volts(0.2499, 0.5, 0.5853) }),
    ("compact full thyristor", Volts { above: Some([0.4, 0.8, 0.9152]), ..volts(0.2499, 0.5, 0.5853) }),
    ("compact stroke thyristor", Volts { above: Some([0.4, 0.8, 0.9152]), ..volts(0.2499, 0.5, 0.5853) }),
    ("compact empty put", Volts { above: Some([0.4, 0.6, 0.7952]), ..volts(0.2499, 0.375, 0.5478) }),
    ("compact full put", Volts { above: Some([0.4, 0.6, 0.7952]), ..volts(0.2499, 0.375, 0.5478) }),
    ("compact stroke put", Volts { above: Some([0.4, 0.6, 0.7952]), ..volts(0.2499, 0.375, 0.5478) }),
    ("compact empty gto", Volts { above: Some([0.4, 0.6, 0.7952]), ..volts(0.2499, 0.375, 0.5478) }),
    ("compact full gto", Volts { above: Some([0.4, 0.6, 0.7952]), ..volts(0.2499, 0.375, 0.5478) }),
    ("compact stroke gto", Volts { above: Some([0.4, 0.6, 0.7952]), ..volts(0.2499, 0.375, 0.5478) }),
    ("compact empty gtobar", Volts { above: Some([0.4, 0.6, 0.7952]), ..volts(0.2499, 0.375, 0.5478) }),
    ("compact full gtobar", Volts { above: Some([0.4, 0.6, 0.7952]), ..volts(0.2499, 0.375, 0.5478) }),
    ("compact stroke gtobar", Volts { above: Some([0.4, 0.6, 0.7952]), ..volts(0.2499, 0.375, 0.5478) }),
    ("compact empty agtobar", Volts { above: Some([0.4, 0.6, 0.7952]), ..volts(0.2499, 0.375, 0.5478) }),
    ("compact full agtobar", Volts { above: Some([0.4, 0.6, 0.7952]), ..volts(0.2499, 0.375, 0.5478) }),
    ("compact stroke agtobar", Volts { above: Some([0.4, 0.6, 0.7952]), ..volts(0.2499, 0.375, 0.5478) }),
    ("compact empty igct", Volts { above: Some([0.4, 0.6, 0.7952]), ..volts(0.2499, 0.375, 0.5478) }),
    ("compact full igct", Volts { above: Some([0.4, 0.6, 0.7952]), ..volts(0.2499, 0.375, 0.5478) }),
    ("compact stroke igct", Volts { above: Some([0.4, 0.6, 0.7952]), ..volts(0.2499, 0.375, 0.5478) }),
    ("empty led@cathode", Volts { above: Some([0.45, 0.675, 0.8777]), ..volts(0.25, 0.375, 0.5479) }),
    ("empty photodiode@cathode", Volts { above: Some([0.45, 0.675, 0.8777]), ..volts(0.25, 0.375, 0.5479) }),
    ("empty ZZener diode@straight", volts(0.325, 0.8124, 0.829)),
    ("empty TVS diode@straight", volts(0.325, 0.4875, 0.6715)),
    ("full led@cathode", Volts { above: Some([0.45, 0.675, 0.8777]), ..volts(0.25, 0.375, 0.5479) }),
    ("full photodiode@cathode", Volts { above: Some([0.45, 0.675, 0.8777]), ..volts(0.25, 0.375, 0.5479) }),
    ("full ZZener diode@straight", volts(0.325, 0.8124, 0.829)),
    ("full TVS diode@straight", volts(0.325, 0.4875, 0.6715)),
    ("stroke led@cathode", Volts { above: Some([0.45, 0.675, 0.8777]), ..volts(0.25, 0.375, 0.5479) }),
    ("stroke photodiode@cathode", Volts { above: Some([0.45, 0.675, 0.8777]), ..volts(0.25, 0.375, 0.5479) }),
    ("stroke ZZener diode@straight", volts(0.325, 0.8124, 0.829)),
    ("stroke TVS diode@straight", volts(0.325, 0.4875, 0.6715)),
];

/// The voltage of a voltage source (and of a battery): a straight arrow across it, `[half its
/// length, how far above the line, how far the label's text is above the arrow]`, in
/// `bipoles/length`. Measured on what CircuiTikZ draws.
const SOURCE_VOLTS: &[(&str, [f64; 3])] = &[
    ("battery", [0.231, 0.4, 0.1153]),
    ("battery1", [0.198, 0.3429, 0.1153]),
    ("battery2", [0.198, 0.3429, 0.1153]),
    ("solar", [0.198, 0.3429, 0.1153]),
    ("baertty", [0.198, 0.3429, 0.1552]),
    ("european voltage source", [0.2286, 0.396, 0.1153]),
    ("cute european voltage source", [0.2286, 0.396, 0.1552]),
    ("american voltage source", [0.2286, 0.396, 0.1552]),
    ("sinusoidal voltage source", [0.2286, 0.396, 0.1153]),
    ("european controlled voltage source", [0.2667, 0.462, 0.1252]),
    ("cute european controlled voltage source", [0.2667, 0.4619, 0.1552]),
    ("american controlled voltage source", [0.2667, 0.4619, 0.1552]),
    ("empty controlled source", [0.2667, 0.462, 0.1252]),
    ("controlled sinusoidal voltage source", [0.2667, 0.462, 0.1252]),
    ("noise voltage source", [0.2286, 0.396, 0.1552]),
    ("square voltage source", [0.2286, 0.396, 0.1153]),
    ("vsourcetri", [0.2286, 0.396, 0.1153]),
    ("esource", [0.2286, 0.396, 0.1153]),
    ("pvsource", [0.2286, 0.396, 0.1153]),
    ("pvmodule", [0.2286, 0.396, 0.1153]),
    ("voosource", [0.1486, 0.2574, 0.0942]),
    ("oosourceatrans", [0.1372, 0.2376, 0.0913]),
    ("oosourcetrans", [0.1372, 0.2376, 0.0913]),
    ("oosourcetrans large", [0.1372, 0.2376, 0.0913]),
    ("ooosource", [0.2286, 0.396, 0.1153]),
    ("dcvsource", [0.2286, 0.396, 0.1153]),
];

impl Component {
    /// Where a voltage source's voltage goes.
    pub fn source_volts(&self) -> Option<[f64; 3]> {
        SOURCE_VOLTS.iter().find(|(n, _)| *n == self.name).map(|(_, v)| *v)
    }
}

impl Component {
    /// Where its voltage goes.
    pub fn volts(&self) -> Volts {
        // by default as far as it reaches on that side: a switch's lever, above, takes its voltage farther
        let side = |reach: f64| [reach, 1.5 * reach, 0.1352 + 1.0969 * 1.5 * reach];
        let [south, bump, label] = side(self.bottom);
        let above = (self.top != self.bottom).then(|| side(self.top));
        VOLTS.iter().find(|(n, _)| *n == self.name).map_or(Volts { above, ..volts(south, bump, label) }, |(_, v)| *v)
    }
}

mod blocks;
mod diodes;
mod instruments;
pub use blocks::{names as block_names, setting_name as block_setting_name, size as blocks_size, text as block_text};
mod mechanicals;
mod misc;
mod sources;
mod switches;
mod transistors;

/// The tables of components, by group of the manual.
const GROUPS: &[&[Component]] = &[
    COMPONENTS,
    sources::COMPONENTS,
    instruments::COMPONENTS,
    mechanicals::COMPONENTS,
    misc::COMPONENTS,
    diodes::COMPONENTS,
    switches::COMPONENTS,
    transistors::COMPONENTS,
    blocks::COMPONENTS,
];

/// The diodes' keys: the short ones (`D`, `sD`, `Ty`, `triac`…) follow the `diode` setting (`empty`,
/// `full`, `stroke`; the styles `empty diodes`, `full diodes`, `stroke diodes`), the others name a
/// shape (`D*` is always full).
const DIODE_ALIASES: &[(&str, Target)] = &[
    ("D", Diode("empty diode", "full diode", "stroke diode")),
    ("sD", Diode("empty Schottky diode", "full Schottky diode", "stroke Schottky diode")),
    ("zD", Diode("empty Zener diode", "full Zener diode", "stroke Zener diode")),
    ("zzD", Diode("empty ZZener diode", "full ZZener diode", "stroke ZZener diode")),
    ("tD", Diode("empty tunnel diode", "full tunnel diode", "stroke tunnel diode")),
    ("pD", Diode("empty photodiode", "full photodiode", "stroke photodiode")),
    ("leD", Diode("empty led", "full led", "stroke led")),
    ("VC", Diode("empty varcap", "full varcap", "stroke varcap")),
    ("tvsD", Diode("empty TVS diode", "full TVS diode", "stroke TVS diode")),
    ("shD", Diode("empty Shockley diode", "full Shockley diode", "empty Shockley diode")),
    ("biD", Diode("empty bidirectionaldiode", "full bidirectionaldiode", "empty bidirectionaldiode")),
    ("Tr", Diode("empty triac", "full triac", "empty triac")),
    ("Ty", Diode("empty thyristor", "full thyristor", "stroke thyristor")),
    ("PUT", Diode("empty put", "full put", "stroke put")),
    ("GTO", Diode("empty gto", "full gto", "stroke gto")),
    ("GTOb", Diode("empty gtobar", "full gtobar", "stroke gtobar")),
    ("aGTOb", Diode("empty agtobar", "full agtobar", "stroke agtobar")),
    ("IGCT", Diode("empty igct", "full igct", "stroke igct")),
    ("triac", Diode("empty triac", "full triac", "empty triac")),
    ("thyristor", Diode("empty thyristor", "full thyristor", "stroke thyristor")),
    ("put", Diode("empty put", "full put", "stroke put")),
    ("gto", Diode("empty gto", "full gto", "stroke gto")),
    ("gtobar", Diode("empty gtobar", "full gtobar", "stroke gtobar")),
    ("agtobar", Diode("empty agtobar", "full agtobar", "stroke agtobar")),
    ("igct", Diode("empty igct", "full igct", "stroke igct")),
    ("empty diode", Fixed("empty diode")),
    ("Do", Fixed("empty diode")),
    ("empty Schottky diode", Fixed("empty Schottky diode")),
    ("sDo", Fixed("empty Schottky diode")),
    ("empty Zener diode", Fixed("empty Zener diode")),
    ("zDo", Fixed("empty Zener diode")),
    ("empty ZZener diode", Fixed("empty ZZener diode")),
    ("zzDo", Fixed("empty ZZener diode")),
    ("empty tunnel diode", Fixed("empty tunnel diode")),
    ("tDo", Fixed("empty tunnel diode")),
    ("empty photodiode", Fixed("empty photodiode")),
    ("pDo", Fixed("empty photodiode")),
    ("empty led", Fixed("empty led")),
    ("leDo", Fixed("empty led")),
    ("empty laser diode", Fixed("empty laser diode")),
    ("lasD", Fixed("empty laser diode")),
    ("empty varcap", Fixed("empty varcap")),
    ("VCo", Fixed("empty varcap")),
    ("empty TVS diode", Fixed("empty TVS diode")),
    ("tvsDo", Fixed("empty TVS diode")),
    ("empty Shockley diode", Fixed("empty Shockley diode")),
    ("shDo", Fixed("empty Shockley diode")),
    ("empty bidirectionaldiode", Fixed("empty bidirectionaldiode")),
    ("biDo", Fixed("empty bidirectionaldiode")),
    ("full diode", Fixed("full diode")),
    ("D*", Fixed("full diode")),
    ("full Schottky diode", Fixed("full Schottky diode")),
    ("sD*", Fixed("full Schottky diode")),
    ("full Zener diode", Fixed("full Zener diode")),
    ("zD*", Fixed("full Zener diode")),
    ("full ZZener diode", Fixed("full ZZener diode")),
    ("zzD*", Fixed("full ZZener diode")),
    ("full tunnel diode", Fixed("full tunnel diode")),
    ("tD*", Fixed("full tunnel diode")),
    ("full photodiode", Fixed("full photodiode")),
    ("pD*", Fixed("full photodiode")),
    ("full led", Fixed("full led")),
    ("leD*", Fixed("full led")),
    ("full laser diode", Fixed("full laser diode")),
    ("lasD*", Fixed("full laser diode")),
    ("full varcap", Fixed("full varcap")),
    ("VC*", Fixed("full varcap")),
    ("full TVS diode", Fixed("full TVS diode")),
    ("tvsD*", Fixed("full TVS diode")),
    ("full Shockley diode", Fixed("full Shockley diode")),
    ("shD*", Fixed("full Shockley diode")),
    ("full bidirectionaldiode", Fixed("full bidirectionaldiode")),
    ("biD*", Fixed("full bidirectionaldiode")),
    ("stroke diode", Fixed("stroke diode")),
    ("D-", Fixed("stroke diode")),
    ("stroke Schottky diode", Fixed("stroke Schottky diode")),
    ("sD-", Fixed("stroke Schottky diode")),
    ("stroke Zener diode", Fixed("stroke Zener diode")),
    ("zD-", Fixed("stroke Zener diode")),
    ("stroke ZZener diode", Fixed("stroke ZZener diode")),
    ("zzD-", Fixed("stroke ZZener diode")),
    ("stroke tunnel diode", Fixed("stroke tunnel diode")),
    ("tD-", Fixed("stroke tunnel diode")),
    ("stroke photodiode", Fixed("stroke photodiode")),
    ("pD-", Fixed("stroke photodiode")),
    ("stroke led", Fixed("stroke led")),
    ("leD-", Fixed("stroke led")),
    ("stroke laser diode", Fixed("stroke laser diode")),
    ("lasD-", Fixed("stroke laser diode")),
    ("stroke varcap", Fixed("stroke varcap")),
    ("VC-", Fixed("stroke varcap")),
    ("stroke TVS diode", Fixed("stroke TVS diode")),
    ("tvsD-", Fixed("stroke TVS diode")),
    ("empty triac", Fixed("empty triac")),
    ("Tro", Fixed("empty triac")),
    ("full triac", Fixed("full triac")),
    ("Tr*", Fixed("full triac")),
    ("empty thyristor", Fixed("empty thyristor")),
    ("Tyo", Fixed("empty thyristor")),
    ("full thyristor", Fixed("full thyristor")),
    ("Ty*", Fixed("full thyristor")),
    ("stroke thyristor", Fixed("stroke thyristor")),
    ("Ty-", Fixed("stroke thyristor")),
    ("empty put", Fixed("empty put")),
    ("PUTo", Fixed("empty put")),
    ("full put", Fixed("full put")),
    ("PUT*", Fixed("full put")),
    ("stroke put", Fixed("stroke put")),
    ("PUT-", Fixed("stroke put")),
    ("empty gto", Fixed("empty gto")),
    ("GTOo", Fixed("empty gto")),
    ("full gto", Fixed("full gto")),
    ("GTO*", Fixed("full gto")),
    ("stroke gto", Fixed("stroke gto")),
    ("GTO-", Fixed("stroke gto")),
    ("empty gtobar", Fixed("empty gtobar")),
    ("GTObo", Fixed("empty gtobar")),
    ("full gtobar", Fixed("full gtobar")),
    ("GTOb*", Fixed("full gtobar")),
    ("stroke gtobar", Fixed("stroke gtobar")),
    ("GTOb-", Fixed("stroke gtobar")),
    ("empty agtobar", Fixed("empty agtobar")),
    ("aGTObo", Fixed("empty agtobar")),
    ("full agtobar", Fixed("full agtobar")),
    ("aGTOb*", Fixed("full agtobar")),
    ("stroke agtobar", Fixed("stroke agtobar")),
    ("aGTOb-", Fixed("stroke agtobar")),
    ("empty igct", Fixed("empty igct")),
    ("IGCTo", Fixed("empty igct")),
    ("full igct", Fixed("full igct")),
    ("IGCT*", Fixed("full igct")),
    ("stroke igct", Fixed("stroke igct")),
    ("IGCT-", Fixed("stroke igct")),
];

/// The switches' keys (`switch`, `spst`, `cute open switch`…).
const SWITCH_ALIASES: &[(&str, Target)] = &[
    ("switch", Fixed("switch")),
    ("spst", Fixed("switch")),
    ("closing switch", Fixed("closing switch")),
    ("cspst", Fixed("closing switch")),
    ("opening switch", Fixed("opening switch")),
    ("ospst", Fixed("opening switch")),
    ("normal open switch", Fixed("normal open switch")),
    ("nos", Fixed("normal open switch")),
    ("normal closed switch", Fixed("normal closed switch")),
    ("ncs", Fixed("normal closed switch")),
    ("opening normal closed switch", Fixed("opening normal closed switch")),
    ("oncs", Fixed("opening normal closed switch")),
    ("closing normal closed switch", Fixed("closing normal closed switch")),
    ("cncs", Fixed("closing normal closed switch")),
    ("opening normal open switch", Fixed("opening normal open switch")),
    ("onos", Fixed("opening normal open switch")),
    ("closing normal open switch", Fixed("closing normal open switch")),
    ("cnos", Fixed("closing normal open switch")),
    ("push button", Fixed("push button")),
    ("normally open push button", Fixed("push button")),
    ("nopb", Fixed("push button")),
    ("normally closed push button", Fixed("normally closed push button")),
    ("ncpb", Fixed("normally closed push button")),
    ("normally open push button closed", Fixed("normally open push button closed")),
    ("nopbc", Fixed("normally open push button closed")),
    ("normally closed push button open", Fixed("normally closed push button open")),
    ("ncpbo", Fixed("normally closed push button open")),
    ("toggle switch", Fixed("toggle switch")),
    ("reed", Fixed("reed")),
    ("cute closed switch", Fixed("cute closed switch")),
    ("ccsw", Fixed("cute closed switch")),
    ("cute open switch", Fixed("cute open switch")),
    ("cosw", Fixed("cute open switch")),
    ("cute closing switch", Fixed("cute closing switch")),
    ("ccgsw", Fixed("cute closing switch")),
    ("cute opening switch", Fixed("cute opening switch")),
    ("cogsw", Fixed("cute opening switch")),
    ("inline proximeter", Fixed("inline proximeter")),
    ("bare jumper", Fixed("bare jumper")),
    ("open jumper", Fixed("open jumper")),
    ("closed jumper", Fixed("closed jumper")),
    ("three-pins jumper", Fixed("three-pins jumper")),
    ("open solder jumper", Fixed("open solder jumper")),
    ("closed solder jumper", Fixed("closed solder jumper")),
    ("open double solder jumper", Fixed("open double solder jumper")),
    ("left double solder jumper", Fixed("left double solder jumper")),
    ("right double solder jumper", Fixed("right double solder jumper")),
    ("closed double solder jumper", Fixed("closed double solder jumper")),
];

/// The transistors on a path: `Tnpn=Q` writes `Q` in its middle, as `Telmech=M` does.
const TRANSISTOR_ALIASES: &[(&str, Target)] = &[
    ("Telmech", Fixed("Telmech")),
    ("inline not", Fixed("inline not")),
    ("inline buffer", Fixed("inline buffer")),
    ("inline schmitt", Fixed("inline schmitt")),
    ("inline invschmitt", Fixed("inline invschmitt")),
    ("inline tgate", Fixed("inline tgate")),
    ("inline double tgate", Fixed("inline double tgate")),
    ("TLnigbt", Fixed("TLnigbt")),
    ("TLpigbt", Fixed("TLpigbt")),
    ("Themt", Fixed("Themt")),
    ("Tnfet", Fixed("Tnfet")),
    ("Tnigbt", Fixed("Tnigbt")),
    ("Tnigfetd", Fixed("Tnigfetd")),
    ("Tnigfete", Fixed("Tnigfete")),
    ("Tnigfetebulk", Fixed("Tnigfetebulk")),
    ("Tnjfet", Fixed("Tnjfet")),
    ("Tnmos", Fixed("Tnmos")),
    ("Tnmosd", Fixed("Tnmosd")),
    ("Tnpn", Fixed("Tnpn")),
    ("Tpfet", Fixed("Tpfet")),
    ("Tpigbt", Fixed("Tpigbt")),
    ("Tpigfetd", Fixed("Tpigfetd")),
    ("Tpigfete", Fixed("Tpigfete")),
    ("Tpigfetebulk", Fixed("Tpigfetebulk")),
    ("Tpjfet", Fixed("Tpjfet")),
    ("Tpmos", Fixed("Tpmos")),
    ("Tpmosd", Fixed("Tpmosd")),
    ("Tpnp", Fixed("Tpnp")),
];

/// The bipoles' node shapes, as CircuiTikZ names them (`resistorshape`, `fulllediodeshape`), and the
/// key of the component each draws: `node[resistorshape]`, a double bipole's `double bipole L`.
pub const BIPOLE_SHAPES: &[(&str, &str)] = &[
    ("afuseshape", "afuse"),
    ("agenericshape", "ageneric"),
    ("americangasfilledsurgearrestershape", "american gas filled surge arrester"),
    ("americaninductorshape", "american inductor"),
    ("ammetershape", "ammeter"),
    ("baerttyshape", "baertty"),
    ("barriershape", "barrier"),
    ("battery1shape", "battery1"),
    ("battery2shape", "battery2"),
    ("batteryshape", "battery"),
    ("bjumpershape", "bare jumper"),
    ("bmultiwireshape", "bmultiwire"),
    ("bulbshape", "bulb"),
    ("buzzershape", "buzzer"),
    ("capacitivesensshape", "capacitive sensor"),
    ("capacitorshape", "capacitor"),
    ("ccapacitorshape", "curved capacitor"),
    ("cdsjumpershape", "closed double solder jumper"),
    ("cisourceAMshape", "american controlled current source"),
    ("cisourceCshape", "cute european controlled current source"),
    ("cisourceshape", "european controlled current source"),
    ("cisourcesinshape", "controlled sinusoidal current source"),
    ("cjumpershape", "closed jumper"),
    ("cncsshape", "closing normal closed switch"),
    ("cnosshape", "closing normal open switch"),
    ("cpeshape", "cpe"),
    ("crossingshape", "crossing"),
    ("csjumpershape", "closed solder jumper"),
    ("cspstshape", "switch"),
    ("currtapshape", "currtap"),
    ("cutechokeshape", "cute choke"),
    ("cuteclosedswitchshape", "cute closed switch"),
    ("cuteclosingswitchshape", "cute closing switch"),
    ("cuteinductorshape", "cute inductor"),
    ("cuteopeningswitchshape", "cute opening switch"),
    ("cuteopenswitchshape", "cute open switch"),
    ("cvsourceAMshape", "american controlled voltage source"),
    ("cvsourceCshape", "cute european controlled voltage source"),
    ("cvsourceshape", "european controlled voltage source"),
    ("cvsourcesinshape", "controlled sinusoidal voltage source"),
    ("dampershape", "damper"),
    ("dcisourceshape", "dcisource"),
    ("dcvsourceshape", "dcvsource"),
    ("ecapacitorshape", "ecapacitor"),
    ("ecsourceshape", "empty controlled source"),
    ("emptybidirectionaldiodeshape", "empty bidirectionaldiode"),
    ("emptydiodeshape", "empty diode"),
    ("emptylaserdiodeshape", "empty laser diode"),
    ("emptylediodeshape", "empty led"),
    ("emptypdiodeshape", "empty photodiode"),
    ("emptysdiodeshape", "empty Schottky diode"),
    ("emptyshdiodeshape", "empty Shockley diode"),
    ("emptytdiodeshape", "empty tunnel diode"),
    ("emptytvsdiodeshape", "empty TVS diode"),
    ("emptyvarcapshape", "empty varcap"),
    ("emptyzdiodeshape", "empty Zener diode"),
    ("emptyzzdiodeshape", "empty ZZener diode"),
    ("esourceshape", "esource"),
    ("europeangasfilledsurgearrestershape", "european gas filled surge arrester"),
    ("ferrocapshape", "feC"),
    ("fullagtobarshape", "full agtobar"),
    ("fullbidirectionaldiodeshape", "full bidirectionaldiode"),
    ("fulldiodeshape", "full diode"),
    ("fullgtobarshape", "full gtobar"),
    ("fullgtoshape", "full gto"),
    ("fulligctshape", "full igct"),
    ("fulllaserdiodeshape", "full laser diode"),
    ("fulllediodeshape", "full led"),
    ("fullpdiodeshape", "full photodiode"),
    ("fullputshape", "full put"),
    ("fullsdiodeshape", "full Schottky diode"),
    ("fullshdiodeshape", "full Shockley diode"),
    ("fulltdiodeshape", "full tunnel diode"),
    ("fullthyristorshape", "full thyristor"),
    ("fulltriacshape", "full triac"),
    ("fulltvsdiodeshape", "full TVS diode"),
    ("fullvarcapshape", "full varcap"),
    ("fullzdiodeshape", "full Zener diode"),
    ("fullzzdiodeshape", "full ZZener diode"),
    ("fuseshape", "fuse"),
    ("genericpotentiometershape", "european potentiometer"),
    ("genericshape", "european resistor"),
    ("iecconnshape", "iec connector"),
    ("iloop2shape", "iloop2"),
    ("iloopshape", "iloop"),
    ("inductorshape", "european inductor"),
    ("inertershape", "inerter"),
    ("inlineproximetershape", "inline proximeter"),
    ("isourceAMshape", "american current source"),
    ("isourceCshape", "cute european current source"),
    ("isourceNshape", "noise current source"),
    ("isourceshape", "european current source"),
    ("isourcesinshape", "sinusoidal current source"),
    ("lampshape", "lamp"),
    ("ldgenericshape", "european light dependent resistor"),
    ("ldresistorshape", "american light dependent resistor"),
    ("ldsjumpershape", "left double solder jumper"),
    ("loudspeakershape", "loudspeaker"),
    ("massshape", "mass"),
    ("memristorshape", "memristor"),
    ("micshape", "mic"),
    ("movshape", "mov"),
    ("multiwireshape", "multiwire"),
    ("ncpushbuttonoshape", "normally closed push button open"),
    ("ncpushbuttonshape", "normally closed push button"),
    ("ncsshape", "normal closed switch"),
    ("neonlampacshape", "neonlampac"),
    ("neonlampccshape", "neonlampcc"),
    ("noratorshape", "norator"),
    ("nosshape", "normal open switch"),
    ("nullatorshape", "nullator"),
    ("odsjumpershape", "open double solder jumper"),
    ("ohmmetershape", "ohmmeter"),
    ("ojumpershape", "open jumper"),
    ("oncsshape", "opening normal closed switch"),
    ("onosshape", "opening normal open switch"),
    ("ooosourceshape", "ooosource"),
    ("oosourceatransshape", "oosourceatrans"),
    ("oosourceshape", "ioosource"),
    ("oosourcetransshape", "oosourcetrans"),
    ("openbarriershape", "openbarrier"),
    ("openshape", "open"),
    ("oscopeshape", "oscope"),
    ("osjumpershape", "open solder jumper"),
    ("ospstshape", "opening switch"),
    ("photoresistorshape", "phR"),
    ("piezoelectricshape", "piezoelectric"),
    ("potentiometershape", "american potentiometer"),
    ("pushbuttoncshape", "normally open push button closed"),
    ("pushbuttonshape", "push button"),
    ("pvmoduleshape", "pvmodule"),
    ("pvsourceshape", "pvsource"),
    ("qiprobeshape", "qiprobe"),
    ("qpprobeshape", "qpprobe"),
    ("qvprobeshape", "qvprobe"),
    ("rbuzzershape", "rbuzzer"),
    ("rdsjumpershape", "right double solder jumper"),
    ("reedshape", "reed"),
    ("relaisshape", "relais"),
    ("resistivesensshape", "american resistive sensor"),
    ("resistorshape", "american resistor"),
    ("rmetershape", "rmeter"),
    ("rmeterwashape", "rmeterwa"),
    ("sgenericshape", "sgeneric"),
    ("shortshape", "short"),
    ("singenericshape", "singeneric"),
    ("smetershape", "smeter"),
    ("solarshape", "solar"),
    ("sparkgapshape", "sparkgap"),
    ("springshape", "spring"),
    ("squidshape", "squid"),
    ("tgenericshape", "variable european resistor"),
    ("thermistorntcshape", "thRn"),
    ("thermistorptcshape", "thRp"),
    ("thermistorshape", "european resistive sensor"),
    ("thermocoupleshape", "thermocouple"),
    ("tjumpershape", "three-pins jumper"),
    ("tlmicshape", "tlmic"),
    ("tmultiwireshape", "tmultiwire"),
    ("toggleswitchshape", "toggle switch"),
    ("varistorshape", "varistor"),
    ("vcapacitorshape", "variable capacitor"),
    ("viscoeshape", "viscoe"),
    ("voltmetershape", "voltmeter"),
    ("vresistorshape", "variable american resistor"),
    ("vsourceAMshape", "american voltage source"),
    ("vsourceCshape", "cute european voltage source"),
    ("vsourceNshape", "noise voltage source"),
    ("vsourceshape", "european voltage source"),
    ("vsourcesinshape", "sinusoidal voltage source"),
    ("vsourcesquareshape", "square voltage source"),
    ("vsourcetrishape", "vsourcetri"),
    ("wbulbshape", "wbulb"),
    ("wfuseshape", "wfuse"),
    ("xgenericshape", "xgeneric"),
];

/// The bipoles' node shapes, with the blocks'.
pub fn bipole_shapes() -> impl Iterator<Item = &'static (&'static str, &'static str)> {
    BIPOLE_SHAPES.iter().chain(blocks::SHAPES)
}

pub fn find(name: &str) -> Option<&'static Component> {
    GROUPS.iter().flat_map(|g| g.iter()).find(|c| c.name == name)
}

/// Every key that names a component, and what it names.
pub fn aliases() -> impl Iterator<Item = &'static (&'static str, Target)> {
    ALIASES.iter().chain(SOURCE_ALIASES).chain(OTHER_ALIASES).chain(DIODE_ALIASES).chain(SWITCH_ALIASES).chain(TRANSISTOR_ALIASES).chain(blocks::ALIASES)
}

pub fn target(key: &str) -> Option<Target> {
    aliases().find(|(k, _)| *k == key).map(|(_, t)| *t)
}

/// Whether a component is a source: its value (`V=…`, `I=…`) is its voltage or its current, not a
/// label, and its voltages and currents follow the rules of generators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Not,
    Voltage,
    Current,
}

impl Component {
    pub fn source(&self) -> Source {
        if self.source_volts().is_some() {
            Source::Voltage
        } else if matches!(self.class, Class::Sources | Class::CSources) && !matches!(self.name, "nullator" | "norator") {
            Source::Current
        } else {
            Source::Not
        }
    }
}

/// The sources' keys: `V`, `vsource` and `voltage source` follow the `voltage` style (European or
/// American), `I`, `isource` and `current source` the `current` style.
const SOURCE_ALIASES: &[(&str, Target)] = &[
    ("battery", Fixed("battery")),
    ("battery1", Fixed("battery1")),
    ("battery2", Fixed("battery2")),
    ("solar", Fixed("solar")),
    ("baertty", Fixed("baertty")),
    ("V", Voltage("european voltage source", "american voltage source")),
    ("vsource", Voltage("european voltage source", "american voltage source")),
    ("voltage source", Voltage("european voltage source", "american voltage source")),
    ("european voltage source", Fixed("european voltage source")),
    ("vsourceEU", Fixed("european voltage source")),
    ("cute european voltage source", Fixed("cute european voltage source")),
    ("vsourceC", Fixed("cute european voltage source")),
    ("ceV", Fixed("cute european voltage source")),
    ("american voltage source", Fixed("american voltage source")),
    ("vsourceAM", Fixed("american voltage source")),
    ("I", Current("european current source", "american current source")),
    ("isource", Current("european current source", "american current source")),
    ("current source", Current("european current source", "american current source")),
    ("european current source", Fixed("european current source")),
    ("isourceEU", Fixed("european current source")),
    ("cute european current source", Fixed("cute european current source")),
    ("isourceC", Fixed("cute european current source")),
    ("ceI", Fixed("cute european current source")),
    ("american current source", Fixed("american current source")),
    ("isourceAM", Fixed("american current source")),
    ("sinusoidal voltage source", Fixed("sinusoidal voltage source")),
    ("vsourcesin", Fixed("sinusoidal voltage source")),
    ("sV", Fixed("sinusoidal voltage source")),
    ("sinusoidal current source", Fixed("sinusoidal current source")),
    ("isourcesin", Fixed("sinusoidal current source")),
    ("sI", Fixed("sinusoidal current source")),
    ("cV", Voltage("european controlled voltage source", "american controlled voltage source")),
    ("cvsource", Voltage("european controlled voltage source", "american controlled voltage source")),
    ("controlled voltage source", Voltage("european controlled voltage source", "american controlled voltage source")),
    ("european controlled voltage source", Fixed("european controlled voltage source")),
    ("cvsourceEU", Fixed("european controlled voltage source")),
    ("cute european controlled voltage source", Fixed("cute european controlled voltage source")),
    ("cvsourceC", Fixed("cute european controlled voltage source")),
    ("cceV", Fixed("cute european controlled voltage source")),
    ("american controlled voltage source", Fixed("american controlled voltage source")),
    ("cvsourceAM", Fixed("american controlled voltage source")),
    ("cI", Current("european controlled current source", "american controlled current source")),
    ("cisource", Current("european controlled current source", "american controlled current source")),
    ("controlled current source", Current("european controlled current source", "american controlled current source")),
    ("european controlled current source", Fixed("european controlled current source")),
    ("cisourceEU", Fixed("european controlled current source")),
    ("cute european controlled current source", Fixed("cute european controlled current source")),
    ("cisourceC", Fixed("cute european controlled current source")),
    ("cceI", Fixed("cute european controlled current source")),
    ("american controlled current source", Fixed("american controlled current source")),
    ("cisourceAM", Fixed("american controlled current source")),
    ("empty controlled source", Fixed("empty controlled source")),
    ("ecsource", Fixed("empty controlled source")),
    ("controlled sinusoidal voltage source", Fixed("controlled sinusoidal voltage source")),
    ("controlled vsourcesin", Fixed("controlled sinusoidal voltage source")),
    ("cvsourcesin", Fixed("controlled sinusoidal voltage source")),
    ("csV", Fixed("controlled sinusoidal voltage source")),
    ("controlled sinusoidal current source", Fixed("controlled sinusoidal current source")),
    ("controlled isourcesin", Fixed("controlled sinusoidal current source")),
    ("cisourcesin", Fixed("controlled sinusoidal current source")),
    ("csI", Fixed("controlled sinusoidal current source")),
    ("noise voltage source", Fixed("noise voltage source")),
    ("vsourceN", Fixed("noise voltage source")),
    ("nV", Fixed("noise voltage source")),
    ("noise current source", Fixed("noise current source")),
    ("isourceN", Fixed("noise current source")),
    ("nI", Fixed("noise current source")),
    ("square voltage source", Fixed("square voltage source")),
    ("vsourcesquare", Fixed("square voltage source")),
    ("sqV", Fixed("square voltage source")),
    ("vsourcetri", Fixed("vsourcetri")),
    ("tV", Fixed("vsourcetri")),
    ("esource", Fixed("esource")),
    ("pvsource", Fixed("pvsource")),
    ("pvmodule", Fixed("pvmodule")),
    ("ioosource", Fixed("ioosource")),
    ("voosource", Fixed("voosource")),
    ("oosourceatrans", Fixed("oosourceatrans")),
    ("oosourcetrans", Fixed("oosourcetrans")),
    ("ooosource", Fixed("ooosource")),
    ("nullator", Fixed("nullator")),
    ("norator", Fixed("norator")),
    ("dcvsource", Fixed("dcvsource")),
    ("dcisource", Fixed("dcisource")),
];

/// The instruments', the mechanical elements' and the miscellaneous ones' keys.
const OTHER_ALIASES: &[(&str, Target)] = &[
    ("rmeter", Fixed("rmeter")),
    ("rmeterwa", Fixed("rmeterwa")),
    ("ammeter", Fixed("ammeter")),
    ("voltmeter", Fixed("voltmeter")),
    ("ohmmeter", Fixed("ohmmeter")),
    ("smeter", Fixed("smeter")),
    ("qiprobe", Fixed("qiprobe")),
    ("qvprobe", Fixed("qvprobe")),
    ("qpprobe", Fixed("qpprobe")),
    ("oscope", Fixed("oscope")),
    ("iloop", Fixed("iloop")),
    ("iloop2", Fixed("iloop2")),
    ("currtap", Fixed("currtap")),
    ("damper", Fixed("damper")),
    ("inerter", Fixed("inerter")),
    ("spring", Fixed("spring")),
    ("viscoe", Fixed("viscoe")),
    ("mass", Fixed("mass")),
    ("thermocouple", Fixed("thermocouple")),
    ("fuse", Fixed("fuse")),
    ("afuse", Fixed("afuse")),
    ("asymmetric fuse", Fixed("afuse")),
    ("wfuse", Fixed("wfuse")),
    ("wiggly fuse", Fixed("wfuse")),
    ("relais", Fixed("relais")),
    ("squid", Fixed("squid")),
    ("barrier", Fixed("barrier")),
    ("openbarrier", Fixed("openbarrier")),
    ("european gas filled surge arrester", Fixed("european gas filled surge arrester")),
    ("american gas filled surge arrester", Fixed("american gas filled surge arrester")),
    ("lamp", Fixed("lamp")),
    ("bulb", Fixed("bulb")),
    ("wbulb", Fixed("wbulb")),
    ("neonlampcc", Fixed("neonlampcc")),
    ("neonlampac", Fixed("neonlampac")),
    ("sparkgap", Fixed("sparkgap")),
    ("sparkgap/circle", Fixed("sparkgap circle")),
    ("sparkgap/dot", Fixed("sparkgap dot circle")),
    ("loudspeaker", Fixed("loudspeaker")),
    ("mic", Fixed("mic")),
    ("tlmic", Fixed("tlmic")),
    ("buzzer", Fixed("buzzer")),
    ("rbuzzer", Fixed("rbuzzer")),
];

/// A zig-zag of `count` teeth from `-half` to `half`, `high` on each side, the first tooth up, from
/// half a stroke (`thick`) before `-half` to as much after `half`.
pub fn zigzag(half: f64, high: f64, thick: f64, count: usize) -> Vec<Op> {
    let n = 2 * count.max(1);
    let step = 2.0 * half / n as f64;
    let mut ops = vec![Op::Move(-half - thick / 2.0, 0.0)];
    for i in 0..n {
        let y = if i % 2 == 0 { high } else { -high };
        ops.push(Op::Line(-half + (i as f64 + 0.5) * step, y));
    }
    ops.push(Op::Line(half + thick / 2.0, 0.0));
    ops
}

/// `arches` arches joined by small loops below the line, from `-half` to `half`: each arch is half an
/// ellipse `h` high, each loop half an ellipse half as high and as wide as half the arch's height.
pub fn coil(half: f64, h: f64, arches: usize) -> Vec<Op> {
    let back = h / 2.0;
    let arch = (2.0 * half + (arches - 1) as f64 * back) / arches as f64;
    let mut ops = vec![Op::Move(-half, 0.0)];
    let mut x = -half;
    for i in 0..arches {
        half_ellipse(&mut ops, x, arch / 2.0, h);
        x += arch;
        if i + 1 < arches {
            half_ellipse(&mut ops, x, -back / 2.0, -h / 2.0);
            x -= back;
        }
    }
    ops
}

/// `arches` arches side by side from `-half` to `half`, each half an ellipse `h` high.
pub fn arches(half: f64, h: f64, arches: usize) -> Vec<Op> {
    let arch = 2.0 * half / arches as f64;
    let mut ops = vec![Op::Move(-half, 0.0)];
    for i in 0..arches {
        half_ellipse(&mut ops, -half + i as f64 * arch, arch / 2.0, h);
    }
    ops
}

/// Half an ellipse from `(x, 0)` to `(x + 2rx, 0)` through `(x + rx, ry)`, as two quarter Béziers.
fn half_ellipse(ops: &mut Vec<Op>, x: f64, rx: f64, ry: f64) {
    const K: f64 = 0.552_284_75;
    let c = x + rx;
    ops.push(Op::Curve([x, K * ry], [c - K * rx, ry], [c, ry]));
    ops.push(Op::Curve([c + K * rx, ry], [c + rx, K * ry], [c + rx, 0.0]));
}

/// The head of the arrows of tunable components and potentiometers (CircuiTikZ's "latexslim"), for
/// a stroke `width` cm wide: its outline from the tip, as (back, side) pairs in units of its length,
/// and its length. The line stops `ARROW_INSET` of the length before the tip, inside the head.
pub const ARROW_HEAD: [[f64; 2]; 9] = [[0.25, 0.05], [0.70, 0.15], [1.0, 0.375], [0.75, 0.1], [0.75, -0.1], [1.0, -0.375], [0.70, -0.15], [0.25, -0.05], [0.0, 0.0]];
pub const ARROW_INSET: f64 = 0.6;

/// The length of an arrow head for a stroke `width` cm wide (measured: 2.79 bp and three widths).
pub fn arrow_length(width: f64) -> f64 {
    2.79 / 72.0 * 2.54 + 3.0 * width
}

/// The traces of an oscilloscope's screen (`bipoles/oscope/waveform`), measured at its default size.
pub const WAVEFORMS: &[(&str, &[Op])] = &[
    ("ramps", &[Op::Move(-0.225, -0.075), Op::Line(-0.015, 0.075), Op::Line(-0.015, -0.075), Op::Line(0.195, 0.075), Op::Line(0.195, -0.075)]),
    (
        "sin",
        &[
            Op::Move(-0.18, 0.0),
            Op::Curve([-0.1507, 0.0614], [-0.1226, 0.12], [-0.0901, 0.12]),
            Op::Curve([-0.0575, 0.12], [-0.0294, 0.0614], [0.0, 0.0]),
            Op::Curve([0.0294, -0.0614], [0.0574, -0.12], [0.09, -0.12]),
            Op::Curve([0.1226, -0.12], [0.1506, -0.0614], [0.18, 0.0]),
        ],
    ),
    (
        "square",
        &[
            Op::Move(-0.225, -0.075),
            Op::Line(-0.18, -0.075),
            Op::Line(-0.18, 0.075),
            Op::Line(0.0, 0.075),
            Op::Line(0.0, -0.075),
            Op::Line(0.18, -0.075),
            Op::Line(0.18, 0.075),
            Op::Line(0.225, 0.075),
        ],
    ),
    (
        "triangle",
        &[Op::Move(-0.225, 0.0), Op::Line(-0.18, -0.075), Op::Line(-0.09, 0.075), Op::Line(0.0, -0.075), Op::Line(0.09, 0.075), Op::Line(0.18, -0.075), Op::Line(0.225, 0.0)],
    ),
    (
        "lissajous",
        &[
            Op::Move(0.15, 0.105),
            Op::Curve([0.1003, 0.1382], [-0.0072, 0.118], [-0.09, 0.0601]),
            Op::Curve([-0.1729, 0.0021], [-0.1997, -0.0719], [-0.15, -0.105]),
            Op::Curve([-0.1003, -0.1381], [0.0072, -0.118], [0.09, -0.0599]),
            Op::Curve([0.1729, -0.002], [0.1997, 0.0719], [0.15, 0.105]),
            Op::Close,
        ],
    ),
    ("zero", &[Op::Move(-0.225, 0.0), Op::Line(0.225, 0.0)]),
    ("none", &[]),
];
