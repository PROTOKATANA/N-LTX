//! Matrices, cases, alignments and `array`, with TeX's measures:
//! rows with a strut of 0.84 em above and 0.36 em below (`\arraystretch` 1, `\baselineskip` 12pt),
//! columns separated by 2 × `\arraycolsep` (1 em), `\jot` of 0.3 em between alignment rows.
//! `\arraystretch` makes the strut of `array` and of the matrices taller, as in LaTeX; the dashed
//! rules (`:`, `\hdashline`) follow arydshln: dashes and gaps of 4pt.

use crate::tree::{Node, TableKind, TableRule};
use crate::Error;

use super::arrow::LabeledArrow;
use super::{em, k, stretch, Context, Item, Level, MathBox, Style};

type R<T> = Result<T, Error>;

/// The rules of a table and the stretch of its rows.
pub struct Lines<'a> {
    /// Boundaries with a `|` rule (0 = before the first column).
    pub verticals: &'a [usize],
    /// Boundaries with a dashed `:` rule.
    pub dashed: &'a [usize],
    pub horizontals: &'a [TableRule],
    /// `\arraystretch`.
    pub stretch: f32,
}

/// A table as the tree hands it to the layout: shape, cells, rules and the text between columns.
pub struct Table<'a> {
    pub kind: TableKind,
    pub rows: &'a [Vec<Node>],
    pub columns: &'a [char],
    pub lines: Lines<'a>,
    pub between: &'a [(usize, Node)],
    pub intertexts: &'a [usize],
}

/// A composed cell: starting column, how many columns it spans, its own alignment.
struct Cell<'a> {
    column: usize,
    span: usize,
    alignment: Option<char>,
    math_box: MathBox,
    node: &'a Node,
}

/// The style of a cell, and the outer style intertext is set in.
struct CellStyles {
    cell: Style,
    outer: Style,
}

struct Draft<'a> {
    table: &'a Table<'a>,
    cells: Vec<Vec<Cell<'a>>>,
    ncol: usize,
    cell_style: Style,
    outer: Style,
}

/// Column widths, row baselines and the rules, once every cell has been composed.
struct Grid<'a> {
    kind: TableKind,
    columns: &'a [char],
    verticals: &'a [usize],
    dashed: &'a [usize],
    horizontals: &'a [TableRule],
    between: &'a [(usize, Node)],
    intertexts: &'a [usize],
    stretch: f32,
    cells: Vec<Vec<Cell<'a>>>,
    ncol: usize,
    cell_style: Style,
    outer: Style,
    widths: Vec<f32>,
    heights: Vec<f32>,
    depths: Vec<f32>,
    boundary: Vec<f32>,
    boundary_content: Vec<Option<MathBox>>,
    x_col: Vec<f32>,
    x_boundary: Vec<f32>,
    bases: Vec<f32>,
    y_rule: Vec<f32>,
    total_width: f32,
    total_height: f32,
    thickness: f32,
    jot: f32,
    border: Option<(MathBox, MathBox)>,
    top: f32,
}

pub fn compose(table: &Table, e: Style, ctx: &mut Context) -> R<MathBox> {
    let cell_style = cell_style(table.kind, e);
    let ncol = column_count(table);
    let cells = compose_cells(table, CellStyles { cell: cell_style, outer: e }, ctx)?;
    let grid = Grid::measure(Draft { table, cells, ncol, cell_style, outer: e }, ctx)?;
    grid.finish(ctx)
}

fn cell_style(kind: TableKind, e: Style) -> Style {
    use TableKind::*;
    match kind {
        SmallMatrix => Style { level: Level::Script, cramped: e.cramped, font: e.font },
        Substack => e,
        Aligned | Centered => Style { level: Level::Display, cramped: false, font: e.font },
        _ => e.not_display(),
    }
}

fn column_count(table: &Table) -> usize {
    let span = |node: &Node| {
        if let Node::Multicolumn { columns, .. } = node {
            *columns
        } else {
            1
        }
    };
    table.rows.iter().enumerate().filter(|(i, _)| !table.intertexts.contains(i)).map(|(_, l)| l.iter().map(span).sum::<usize>()).max().unwrap_or(0).max(table.columns.len()).max(1)
}

