//! The drawing layer: TikZ on top of [`crate::tex`]. Commands such as `\draw` read their arguments
//! from the token stream, the way `tikz.code.tex` does; options are pgfkeys; the graphic state is
//! saved with TeX groups; what is drawn goes to a [`Canvas`].

pub mod animations;
pub mod arrows;
pub mod basic;
pub mod chains;
pub mod circuits;
mod command;
pub mod coords;
pub mod decorations;
pub mod fadings;
pub mod functional;
pub mod graphics;
pub mod intersections;
mod layout;
pub mod libraries;
pub mod lsystem;
pub mod marks;
pub mod matrix;
pub mod mindmap;
pub mod nodes;
pub mod nonlinear;
pub mod number;
pub mod options;
pub mod output;
pub mod path;
pub mod patterns;
mod pgffor;
pub mod pics;
mod picture;
pub mod piece;
mod read;
mod running;
pub mod saveboxes;
mod scope;
pub mod shadings;
pub mod shapes;
pub mod spy;
mod startup;
pub mod state;
pub mod three_d;
pub mod tikzmath;
pub mod trees;
pub mod turtle;
pub mod user_shapes;
pub mod views;
mod vocab;

use layout::{typeset, Apart, Layout, PictureBox, SPACE};
pub use pgffor::{foreach_items, foreach_options, ForeachOptions};
pub use vocab::Cmd;
use vocab::{path_command, COMMANDS};

use std::collections::HashMap;
use std::rc::Rc;

use crate::canvas::{Canvas, Item};
use crate::color::Color;
use crate::scene::{Point, Transform};
use crate::tex::{self, Cat, Meaning, Tex, Token};
use crate::Error;

pub use state::GState;

/// A pgf scope (picture, `scope`, node): what must be closed in the canvas when it ends.
#[derive(Debug, Clone, Default)]
struct PgfScope {
    ends: usize,
    /// the item opened last in it is a fading: a new fading takes its place
    fading_last: bool,
    /// a path clipped it: what it draws after does not make the picture larger
    pub clipped: bool,
}

/// Cap on statements per figure: nested loops do not hang the caller. It is high enough for a figure
/// of tens of thousands of elements, and still cuts a runaway loop in under a second.
pub const MAX_STEPS: usize = 300_000;
/// Cap on the segments of one decoration: a tiny `segment length` does not fill the output.
pub const MAX_DECORATION_STEPS: usize = 100_000;
/// Cap on nesting of scopes, loops and environments.
pub const MAX_DEPTH: usize = 64;

