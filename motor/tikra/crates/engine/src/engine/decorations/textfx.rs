//! `/pgf/decoration/text effects/…` settings kept in the graphic state.

use crate::tex::Token;

/// The `/pgf/decoration/text effects/…` settings kept in the graphic state.
#[derive(Debug, Clone)]
pub struct Settings {
    pub path_from_text: bool,
    pub path_from_text_angle: f64,
    pub fit: bool,
    pub scale: bool,
    pub segment_from_width: bool,
    /// None: a space
    pub word_separator: Option<String>,
    pub character_command: Option<Vec<Token>>,
    pub reverse: bool,
    pub group_letters: bool,
    pub repeat: i64,
    /// the macros named by `character count=`, `character total=`, `letter count=`…
    pub vars: [Option<String>; 6],
    /// `style characters=… with {…}`: a character's text and its options
    pub styles: Vec<(String, Vec<Token>)>,
    /// `replace characters=… with {…}`: a character's text and the code drawn instead
    pub replacements: Vec<(String, Vec<Token>)>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            path_from_text: false,
            path_from_text_angle: 0.0,
            fit: false,
            scale: false,
            segment_from_width: true,
            word_separator: None,
            character_command: None,
            reverse: false,
            group_letters: false,
            repeat: 0,
            vars: Default::default(),
            styles: Vec::new(),
            replacements: Vec::new(),
        }
    }
}
