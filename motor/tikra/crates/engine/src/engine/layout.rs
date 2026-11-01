//! Pictures laid out as TeX sets boxes in a paragraph, and each `tabular` folded into one box.

use crate::canvas::{Canvas, Item};
use crate::scene::Point;

/// A finished picture: its canvas and the height of its baseline in it (cm).
#[derive(Debug, Clone)]
pub(super) struct PictureBox {
    pub(super) canvas: Canvas,
    pub(super) baseline: f64,
}

/// What a part of the document typesets on its own: its pictures, how they are laid out, its words.
pub(super) type Apart = (Vec<PictureBox>, Vec<Layout>, Vec<(String, String)>);

/// What a document puts between pictures: TeX typesets them in lines, like words.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Layout {
    Picture(usize),
    /// a word of the text around the pictures (in `words`)
    Word(usize),
    Space,
    /// `\quad`, `\qquad`, `\hspace{…}`: glue of that width (cm)
    Skip(f64),
    Par,
    /// `\\`: the line ends there
    Break,
    /// `center` begins (true) or ends: its lines are centered in the line's width
    Center(bool),
    /// a `tabular` begins: its columns' alignments, two bits each (`l` 0, `c` 1, `r` 2), with
    /// their count in the low byte
    TabStart(u32),
    /// `&`, `\\` and the end of a `tabular`
    Cell,
    Row,
    TabEnd,
    /// the width of the line (cm), in a `\vbox` of that `\hsize`
    Width(f64),
    /// the pictures are in a `\vbox` (not a `minipage`): glue at its end is part of its height
    Vbox,
}

/// TeX's interword space and line spacing for Computer Modern at 10 pt, in cm.
pub(super) const SPACE: f64 = 3.33333 * crate::PT;
const BASELINE_SKIP: f64 = 12.0 * crate::PT;
const LINE_SKIP: f64 = 1.0 * crate::PT;
/// LaTeX's `\topsep` and `\partopsep` at 10 pt (their natural sizes), in cm.
const TOPSEP: f64 = 8.0 * crate::PT;
const PARTOPSEP: f64 = 2.0 * crate::PT;
/// `\tabcolsep`, and the strut of a row (`\arraystretch` 1: 0.7 and 0.3 `\baselineskip`), in cm.
const TABCOLSEP: f64 = 6.0 * crate::PT;
const STRUT_HEIGHT: f64 = 8.4 * crate::PT;
const STRUT_DEPTH: f64 = 3.6 * crate::PT;
/// The math axis of Computer Modern at 10 pt, which a `tabular` is centered on.
const MATH_AXIS: f64 = 2.5 * crate::PT;

struct Line {
    items: Vec<Layout>,
    center: bool,
    empty_kept: bool,
    /// vertical glue above the line (`\addvspace`: the larger of two that meet)
    skip: f64,
}

fn new_line(center: bool) -> Line {
    Line { items: Vec::new(), center, empty_kept: false, skip: 0.0 }
}

struct Gathered {
    lines: Vec<Line>,
    hsize: Option<f64>,
    vbox: bool,
}

/// The lines of a paragraph: a paragraph ends one, so does `\\`.
fn gather_lines(layout: &[Layout]) -> Gathered {
    let mut lines = vec![new_line(false)];
    let mut center = false;
    let mut hsize: Option<f64> = None;
    let mut after_break = false;
    let mut vbox = false;
    // whether TeX is between paragraphs (vertical mode), and whether it was before the last
    // paragraph's end
    let (mut vmode, mut vmode_before_par) = (true, true);
    // `center` is a list: `\topsep` above and below it, `\partopsep` more if it begins a paragraph
    let mut topsep_add = 0.0;
    for l in layout {
        match l {
            Layout::Par => {
                // in `center`, `\\` ends the paragraph itself: the paragraph's end after it adds no line
                if after_break && !center {
                    lines.last_mut().expect("a line").empty_kept = true;
                }
                vmode_before_par = vmode;
                vmode = true;
                lines.push(new_line(center));
            }
            Layout::Break => {
                vmode = center;
                lines.push(new_line(center));
            }
            Layout::Center(c) => {
                center = *c;
                if center {
                    topsep_add = TOPSEP + if vmode_before_par { PARTOPSEP } else { 0.0 };
                }
                let last = lines.last_mut().expect("a line");
                last.center = center;
                last.skip = last.skip.max(topsep_add);
            }
            Layout::Vbox => vbox = true,
            other => {
                if let Layout::Width(w) = other {
                    hsize = Some(*w);
                }
                if matches!(other, Layout::Picture(_) | Layout::Word(_) | Layout::Skip(_)) {
                    vmode = false;
                }
                lines.last_mut().expect("a line").items.push(*other);
            }
        }
        after_break = matches!(l, Layout::Break) || (after_break && matches!(l, Layout::Center(_)));
    }
    Gathered { lines, hsize, vbox }
}