pub struct Engine {
    pub tex: Tex,
    pub st: state::Shared<GState>,
    saved: Vec<state::Shared<GState>>,
    pub canvas: Canvas,
    pub nodes: HashMap<String, nodes::NodeInfo>,
    pub colors: HashMap<String, Color>,
    /// `\pagecolor`: the color of the page behind the figure
    page_color: Option<Color>,
    /// `varwidth=…` of the `standalone` class: the widest the page gets (cm)
    varwidth: Option<f64>,
    /// the group depth inside `\begin{document}`, where a `\color` lasts to the page's end
    document_depth: Option<usize>,
    scopes: Vec<PgfScope>,
    /// how many pictures are open (`tikzpicture` inside `tikzpicture` is a scope)
    pub picture_depth: usize,
    pub pictures: usize,
    pub steps: usize,
    pub depth: usize,
    /// text outside nodes was seen and reported
    text_reported: bool,
    /// the native keys, by id − NATIVE_KEY_BASE
    pub key_fns: Vec<options::KeyFn>,
    /// the keys of libraries from outside ([`crate::extension`]), by id − EXTENSION_KEY_BASE
    extension_fns: Vec<crate::extension::KeyFn>,
    /// the commands of the pieces, by id − PIECE_COMMAND_BASE
    command_fns: Vec<piece::CommandFn>,
    /// what the pieces add to the work of the engine
    hooks: piece::Hooks,
    /// what the pieces keep during a figure
    slots: piece::Slots,
    /// `use as bounding box` was used: the bounding box no longer grows
    pub canvas_frozen: bool,
    /// nested runs of token lists (styles, loop bodies)
    nesting: usize,
    /// the basic layer's path being built
    pub pgf_path: basic::PgfPath,
    /// the points of `\pgfplotstreampoint` since `\pgfplotstreamstart`, on the canvas
    pub plot_stream: Vec<crate::scene::Point>,
    /// `execute at end scope` code of the open scopes, innermost last
    scope_ends: Vec<Vec<Token>>,
    /// for each `{` in a picture: whether it opened a scope (`{[options] …}`, the scopes library)
    brace_scopes: Vec<bool>,
    /// layers: their order, the items of each but main, and what `\pgfonlayer` set aside
    layer_order: Vec<String>,
    layers: HashMap<String, Vec<Item>>,
    on_layer: Vec<(String, Vec<Item>)>,
    /// the last node made (a tree's parent), its name, and how many tree parents were named
    pub last_node: Option<nodes::NodeInfo>,
    /// the shifts a node's animations move it by (canvas), which its picture's size makes room for
    pub anim_shifts: Vec<Point>,
    /// The options the system layer's animations give, at the snapshot time, to the next object
    /// with each name (`\pgfidrefnextuse`, `\pgfsysanimate`).
    pub sys_animations: HashMap<String, Vec<Token>>,
    pub last_node_name: Option<String>,
    tree_count: usize,
    /// the chains of the chains library (global, as TikZ keeps their counts)
    pub chain_list: std::collections::HashMap<String, chains::Chain>,
    /// the types `\tikzmath` declarations gave variables
    pub math_kinds: HashMap<String, tikzmath::Kind>,
    /// path operations to run after the node just made (`after node path`, joins)
    pub after_node_path: Vec<Token>,
    /// The open `local bounding box`es: their names and the group depth they were opened at.
    pub local_boxes: Vec<(String, usize)>,
    /// The picture's size when each `pgfinterruptboundingbox` opened, restored when it closes.
    interrupted_boxes: Vec<(Option<Point>, Option<Point>)>,
    /// For each path statement being read, its `scope fading`s.
    pub path_fadings: Vec<Vec<fadings::PathFading>>,
    /// The `scope fading`s of a path that drew nothing: the next path is faded, the fading fitted
    /// to the box of the path that gave it (none: at its natural size).
    pub carried_fadings: Vec<fadings::CarriedFading>,
    /// How many `transform canvas` groups have been opened: a path statement closes its own.
    pub canvas_transforms: usize,
    /// For each path statement being read, the `fading transform` and `fit fading` its options (and
    /// its nodes') set: what its `scope fading`s use.
    pub path_fading_settings: Vec<(Transform, bool)>,
    /// the path's last point and where it came from, for `turn` in the next coordinate
    pub turn_from: (crate::scene::Point, crate::scene::Point),
    /// where the path's subpath started (`current subpath start`)
    pub subpath_start: crate::scene::Point,
    /// the last point of the last path, on the canvas (`\pgfgetlastxy`); it outlives scopes
    pub last_xy: crate::scene::Point,
    /// fadings defined by `\tikzfading`, in their 100 bp box
    pub fadings: HashMap<String, crate::canvas::Shading>,
    /// fadings drawn as pictures (`\pgfdeclarefading`, `tikzfadingfrompicture`)
    pub picture_fadings: HashMap<String, fadings::PictureFading>,
    /// the basic layer's declared shadings
    pub basic_shadings: HashMap<String, shadings::BasicShading>,
    /// the declared functional shadings
    pub functional_shadings: HashMap<String, functional::Functional>,
    /// marks declared by `\pgfdeclareplotmark`: their code
    pub plot_marks_declared: HashMap<String, Vec<Token>>,
    /// plot handlers declared by `\pgfdeclareplothandler`: the code for each point (`##1` the
    /// point, `#1`… the handler's arguments)
    pub plot_handlers: HashMap<String, String>,
    /// paths kept by `save path`
    pub saved_paths: HashMap<String, Vec<crate::scene::Segment>>,
    /// the points `\pgfintersectionofpaths` found (canvas)
    pub pgf_solutions: Vec<Point>,
    /// patterns declared by the basic layer
    pub user_patterns: HashMap<String, patterns::UserPattern>,
    /// shapes declared by `\pgfdeclareshape`
    pub user_shapes: HashMap<String, user_shapes::UserShape>,
    /// while a declared shape's path code runs: what it draws, kept for the shape
    pub shape_capture: Option<user_shapes::ShapeCapture>,
    /// the node shapes libraries declared, by name
    pub extension_shapes: HashMap<String, crate::extension::ShapeFn>,
    /// the node `\pgfpositionnodelater` kept: what it draws, where its anchors are, its name
    pub later_node: Option<(Vec<Item>, nodes::NodeInfo, Option<String>)>,
    /// the text box a node must take (a matrix's cells), instead of its text's
    pub forced_box: Option<(f64, f64, f64)>,
    /// the shading a node's text is (`\pgfuseshading{name}`), with its half width and height (cm)
    pub text_shading: Option<(String, f64, f64)>,
    /// the picture a node's text is (`\begin{tikzpicture}…\end{tikzpicture}`, `\tikz…`,
    /// `\usebox`): what it draws, its box's corners, and its depth below the baseline (cm)
    pub text_picture: Option<(Vec<Item>, Point, Point, f64)>,
    /// the name of each child being grown, set by the first node its statement makes
    pub tree_child_names: Vec<Option<String>>,
    /// a child's `edge from parent[…]`, read inside its statement
    pub tree_edge_out: Option<(Vec<Token>, Vec<Token>)>,
    /// what the `behind path` nodes of the path being built draw, drawn before it
    pub behind_path: Vec<Item>,
    /// open `\pgfdecoration`s
    pub decoration_envs: Vec<decorations::Env>,
    /// paths kept by `name path`
    pub named_paths: HashMap<String, Vec<crate::scene::Segment>>,
    /// pictures already drawn, and how the document lays them out
    boxes: Vec<PictureBox>,
    layout: Vec<Layout>,
    /// the words of the text around the pictures, each with the font it starts in
    words: Vec<(String, String)>,
    /// the boxes `\savebox`, `\sbox` and `lrbox` keep, by their names (`\newsavebox`)
    saved_boxes: HashMap<String, PictureBox>,
    /// keys run so far
    /// `/handlers/first char syntax` was set: keys are checked for a handler of their first character.
    pub first_char_syntax: bool,
    pub keys_run: usize,
    /// how many keys may run: grows with the lists of the figure
    keys_allowed: usize,
    /// the key lists being run, innermost last: more than one is a style's
    key_lists: usize,
    /// the cap on keys was reached and told
    keys_capped: bool,
    /// the full path of the native key being run (natives that share code tell keys apart by it)
    pub current_key: String,
    /// node sets (`new set`, `set=`): a node with `set=` joins the set once it has a name
    pub node_sets: nodes::NodeSets,
    /// the open spy scopes, innermost last
    pub spy_scopes: Vec<spy::SpyScope>,
    /// what the figure shows, noted as it is drawn, for its description in words
    pub facts: crate::describe::Facts,
}

