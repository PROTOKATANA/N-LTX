use crate::canvas::{Fill, Item, Paint, Stroke};
use crate::label;
use crate::scene::{Point, Segment, Transform};
use crate::tex::{self, Token};
use crate::{Error, PT};

use super::super::graphics;
use super::super::{tikra_tokens, Engine};

impl Engine {
    /// The node's drawing, and what its labels need: the node, their specifications, and the last
    /// node to leave after them.
    /// A picture inside a node's text, drawn on a canvas of its own, its coordinates from its
    /// origin: what it draws and its box's corners (none when it draws nothing).
    pub(super) fn inner_picture(&mut self, code: Vec<Token>) -> Result<Option<(Vec<Item>, Point, Point)>, Error> {
        let saved_canvas = std::mem::take(&mut self.canvas);
        let saved_frozen = std::mem::replace(&mut self.canvas_frozen, false);
        self.begin_group();
        self.begin_scope();
        self.st.transform = Transform::default();
        self.st.pending = Transform::default();
        let r = self.run_tokens(code);
        self.end_scope();
        self.end_group();
        let crate::canvas::Canvas { items, min, max, .. } = std::mem::replace(&mut self.canvas, saved_canvas);
        self.canvas_frozen = saved_frozen;
        r?;
        Ok(min.zip(max).map(|(lo, hi)| (items, lo, hi)))
    }

    /// A node's text expanded, the pgfmath commands and the definitions in it run where they are,
    /// as TeX runs them while it typesets the text: `\pgfmathparse{2+2}\pgfmathresult` is 4, and
    /// `\def\a{x}\a` is x. A picture in the text is left as it is, for it to be drawn.
    pub(super) fn expand_text(&mut self, tokens: Vec<Token>) -> Result<Vec<Token>, Error> {
        const RUN: &[&str] = &[
            "pgfmathparse",
            "pgfmathsetmacro",
            "pgfmathtruncatemacro",
            "pgfmathsetlengthmacro",
            "def",
            "gdef",
            "edef",
            "xdef",
            "let",
            "advance",
            "multiply",
            "divide",
            "@ifnextchar",
            "setkeys",
            "begin",
            "tikz",
        ];
        // a \relax at the end, so that an assignment's number stops there, as at the end of TeX's
        // box, and not after the text
        let mut tokens = tokens;
        tokens.push(Token::cs("relax"));
        let end = |mut out: Vec<Token>| {
            if out.last().is_some_and(|t| t.is_cs("relax")) {
                out.pop();
            }
            Ok(out)
        };
        let depth = self.tex.expand_start(tokens);
        let mut out = Vec::new();
        loop {
            let (more, stop) = self.tex.expand_more(depth, RUN)?;
            // \relax, and what is \let to it, typesets nothing
            out.extend(more.into_iter().filter(|t| !matches!(t, Token::Cs(_)) || self.tex.meaning(t) != tex::Meaning::Prim(tex::Prim::Relax)));
            let Some(t) = stop else { return end(out) };
            if t.is_cs("begin") || t.is_cs("tikz") {
                // `\begin{name}` as it is; from a picture on, the rest too (its `\foreach \number`
                // is a variable, not TeX's \number)
                let mut picture = t.is_cs("tikz");
                out.push(t);
                if !picture {
                    let mut name = Vec::new();
                    if let Some(open) = self.tex.get_token_until(depth) {
                        if matches!(open, Token::Char(_, tex::Cat::Begin)) {
                            while let Some(x) = self.tex.get_token_until(depth) {
                                let end = matches!(x, Token::Char(_, tex::Cat::End));
                                name.push(x);
                                if end {
                                    break;
                                }
                            }
                        }
                        picture = tex::tokens_to_string(&name) == "tikzpicture}";
                        out.push(open);
                        out.extend(name);
                    }
                }
                if picture {
                    while let Some(x) = self.tex.get_token_until(depth) {
                        out.push(x);
                    }
                    return end(out);
                }
                continue;
            }
            let r = match self.tex.meaning(&t) {
                tex::Meaning::Native(id) => self.command(id, &t),
                // a register given a value (`\count=3`, `\count 3`); else it is an argument, as
                // `\linewidth` of a `\parbox` that is written as it is
                tex::Meaning::Count(_) | tex::Meaning::Dimen(_) | tex::Meaning::Skip(_) | tex::Meaning::Toks(_) => {
                    let next = self.tex.next_nonspace();
                    let value = next.as_ref().is_some_and(|n| matches!(n, Token::Char(c, _) if c.is_ascii_digit() || matches!(c, '=' | '-' | '+' | '.')));
                    if let Some(n) = next {
                        self.tex.back(n);
                    }
                    if value {
                        self.tex.execute_basic(t).map(|_| ())
                    } else {
                        out.push(t);
                        Ok(())
                    }
                }
                _ => self.tex.execute_basic(t).map(|_| ()),
            };
            if let Err(e) = r {
                self.tex.drain_to(depth);
                return Err(e);
            }
        }
    }

