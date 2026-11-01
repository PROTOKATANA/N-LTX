//! The figure in words, for readers who cannot see it: what the engine noted while it drew (the
//! nodes and their text, the connections between them, the axes and their plots) becomes a short
//! alternative text, the SVG's `<title>`, and a longer description, its `<desc>`. Math in the text
//! is read as mathra speaks it.
//!
//! The words come in the 16 languages of mathra, whatever the figure is written in: each language
//! is a JSON file in `src/locales/` of phrases with `{0}`, `{1}`…, its plural forms as `key.one`,
//! `key.two`, `key.few`, `key.many` and `key.other`; a phrase a language lacks is English.

use mathra::locale::Locale;

use crate::scene::Point;

/// What the engine notes while it draws.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    pub nodes: Vec<NodeFact>,
    pub links: Vec<Link>,
    pub axes: Vec<AxisFact>,
    /// Charts of the data visualization library, told by their text alone.
    pub data_visualizations: usize,
    /// Paths drawn, filled or shaded outside nodes and axes.
    pub shapes: usize,
    /// While above zero (an axis drawing its ticks, labels and legend), nothing is noted.
    pub mute: usize,
    /// The text the author gave (`alt={…}`): told as it is, instead of what tikra would write.
    pub author_alt: String,
    /// The node the coordinate just read names (`(a)`, `(a.east)`), for the path's connections.
    pub touched: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NodeFact {
    /// The name it is known by (`tikz@f@…` when it has none).
    pub key: String,
    pub named: bool,
    /// The text, as TeX.
    pub text: String,
    pub at: Point,
    /// A label on a connection (`-- node {yes} (b)`), told with it.
    pub on_link: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrow {
    None,
    Forward,
    Backward,
    Both,
}

/// A drawn line or curve from one node to another.
#[derive(Debug, Clone)]
pub struct Link {
    pub from: String,
    pub to: String,
    pub arrow: Arrow,
    /// The nodes placed on it, by index into [`Facts::nodes`].
    pub labels: Vec<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct AxisFact {
    /// `polar`, `ternary`, `smith` or empty.
    pub kind: &'static str,
    pub three_d: bool,
    pub title: String,
    pub labels: [String; 3],
    /// The data's range on each axis (with the limits given), when known.
    pub range: [Option<[f64; 2]>; 3],
    pub log: [bool; 3],
    /// `symbolic x coords` and friends.
    pub symbolic: [Vec<String>; 3],
    pub series: Vec<Series>,
    /// The range of the values colors stand for (surfaces, scatter by meta).
    pub colors: Option<[f64; 2]>,
}

#[derive(Debug, Clone, Default)]
pub struct Series {
    pub kind: &'static str,
    /// Its legend entry, as TeX.
    pub name: String,
    /// The formula it was sampled from, when it was.
    pub expression: String,
    pub points: Vec<[f64; 3]>,
    /// A box plot: lower whisker, lower quartile, median, upper quartile, upper whisker.
    pub boxplot: Option<[Option<f64>; 5]>,
}

/// The figure in words: `alt` short (the `<title>`, an image's `alt`), `text` the whole of it.
/// When the author wrote it (`alt={…}`), both are the author's text and the SVG has no `<desc>`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Description {
    pub alt: String,
    pub text: String,
    pub by_author: bool,
}

/// Points listed one by one up to this many; beyond, their range.
pub(in crate::describe) const LIST_POINTS: usize = 12;
/// Nodes and connections told one by one up to these many.
pub(in crate::describe) const LIST_NODES: usize = 40;
pub(in crate::describe) const LIST_LINKS: usize = 60;

mod chart;
mod speech;
mod words;

pub use speech::{num, num_in, spoken};

use chart::{describe_axis, visible_nodes};
use speech::is_number;
use words::Words;

/// The author's text, when `alt={…}` was given.
fn author_description(w: &Words, f: &Facts) -> Option<Description> {
    let author = w.spoken(&f.author_alt);
    if author.is_empty() {
        None
    } else {
        Some(Description { alt: author.clone(), text: author, by_author: true })
    }
}

/// Connections worth telling, and how many named ends they join.
struct ToldLinks {
    lines: Vec<String>,
    nodes: usize,
}

fn told_links(w: &Words, f: &Facts) -> ToldLinks {
    let name_of = |key: &str| -> Option<String> {
        let n = f.nodes.iter().rev().find(|n| n.key == key)?;
        let t = w.spoken(&n.text);
        if !t.is_empty() {
            Some(t)
        } else if n.named && !n.key.contains('@') {
            // a name the author gave (internal names have an `@`)
            Some(n.key.clone())
        } else {
            None
        }
    };
    let mut lines: Vec<String> = Vec::new();
    let mut joined: Vec<&str> = Vec::new();
    for l in &f.links {
        let (Some(a), Some(b)) = (name_of(&l.from), name_of(&l.to)) else {
            continue;
        };
        for k in [&l.from, &l.to] {
            if !joined.contains(&k.as_str()) {
                joined.push(k);
            }
        }
        let mut s = match l.arrow {
            _ if l.from == l.to => w.t("link.self", &[&a]),
            Arrow::Forward => w.t("link.forward", &[&a, &b]),
            Arrow::Backward => w.t("link.forward", &[&b, &a]),
            Arrow::Both => w.t("link.both", &[&a, &b]),
            Arrow::None => w.t("link.none", &[&a, &b]),
        };
        let labels: Vec<String> = l.labels.iter().map(|&i| w.spoken(&f.nodes[i].text)).filter(|t| !t.is_empty()).collect();
        if !labels.is_empty() {
            s = w.t("link.labeled", &[&s, &w.quoted_list(&labels)]);
        }
        if !lines.contains(&s) {
            lines.push(s);
        }
    }
    ToldLinks { lines, nodes: joined.len() }
}

/// Node texts worth reading, and whether each one is only a number.
struct NodeWords {
    texts: Vec<(String, bool)>,
    words: Vec<String>,
}

fn node_words(w: &Words, nodes: &[&NodeFact]) -> NodeWords {
    let texts: Vec<(String, bool)> = nodes.iter().map(|n| (w.spoken(&n.text), is_number(&n.text))).filter(|t| !t.0.is_empty()).collect();
    let words: Vec<String> = texts.iter().map(|t| t.0.clone()).collect();
    NodeWords { texts, words }
}

/// The description as it is being written: the language, the short text and the sentences.
struct Prose<'a> {
    w: &'a Words,
    alt: String,
    parts: Vec<String>,
}

