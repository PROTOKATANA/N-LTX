//! The fields of the engine, grouped by what they are for.

use super::*;

/// What the pieces and libraries put in the engine.
#[derive(Clone, Default)]
pub struct Plugged {
    /// the native keys, by id − NATIVE_KEY_BASE
    pub key_fns: Vec<options::KeyFn>,
    /// the keys of libraries from outside ([`crate::extension`]), by id − EXTENSION_KEY_BASE
    pub extension_fns: Vec<crate::extension::KeyFn>,
    /// the commands of the pieces, by id − PIECE_COMMAND_BASE
    pub command_fns: Vec<piece::CommandFn>,
    /// what the pieces add to the work of the engine
    pub(super) hooks: piece::Hooks,
    /// what the pieces keep during a figure
    pub slots: piece::Slots,
}

/// The keys being run, and how many may run.
#[derive(Clone)]
pub struct KeyRun {
    /// keys run so far
    /// `/handlers/first char syntax` was set: keys are checked for a handler of their first character.
    pub first_char_syntax: bool,
    pub keys_run: usize,
    /// how many keys may run: grows with the lists of the figure
    pub keys_allowed: usize,
    /// the key lists being run, innermost last: more than one is a style's
    pub key_lists: usize,
    /// the cap on keys was reached and told
    pub keys_capped: bool,
    /// the full path of the native key being run (natives that share code tell keys apart by it)
    pub current_key: String,
}

impl Default for KeyRun {
    fn default() -> Self {
        KeyRun { first_char_syntax: false, keys_run: 0, keys_allowed: MAX_KEYS, key_lists: 0, keys_capped: false, current_key: String::new() }
    }
}

/// The document around the pictures: its text, boxes and page.
#[derive(Clone, Default)]
pub struct Page {
    /// `\pagecolor`: the color of the page behind the figure
    pub page_color: Option<Color>,
    /// `varwidth=…` of the `standalone` class: the widest the page gets (cm)
    pub varwidth: Option<f64>,
    /// the group depth inside `\begin{document}`, where a `\color` lasts to the page's end
    pub document_depth: Option<usize>,
    /// text outside nodes was seen and reported
    pub text_reported: bool,
    /// pictures already drawn, and how the document lays them out
    pub(super) boxes: Vec<PictureBox>,
    pub(super) layout: Vec<Layout>,
    /// the words of the text around the pictures, each with the font it starts in
    pub words: Vec<(String, String)>,
    /// the boxes `\savebox`, `\sbox` and `lrbox` keep, by their names (`\newsavebox`)
    pub(super) saved_boxes: HashMap<String, PictureBox>,
}

/// The layers of a picture.
#[derive(Clone)]
pub struct Layers {
    /// layers: their order, the items of each but main, and what `\pgfonlayer` set aside
    pub layer_order: Vec<String>,
    pub layers: HashMap<String, Vec<Item>>,
    pub on_layer: Vec<(String, Vec<Item>)>,
}

impl Default for Layers {
    fn default() -> Self {
        Layers { layer_order: vec!["background".into(), "main".into()], layers: HashMap::new(), on_layer: Vec::new() }
    }
}

/// Fadings: declared, and those of the path being drawn.
#[derive(Clone, Default)]
pub struct Fadings {
    /// For each path statement being read, its `scope fading`s.
    pub path_fadings: Vec<Vec<fadings::PathFading>>,
    /// The `scope fading`s of a path that drew nothing: the next path is faded, the fading fitted
    /// to the box of the path that gave it (none: at its natural size).
    pub carried_fadings: Vec<fadings::CarriedFading>,
    /// For each path statement being read, the `fading transform` and `fit fading` its options (and
    /// its nodes') set: what its `scope fading`s use.
    pub path_fading_settings: Vec<(Transform, bool)>,
    /// fadings defined by `\tikzfading`, in their 100 bp box
    pub fadings: HashMap<String, crate::canvas::Shading>,
    /// fadings drawn as pictures (`\pgfdeclarefading`, `tikzfadingfrompicture`)
    pub picture_fadings: HashMap<String, fadings::PictureFading>,
}

