//! A blank engine, then the core commands, colors, text and keys.

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
        page_color: None,
        varwidth: None,
        document_depth: None,
        scopes: Vec::new(),
        picture_depth: 0,
        pictures: 0,
        steps: 0,
        depth: 0,
        text_reported: false,
        key_fns: Vec::new(),
        extension_fns: Vec::new(),
        command_fns: Vec::new(),
        hooks: super::piece::Hooks::default(),
        slots: super::piece::Slots::default(),
        canvas_frozen: false,
        nesting: 0,
        keys_run: 0,
        keys_allowed: MAX_KEYS,
        key_lists: 0,
        keys_capped: false,
        first_char_syntax: false,
        pgf_path: super::basic::PgfPath::default(),
        plot_stream: Vec::new(),
        decoration_envs: Vec::new(),
        named_paths: HashMap::new(),
        last_node: None,
        last_node_name: None,
        tree_count: 0,
        chain_list: Default::default(),
        math_kinds: Default::default(),
        after_node_path: Vec::new(),
        local_boxes: Vec::new(),
        interrupted_boxes: Vec::new(),
        path_fadings: Vec::new(),
        carried_fadings: Vec::new(),
        canvas_transforms: 0,
        path_fading_settings: Vec::new(),
        turn_from: Default::default(),
        subpath_start: Default::default(),
        last_xy: Default::default(),
        tree_edge_out: None,
        behind_path: Vec::new(),
        tree_child_names: Vec::new(),
        forced_box: None,
        text_shading: None,
        text_picture: None,
        fadings: HashMap::new(),
        picture_fadings: HashMap::new(),
        basic_shadings: HashMap::new(),
        functional_shadings: HashMap::new(),
        plot_marks_declared: HashMap::new(),
        plot_handlers: HashMap::new(),
        saved_paths: HashMap::new(),
        pgf_solutions: Vec::new(),
        anim_shifts: Vec::new(),
        sys_animations: HashMap::new(),
        user_patterns: HashMap::new(),
        user_shapes: HashMap::new(),
        shape_capture: None,
        extension_shapes: HashMap::new(),
        later_node: None,
        scope_ends: Vec::new(),
        brace_scopes: Vec::new(),
        layer_order: vec!["background".into(), "main".into()],
        layers: HashMap::new(),
        on_layer: Vec::new(),
        boxes: Vec::new(),
        layout: Vec::new(),
        words: Vec::new(),
        saved_boxes: HashMap::new(),
        current_key: String::new(),
        node_sets: super::nodes::NodeSets::default(),
        spy_scopes: Vec::new(),
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