/// The axes, one sentence each. The short text is the first axis, or how many plots there are.
fn axis_parts(prose: &mut Prose<'_>, f: &Facts) {
    let w = prose.w;
    let axes: Vec<&AxisFact> = f.axes.iter().collect();
    for (i, a) in axes.iter().enumerate() {
        let (short, long) = describe_axis(w, a);
        if i == 0 {
            prose.alt = short.clone();
        }
        if axes.len() > 1 {
            prose.parts.push(w.t("plot_of", &[&(i + 1).to_string(), &axes.len().to_string(), &long]));
        } else {
            prose.parts.push(long);
        }
    }
    if axes.len() > 1 {
        let kinds: Vec<String> = axes.iter().map(|a| describe_axis(w, a).0).collect();
        let count = w.count("plot", axes.len());
        prose.alt = if kinds.iter().all(|k| *k == kinds[0]) { w.t("plots", &[&count, &kinds[0]]) } else { count };
    }
}

/// A diagram: how many nodes and connections, then their texts.
fn diagram_text(prose: &mut Prose<'_>, links: &ToldLinks, words: &NodeWords) {
    let w = prose.w;
    let summary = w.t("diagram", &[&w.count("node", links.nodes), &w.count("connection", links.lines.len())]);
    if prose.alt.is_empty() {
        prose.alt = summary.clone();
    }
    prose.parts.push(w.sentence(&summary));
    if !words.words.is_empty() {
        prose.parts.push(w.sentence(&w.t("nodes", &[&w.capped(&words.words, LIST_NODES)])));
    }
    prose.parts.push(w.sentence(&w.t("connections", &[&w.capped(&links.lines, LIST_LINKS)])));
}

