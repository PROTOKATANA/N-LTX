//! Decorations, as pgf's decoration module and libraries do them, and TikZ's glue
//! (`tikzlibrarydecorations.code.tex`): the `decorate` option and path operation, the
//! `/pgf/decoration/…` keys, `pre` and `post`; and `preaction` and `postaction`, which use the
//! path again with other options (`tikz.code.tex`, `\tikz@finish`).

mod apply;
mod feet;
mod library;
mod textfx;
mod walker;

use std::rc::Rc;

use tikra_engine::keys;
use tikra_engine::scene::Transform;
use tikra_engine::tex::{self, Cat, Meaning, Token};
use tikra_engine::{Error, PT};

use tikra_engine::engine::Engine;
use walker::Walker;

use tikra_engine::engine::decorations::{Mark, Pos, Settings};
use tikra_engine::engine::piece::{CommandFn, Piece};
use tikra_engine::engine::NATIVE_KEY_BASE;
use tikra_engine::keys::macro_name;

/// Decorations of pgf and TikZ: the automaton, the libraries and the basic layer's environments.
pub static PIECE: Piece = Piece { name: "decorations", commands: COMMANDS, decorator: Some(apply::decorate_segments), define, definitions, ..Piece::EMPTY };

const COMMANDS: &[(&str, CommandFn)] = &[
    ("tikra@arrow", |e, _| apply::arrow_command(e, false)),
    ("tikra@arrowreversed", |e, _| apply::arrow_command(e, true)),
    ("tikra@decoration", |e, _| apply::decoration_begin(e)),
    ("tikra@enddecoration", |e, _| apply::decoration_end(e)),
    ("tikra@decoratecurrentpath", |e, _| apply::decorate_current_path(e)),
];

/// The dimension registers and the native keys.
fn define(e: &mut Engine) {
    for (name, r) in REGISTERS {
        e.tex.define(name, Meaning::Dimen(*r));
    }
    for (path, f) in KEYS {
        let id = NATIVE_KEY_BASE + e.plug.key_fns.len() as u16;
        e.plug.key_fns.push(*f);
        e.tex.define(&macro_name(&format!("{path}/.@cmd")), Meaning::Native(id));
    }
}

/// Text effects, then the declarations, then the definitions: the order `define_keys` pushed them.
///
/// A comment between the texts keeps the tokens of three separate frames. A frame starts without
/// skipping blanks, and a comment clears that skipping, so the next end of line is a space again.
fn definitions() -> String {
    let mut out = String::from(textfx::DEFINITIONS);
    out.push_str("%\n\n");
    out.push_str(without_leading_newline(DECLARATIONS));
    out.push_str("%\n\n");
    out.push_str(without_leading_newline(DEFINITIONS));
    out
}

fn without_leading_newline(text: &str) -> &str {
    match text.strip_prefix('\n') {
        Some(rest) => rest,
        None => text,
    }
}

/// How a path is decorated.
enum How {
    /// TikZ's `decorate`, with `pre` and `post`
    Tikz(String),
    /// `\pgfdecoration`: `{name}{length}{before code}{after code}, …`
    List(Vec<Token>),
    /// `\pgfmetadecoration`
    Meta(String),
}

/// An open `\pgfdecoration`: its list, and the path there was before.
#[derive(Debug, Clone)]
pub struct Env {
    list: Vec<Token>,
    meta: bool,
    existing: tikra_engine::engine::basic::PgfPath,
}

/// Items of a list at top-level commas.
fn split_commas(v: &[Token]) -> Vec<Vec<Token>> {
    let mut out = vec![Vec::new()];
    let mut depth = 0usize;
    for t in v {
        match t {
            Token::Char(_, Cat::Begin) => depth += 1,
            Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
            Token::Char(',', _) if depth == 0 => {
                out.push(Vec::new());
                continue;
            }
            _ => {}
        }
        out.last_mut().expect("one").push(t.clone());
    }
    out.retain(|i| !keys::trim(i).is_empty());
    out
}