fn compose_cells<'a>(table: &Table<'a>, styles: CellStyles, ctx: &mut Context) -> R<Vec<Vec<Cell<'a>>>> {
    let mut cells = Vec::new();
    for (i, row) in table.rows.iter().enumerate() {
        let mut c = Vec::new();
        let mut j = 0;
        for node in row {
            let initial_ord = table.kind == TableKind::Aligned && j % 2 == 1 && !table.intertexts.contains(&i);
            let (n, alignment, content) = match node {
                Node::Multicolumn { columns, alignment, base } => (*columns, Some(*alignment), base.as_ref()),
                other => (1, None, other),
            };
            let math_box = if table.intertexts.contains(&i) {
                super::compose(content, styles.outer.not_display(), ctx)?
            } else {
                super::row::compose_cell(content, styles.cell, ctx, initial_ord)?
            };
            c.push(Cell { column: j, span: n, alignment, math_box, node: content });
            j += n;
        }
        cells.push(c);
    }
    Ok(cells)
}

impl<'a> Grid<'a> {
    fn measure(draft: Draft<'a>, ctx: &mut Context) -> R<Grid<'a>> {
        let mut grid = Grid::from_draft(draft);
        grid.row_extents();
        grid.column_widths();
        grid.stretch_diagram_columns();
        grid.prepare_rules();
        grid.border = grid.outer_delimiters()?;
        grid.boundaries(ctx)?;
        grid.column_positions();
        grid.widen_spans();
        grid.widen_intertexts();
        grid.row_positions();
        Ok(grid)
    }

    fn from_draft(draft: Draft<'a>) -> Grid<'a> {
        let Draft { table, cells, ncol, cell_style, outer } = draft;
        let rows = cells.len();
        Grid {
            kind: table.kind,
            columns: table.columns,
            verticals: table.lines.verticals,
            dashed: table.lines.dashed,
            horizontals: table.lines.horizontals,
            between: table.between,
            intertexts: table.intertexts,
            stretch: table.lines.stretch,
            cells,
            ncol,
            cell_style,
            outer,
            widths: vec![0.0; ncol],
            heights: Vec::new(),
            depths: Vec::new(),
            boundary: vec![0.0; ncol + 1],
            boundary_content: vec![None; ncol + 1],
            x_col: vec![0.0; ncol],
            x_boundary: vec![0.0; ncol + 1],
            bases: Vec::new(),
            y_rule: vec![0.0; rows + 1],
            total_width: 0.0,
            total_height: 0.0,
            thickness: 0.0,
            jot: 0.0,
            border: None,
            top: 0.0,
        }
    }

    fn row_extents(&mut self) {
        use TableKind::*;
        let scale = self.cell_style.scale();
        let (strut_height, strut_depth) = match self.kind {
            SmallMatrix => (em(self.cell_style, 0.85), em(self.cell_style, 0.35)),
            Substack => (em(self.cell_style, 0.6), em(self.cell_style, 0.2)),
            // alignments space their rows with \jot, and a diagram is not an array: \arraystretch is not theirs
            Aligned | Centered | Diagram => (0.84 * 1000.0 * scale, 0.36 * 1000.0 * scale),
            _ => (0.84 * 1000.0 * scale * self.stretch, 0.36 * 1000.0 * scale * self.stretch),
        };
        for row in &self.cells {
            self.heights.push(row.iter().map(|c| c.math_box.height).fold(strut_height, f32::max));
            self.depths.push(row.iter().map(|c| c.math_box.depth).fold(strut_depth, f32::max));
        }
    }

    fn column_widths(&mut self) {
        for (i, row) in self.cells.iter().enumerate() {
            if self.intertexts.contains(&i) {
                continue;
            }
            for c in row.iter().filter(|c| c.span == 1) {
                self.widths[c.column] = self.widths[c.column].max(c.math_box.width);
            }
        }
    }

    /// Commutative diagram arrows are at least 2.5 em long (`\minCDarrowwidth`).
    fn stretch_diagram_columns(&mut self) {
        if self.kind != TableKind::Diagram {
            return;
        }
        for (j, l) in self.widths.iter_mut().enumerate() {
            if j % 2 == 1 {
                *l = l.max(em(self.outer, 2.5));
            }
        }
    }

    fn prepare_rules(&mut self) {
        self.jot = if matches!(self.kind, TableKind::Aligned | TableKind::Centered) { em(self.outer, 0.3) } else { 0.0 };
        self.thickness = em(self.outer, 0.04);
    }

    /// `\bordermatrix`: parentheses around the rows and columns from the second on.
    fn outer_delimiters(&self) -> R<Option<(MathBox, MathBox)>> {
        if self.kind == TableKind::BorderMatrix && self.cells.len() > 1 && self.ncol > 1 {
            let core: f32 = (1..self.cells.len()).map(|i| self.heights[i] + self.depths[i]).sum();
            Ok(Some((stretch::delimiter('(', core, self.outer)?, stretch::delimiter(')', core, self.outer)?)))
        } else {
            Ok(None)
        }
    }

    fn boundaries(&mut self, ctx: &mut Context) -> R<()> {
        use TableKind::*;
        let (kind, outer) = (self.kind, self.outer);
        let half_sep = match kind {
            Array => em(outer, 0.5),
            Cases | RCases => em(outer, 0.5),
            Aligned => 0.0,
            SmallMatrix => em(outer, 0.2),
            Substack => 0.0,
            Diagram => em(outer, 0.25),
            _ => em(outer, 0.5),
        };
        let ncol = self.ncol;
        for (j, f) in self.boundary.iter_mut().enumerate() {
            let edge = j == 0 || j == ncol;
            *f = match kind {
                Array => 2.0 * half_sep - if edge { half_sep } else { 0.0 },
                Aligned => {
                    if edge {
                        0.0
                    } else if j % 2 == 0 {
                        em(outer, 2.0)
                    } else {
                        0.0
                    }
                }
                _ => {
                    if edge {
                        0.0
                    } else {
                        2.0 * half_sep
                    }
                }
            };
        }
        let between = self.between;
        let cell_style = self.cell_style;
        for (j, node) in between {
            if *j <= ncol {
                let c = super::compose(node, cell_style, ctx)?;
                self.boundary[*j] = c.width;
                self.boundary_content[*j] = Some(c);
            }
        }
        let (verticals, dashed, thickness) = (self.verticals, self.dashed, self.thickness);
        for j in verticals.iter().chain(dashed) {
            if *j <= ncol {
                self.boundary[*j] += thickness;
            }
        }
        let border_widths = self.border.as_ref().map(|(open, close)| (open.width, close.width));
        if let Some((open_w, close_w)) = border_widths {
            self.boundary[1] = open_w + em(self.outer, 0.4);
            self.boundary[ncol] += close_w;
        }
        Ok(())
    }

    fn column_positions(&mut self) {
        let mut x = 0.0;
        for j in 0..self.ncol {
            self.x_boundary[j] = x;
            x += self.boundary[j];
            self.x_col[j] = x;
            x += self.widths[j];
        }
        self.x_boundary[self.ncol] = x;
        self.total_width = x + self.boundary[self.ncol];
    }

    /// Multi-column cells that do not fit widen the last column they span.
    fn widen_spans(&mut self) {
        let ncol = self.ncol;
        for row in &self.cells {
            for c in row.iter().filter(|c| c.span > 1) {
                let end = (c.column + c.span).min(ncol) - 1;
                let space = self.x_col[end] + self.widths[end] - self.x_col[c.column.min(ncol - 1)];
                if c.math_box.width > space {
                    let extra = c.math_box.width - space;
                    self.widths[end] += extra;
                    for x in self.x_col.iter_mut().skip(end + 1) {
                        *x += extra;
                    }
                    for f in self.x_boundary.iter_mut().skip(end + 1) {
                        *f += extra;
                    }
                    self.total_width += extra;
                }
            }
        }
    }

    fn widen_intertexts(&mut self) {
        for (i, row) in self.cells.iter().enumerate() {
            if self.intertexts.contains(&i) {
                if let Some(c) = row.first() {
                    self.total_width = self.total_width.max(c.math_box.width);
                }
            }
        }
    }

    fn row_positions(&mut self) {
        let horizontals = self.horizontals;
        let draws_rule = |i: usize| horizontals.iter().any(|t| t.row == i);
        let mut y = 0.0;
        for i in 0..self.cells.len() {
            self.y_rule[i] = y;
            if draws_rule(i) {
                y -= self.thickness;
            }
            if i > 0 {
                y -= self.jot;
            }
            y -= self.heights[i];
            self.bases.push(y);
            y -= self.depths[i];
        }
        let rows = self.cells.len();
        self.y_rule[rows] = y;
        if draws_rule(rows) {
            y -= self.thickness;
        }
        self.total_height = -y;
        // center on the math axis
        let axis = k(self.outer, |c| c.axis_height());
        self.top = axis + self.total_height / 2.0;
    }

    fn alignment(&self, j: usize) -> char {
        use TableKind::*;
        match self.kind {
            Array => self.columns.get(j).copied().unwrap_or('c'),
            Cases | RCases => 'l',
            Aligned => {
                if j.is_multiple_of(2) {
                    'r'
                } else {
                    'l'
                }
            }
            Substack => self.columns.first().copied().unwrap_or('c'),
            _ => self.columns.first().copied().unwrap_or('c'),
        }
    }

    fn finish(self, ctx: &mut Context) -> R<MathBox> {
        let mut body = MathBox::space(self.total_width);
        self.place_cells(&mut body, ctx)?;
        self.place_between(&mut body);
        self.draw_vertical_rules(&mut body);
        self.draw_horizontal_rules(&mut body);
        self.place_border(&mut body);
        body.height = self.top;
        body.depth = self.total_height - self.top;
        wrap_matrix(self.kind, body, self.outer)
    }

    fn place_cells(&self, body: &mut MathBox, ctx: &mut Context) -> R<()> {
        for (i, row) in self.cells.iter().enumerate() {
            for c in row {
                if self.intertexts.contains(&i) {
                    body.place(&c.math_box, 0.0, self.top + self.bases[i]);
                    continue;
                }
                let j = c.column.min(self.ncol - 1);
                let end = (c.column + c.span).min(self.ncol) - 1;
                let space = self.x_col[end] + self.widths[end] - self.x_col[j];
                // horizontal diagram arrows stretch to the column width
                let stretched = match (c.node, self.kind) {
                    (Node::Arrow { symbol, above, below }, TableKind::Diagram) => Some(super::arrow::compose_wide_arrow(
                        &LabeledArrow { symbol: *symbol, above: above.as_deref(), below: below.as_deref(), minimum: space },
                        self.cell_style,
                        ctx,
                    )?),
                    _ => None,
                };
                let math_box = stretched.as_ref().unwrap_or(&c.math_box);
                let leftover = space - math_box.width;
                let dx = match c.alignment.unwrap_or_else(|| self.alignment(j)) {
                    'l' => 0.0,
                    'r' => leftover,
                    _ => leftover / 2.0,
                };
                body.place(math_box, self.x_col[j] + dx, self.top + self.bases[i]);
            }
        }
        Ok(())
    }

    fn place_between(&self, body: &mut MathBox) {
        for (j, c) in self.boundary_content.iter().enumerate() {
            if let Some(c) = c {
                for (i, _) in self.cells.iter().enumerate() {
                    body.place(c, self.x_boundary[j], self.top + self.bases[i]);
                }
            }
        }
    }

    /// Vertical rules per table row: a merged cell (`\multicolumn`) interrupts the rule.
    fn draw_vertical_rules(&self, body: &mut MathBox) {
        let (dash, gap) = (em(self.outer, 0.4), em(self.outer, 0.4));
        for (j, dashed) in self.verticals.iter().map(|j| (j, false)).chain(self.dashed.iter().map(|j| (j, true))) {
            if *j > self.ncol {
                continue;
            }
            let xv = self.rule_x(*j);
            for (i, row) in self.cells.iter().enumerate() {
                let crosses = self.intertexts.contains(&i) || row.iter().any(|c| c.span > 1 && c.column < *j && *j < c.column + c.span);
                if crosses {
                    continue;
                }
                // from the row's top rule to the next one, in the body's coordinates
                let (low, length) = (self.top + self.y_rule[i + 1], self.y_rule[i] - self.y_rule[i + 1]);
                if dashed {
                    for (from, length) in dashes(length, dash, gap) {
                        body.items.push(Item::Rule { x: xv, y: low + from, width: self.thickness, height: length, color: None });
                    }
                } else {
                    body.items.push(Item::Rule { x: xv, y: low, width: self.thickness, height: length, color: None });
                }
            }
        }
    }

    /// Between columns the rule sits in the middle of the two `\arraycolsep`; on the outside, at
    /// the edge, the `\arraycolsep` inside it (as LaTeX's array puts it).
    fn rule_x(&self, j: usize) -> f32 {
        if self.boundary_content[j].is_some() || j == 0 {
            self.x_boundary[j]
        } else if j == self.ncol {
            self.x_boundary[j] + self.boundary[j] - self.thickness
        } else {
            self.x_boundary[j] + self.boundary[j] / 2.0 - self.thickness / 2.0
        }
    }

    fn draw_horizontal_rules(&self, body: &mut MathBox) {
        let (dash, gap) = (em(self.outer, 0.4), em(self.outer, 0.4));
        let ncol = self.ncol;
        for t in self.horizontals {
            let from = t.from.min(ncol.saturating_sub(1));
            let to = t.to.min(ncol - 1);
            let (x0, x1) = if t.to == usize::MAX && t.from == 0 {
                (0.0, self.total_width)
            } else {
                (self.x_col[from] - self.boundary[from] / 2.0, self.x_col[to] + self.widths[to] + self.boundary[to + 1] / 2.0)
            };
            let yt = self.top + self.y_rule[t.row.min(self.cells.len())] - self.thickness;
            let (x0, width) = (x0.max(0.0), (x1 - x0.max(0.0)).min(self.total_width));
            if t.dashed {
                for (from, length) in dashes(width, dash, gap) {
                    body.items.push(Item::Rule { x: x0 + from, y: yt, width: length, height: self.thickness, color: None });
                }
            } else {
                body.items.push(Item::Rule { x: x0, y: yt, width, height: self.thickness, color: None });
            }
        }
    }

    fn place_border(&self, body: &mut MathBox) {
        let Some((open, close)) = &self.border else {
            return;
        };
        // center the parentheses on the core (rows 2 onward)
        let top = self.top + self.y_rule[1];
        let low = top + self.y_rule[self.cells.len()];
        let axis = k(self.outer, |c| c.axis_height());
        let dy = (top + low) / 2.0 - axis;
        body.place(open, self.x_col[1] - open.width - em(self.outer, 0.1), dy);
        body.place(close, self.x_col[self.ncol - 1] + self.widths[self.ncol - 1] + em(self.outer, 0.05), dy);
    }
}

fn wrap_matrix(kind: TableKind, body: MathBox, e: Style) -> R<MathBox> {
    use TableKind::*;
    let (open, close) = match kind {
        PMatrix => ('(', Some(')')),
        BMatrix => ('[', Some(']')),
        BraceMatrix => ('{', Some('}')),
        VMatrix => ('|', Some('|')),
        VVMatrix => ('‖', Some('‖')),
        Cases => ('{', None),
        RCases => ('}', None),
        _ => return Ok(body),
    };
    let axis = k(e, |c| c.axis_height());
    let half = (body.height - axis).max(body.depth + axis);
    let target = (2.0 * half * 0.901).max(2.0 * half - em(e, 0.5));
    if kind == RCases {
        // the mirror of cases: content, a little space, the brace on the right
        let mut math_box = MathBox::space(em(e, 0.12));
        math_box.append(&body);
        math_box.append(&MathBox::space(em(e, 0.17)));
        math_box.append(&stretch::delimiter(open, target, e)?);
        return Ok(math_box);
    }
    let mut math_box = stretch::delimiter(open, target, e)?;
    if kind == Cases {
        math_box.append(&MathBox::space(em(e, 0.17)));
    }
    math_box.append(&body);
    match close {
        Some(c) => math_box.append(&stretch::delimiter(c, target, e)?),
        None => math_box.append(&MathBox::space(em(e, 0.12))),
    }
    Ok(math_box)
}

/// The dashes of a dashed rule `length` long, as (start, length): as many as fit, the gaps
/// adjusted so the rule starts and ends with a dash.
fn dashes(length: f32, dash: f32, gap: f32) -> Vec<(f32, f32)> {
    if length <= dash {
        return vec![(0.0, length.max(0.0))];
    }
    let n = ((length + gap) / (dash + gap)).round().max(2.0) as usize;
    let gap = (length - n as f32 * dash) / (n - 1) as f32;
    if gap <= 0.0 {
        return vec![(0.0, length)];
    }
    (0..n).map(|i| (i as f32 * (dash + gap), dash)).collect()
}