    /// A shape's own text, expanded, with `\char`\&` as its character and a leading `\color{…}` taken
    /// out as its color.
    pub(super) fn shape_text(&mut self, tex: &str) -> (String, Option<String>) {
        let mut text = self.tex.expand_to_string(tikra_tokens(tex)).unwrap_or_else(|_| tex.to_string());
        while let Some(i) = text.find("\\char") {
            let rest = text[i + 5..].trim_start();
            let rest = rest.strip_prefix('`').unwrap_or(rest);
            let rest = rest.strip_prefix('\\').unwrap_or(rest);
            let Some(ch) = rest.chars().next() else { break };
            let after = rest[ch.len_utf8()..].to_string();
            let ch = if "&%$#_{}".contains(ch) { format!("\\{ch}") } else { ch.to_string() };
            text = format!("{}{ch}{after}", &text[..i]);
        }
        let mut color = None;
        if let Some(rest) = text.trim_start().strip_prefix("\\color") {
            let rest = rest.trim_start();
            if let Some(r) = rest.strip_prefix('{') {
                if let Some(j) = r.find('}') {
                    color = Some(r[..j].to_string());
                    text = r[j + 1..].to_string();
                }
            }
        }
        (text, color)
    }
}
/// `\textcolor{color}{text}` as the whole text, or `\color{color}` before it: (color, text).
pub(super) fn whole_textcolor(t: &str) -> Option<(String, String)> {
    let group = |s: &str| -> Option<(String, usize)> {
        let s2 = s.strip_prefix('{')?;
        let mut depth = 1;
        for (i, ch) in s2.char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((s2[..i].to_string(), i + 2));
                    }
                }
                _ => {}
            }
        }
        None
    };
    // `\color{…}` first: the rest of the text in that color
    if let Some(rest) = t.trim().strip_prefix("\\color").map(str::trim_start) {
        let (c, n) = group(rest)?;
        return Some((c, rest[n..].trim().to_string()));
    }
    let rest = t.trim().strip_prefix("\\textcolor")?.trim_start();
    let (c, n) = group(rest)?;
    let rest = rest[n..].trim_start();
    let (inner, m) = group(rest)?;
    rest[m..].trim().is_empty().then_some((c, inner))
}

impl Engine {
    /// `\textcolor{color}{text}` inside a node's text, its color resolved for the math typesetter
    /// (which colors only in math): `$\textcolor[HTML]{rrggbb}{\text{text}}$`.
    /// `\pgfdeclareimage[options]{name}{file}`: the image `\pgfuseimage{name}` puts in a text.
    pub(in super::super) fn declare_image(&mut self) -> Result<(), Error> {
        let options = self.optional()?.map(|o| tex::tokens_to_string(&o)).unwrap_or_default();
        let name = self.argument_string()?;
        let file = self.argument_string()?;
        let name = name.trim();
        self.tex.define_string(&format!("tikra@image@{name}@options"), &options, false);
        self.tex.define_string(&format!("tikra@image@{name}@file"), file.trim(), false);
        Ok(())
    }

