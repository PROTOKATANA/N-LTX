//! Decoration and meta-decoration names, and the macros a declared decoration stores.

use crate::engine::Engine;
use crate::tex::Token;

/// A decoration (`\pgfdeclaredecoration`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dec {
    LineTo,
    MoveTo,
    CurveTo,
    Zigzag,
    Saw,
    RandomSteps,
    Bent,
    Snake,
    Coil,
    Bumps,
    Ticks,
    ExpandingWaves,
    Waves,
    Border,
    Brace,
    ShowPathConstruction,
    Markings,
    TextAlongPath,
    Triangles,
    Crosses,
    ShapeBackgrounds,
    Footprints,
    Koch1,
    Koch2,
    KochSnowflake,
    Cantor,
    TextEffects,
    /// the mindmap library's
    CircleConnectionBar,
    /// declared in the document with `\pgfdeclaredecoration`, run as TeX
    User(&'static str),
}

/// A meta-decoration (`\pgfdeclaremetadecoration`): `straight zigzag`, and TikZ's `tikz@internal`
/// that puts `pre` and `post` around the decoration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Meta {
    StraightZigzag,
    Internal,
    /// declared with `\pgfdeclaremetadecoration`
    User(&'static str),
}

/// A name kept for the rest of the run (state and decoration names of the document are few).
pub fn intern(s: &str) -> &'static str {
    thread_local! {
        static NAMES: std::cell::RefCell<std::collections::HashSet<&'static str>> = Default::default();
    }
    NAMES.with(|n| {
        let mut n = n.borrow_mut();
        if let Some(x) = n.get(s) {
            return *x;
        }
        let x: &'static str = Box::leak(s.to_string().into_boxed_str());
        n.insert(x);
        x
    })
}

/// The body of a macro, if defined.
pub(super) fn macro_body(e: &Engine, name: &str) -> Option<Vec<Token>> {
    match e.tex.meaning_of(name) {
        crate::tex::Meaning::Macro(m) => Some(crate::tex::substitute(&m.body, &[])),
        _ => None,
    }
}

/// A decoration by name: one declared in the document, else one of pgf's libraries.
pub fn resolve(e: &Engine, name: &str) -> Option<Dec> {
    let name = name.trim();
    if e.tex.is_defined(&format!("pgf@decorate@@{name}@initial")) {
        return Some(Dec::User(intern(name)));
    }
    lookup(name)
}

pub fn resolve_meta(e: &Engine, name: &str) -> Option<Meta> {
    let name = name.trim();
    if e.tex.is_defined(&format!("pgf@metadecoration@@{name}@initial")) {
        return Some(Meta::User(intern(name)));
    }
    lookup_meta(name)
}

pub fn lookup(name: &str) -> Option<Dec> {
    Some(match name.trim() {
        "lineto" => Dec::LineTo,
        "moveto" => Dec::MoveTo,
        "curveto" => Dec::CurveTo,
        "zigzag" => Dec::Zigzag,
        "saw" => Dec::Saw,
        "random steps" => Dec::RandomSteps,
        "bent" => Dec::Bent,
        "snake" => Dec::Snake,
        "coil" => Dec::Coil,
        "bumps" => Dec::Bumps,
        "ticks" => Dec::Ticks,
        "expanding waves" => Dec::ExpandingWaves,
        "waves" => Dec::Waves,
        "border" => Dec::Border,
        "brace" => Dec::Brace,
        "show path construction" => Dec::ShowPathConstruction,
        "markings" => Dec::Markings,
        "text along path" => Dec::TextAlongPath,
        "triangles" => Dec::Triangles,
        "crosses" => Dec::Crosses,
        "shape backgrounds" => Dec::ShapeBackgrounds,
        "footprints" => Dec::Footprints,
        "Koch curve type 1" => Dec::Koch1,
        "Koch curve type 2" => Dec::Koch2,
        "Koch snowflake" => Dec::KochSnowflake,
        "Cantor set" => Dec::Cantor,
        "text effects along path" => Dec::TextEffects,
        "circle connection bar" => Dec::CircleConnectionBar,
        _ => return None,
    })
}

pub fn lookup_meta(name: &str) -> Option<Meta> {
    match name.trim() {
        "straight zigzag" => Some(Meta::StraightZigzag),
        _ => None,
    }
}

pub(super) fn initial(d: Dec) -> &'static str {
    match d {
        Dec::LineTo | Dec::MoveTo | Dec::CurveTo | Dec::Saw | Dec::Snake | Dec::Bumps | Dec::ExpandingWaves | Dec::TextAlongPath | Dec::CircleConnectionBar => "initial",
        Dec::Zigzag => "up from center",
        Dec::RandomSteps => "start",
        Dec::Bent => "bent",
        Dec::Coil => "coil",
        Dec::Ticks => "ticks",
        Dec::Waves => "wave",
        Dec::Border => "tick",
        Dec::Brace => "brace",
        Dec::ShowPathConstruction => "check for moveto",
        Dec::Markings | Dec::Triangles | Dec::Crosses | Dec::Koch1 | Dec::Koch2 | Dec::KochSnowflake | Dec::Cantor => "init",
        Dec::ShapeBackgrounds => "initialise",
        Dec::TextEffects => "setup",
        Dec::Footprints => "left",
        Dec::User(_) => "initial",
    }
}
