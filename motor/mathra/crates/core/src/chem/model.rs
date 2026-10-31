//! Tree of meaning for chemistry: what is drawn, not where. A scheme is a sequence of compounds,
//! plus signs and reaction arrows; a molecule is a graph of atoms and bonds, with the author's
//! drawing directions (chemfig's angles) kept as hints for the layout.

use serde::Serialize;

/// Index of an atom in [`Molecule::atoms`].
pub type AtomId = usize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Scheme {
    pub items: Vec<Item>,
    /// Curved arrows of a mechanism (`\chemmove`), between named atoms (`@{name}`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub moves: Vec<ElectronMove>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Item {
    Compound(Compound),
    Plus,
    Arrow(Arrow),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Compound {
    pub body: Body,
    /// `\chemname{…}{name}`: written under the compound (LaTeX text, `$…$` for math).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Body {
    /// A structural formula (`\chemfig`).
    Structure(Molecule),
    /// A formula or any math, typeset by mathra: `\ce{2H2O}`, `$\Delta H$`.
    Math { tex: String },
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Molecule {
    pub atoms: Vec<Atom>,
    pub bonds: Vec<Bond>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rings: Vec<Ring>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Atom {
    /// The label as written (`H_3C`, `NH_2^+`); empty for a carbon at a vertex of the skeleton.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub label: String,
    /// The label split into atoms (`H_3C` → `H_3`, `C`): bonds attach to one of them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pieces: Vec<String>,
    /// The piece that sits on the atom's position (the one a bond arrives at).
    #[serde(skip_serializing_if = "is_zero")]
    pub anchor: usize,
    /// Lone pairs, radicals and pair lines around the atom (`\lewis`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lewis: Vec<Lewis>,
    /// `@{name}`: the name a mechanism arrow uses to point at this atom.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook: Option<String>,
}

fn is_zero(v: &usize) -> bool {
    *v == 0
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Lewis {
    /// Direction in degrees, counterclockwise from the right.
    pub angle: f32,
    pub mark: LewisMark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LewisMark {
    /// `:` two dots
    Pair,
    /// `.` one dot
    Radical,
    /// `|` a short line for a pair
    Line,
    /// `o` an empty box: a vacancy
    Vacancy,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Bond {
    pub from: AtomId,
    pub to: AtomId,
    pub kind: BondKind,
    /// Drawing direction in degrees, counterclockwise from the right (chemfig's angle, resolved).
    pub angle: f32,
    /// Length as a multiple of the standard bond.
    #[serde(skip_serializing_if = "is_one")]
    pub length: f32,
    /// The piece of each label the bond leaves from and arrives at.
    #[serde(skip_serializing_if = "is_zero")]
    pub departure: usize,
    #[serde(skip_serializing_if = "is_zero")]
    pub arrival: usize,
    /// The ring this bond is a side of: a double bond is then drawn inside it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ring: Option<usize>,
}

fn is_one(v: &f32) -> bool {
    *v == 1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BondKind {
    Single,
    Double,
    Triple,
    /// Cram wedge, narrow at the start, wide at the end (`>`); `WedgeBack` is the reverse (`<`).
    Wedge,
    WedgeBack,
    /// Hashed wedge (`>:`, `<:`).
    Hash,
    HashBack,
    /// Hollow wedge (`>|`, `<|`).
    Hollow,
    HollowBack,
    /// A dashed line: a partial or hydrogen bond.
    Dashed,
    /// Present in the graph, not drawn (`draw=none`).
    Invisible,
}

impl BondKind {
    /// How many electron pairs the bond shares, for the hydrogens a carbon carries.
    pub fn order(self) -> u32 {
        match self {
            BondKind::Double => 2,
            BondKind::Triple => 3,
            BondKind::Dashed | BondKind::Invisible => 0,
            _ => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ring {
    pub size: usize,
    /// The atoms around the ring, in drawing order.
    pub atoms: Vec<AtomId>,
    /// `**6(…)`: a circle inside for the delocalized bonds.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub aromatic: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Arrow {
    pub kind: ArrowKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub above: Option<Label>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub below: Option<Label>,
    /// Direction in degrees (0: to the right, -90: down).
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub angle: f32,
    #[serde(skip_serializing_if = "is_one")]
    pub length: f32,
}

fn is_zero_f32(v: &f32) -> bool {
    *v == 0.0
}

/// A label over or under a reaction arrow: text (`$…$` for math) or a structure (a reagent drawn).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Label {
    Text { tex: String },
    Structure(Molecule),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArrowKind {
    /// `->` reacts to give
    Forward,
    /// `<-`
    Backward,
    /// `<=>` equilibrium: two harpoons
    Equilibrium,
    /// `<->>` equilibrium favoring the products; `<<->` favoring the reactants
    EquilibriumRight,
    EquilibriumLeft,
    /// `<->` resonance: one shaft, a head at each end
    Resonance,
    /// `=>` retrosynthesis: an open double arrow
    Retro,
    /// `-/>` does not react: crossed
    NoReaction,
    /// `0` nothing drawn: only the labels, or space
    Invisible,
}

/// A curved arrow of a mechanism: an electron pair (or one electron) moving between two atoms.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ElectronMove {
    pub from: String,
    pub to: String,
    /// Leaving and arriving directions in degrees (TikZ's `controls +(θ:l)` and `out=`/`in=`); without
    /// them the arrow bends 45° to the left of the straight line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_angle: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_angle: Option<f32>,
    /// How far each control point reaches, in em; without it, 0.4 of the distance between the atoms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reach: Option<(f32, f32)>,
    /// `bend left=θ` (positive) or `bend right=θ` (negative), when no angle is given.
    pub bend: f32,
    /// A fishhook (half head): one electron.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub single: bool,
}
