//! Shapes declared in the document: `\pgfdeclareshape{name}{…}`. What a node of the shape has is
//! computed when the node is made, as pgf does: the saved anchors, dimensions and macros
//! (`\savedanchor`, `\saveddimen`, `\savedmacro`) from the node's text box and sizes, then the
//! anchors (`\anchor{name}{code}`) from them, then the paths of each layer (`\backgroundpath{code}`,
//! `\behindbackgroundpath{code}`, …). The shape it inherits from (`\inheritsavedanchors[from=…]`)
//! gives what it does not declare: its saved anchors, anchors, border and inherited paths.

use std::collections::VecDeque;

use crate::scene::{Point, Segment, Transform};
use crate::tex::{self, Token};
use crate::{Error, PT};

use super::shapes::{Border, Geometry, Layer, ShapePath, ShapeText, Usage};
use super::Engine;

/// The box register that holds the node's text while its shape is computed (`\pgfnodeparttextbox`).
pub const TEXT_BOX: i64 = 250;

/// What a `\saved…` command saves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Saved {
    /// a point (`\pgf@x`, `\pgf@y` after the code)
    Anchor,
    /// a dimension (`\pgf@x` after the code)
    Dimen,
    /// the macro the code defines
    Macro,
}

/// A declared shape: the shape whose geometry it takes, its saved values, anchors and paths' code.
#[derive(Debug, Clone, Default)]
pub struct UserShape {
    pub base: String,
    /// `\savedanchor\name{code}` and the others, in order
    pub saved: Vec<(Saved, String, Vec<Token>)>,
    pub anchors: Vec<(String, Vec<Token>)>,
    /// `\inheritanchorborder[from=…]`
    pub border: Option<String>,
    /// the layers whose paths come from the inherited shape (`\inheritbackgroundpath`, …)
    pub inherited: Vec<Layer>,
    pub paths: Vec<(Layer, Vec<Token>)>,
}

/// The node's sizes the shape's code reads: its text box (wd, ht, dp), minimum width and height,
/// inner and outer separations.
#[derive(Debug, Clone, Copy)]
pub struct ShapeInput {
    pub text: (f64, f64, f64),
    pub minimum: Point,
    pub inner: Point,
    pub outer: Point,
}

/// What `\pgfusepath` and `\pgftext` give while a shape's path code runs: paths and texts of the
/// shape, in the layer being run.
#[derive(Clone, Debug, Default)]
pub struct ShapeCapture {
    pub layer: Option<Layer>,
    pub paths: Vec<ShapePath>,
    pub texts: Vec<ShapeText>,
}

fn layer_of(cs: &str) -> Option<Layer> {
    Some(match cs {
        "behindbackgroundpath" => Layer::BehindBackground,
        "backgroundpath" => Layer::Background,
        "beforebackgroundpath" => Layer::BeforeBackground,
        "behindforegroundpath" => Layer::BehindForeground,
        "foregroundpath" => Layer::Foreground,
        "beforeforegroundpath" => Layer::BeforeForeground,
        _ => return None,
    })
}

/// The layers in the order pgf runs them.
const LAYERS: [Layer; 6] = [Layer::BehindBackground, Layer::Background, Layer::BeforeBackground, Layer::BehindForeground, Layer::Foreground, Layer::BeforeForeground];

/// Code with `\relax` after it: a dimension at its end is not read on past it.
fn ended(code: &[Token]) -> Vec<Token> {
    let mut v = code.to_vec();
    v.push(Token::cs("relax"));
    v
}

fn sp(v: f64) -> i64 {
    (v / PT * 65536.0).round() as i64
}

fn skip_spaces(rest: &mut VecDeque<Token>) {
    while rest.front().is_some_and(|t| t.is_space()) {
        rest.pop_front();
    }
}