struct Measured<'a> {
    boxes: &'a [PictureBox],
    labels: &'a [crate::scene::Label],
}

fn item_width(item: &Layout, measured: &Measured) -> f64 {
    match item {
        Layout::Picture(i) => {
            let b = &measured.boxes[*i];
            b.canvas.min.zip(b.canvas.max).map_or(0.0, |(a, b)| b.x - a.x)
        }
        Layout::Word(i) => measured.labels[*i].width(),
        Layout::Skip(w) => *w,
        Layout::Space => SPACE,
        _ => 0.0,
    }
}

struct Breaker<'a> {
    width: f64,
    measured: &'a Measured<'a>,
    broken: Vec<Line>,
    cur: Line,
    chunk: Vec<Layout>,
    chunk_w: f64,
    x: f64,
    pending: f64,
    started: bool,
}

impl Breaker<'_> {
    fn flush(&mut self) {
        if self.chunk.is_empty() {
            return;
        }
        let content = self.chunk.iter().any(|c| matches!(c, Layout::Picture(_) | Layout::Word(_)));
        if self.started && content && self.x + self.pending + self.chunk_w > self.width + 1e-9 {
            let next = Line { items: Vec::new(), center: self.cur.center, empty_kept: false, skip: 0.0 };
            self.broken.push(std::mem::replace(&mut self.cur, next));
            self.x = 0.0;
            self.pending = 0.0;
        } else if self.started {
            self.cur.items.push(Layout::Space);
            self.x += self.pending;
        }
        self.pending = 0.0;
        self.cur.items.append(&mut self.chunk);
        self.x += self.chunk_w;
        self.chunk_w = 0.0;
        self.started |= content;
    }

    fn take_line(&mut self, line: Line) {
        self.cur = Line { items: Vec::new(), center: line.center, empty_kept: line.empty_kept, skip: line.skip };
        self.chunk.clear();
        self.chunk_w = 0.0;
        self.x = 0.0;
        self.pending = 0.0;
        self.started = false;
        for item in line.items {
            if matches!(item, Layout::Space) {
                self.flush();
                if self.started {
                    self.pending = SPACE;
                }
            } else {
                self.chunk_w += item_width(&item, self.measured);
                self.chunk.push(item);
            }
        }
        self.flush();
        self.broken.push(std::mem::replace(&mut self.cur, new_line(false)));
    }
}

/// A line longer than the line's width is broken at its spaces.
fn break_wide(lines: Vec<Line>, width: f64, measured: &Measured) -> Vec<Line> {
    let mut breaker = Breaker { width, measured, broken: Vec::new(), cur: new_line(false), chunk: Vec::new(), chunk_w: 0.0, x: 0.0, pending: 0.0, started: false };
    for line in lines {
        breaker.take_line(line);
    }
    breaker.broken
}

fn word_labels(words: &[(String, String)]) -> Vec<crate::scene::Label> {
    let mut warnings = Vec::new();
    words.iter().map(|(font, w)| crate::label::compose_label(w, font, None, 'l', crate::color::Color::BLACK, &mut warnings)).collect()
}

struct Placed<'a> {
    boxes: &'a [PictureBox],
    labels: &'a [crate::scene::Label],
    layout: &'a [Layout],
    lines: &'a [Line],
    hsize: Option<f64>,
    vbox: bool,
    margin: f64,
}

fn line_shown(line: &Line, has_picture: bool) -> bool {
    line.items.iter().any(|x| matches!(x, Layout::Picture(_))) || (has_picture && (line.empty_kept || line.items.iter().any(|x| matches!(x, Layout::Word(_)))))
}

struct RowSize {
    height: f64,
    depth: f64,
}

fn line_size(line: &Line, placed: &Placed) -> RowSize {
    let (mut height, mut depth) = (0.0f64, 0.0f64);
    for l in &line.items {
        match l {
            Layout::Picture(i) => {
                let b = &placed.boxes[*i];
                let (lo, hi) = (b.canvas.min.map_or(0.0, |m| m.y), b.canvas.max.map_or(0.0, |m| m.y));
                height = height.max(hi - b.baseline);
                depth = depth.max(b.baseline - lo);
            }
            Layout::Word(i) => {
                height = height.max(placed.labels[*i].height());
                depth = depth.max(placed.labels[*i].depth());
            }
            _ => {}
        }
    }
    RowSize { height, depth }
}

