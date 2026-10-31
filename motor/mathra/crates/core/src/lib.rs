//! mathra: equations as a tree of meaning, drawn with explicit rules.
//!
//! It reads math LaTeX (and MathML and AsciiMath), commutative diagrams (amscd, tikz-cd, xymatrix)
//! and chemistry (mhchem formulas, chemfig structures, reaction schemes and mechanism arrows), and
//! writes an SVG that carries its speech in 16 languages, its braille and its source.
//!
//! ```
//! let svg = mathra::tex_to_svg("3.14").unwrap();
//! assert!(svg.contains("-speech\">3.14</title>"));
//!
//! # #[cfg(feature = "chem")] {
//! let r = mathra::convert(r"\chemfig{H_3C-*6(=-=-=-)}", &mathra::Options::default()).unwrap();
//! assert_eq!(r.speech, "structure of C7 H8, with 1 aromatic ring");
//! # }
//! ```
//!
//! # Features
//!
//! - `chem` (default): structural chemistry, the `chem` module (`spec/chemistry.md`). Without it,
//!   `\chemfig` and `\schemestart` are unsupported commands.
//! - `latin-modern`: Latin Modern Math as a second font, chosen with [`convert_in_font`]; some 700 KB
//!   more in the binary.

#[cfg(feature = "chem")]
pub mod chem;
pub mod font;
pub mod input;
pub mod layout;
pub mod locale;
pub mod output;
pub mod rules;
pub mod tree;

use std::fmt;

pub use locale::Locale;
pub use output::braille::BrailleCode;
pub use rules::Applied;
pub use tree::{Class, Node};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The notation the equation is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Input {
    /// Math LaTeX. MathML pasted here (starting with `<math`) is recognized and read as MathML (L-ENT-4).
    #[default]
    Latex,
    /// Presentation MathML: what Word, Google Docs, MathType and MathJax export.
    MathMl,
    /// AsciiMath (asciimath.org): `sum_(i=1)^n i^2`.
    AsciiMath,
}

impl Input {
    pub const ALL: [Input; 3] = [Input::Latex, Input::MathMl, Input::AsciiMath];

    pub fn code(self) -> &'static str {
        match self {
            Input::Latex => "latex",
            Input::MathMl => "mathml",
            Input::AsciiMath => "asciimath",
        }
    }
}

impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::str::FromStr for Input {
    type Err = String;

    /// `latex` (or `tex`), `mathml` (or `mml`), `asciimath` (or `am`), in any case.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "latex" | "tex" => Ok(Input::Latex),
            "mathml" | "mml" => Ok(Input::MathMl),
            "asciimath" | "am" => Ok(Input::AsciiMath),
            other => Err(format!("unknown input format: {other} (use latex, mathml or asciimath)")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Options {
    pub locale: Locale,
    /// Display equation (like `$$…$$`), or inline with the text (like `$…$`).
    pub display: bool,
    /// Refuse invalid LaTeX, like TeX does. Without it, mathra recovers what it can and returns warnings.
    pub strict: bool,
    /// The notation of the input. LaTeX by default.
    pub input: Input,
    /// The braille code of [`Output::braille`]. `None`: the language's (CMU in pt-BR and es, Nemeth elsewhere).
    pub braille: Option<BrailleCode>,
    /// The equation writes itself in the browser, glyph by glyph and stroke by stroke, in the order
    /// the layout set it (CSS in the SVG, S-SVG-10). Off by default; without it the SVG is unchanged.
    pub animate: bool,
    /// Each element of the SVG numbered by that order (`data-step`, and `data-steps` on the `<svg>`),
    /// with no CSS: for a page that walks through the equation itself. The animation numbers them too.
    pub steps: bool,
    /// Text kerned as TeX's OT1 or T1 fonts kern it; none by default.
    pub text_kerning: layout::kerning::TextKerning,
}

impl Default for Options {
    fn default() -> Self {
        Options { locale: Locale::default(), display: true, strict: false, input: Input::Latex, braille: None, animate: false, steps: false, text_kerning: Default::default() }
    }
}

/// The tree of meaning of an equation, in the notation `options.input` says.
fn read(source: &str, options: &Options, registry: &mut rules::Registry) -> Result<(Node, Input), Error> {
    let input = match options.input {
        Input::Latex if input::mathml::looks_like_mathml(source) => {
            registry.note(rules::L_ENT_4, "MathML pasted as LaTeX: read as MathML");
            Input::MathMl
        }
        other => other,
    };
    let tree = match input {
        Input::Latex => input::latex::read_with(source, options.locale, registry, options.strict)?,
        Input::MathMl => input::mathml::read_with(source, options.locale, registry, options.strict)?,
        Input::AsciiMath => input::asciimath::read_with(source, options.locale, registry, options.strict)?,
    };
    Ok((tree, input))
}

#[derive(Debug, Clone)]
pub struct Output {
    pub svg: String,
    pub tree: Node,
    pub speech: String,
    pub text: String,
    /// The equation in braille (Unicode braille cells), in the code of `braille_code`.
    pub braille: String,
    pub braille_code: BrailleCode,
    /// Rules that changed something in this equation, in the order they acted.
    pub applied: Vec<Applied>,
    /// Input problems the tolerant mode worked around (empty in strict mode).
    pub warnings: Vec<Applied>,
}

impl Output {
    /// Every output as one JSON object: `svg`, `speech`, `text`, `tree`, `applied` and `warnings`, the
    /// last two as lists of `{"rule", "description"}`. What the JavaScript and Python packages return.
    pub fn to_json(&self) -> String {
        let rules = |list: &[Applied]| -> Vec<serde_json::Value> { list.iter().map(|a| serde_json::json!({ "rule": a.rule, "description": a.description })).collect() };
        let tree: serde_json::Value = serde_json::from_str(&self.tree.to_json()).expect("the tree serializes to valid JSON");
        serde_json::json!({
            "svg": self.svg,
            "speech": self.speech,
            "text": self.text,
            "braille": self.braille,
            "braille_code": self.braille_code.code(),
            "tree": tree,
            "applied": rules(&self.applied),
            "warnings": rules(&self.warnings),
        })
        .to_string()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Syntax { column: usize, message: String },
    MissingGlyph { character: char },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Syntax { column, message } => write!(f, "column {column}: {message}"),
            Error::MissingGlyph { character } => {
                write!(f, "the {} font has no '{character}' character (U+{:04X})", font::NAME, *character as u32)
            }
        }
    }
}

impl std::error::Error for Error {}

/// Converts an equation (LaTeX by default; see [`Options::input`]) to SVG, speech, text and tree.
pub fn convert(tex: &str, options: &Options) -> Result<Output, Error> {
    convert_in_font(tex, options, font::MathFont::default())
}

/// [`convert`], with the equation set in `font`.
pub fn convert_in_font(tex: &str, options: &Options, font: font::MathFont) -> Result<Output, Error> {
    let mut registry = rules::Registry::default();
    let (tree, input) = read(tex, options, &mut registry)?;
    let style = if options.display { layout::Style::display() } else { layout::Style::text() }.in_font(font);
    let math_box = {
        let mut ctx = layout::Context { locale: options.locale, registry: &mut registry, strict: options.strict, kerning: options.text_kerning };
        layout::compose_list(&tree, style, &mut ctx)?
    };
    let speech = output::speech::speak(&tree, options.locale);
    let text = output::text::linear(&tree, options.locale);
    let braille_code = options.braille.unwrap_or(BrailleCode::for_locale(options.locale));
    let braille = output::braille::transcribe(&tree, braille_code);
    for c in &braille.missing {
        registry.note(rules::S_BRL_1, format!("no {braille_code} braille sign for '{c}' (U+{:04X}): written as the full cell ⠿", *c as u32));
    }
    let tree_json = tree.to_json();
    registry.dedup();
    let svg = output::svg::generate(&output::svg::Content {
        math_box: &math_box,
        source: tex,
        input,
        tree_json: &tree_json,
        speech: &speech,
        text: &text,
        braille: &braille.text,
        braille_code,
        locale: options.locale,
        warnings: &registry.warnings,
        font,
        steps: options.steps,
        animate: options.animate,
    });
    Ok(Output { svg, tree, speech, text, braille: braille.text, braille_code, applied: registry.applied, warnings: registry.warnings })
}

/// The composed box, without generating SVG: for callers that draw the equation inside something
/// else (tikra's figure labels). Also returns the tolerant mode's warnings.
pub fn typeset(tex: &str, options: &Options) -> Result<(layout::MathBox, Vec<Applied>), Error> {
    typeset_in_font(tex, options, font::MathFont::default())
}

/// [`typeset`], with the equation set in `font` (its glyph ids are that font's).
pub fn typeset_in_font(tex: &str, options: &Options, font: font::MathFont) -> Result<(layout::MathBox, Vec<Applied>), Error> {
    let mut registry = rules::Registry::default();
    let (tree, _) = read(tex, options, &mut registry)?;
    let style = if options.display { layout::Style::display() } else { layout::Style::text() }.in_font(font);
    let mut ctx = layout::Context { locale: options.locale, registry: &mut registry, strict: options.strict, kerning: options.text_kerning };
    let math_box = layout::compose_list(&tree, style, &mut ctx)?;
    registry.dedup();
    Ok((math_box, registry.warnings))
}

/// Shortcut with the default locale (English).
pub fn tex_to_svg(tex: &str) -> Result<String, Error> {
    convert(tex, &Options::default()).map(|r| r.svg)
}
