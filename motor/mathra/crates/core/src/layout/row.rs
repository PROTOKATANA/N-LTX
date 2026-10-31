//! Atom classes and the space TeX puts between them.

use crate::locale::Locale;
use crate::rules::{self, Registry};
use crate::tree::{Class, Node, TableKind, Variant};
use crate::Error;

use super::{em, Context, MathBox, Style};

type R<T> = Result<T, Error>;

/// Atom class of a node. `None` for explicit spaces, which do not count as neighbors.
pub fn class_of(node: &Node) -> Option<Class> {
    Some(match node {
        Node::Space { .. } => return None,
        Node::Function { .. } | Node::LargeOperator { .. } => Class::Op,
        Node::Operator { class, .. } | Node::FixedDelimiter { class, .. } | Node::ForcedClass { class, .. } => *class,
        Node::Fraction { .. } | Node::Delimited { .. } => Class::Inner,
        Node::Table { shape: kind, .. } if !matches!(kind, TableKind::Array | TableKind::Aligned | TableKind::Centered | TableKind::Substack | TableKind::Diagram) => Class::Inner,
        Node::Multicolumn { base, .. } | Node::Raised { base, .. } | Node::Scaled { base, .. } | Node::Smashed { base, .. } | Node::Unit { base, .. } => return class_of(base),
        // the text style's branch: a class does not know the style
        Node::Choice { branches } => return branches.get(1).and_then(class_of),
        Node::Scripts { base, .. } => return class_of(base),
        Node::Arrow { .. } => Class::Rel,
        Node::Colored { base, .. } | Node::Style { base, .. } => match base.as_ref() {
            Node::Row { children } if children.len() == 1 => return class_of(&children[0]),
            Node::Row { .. } => Class::Ord,
            other => return class_of(other),
        },
        _ => Class::Ord,
    })
}

/// The row whose classes are being decided, including the invisible Ord an alignment cell may put in front.
struct ClassRow<'a> {
    children: &'a [Node],
    initial_ord: bool,
}

fn neighbor_before(classes: &[Option<Class>], index: usize, initial_ord: bool) -> Option<Class> {
    classes[..index].iter().rev().flatten().next().copied().or(if initial_ord { Some(Class::Ord) } else { None })
}

fn neighbor_after(classes: &[Option<Class>], index: usize) -> Option<Class> {
    classes[index + 1..].iter().flatten().next().copied()
}

/// L-INT-1 (pt-BR, fr…): ]a, b[ is an open interval, so ] opens and [ closes.
fn mark_open_intervals(row: &ClassRow, classes: &mut [Option<Class>], registry: &mut Option<&mut Registry>) {
    for i in 0..row.children.len() {
        let Node::Operator { symbol, .. } = &row.children[i] else {
            continue;
        };
        let before = neighbor_before(classes, i, row.initial_ord);
        let after = neighbor_after(classes, i);
        let no_operand = matches!(before, None | Some(Class::Bin | Class::Op | Class::Rel | Class::Open | Class::Punct));
        if *symbol == ']' && no_operand && after.is_some() {
            classes[i] = Some(Class::Open);
            if let Some(r) = registry.as_deref_mut() {
                r.note(rules::L_INT_1, "']' at the start read as an interval opening");
            }
        } else if *symbol == '['
            && matches!(before, Some(Class::Ord | Class::Close | Class::Inner))
            && matches!(after, None | Some(Class::Rel | Class::Punct | Class::Close | Class::Bin))
            && row.children[..i].iter().any(|n| matches!(n, Node::Operator { symbol: ']', .. }))
        {
            classes[i] = Some(Class::Close);
            if let Some(r) = registry.as_deref_mut() {
                r.note(rules::L_INT_1, "'[' at the end read as an interval closing");
            }
        }
    }
}

/// L-ESP-2 (TeXbook, Appendix G, rules 5 and 6): a Bin without operands on both sides becomes Ord.
fn mark_unary_bins(row: &ClassRow, classes: &mut [Option<Class>], registry: &mut Option<&mut Registry>) {
    for i in 0..classes.len() {
        if classes[i] != Some(Class::Bin) {
            continue;
        }
        let before = neighbor_before(classes, i, row.initial_ord);
        let after = neighbor_after(classes, i);
        let no_left = matches!(before, None | Some(Class::Bin | Class::Op | Class::Rel | Class::Open | Class::Punct));
        let no_right = matches!(after, None | Some(Class::Rel | Class::Close | Class::Punct));
        if no_left || no_right {
            classes[i] = Some(Class::Ord);
            if let (Some(r), Node::Operator { symbol, .. }) = (registry.as_deref_mut(), &row.children[i]) {
                r.note(rules::L_ESP_2, format!("'{symbol}' is unary: no operator spacing"));
            }
        }
    }
}

