//! Commutative diagrams with arrows between any two cells: tikz-cd (`\begin{tikzcd}`) and xymatrix
//! (`\xymatrix{…}`). Both become the same [`Node::ArrowDiagram`]: a grid of cells and a list of arrows.

use super::*;

/// Options of a tikz-cd arrow that only tune the drawing (positions, anchors, fonts): read and ignored.
const LAYOUT_KEYS: &[&str] = &[
    "crossing over",
    "near start",
    "near end",
    "very near start",
    "very near end",
    "at start",
    "at end",
    "pos",
    "sloped",
    "inner sep",
    "outer sep",
    "font",
    "start anchor",
    "end anchor",
    "shorten",
    "shorten <",
    "shorten >",
    "looseness",
    "labels",
    "marking",
    "thick",
    "very thick",
    "thin",
    "line width",
    "color",
    "draw",
    "anchor",
    "allow upside down",
    "outer xsep",
    "outer ysep",
    "yshift",
    "xshift",
];

/// tikz-cd's label positions, as a fraction of the arrow.
fn position(key: &str) -> Option<f32> {
    match key {
        "at start" => Some(0.0),
        "very near start" => Some(0.125),
        "near start" => Some(0.25),
        "midway" => Some(0.5),
        "near end" => Some(0.75),
        "very near end" => Some(0.875),
        "at end" => Some(1.0),
        _ => key.strip_prefix("pos=").and_then(|v| v.trim().parse::<f32>().ok()).map(|v| v.clamp(0.0, 1.0)),
    }
}

/// Cell offset of a direction word: `r`, `dr`, `uul`…
fn step(word: &str) -> Option<(isize, isize)> {
    let (mut di, mut dj) = (0isize, 0isize);
    for c in word.chars() {
        match c {
            'r' => dj += 1,
            'l' => dj -= 1,
            'd' => di += 1,
            'u' => di -= 1,
            _ => return None,
        }
    }
    Some((di, dj))
}

/// Splits at `sep` outside braces, brackets and quotes.
fn split_top(s: &str, sep: char) -> Vec<String> {
    let mut parts = vec![String::new()];
    let (mut level, mut quoted) = (0i32, false);
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                parts.last_mut().unwrap().push(c);
                if let Some(d) = chars.next() {
                    parts.last_mut().unwrap().push(d);
                }
                continue;
            }
            '{' | '[' => level += 1,
            '}' | ']' => level -= 1,
            '"' if level == 0 => quoted = !quoted,
            x if x == sep && level == 0 && !quoted => {
                parts.push(String::new());
                continue;
            }
            _ => {}
        }
        parts.last_mut().unwrap().push(c);
    }
    parts
}

/// Rows at `\\` and cells at `&`, outside braces.
fn grid(raw: &str) -> Vec<Vec<String>> {
    let c: Vec<char> = raw.chars().collect();
    let mut rows: Vec<Vec<String>> = vec![vec![String::new()]];
    let mut level = 0i32;
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            '\\' if level == 0 && c.get(i + 1) == Some(&'\\') => {
                rows.push(vec![String::new()]);
                i += 2;
                continue;
            }
            '\\' => {
                let cell = rows.last_mut().unwrap().last_mut().unwrap();
                cell.push('\\');
                if let Some(d) = c.get(i + 1) {
                    cell.push(*d);
                }
                i += 2;
                continue;
            }
            '{' => level += 1,
            '}' => level -= 1,
            '&' if level == 0 => {
                rows.last_mut().unwrap().push(String::new());
                i += 1;
                continue;
            }
            _ => {}
        }
        rows.last_mut().unwrap().last_mut().unwrap().push(c[i]);
        i += 1;
    }
    // a trailing \\ leaves an empty last row
    while rows.len() > 1 && rows.last().is_some_and(|r| r.iter().all(|c| c.trim().is_empty())) {
        rows.pop();
    }
    rows
}

/// A cell with its arrows taken out: the remaining text and the raw arrow specs.
struct Pieces {
    text: String,
    arrows: Vec<String>,
}