/// Dimension registers that decoration code can read (`\pgfdecoratedpathlength`…).
const REGISTERS: &[(&str, u32)] = &[
    ("pgfdecoratedpathlength", 4010),
    ("pgfdecoratedcompleteddistance", 4011),
    ("pgfdecoratedremainingdistance", 4012),
    ("pgfdecoratedinputsegmentlength", 4013),
    ("pgfdecoratedinputsegmentcompleteddistance", 4014),
    ("pgfdecoratedinputsegmentremainingdistance", 4015),
    ("pgfdecorationsegmentlength", 4016),
    ("pgfdecorationsegmentamplitude", 4017),
];

fn sp(pt: f64) -> i64 {
    (pt * 65536.0).round() as i64
}

/// Sets the registers to where the walk is.
pub fn set_registers(e: &mut Engine, w: &Walker) {
    let values = [w.section, w.completed, w.remaining, w.seg.len, w.seg_completed, w.seg_remaining];
    for ((_, r), v) in REGISTERS.iter().zip(values) {
        e.tex.dimens.insert(*r, sp(v));
    }
    e.tex.define_string("pgfdecoratedangle", &format!("{}", w.angle), false);
}

macro_rules! keyfn {
    ($name:ident, |$e:ident, $v:ident| $body:expr) => {
        fn $name($e: &mut Engine, $v: Option<Vec<Token>>) -> Result<(), Error> {
            $body
        }
    };
}

fn text(e: &mut Engine, v: &Option<Vec<Token>>) -> Result<String, Error> {
    Ok(match v {
        Some(v) => e.tex.expand_to_string(v.clone())?.trim().to_string(),
        None => String::new(),
    })
}

fn pt(e: &mut Engine, v: Option<Vec<Token>>) -> Result<Option<f64>, Error> {
    match v {
        Some(v) => Ok(Some(e.length_of(v)? / PT)),
        None => Ok(None),
    }
}

/// `true` (or no value), `false`.
fn boolean(e: &mut Engine, v: &Option<Vec<Token>>) -> Result<bool, Error> {
    Ok(!matches!(text(e, v)?.as_str(), "false"))
}

pub fn current_name(e: &Engine) -> String {
    let name = keys::value(&e.tex, "pgfkeyscurrentname").or_else(|| match e.tex.meaning_of("pgfkeyscurrentname") {
        Meaning::Macro(m) => Some(tex::substitute(&m.body, &[])),
        _ => None,
    });
    name.map(|v| tex::tokens_to_string(&v)).unwrap_or_default()
}