/// Classes after the context rules (L-INT-1 and L-ESP-2).
pub fn classes(children: &[Node], locale: Locale, mut registry: Option<&mut Registry>, initial_ord: bool) -> Vec<Option<Class>> {
    let mut found: Vec<Option<Class>> = children.iter().map(class_of).collect();
    let row = ClassRow { children, initial_ord };
    if locale.open_interval() {
        mark_open_intervals(&row, &mut found, &mut registry);
    }
    mark_unary_bins(&row, &mut found, &mut registry);
    found
}

/// L-ESP-1: space between atoms in mu (1/18 em). TeXbook table, ch. 18.
/// Negative values are the ones TeX puts in parentheses: they vanish in script style.
fn mu_space(left: Class, right: Class, index: bool) -> f32 {
    use Class::*;
    let idx = |c: Class| match c {
        Ord => 0,
        Op => 1,
        Bin => 2,
        Rel => 3,
        Open => 4,
        Close => 5,
        Punct => 6,
        Inner => 7,
    };
    const T: [[i8; 8]; 8] = [
        // Ord Op Bin Rel Open Close Punct Inner
        [0, 1, -2, -3, 0, 0, 0, -1],     // Ord
        [1, 1, 0, -3, 0, 0, 0, -1],      // Op
        [-2, -2, 0, 0, -2, 0, 0, -2],    // Bin
        [-3, -3, 0, 0, -3, 0, 0, -3],    // Rel
        [0, 0, 0, 0, 0, 0, 0, 0],        // Open
        [0, 1, -2, -3, 0, 0, 0, -1],     // Close
        [-1, -1, 0, -1, -1, -1, -1, -1], // Punct
        [-1, 1, -2, -3, -1, 0, -1, -1],  // Inner
    ];
    let v = T[idx(left)][idx(right)];
    if v < 0 && index {
        return 0.0;
    }
    match v.abs() {
        1 => 3.0,
        2 => 4.0,
        3 => 5.0,
        _ => 0.0,
    }
}

pub(crate) fn compose_row(children: &[Node], e: Style, ctx: &mut Context, initial_ord: bool) -> R<MathBox> {
    if let ([single], false) = (children, initial_ord) {
        // the row of a single node is the node itself: keeps italic and accent point
        return super::compose(single, e, ctx);
    }
    let cl = classes(children, ctx.locale, Some(ctx.registry), initial_ord);
    let mu = em(e, 1.0 / 18.0);
    let mut math_box = MathBox::empty();
    let mut previous = if initial_ord { Some(Class::Ord) } else { None };
    let mut pending_italic = 0.0;
    for (node, class) in children.iter().zip(&cl) {
        let b = super::compose(node, e, ctx)?;
        if let (Some(a), Some(c)) = (previous, class) {
            math_box.append(&MathBox::space(mu_space(a, *c, e.is_script()) * mu));
        }
        // italic correction after a slanted letter, before whatever follows (Appendix G, rule 17)
        if pending_italic > 0.0 {
            math_box.append(&MathBox::space(pending_italic));
        }
        pending_italic = if matches!(node, Node::Identifier { variant: Variant::Italic | Variant::BoldItalic, .. }) { b.italic } else { 0.0 };
        math_box.append(&b);
        if class.is_some() {
            previous = *class;
        }
    }
    // and after the row's last letter: TeX ends the list with its kern
    if pending_italic > 0.0 {
        math_box.append(&MathBox::space(pending_italic));
    }
    math_box.single_glyph = false;
    Ok(math_box)
}

/// A math list on its own (the formula, a script, a numerator, a radicand): a slanted letter that
/// ends it keeps its italic correction (Appendix G, rule 17), as a row does after its last letter.
pub fn compose_list(node: &Node, e: Style, ctx: &mut Context) -> R<MathBox> {
    let mut math_box = super::compose(node, e, ctx)?;
    if ends_slanted(node) && math_box.italic > 0.0 {
        let italic = math_box.italic;
        math_box.append(&MathBox::space(italic));
        math_box.single_glyph = false;
    }
    Ok(math_box)
}

/// A slanted letter or word (`\mathit{diff}`), alone or in a row of one: the row adds no kern of its own.
fn ends_slanted(node: &Node) -> bool {
    match node {
        Node::Identifier { variant: Variant::Italic | Variant::BoldItalic, .. } => true,
        Node::Row { children } if children.len() == 1 => ends_slanted(&children[0]),
        _ => false,
    }
}

/// A cell: if it is a row, compose it with an invisible Ord in front (so `&=` gets relation spacing).
///
/// `initial_ord` stays a parameter of its own: it is the spacing mode of the cell, not a property of
/// the node, and the style and the context are the layout environment. Wrapping any of them together
/// would only rename the argument list.
pub(super) fn compose_cell(node: &Node, e: Style, ctx: &mut Context, initial_ord: bool) -> R<MathBox> {
    match node {
        Node::Row { children } => compose_row(children, e, ctx, initial_ord),
        other => compose_row(std::slice::from_ref(other), e, ctx, initial_ord),
    }
}
