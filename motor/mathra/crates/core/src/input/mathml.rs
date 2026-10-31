//! Reader for presentation MathML, the markup Word, Google Docs, MathType, LibreOffice and MathJax export.
//! It builds the same tree of meaning as the LaTeX reader, so layout, speech and text are shared.
//! Content MathML (`<apply>`, `<ci>`) is meaning without a drawing: it is outside this reader.

use roxmltree::{Document, Node as Xml, ParsingOptions};

use crate::locale::Locale;
use crate::rules::{self, Registry};
use crate::tree::*;
use crate::Error;

use super::latex::NESTING_LIMIT;
use super::symbols;

type R<T> = Result<T, Error>;

/// Does this text start with a `<math>` element (with or without a namespace prefix or an XML prolog)?
pub fn looks_like_mathml(source: &str) -> bool {
    let mut s = source.trim_start();
    // XML prolog and comments before the element
    loop {
        if let Some(rest) = s.strip_prefix("<?") {
            s = rest.split_once("?>").map_or("", |(_, r)| r).trim_start();
        } else if let Some(rest) = s.strip_prefix("<!--") {
            s = rest.split_once("-->").map_or("", |(_, r)| r).trim_start();
        } else {
            break;
        }
    }
    let Some(tag) = s.strip_prefix('<') else {
        return false;
    };
    let name: String = tag.chars().take_while(|c| c.is_alphanumeric() || *c == ':' || *c == '-').collect();
    name == "math" || name.ends_with(":math")
}

pub fn read_with(source: &str, locale: Locale, registry: &mut Registry, strict: bool) -> R<Node> {
    let text = replace_entities(source, registry, strict)?;
    let options = ParsingOptions { allow_dtd: true, ..ParsingOptions::default() };
    let doc = Document::parse_with_options(&text, options).map_err(|e| {
        let pos = e.pos();
        Error::Syntax { column: pos.col as usize, message: format!("invalid MathML (line {}): {e}", pos.row) }
    })?;
    let mut reader = Reader { source: &text, locale, registry, strict, depth: 0 };
    let root = doc.root_element();
    let node = reader.element(root, Variant::Italic)?;
    Ok(match node {
        Node::Row { .. } => node,
        other => Node::row(vec![other]),
    })
}

struct Reader<'a> {
    source: &'a str,
    locale: Locale,
    registry: &'a mut Registry,
    strict: bool,
    depth: usize,
}