/// A figure told by the text on it. The short text stays when the axes already gave one.
fn figure_text(prose: &mut Prose<'_>, f: &Facts, words: &NodeWords) {
    let w = prose.w;
    let charts = f.data_visualizations;
    let what = if !f.axes.is_empty() {
        "text.other"
    } else if charts > 0 {
        "text.chart"
    } else {
        "text.figure"
    };
    if f.axes.is_empty() && charts > 0 {
        prose.parts.push(w.sentence(&w.count("datavis", charts)));
    }
    prose.parts.push(w.sentence(&w.t(what, &[&w.capped(&words.words, LIST_NODES)])));
    if !prose.alt.is_empty() {
        return;
    }
    // words before numbers: tick labels say less than titles and names
    let mut first: Vec<String> = words.texts.iter().filter(|t| !t.1).take(3).map(|t| t.0.clone()).collect();
    if first.is_empty() {
        first = words.words.iter().take(3).cloned().collect();
    }
    let mut list = w.quoted_list(&first);
    if words.words.len() > first.len() {
        list.push_str(w.phrase("sep.items"));
        list.push('…');
    }
    prose.alt = if charts > 0 {
        w.t("alt.chart_text", &[&list])
    } else if f.shapes == 0 {
        w.t("alt.drawing_text", &[&list])
    } else {
        w.t("alt.shapes_text", &[&w.count("shape", f.shapes), &list])
    };
}

fn shapes_alt(w: &Words, f: &Facts) -> String {
    match f.shapes {
        0 => w.t("alt.empty", &[]),
        n => w.t("alt.shapes", &[&w.count("shape", n)]),
    }
}

/// The figure in words, in `locale`; the author's text (`alt={…}`) when there is one.
pub fn describe(f: &Facts, locale: Locale) -> Description {
    let w = Words { locale };
    if let Some(written) = author_description(&w, f) {
        return written;
    }
    let nodes = visible_nodes(f);
    let links = told_links(&w, f);
    let words = node_words(&w, &nodes);
    let mut prose = Prose { w: &w, alt: String::new(), parts: Vec::new() };
    axis_parts(&mut prose, f);
    if !links.lines.is_empty() {
        diagram_text(&mut prose, &links, &words);
    } else if !words.words.is_empty() {
        figure_text(&mut prose, f, &words);
    }
    if prose.alt.is_empty() {
        prose.alt = shapes_alt(&w, f);
    }
    let text = if prose.parts.is_empty() { w.sentence(&prose.alt) } else { prose.parts.join(w.phrase("sep.sentence")) };
    Description { alt: prose.alt, text, by_author: false }
}

#[cfg(test)]
mod tests {
    use super::words::{phrases, plural_form, Words};
    use super::*;

    #[test]
    fn numbers() {
        assert_eq!(num(0.0), "0");
        assert_eq!(num(2.5), "2.5");
        assert_eq!(num(1.0 / 3.0), "0.3333");
        assert_eq!(num(2019.0), "2019");
        assert_eq!(num(-0.00001), "-0.00001");
        assert_eq!(num(f64::NAN), "NaN");
        // as precise as the range it is in
        assert_eq!(num_in(0.000000000069, 4.0), "0");
        assert_eq!(num_in(0.004432, 0.47), "0.0044");
    }

    #[test]
    fn an_axis_with_nothing_to_tell() {
        let f = Facts { axes: vec![AxisFact { series: vec![Series::default()], ..Default::default() }], ..Default::default() };
        assert_eq!(describe(&f, Locale::En), Description { alt: "Chart".into(), text: "Chart. No data.".into(), by_author: false });
        // a label, without a range
        let a = AxisFact { labels: ["time".into(), String::new(), String::new()], ..Default::default() };
        assert_eq!(describe(&Facts { axes: vec![a], ..Default::default() }, Locale::En).text, "Chart. Horizontal axis “time”. No data.");
    }

