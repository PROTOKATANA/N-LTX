//! The animations library, as a still picture shows it: `:fill = {0s="black", 2s="red"}` after a
//! command, `animate={myself:rotate={…}}` in options. A document drawn without `make snapshot of=t`
//! shows everything in its base state, so the animations do nothing; with a snapshot time, each
//! animated attribute takes its value at that time (between two times, numbers, lengths and colors
//! are interpolated; before the first, the first one holds; after the last, the animation is over,
//! unless it freezes, and the attribute is back to its base value), as an option. An
//! animation that waits for an event (`begin on=click`) has not started in a snapshot.

use crate::scene::{Point, Transform};
use crate::tex::{self, Cat, Token};
use crate::Error;

use super::{tikra_tokens, Engine};

/// `2s`, `50ms`, `1.5`: seconds.
fn seconds(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Some(ms) = s.strip_suffix("ms") {
        return ms.trim().parse::<f64>().ok().map(|v| v / 1000.0);
    }
    if let Some(min) = s.strip_suffix("min") {
        return min.trim().parse::<f64>().ok().map(|v| v * 60.0);
    }
    s.strip_suffix('s').unwrap_or(s).trim().parse().ok()
}

/// A number followed by a unit (`2mm`, `90`): the number and the unit.
fn number_unit(s: &str) -> Option<(f64, &str)> {
    let s = s.trim();
    let end = s.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+')).unwrap_or(s.len());
    Some((s[..end].parse().ok()?, s[end..].trim()))
}

/// The value between `a` and `b` at `f` (0…1): numbers with their unit, or colors mixed as xcolor
/// mixes them.
fn between(a: &str, b: &str, f: f64) -> String {
    match (number_unit(a), number_unit(b)) {
        (Some((x, u)), Some((y, w))) if u == w || w.is_empty() => format!("{}{u}", super::pgf_number(x + (y - x) * f)),
        _ if f <= 0.0 => a.to_string(),
        _ if f >= 1.0 => b.to_string(),
        _ => format!("{b}!{}!{a}", super::pgf_number(f * 100.0)),
    }
}

/// The option an attribute becomes (`fill opacity` stays `fill opacity`, `position` is `shift`).
fn option_name(attribute: &str) -> &str {
    match attribute {
        "position" | "translate" => "shift",
        "rotate" | "xshift" | "yshift" | "shift" | "scale" | "xscale" | "yscale" | "xslant" | "yslant" => attribute,
        "fill" | "draw" | "color" | "text" | "line width" | "opacity" | "fill opacity" | "draw opacity" | "text opacity" | "dash pattern" | "dash phase" => attribute,
        other => other,
    }
}

impl Engine {
    /// `make snapshot of=t` (in seconds).
    pub fn set_snapshot(&mut self, v: &str) {
        self.st.snapshot = seconds(v);
    }

    /// The option an animated attribute gives at the snapshot time (none without a snapshot).
    pub fn animation_option(&mut self, attribute: &str, spec: Vec<Token>) -> Result<Option<Vec<Token>>, Error> {
        // `whom:attribute`: the object is the one the options belong to here
        let attribute = attribute.rsplit(':').next().unwrap_or(attribute).trim().to_string();
        let Some(t) = self.st.snapshot else {
            self.animation_room(&attribute, &spec);
            return Ok(None);
        };
        let mut times: Vec<(f64, String)> = Vec::new();
        let mut begin = 0.0;
        let mut freeze = false;
        for item in crate::keys::split_list(&spec) {
            let value = item.value.map(|v| tex::tokens_to_string(&v)).unwrap_or_default();
            let value = value.trim().trim_matches('"').to_string();
            match item.key.as_str() {
                "begin" => begin = seconds(&value).unwrap_or(0.0),
                // waiting for an event: not started in a still picture
                "begin on" => return Ok(None),
                // in a snapshot, the animation started at this time
                "begin snapshot" => begin = seconds(&value).unwrap_or(0.0),
                "freeze" => freeze = true,
                "end" | "repeats" | "forever" | "restart" | "remove" | "whom" | "entry control" | "exit control" | "stay" | "jump" => {}
                key => {
                    if let Some(s) = seconds(key) {
                        times.push((s, value));
                    }
                }
            }
        }
        if times.is_empty() {
            return Ok(None);
        }
        times.sort_by(|a, b| a.0.total_cmp(&b.0));
        let at = t - begin;
        // over: the attribute is back to its base value, unless the animation freezes
        if at > times[times.len() - 1].0 && !freeze {
            return Ok(None);
        }
        let value = match times.iter().position(|(s, _)| *s > at) {
            Some(0) => times[0].1.clone(),
            None => times[times.len() - 1].1.clone(),
            Some(i) => {
                let ((s0, a), (s1, b)) = (&times[i - 1], &times[i]);
                between(a, b, if s1 > s0 { (at - s0) / (s1 - s0) } else { 1.0 })
            }
        };
        let mut v = tikra_tokens(&format!("{}=", option_name(&attribute)));
        v.push(Token::Char('{', Cat::Begin));
        v.extend(tikra_tokens(&value));
        v.push(Token::Char('}', Cat::End));
        // pgf animates the canvas: a rotation or a scaling turns the text and lines too
        if matches!(attribute.as_str(), "rotate" | "scale" | "xscale" | "yscale" | "xslant" | "yslant") {
            let mut w = tikra_tokens("transform canvas=");
            w.push(Token::Char('{', Cat::Begin));
            w.extend(v);
            w.push(Token::Char('}', Cat::End));
            return Ok(Some(w));
        }
        Ok(Some(v))
    }