    /// A text that is an image (`\pgfuseimage{name}`, `\pgfimage[options]{file}`,
    /// `\includegraphics[options]{file}`): what it draws from its box's lower left corner, and the
    /// box's corners. Only PNG images are drawn; one not drawn keeps the size its options give.
    pub(super) fn text_image(&mut self, text: &str) -> Result<Option<(Vec<Item>, Point, Point)>, Error> {
        let t = text.trim();
        let (options, file) = if let Some(r) = t.strip_prefix("\\pgfuseimage") {
            let name = r.trim().trim_start_matches('{').trim_end_matches('}').trim();
            let value = |e: &Engine, k: &str| match e.tex.meaning_of(&format!("tikra@image@{name}@{k}")) {
                tex::Meaning::Macro(m) => Some(tex::tokens_to_string(&tex::substitute(&m.body, &[]))),
                _ => None,
            };
            let Some(file) = value(self, "file") else {
                self.warn(format!("\\pgfuseimage: no image '{name}' declared"));
                return Ok(None);
            };
            (value(self, "options").unwrap_or_default(), file)
        } else if let Some(r) = t.strip_prefix("\\pgfimage").or_else(|| t.strip_prefix("\\includegraphics")) {
            let r = r.trim_start();
            let (options, r) = match r.strip_prefix('[').and_then(|r| r.split_once(']')) {
                Some((o, r)) => (o.to_string(), r.trim_start()),
                None => (String::new(), r),
            };
            let Some(file) = r.strip_prefix('{').and_then(|r| r.strip_suffix('}')).filter(|f| !f.contains(['{', '}'])) else { return Ok(None) };
            (options, file.trim().to_string())
        } else {
            return Ok(None);
        };
        let (mut width, mut height, mut scale) = (None, None, 1.0);
        for item in crate::keys::split_list(&tikra_tokens(&options)) {
            match (item.key.as_str(), item.value) {
                ("width", Some(v)) => width = Some(self.length_of(v)?),
                ("height", Some(v)) => height = Some(self.length_of(v)?),
                ("scale", Some(v)) => scale = self.number_of(v)?,
                ("interpolate" | "keepaspectratio", _) => {}
                (k, _) => self.warn(format!("image option '{k}' ignored")),
            }
        }
        // the `mwe` package's placeholder, which documents load as if it were a file of theirs
        if file.trim().trim_end_matches(".pdf") == "example-image-a" {
            let bp = 2.54 / 72.0;
            let (nw, nh) = (320.0 * bp, 240.0 * bp);
            let (w, h) = match (width, height) {
                (Some(w), Some(h)) => (w, h),
                (Some(w), None) => (w, w * nh / nw),
                (None, Some(h)) => (h * nw / nh, h),
                (None, None) => (nw * scale, nh * scale),
            };
            return Ok(Some((self.example_image(w, h), Point::default(), Point::new(w, h))));
        }
        let image = graphics::load_png(&file);
        let natural = image.as_ref().map(|(_, n)| *n).ok();
        let (w, h) = match (width, height, natural) {
            (Some(w), Some(h), _) => (w, h),
            (Some(w), None, Some((nw, nh))) => (w, w * nh / nw),
            (None, Some(h), Some((nw, nh))) => (h * nw / nh, h),
            (None, None, Some((nw, nh))) => (nw * scale, nh * scale),
            (w, h, None) => (w.unwrap_or(0.0), h.unwrap_or(0.0)),
        };
        let mut drawn = Vec::new();
        match image {
            Ok((png, _)) => drawn.push(Item::Image { png, place: Transform { a: w, b: 0.0, c: 0.0, d: h, e: 0.0, f: 0.0 } }),
            Err(m) => self.warn(format!("image not drawn: {m}")),
        }
        Ok(Some((drawn, Point::default(), Point::new(w, h))))
    }

