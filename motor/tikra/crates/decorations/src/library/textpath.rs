//! `text along path`: each character measured, then typeset on the path.

use super::*;
use tikra_engine::canvas::Item;
use tikra_engine::color::Color;

impl Run<'_> {
    pub(super) fn text_width(&self) -> f64 {
        // no more text: a box 16383pt wide ends the decoration
        self.text.current.as_ref().map_or(16383.0, |c| c.2)
    }

    pub(super) fn text_initial(&mut self) -> Result<(), Error> {
        let left = self.key_pt("text align/left indent", 0.0);
        let right = self.key_pt("text align/right indent", 0.0);
        let align = self.key_text("text align/align");
        self.w.remaining = self.w.section - right;
        self.w.section = self.w.remaining;
        // the width of the whole text, character by character
        let mut probe = TextState { rest: self.s.text.iter().cloned().collect(), ..Default::default() };
        let (mut width, mut chars, mut spaces) = (0.0, 0i64, 0i64);
        loop {
            self.scan_char(&mut probe)?;
            let Some(c) = &probe.current else { break };
            width += c.2;
            chars += 1;
            if probe.is_space {
                spaces += 1;
            }
            if chars > 100_000 {
                break;
            }
        }
        let mut tx = TextState { rest: self.s.text.iter().cloned().collect(), indent_left: left, fit: self.s.text_fit, stretch: self.s.text_stretch, ..Default::default() };
        let mut y = self.w.remaining;
        if tx.fit {
            y -= left + width;
            if tx.stretch {
                y /= spaces.max(1) as f64;
                tx.space_shift = y;
            } else {
                y /= (chars - 1).max(1) as f64;
                tx.char_shift = y;
            }
            if y < 0.0 {
                tx.fit = false;
                tx.stretch = false;
                tx.char_shift = 0.0;
                tx.space_shift = 0.0;
            }
        } else if align == "right" {
            tx.indent_left = y - width;
        } else if align == "center" {
            tx.indent_left = 0.5 * (y - width - left) + left;
        }
        self.text = tx;
        Ok(())
    }

    pub(super) fn text_scan(&mut self) -> Result<(), Error> {
        let mut tx = std::mem::take(&mut self.text);
        let r = self.scan_char(&mut tx);
        self.text = tx;
        r
    }

    /// `\pgf@lib@dec@text@scanchar`: the next character's box.
    fn scan_char(&mut self, tx: &mut TextState) -> Result<(), Error> {
        tx.is_space = false;
        tx.current = None;
        let mut guard = 0;
        while let Some(t) = tx.rest.pop_front() {
            guard += 1;
            if guard > 10_000 {
                break;
            }
            match &t {
                Token::Char(_, tikra_engine::tex::Cat::Space) => {
                    tx.is_space = true;
                    tx.current = Some((" ".into(), tx.format.clone(), self.char_width(" ", &tx.format, tx.math)));
                    return Ok(());
                }
                Token::Char(_, tikra_engine::tex::Cat::Begin) => {
                    let mut depth = 1;
                    let mut inner = Vec::new();
                    while let Some(u) = tx.rest.pop_front() {
                        match u {
                            Token::Char(_, tikra_engine::tex::Cat::Begin) => depth += 1,
                            Token::Char(_, tikra_engine::tex::Cat::End) => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        inner.push(u);
                    }
                    let s = tikra_engine::tex::tokens_to_string(&inner);
                    let w = self.char_width(&s, &tx.format, tx.math);
                    tx.current = Some((s, tx.format.clone(), w));
                    return Ok(());
                }
                Token::Char('$', _) => tx.math = !tx.math,
                Token::Char(c, _) if *c == self.s.text_format.0 => {
                    // |format| or |+format| up to the closing delimiter
                    let close = self.s.text_format.1.unwrap_or(self.s.text_format.0);
                    let mut f = Vec::new();
                    while let Some(u) = tx.rest.pop_front() {
                        if matches!(u, Token::Char(c, _) if c == close) {
                            break;
                        }
                        f.push(u);
                    }
                    let s = tikra_engine::tex::tokens_to_string(&f);
                    match s.strip_prefix('+') {
                        Some(add) => tx.format.push_str(add),
                        None => tx.format = s,
                    }
                }
                Token::Char(c, _) => {
                    let s = c.to_string();
                    let w = self.char_width(&s, &tx.format, tx.math);
                    tx.current = Some((s, tx.format.clone(), w));
                    return Ok(());
                }
                Token::Cs(name) if &**name == " " || &**name == "space" => {
                    tx.current = Some((" ".into(), tx.format.clone(), self.char_width(" ", &tx.format, tx.math)));
                    return Ok(());
                }
                Token::Cs(_) => {
                    let m = self.e.tex.meaning(&t);
                    if m.is_expandable() {
                        // expanded once, then scanned again
                        let depth = self.e.tex.depth();
                        self.e.tex.push_frame(vec![t.clone()]);
                        let mut out = Vec::new();
                        if let Some(x) = self.e.tex.get_token_until(depth) {
                            let m = self.e.tex.meaning(&x);
                            self.e.tex.expand(&x, &m)?;
                        }
                        while let Some(x) = self.e.tex.get_token_until(depth) {
                            out.push(x);
                        }
                        self.e.tex.drain_to(depth);
                        for x in out.into_iter().rev() {
                            tx.rest.push_front(x);
                        }
                    } else {
                        let s = tikra_engine::tex::show_token(&t);
                        let w = self.char_width(&s, &tx.format, tx.math);
                        tx.current = Some((s, tx.format.clone(), w));
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }

    /// The label of one character, with the format's color (`\color{…}`) taken out.
    fn char_label(&mut self, text: &str, format: &str, math: bool) -> tikra_engine::scene::Label {
        let (font, color) = split_color(format);
        let color = color.and_then(|c| self.e.color(&c)).or_else(|| self.e.color(&self.s.text_color.clone())).unwrap_or(self.e.st.paint.ink.unwrap_or(Color::BLACK));
        let body = if math { format!("${text}$") } else { text.to_string() };
        let mut warnings = Vec::new();
        let label = tikra_engine::label::compose_label(&body, &font, None, 'l', color, &mut warnings);
        for w in warnings {
            self.e.warn(w);
        }
        label
    }

    fn char_width(&mut self, text: &str, format: &str, math: bool) -> f64 {
        if text == " " {
            // an interword space of the 10pt font
            return 3.33333 * tikra_engine::label::font_size(format);
        }
        self.char_label(text, format, math).width() / PT
    }

    pub(super) fn text_typeset(&mut self, t: &Transform) {
        let Some((s, format, w)) = self.text.current.clone() else {
            return;
        };
        if s == " " {
            return;
        }
        let math = self.text.math;
        let label = self.char_label(&s, &format, math);
        let place = to_cm(&shifted(t, -0.5 * w, 0.0));
        // as \pgfqboxsynced, the box does not count in the picture's size
        self.e.canvas.push(Item::Text { label, place, opacity: self.e.st.paint.text_opacity });
    }
}