/// The argument at the front: a group's content, or one token.
fn take_group(rest: &mut VecDeque<Token>) -> Vec<Token> {
    skip_spaces(rest);
    match rest.pop_front() {
        Some(Token::Char(_, tex::Cat::Begin)) => {
            let mut depth = 1;
            let mut out = Vec::new();
            while let Some(t) = rest.pop_front() {
                match t {
                    Token::Char(_, tex::Cat::Begin) => depth += 1,
                    Token::Char(_, tex::Cat::End) => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                out.push(t);
            }
            out
        }
        Some(t) => vec![t],
        None => Vec::new(),
    }
}

/// `[from=name]`, or nothing.
fn take_option(rest: &mut VecDeque<Token>) -> String {
    skip_spaces(rest);
    if !rest.front().is_some_and(|t| t.is_char('[')) {
        return String::new();
    }
    rest.pop_front();
    let mut out = Vec::new();
    while let Some(t) = rest.pop_front() {
        if t.is_char(']') {
            break;
        }
        out.push(t);
    }
    let s = tex::tokens_to_string(&out);
    s.split_once('=').map_or(s.clone(), |(_, v)| v.to_string()).trim().to_string()
}

fn push_saved(shape: &mut UserShape, cs: &str, rest: &mut VecDeque<Token>) {
    let kind = match cs {
        "savedanchor" => Saved::Anchor,
        "saveddimen" => Saved::Dimen,
        _ => Saved::Macro,
    };
    let target = take_group(rest);
    let code = take_group(rest);
    if let [Token::Cs(n)] = &target[..] {
        shape.saved.push((kind, n.to_string(), code));
    }
}

/// The body of `\pgfdeclareshape`, and the warnings for commands that are not supported.
fn shape_from_body(name: &str, body: Vec<Token>) -> (UserShape, Vec<String>) {
    let mut shape = UserShape { base: "rectangle".into(), ..UserShape::default() };
    let mut warnings = Vec::new();
    let mut rest: VecDeque<Token> = body.into_iter().collect();
    while let Some(t) = rest.pop_front() {
        let Token::Cs(cs) = &t else { continue };
        match &**cs {
            "inheritsavedanchors" => shape.base = take_option(&mut rest),
            "inheritanchorborder" => shape.border = Some(take_option(&mut rest)),
            "inheritanchor" => {
                // the inherited shape's anchors are kept
                take_option(&mut rest);
                take_group(&mut rest);
            }
            c if c.starts_with("inherit") && layer_of(&c[7..]).is_some() => {
                take_option(&mut rest);
                shape.inherited.extend(layer_of(&c[7..]));
            }
            c if layer_of(c).is_some() => {
                let code = take_group(&mut rest);
                shape.paths.extend(layer_of(c).map(|l| (l, code)));
            }
            "savedanchor" | "saveddimen" | "savedmacro" => push_saved(&mut shape, cs, &mut rest),
            "anchor" => {
                let a = tex::tokens_to_string(&take_group(&mut rest));
                let code = take_group(&mut rest);
                shape.anchors.push((a.split_whitespace().collect::<Vec<_>>().join(" "), code));
            }
            "anchorborder" | "nodeparts" => {
                warnings.push(format!("\\pgfdeclareshape{{{}}}: \\{cs} not supported: the inherited ones used", name.trim()));
                take_group(&mut rest);
            }
            _ => {}
        }
    }
    (shape, warnings)
}

fn define_point(e: &mut Engine, name: &str, p: Point) {
    let code = super::tikra_tokens(&format!("\\pgf@x={}pt\\pgf@y={}pt\\relax", p.x / PT, p.y / PT));
    e.tex.set_meaning(&Token::cs(name), super::macro_tokens(code), false);
}

fn bind_shape_box(e: &mut Engine, input: ShapeInput) -> Result<(), Error> {
    let (wd, ht, dp) = input.text;
    // (a box register only here: outside, `\setbox\pgfnodeparttextbox` names a node part's box)
    e.run_tokens(super::tikra_tokens(&format!("\\chardef\\pgfnodeparttextbox={TEXT_BOX}\\relax")))?;
    e.tex.box_dims.insert(TEXT_BOX, [sp(wd), sp(ht), sp(dp)]);
    for (m, v) in [
        ("pgfshapeminwidth", input.minimum.x),
        ("pgfshapeminheight", input.minimum.y),
        ("pgfshapeinnerxsep", input.inner.x),
        ("pgfshapeinnerysep", input.inner.y),
        ("pgfshapeouterxsep", input.outer.x),
        ("pgfshapeouterysep", input.outer.y),
    ] {
        e.tex.define_string(m, &format!("{}pt", v / PT), false);
    }
    e.st.transform = Transform::default();
    Ok(())
}

fn bind_inherited_anchors(e: &mut Engine, g: &Geometry) {
    for (n, a) in [("southwest", "south west"), ("northeast", "north east"), ("centerpoint", "center")] {
        if let Some(p) = g.anchor(a) {
            define_point(e, n, p);
        }
    }
    if let Border::Ellipse(r) = g.border {
        e.tex.define_string("radius", &format!("{}pt", r.x / PT), false);
    }
}

fn run_saved_values<'a>(e: &mut Engine, shape: &'a UserShape) -> Result<Vec<(&'a str, Point)>, Error> {
    let mut points = Vec::new();
    for (kind, n, code) in &shape.saved {
        e.begin_group();
        let r = e.run_tokens(ended(code));
        let (p, m) = (e.pgf_xy(), e.tex.meaning(&Token::cs(n)));
        e.end_group();
        r?;
        match kind {
            Saved::Anchor => {
                define_point(e, n, p);
                points.push((n.as_str(), p));
            }
            Saved::Dimen => e.tex.define_string(n, &format!("{}pt", p.x / PT), false),
            Saved::Macro => e.tex.set_meaning(&Token::cs(n), m, false),
        }
    }
    Ok(points)
}

struct AnchorRun<'a> {
    name: &'a str,
    shape: &'a UserShape,
}

fn bind_anchor_macros(e: &mut Engine, g: &Geometry, run: &AnchorRun<'_>) {
    for (a, p) in g.anchors.clone() {
        define_point(e, &format!("pgf@anchor@{}@{a}", run.name), p);
    }
    for (a, code) in &run.shape.anchors {
        e.tex.set_meaning(&Token::cs(&format!("pgf@anchor@{}@{a}", run.name)), super::macro_tokens(code.clone()), false);
    }
}