keyfn!(k_decorate, |e, v| {
    e.st.decorate = boolean(e, &v)?;
    Ok(())
});
keyfn!(k_snake, |e, v| {
    let n = text(e, &v)?;
    e.st.snake = if n == "none" { None } else { Some(n) };
    Ok(())
});
keyfn!(k_decoration, |e, v| match v {
    Some(v) => e.process_keys(v, Some("/pgf/decoration")),
    None => Ok(()),
});
keyfn!(k_unknown, |e, v| {
    // a decoration's name, else an unknown key
    let name = current_name(e);
    if v.is_none() && (library::resolve(e, &name).is_some() || library::resolve_meta(e, &name).is_some()) {
        e.st.decoration.name = Some(name);
    } else {
        e.warn(format!("option '{name}' not supported: ignored"));
    }
    Ok(())
});
keyfn!(k_name, |e, v| {
    let n = text(e, &v)?;
    e.st.decoration.name = if n == "none" { None } else { Some(n) };
    Ok(())
});
keyfn!(k_amplitude, |e, v| {
    if let Some(x) = pt(e, v)? {
        e.st.decoration.amplitude = x;
    }
    Ok(())
});
keyfn!(k_segment_length, |e, v| {
    if let Some(x) = pt(e, v)? {
        e.st.decoration.segment_length = x;
    }
    Ok(())
});
keyfn!(k_meta_amplitude, |e, v| {
    if let Some(x) = pt(e, v)? {
        e.st.decoration.meta_amplitude = x;
    }
    Ok(())
});
keyfn!(k_meta_segment_length, |e, v| {
    if let Some(x) = pt(e, v)? {
        e.st.decoration.meta_segment_length = x;
    }
    Ok(())
});
keyfn!(k_angle, |e, v| {
    if let Some(v) = v {
        e.st.decoration.angle = e.number_of(v)?;
    }
    Ok(())
});
keyfn!(k_aspect, |e, v| {
    if let Some(v) = v {
        e.st.decoration.aspect = e.number_of(v)?;
        // the number, for decoration code to read
        e.tex.define_string("pgfdecorationsegmentaspect", &tikra_engine::engine::pgf_number(e.st.decoration.aspect), false);
    }
    Ok(())
});
keyfn!(k_raise, |e, v| {
    e.st.decoration.raise = pt(e, v)?;
    Ok(())
});
keyfn!(k_mirror, |e, v| {
    e.st.decoration.mirror = boolean(e, &v)?;
    Ok(())
});
keyfn!(k_transform, |e, v| {
    // the transformations of TikZ options, from the identity
    let saved = e.st.pending;
    e.st.pending = Transform::default();
    let r = e.process_keys(v.unwrap_or_default(), Some("/tikz"));
    let t = e.st.pending;
    e.st.pending = saved;
    r?;
    e.st.decoration.user_transform = Some(Transform { e: t.e / PT, f: t.f / PT, ..t });
    Ok(())
});
keyfn!(k_corners, |e, v| {
    e.st.decoration.has_corners = boolean(e, &v)?;
    Ok(())
});
keyfn!(k_reverse, |e, v| {
    e.st.decoration.reverse = boolean(e, &v)?;
    Ok(())
});
keyfn!(k_reset_marks, |e, _v| {
    e.st.decoration.marks = Rc::new(Vec::new());
    Ok(())
});
keyfn!(k_mark, |e, v| {
    let v = v.unwrap_or_default();
    match parse_mark(&v) {
        Some(m) => Rc::make_mut(&mut e.st.decoration.marks).push(m),
        None => e.warn(format!("mark '{}' not understood: ignored", tex::tokens_to_string(&v).trim())),
    }
    Ok(())
});
keyfn!(k_connection_node, |e, v| {
    e.st.decoration.connection_node = text(e, &v)?;
    Ok(())
});
keyfn!(k_moveto_code, |e, v| {
    e.st.decoration.segment_code[0] = v;
    Ok(())
});
keyfn!(k_lineto_code, |e, v| {
    e.st.decoration.segment_code[1] = v;
    Ok(())
});
keyfn!(k_curveto_code, |e, v| {
    e.st.decoration.segment_code[2] = v;
    Ok(())
});
keyfn!(k_closepath_code, |e, v| {
    e.st.decoration.segment_code[3] = v;
    Ok(())
});
keyfn!(k_text, |e, v| {
    e.st.decoration.text = v.unwrap_or_default();
    Ok(())
});
keyfn!(k_text_color, |e, v| {
    e.st.decoration.text_color = text(e, &v)?;
    Ok(())
});
keyfn!(k_text_format_delimiters, |e, v| {
    let v = v.unwrap_or_default();
    let parts = tikra_engine::engine::options::split_arguments(&v, 2);
    let first = |p: &Vec<Token>| tex::tokens_to_string(p).trim().chars().next();
    if let Some(open) = first(&parts[0]) {
        e.st.decoration.text_format = (open, first(&parts[1]));
    }
    Ok(())
});
keyfn!(k_fit, |e, v| {
    e.st.decoration.text_fit = boolean(e, &v)?;
    Ok(())
});
keyfn!(k_fit_spaces, |e, v| {
    let b = boolean(e, &v)?;
    e.st.decoration.text_fit = b;
    e.st.decoration.text_stretch = b;
    Ok(())
});
keyfn!(k_shape_sloped, |e, v| {
    e.st.decoration.shape_sloped = boolean(e, &v)?;
    Ok(())
});
keyfn!(k_shape_scaled, |e, v| {
    e.st.decoration.shape_scaled = boolean(e, &v)?;
    Ok(())
});
keyfn!(k_shape_sep, |e, v| {
    e.st.decoration.shape_sep = text(e, &v)?;
    Ok(())
});
keyfn!(k_shape_spread, |e, v| {
    e.st.decoration.shape_spread = text(e, &v)?;
    Ok(())
});
keyfn!(k_preaction, |e, v| {
    Rc::make_mut(&mut e.st.preactions).push(v.unwrap_or_default());
    Ok(())
});
keyfn!(k_postaction, |e, v| {
    Rc::make_mut(&mut e.st.postactions).push(v.unwrap_or_default());
    Ok(())
});

