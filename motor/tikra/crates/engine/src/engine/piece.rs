//! How the parts of TikZ outside the engine plug into it.
//!
//! A piece brings its commands and its definitions, and depends on the engine, which knows none of
//! them: an engine is built with any set of pieces, and each can be changed or replaced alone. What a
//! piece keeps during a figure is its own: [`Engine::piece_state`].

use std::any::Any;

use super::coords::Coord;
use super::{Engine, NATIVE_KEY_BASE};
use crate::extension::Library;
use crate::scene::{Point, Segment};
use crate::tex::{Meaning, Token};
use crate::Error;

/// A command of a piece: it reads its arguments from the input.
pub type CommandFn = fn(&mut Engine, &Token) -> Result<(), Error>;

/// A word a piece adds to paths (`graph`): it reads the rest of its operation from the input.
pub type PathOperationFn = fn(&mut Engine) -> Result<(), Error>;

/// A coordinate system a piece adds (`visualization cs`): from its options to a point.
pub type CoordinateSystemFn = fn(&mut Engine, &str) -> Result<Point, Error>;

/// What a piece does with a statement of TikZ before the engine runs it (an axis keeps them for
/// later): `Some` if it took it, with its name and its token.
pub type StatementFilterFn = fn(&mut Engine, &str, &Token) -> Option<Result<(), Error>>;

/// An anchor a piece adds to nodes (`near ticklabel`): where it is, if `name` is one of its. It gets
/// the anchors the node has through `local`.
pub type NodeAnchorFn = fn(&mut Engine, &str, &dyn Fn(&str) -> Option<Point>) -> Option<Point>;

/// A coordinate system a piece knows among many (`axis cs`, `rel axis cs`…), from its name and its
/// options: `None` if it is not one of its, or it does not apply now.
pub type CoordinateFallbackFn = fn(&mut Engine, &str, &str) -> Option<Result<Coord, Error>>;

/// What a piece reads in a plain `(x,y)` while it applies (inside an axis): `None` leaves it to the
/// engine.
pub type PlainCoordinateFn = fn(&mut Engine, &[String]) -> Option<Result<Point, Error>>;

/// A command in the text of a node that a piece runs as the text is read, in its place among the
/// others: it gets the tokens and where the command is, and returns where to go on.
pub type NodeTextFn = fn(&mut Engine, &[Token], usize) -> Result<usize, Error>;

/// One end of an environment of a piece, with its name.
pub type EnvironmentFn = fn(&mut Engine, &str) -> Result<(), Error>;

/// A decoration a piece does (`soft clip`): from the segments to what replaces them.
pub type DecorationFn = fn(&mut Engine, &[Segment]) -> Result<Vec<Segment>, Error>;

/// The first id of the commands of the pieces (`Meaning::Native`).
pub const PIECE_COMMAND_BASE: u16 = 1000;

pub struct Piece {
    /// Unique among the pieces of an engine, which is kept by their names.
    pub name: &'static str,
    pub commands: &'static [(&'static str, CommandFn)],
    pub path_operations: &'static [(&'static str, PathOperationFn)],
    pub coordinate_systems: &'static [(&'static str, CoordinateSystemFn)],
    pub coordinate_fallbacks: &'static [CoordinateFallbackFn],
    pub plain_coordinates: &'static [PlainCoordinateFn],
    pub node_text: &'static [(&'static str, NodeTextFn)],
    /// Name, begin and end.
    pub environments: &'static [(&'static str, EnvironmentFn, EnvironmentFn)],
    pub decorations: &'static [(&'static str, DecorationFn)],
    /// Its commands that read their own input (`\\datavisualization`, data line by line): `\\tikz`
    /// runs them as they come, instead of reading up to the `;` first.
    pub reading_input: &'static [&'static str],
    pub statement_filters: &'static [StatementFilterFn],
    pub node_anchors: &'static [NodeAnchorFn],
    /// What it sets up before TikZ's definitions run: registers, macros, keys.
    pub define: fn(&mut Engine),
    /// What it sets up after them, as they would overwrite it: a key it gives its own meaning, as
    /// `/tikz/mark options` in pgfplots.
    pub after_tikz: fn(&mut Engine),
    /// TeX run with TikZ's definitions, with `@` a letter.
    pub definitions: fn() -> String,
}