fn centered_x(line: &Line, hsize: Option<f64>, measured: &Measured) -> f64 {
    let Some(w) = hsize.filter(|_| line.center) else { return 0.0 };
    let first = line.items.iter().position(|l| matches!(l, Layout::Picture(_) | Layout::Word(_) | Layout::Skip(_))).unwrap_or(line.items.len());
    let natural: f64 = line.items[first..].iter().map(|item| item_width(item, measured)).sum();
    ((w - natural) / 2.0).max(0.0)
}

struct Pen<'a> {
    out: &'a mut Canvas,
    boxes: &'a [PictureBox],
    labels: &'a [crate::scene::Label],
    x: f64,
    baseline_y: f64,
    started: bool,
}

impl Pen<'_> {
    fn paint(&mut self, item: &Layout) {
        match item {
            Layout::Space if self.started => self.x += SPACE,
            // glue at the start of a paragraph stays (`\hskip 1cm` before a picture)
            Layout::Skip(w) => self.x += w,
            Layout::Width(w) => {
                self.out.include(Point::new(0.0, self.baseline_y));
                self.out.include(Point::new(*w, self.baseline_y));
            }
            Layout::Word(i) => self.paint_word(*i),
            Layout::Picture(i) => self.paint_picture(*i),
            _ => {}
        }
    }

    fn paint_word(&mut self, index: usize) {
        let label = &self.labels[index];
        self.out.include(Point::new(self.x, self.baseline_y - label.depth()));
        self.out.include(Point::new(self.x + label.width(), self.baseline_y + label.height()));
        self.out.items.push(Item::Text { label: label.clone(), place: crate::scene::Transform::translation(self.x, self.baseline_y), opacity: 1.0 });
        self.x += label.width();
        self.started = true;
    }

    fn paint_picture(&mut self, index: usize) {
        let b = &self.boxes[index];
        let (Some(min), Some(max)) = (b.canvas.min, b.canvas.max) else {
            // an empty picture is still where the line starts: the glue after it counts
            if !self.started {
                self.out.include(Point::new(self.x, self.baseline_y));
            }
            self.started = true;
            return;
        };
        let d = Point::new(self.x - min.x, self.baseline_y - b.baseline);
        for item in &b.canvas.items {
            let mut it = item.clone();
            it.translate(d);
            self.out.items.push(it);
        }
        self.out.named.extend(b.canvas.named.iter().map(|(n, p)| (n.clone(), p.plus(d))));
        self.out.origin.get_or_insert(d);
        // (what glue moved left of the line's start, `\hskip-1cm`, is off the page)
        self.out.include(Point::new((min.x + d.x).max(0.0), min.y + d.y));
        self.out.include(max.plus(d));
        self.x += max.x - min.x;
        self.started = true;
    }
}

fn place_lines(page: &Placed) -> Canvas {
    let mut out = Canvas { margin: page.margin, ..Canvas::default() };
    let mut baseline_y = 0.0;
    let mut previous_depth: Option<f64> = None;
    let has_picture = page.layout.iter().any(|x| matches!(x, Layout::Picture(_)));
    // the glue waiting above the next line (that of lines not shown too)
    let mut skip = 0.0f64;
    let measured = Measured { boxes: page.boxes, labels: page.labels };
    for line in page.lines {
        skip = skip.max(line.skip);
        if !line_shown(line, has_picture) {
            continue;
        }
        let size = line_size(line, page);
        if let Some(d) = previous_depth {
            let gap = BASELINE_SKIP - d - size.height;
            baseline_y -= skip + if gap >= 0.0 { BASELINE_SKIP } else { d + LINE_SKIP + size.height };
        }
        // (glue at the top of the box is dropped)
        skip = 0.0;
        let mut pen = Pen { out: &mut out, boxes: page.boxes, labels: page.labels, x: centered_x(line, page.hsize, &measured), baseline_y, started: false };
        for item in &line.items {
            pen.paint(item);
        }
        previous_depth = Some(size.depth);
    }
    // glue at the end of a `\vbox` (that below a `center`) is part of its height; the page drops it
    if let (true, Some(d)) = (page.vbox && skip > 0.0, previous_depth) {
        out.include(Point::new(0.0, baseline_y - d - skip));
    }
    // a `\vbox` of a known `\hsize` is that wide: what sticks out of it is cut off with the page
    if let (Some(w), Some(lo), Some(hi)) = (page.hsize, out.min.as_mut(), out.max.as_mut()) {
        lo.x = 0.0;
        hi.x = w;
    }
    out
}

