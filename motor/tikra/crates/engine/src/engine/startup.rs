//! A blank engine, then the core commands, colors, text and keys.

use super::groups;
use super::*;
use crate::canvas::Canvas;
use crate::color::Color;
use crate::tex::{Meaning, Tex};
use std::collections::HashMap;

impl Engine {
    pub(super) fn build(pieces: &[&'static super::piece::Piece]) -> Engine {
        let mut e = blank();
        register_commands(&mut e);
        seed_colors(&mut e);
        define_core_tex(&mut e);
        super::basic::define(&mut e);
        super::nonlinear::define(&mut e);
        plug_pieces(&mut e, pieces);
        super::lsystem::define(&mut e);
        super::options::define_keys(&mut e, pieces);
        after_tikz(&mut e, pieces);
        e
    }
}

fn blank() -> Engine {
    Engine {
        tex: Tex::new(),
        st: super::state::Shared::default(),
        saved: Vec::new(),
        canvas: Canvas::default(),
        nodes: HashMap::new(),
        colors: HashMap::new(),
        page: groups::Page::default(),
        scopes: Vec::new(),
        picture_depth: 0,
        pictures: 0,
        steps: 0,
        depth: 0,
        plug: groups::Plugged::default(),
        canvas_frozen: false,
        nesting: 0,
        pgf: groups::PgfLayer::default(),
        scoped: groups::Scoped::default(),
        layering: groups::Layers::default(),
        last_node: None,
        anim: groups::Animations::default(),
        last_node_name: None,
        tree: groups::TreeRun::default(),
        chain_list: Default::default(),
        after_node_path: Vec::new(),
        fading: groups::Fadings::default(),
        declared: groups::Declared::default(),
        node_run: groups::NodeRun::default(),
        keyrun: groups::KeyRun::default(),
        node_sets: super::nodes::NodeSets::default(),
        facts: Default::default(),
    }
}

fn register_commands(e: &mut Engine) {
    for (name, cmd) in super::vocab::COMMANDS {
        e.tex.define(name, Meaning::Native(*cmd as u16));
        // pgfmath's inner forms (`\pgfmathadd@`), used by documents with @ a letter
        if *cmd == Cmd::PgfmathFunction {
            e.tex.define(&format!("{name}@"), Meaning::Native(*cmd as u16));
        }
    }
}

fn seed_colors(e: &mut Engine) {
    e.colors.insert("transparent".into(), Color::BLACK);
    e.colors.insert("pgftransparent".into(), Color::BLACK);
}

fn define_core_tex(e: &mut Engine) {
    e.tex.define_text(r"\let\bgroup={\let\egroup=}\def\pgfkeysnovalue{\pgfkeys@novalue}\def\pgfkeys@novalue{}\def\pgfkeysvalueof#1{\csname pgfk@#1\endcsname}\def\pgfkeysgetvalue#1#2{\expandafter\let\expandafter#2\csname pgfk@#1\endcsname}\def\pgfkeyssetvalue#1#2{\expandafter\def\csname pgfk@#1\endcsname{#2}}\def\pgfkeysifdefined#1{\ifcsname pgfk@#1\endcsname\expandafter\@firstoftwo\else\expandafter\@secondoftwo\fi}\def\pgfmathresult{}\def\tikzlastnode{}");
}

fn plug_pieces(e: &mut Engine, pieces: &[&'static super::piece::Piece]) {
    for piece in pieces {
        e.plug(piece);
    }
}

fn after_tikz(e: &mut Engine, pieces: &[&'static super::piece::Piece]) {
    for piece in pieces {
        (piece.after_tikz)(e);
    }
}