/// The keys done natively.
pub const KEYS: &[(&str, tikra_engine::engine::options::KeyFn)] = &[
    ("/tikz/decorate", k_decorate),
    ("/tikz/snake", k_snake),
    ("/tikz/preaction", k_preaction),
    ("/tikz/postaction", k_postaction),
    ("/pgf/decoration", k_decoration),
    ("/pgf/decoration/.unknown", k_unknown),
    ("/pgf/decoration/name", k_name),
    ("/pgf/decoration/amplitude", k_amplitude),
    ("/pgf/decoration/segment length", k_segment_length),
    ("/pgf/decoration/meta-amplitude", k_meta_amplitude),
    ("/pgf/decoration/meta-segment length", k_meta_segment_length),
    ("/pgf/decoration/angle", k_angle),
    ("/pgf/decoration/aspect", k_aspect),
    ("/pgf/decoration/raise", k_raise),
    ("/pgf/decoration/mirror", k_mirror),
    ("/pgf/decoration/transform", k_transform),
    ("/pgf/decoration/path has corners", k_corners),
    ("/pgf/decoration/reverse path", k_reverse),
    ("/pgf/decoration/reset marks", k_reset_marks),
    ("/pgf/decoration/mark", k_mark),
    ("/pgf/decoration/mark connection node", k_connection_node),
    ("/pgf/decoration/moveto code", k_moveto_code),
    ("/pgf/decoration/lineto code", k_lineto_code),
    ("/pgf/decoration/curveto code", k_curveto_code),
    ("/pgf/decoration/closepath code", k_closepath_code),
    ("/pgf/decoration/text", k_text),
    ("/pgf/decoration/text color", k_text_color),
    ("/pgf/decoration/text format delimiters", k_text_format_delimiters),
    ("/pgf/decoration/text align/fit to path", k_fit),
    ("/pgf/decoration/text align/fit to path stretching spaces", k_fit_spaces),
    ("/pgf/decoration/shape sloped", k_shape_sloped),
    ("/pgf/decoration/shape scaled", k_shape_scaled),
    ("/pgf/decoration/shape sep", k_shape_sep),
    ("/pgf/decoration/shape evenly spread", k_shape_spread),
    ("/tikz/text effects", textfx::k_text_effects),
    ("/pgf/decoration/text effects/.unknown", textfx::key_unknown),
    ("/pgf/decoration/text effects/path from text", textfx::k_path_from_text),
    ("/pgf/decoration/text effects/path from text angle", textfx::k_path_from_text_angle),
    ("/pgf/decoration/text effects/fit text to path", textfx::k_fit),
    ("/pgf/decoration/text effects/scale text to path", textfx::k_scale),
    ("/pgf/decoration/text effects/segment from width", textfx::k_segment_from_width),
    ("/pgf/decoration/text effects/word separator", textfx::k_word_separator),
    ("/pgf/decoration/text effects/character command", textfx::k_character_command),
    ("/pgf/decoration/text effects/reverse text", textfx::k_reverse),
    ("/pgf/decoration/text effects/group letters into words", textfx::k_group),
    ("/pgf/decoration/text effects/group letters", textfx::k_group),
    ("/pgf/decoration/text effects/repeat text", textfx::k_repeat),
    ("/pgf/decoration/text effects/character count", textfx::k_character_count),
    ("/pgf/decoration/text effects/character total", textfx::k_character_total),
    ("/pgf/decoration/text effects/letter count", textfx::k_letter_count),
    ("/pgf/decoration/text effects/letter total", textfx::k_letter_total),
    ("/pgf/decoration/text effects/word count", textfx::k_word_count),
    ("/pgf/decoration/text effects/word total", textfx::k_word_total),
    ("/pgf/decoration/text effects/style characters", textfx::k_style_characters),
    ("/pgf/decoration/text effects/replace characters", textfx::k_replace_characters),
    ("/pgf/decoration/text effects/characters/.append", textfx::k_characters_append),
    ("/pgf/decoration/text effects/character widths/.append", textfx::k_widths_append),
];

