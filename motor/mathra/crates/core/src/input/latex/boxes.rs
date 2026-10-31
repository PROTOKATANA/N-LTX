//! Boxes that move, scale, frame or overlap their contents.

use super::*;

impl Reader<'_> {
    pub(super) fn box_command(&mut self, call: CommandCall<'_>) -> R<Option<Option<Stop>>> {
        let node: Node = match call.name {
            "genfrac" => self.general_fraction()?,
            "mathchoice" => {
                let mut branches = Vec::with_capacity(4);
                for _ in 0..4 {
                    branches.push(self.argument()?);
                }
                Node::Choice { branches }
            }
            "vcenter" => Node::VCenter { base: Box::new(self.argument()?) },
            "smash" => {
                let option = self.raw_optional().unwrap_or_default();
                let base = self.argument()?;
                Node::Smashed { base: Box::new(base), height: option != "b", depth: option != "t" }
            }
            "mathllap" | "mathrlap" | "mathclap" | "llap" | "rlap" | "clap" => {
                let side = match call.name.trim_start_matches("math") {
                    "llap" => 'l',
                    "rlap" => 'r',
                    _ => 'c',
                };
                let base = if call.name.starts_with("math") { self.argument()? } else { self.text()? };
                Node::Overlapped { base: Box::new(base), side }
            }
            // graphicx's boxes, their argument in text mode
            "raisebox" => self.raised_box(call.column)?,
            "scalebox" => self.scaled_box(call.column)?,
            "resizebox" => self.resized_box(call.column, call.starred)?,
            "rotatebox" => self.rotated_box(call.column)?,
            "rule" => self.drawn_rule(call.column)?,
            "sideset" => self.side_set(call.items)?,
            "textsuperscript" | "textsubscript" => {
                let t = self.text()?;
                if call.name == "textsuperscript" {
                    Node::Scripts { base: Box::new(Node::row(vec![])), sup: Some(Box::new(t)), sub: None }
                } else {
                    Node::Scripts { base: Box::new(Node::row(vec![])), sup: None, sub: Some(Box::new(t)) }
                }
            }
            _ => return Ok(None),
        };
        call.items.push(node);
        Ok(Some(None))
    }

    /// `\genfrac`: a fraction with its own delimiters, thickness and style.
    fn general_fraction(&mut self) -> R<Node> {
        let open = self.raw_text()?;
        let close = self.raw_text()?;
        let thickness = self.raw_text()?;
        let style = self.raw_text()?;
        let num = self.argument()?;
        let den = self.argument()?;
        let style = match style.trim() {
            "0" => FractionStyle::Display,
            "" => FractionStyle::Auto,
            _ => FractionStyle::Row,
        };
        let rule = !thickness.trim().trim_start_matches(['0', '.']).trim_end_matches(char::is_alphabetic).is_empty() || thickness.trim().is_empty();
        // an empty thickness is the font's; a given one is drawn as given
        let given = measure_em(&thickness).filter(|_| rule);
        let fraction = Node::Fraction { num: Box::new(num), den: Box::new(den), style, rule, thickness: given };
        let delim = |t: &str| -> Option<char> {
            let t = t.trim();
            match t.strip_prefix('\\') {
                Some(n) => symbols::delimiter_command(n).flatten(),
                None => t.chars().next().and_then(|c| symbols::delimiter_char(c).flatten()),
            }
        };
        if open.trim().is_empty() && close.trim().is_empty() {
            Ok(fraction)
        } else {
            Ok(Node::Delimited { open: delim(&open), close: delim(&close), parts: vec![fraction], middles: vec![] })
        }
    }

    /// `\raisebox{lift}[height][depth]{text}`.
    fn raised_box(&mut self, column: usize) -> R<Node> {
        let measure = self.raw_text()?;
        let em = measure_em(&measure).ok_or(Error::Syntax { column, message: format!("unrecognized measure: {measure}") })?;
        let height = self.box_length(column)?;
        let depth = if height.is_some() { self.box_length(column)? } else { None };
        Ok(Node::Raised { em, height: height.flatten(), depth: depth.flatten(), base: Box::new(self.text()?) })
    }

    /// `\scalebox{h}[v]{text}`. A factor is kept between 0.05 and 20, and the sign is kept.
    fn scaled_box(&mut self, column: usize) -> R<Node> {
        let factor = |s: &str| s.trim().parse::<f32>().ok().filter(|f| f.is_finite()).map(|f| f.signum() * f.abs().clamp(0.05, 20.0));
        let x = factor(&self.raw_text()?).ok_or(Error::Syntax { column, message: "\\scalebox{factor}{…} takes a number".into() })?;
        let y = match self.raw_optional() {
            Some(y) => Some(factor(&y).ok_or(Error::Syntax { column, message: "\\scalebox{h}[v]{…} takes numbers".into() })?),
            None => None,
        };
        Ok(Node::Scaled { factor: x, vertical: y.filter(|y| *y != x), base: Box::new(self.text()?) })
    }

    /// `\resizebox{width}{height}{text}`. `!` leaves that side alone. The star uses the total height.
    fn resized_box(&mut self, column: usize, starred: bool) -> R<Node> {
        let mut size = Vec::with_capacity(2);
        for _ in 0..2 {
            let raw = self.raw_text()?;
            size.push(match raw.trim() {
                "!" => None,
                m => match box_length(m) {
                    Some(l) => Some(l),
                    None => return self.error(column, format!("\\resizebox: unrecognized measure: {m}")),
                },
            });
        }
        Ok(Node::Resized { width: size[0], height: size[1], total: starred, base: Box::new(self.text()?) })
    }

    /// `\rotatebox[origin=…]{angle}{text}`. `x=`, `y=` and `units=` are ignored. A bare `origin` is the center.
    fn rotated_box(&mut self, column: usize) -> R<Node> {
        let options = self.raw_optional().unwrap_or_default();
        let origin = options.split(',').find_map(|o| o.trim().strip_prefix("origin")).map(|o| o.trim_start().trim_start_matches('=').trim().to_string()).unwrap_or_default();
        let origin = if options.contains("origin") && origin.is_empty() { "c".to_string() } else { origin };
        let angle = self.raw_text()?;
        let degrees = angle.trim().parse::<f32>().ok().filter(|a| a.is_finite()).ok_or(Error::Syntax { column, message: "\\rotatebox{angle}{…} takes a number".into() })?;
        Ok(Node::Rotated { degrees: degrees % 360.0, origin, base: Box::new(self.text()?) })
    }

    /// `\rule[raise]{width}{height}`.
    fn drawn_rule(&mut self, column: usize) -> R<Node> {
        let raise = self.raw_optional().and_then(|m| measure_em(&m)).unwrap_or(0.0);
        let width = self.raw_text()?;
        let height = self.raw_text()?;
        match (measure_em(&width), measure_em(&height)) {
            (Some(width), Some(height)) => Ok(Node::Rule { width, height, raise }),
            _ => self.error(column, "\\rule{width}{height}: unrecognized measures"),
        }
    }

    /// `\sideset{before}{after}{operator}`: the operator's own scripts stay on it.
    fn side_set(&mut self, items: &mut Vec<Node>) -> R<Node> {
        let before = self.raw_text()?;
        let after = self.raw_text()?;
        let pre = self.sub_reader(&format!("{{}}{before}"), self.variant)?;
        let pos = self.sub_reader(&format!("{{}}{after}"), self.variant)?;
        let op = self.argument()?;
        let mut tmp = vec![op];
        self.skip_spaces();
        if matches!(self.current(), Some('^' | '_')) {
            self.scripts(&mut tmp)?;
        }
        items.push(pre);
        items.extend(tmp);
        Ok(pos)
    }

    /// The optional `[length]` of a graphicx box: `None` without brackets; `Some(None)` for a length it
    /// cannot read, ignored in tolerant mode (E-REC-9).
    pub(super) fn box_length(&mut self, col: usize) -> R<Option<Option<BoxLength>>> {
        let Some(raw) = self.raw_optional() else { return Ok(None) };
        match box_length(&raw) {
            Some(l) => Ok(Some(Some(l))),
            None => {
                self.tolerate(rules::E_REC_9, col, format!("unrecognized measure: {}", raw.trim()), "ignored")?;
                Ok(Some(None))
            }
        }
    }
}