    /// `example-image-a` of the `mwe` package, `w` by `h` from the origin: as its PDF draws it on
    /// its 320 by 240 bp, a 75% gray box with its diagonals and middle lines in 60% gray, a frame
    /// and a 160 bp Latin Modern Sans `A`.
    fn example_image(&mut self, w: f64, h: f64) -> Vec<Item> {
        // cm per bp of the image
        let (sx, sy) = (w / 320.0, h / 240.0);
        let p = |x: f64, y: f64| Point::new(x * sx, y * sy);
        let rect = |inset: f64| {
            vec![
                Segment::MoveTo(p(inset, inset)),
                Segment::LineTo(p(inset, 240.0 - inset)),
                Segment::LineTo(p(320.0 - inset, 240.0 - inset)),
                Segment::LineTo(p(320.0 - inset, inset)),
                Segment::Close,
            ]
        };
        let base = self.stroke();
        let stroke = |gray: f64, width: f64| Stroke { color: crate::color::Color::gray(gray), width: width * sx, dash: None, ..base.clone() };
        let lines = [(0.0, 0.0, 320.0, 240.0), (320.0, 0.0, 0.0, 240.0), (160.0, 0.0, 160.0, 240.0), (0.0, 120.0, 320.0, 120.0)];
        let mut items = vec![
            Item::Path { segments: rect(0.0), fill: Some(Fill { paint: Paint::Solid(crate::color::Color::gray(0.75)), opacity: 1.0, even_odd: false }), stroke: None },
            Item::Path {
                segments: lines.iter().flat_map(|&(a, b, c, d)| [Segment::MoveTo(p(a, b)), Segment::LineTo(p(c, d))]).collect(),
                fill: None,
                stroke: Some(stroke(0.6, 0.79701)),
            },
            // (its 1.59404 bp frame is clipped to the box: the inner half)
            Item::Path { segments: rect(0.39851), fill: None, stroke: Some(stroke(0.0, 0.79702)) },
        ];
        let mut warnings = Vec::new();
        let label = label::compose_label("A", "\\sffamily", None, 'l', crate::color::Color::BLACK, &mut warnings);
        // 160 bp, from the 10 pt the label is set in
        let k = 160.0 * sx / (10.0 * PT);
        items.push(Item::Text { label, place: Transform { a: k, b: 0.0, c: 0.0, d: k, e: 110.042 * sx, f: 63.623 * sy }, opacity: 1.0 });
        items
    }

    pub(super) fn inline_colors(&mut self, text: &str) -> String {
        let mut out = String::new();
        let mut rest = text;
        while let Some(i) = rest.find("\\textcolor{") {
            out.push_str(&rest[..i]);
            let after = &rest[i + "\\textcolor{".len()..];
            let Some(end) = after.find('}') else {
                out.push_str(&rest[i..]);
                return out;
            };
            let spec = &after[..end];
            let body_start = &after[end + 1..];
            // the braced text after the color
            let Some(body) = body_start.strip_prefix('{') else {
                out.push_str(&rest[i..i + "\\textcolor{".len() + end + 1]);
                rest = body_start;
                continue;
            };
            let mut depth = 1;
            let mut close = body.len();
            for (k, c) in body.char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            close = k;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let inner = &body[..close];
            match self.color(spec) {
                Some(c) => {
                    let hex = c.hex();
                    let hex = hex.trim_start_matches('#');
                    // in math, a color of math; math inside stays math; text goes in \text
                    if label::in_math(&out) {
                        out.push_str(&format!("\\textcolor[HTML]{{{hex}}}{{{inner}}}"));
                    } else if inner.contains('$') {
                        out.push_str(&format!("$\\textcolor[HTML]{{{hex}}}{{{}}}$", inner.replace('$', "")));
                    } else {
                        out.push_str(&format!("$\\textcolor[HTML]{{{hex}}}{{\\text{{{inner}}}}}$"));
                    }
                }
                None => out.push_str(inner),
            }
            rest = body.get(close + 1..).unwrap_or("");
        }
        out.push_str(rest);
        out
    }
}
