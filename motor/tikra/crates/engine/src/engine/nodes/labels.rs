use crate::canvas::Item;
use crate::keys;
use crate::scene::{Point, Transform};
use crate::tex::{self, Token};
use crate::{Error, PT};

use super::super::{tikra_tokens, Engine};
use super::*;

/// What `label` and `pin` ask for, once their options are read.
pub(super) struct LabelRequest {
    opts: Vec<Token>,
    text: Vec<Token>,
    position: String,
    given_distance: Option<f64>,
}

/// TikZ's direction names for a label, in degrees (`below` is 270 here).
pub(super) fn label_bearing(position: &str) -> Option<f64> {
    const BEARINGS: &[(&str, f64)] = &[
        ("above", 90.0),
        ("north", 90.0),
        ("below", 270.0),
        ("south", 270.0),
        ("left", 180.0),
        ("west", 180.0),
        ("right", 0.0),
        ("east", 0.0),
        ("above left", 135.0),
        ("north west", 135.0),
        ("above right", 45.0),
        ("north east", 45.0),
        ("below left", 225.0),
        ("south west", 225.0),
        ("below right", 315.0),
        ("south east", 315.0),
        ("center", f64::NAN),
        ("centered", f64::NAN),
    ];
    BEARINGS.iter().find(|(name, _)| *name == position).map(|(_, angle)| *angle)
}

impl Engine {
    /// The options of a label, and the position and text they leave.
    fn read_label_request(&mut self, spec: Vec<Token>, pin: bool) -> Result<LabelRequest, Error> {
        let mut v = keys::trim(&spec).to_vec();
        let mut opts = Vec::new();
        if v.first().is_some_and(|t| t.is_char('[')) {
            let mut depth = 0usize;
            let mut end = 0;
            for (i, t) in v.iter().enumerate() {
                if t.is_char('[') {
                    depth += 1;
                } else if t.is_char(']') {
                    depth -= 1;
                    if depth == 0 {
                        end = i;
                        break;
                    }
                }
            }
            opts = v[1..end].to_vec();
            v = v[end + 1..].to_vec();
        }
        let s = tex::tokens_to_string(&v);
        // the angle is up to the first top-level colon
        let (angle_text, text) = match split_colon(&v) {
            // (expanded: `pin=\angle+15:text` is at the angle the macro gives)
            Some((a, b)) => (self.math_string(a)?.trim().to_string(), b),
            None => (String::new(), v.clone()),
        };
        let _ = s;
        // with `direction shorthands` (the quotes library's labels, `"text" above`), a direction
        // among the label's options is its position; otherwise it places the label's anchor
        let shorthands = keys::split_list(&opts).iter().any(|i| i.key == "direction shorthands");
        let directions: &[&str] =
            if shorthands { &["above", "below", "left", "right", "above left", "above right", "below left", "below right", "centered", "direction shorthands"] } else { &[] };
        let mut given = None;
        // `label position` among the label's options is its default position (`\tikz@label@angle`
        // expands it after them)
        let distance_key = if pin { "pin distance" } else { "label distance" };
        let mut given_distance = None;
        for i in keys::split_list(&opts) {
            if i.key == "label position" {
                if let Some(v) = &i.value {
                    given = Some(tex::tokens_to_string(v).trim().to_string());
                }
            }
            // and the distance among them is read after them too
            if i.key == distance_key {
                if let Some(v) = i.value {
                    given_distance = self.length_of(v).ok();
                }
            }
        }
        let items = keys::split_list(&opts);
        let taken = |i: &keys::Item| i.value.is_none() && directions.contains(&i.key.as_str());
        if items.iter().any(taken) {
            let mut rest = Vec::new();
            for i in items {
                if i.value.is_none() && directions.contains(&i.key.as_str()) {
                    if i.key != "direction shorthands" {
                        given = Some(i.key.clone());
                    }
                } else {
                    if !rest.is_empty() {
                        rest.push(Token::other(','));
                    }
                    rest.extend(tikra_tokens(&i.key));
                    if let Some(v) = i.value {
                        rest.push(Token::other('='));
                        rest.push(Token::Char('{', tex::Cat::Begin));
                        rest.extend(v);
                        rest.push(Token::Char('}', tex::Cat::End));
                    }
                }
            }
            opts = rest;
        }
        let position = if !angle_text.is_empty() {
            angle_text
        } else if let Some(g) = given {
            g
        } else {
            self.key_text("/tikz/label position").trim().to_string()
        };
        Ok(LabelRequest { opts, text, position, given_distance })
    }