impl Reader<'_> {
    fn column(&self, node: Xml) -> usize {
        self.source[..node.range().start].chars().count() + 1
    }

    /// In strict mode, an error; in tolerant mode, a warning (E-REC-10), and the reading goes on.
    fn tolerate(&mut self, node: Xml, problem: String, recovery: &str) -> R<()> {
        let column = self.column(node);
        if self.strict {
            return Err(Error::Syntax { column, message: problem });
        }
        self.registry.warn(rules::E_REC_10, format!("column {column}: {problem}; {recovery}"));
        Ok(())
    }

    fn elements<'b>(node: Xml<'b, 'b>) -> Vec<Xml<'b, 'b>> {
        node.children().filter(Xml::is_element).collect()
    }

    /// The children of an element as a row (`<mrow>`, and every element with an inferred mrow).
    fn row(&mut self, node: Xml, variant: Variant) -> R<Node> {
        let mut children = Vec::new();
        let elements = Self::elements(node);
        for (i, child) in elements.iter().enumerate() {
            // <mi>rank</mi><mo>&ApplyFunction;</mo>: a function name, even one mathra does not know
            let applied = elements.get(i + 1).is_some_and(|n| n.tag_name().name() == "mo" && text_of(*n) == "\u{2061}");
            let node = match self.element(*child, variant)? {
                Node::Identifier { name, .. } if applied && name.chars().count() > 1 => Node::Function { name, limits: false },
                other => other,
            };
            match node {
                Node::Row { children: c } if c.is_empty() => {}
                other => children.push(other),
            }
        }
        Ok(fences(children))
    }

    /// Exactly `n` element children, as nodes. A missing one is left empty (E-REC-10).
    fn arguments(&mut self, node: Xml, n: usize, variant: Variant) -> R<Vec<Node>> {
        let children = Self::elements(node);
        if children.len() != n {
            let name = node.tag_name().name();
            self.tolerate(node, format!("<{name}> takes {n} children and has {}", children.len()), "the missing ones left empty, the extra ones ignored")?;
        }
        let mut out = Vec::new();
        for i in 0..n {
            out.push(match children.get(i) {
                Some(c) => self.element(*c, variant)?,
                None => Node::row(vec![]),
            });
        }
        Ok(out)
    }

    fn element(&mut self, node: Xml, variant: Variant) -> R<Node> {
        if self.depth >= NESTING_LIMIT {
            return Err(Error::Syntax { column: self.column(node), message: format!("nesting deeper than {NESTING_LIMIT} levels") });
        }
        self.depth += 1;
        let r = self.element_unguarded(node, variant);
        self.depth -= 1;
        r
    }

    fn element_unguarded(&mut self, node: Xml, inherited: Variant) -> R<Node> {
        let variant = node.attribute("mathvariant").and_then(parse_variant).unwrap_or(inherited);
        let name = node.tag_name().name();
        let node_out = match name {
            "math" | "mrow" | "mpadded" | "mtd" => self.row(node, variant)?,
            "mstyle" => {
                let base = self.row(node, variant)?;
                let base = match node.attribute("displaystyle") {
                    Some("true") => Node::Style { level: 0, base: Box::new(base) },
                    Some("false") => Node::Style { level: 1, base: Box::new(base) },
                    _ => base,
                };
                self.paint(node, base)
            }
            "semantics" => match Self::elements(node).into_iter().find(|c| !c.tag_name().name().starts_with("annotation")) {
                Some(first) => self.element(first, variant)?,
                None => Node::row(vec![]),
            },
            "annotation" | "annotation-xml" | "none" | "mprescripts" | "malignmark" | "maligngroup" => Node::row(vec![]),
            "maction" => {
                let children = Self::elements(node);
                let selection = node.attribute("selection").and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                match children.get(selection.saturating_sub(1)).or(children.first()) {
                    Some(c) => self.element(*c, variant)?,
                    None => Node::row(vec![]),
                }
            }
            "merror" => Node::Colored { color: "#c0392b".into(), base: Box::new(self.row(node, variant)?) },
            "mphantom" => Node::Phantom { base: Box::new(self.row(node, variant)?), width: true, height: true },
            "mi" => self.identifier(node, variant),
            "mn" => self.number(node),
            "mo" => operator(&text_of(node)),
            "mtext" => Node::Text { parts: vec![TextPart::Text(text_of(node))] },
            "ms" => {
                let (l, r) = (node.attribute("lquote").unwrap_or("\""), node.attribute("rquote").unwrap_or("\""));
                Node::Text { parts: vec![TextPart::Text(format!("{l}{}{r}", text_of(node)))] }
            }
            "mspace" => Node::Space { em: node.attribute("width").and_then(measure_em).unwrap_or(0.0) },
            "mfrac" => {
                let [num, den]: [Node; 2] = self.arguments(node, 2, variant)?.try_into().expect("two arguments");
                let rule = !node.attribute("linethickness").is_some_and(|t| t.trim().trim_end_matches(char::is_alphabetic).trim().parse::<f32>() == Ok(0.0));
                Node::Fraction { num: Box::new(num), den: Box::new(den), style: FractionStyle::Auto, rule, thickness: None }
            }
            "msqrt" => Node::Root { radicand: Box::new(self.row(node, variant)?), index: None },
            "mroot" => {
                let [radicand, index]: [Node; 2] = self.arguments(node, 2, variant)?.try_into().expect("two arguments");
                Node::Root { radicand: Box::new(radicand), index: Some(Box::new(index)) }
            }
            "msub" | "msup" => {
                let [base, script]: [Node; 2] = self.arguments(node, 2, variant)?.try_into().expect("two arguments");
                let (sup, sub) = if name == "msup" { (Some(script), None) } else { (None, Some(script)) };
                scripts(base, sup, sub)
            }
            "msubsup" => {
                let [base, sub, sup]: [Node; 3] = self.arguments(node, 3, variant)?.try_into().expect("three arguments");
                scripts(base, Some(sup), Some(sub))
            }
            "munder" | "mover" | "munderover" => self.under_over(node, variant)?,
            "mmultiscripts" => self.multiscripts(node, variant)?,
            "mfenced" => {
                let open = node.attribute("open").unwrap_or("(").trim().chars().next();
                let close = node.attribute("close").unwrap_or(")").trim().chars().next();
                let separators: Vec<char> = node.attribute("separators").unwrap_or(",").chars().filter(|c| !c.is_whitespace()).collect();
                let mut inner = Vec::new();
                for (i, child) in Self::elements(node).into_iter().enumerate() {
                    if i > 0 {
                        if let Some(&s) = separators.get(i - 1).or(separators.last()) {
                            inner.push(operator(&s.to_string()));
                        }
                    }
                    inner.push(self.element(child, variant)?);
                }
                let inner = Node::row(inner).simplify();
                if matches!(inner, Node::Table { .. }) {
                    // a table between the fences: a matrix or cases, as in an mrow
                    let fence = |c: Option<char>| c.map_or(Node::row(vec![]), |c| operator(&c.to_string()));
                    let children = [fence(open), inner, fence(close)].into_iter().filter(|n| *n != Node::row(vec![])).collect();
                    fences(children)
                } else {
                    Node::Delimited { open, close, parts: vec![inner], middles: vec![] }
                }
            }
            "menclose" => self.enclose(node, variant)?,
            "mtable" => self.table(node, variant)?,
            "mtr" | "mlabeledtr" => {
                // a row outside a table: its cells side by side
                self.row(node, variant)?
            }
            other => {
                let problem = if matches!(other, "apply" | "ci" | "cn" | "csymbol" | "bind" | "bvar") {
                    format!("content MathML <{other}> is not supported (only presentation MathML)")
                } else {
                    format!("unknown MathML element <{other}>")
                };
                self.tolerate(node, problem, "its children read as a row")?;
                self.row(node, variant)?
            }
        };
        // mstyle already painted itself, around its style
        Ok(if name == "mstyle" { node_out } else { self.paint(node, node_out) })
    }

    /// `mathcolor` and `mathbackground`, on any element that carries them.
    fn paint(&mut self, node: Xml, base: Node) -> Node {
        let valid = |c: &str| !c.is_empty() && c.len() < 40 && c.chars().all(|x| x.is_ascii_alphanumeric() || "#(),. ".contains(x));
        let mut out = base;
        if let Some(color) = node.attribute("mathcolor").or(node.attribute("color")).filter(|c| valid(c)) {
            out = Node::Colored { color: color.to_string(), base: Box::new(out) };
        }
        if let Some(background) = node.attribute("mathbackground").or(node.attribute("background")).filter(|c| valid(c)) {
            out = Node::Framed { base: Box::new(out), shape: Frame::Fill, background: Some(background.to_string()), border: None };
        }
        out
    }

    fn identifier(&mut self, node: Xml, variant: Variant) -> Node {
        let t = text_of(node);
        let explicit = node.attribute("mathvariant").and_then(parse_variant);
        let mut chars = t.chars();
        match (chars.next(), chars.next()) {
            (None, _) => Node::row(vec![]),
            (Some(c), None) => {
                // a single letter is italic, except uppercase Greek, as in TeX; mathvariant decides otherwise
                let natural = if c.is_ascii_alphabetic() || ('α'..='ω').contains(&c) || c == 'ı' || c == 'ȷ' { Variant::Italic } else { Variant::Upright };
                let variant = explicit.unwrap_or(if variant == Variant::Italic { natural } else { variant });
                if c.is_alphabetic() {
                    Node::Identifier { name: c.to_string(), variant }
                } else {
                    operator(&t)
                }
            }
            // several letters: a function name (sin, lim, and rank or Var, which MathJax writes as a bare
            // <mi>: MathML 3.2.3 reads a multi-letter mi as a function name); words come in <mtext>
            _ => match symbols::function(&t) {
                Some((canonical, limits)) => Node::Function { name: canonical.to_string(), limits },
                None if explicit.is_none_or(|v| v == Variant::Upright) && t.chars().all(char::is_alphabetic) => Node::Function { name: t, limits: false },
                None => Node::Identifier { name: t, variant: explicit.unwrap_or(if variant == Variant::Italic { Variant::Upright } else { variant }) },
            },
        }
    }

    fn number(&mut self, node: Xml) -> Node {
        let t: String = text_of(node).chars().filter(|c| !c.is_whitespace()).collect();
        match number_value(&t, self.locale) {
            Some((value, thousands)) => {
                if t.contains(',') && thousands.is_none() {
                    self.registry.note(rules::L_NUM_1, format!("'{t}': comma between digits read as decimal separator ({value})"));
                }
                Node::Number { value, group: false, thousands }
            }
            None => Node::Text { parts: vec![TextPart::Text(t)] },
        }
    }

    fn under_over(&mut self, node: Xml, variant: Variant) -> R<Node> {
        let name = node.tag_name().name();
        let n = if name == "munderover" { 3 } else { 2 };
        let mut args = self.arguments(node, n, variant)?.into_iter();
        let base = args.next().expect("base");
        let (under, over) = match name {
            "munder" => (args.next(), None),
            "mover" => (None, args.next()),
            _ => {
                let under = args.next();
                (under, args.next())
            }
        };
        let accent_attr = |a: &str| node.attribute(a) == Some("true");
        // a large operator or lim: limits; integrals keep TeX's side limits unless MathML stacks them
        if matches!(base, Node::LargeOperator { .. } | Node::Function { .. }) {
            let base = match base {
                Node::LargeOperator { symbol, .. } => Node::LargeOperator { symbol, limits: Some(true) },
                other => other,
            };
            return Ok(scripts(base, over, under));
        }
        // the label of a brace drawn in an inner munder/mover
        if let (Node::Brace { base: b, above, label: None, bracket }, Some(label)) =
            (&base, if matches!(base, Node::Brace { above: true, .. }) { over.clone() } else { under.clone() })
        {
            if under.is_none() || over.is_none() {
                return Ok(Node::Brace { base: b.clone(), above: *above, label: Some(Box::new(label)), bracket: *bracket });
            }
        }
        let mut out = base;
        let mut above_left = None;
        let mut below_left = None;
        let narrow = matches!(out, Node::Identifier { .. } | Node::Number { .. });
        if let Some(o) = over {
            match decoration(&o, true, accent_attr("accent") || narrow) {
                Some(d) => out = d.apply(out),
                None => above_left = Some(o),
            }
        }
        if let Some(u) = under {
            match decoration(&u, false, accent_attr("accentunder")) {
                Some(d) => out = d.apply(out),
                None => below_left = Some(u),
            }
        }
        if above_left.is_none() && below_left.is_none() {
            return Ok(out);
        }
        let class = match &out {
            Node::Operator { class, .. } => *class,
            _ => Class::Ord,
        };
        let stacked = Node::Stacked { base: Box::new(out), above: above_left.map(Box::new), below: below_left.map(Box::new) };
        Ok(if class == Class::Ord { stacked } else { Node::ForcedClass { class, base: Box::new(stacked) } })
    }

    fn multiscripts(&mut self, node: Xml, variant: Variant) -> R<Node> {
        let children = Self::elements(node);
        let Some(first) = children.first() else {
            return Ok(Node::row(vec![]));
        };
        let base = self.element(*first, variant)?;
        let mut post = Vec::new();
        let mut pre = Vec::new();
        let mut in_pre = false;
        for c in &children[1..] {
            if c.tag_name().name() == "mprescripts" {
                in_pre = true;
                continue;
            }
            let n = self.element(*c, variant)?;
            if in_pre {
                pre.push(n);
            } else {
                post.push(n);
            }
        }
        // pairs of (subscript, superscript); <none/> is an empty row
        let pair = |list: &[Node]| -> (Option<Node>, Option<Node>) {
            let join = |nodes: Vec<Node>| -> Option<Node> {
                let nodes: Vec<Node> = nodes.into_iter().filter(|n| *n != Node::row(vec![])).collect();
                if nodes.is_empty() {
                    None
                } else {
                    Some(Node::row(nodes).simplify())
                }
            };
            let subs = list.iter().step_by(2).cloned().collect();
            let sups = list.iter().skip(1).step_by(2).cloned().collect();
            (join(subs), join(sups))
        };
        let mut row = Vec::new();
        let (pre_sub, pre_sup) = pair(&pre);
        if pre_sub.is_some() || pre_sup.is_some() {
            row.push(scripts(Node::row(vec![]), pre_sup, pre_sub));
        }
        let (sub, sup) = pair(&post);
        row.push(if sub.is_some() || sup.is_some() { scripts(base, sup, sub) } else { base });
        Ok(Node::row(row).simplify())
    }

    fn enclose(&mut self, node: Xml, variant: Variant) -> R<Node> {
        let base = self.row(node, variant)?;
        let notation = node.attribute("notation").unwrap_or("longdiv");
        let has = |n: &str| notation.split_whitespace().any(|x| x == n);
        let mut out = base;
        if has("updiagonalstrike") && has("downdiagonalstrike") {
            out = Node::Canceled { base: Box::new(out), shape: Cancel::X };
        } else if has("updiagonalstrike") || has("horizontalstrike") {
            out = Node::Canceled { base: Box::new(out), shape: Cancel::Bar };
        } else if has("downdiagonalstrike") {
            out = Node::Canceled { base: Box::new(out), shape: Cancel::Backslash };
        }
        if has("top") || has("actuarial") {
            out = Node::Bar { base: Box::new(out), above: true };
        }
        if has("bottom") {
            out = Node::Bar { base: Box::new(out), above: false };
        }
        if has("radical") {
            out = Node::Root { radicand: Box::new(out), index: None };
        }
        if has("box") || has("roundedbox") {
            out = Node::Framed { base: Box::new(out), shape: Frame::Rectangle, background: None, border: None };
        } else if has("circle") {
            out = Node::Framed { base: Box::new(out), shape: Frame::Circle, background: None, border: None };
        }
        Ok(out)
    }

    fn table(&mut self, node: Xml, variant: Variant) -> R<Node> {
        let mut rows = Vec::new();
        for tr in Self::elements(node) {
            let row_name = tr.tag_name().name();
            if !matches!(row_name, "mtr" | "mlabeledtr") {
                // a cell without a row: a row of its own
                rows.push(vec![self.element(tr, variant)?]);
                continue;
            }
            let mut cells = Vec::new();
            // the first cell of mlabeledtr is the equation number, drawn at the end like \tag
            let cell_nodes = Self::elements(tr);
            let (label, cell_nodes) = if row_name == "mlabeledtr" && !cell_nodes.is_empty() { (Some(cell_nodes[0]), &cell_nodes[1..]) } else { (None, &cell_nodes[..]) };
            for td in cell_nodes {
                cells.push(self.element(*td, variant)?);
            }
            if let Some(label) = label {
                let l = self.element(label, variant)?;
                if let Some(last) = cells.pop() {
                    cells.push(Node::row(vec![last, Node::Space { em: 2.0 }, l]));
                }
            }
            rows.push(cells);
        }
        let ncol = rows.iter().map(Vec::len).max().unwrap_or(0);
        let align = |a: &str| -> char {
            match a {
                "left" => 'l',
                "right" => 'r',
                _ => 'c',
            }
        };
        let columns: Vec<char> = match node.attribute("columnalign") {
            Some(spec) => {
                let given: Vec<char> = spec.split_whitespace().map(align).collect();
                (0..ncol).map(|j| *given.get(j).or(given.last()).unwrap_or(&'c')).collect()
            }
            None => vec![],
        };
        let lines = |spec: Option<&str>, n: usize| -> Vec<usize> {
            let Some(spec) = spec else { return vec![] };
            let given: Vec<&str> = spec.split_whitespace().collect();
            (1..n).filter(|j| matches!(given.get(j - 1).or(given.last()).copied(), Some("solid" | "dashed"))).collect()
        };
        let verticals = lines(node.attribute("columnlines"), ncol);
        let horizontals = lines(node.attribute("rowlines"), rows.len()).into_iter().map(|r| TableRule { row: r, from: 0, to: usize::MAX, dashed: false }).collect();
        let shape = if columns.is_empty() && verticals.is_empty() { TableKind::Matrix } else { TableKind::Array };
        Ok(Node::Table { shape, rows, columns, verticals, dashed: vec![], horizontals, between: vec![], intertexts: vec![], stretch: 1.0 })
    }
}