/// Cap on nested runs (a style calling itself).
pub const MAX_NESTING: usize = 150;
/// Cap on the keys of a figure beyond [`KEYS_PER_LIST`] (styles that multiply:
/// `s/.style={t,t,t,t}`…). The figures of the manuals and the corpus run at most about 10,000.
pub const MAX_KEYS: usize = 50_000;
/// The keys each list in the figure itself adds to [`MAX_KEYS`], not those a style brings: a figure
/// of many elements is no style that multiplies, and with a fixed cap it lost everything after its
/// 16,000th `\fill[red]`.
pub const KEYS_PER_LIST: usize = 16;

/// Ids at and above this are native keys (`\pgfk@…/.@cmd`), below are commands.
pub const NATIVE_KEY_BASE: u16 = 4000;

/// Ids at and above this are the keys of libraries from outside ([`crate::extension`]).
pub const EXTENSION_KEY_BASE: u16 = 40000;

thread_local! {
    /// The engine after defining every command and key, by the names of its parts: cloned for each
    /// figure.
    static PROTOTYPES: std::cell::RefCell<Vec<(Vec<&'static str>, Engine)>> = const { std::cell::RefCell::new(Vec::new()) };
}

impl Clone for Engine {
    fn clone(&self) -> Self {
        Engine {
            tex: self.tex.clone(),
            st: self.st.clone(),
            saved: self.saved.clone(),
            canvas: self.canvas.clone(),
            nodes: self.nodes.clone(),
            colors: self.colors.clone(),
            page_color: self.page_color,
            varwidth: self.varwidth,
            document_depth: self.document_depth,
            scopes: self.scopes.clone(),
            picture_depth: self.picture_depth,
            pictures: self.pictures,
            steps: self.steps,
            depth: self.depth,
            text_reported: self.text_reported,
            key_fns: self.key_fns.clone(),
            extension_fns: self.extension_fns.clone(),
            command_fns: self.command_fns.clone(),
            hooks: self.hooks.clone(),
            slots: self.slots.clone(),
            canvas_frozen: self.canvas_frozen,
            nesting: self.nesting,
            keys_run: self.keys_run,
            keys_allowed: self.keys_allowed,
            key_lists: self.key_lists,
            keys_capped: self.keys_capped,
            first_char_syntax: self.first_char_syntax,
            pgf_path: self.pgf_path.clone(),
            plot_stream: Vec::new(),
            decoration_envs: self.decoration_envs.clone(),
            named_paths: self.named_paths.clone(),
            last_node: self.last_node.clone(),
            last_node_name: self.last_node_name.clone(),
            tree_count: self.tree_count,
            chain_list: self.chain_list.clone(),
            math_kinds: self.math_kinds.clone(),
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
            tree_edge_out: self.tree_edge_out.clone(),
            behind_path: Vec::new(),
            tree_child_names: self.tree_child_names.clone(),
            forced_box: self.forced_box,
            text_shading: None,
            text_picture: None,
            fadings: self.fadings.clone(),
            picture_fadings: self.picture_fadings.clone(),
            basic_shadings: self.basic_shadings.clone(),
            functional_shadings: self.functional_shadings.clone(),
            plot_marks_declared: self.plot_marks_declared.clone(),
            plot_handlers: self.plot_handlers.clone(),
            saved_paths: self.saved_paths.clone(),
            pgf_solutions: self.pgf_solutions.clone(),
            anim_shifts: Vec::new(),
            sys_animations: HashMap::new(),
            user_patterns: self.user_patterns.clone(),
            user_shapes: self.user_shapes.clone(),
            shape_capture: None,
            extension_shapes: self.extension_shapes.clone(),
            later_node: None,
            scope_ends: self.scope_ends.clone(),
            brace_scopes: Vec::new(),
            layer_order: self.layer_order.clone(),
            layers: self.layers.clone(),
            on_layer: self.on_layer.clone(),
            boxes: self.boxes.clone(),
            layout: self.layout.clone(),
            words: self.words.clone(),
            saved_boxes: self.saved_boxes.clone(),
            current_key: self.current_key.clone(),
            node_sets: self.node_sets.clone(),
            spy_scopes: self.spy_scopes.clone(),
            facts: self.facts.clone(),
        }
    }
}

impl Engine {
    /// A fresh engine with TikZ's core defined.
    pub fn new() -> Engine {
        Engine::of(piece::Parts::default())
    }

    /// A fresh engine with `parts` too (built once per thread for each set of parts, then cloned).
    pub fn of(parts: piece::Parts<'_>) -> Engine {
        arrows::clear_declared();
        let known = PROTOTYPES.with(|p| {
            let prototypes = p.borrow();
            let same = |names: &Vec<&'static str>| names.iter().copied().eq(parts.names());
            prototypes.iter().find(|(names, _)| same(names)).map(|(_, e)| e.clone())
        });
        if let Some(e) = known {
            return e;
        }
        let mut e = Engine::build(parts.pieces);
        for library in parts.libraries {
            e.register(library);
        }
        e.tex.warnings.clear();
        PROTOTYPES.with(|p| p.borrow_mut().push((parts.names().collect(), e.clone())));
        e
    }

    /// Defines a library's keys, then runs its definitions as TeX with `@` a letter.
    fn register(&mut self, library: &crate::extension::Library) {
        self.register_keys(library);
        let path = std::mem::replace(&mut self.st.key_path, "/tikz".into());
        let mut lexer = Tex::new_lexer();
        lexer.push_text(&(library.definitions)());
        let mut tokens = Vec::new();
        while let Some(t) = lexer.get_token() {
            tokens.push(t);
        }
        if let Err(err) = self.run_tokens(tokens) {
            self.warn(format!("library {}: definitions failed: {err}", library.name));
        }
        self.st.key_path = path;
    }

    /// Defines a library's keys only (TikZ's own libraries run their definitions with TikZ's).
    pub(crate) fn register_keys(&mut self, library: &crate::extension::Library) {
        for (path, f) in library.keys {
            let id = EXTENSION_KEY_BASE as usize + self.extension_fns.len();
            if id > u16::MAX as usize {
                self.warn(format!("library {}: too many keys, {path} left out", library.name));
                continue;
            }
            self.extension_fns.push(*f);
            self.tex.define(&crate::keys::macro_name(&format!("{path}/.@cmd")), Meaning::Native(id as u16));
        }
    }

    fn command(&mut self, id: u16, t: &Token) -> Result<(), Error> {
        if id >= piece::PIECE_COMMAND_BASE {
            let command = self.command_fns[(id - piece::PIECE_COMMAND_BASE) as usize];
            return command(self, t);
        }
        if basic::is_basic(id) {
            return self.basic_command(id);
        }
        if lsystem::is_lsystem(id) {
            return self.lsystem_command(id);
        }
        let Some(&(name, cmd)) = COMMANDS.iter().find(|(_, c)| *c as u16 == id) else { return Ok(()) };
        if let Some(r) = self.filtered_statement(name, t) {
            return r;
        }
        if let Some(defaults) = path_command(cmd) {
            let r = path::statement(self, defaults);
            return self.recover(r, t);
        }
        match cmd {
            Cmd::Node => self.command_node(t),
            Cmd::Coordinate => {
                self.tex.push_tokens(tikra_tokens("coordinate "));
                let r = path::statement(self, "");
                self.recover(r, t)
            }
            Cmd::Pic => {
                self.tex.push_tokens(tikra_tokens("pic "));
                let r = path::statement(self, "");
                self.recover(r, t)
            }
            Cmd::NamePrefix => {
                let p = self.argument_string()?;
                self.st.name_prefix = if p == ".." { String::new() } else { p };
                Ok(())
            }
            Cmd::Matrix => self.matrix(),
            Cmd::PgfMatrix => self.pgf_matrix(),
            Cmd::Chainin => self.chainin(),
            Cmd::TikzMath => self.tikzmath(),
            Cmd::DeclareFading => self.declare_fading(),
            Cmd::DeclareImage => self.declare_image(),
            Cmd::DeclarePatternForm => self.declare_pattern(false),
            Cmd::DeclarePatternColored => self.declare_pattern(true),
            Cmd::SetFillPattern => self.set_fill_pattern(),
            Cmd::TikzDeclarePattern => self.tikz_declare_pattern(true),
            Cmd::PgfDeclarePattern => self.tikz_declare_pattern(false),
            Cmd::ArrowsDeclare => self.arrows_declare(),
            Cmd::DeclareShape => self.declare_shape(),
            // `\pgfpositionnodelater{macro}`: the nodes of the group kept, not drawn (the macro,
            // which would save them, is not run: `\pgfpositionnodenow` places the last one)
            Cmd::PositionNodeLater => {
                let _ = self.argument()?;
                self.st.position_later = true;
                Ok(())
            }
            Cmd::PositionNodeNow => self.position_node_now(),
            Cmd::DeclareShadingH => self.declare_shading('h'),
            Cmd::DeclareShadingV => self.declare_shading('v'),
            Cmd::DeclareShadingR => self.declare_shading('r'),
            Cmd::UseShading => self.use_shading(),
            Cmd::ShadePath => self.shade_path(),
            Cmd::SetFading => self.set_fading(),
            Cmd::TransformNonlinear => self.transform_nonlinear(),
            Cmd::ApproximateNonlinear => self.approximate_nonlinear(true),
            Cmd::ApproximateNonlinearTranslation => self.approximate_nonlinear(false),
            Cmd::SetCurvilinear => self.set_curvilinear(),
            Cmd::PointCurvilinear => self.point_curvilinear(),
            Cmd::PointCurvilinearPolar => self.point_curvilinear_polar(),
            Cmd::ToNumber => self.to_number(),
            Cmd::MathSinCos => self.math_sincos(),
            Cmd::SetFadingForPath => self.set_fading_for_path(false),
            Cmd::SetFadingForPathStroked => self.set_fading_for_path(true),
            Cmd::SetSeed => self.command_set_seed(),
            Cmd::DeclareRandomList => self.command_random_list(),
            Cmd::Unsupported => {
                let name = tex::show_token(t);
                let rest = if self.picture_depth > 0 { self.skip_statement()? } else { String::new() };
                self.warn(format!("{name} not supported: '{}' ignored", rest.chars().take(40).collect::<String>().trim()));
                Ok(())
            }
            Cmd::Tikz => self.tikz_command(),
            Cmd::Arrow => self.arrow_command(false),
            Cmd::ArrowReversed => self.arrow_command(true),
            Cmd::DecorationBegin => self.decoration_begin(),
            Cmd::DecorationEnd => self.decoration_end(),
            Cmd::DecorateCurrentPath => self.decorate_current_path(),
            Cmd::Spy => self.spy_command(),
            Cmd::SpyBegin => {
                self.spy_begin();
                Ok(())
            }
            Cmd::SpyEnd => self.spy_end(),
            Cmd::Begin => self.begin_environment(),
            Cmd::End => self.end_environment(),
            Cmd::Scoped => self.command_scoped(),
            Cmd::Tikzset => self.command_tikzset(),
            Cmd::Pgfset => {
                let a = self.argument()?;
                self.keys_command(a, Some("/pgf"), true)
            }
            Cmd::Pgfplotsset => {
                let a = self.argument()?;
                self.keys_command(a, Some("/pgfplots"), true)
            }
            Cmd::Pgfkeys => {
                let a = self.argument()?;
                self.keys_command(a, Some("/"), true)
            }
            Cmd::Pgfqkeys => {
                let p = self.argument_string()?;
                let a = self.argument()?;
                self.keys_command(a, Some(&p), true)
            }
            Cmd::Pgfkeysalso => {
                let a = self.argument()?;
                self.keys_command(a, None, false)
            }
            Cmd::Tikzstyle => self.command_tikzstyle(),
            Cmd::Foreach => self.foreach(),
            Cmd::Definecolor => self.command_definecolor(),
            Cmd::Colorlet => self.command_colorlet(),
            Cmd::Color => self.command_color(),
            Cmd::Pgfqbox => {
                // `\pgfqbox{\box}`: a box put in the picture, outside its size
                let b = self.argument()?;
                self.warn(format!("\\pgfqbox{{{}}}: the box is not drawn", tex::tokens_to_string(&b).trim()));
                Ok(())
            }
            Cmd::Pagecolor => self.command_pagecolor(),
            Cmd::Nopagecolor => {
                self.page_color = None;
                Ok(())
            }
            Cmd::Textcolor => {
                // text, typeset by TeX: outside nodes it is ignored
                let _ = self.argument()?;
                let _ = self.argument()?;
                Ok(())
            }
            Cmd::Pgfmathsetmacro | Cmd::Pgfmathtruncatemacro | Cmd::Pgfmathsetlengthmacro => self.command_math_macro(cmd),
            Cmd::Pgfmathparse => self.command_math_parse(),
            // `\pgfmathadd{a}{b}` and the other functions as commands: `\pgfmathresult` is their value
            Cmd::PgfmathFunction => self.command_math_function(t),
            Cmd::Pgfmathsetcount | Cmd::Pgfmathsetlength | Cmd::Pgfmathaddtolength | Cmd::Pgfmathsetcounter | Cmd::Pgfmathaddtocounter => self.command_math_register(cmd, t),
            Cmd::Pgfmathdeclarefunction => self.command_declare_function(),
            Cmd::Pgfmathprintnumber => self.command_print_number(),
            Cmd::Usetikzlibrary | Cmd::Usepackage | Cmd::Documentclass => self.command_load(cmd),
            Cmd::PassOptionsToPackage => self.command_pass_options(),
            Cmd::OnLayer => {
                let name = self.argument_string()?;
                self.begin_on_layer(name);
                Ok(())
            }
            Cmd::EndOnLayer => {
                self.end_on_layer();
                Ok(())
            }
            Cmd::SetLayers => {
                let list = self.argument_string()?;
                self.layer_order = list.split(',').map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
                Ok(())
            }
            Cmd::Frame => self.frame_macros(),
            Cmd::Tikzfading => self.tikzfading(),
            Cmd::Newsavebox => self.new_save_box(),
            Cmd::Savebox => self.save_box(t.is_cs("savebox")),
            Cmd::Usebox => self.use_box(),
            Cmd::PreviewEnvironment => self.preview_environment(),
            Cmd::Unboxed => self.unboxed(t),
            Cmd::Ignore1 => {
                let _ = self.optional()?;
                let _ = self.argument()?;
                Ok(())
            }
            // `\hspace{1cm}` (or `\hspace*`) between pictures: the space TeX leaves there
            // `\fbox{…}`: a frame of `\fboxrule` around the pictures, `\fboxsep` away
            Cmd::Fbox => self.command_fbox(),
            // `\scalebox{0.7}[1]{…}`: the pictures in it scaled, by the second factor up (the first
            // without it), around their origin
            Cmd::Scalebox => self.command_scalebox(),
            // `\resizebox{7cm}{!}{…}`: the pictures in it scaled to that width and height (`!`
            // keeps the other's ratio); starred, the height is the height plus the depth
            Cmd::Resizebox => self.command_resizebox(),
            Cmd::Hspace => self.command_hspace(),
            // `\hskip 1cm plus 1fil`: glue of its natural width between pictures
            Cmd::Hskip => self.command_hskip(),
            // the picture's size forgotten: what follows makes it anew
            Cmd::ResetBoundingBox => {
                (self.canvas.min, self.canvas.max) = (None, None);
                Ok(())
            }
            Cmd::Ignore2 => {
                let _ = self.argument()?;
                let _ = self.argument()?;
                Ok(())
            }
            Cmd::FontSize => {
                if let Token::Cs(n) = t {
                    self.st.text.font_size = font_size(n);
                    self.st.text.ambient_size = self.st.text.font_size;
                    self.st.text.font.push_str(&format!("\\{n}"));
                }
                Ok(())
            }
            Cmd::CheckUnit => self.command_check_unit(),
            Cmd::Quad => {
                // 1em and 2em of the 10pt font, as glue between the pictures
                if self.picture_depth == 0 {
                    let em = if t.is_cs("qquad") { 2.0 } else { 1.0 };
                    self.layout.push(Layout::Skip(em * 10.0 * crate::PT));
                }
                Ok(())
            }
            Cmd::FontSeries => {
                if let Token::Cs(n) = t {
                    self.st.text.font.push_str(&format!("\\{n}"));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Engine::new()
    }
}

/// Errors that stop the whole figure: they guard against hostile input.
pub fn is_limit(e: &Error) -> bool {
    let s = e.to_string();
    s.contains("figure too large") || s.contains("scope/\\foreach nesting went past") || s.contains("picture options:") || s.contains("scope options:")
}

/// A macro without parameters holding text.
pub fn macro_from(s: &str) -> Meaning {
    macro_tokens(tex::string_tokens(s))
}

pub fn macro_tokens(v: Vec<Token>) -> Meaning {
    Meaning::Macro(Rc::new(tex::Macro { params: Vec::new(), body: v.into_iter().map(tex::Body::Tok).collect(), optional: None, protected: false }))
}

/// Text read as TeX reads it (control sequences, braces), for internal use.
pub fn tikra_tokens(text: &str) -> Vec<Token> {
    let mut t = Tex::new_lexer();
    t.push_text(text);
    let mut v = Vec::new();
    while let Some(x) = t.get_token() {
        v.push(x);
    }
    v
}

/// A number as pgfmath writes one: `2`, `0.5`, `-3.14159`.
pub fn pgf_number(v: f64) -> String {
    let r = (v * 100000.0).round() / 100000.0;
    if r.fract() == 0.0 && r.abs() < 1e15 {
        format!("{}", r as i64)
    } else {
        format!("{r}")
    }
}

fn font_size(name: &str) -> f64 {
    match name {
        "tiny" => 5.0,
        "scriptsize" => 7.0,
        "footnotesize" => 8.0,
        "small" => 9.0,
        "large" => 12.0,
        "Large" => 14.4,
        "LARGE" => 17.28,
        "huge" => 20.74,
        "Huge" => 24.88,
        _ => 10.0,
    }
}

/// `1/a` → [`1`, `a`]: an item split at its top-level slashes.
pub fn split_slash(item: &[Token]) -> Vec<Vec<Token>> {
    let mut parts = vec![Vec::new()];
    let mut depth = 0usize;
    for t in item {
        match t {
            Token::Char(_, Cat::Begin) => depth += 1,
            Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
            Token::Char('/', _) if depth == 0 => {
                parts.push(Vec::new());
                continue;
            }
            _ => {}
        }
        if let Some(p) = parts.last_mut() {
            p.push(t.clone());
        }
    }
    parts.into_iter().map(|p| tex::strip_outer_braces(crate::keys::trim(&p).to_vec())).collect()
}

/// Runs a whole document (or figure) and returns the canvas and the warnings.
pub fn run_document(code: &str) -> Result<(Canvas, Vec<String>), Error> {
    run_document_noting(code).map(|(canvas, warnings, _)| (canvas, warnings))
}

/// [`run_document`], and what the figure shows, for its description in words.
pub fn run_document_noting(code: &str) -> Result<(Canvas, Vec<String>, crate::describe::Facts), Error> {
    run_document_with(code, &[])
}

/// [`run_document_noting`] with libraries from outside defined.
pub fn run_document_with(code: &str, libraries: &[&'static crate::extension::Library]) -> Result<(Canvas, Vec<String>, crate::describe::Facts), Error> {
    run_document_in(code, piece::Parts { pieces: &[], libraries })
}

/// Runs a whole document in an engine built with `parts`.
pub fn run_document_in(code: &str, parts: piece::Parts<'_>) -> Result<(Canvas, Vec<String>, crate::describe::Facts), Error> {
    let mut e = Engine::of(parts);
    crate::expr::reset_random();
    // tikra's margin around a figure, unless a standalone border says otherwise
    e.canvas.margin = 2.0 * crate::PT;
    let (code, files) = crate::files::split_filecontents(code);
    for (name, lines) in files {
        e.tex.define_string(&format!("tikra@file@{name}"), &lines, true);
    }
    e.tex.push_text(&code);
    e.run()?;
    // anything left open is closed
    while e.picture_depth > 0 {
        e.end_picture();
    }
    while !e.scopes.is_empty() {
        e.end_scope();
    }
    let margin = e.canvas.margin;
    e.canvas = typeset(&e.boxes, &e.layout, &e.words, margin);
    e.canvas.page_color = e.page_color;
    let mut warnings = std::mem::take(&mut e.tex.warnings);
    warnings.dedup();
    let _ = Transform::default();
    Ok((e.canvas, warnings, std::mem::take(&mut e.facts)))
}