/// Lays the pictures out as TeX sets boxes in a paragraph: side by side on a baseline, a space
/// where the source had one, a new line at each paragraph; then one canvas with all of them.
pub(super) fn typeset(boxes: &[PictureBox], layout: &[Layout], words: &[(String, String)], margin: f64) -> Canvas {
    let labels = word_labels(words);
    let (table_boxes, layout) = tables(boxes, layout, &labels);
    let gathered = gather_lines(&layout);
    let measured = Measured { boxes: &table_boxes, labels: &labels };
    let lines = match gathered.hsize {
        Some(w) => break_wide(gathered.lines, w, &measured),
        None => gathered.lines,
    };
    place_lines(&Placed { boxes: &table_boxes, labels: &labels, layout: &layout, lines: &lines, hsize: gathered.hsize, vbox: gathered.vbox, margin })
}

struct Extent {
    width: f64,
    height: f64,
    depth: f64,
}

fn measure_item(all: &[PictureBox], labels: &[crate::scene::Label], item: &Layout) -> Option<Extent> {
    match item {
        Layout::Picture(i) => {
            let b = &all[*i];
            let (lo, hi) = b.canvas.min.zip(b.canvas.max)?;
            Some(Extent { width: hi.x - lo.x, height: hi.y - b.baseline, depth: b.baseline - lo.y })
        }
        Layout::Word(i) => Some(Extent { width: labels[*i].width(), height: labels[*i].height(), depth: labels[*i].depth() }),
        Layout::Skip(w) => Some(Extent { width: *w, height: 0.0, depth: 0.0 }),
        Layout::Space => Some(Extent { width: SPACE, height: 0.0, depth: 0.0 }),
        _ => None,
    }
}

fn cell_extent(all: &[PictureBox], labels: &[crate::scene::Label], cell: &[Layout]) -> Extent {
    cell.iter().filter_map(|l| measure_item(all, labels, l)).fold(Extent { width: 0.0, height: 0.0, depth: 0.0 }, |acc, e| Extent {
        width: acc.width + e.width,
        height: acc.height.max(e.height),
        depth: acc.depth.max(e.depth),
    })
}

fn trim_cell(cell: &mut Vec<Layout>) {
    while cell.last() == Some(&Layout::Space) {
        cell.pop();
    }
    let first = cell.iter().position(|l| *l != Layout::Space).unwrap_or(cell.len());
    cell.drain(..first);
}

fn drop_blank_last_row(rows: &mut Vec<Vec<Vec<Layout>>>) {
    // `\\` before `\end{tabular}` ends the last row: nothing after it
    if rows.len() > 1 && rows.last().is_some_and(|r| r.iter().all(Vec::is_empty)) {
        rows.pop();
    }
}

/// The rows of one `tabular`, and the `Width` / `Vbox` it must keep outside the table.
fn read_rows(layout: &[Layout], i: &mut usize, out: &mut Vec<Layout>) -> Vec<Vec<Vec<Layout>>> {
    let mut rows: Vec<Vec<Vec<Layout>>> = vec![vec![Vec::new()]];
    *i += 1;
    while *i < layout.len() && !matches!(layout[*i], Layout::TabEnd) {
        match layout[*i] {
            Layout::Cell => rows.last_mut().expect("a row").push(Vec::new()),
            Layout::Row => rows.push(vec![Vec::new()]),
            Layout::Picture(_) | Layout::Word(_) | Layout::Skip(_) | Layout::Space => rows.last_mut().expect("a row").last_mut().expect("a cell").push(layout[*i]),
            // the line's width stays known
            Layout::Width(w) if !out.contains(&Layout::Width(w)) => out.push(Layout::Width(w)),
            Layout::Vbox if !out.contains(&Layout::Vbox) => out.push(Layout::Vbox),
            _ => {}
        }
        *i += 1;
    }
    *i += 1;
    for cell in rows.iter_mut().flatten() {
        trim_cell(cell);
    }
    drop_blank_last_row(&mut rows);
    rows
}