impl Piece {
    /// A piece with nothing: each says what it has, with `..Piece::EMPTY`.
    pub const EMPTY: Piece = Piece {
        name: "",
        commands: &[],
        path_operations: &[],
        coordinate_systems: &[],
        coordinate_fallbacks: &[],
        plain_coordinates: &[],
        node_text: &[],
        environments: &[],
        decorations: &[],
        reading_input: &[],
        statement_filters: &[],
        node_anchors: &[],
        define: |_| {},
        after_tikz: |_| {},
        definitions: String::new,
    };
}

/// What the pieces add to the work of the engine. It asks here only for what it does not know, so
/// they cost nothing to the figures that do not use them.
#[derive(Clone, Default)]
pub(super) struct Hooks {
    path_operations: Vec<(&'static str, PathOperationFn)>,
    coordinate_systems: Vec<(&'static str, CoordinateSystemFn)>,
    coordinate_fallbacks: Vec<CoordinateFallbackFn>,
    plain_coordinates: Vec<PlainCoordinateFn>,
    node_text: Vec<(&'static str, NodeTextFn)>,
    environments: Vec<(&'static str, EnvironmentFn, EnvironmentFn)>,
    decorations: Vec<(&'static str, DecorationFn)>,
    reading_input: Vec<&'static str>,
    statement_filters: Vec<StatementFilterFn>,
    node_anchors: Vec<NodeAnchorFn>,
}

/// What an engine is built with, besides itself.
#[derive(Clone, Copy, Default)]
pub struct Parts<'a> {
    pub pieces: &'a [&'static Piece],
    pub libraries: &'a [&'static Library],
}

impl Parts<'_> {
    pub(super) fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        let pieces = self.pieces.iter().map(|p| p.name);
        pieces.chain(self.libraries.iter().map(|l| l.name))
    }
}