fn measure_anchors(e: &mut Engine, g: &mut Geometry, shape: &UserShape) -> Result<(), Error> {
    let mut declared = Vec::new();
    for (a, code) in &shape.anchors {
        e.begin_group();
        let r = e.run_tokens(ended(code));
        let p = e.pgf_xy();
        e.end_group();
        r?;
        declared.push((a.clone(), p));
    }
    g.anchors.retain(|(n, _)| !declared.iter().any(|(d, _)| d == n));
    if let Some((_, c)) = declared.iter().find(|(n, _)| n == "center") {
        g.center = *c;
    }
    g.anchors.extend(declared);
    Ok(())
}

fn border_from_saved(g: &mut Geometry, shape: &UserShape, saved: &[(&str, Point)]) {
    let point = |n: &str| saved.iter().find(|(m, _)| *m == n).map(|(_, p)| *p);
    if let (Some("rectangle"), Some(sw), Some(ne)) = (shape.border.as_deref(), point("southwest"), point("northeast")) {
        g.center = sw.lerp(ne, 0.5);
        g.border = Border::Rectangle(ne.minus(sw).scale(0.5));
    }
}

struct LayerCapture<'a> {
    layer: Layer,
    code: &'a [Token],
    left: Vec<Segment>,
}

fn capture_layer(e: &mut Engine, g: &mut Geometry, cap: LayerCapture<'_>) -> Result<Vec<Segment>, Error> {
    let LayerCapture { layer, code, left } = cap;
    let saved_path = std::mem::take(&mut e.pgf.pgf_path);
    e.pgf.pgf_path.segments = left;
    let saved_capture = e.declared.shape_capture.replace(ShapeCapture { layer: Some(layer), ..ShapeCapture::default() });
    e.begin_group();
    let r = e.run_tokens(ended(code));
    e.end_group();
    let capture = std::mem::replace(&mut e.declared.shape_capture, saved_capture).unwrap_or_default();
    let path = std::mem::replace(&mut e.pgf.pgf_path, saved_path);
    r?;
    g.paths.extend(capture.paths);
    g.texts.extend(capture.texts);
    Ok(path.segments)
}

fn run_layer_paths(e: &mut Engine, g: &mut Geometry, shape: &UserShape) -> Result<(), Error> {
    // the inherited shape's paths only in the layers inherited
    g.paths.retain(|p| shape.inherited.contains(&p.layer));
    if !shape.paths.is_empty() {
        g.roundable = false;
    }
    // the code of each layer, in pgf's order: what it uses is drawn as it says; the path left
    // over goes on to the background or foreground path, which is used as the node says
    let mut left = Vec::new();
    for layer in LAYERS {
        for (_, code) in shape.paths.iter().filter(|(l, _)| *l == layer) {
            left = capture_layer(e, g, LayerCapture { layer, code, left })?;
        }
        if matches!(layer, Layer::Background | Layer::Foreground) && !left.is_empty() {
            let segments = std::mem::take(&mut left);
            g.paths.push(ShapePath { layer, segments, usage: Usage::Node });
        }
    }
    Ok(())
}

impl Engine {
    /// `\pgfdeclareshape{name}{body}`.
    /// `\pgfdeclareshape{name}{body}`.
    pub fn declare_shape(&mut self) -> Result<(), Error> {
        let name = self.argument_string()?;
        let body = self.argument()?;
        let (mut shape, warnings) = shape_from_body(&name, body);
        for warning in warnings {
            self.warn(warning);
        }
        if !super::shapes::is_shape(&shape.base) {
            self.warn(format!("\\pgfdeclareshape{{{}}}: shape '{}' to inherit from not supported: a rectangle", name.trim(), shape.base));
            shape.base = "rectangle".into();
        }
        self.declared.user_shapes.insert(name.trim().to_string(), shape);
        Ok(())
    }

    /// The geometry of a node of a declared shape: the inherited shape's, with the declared saved
    /// values, anchors and paths, their code run in the node's coordinates (those of pgf's shapes:
    /// the text's origin at 0).
    pub fn user_shape_geometry(&mut self, name: &str, shape: &UserShape, mut g: Geometry, input: ShapeInput) -> Result<Geometry, Error> {
        self.begin_group();
        let r = self.user_shape_in_group(name, shape, &mut g, input);
        self.end_group();
        r?;
        Ok(g)
    }

    fn user_shape_in_group(&mut self, name: &str, shape: &UserShape, g: &mut Geometry, input: ShapeInput) -> Result<(), Error> {
        // what the code reads: the text box, the sizes as `\pgfshape…` macros
        bind_shape_box(self, input)?;
        // the inherited shape's saved anchors
        bind_inherited_anchors(self, g);
        // the declared ones, in order, each from what the ones before saved
        let points = run_saved_values(self, shape)?;
        // the anchors: each also `\pgf@anchor@shape@anchor`, as the code may call them
        bind_anchor_macros(self, g, &AnchorRun { name, shape });
        measure_anchors(self, g, shape)?;
        // the rectangle's border around the shape's own corners, measured from their middle
        border_from_saved(g, shape, &points);
        run_layer_paths(self, g, shape)
    }
}