    /// The shifts an `xshift`, `yshift` or `shift` animation goes through: as pgf does, the picture
    /// makes room for the animated object at each of them.
    fn animation_room(&mut self, attribute: &str, spec: &[Token]) {
        if !matches!(attribute, "xshift" | "yshift" | "shift" | "position" | "translate") {
            return;
        }
        let linear = Transform { e: 0.0, f: 0.0, ..self.st.transform };
        for item in crate::keys::split_list(spec) {
            if seconds(&item.key).is_none() {
                continue;
            }
            let value = item.value.map(|v| tex::tokens_to_string(&v)).unwrap_or_default();
            let value = value.trim().trim_matches('"').trim().to_string();
            let d = match attribute {
                "xshift" => self.length(&value).map(|x| Point::new(x, 0.0)),
                "yshift" => self.length(&value).map(|y| Point::new(0.0, y)),
                _ => {
                    let origin = self.st.transform.apply(Point::default());
                    self.coordinate(value.trim_start_matches('(').trim_end_matches(')')).map(|c| c.p.minus(origin))
                }
            };
            if let Ok(d) = d {
                self.anim_shifts.push(if attribute == "xshift" || attribute == "yshift" { linear.apply(d) } else { d });
            }
        }
    }

    /// `\pgfsysanimate{attribute}` in snapshot mode: the value the timeline gives the attribute at
    /// the snapshot time, an option of the next object named `whom` (pgf attaches it to the id).
    pub fn sys_animate(&mut self, attribute: &str, whom: &str, spec: Vec<Token>) -> Result<(), Error> {
        let attribute = match attribute {
            "fillopacity" => "fill opacity",
            "strokeopacity" => "draw opacity",
            "linewidth" => "line width",
            "opacity" => "opacity",
            // the other attributes keep their base value
            _ => return Ok(()),
        };
        if let Some(o) = self.animation_option(attribute, spec)? {
            let options = self.sys_animations.entry(whom.to_string()).or_default();
            if !options.is_empty() {
                options.push(Token::other(','));
            }
            options.extend(o);
        }
        Ok(())
    }

    /// `:attribute = {…}` after a command or in a path (the `:` read): the option it gives, if any.
    pub fn read_animation(&mut self) -> Result<Option<Vec<Token>>, Error> {
        let mut name = String::new();
        loop {
            match self.tex.get_token() {
                Some(t) if t.is_char('=') => break,
                Some(Token::Char(c, _)) => name.push(c),
                Some(t) => name.push_str(&tex::show_token(&t)),
                None => return Ok(None),
            }
        }
        let spec = match self.tex.next_nonspace() {
            Some(Token::Char(_, Cat::Begin)) => self.tex.read_group_body()?,
            Some(t) => vec![t],
            None => Vec::new(),
        };
        self.animation_option(name.trim(), spec)
    }

    /// `animate={…}`: its `whom:attribute={…}` entries (and styles that give them) at the snapshot.
    pub fn animate_key(&mut self, v: Vec<Token>) -> Result<(), Error> {
        for item in crate::keys::split_list(&v) {
            match item.value {
                // `myself: = { :rotate = {…}, :color = {…} }`: one object, several attributes
                Some(value) if item.key.trim_end().ends_with(':') => {
                    for sub in crate::keys::split_list(&value) {
                        if let Some(o) = self.animation_option(&sub.key, sub.value.unwrap_or_default())? {
                            self.process_keys(o, Some("/tikz"))?;
                        }
                    }
                }
                Some(value) if item.key.contains(':') => {
                    if let Some(o) = self.animation_option(&item.key, value)? {
                        self.process_keys(o, Some("/tikz"))?;
                    }
                }
                // a style of /tikz/animate (`shake = 1mm`): its entries come back here as TikZ keys
                value => {
                    let mut keys = tikra_tokens(&format!("/tikz/animate/{}", item.key));
                    if let Some(value) = value {
                        keys.push(Token::other('='));
                        keys.push(Token::Char('{', Cat::Begin));
                        keys.extend(value);
                        keys.push(Token::Char('}', Cat::End));
                    }
                    self.process_keys(keys, Some("/tikz"))?;
                }
            }
        }
        Ok(())
    }
}