/// Text of an element, with the white space MathML ignores (around and repeated) removed.
/// Only XML white space is trimmed and collapsed: a no-break space (`&#xA0;`, as in MathJax's
/// `<mtext>&#xA0;and&#xA0;</mtext>`) is content and stays.
fn text_of(node: Xml) -> String {
    let raw: String = node.descendants().filter(Xml::is_text).filter_map(|t| t.text()).collect();
    raw.split([' ', '\t', '\r', '\n']).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ")
}

fn parse_variant(v: &str) -> Option<Variant> {
    Some(match v {
        "normal" => Variant::Upright,
        "italic" => Variant::Italic,
        "bold" => Variant::Bold,
        "bold-italic" => Variant::BoldItalic,
        "double-struck" => Variant::DoubleStruck,
        "script" | "bold-script" => Variant::Calligraphic,
        "fraktur" | "bold-fraktur" => Variant::Fraktur,
        "sans-serif" | "bold-sans-serif" | "sans-serif-italic" | "sans-serif-bold-italic" => Variant::SansSerif,
        "monospace" => Variant::Monospace,
        _ => return None,
    })
}

/// `3.14`, and in the languages with a decimal comma `3,14` (L-NUM-1); `1,000` or `1.000` with the
/// language's thousands separator (L-NUM-3). The value in canonical form, with a dot.
fn number_value(t: &str, locale: Locale) -> Option<(String, Option<char>)> {
    if t.is_empty() || !t.chars().next().is_some_and(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    let decimal = locale.decimal_separator();
    let thousands = locale.thousands_separator();
    if let Some(sep) = thousands {
        let (int, frac) = t.split_once(decimal).map_or((t, None), |(a, b)| (a, Some(b)));
        let groups: Vec<&str> = int.split(sep).collect();
        if groups.len() > 1
            && groups[0].len() <= 3
            && groups.iter().all(|g| !g.is_empty() && g.chars().all(|c| c.is_ascii_digit()))
            && groups[1..].iter().all(|g| g.len() == 3)
            && frac.is_none_or(|f| f.chars().all(|c| c.is_ascii_digit()))
        {
            let mut value = groups.concat();
            if let Some(f) = frac {
                value.push('.');
                value.push_str(f);
            }
            return Some((value, Some(sep)));
        }
    }
    let canonical = if locale.comma_is_decimal() { t.replace(',', ".") } else { t.to_string() };
    (canonical.matches('.').count() <= 1 && canonical.chars().all(|c| c.is_ascii_digit() || c == '.')).then_some((canonical, None))
}

/// `<mo>`: the class of the character, a large operator, or a function name (`lim`, `max`).
fn operator(t: &str) -> Node {
    let t = t.trim();
    // invisible operators: function application, invisible times, separator and plus
    let visible: String = t.chars().filter(|c| !matches!(c, '\u{2061}'..='\u{2064}')).collect();
    let mut chars = visible.chars();
    match (chars.next(), chars.next()) {
        (None, _) => Node::row(vec![]),
        (Some(c), None) => {
            if let Some(symbol) = large_operator(c) {
                return Node::LargeOperator { symbol, limits: None };
            }
            let symbol = match c {
                '-' => '−',
                '*' => '∗',
                // MathML's dot operator is LaTeX's \cdot
                '⋅' => '·',
                '\'' => '′',
                other => other,
            };
            Node::Operator { symbol, class: symbols::class_of_char(c).or_else(|| symbols::class_of_char(symbol)).unwrap_or(Class::Ord) }
        }
        // MathJax writes consecutive relations as one <mo>≤≥≠</mo>: each symbol is an operator
        _ if visible.chars().all(|c| !c.is_alphanumeric()) && !matches!(visible.as_str(), "..." | "''" | ":=" | "!=" | "<=" | ">=" | "->" | "++" | "--" | "&&" | "||") => {
            Node::row(visible.chars().map(|c| operator(&c.to_string())).collect())
        }
        _ => match visible.as_str() {
            "..." => Node::Operator { symbol: '…', class: Class::Inner },
            "''" => Node::Operator { symbol: '″', class: Class::Ord },
            ":=" => Node::Operator { symbol: '≔', class: Class::Rel },
            "!=" => Node::Operator { symbol: '≠', class: Class::Rel },
            "<=" => Node::Operator { symbol: '≤', class: Class::Rel },
            ">=" => Node::Operator { symbol: '≥', class: Class::Rel },
            "->" => Node::Operator { symbol: '→', class: Class::Rel },
            "++" | "--" | "&&" | "||" => Node::Text { parts: vec![TextPart::Text(visible.clone())] },
            word => match symbols::function(word) {
                Some((canonical, limits)) => Node::Function { name: canonical.to_string(), limits },
                None => Node::Function { name: word.to_string(), limits: false },
            },
        },
    }
}

fn large_operator(c: char) -> Option<char> {
    matches!(
        c,
        '∑' | '∏'
            | '∐'
            | '⋃'
            | '⋂'
            | '⨁'
            | '⨂'
            | '⨀'
            | '⋁'
            | '⋀'
            | '⨆'
            | '⨄'
            | '∫'
            | '∬'
            | '∭'
            | '⨌'
            | '∮'
            | '∯'
            | '∰'
            | '∱'
            | '∲'
            | '∳'
            | '⨏'
            | '⨖'
    )
    .then_some(c)
}

fn scripts(base: Node, sup: Option<Node>, sub: Option<Node>) -> Node {
    Node::Scripts { base: Box::new(base), sup: sup.map(Box::new), sub: sub.map(Box::new) }
}

/// What a lone `<mo>` over or under a base means.
enum Decoration {
    Accent(char, bool, bool),
    Bar(bool),
    Brace(bool, bool),
}

impl Decoration {
    fn apply(self, base: Node) -> Node {
        match self {
            Decoration::Accent(accent, stretchy, below) => Node::Accent { base: Box::new(base), accent, stretchy, below },
            Decoration::Bar(above) => Node::Bar { base: Box::new(base), above },
            Decoration::Brace(above, bracket) => Node::Brace { base: Box::new(base), above, label: None, bracket },
        }
    }
}

fn decoration(mark: &Node, above: bool, accent: bool) -> Option<Decoration> {
    let c = match mark {
        Node::Operator { symbol, .. } => *symbol,
        Node::Identifier { name, .. } if name.chars().count() == 1 => name.chars().next()?,
        Node::Text { parts } => match parts.as_slice() {
            [TextPart::Text(t)] if t.chars().count() == 1 => t.chars().next()?,
            _ => return None,
        },
        _ => return None,
    };
    Some(match (c, above) {
        ('⏞' | '︷', true) => Decoration::Brace(true, false),
        ('⏟' | '︸', false) => Decoration::Brace(false, false),
        ('⎴', true) => Decoration::Brace(true, true),
        ('⎵', false) => Decoration::Brace(false, true),
        ('¯' | '‾' | '_' | '―' | '−' | '\u{0332}' | '\u{0305}', _) => Decoration::Bar(above),
        ('→' | '⟶' | '\u{20D7}', true) if !accent => Decoration::Accent('→', true, false),
        ('→' | '⟶' | '\u{20D7}', true) => Decoration::Accent('\u{20D7}', false, false),
        ('←' | '⟵', _) => Decoration::Accent('←', true, !above),
        ('→' | '⟶', false) => Decoration::Accent('→', true, true),
        ('↔' | '⟷', _) => Decoration::Accent('↔', true, !above),
        ('^' | 'ˆ' | '\u{0302}', true) => Decoration::Accent('\u{0302}', !accent, false),
        ('~' | '˜' | '\u{0303}' | '∼', true) => Decoration::Accent('\u{0303}', !accent, false),
        ('ˇ' | '\u{030C}', true) => Decoration::Accent('\u{030C}', false, false),
        ('˙' | '.' | '\u{0307}', true) => Decoration::Accent('\u{0307}', false, false),
        ('¨' | '\u{0308}', true) => Decoration::Accent('\u{0308}', false, false),
        ('\u{20DB}', true) => Decoration::Accent('\u{20DB}', false, false),
        ('´' | '\u{0301}', true) => Decoration::Accent('\u{0301}', false, false),
        ('`' | '\u{0300}', true) => Decoration::Accent('\u{0300}', false, false),
        ('˘' | '\u{0306}', true) => Decoration::Accent('\u{0306}', false, false),
        ('˚' | '\u{030A}', true) => Decoration::Accent('\u{030A}', false, false),
        ('⏜' | '⌒', true) => Decoration::Accent('⏜', true, false),
        ('⏝', false) => Decoration::Accent('⏝', true, true),
        _ => return None,
    })
}

/// `<mrow>` with fences at its ends: `(`…`)` stretch with the content (like `\left…\right`), and a table
/// between fences is a matrix (`(`, `[`, `|`, `‖`, `{…}`) or `cases` (`{` alone).
fn fences(children: Vec<Node>) -> Node {
    let fence = |n: &Node, class: Class| -> Option<char> {
        match n {
            // bars are fences on either side, whatever their class; MathJax writes \\| as ∥ (U+2225), the
            // parallel sign, where a norm is meant
            Node::Operator { symbol: '∥', .. } => Some('‖'),
            Node::Operator { symbol, class: c } if *c == class || matches!(symbol, '|' | '‖') => Some(*symbol),
            _ => None,
        }
    };
    let n = children.len();
    if n >= 2 {
        let open = fence(&children[0], Class::Open);
        let close = fence(&children[n - 1], Class::Close);
        // a table between fences
        // a table between fences, with or without column alignments (MathJax writes columnalign on cases)
        let inner_table = |range: std::ops::Range<usize>| -> Option<(TableKind, Vec<Vec<Node>>)> {
            match &children[range] {
                [Node::Table { shape: TableKind::Matrix | TableKind::Array, rows, .. }] => Some((TableKind::Matrix, rows.clone())),
                _ => None,
            }
        };
        let table = |shape: TableKind, rows: Vec<Vec<Node>>| Node::Table {
            shape,
            rows,
            columns: vec![],
            verticals: vec![],
            dashed: vec![],
            horizontals: vec![],
            between: vec![],
            intertexts: vec![],
            stretch: 1.0,
        };
        if let (Some(o), Some(c)) = (open, close) {
            if let Some((_, rows)) = inner_table(1..n - 1) {
                let shape = match (o, c) {
                    ('(', ')') => Some(TableKind::PMatrix),
                    ('[', ']') => Some(TableKind::BMatrix),
                    ('{', '}') => Some(TableKind::BraceMatrix),
                    ('|', '|') => Some(TableKind::VMatrix),
                    ('‖', '‖') => Some(TableKind::VVMatrix),
                    _ => None,
                };
                if let Some(shape) = shape {
                    return table(shape, rows);
                }
            }
        }
        if open == Some('{') && n == 2 {
            if let Some((_, rows)) = inner_table(1..2) {
                return table(TableKind::Cases, rows);
            }
        }
        if close == Some('}') && n == 2 {
            if let Some((_, rows)) = inner_table(0..1) {
                return table(TableKind::RCases, rows);
            }
        }
        if let (Some(o), Some(c)) = (open, close) {
            let middle = &children[1..n - 1];
            if balanced(middle) && (n > 2 || !matches!(o, '|' | '‖')) {
                return Node::Delimited { open: Some(o), close: Some(c), parts: vec![Node::row(middle.to_vec())], middles: vec![] };
            }
        }
    }
    Node::row(children).simplify()
}

/// The opening and closing operators inside never close more than they open.
fn balanced(nodes: &[Node]) -> bool {
    let mut depth = 0i32;
    for n in nodes {
        match n {
            Node::Operator { class: Class::Open, .. } => depth += 1,
            Node::Operator { class: Class::Close, .. } => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

/// MathML lengths: `0.2em`, `1ex`, `3pt`, `2px`, `1mu`, and the named spaces (`thinmathspace`…).
fn measure_em(m: &str) -> Option<f32> {
    let m = m.trim();
    let named = [
        ("veryverythinmathspace", 1.0),
        ("verythinmathspace", 2.0),
        ("thinmathspace", 3.0),
        ("mediummathspace", 4.0),
        ("thickmathspace", 5.0),
        ("verythickmathspace", 6.0),
        ("veryverythickmathspace", 7.0),
    ];
    if let Some((name, mu)) = named.iter().find(|(n, _)| m.trim_start_matches("negative") == *n) {
        let sign = if m.starts_with("negative") && m != *name { -1.0 } else { 1.0 };
        return Some(sign * mu / 18.0);
    }
    let units = [("em", 1.0), ("ex", 0.45), ("pt", 0.1), ("px", 0.075), ("mu", 1.0 / 18.0), ("cm", 2.845), ("mm", 0.2845), ("in", 7.227)];
    for (u, f) in units {
        if let Some(v) = m.strip_suffix(u) {
            return v.trim().parse::<f32>().ok().map(|v| v * f);
        }
    }
    m.parse::<f32>().ok()
}

/// XML only knows `&lt;`, `&gt;`, `&amp;`, `&quot;` and `&apos;`: MathML's named entities (`&InvisibleTimes;`,
/// `&times;`, `&alpha;`) become their characters before parsing. An unknown one is dropped (E-REC-10).
fn replace_entities(source: &str, registry: &mut Registry, strict: bool) -> R<String> {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    let mut offset = 0;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let name: String = after.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
        if !name.is_empty() && after[name.len()..].starts_with(';') && !matches!(name.as_str(), "lt" | "gt" | "amp" | "quot" | "apos") {
            let column = source[..offset + i].chars().count() + 1;
            match entity(&name) {
                Some(c) => out.push(c),
                None if strict => return Err(Error::Syntax { column, message: format!("unknown entity &{name};") }),
                None => registry.warn(rules::E_REC_10, format!("column {column}: unknown entity &{name}; ignored")),
            }
            rest = &after[name.len() + 1..];
            offset += i + name.len() + 2;
        } else {
            out.push('&');
            rest = after;
            offset += i + 1;
        }
    }
    out.push_str(rest);
    Ok(out)
}

fn entity(name: &str) -> Option<char> {
    if let Some(g) = symbols::greek(name) {
        return Some(g);
    }
    Some(match name {
        "ApplyFunction" | "af" => '\u{2061}',
        "InvisibleTimes" | "it" => '\u{2062}',
        "InvisibleComma" | "ic" => '\u{2063}',
        "nbsp" | "NonBreakingSpace" => '\u{00A0}',
        "ThinSpace" | "thinsp" => '\u{2009}',
        "MediumSpace" => '\u{205F}',
        "times" => '×',
        "divide" | "div" => '÷',
        "minus" => '−',
        "plusmn" | "PlusMinus" | "pm" => '±',
        "mnplus" | "MinusPlus" | "mp" => '∓',
        "sdot" | "middot" | "centerdot" | "CenterDot" => '·',
        "ast" => '∗',
        "compfn" | "SmallCircle" => '∘',
        "le" | "leq" | "LessEqual" => '≤',
        "ge" | "geq" | "GreaterEqual" => '≥',
        "ne" | "NotEqual" => '≠',
        "equiv" | "Congruent" => '≡',
        "approx" | "TildeTilde" => '≈',
        "sim" | "Tilde" => '∼',
        "cong" | "TildeFullEqual" => '≅',
        "prop" | "Proportional" | "propto" => '∝',
        "Lt" | "ll" => '≪',
        "Gt" | "gg" => '≫',
        "isin" | "in" | "Element" => '∈',
        "notin" | "NotElement" => '∉',
        "ni" | "ReverseElement" => '∋',
        "sub" | "subset" => '⊂',
        "sup" | "supset" => '⊃',
        "sube" | "subseteq" | "SubsetEqual" => '⊆',
        "supe" | "supseteq" | "SupersetEqual" => '⊇',
        "cap" | "Intersection" => '∩',
        "cup" | "Union" => '∪',
        "and" | "wedge" => '∧',
        "or" | "vee" => '∨',
        "not" | "Not" => '¬',
        "forall" | "ForAll" => '∀',
        "exist" | "Exists" => '∃',
        "nexist" | "NotExists" => '∄',
        "empty" | "emptyset" | "EmptySet" | "varnothing" => '∅',
        "nabla" | "Del" => '∇',
        "part" | "PartialD" => '∂',
        "DifferentialD" | "dd" => 'ⅆ',
        "ExponentialE" | "ee" => 'ⅇ',
        "ImaginaryI" | "ii" => 'ⅈ',
        "infin" | "infty" | "Infinity" => '∞',
        "int" | "Integral" => '∫',
        "Int" => '∬',
        "tint" | "iiint" => '∭',
        "conint" | "oint" | "ContourIntegral" => '∮',
        "sum" | "Sum" => '∑',
        "prod" | "Product" => '∏',
        "coprod" | "Coproduct" => '∐',
        "radic" | "Sqrt" => '√',
        "deg" => '°',
        "prime" => '′',
        "Prime" => '″',
        "tprime" => '‴',
        "hellip" | "mldr" => '…',
        "ctdot" | "cdots" => '⋯',
        "vellip" | "vdots" => '⋮',
        "dtdot" | "ddots" => '⋱',
        "perp" | "bottom" | "UpTee" => '⊥',
        "par" | "parallel" | "DoubleVerticalBar" => '∥',
        "ang" | "angle" => '∠',
        "angmsd" | "measuredangle" => '∡',
        "rarr" | "rightarrow" | "RightArrow" | "srarr" => '→',
        "larr" | "leftarrow" | "LeftArrow" | "slarr" => '←',
        "harr" | "leftrightarrow" | "LeftRightArrow" => '↔',
        "uarr" | "uparrow" | "UpArrow" => '↑',
        "darr" | "downarrow" | "DownArrow" => '↓',
        "rArr" | "Rightarrow" | "DoubleRightArrow" | "Implies" => '⇒',
        "lArr" | "Leftarrow" | "DoubleLeftArrow" => '⇐',
        "hArr" | "iff" | "Leftrightarrow" | "DoubleLeftRightArrow" => '⇔',
        "map" | "mapsto" | "RightTeeArrow" => '↦',
        "rlhar" | "Equilibrium" | "rightleftharpoons" => '⇌',
        "lang" | "langle" | "LeftAngleBracket" => '⟨',
        "rang" | "rangle" | "RightAngleBracket" => '⟩',
        "lceil" | "LeftCeiling" => '⌈',
        "rceil" | "RightCeiling" => '⌉',
        "lfloor" | "LeftFloor" => '⌊',
        "rfloor" | "RightFloor" => '⌋',
        "lbrace" | "lcub" => '{',
        "rbrace" | "rcub" => '}',
        "lsqb" | "lbrack" => '[',
        "rsqb" | "rbrack" => ']',
        "lpar" => '(',
        "rpar" => ')',
        "verbar" | "vert" | "VerticalLine" => '|',
        "Verbar" | "Vert" => '‖',
        "OverBar" => '‾',
        "UnderBar" | "lowbar" => '_',
        "OverBrace" => '⏞',
        "UnderBrace" => '⏟',
        "OverParenthesis" => '⏜',
        "UnderParenthesis" => '⏝',
        "Hat" => '^',
        "DiacriticalTilde" | "tilde" => '˜',
        "DiacriticalDot" | "dot" => '˙',
        "die" | "Dot" | "uml" => '¨',
        "macr" | "strns" => '¯',
        "hbar" | "planck" | "plankv" => 'ℏ',
        "ell" => 'ℓ',
        "weierp" | "wp" => '℘',
        "real" | "Re" => 'ℜ',
        "image" | "Im" => 'ℑ',
        "aleph" => 'ℵ',
        "Ropf" | "reals" => 'ℝ',
        "Nopf" | "naturals" => 'ℕ',
        "Zopf" | "integers" => 'ℤ',
        "Qopf" | "rationals" => 'ℚ',
        "Copf" | "complexes" => 'ℂ',
        "there4" | "therefore" => '∴',
        "becaus" | "because" => '∵',
        "setminus" | "setmn" | "Backslash" => '∖',
        "oplus" | "CirclePlus" => '⊕',
        "otimes" | "CircleTimes" => '⊗',
        "odot" | "CircleDot" => '⊙',
        "star" => '⋆',
        "bull" | "bullet" => '•',
        "dagger" => '†',
        "Dagger" => '‡',
        "check" | "checkmark" => '✓',
        "copy" => '©',
        "reg" => '®',
        "sect" => '§',
        "para" => '¶',
        "micro" => 'µ',
        "ohm" => 'Ω',
        "angst" => 'Å',
        "permil" => '‰',
        "percnt" => '%',
        "dollar" => '$',
        "euro" => '€',
        "pound" => '£',
        "yen" => '¥',
        "cent" => '¢',
        "excl" => '!',
        "quest" => '?',
        "comma" => ',',
        "period" => '.',
        "colon" => ':',
        "semi" => ';',
        "num" => '#',
        "plus" => '+',
        "equals" => '=',
        "sol" => '/',
        "bsol" => '\\',
        "Hacek" => 'ˇ',
        "Breve" | "breve" => '˘',
        "acute" | "DiacriticalAcute" => '´',
        "grave" | "DiacriticalGrave" => '`',
        "lt" => '<',
        "gt" => '>',
        "ndash" => '–',
        "mdash" => '—',
        "epsi" => 'ϵ',
        "epsiv" => 'ε',
        "phiv" => 'φ',
        "thetav" => 'ϑ',
        "sigmaf" => 'ς',
        _ => return None,
    })
}