/// Takes `\arrow[…]` and `\ar[…]` (tikz-cd) out of a cell.
fn tikzcd_pieces(cell: &str) -> Pieces {
    let c: Vec<char> = cell.chars().collect();
    let mut text = String::new();
    let mut arrows = Vec::new();
    let mut i = 0;
    while i < c.len() {
        if c[i] == '\\' {
            let start = i;
            i += 1;
            let mut name = String::new();
            while i < c.len() && c[i].is_ascii_alphabetic() {
                name.push(c[i]);
                i += 1;
            }
            if name == "arrow" || name == "ar" {
                while i < c.len() && c[i].is_whitespace() {
                    i += 1;
                }
                let mut spec = String::new();
                if c.get(i) == Some(&'[') {
                    let (mut level, mut quoted) = (0i32, false);
                    i += 1;
                    while i < c.len() {
                        let ch = c[i];
                        i += 1;
                        match ch {
                            '\\' => {
                                spec.push(ch);
                                if let Some(d) = c.get(i) {
                                    spec.push(*d);
                                    i += 1;
                                }
                                continue;
                            }
                            '"' if level == 0 => quoted = !quoted,
                            '{' => level += 1,
                            '}' => level -= 1,
                            ']' if level == 0 && !quoted => break,
                            _ => {}
                        }
                        spec.push(ch);
                    }
                }
                arrows.push(spec);
                continue;
            }
            text.extend(&c[start..i]);
            if name.is_empty() {
                if let Some(d) = c.get(i) {
                    text.push(*d);
                    i += 1;
                }
            }
            continue;
        }
        text.push(c[i]);
        i += 1;
    }
    Pieces { text, arrows }
}

