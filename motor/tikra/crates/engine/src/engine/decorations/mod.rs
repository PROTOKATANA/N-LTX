//! Decoration settings kept in the graphic state.
//!
//! They follow TeX groups. The decorations piece reads and writes them, and so do `snake` and the
//! mindmap piece.

use std::rc::Rc;

use crate::scene::Transform;
use crate::tex::Token;
use crate::PT;

pub mod textfx;

/// A position of a mark: an expression, or a length already computed (pt).
#[derive(Debug, Clone, PartialEq)]
pub enum Pos {
    Tokens(Vec<Token>),
    Pt(f64),
}

/// `mark=at position … with {…}` and `mark=between positions … and … step … with {…}`.
#[derive(Debug, Clone, PartialEq)]
pub enum Mark {
    At { pos: Pos, action: Vec<Token> },
    Between { start: Pos, end: Pos, step: Pos, action: Vec<Token> },
}

/// The decoration settings kept in the graphic state (the `/pgf/decoration/…` keys that pgf keeps
/// in registers and macros; those it keeps with `.initial` stay pgfkeys values). Lengths in pt.
#[derive(Debug, Clone)]
pub struct Settings {
    /// `\tikz@decoration@name`; None is `none`
    pub name: Option<String>,
    pub amplitude: f64,
    pub segment_length: f64,
    pub angle: f64,
    pub aspect: f64,
    pub meta_amplitude: f64,
    pub meta_segment_length: f64,
    pub raise: Option<f64>,
    pub mirror: bool,
    pub user_transform: Option<Transform>,
    pub has_corners: bool,
    pub reverse: bool,
    pub marks: Rc<Vec<Mark>>,
    pub connection_node: String,
    /// `moveto code`, `lineto code`, `curveto code`, `closepath code`
    pub segment_code: [Option<Vec<Token>>; 4],
    pub text: Vec<Token>,
    pub text_color: String,
    pub text_format: (char, Option<char>),
    pub text_fit: bool,
    pub text_stretch: bool,
    pub shape_sloped: bool,
    pub shape_scaled: bool,
    pub shape_sep: String,
    pub shape_spread: String,
    pub te: textfx::Settings,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            name: None,
            amplitude: 2.5,
            segment_length: 10.0,
            angle: 45.0,
            aspect: 0.5,
            meta_amplitude: 2.5,
            meta_segment_length: 1.0 / PT,
            raise: None,
            mirror: false,
            user_transform: None,
            has_corners: false,
            reverse: false,
            marks: Rc::new(Vec::new()),
            connection_node: String::new(),
            segment_code: [None, None, None, None],
            text: Vec::new(),
            text_color: String::new(),
            text_format: ('|', None),
            text_fit: false,
            text_stretch: false,
            shape_sloped: true,
            shape_scaled: false,
            shape_sep: ".25cm, between centers".into(),
            shape_spread: String::new(),
            te: Default::default(),
        }
    }
}

impl Settings {
    /// `\tikz@dec@user@transform\tikz@dec@mirror\tikz@dec@shift`, in pt.
    pub fn extra_transform(&self) -> Transform {
        let mut t = self.user_transform.unwrap_or_default();
        if self.mirror {
            t = t.compose(&Transform::scaling(1.0, -1.0));
        }
        if let Some(r) = self.raise {
            t = t.compose(&Transform::translation(0.0, r));
        }
        t
    }
}
