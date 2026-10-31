//! How the parts of TikZ outside the engine plug into it.
//!
//! A piece brings its commands and its definitions, and depends on the engine, which knows none of
//! them: an engine is built with any set of pieces, and each can be changed or replaced alone. What a
//! piece keeps during a figure is its own: [`Engine::piece_state`].

use std::any::Any;

use super::{Engine, NATIVE_KEY_BASE};
use crate::extension::Library;
use crate::tex::{Meaning, Token};
use crate::Error;

/// A command of a piece: it reads its arguments from the input.
pub type CommandFn = fn(&mut Engine, &Token) -> Result<(), Error>;

/// The first id of the commands of the pieces (`Meaning::Native`).
pub const PIECE_COMMAND_BASE: u16 = 1000;

pub struct Piece {
    /// Unique among the pieces of an engine, which is kept by their names.
    pub name: &'static str,
    pub commands: &'static [(&'static str, CommandFn)],
    /// What it sets up before TikZ's definitions run: registers, macros, keys.
    pub define: fn(&mut Engine),
    /// TeX run with TikZ's definitions, with `@` a letter.
    pub definitions: fn() -> String,
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
            let id = PIECE_COMMAND_BASE as usize + self.command_fns.len();
            assert!(id < NATIVE_KEY_BASE as usize, "too many commands in the pieces");
            self.tex.define(name, Meaning::Native(id as u16));
            self.command_fns.push(*command);
        }
        (piece.define)(self);
    }

    /// The value of type `T` a piece keeps during a figure, created the first time.
    pub fn piece_state<T: Any + Clone + Default>(&mut self) -> &mut T {
        let found = self.slots.0.iter_mut().position(|s| (**s).as_any_mut().is::<T>());
        let at = found.unwrap_or_else(|| {
            self.slots.0.push(Box::new(T::default()));
            self.slots.0.len() - 1
        });
        match (*self.slots.0[at]).as_any_mut().downcast_mut() {
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

    static PROBE: Piece = Piece {
        name: "probe",
        commands: &[("probecommand", twice)],
        define: |e| e.tex.define_string("probedefined", "yes", true),
        definitions: || r"\def\probeloaded{yes}".to_string(),
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
    }
}