    /// Where a label sits, and the anchor it is pinned by.
    fn label_place(&mut self, info: &NodeInfo, angle: f64, distance: f64) -> (Point, String) {
        // as `tikz@label@post`: at the node's anchor of that (whole) angle, moved away from the
        // center by the distance, with the anchor on the side away from the node
        let absolute = matches!(self.tex.meaning_of("tikra@absolute"), tex::Meaning::Macro(m) if tex::tokens_to_string(&tex::substitute(&m.body, &[])).trim() == "true");
        if angle.is_nan() {
            return (info.center(), "center".to_string());
        }
        if absolute {
            // `absolute`: the angle on the page, whatever the node's transformation
            let u = Point::new(angle.to_radians().cos(), angle.to_radians().sin());
            let border = info.border_toward(info.center().plus(u.scale(PT)));
            return (border.plus(u.scale(distance)), direction_anchor(angle).to_string());
        }
        let border = info.anchor(&format!("{}", angle.trunc())).unwrap_or(info.center());
        let d = border.minus(info.center());
        if d.norm() < 1e-9 {
            // a node without size (a coordinate): straight out at the angle
            return (border.plus(Point::new(angle.to_radians().cos(), angle.to_radians().sin()).scale(distance)), direction_anchor(angle).to_string());
        }
        let u = d.scale(1.0 / d.norm());
        (border.plus(u.scale(distance)), auto_anchor(Point::new(u.y, -u.x).scale(PT), false).to_string())
    }

    /// The edge from the node to its pin.
    fn pin_edge(&mut self, info: &NodeInfo, items: Vec<Item>) -> Result<Vec<Item>, Error> {
        let Some(label) = self.last_node.clone() else { return Ok(items) };
        let edge_options = super::super::spy::macro_body(self, "tikra@pinedge");
        let saved = (self.last_node.clone(), self.last_node_name.clone());
        self.nodes.insert("tikra@pin@node".into(), info.clone());
        self.nodes.insert("tikra@pin@label".into(), label);
        self.begin_group();
        self.st.transform = Transform::default();
        let mut path = tikra_tokens("\\path (tikra@pin@node) edge[every pin edge/.try,");
        path.extend(edge_options);
        path.extend(tikra_tokens("] (tikra@pin@label);"));
        let r = self.run_tokens(path);
        self.end_group();
        r?;
        (self.last_node, self.last_node_name) = saved;
        Ok(items)
    }

    /// `label=[options]angle:text`, `pin=…`: another node next to this one.
    pub(crate) fn label(&mut self, info: &NodeInfo, spec: Vec<Token>, pin: bool) -> Result<Vec<Item>, Error> {
        let req = self.read_label_request(spec, pin)?;
        let angle = label_bearing(&req.position).unwrap_or_else(|| self.number(&req.position).unwrap_or(90.0));
        let distance = match req.given_distance {
            Some(d) => d,
            None if pin => self.key_length("/tikz/pin distance", 3.0 * 0.1515),
            None => self.key_length("/tikz/label distance", 0.0),
        };
        let (at, anchor) = self.label_place(info, angle, distance);
        let mut options = tikra_tokens(&format!("every {}/.try,anchor={anchor},", if pin { "pin" } else { "label" }));
        options.extend(req.opts);
        // `pin edge={…}` among the label's options or styles sets the options of its edge, the
        // last one given winning
        self.tex.set_meaning(&Token::cs("tikra@pinedge"), super::super::macro_from(""), true);
        let items = self.node(NodeSpec { options, name: None, at: Some(at), text: req.text, place: OnPath::Point(at), point: at, coordinate: false, default_pos: None })?;
        if pin {
            return self.pin_edge(info, items);
        }
        Ok(items)
    }
}
/// TikZ's `\tikz@auto@anchor` (left of the direction) and `\tikz@auto@anchor@prime` (right).
pub(super) fn auto_anchor(d: Point, right: bool) -> &'static str {
    let e = 0.05 * PT;
    let (x, y) = (d.x, d.y);
    let left = if x > e {
        if y > e {
            "south east"
        } else if y < -e {
            "south west"
        } else {
            "south"
        }
    } else if x < -e {
        if y > e {
            "north east"
        } else if y < -e {
            "north west"
        } else {
            "north"
        }
    } else if y > 0.0 {
        "east"
    } else {
        "west"
    };
    if !right {
        return left;
    }
    match left {
        "south east" => "north west",
        "south west" => "north east",
        "south" => "north",
        "north east" => "south west",
        "north west" => "south east",
        "north" => "south",
        "east" => "west",
        _ => "east",
    }
}

/// `\tikz@compute@direction`: the anchor of a label at `angle` (whole degrees) away from a point.
pub(super) fn direction_anchor(angle: f64) -> &'static str {
    let a = (angle.trunc() as i64).rem_euclid(360);
    match a {
        0..=3 => "west",
        4..=86 => "south west",
        87..=93 => "south",
        94..=176 => "south east",
        177..=183 => "east",
        184..=266 => "north east",
        267..=273 => "north",
        274..=356 => "north west",
        _ => "west",
    }
}

/// `30:text` → (`30`, `text`): split at the first colon outside braces.
pub(super) fn split_colon(v: &[Token]) -> Option<(Vec<Token>, Vec<Token>)> {
    let mut depth = 0usize;
    for (i, t) in v.iter().enumerate() {
        match t {
            Token::Char(_, tex::Cat::Begin) => depth += 1,
            Token::Char(_, tex::Cat::End) => depth = depth.saturating_sub(1),
            Token::Char(':', _) if depth == 0 => return Some((v[..i].to_vec(), tex::strip_outer_braces(keys::trim(&v[i + 1..]).to_vec()))),
            _ => {}
        }
    }
    None
}