/// The keys pgf defines as values and styles (`.initial`, `.style`), from its own definitions.
pub const DEFINITIONS: &str = r"
\def\pgfdecorationsegmentaspect{0.5}
\pgfkeys{/pgf/decoration/.cd,
  pre/.initial=lineto, post/.initial=lineto, pre length/.initial=0pt, post length/.initial=0pt,
  start radius/.initial=2.5pt, end radius/.initial=2.5pt, radius/.style={/pgf/decoration/start radius={#1},/pgf/decoration/end radius={#1}},
  mirror/.default=true, path has corners/.default=true, reverse path/.default=true, shape sloped/.default=true,
  shape scaled/.default=true,
  shape/.initial=circle, anchor/.initial=center,
  shape start width/.initial=2.5pt, shape start height/.initial=2.5pt,
  shape end width/.initial=2.5pt, shape end height/.initial=2.5pt,
  shape start size/.style={/pgf/decoration/shape start width={#1},/pgf/decoration/shape start height={#1}},
  shape end size/.style={/pgf/decoration/shape end width={#1},/pgf/decoration/shape end height={#1}},
  shape size/.style={/pgf/decoration/shape start size={#1},/pgf/decoration/shape end size={#1}},
  shape width/.style={/pgf/decoration/shape start width={#1},/pgf/decoration/shape end width={#1}},
  shape height/.style={/pgf/decoration/shape start height={#1},/pgf/decoration/shape end height={#1}},
  foot length/.initial=10pt, stride length/.initial=30pt, foot sep/.initial=4pt, foot angle/.initial=10,
  foot of/.initial=human,
  mark info/sequence number/.initial=0, mark info/distance from start/.initial=,
  text align/.code={\pgfqkeys{/pgf/decoration/text align}{#1}},
  text align/align/.initial=left,
  text align/left/.style={/pgf/decoration/text align/align=left},
  text align/right/.style={/pgf/decoration/text align/align=right},
  text align/center/.style={/pgf/decoration/text align/align=center},
  text align/left indent/.initial=0pt, text align/right indent/.initial=0pt,
  text align/fit to path/.default=true, text align/fit to path stretching spaces/.default=true,
}
% the older names of the decoration keys, under /pgf
\pgfkeys{/pgf/.cd,
  segment length/.style={/pgf/decoration/segment length={#1}}, segment angle/.style={/pgf/decoration/angle={#1}},
  segment aspect/.style={/pgf/decoration/aspect={#1}},
  segment amplitude/.style={/pgf/decoration/amplitude={#1},/pgf/decoration/shape height={2*(#1)}},
  segment object length/.style={/pgf/decoration/shape width={#1},/pgf/decoration/radius={#1}},
}
% the snakes library's keys, as the decoration keys they stand for
\tikzset{
  line before snake/.style={/pgf/decoration/pre=lineto,/pgf/decoration/pre length={#1}},
  line after snake/.style={/pgf/decoration/post=lineto,/pgf/decoration/post length={#1}},
  gap before snake/.style={/pgf/decoration/pre=moveto,/pgf/decoration/pre length={#1}},
  gap after snake/.style={/pgf/decoration/post=moveto,/pgf/decoration/post length={#1}},
  gap around snake/.style={gap before snake={#1},gap after snake={#1}},
  raise snake/.style={/pgf/decoration/raise={#1}}, mirror snake/.style={/pgf/decoration/mirror={#1}},
  mirror snake/.default=true,
}
";

/// The decorations of the document, stored as pgf stores them, and the basic layer's
/// decoration environments.
pub const DECLARATIONS: &str = r"
\long\def\pgfdeclaredecoration#1#2#3{\def\pgf@decorate@name{#1}\expandafter\def\csname pgf@decorate@@#1@initial\endcsname{#2}\let\state\pgf@decorate@state#3\let\state\relax}
\def\pgf@decorate@state#1{\pgfutil@ifnextchar[{\pgf@decorate@@start{#1}}{\pgf@decorate@@start{#1}[]}}
\long\def\pgf@decorate@@start#1[#2]#3{\expandafter\def\csname pgf@decorate@@\pgf@decorate@name @#1@options\endcsname{#2}\expandafter\def\csname pgf@decorate@@\pgf@decorate@name @#1@code\endcsname{#3}}
\long\def\pgfdeclaremetadecoration#1#2#3{\def\pgf@metadecoration@name{#1}\expandafter\def\csname pgf@metadecoration@@#1@initial\endcsname{#2}\let\state\pgf@metadecoration@state#3\let\state\relax}
\def\pgf@metadecoration@state#1{\pgfutil@ifnextchar[{\pgf@metadecoration@@state{#1}}{\pgf@metadecoration@@state{#1}[]}}
\long\def\pgf@metadecoration@@state#1[#2]#3{\expandafter\def\csname pgf@metadecoration@@\pgf@metadecoration@name @#1@options\endcsname{#2}\expandafter\def\csname pgf@metadecoration@@\pgf@metadecoration@name @#1@code\endcsname{#3}}
\def\pgfsetdecorationsegmenttransformation#1{\def\pgf@decorate@additionaltransform{#1}}
\def\pgf@decorate@additionaltransform{}
\def\pgfdecoration#1{\tikra@decoration{#1}{0}}
\def\endpgfdecoration{\tikra@enddecoration}
\def\pgfmetadecoration#1{\tikra@decoration{#1}{1}}
\def\endpgfmetadecoration{\tikra@enddecoration}
\let\startpgfdecoration\pgfdecoration \let\stoppgfdecoration\endpgfdecoration
\let\startpgfmetadecoration\pgfmetadecoration \let\stoppgfmetadecoration\endpgfmetadecoration
\newenvironment{pgfdecoration}[1]{\tikra@decoration{#1}{0}}{\tikra@enddecoration}
\newenvironment{pgfmetadecoration}[1]{\tikra@decoration{#1}{1}}{\tikra@enddecoration}
\long\def\pgfdecoratepath#1#2{\pgfdecoration{{#1}{\pgfdecoratedpathlength}}#2\endpgfdecoration}
\def\pgfdecoratecurrentpath{\tikra@decoratecurrentpath}
";

/// Splits tokens at the first `word` (letters) outside braces.
pub fn split_word(v: &[Token], word: &str) -> Option<(Vec<Token>, Vec<Token>)> {
    let w: Vec<char> = word.chars().collect();
    let mut depth = 0usize;
    for i in 0..v.len() {
        match v[i] {
            Token::Char(_, Cat::Begin) => depth += 1,
            Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth == 0 && i + w.len() <= v.len() && w.iter().enumerate().all(|(k, c)| matches!(v[i + k], Token::Char(d, _) if d == *c)) {
            return Some((v[..i].to_vec(), v[i + w.len()..].to_vec()));
        }
    }
    None
}

fn unbrace(v: Vec<Token>) -> Vec<Token> {
    tex::strip_outer_braces(keys::trim(&v).to_vec())
}

/// `at position #1 with #2`, `between positions #1 and #2 step #3 with #4`.
fn parse_mark(v: &[Token]) -> Option<Mark> {
    let v = keys::trim(v).to_vec();
    if let Some((_, rest)) = split_word(&v, "between positions") {
        let (start, rest) = split_word(&rest, "and")?;
        let (end, rest) = split_word(&rest, "step")?;
        let (step, action) = split_word(&rest, "with")?;
        return Some(Mark::Between { start: Pos::Tokens(unbrace(start)), end: Pos::Tokens(unbrace(end)), step: Pos::Tokens(unbrace(step)), action: unbrace(action) });
    }
    let (_, rest) = split_word(&v, "at position")?;
    let (pos, action) = split_word(&rest, "with")?;
    Some(Mark::At { pos: Pos::Tokens(unbrace(pos)), action: unbrace(action) })
}

#[cfg(test)]
mod frame_tokens {
    use super::{definitions, textfx, DECLARATIONS, DEFINITIONS};
    use tikra_engine::tex::{Tex, Token};

    fn lex(parts: &[&str]) -> Vec<Token> {
        let mut lexer = Tex::new_lexer();
        for part in parts {
            lexer.push_text(part);
        }
        let mut out = Vec::new();
        while let Some(t) = lexer.get_token() {
            out.push(t);
        }
        out
    }

    /// The three texts, joined, tokenize as the three frames `define_keys` used to push.
    #[test]
    fn three_texts_keep_the_tokens_of_three_frames() {
        let separate = lex(&[DEFINITIONS, DECLARATIONS, textfx::DEFINITIONS]);
        let text = definitions();
        let joined = lex(&[&text]);
        assert_eq!(joined.len(), separate.len(), "token count");
        for (i, (got, want)) in joined.iter().zip(&separate).enumerate() {
            assert_eq!(got, want, "token {i}");
        }
    }
}