/// What a figure declares and uses later by name.
#[derive(Clone, Default)]
pub struct Declared {
    /// the basic layer's declared shadings
    pub basic_shadings: HashMap<String, shadings::BasicShading>,
    /// the declared functional shadings
    pub functional_shadings: HashMap<String, functional::Functional>,
    /// marks declared by `\pgfdeclareplotmark`: their code
    pub plot_marks_declared: HashMap<String, Vec<Token>>,
    /// plot handlers declared by `\pgfdeclareplothandler`: the code for each point (`##1` the
    /// point, `#1`… the handler's arguments)
    pub plot_handlers: HashMap<String, String>,
    /// patterns declared by the basic layer
    pub user_patterns: HashMap<String, patterns::UserPattern>,
    /// shapes declared by `\pgfdeclareshape`
    pub user_shapes: HashMap<String, user_shapes::UserShape>,
    /// while a declared shape's path code runs: what it draws, kept for the shape
    pub shape_capture: Option<user_shapes::ShapeCapture>,
    /// the node shapes libraries declared, by name
    pub extension_shapes: HashMap<String, crate::extension::ShapeFn>,
}

/// The tree being built.
#[derive(Clone, Default)]
pub struct TreeRun {
    pub tree_count: usize,
    /// the name of each child being grown, set by the first node its statement makes
    pub tree_child_names: Vec<Option<String>>,
    /// a child's `edge from parent[…]`, read inside its statement
    pub tree_edge_out: Option<(Vec<Token>, Vec<Token>)>,
}

/// What the animations give objects at the snapshot time.
#[derive(Clone, Default)]
pub struct Animations {
    /// the shifts a node's animations move it by (canvas), which its picture's size makes room for
    pub anim_shifts: Vec<Point>,
    /// The options the system layer's animations give, at the snapshot time, to the next object
    /// with each name (`\pgfidrefnextuse`, `\pgfsysanimate`).
    pub sys_animations: HashMap<String, Vec<Token>>,
}

/// The basic layer: its path, its points and the paths it keeps.
#[derive(Clone, Default)]
pub struct PgfLayer {
    /// the basic layer's path being built
    pub pgf_path: basic::PgfPath,
    /// the points of `\pgfplotstreampoint` since `\pgfplotstreamstart`, on the canvas
    pub plot_stream: Vec<crate::scene::Point>,
    /// the path's last point and where it came from, for `turn` in the next coordinate
    pub turn_from: (crate::scene::Point, crate::scene::Point),
    /// where the path's subpath started (`current subpath start`)
    pub subpath_start: crate::scene::Point,
    /// the last point of the last path, on the canvas (`\pgfgetlastxy`); it outlives scopes
    pub last_xy: crate::scene::Point,
    /// paths kept by `save path`
    pub saved_paths: HashMap<String, Vec<crate::scene::Segment>>,
    /// the points `\pgfintersectionofpaths` found (canvas)
    pub pgf_solutions: Vec<Point>,
    /// paths kept by `name path`
    pub named_paths: HashMap<String, Vec<crate::scene::Segment>>,
}

/// What the node being made carries.
#[derive(Clone, Default)]
pub struct NodeRun {
    /// the node `\pgfpositionnodelater` kept: what it draws, where its anchors are, its name
    pub later_node: Option<(Vec<Item>, nodes::NodeInfo, Option<String>)>,
    /// the text box a node must take (a matrix's cells), instead of its text's
    pub forced_box: Option<(f64, f64, f64)>,
    /// the shading a node's text is (`\pgfuseshading{name}`), with its half width and height (cm)
    pub text_shading: Option<(String, f64, f64)>,
    /// the picture a node's text is (`\begin{tikzpicture}…\end{tikzpicture}`, `\tikz…`,
    /// `\usebox`): what it draws, its box's corners, and its depth below the baseline (cm)
    pub text_picture: Option<(Vec<Item>, Point, Point, f64)>,
    /// what the `behind path` nodes of the path being built draw, drawn before it
    pub behind_path: Vec<Item>,
}

/// What the open scopes leave for when they end.
#[derive(Clone, Default)]
pub struct Scoped {
    /// `execute at end scope` code of the open scopes, innermost last
    pub scope_ends: Vec<Vec<Token>>,
    /// for each `{` in a picture: whether it opened a scope (`{[options] …}`, the scopes library)
    pub brace_scopes: Vec<bool>,
    /// The open `local bounding box`es: their names and the group depth they were opened at.
    pub local_boxes: Vec<(String, usize)>,
    /// The picture's size when each `pgfinterruptboundingbox` opened, restored when it closes.
    pub interrupted_boxes: Vec<(Option<Point>, Option<Point>)>,
    /// How many `transform canvas` groups have been opened: a path statement closes its own.
    pub canvas_transforms: usize,
}