impl Reader<'_> {
    /// `\begin{tikzcd}[options] A \arrow[r, "f"] & B \end{tikzcd}`; the environment's options are ignored.
    pub(super) fn tikzcd(&mut self, raw: &str, col: usize) -> R<Node> {
        let mut body = raw.trim_start();
        if body.starts_with('[') {
            let mut level = 0;
            for (k, ch) in body.char_indices() {
                match ch {
                    '[' | '{' => level += 1,
                    ']' | '}' => {
                        level -= 1;
                        if level == 0 {
                            body = &body[k + 1..];
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
        let rows = grid(body);
        let mut cells = Vec::new();
        let mut arrows = Vec::new();
        for (i, row) in rows.iter().enumerate() {
            let mut out = Vec::new();
            for (j, cell) in row.iter().enumerate() {
                let pieces = tikzcd_pieces(cell);
                out.push(self.sub_reader(&pieces.text, Variant::Italic)?);
                for spec in &pieces.arrows {
                    if let Some(a) = self.tikzcd_arrow(spec, (i, j), col)? {
                        arrows.push(a);
                    }
                }
            }
            cells.push(out);
        }
        self.finish_diagram(cells, arrows, col)
    }

    /// One `\arrow[…]` of tikz-cd, leaving cell `from`.
    fn tikzcd_arrow(&mut self, spec: &str, from: (usize, usize), col: usize) -> R<Option<DiagramArrow>> {
        let mut a = blank_arrow(from);
        let mut target: Option<(isize, isize)> = None;
        let mut origin = from;
        // a position given as an arrow option applies to the labels without their own
        let mut arrow_position = None;
        for item in split_top(spec, ',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            if let Some(rest) = item.strip_prefix('"') {
                // "label" followed by its own keys: ' (swap), description, near start…
                let Some(end) = quote_end(rest) else {
                    self.tolerate(rules::E_REC_4, col, format!("label without its closing quote: {item}"), "read up to the end")?;
                    let node = self.sub_reader(rest, Variant::Italic)?;
                    a.labels.push(ArrowLabel { node, side: LabelSide::Left, position: f32::NAN });
                    continue;
                };
                let (text, keys) = (&rest[..end], rest[end + 1..].trim());
                let text = text.strip_prefix('{').and_then(|t| t.strip_suffix('}')).unwrap_or(text);
                let mut side = LabelSide::Left;
                let mut at = None;
                let words: Vec<&str> = keys.trim_start_matches('\'').split_whitespace().collect();
                for key in &words {
                    match *key {
                        "'" | "swap" => side = LabelSide::Right,
                        "description" => side = LabelSide::On,
                        _ => {}
                    }
                }
                for n in (1..=3).rev() {
                    for w in words.windows(n) {
                        at = at.or_else(|| position(&w.join(" ")));
                    }
                }
                if keys.starts_with('\'') {
                    side = LabelSide::Right;
                }
                let node = self.sub_reader(text, Variant::Italic)?;
                a.labels.push(ArrowLabel { node, side, position: at.unwrap_or(f32::NAN) });
                continue;
            }
            let (key, value) = match item.split_once('=') {
                Some((k, v)) => (k.trim(), Some(v.trim())),
                None => (item, None),
            };
            if let (Some(s), None) = (step(key), value) {
                target = Some(s);
                continue;
            }
            match key {
                "to" | "from" => {
                    let cell = value.and_then(|v| v.split_once('-')).and_then(|(r, c)| Some((r.trim().parse::<usize>().ok()?, c.trim().parse::<usize>().ok()?)));
                    match cell {
                        Some((r, c)) if r > 0 && c > 0 => {
                            if key == "to" {
                                target = Some((r as isize - 1 - from.0 as isize, c as isize - 1 - from.1 as isize));
                            } else {
                                origin = (r - 1, c - 1);
                            }
                        }
                        _ => self.tolerate(rules::E_REC_1, col, format!("arrow cell not in row-column form: {item}"), "ignored")?,
                    }
                }
                "dashed" | "dash pattern" | "densely dashed" | "loosely dashed" => a.line = ArrowLine::Dashed,
                "dotted" | "densely dotted" | "loosely dotted" => a.line = ArrowLine::Dotted,
                "squiggly" | "leadsto" | "rightsquigarrow" => a.line = ArrowLine::Squiggly,
                "equal" | "equals" | "Equal" => {
                    a.line = ArrowLine::Double;
                    a.head = ArrowHead::None;
                }
                "Rightarrow" | "Longrightarrow" => a.line = ArrowLine::Double,
                "Leftarrow" | "Longleftarrow" => {
                    a.line = ArrowLine::Double;
                    a.head = ArrowHead::None;
                    a.tail = ArrowTail::Head;
                }
                "Leftrightarrow" | "Longleftrightarrow" => {
                    a.line = ArrowLine::Double;
                    a.tail = ArrowTail::Head;
                }
                "leftrightarrow" | "longleftrightarrow" => a.tail = ArrowTail::Head,
                "leftarrow" | "longleftarrow" => {
                    a.head = ArrowHead::None;
                    a.tail = ArrowTail::Head;
                }
                "rightarrow" | "longrightarrow" | "to head" => a.head = ArrowHead::Normal,
                "two heads" | "twoheadrightarrow" => a.head = ArrowHead::TwoHeads,
                "hook" | "hookrightarrow" => a.tail = ArrowTail::Hook,
                "hook'" => a.tail = ArrowTail::HookRight,
                "tail" | "rightarrowtail" | "tail'" => a.tail = ArrowTail::Tail,
                "mapsto" | "maps to" | "mapsfrom" => a.tail = ArrowTail::Bar,
                "no head" | "dash" | "-" | "no tail" => {
                    if key != "no tail" {
                        a.head = ArrowHead::None;
                    }
                }
                "phantom" => {
                    a.line = ArrowLine::None;
                    a.head = ArrowHead::None;
                    for l in &mut a.labels {
                        l.side = LabelSide::On;
                    }
                }
                "bend left" | "bend right" | "curve" => {
                    let degrees = value.and_then(|v| v.parse::<i16>().ok()).unwrap_or(30).clamp(-89, 89);
                    a.bend = if key == "bend right" { -degrees } else { degrees };
                }
                "shift left" | "shift right" => {
                    let n = value.and_then(|v| v.parse::<f32>().ok()).unwrap_or(1.0);
                    // tikz-cd shifts by n × 0.56 ex; an ex is about 0.43 em
                    let em = n.clamp(-10.0, 10.0) * 0.24;
                    a.shift = if key == "shift right" { -em } else { em };
                }
                "swap" => a.labels.iter_mut().for_each(|l| l.side = LabelSide::Right),
                k if position(item).is_some() || position(k).is_some() => arrow_position = position(item).or(position(k)),
                k if LAYOUT_KEYS.contains(&k) || is_color(k) => {}
                other => self.tolerate(rules::E_REC_1, col, format!("unsupported arrow option: {other}"), "ignored")?,
            }
        }
        if a.line == ArrowLine::None {
            for l in &mut a.labels {
                l.side = LabelSide::On;
            }
        }
        for l in &mut a.labels {
            if l.position.is_nan() {
                l.position = arrow_position.unwrap_or(0.5);
            }
        }
        a.from = origin;
        let Some((di, dj)) = target else {
            self.tolerate(rules::E_REC_7, col, format!("arrow without a direction: \\arrow[{spec}]"), "left out")?;
            return Ok(None);
        };
        match destination(origin, di, dj) {
            Some(to) => Ok(Some(DiagramArrow { to, ..a })),
            None => {
                self.tolerate(rules::E_REC_7, col, format!("arrow pointing outside the diagram: \\arrow[{spec}]"), "left out")?;
                Ok(None)
            }
        }
    }

    /// `\xymatrix@C=2em{ A \ar[r]^f & B }`: the options before the brace are ignored.
    pub(super) fn xymatrix(&mut self, col: usize) -> R<Node> {
        self.skip_spaces();
        while self.current() == Some('@') {
            while self.current().is_some_and(|c| c != '{') {
                self.pos += 1;
            }
        }
        let raw = self.raw_text()?;
        let rows = grid(&raw);
        let mut cells = Vec::new();
        let mut arrows = Vec::new();
        for (i, row) in rows.iter().enumerate() {
            let mut out = Vec::new();
            for (j, cell) in row.iter().enumerate() {
                let (text, found) = self.xy_pieces(cell, (i, j), col)?;
                out.push(self.sub_reader(&text, Variant::Italic)?);
                arrows.extend(found);
            }
            cells.push(out);
        }
        self.finish_diagram(cells, arrows, col)
    }

    /// Takes the `\ar…` arrows out of an xymatrix cell; returns the rest of the cell.
    fn xy_pieces(&mut self, cell: &str, from: (usize, usize), col: usize) -> R<(String, Vec<DiagramArrow>)> {
        let c: Vec<char> = cell.chars().collect();
        let mut text = String::new();
        let mut arrows = Vec::new();
        let mut i = 0;
        while i < c.len() {
            if c[i] != '\\' {
                text.push(c[i]);
                i += 1;
                continue;
            }
            let start = i;
            i += 1;
            let mut name = String::new();
            while i < c.len() && c[i].is_ascii_alphabetic() {
                name.push(c[i]);
                i += 1;
            }
            if name != "ar" {
                text.extend(&c[start..i]);
                if name.is_empty() && i < c.len() {
                    text.push(c[i]);
                    i += 1;
                }
                continue;
            }
            let mut a = blank_arrow(from);
            let mut target = None;
            loop {
                while i < c.len() && c[i].is_whitespace() {
                    i += 1;
                }
                match c.get(i) {
                    Some('@') => {
                        i += 1;
                        match c.get(i) {
                            Some('{') => {
                                let style = group(&c, &mut i, '{', '}');
                                xy_style(&style, &mut a);
                            }
                            Some('/') => {
                                i += 1;
                                let mut curve = String::new();
                                while i < c.len() && c[i] != '/' {
                                    curve.push(c[i]);
                                    i += 1;
                                }
                                i += 1;
                                let size = curve.trim_start_matches(['^', '_']).trim();
                                let degrees = if size.is_empty() { 30 } else { 40 };
                                a.bend = if curve.trim_start().starts_with('_') { -degrees } else { degrees };
                            }
                            Some('<') => {
                                let amount = group(&c, &mut i, '<', '>');
                                let pt: f32 = amount.trim_end_matches("pt").trim().parse().unwrap_or(2.0);
                                // positive moves to the left of the direction, like xy-pic
                                a.shift = (pt / 10.0).clamp(-5.0, 5.0);
                            }
                            Some(d) if d.is_ascii_alphabetic() || *d == '-' || *d == '=' || *d == '.' || *d == '~' => {
                                // @-> and friends without braces: one style character run
                                let mut style = String::new();
                                while i < c.len() && !c[i].is_whitespace() && !matches!(c[i], '[' | '^' | '_' | '|' | '"' | '\\') {
                                    style.push(c[i]);
                                    i += 1;
                                }
                                xy_style(&style, &mut a);
                            }
                            _ => {}
                        }
                    }
                    Some('[') => {
                        let word = group(&c, &mut i, '[', ']');
                        match step(word.trim()) {
                            Some(s) => target = Some(s),
                            None => self.tolerate(rules::E_REC_1, col, format!("unsupported xymatrix direction: [{word}]"), "arrow left out")?,
                        }
                    }
                    Some('"') => {
                        i += 1;
                        let mut cell = String::new();
                        while i < c.len() && c[i] != '"' {
                            cell.push(c[i]);
                            i += 1;
                        }
                        i += 1;
                        match cell.split_once(',').and_then(|(r, k)| Some((r.trim().parse::<usize>().ok()?, k.trim().parse::<usize>().ok()?))) {
                            Some((r, k)) if r > 0 && k > 0 => target = Some((r as isize - 1 - from.0 as isize, k as isize - 1 - from.1 as isize)),
                            _ => self.tolerate(rules::E_REC_1, col, format!("unsupported xymatrix target: \"{cell}\""), "arrow left out")?,
                        }
                    }
                    Some(m @ ('^' | '_' | '|')) => {
                        let side = match m {
                            '^' => LabelSide::Left,
                            '_' => LabelSide::Right,
                            _ => LabelSide::On,
                        };
                        i += 1;
                        // position modifiers: ^< near the start, ^> near the end, ^(0.3) a fraction
                        let mut at = 0.5f32;
                        while i < c.len() && matches!(c[i], '<' | '>') {
                            at = if c[i] == '<' { at.min(0.5) - 0.25 } else { at.max(0.5) + 0.25 };
                            i += 1;
                        }
                        if c.get(i) == Some(&'(') {
                            at = group(&c, &mut i, '(', ')').trim().parse().unwrap_or(at);
                        }
                        let at = at.clamp(0.0, 1.0);
                        while i < c.len() && c[i].is_whitespace() {
                            i += 1;
                        }
                        let label = match c.get(i) {
                            Some('{') => group(&c, &mut i, '{', '}'),
                            Some('\\') => {
                                let s = i;
                                i += 1;
                                while i < c.len() && c[i].is_ascii_alphabetic() {
                                    i += 1;
                                }
                                if i == s + 1 && i < c.len() {
                                    i += 1;
                                }
                                c[s..i].iter().collect()
                            }
                            Some(x) => {
                                i += 1;
                                x.to_string()
                            }
                            None => String::new(),
                        };
                        let node = self.sub_reader(&label, Variant::Italic)?;
                        a.labels.push(ArrowLabel { node, side, position: at });
                    }
                    _ => break,
                }
            }
            if a.line == ArrowLine::None {
                for l in &mut a.labels {
                    l.side = LabelSide::On;
                }
            }
            match target.and_then(|(di, dj)| destination(from, di, dj)) {
                Some(to) => arrows.push(DiagramArrow { to, ..a }),
                None if target.is_none() => self.tolerate(rules::E_REC_7, col, "\\ar without a direction", "left out")?,
                None => self.tolerate(rules::E_REC_7, col, "\\ar pointing outside the diagram", "left out")?,
            }
        }
        Ok((text, arrows))
    }

    /// Pads the rows to the same length and drops arrows that leave the grid.
    fn finish_diagram(&mut self, mut cells: Vec<Vec<Node>>, arrows: Vec<DiagramArrow>, col: usize) -> R<Node> {
        let ncol = cells.iter().map(Vec::len).max().unwrap_or(0);
        for row in &mut cells {
            row.resize(ncol, Node::row(vec![]));
        }
        let mut kept = Vec::new();
        for a in arrows {
            let inside = |(r, c): (usize, usize)| r < cells.len() && c < ncol;
            if !inside(a.from) || !inside(a.to) {
                self.tolerate(rules::E_REC_7, col, format!("arrow to a cell outside the diagram: row {}, column {}", a.to.0 + 1, a.to.1 + 1), "left out")?;
            } else if a.from == a.to {
                self.tolerate(rules::E_REC_7, col, "arrow from a cell to itself", "left out")?;
            } else {
                kept.push(a);
            }
        }
        Ok(Node::ArrowDiagram { cells, arrows: kept })
    }
}

fn blank_arrow(from: (usize, usize)) -> DiagramArrow {
    DiagramArrow { from, to: from, labels: vec![], line: ArrowLine::Solid, head: ArrowHead::Normal, tail: ArrowTail::None, bend: 0, shift: 0.0 }
}

fn destination(from: (usize, usize), di: isize, dj: isize) -> Option<(usize, usize)> {
    let r = from.0 as isize + di;
    let c = from.1 as isize + dj;
    (r >= 0 && c >= 0).then_some((r as usize, c as usize))
}

/// End of a quoted label whose text holds braces with quotes inside: `"{a"b}"`.
fn quote_end(rest: &str) -> Option<usize> {
    let mut level = 0i32;
    for (k, ch) in rest.char_indices() {
        match ch {
            '{' => level += 1,
            '}' => level -= 1,
            '"' if level == 0 => return Some(k),
            _ => {}
        }
    }
    None
}

/// Content between `open` and its matching `close`, starting at `c[*i] == open`; leaves `i` after it.
fn group(c: &[char], i: &mut usize, open: char, close: char) -> String {
    let mut level = 0i32;
    let mut s = String::new();
    while *i < c.len() {
        let ch = c[*i];
        *i += 1;
        if ch == open {
            level += 1;
            if level == 1 {
                continue;
            }
        } else if ch == close {
            level -= 1;
            if level == 0 {
                return s;
            }
        }
        s.push(ch);
    }
    s
}

/// xy-pic arrow styles: `@{-->}`, `@{.>}`, `@{=>}`, `@{^{(}->}`, `@{|->}`, `@{->>}`, `@{-}`, `@{~>}`, `@{}`.
fn xy_style(style: &str, a: &mut DiagramArrow) {
    let s = style.trim();
    if s.is_empty() {
        a.line = ArrowLine::None;
        a.head = ArrowHead::None;
        return;
    }
    let (tail, rest) = if let Some(r) = s.strip_prefix("^{(}") {
        (ArrowTail::Hook, r)
    } else if let Some(r) = s.strip_prefix("_{(}") {
        (ArrowTail::HookRight, r)
    } else if let Some(r) = s.strip_prefix('|') {
        (ArrowTail::Bar, r)
    } else if let Some(r) = s.strip_prefix('<') {
        (ArrowTail::Head, r)
    } else if let Some(r) = s.strip_prefix('>') {
        (ArrowTail::Tail, r)
    } else {
        (ArrowTail::None, s)
    };
    a.tail = tail;
    let (body, head) = if let Some(b) = rest.strip_suffix(">>") {
        (b, ArrowHead::TwoHeads)
    } else if let Some(b) = rest.strip_suffix('>') {
        (b, ArrowHead::Normal)
    } else {
        (rest, ArrowHead::None)
    };
    a.head = head;
    a.line = match body {
        "--" => ArrowLine::Dashed,
        "." | ".." => ArrowLine::Dotted,
        "=" | "==" => ArrowLine::Double,
        "~" | "~~" => ArrowLine::Squiggly,
        _ => ArrowLine::Solid,
    };
}

fn is_color(k: &str) -> bool {
    matches!(k, "red" | "blue" | "green" | "black" | "gray" | "orange" | "purple" | "teal" | "violet" | "cyan" | "magenta" | "brown" | "white" | "yellow")
}