    #[test]
    fn plural_forms() {
        let forms = |l: Locale| [0, 1, 2, 3, 5, 11, 12, 21, 22, 25, 101, 111].map(|n| plural_form(l, n));
        assert_eq!(forms(Locale::En), ["other", "one", "other", "other", "other", "other", "other", "other", "other", "other", "other", "other"]);
        assert_eq!(forms(Locale::PtBr)[..3], ["one", "one", "other"]);
        assert_eq!(forms(Locale::Ru), ["many", "one", "few", "few", "many", "many", "many", "one", "few", "many", "one", "many"]);
        assert_eq!(forms(Locale::Pl), ["many", "one", "few", "few", "many", "many", "many", "many", "few", "many", "many", "many"]);
        assert_eq!(forms(Locale::Ar), ["other", "one", "two", "few", "few", "other", "other", "other", "other", "other", "other", "other"]);
        assert_eq!(forms(Locale::Ja), ["other"; 12]);
    }

    #[test]
    fn every_language_has_its_words() {
        for l in Locale::ALL {
            let p = phrases(l);
            assert_eq!(p["code"], l.code());
            // every phrase a language has is one English has, with the same placeholders
            for (k, v) in p.iter().filter(|(k, _)| !["code", "name"].contains(&k.as_str())) {
                let base = k.rsplit_once('.').filter(|(_, f)| ["one", "two", "few", "many", "other"].contains(f)).map_or(k.as_str(), |(b, _)| b);
                let en = phrases(Locale::En).get(k.as_str()).or_else(|| phrases(Locale::En).get(&format!("{base}.other")));
                let en = en.unwrap_or_else(|| panic!("{}: '{k}' is not an English phrase", l.code()));
                for i in 0..5 {
                    let ph = format!("{{{i}}}");
                    // a plural form may leave out the number ("one node")
                    if !(k.ends_with(".one") || k.ends_with(".two")) {
                        assert_eq!(v.contains(&ph), en.contains(&ph), "{}: '{k}' and {ph}", l.code());
                    }
                }
            }
            let w = Words { locale: l };
            assert!(!w.count("node", 3).is_empty() && !w.t("chart.bar", &[]).is_empty());
        }
    }

    #[test]
    fn in_portuguese() {
        let a = AxisFact {
            title: "Vendas".into(),
            range: [Some([0.0, 1.0]), Some([0.0, 10.0]), None],
            series: vec![Series { kind: "line", points: vec![[0.0, 1.5, 0.0], [1.0, 2.25, 0.0]], ..Default::default() }],
            ..Default::default()
        };
        let d = describe(&Facts { axes: vec![a], ..Default::default() }, Locale::PtBr);
        assert_eq!(d.alt, "Gráfico de linhas: Vendas");
        assert_eq!(d.text, "Gráfico de linhas com o título “Vendas”. Eixo horizontal, de 0 a 1. Eixo vertical, de 0 a 10. 2 pontos — (0; 1,5); (1; 2,25).");
        assert_eq!(spoken(r"$\frac{3}{4}$", Locale::PtBr), "três quartos");
    }

    #[test]
    fn spoken_text() {
        assert_eq!(spoken(r"\textbf{Start}", Locale::En), "Start");
        assert_eq!(spoken(r"Line one\\Line two", Locale::En), "Line one Line two");
        assert_eq!(spoken(r"temperature ($^\circ$C)", Locale::En), "temperature (degrees C)");
        assert_eq!(spoken(r"50\% off", Locale::En), "50% off");
        assert_eq!(spoken(r"\textcolor{red}{hot}", Locale::En), "hot");
        assert_eq!(spoken(r"$x$", Locale::En), "x");
        assert_eq!(spoken(r"$\frac{3}{4}$", Locale::En), "3 over 4");
        assert_eq!(spoken(r"$$x$$, \(y\) and \[z\]", Locale::En), "x, y and z");
        assert_eq!(spoken(r"costs $5", Locale::En), "costs $5");
        assert_eq!(spoken(r"{a}\\[2pt]b~c\ldots\", Locale::En), "a b c…");
        assert_eq!(spoken(r"\hspace*{1cm}x \includegraphics [width=1cm]{f{1}.png}y", Locale::En), "x y");
    }
}