fn column_widths(rows: &[Vec<Vec<Layout>>], all: &[PictureBox], labels: &[crate::scene::Label]) -> Vec<f64> {
    let n = rows.iter().map(Vec::len).max().unwrap_or(0);
    (0..n).map(|k| rows.iter().filter_map(|r| r.get(k)).map(|c| cell_extent(all, labels, c).width).fold(0.0, f64::max)).collect()
}

fn column_align(code: u32, column: usize) -> u32 {
    let ncols = (code & 0xff) as usize;
    if column < ncols {
        (code >> (8 + 2 * column)) & 3
    } else {
        0
    }
}

struct Grid<'a> {
    rows: &'a [Vec<Vec<Layout>>],
    widths: &'a [f64],
    code: u32,
    all: &'a [PictureBox],
    labels: &'a [crate::scene::Label],
}

struct CellAt {
    x: f64,
    base: f64,
}

fn paint_picture(canvas: &mut Canvas, b: &PictureBox, at: CellAt) -> f64 {
    let Some((lo, hi)) = b.canvas.min.zip(b.canvas.max) else { return 0.0 };
    let shift = Point::new(at.x - lo.x, at.base - b.baseline);
    for item in &b.canvas.items {
        let mut it = item.clone();
        it.translate(shift);
        canvas.items.push(it);
    }
    canvas.include(lo.plus(shift));
    canvas.include(hi.plus(shift));
    hi.x - lo.x
}

fn paint_word(canvas: &mut Canvas, label: &crate::scene::Label, at: CellAt) -> f64 {
    canvas.items.push(Item::Text { label: label.clone(), place: crate::scene::Transform::translation(at.x, at.base), opacity: 1.0 });
    canvas.include(Point::new(at.x, at.base - label.depth()));
    canvas.include(Point::new(at.x + label.width(), at.base + label.height()));
    label.width()
}

fn paint_grid(grid: &Grid) -> (Canvas, f64) {
    let mut canvas = Canvas::default();
    let mut y = 0.0;
    for row in grid.rows {
        let (h, d) = row.iter().map(|c| cell_extent(grid.all, grid.labels, c)).fold((STRUT_HEIGHT, STRUT_DEPTH), |(h, d), e| (h.max(e.height), d.max(e.depth)));
        let base = y - h;
        let mut x = 0.0;
        for (k, w) in grid.widths.iter().enumerate() {
            let cell = row.get(k).map_or(&[][..], Vec::as_slice);
            let room = w - cell_extent(grid.all, grid.labels, cell).width;
            let mut cx = x
                + TABCOLSEP
                + match column_align(grid.code, k) {
                    2 => room,
                    1 => room / 2.0,
                    _ => 0.0,
                };
            for l in cell {
                match l {
                    Layout::Picture(p) => cx += paint_picture(&mut canvas, &grid.all[*p], CellAt { x: cx, base }),
                    Layout::Word(wd) => cx += paint_word(&mut canvas, &grid.labels[*wd], CellAt { x: cx, base }),
                    Layout::Skip(s) => cx += s,
                    Layout::Space => cx += SPACE,
                    _ => {}
                }
            }
            x += w + 2.0 * TABCOLSEP;
        }
        // the table's box: its columns' room and its rows' struts
        canvas.include(Point::new(0.0, y));
        canvas.include(Point::new(x, base - d));
        y = base - d;
    }
    (canvas, y)
}

/// The pictures and words of each `tabular` set in its cells, as one box: its rows one under the
/// other (struts making them tall enough), its columns as wide as their widest cell with
/// `\tabcolsep` on both sides, the whole centered on the math axis. Returns the boxes (the
/// pictures', then the tables') and the layout with each table as one of them.
fn tables(boxes: &[PictureBox], layout: &[Layout], labels: &[crate::scene::Label]) -> (Vec<PictureBox>, Vec<Layout>) {
    let mut all: Vec<PictureBox> = boxes.to_vec();
    if !layout.iter().any(|l| matches!(l, Layout::TabStart(_))) {
        return (all, layout.to_vec());
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < layout.len() {
        let Layout::TabStart(code) = layout[i] else {
            out.push(layout[i]);
            i += 1;
            continue;
        };
        let rows = read_rows(layout, &mut i, &mut out);
        let widths = column_widths(&rows, &all, labels);
        let (canvas, y) = paint_grid(&Grid { rows: &rows, widths: &widths, code, all: &all, labels });
        // centered on the math axis
        let baseline = y / 2.0 - MATH_AXIS;
        out.push(Layout::Picture(all.len()));
        all.push(PictureBox { canvas, baseline });
    }
    (all, out)
}