trait Slot: Any {
    fn clone_slot(&self) -> Box<dyn Slot>;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Any + Clone> Slot for T {
    fn clone_slot(&self) -> Box<dyn Slot> {
        Box::new(self.clone())
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// What the pieces keep, one value of each type. The engine does not know what they are.
#[derive(Default)]
pub struct Slots(Vec<Box<dyn Slot>>);

impl Clone for Slots {
    fn clone(&self) -> Self {
        Slots(self.0.iter().map(|s| (**s).clone_slot()).collect())
    }
}

impl Engine {
    /// Puts `piece` in this engine: its commands, then what it sets up.
    pub(super) fn plug(&mut self, piece: &Piece) {
        for (name, command) in piece.commands {
            let id = PIECE_COMMAND_BASE as usize + self.plug.command_fns.len();
            assert!(id < NATIVE_KEY_BASE as usize, "too many commands in the pieces");
            self.tex.define(name, Meaning::Native(id as u16));
            self.plug.command_fns.push(*command);
        }
        self.plug.hooks.path_operations.extend_from_slice(piece.path_operations);
        self.plug.hooks.coordinate_systems.extend_from_slice(piece.coordinate_systems);
        self.plug.hooks.coordinate_fallbacks.extend_from_slice(piece.coordinate_fallbacks);
        self.plug.hooks.plain_coordinates.extend_from_slice(piece.plain_coordinates);
        self.plug.hooks.node_text.extend_from_slice(piece.node_text);
        self.plug.hooks.environments.extend_from_slice(piece.environments);
        self.plug.hooks.decorations.extend_from_slice(piece.decorations);
        self.plug.hooks.reading_input.extend_from_slice(piece.reading_input);
        self.plug.hooks.statement_filters.extend_from_slice(piece.statement_filters);
        self.plug.hooks.node_anchors.extend_from_slice(piece.node_anchors);
        (piece.define)(self);
    }

    /// What a piece does with the word `word` of a path.
    pub(super) fn path_operation(&self, word: &str) -> Option<PathOperationFn> {
        self.plug.hooks.path_operations.iter().find(|(w, _)| *w == word).map(|(_, operation)| *operation)
    }

    /// The coordinate system `name` of a piece.
    pub(super) fn coordinate_system_of(&self, name: &str) -> Option<CoordinateSystemFn> {
        self.plug.hooks.coordinate_systems.iter().find(|(n, _)| *n == name).map(|(_, system)| *system)
    }

    /// The coordinate system `name` of the pieces that know many.
    pub(super) fn piece_coordinate(&mut self, name: &str, options: &str) -> Option<Result<Coord, Error>> {
        (0..self.plug.hooks.coordinate_fallbacks.len()).find_map(|i| (self.plug.hooks.coordinate_fallbacks[i])(self, name, options))
    }

    /// What a piece reads in a plain `(x,y)`.
    pub(super) fn piece_plain_coordinate(&mut self, parts: &[String]) -> Option<Result<Point, Error>> {
        (0..self.plug.hooks.plain_coordinates.len()).find_map(|i| (self.plug.hooks.plain_coordinates[i])(self, parts))
    }

    /// What a piece does with the command `name` in the text of a node.
    pub fn node_text_command(&self, name: &str) -> Option<NodeTextFn> {
        self.plug.hooks.node_text.iter().find(|(n, _)| *n == name).map(|(_, command)| *command)
    }

    /// The two ends of the environment `name` of a piece.
    pub(super) fn piece_environment(&self, name: &str) -> Option<(EnvironmentFn, EnvironmentFn)> {
        self.plug.hooks.environments.iter().find(|(n, _, _)| *n == name).map(|(_, begin, end)| (*begin, *end))
    }

    /// The decoration `name` of a piece.
    pub(super) fn piece_decoration(&self, name: &str) -> Option<DecorationFn> {
        self.plug.hooks.decorations.iter().find(|(n, _)| *n == name).map(|(_, decoration)| *decoration)
    }

    /// What a piece did with the statement `name`, if it took it.
    pub(super) fn filtered_statement(&mut self, name: &str, t: &Token) -> Option<Result<(), Error>> {
        (0..self.plug.hooks.statement_filters.len()).find_map(|i| (self.plug.hooks.statement_filters[i])(self, name, t))
    }

    /// Where the anchor `name` of a piece is, if it is one of theirs.
    pub(super) fn piece_anchor(&mut self, name: &str, local: &dyn Fn(&str) -> Option<Point>) -> Option<Point> {
        (0..self.plug.hooks.node_anchors.len()).find_map(|i| (self.plug.hooks.node_anchors[i])(self, name, local))
    }

    /// Whether `t` is a command of a piece that reads its own input.
    pub(super) fn reads_its_input(&self, t: &Token) -> bool {
        matches!(t, Token::Cs(name) if self.plug.hooks.reading_input.contains(&&**name))
    }

    /// The value of type `T` a piece keeps during a figure, created the first time.
    pub fn piece_state<T: Any + Clone + Default>(&mut self) -> &mut T {
        let found = self.plug.slots.0.iter_mut().position(|s| (**s).as_any_mut().is::<T>());
        let at = found.unwrap_or_else(|| {
            self.plug.slots.0.push(Box::new(T::default()));
            self.plug.slots.0.len() - 1
        });
        match (*self.plug.slots.0[at]).as_any_mut().downcast_mut() {
            Some(state) => state,
            None => unreachable!("the slot at `at` holds a `T`"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default, PartialEq, Debug)]
    struct Count(u32);

    #[derive(Clone, Default, PartialEq, Debug)]
    struct Name(String);

    #[test]
    fn each_type_has_its_own_slot_and_a_copy_keeps_it_apart() {
        let mut e = Engine::new();
        e.piece_state::<Count>().0 = 7;
        e.piece_state::<Name>().0 = "x".into();
        let mut copy = e.clone();
        copy.piece_state::<Count>().0 = 8;
        assert_eq!(*e.piece_state::<Count>(), Count(7));
        assert_eq!(*copy.piece_state::<Count>(), Count(8));
        assert_eq!(copy.piece_state::<Name>().0, "x");
    }

    fn twice(e: &mut Engine, _: &Token) -> Result<(), Error> {
        e.tex.push_text("\\def\\done{yes}");
        Ok(())
    }

    /// Reads what comes after it itself, up to a `!`: no `;` for `\\tikz` to look for.
    fn live(e: &mut Engine, _: &Token) -> Result<(), Error> {
        let read = e.tex.read_until_char('!', false)?;
        e.tex.define_string("livedread", &crate::tex::tokens_to_string(&read), true);
        Ok(())
    }

    static PROBE: Piece = Piece {
        name: "probe",
        commands: &[("probecommand", twice), ("probelive", live)],
        path_operations: &[("probeword", |e| {
            e.tex.define_string("walked", "yes", true);
            Ok(())
        })],
        coordinate_systems: &[("probe", |_, args| Ok(Point::new(args.trim().len() as f64, 0.0)))],
        reading_input: &["probelive"],
        statement_filters: &[|e, name, _| {
            (name == "coordinate" && e.tex.is_defined("probeholds")).then(|| {
                e.tex.define_string("probeheld", "yes", true);
                e.skip_statement().map(|_| ())
            })
        }],
        node_anchors: &[|_, name, local| (name == "probe anchor").then(|| local("center")).flatten()],
        coordinate_fallbacks: &[|_, name, _| (name == "probefall").then(|| Ok(Coord { p: Point::new(5.0, 0.0), node: None }))],
        environments: &[(
            "probeenv",
            |e, _| {
                e.tex.define_string("envbegan", "yes", true);
                Ok(())
            },
            |e, _| {
                e.tex.define_string("envended", "yes", true);
                Ok(())
            },
        )],
        decorations: &[("probedeco", |_, segs| Ok(segs.iter().rev().cloned().collect()))],
        define: |e| e.tex.define_string("probedefined", "yes", true),
        definitions: || r"\def\probeloaded{yes}".to_string(),
        ..Piece::EMPTY
    };

    #[test]
    fn a_piece_brings_its_commands_and_definitions() {
        let mut bare = Engine::new();
        assert!(!bare.tex.is_defined("probeloaded"));
        let pieces: &[&'static Piece] = &[&PROBE];
        let mut e = Engine::of(Parts { pieces, libraries: &[] });
        assert!(e.tex.is_defined("probeloaded"), "its definitions ran");
        assert!(e.tex.is_defined("probedefined"), "what it sets up is there");
        e.tex.push_text(r"\probecommand");
        e.run().unwrap();
        assert!(e.tex.is_defined("done"), "its command ran");
        bare.tex.push_text(r"\probecommand");
        bare.run().unwrap();
        assert!(!bare.tex.is_defined("done"), "the engine does not know it");
        e.tex.push_text(r"\begin{tikzpicture}\path (0,0) probeword;\end{tikzpicture}");
        e.run().unwrap();
        assert!(e.tex.warnings.is_empty(), "{:?}", e.tex.warnings);
        assert!(e.tex.is_defined("walked"), "its word of a path ran");
        e.tex.push_text(r"\begin{tikzpicture}\coordinate (a) at (probe cs:abc);\end{tikzpicture}");
        e.run().unwrap();
        assert!(e.tex.warnings.is_empty(), "{:?}", e.tex.warnings);
        assert!((e.nodes["a"].center().x - 3.0).abs() < 1e-9, "its coordinate system ran");
        e.tex.push_text(r"\begin{probeenv}\end{probeenv}");
        e.run().unwrap();
        assert!(e.tex.is_defined("envbegan") && e.tex.is_defined("envended"), "its environment ran");
        assert_eq!(e.coordinate("probefall cs:x").unwrap().p.x, 5.0, "its coordinate system ran");
        assert!(e.piece_decoration("probedeco").is_some() && e.piece_decoration("zigzag").is_none());
        e.tex.define_string("probeholds", "yes", true);
        e.tex.push_text(r"\begin{tikzpicture}\coordinate (b) at (1,1);\end{tikzpicture}");
        e.run().unwrap();
        assert!(e.tex.is_defined("probeheld") && !e.nodes.contains_key("b"), "it took the statement");
        e.tex.push_text(r"\begin{tikzpicture}\node[anchor=probe anchor] (c) {x};\end{tikzpicture}");
        e.run().unwrap();
        assert!(e.tex.warnings.is_empty(), "its anchor is known: {:?}", e.tex.warnings);
        e.tex.push_text(r"\tikz\probelive with no semicolon!");
        e.run().unwrap();
        assert!(e.tex.is_defined("livedread"), "it read its own input");
    }
}
