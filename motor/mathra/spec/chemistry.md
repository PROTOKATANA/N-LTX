# Structural chemistry (L-CHE-2)

With the `chem` feature (on by default), mathra reads chemfig: structures, reaction schemes and mechanism arrows, inside any equation. Why this is inside the boundary is in `boundary.md`; the code is in `crates/core/src/chem`, the tests in `crates/core/tests/chemfig.rs`. A one-line formula (`\ce{H2SO4}`) is still mhchem (L-CHE-1), and a scheme hands its formulas and every label to the rest of mathra.

The whole construction becomes one node of the tree, `chemistry`, holding a scheme: compounds (a structure, or math), plus signs, arrows, and mechanism arrows between hooked atoms. A structure is a graph: atoms with their labels split into pieces, bonds with their kind and chemfig's direction, rings.

## Input

The reader takes, as written: `\chemfig[options]{…}`, `\schemestart … \schemestop`, `\chemname[dim]{…}{name}` (and `\chemname*`), and the `\chemmove{…}` right after any of them.

### chemfig bodies


| Construction | Reading |
|---|---|
| `H_3C`, `NH_3^+`, `O\ominus`, `{CH_3}` | an atom label, split into pieces at each capital letter (`H_3C` → `H_3`, `C`); braces keep a piece whole; a script takes the next character even if it is a bond sign (`^-`) |
| `-` `=` `~` | single, double, triple bond |
| `>` `<` · `>:` `<:` · `>|` `<|` | wedge, hashed wedge, hollow wedge; `>` is wide at the end, `<` at the start |
| `-[a,len,dep,arr,tikz]` | `a`: `n` × 45°, `:x` absolute, `::x` relative to the previous bond, empty keeps the previous direction; `len` a length factor; `dep`, `arr` the pieces (from 1) the bond leaves from and arrives at; `tikz`: `draw=none` hides the bond, `dash…`/`dotted` dashes it |
| `(…)` | a branch from the current atom; on a ring vertex its first bond points outward |
| `*n(…)`, `**n(…)` | a ring of n sides (3 to 40), with a circle for `**`; it may list fewer bonds than sides |
| a ring at a ring vertex | fused: it lies outside and shares the parent's next side, coming back along it |
| `?`, `?[name]`, `?[name,{=}]` | the first mark waits, the second of the same name draws a bond (single, or the kind given) |
| `\lewis{0:2.4|6o,X}` | label X with marks: direction digit 0–7, then `:` pair (default), `.` radical, `|` line, `o` vacancy |
| `\chemabove{X}{c}`, `\chembelow{X}{c}` | X with c above or below |
| `@{name}` | before an atom: the hook a mechanism arrow points at |

### Schemes

| Construction | Reading |
|---|---|
| `\schemestart … \schemestop` | markers, optional |
| `\chemfig[opts]{…}` | a structure; the options are ignored |
| `\ce{…}`, `$…$`, loose text | a formula or math, typeset by mathra (loose text goes through `\ce`) |
| `\+` | a plus sign; its spacing arguments are ignored |
| `\chemname{X}{name}`, `\chemname*`, `\chemname[dim]` | X with its name below |
| `\subscheme{…}` | its items, in place |
| `\arrow{type[above][below]}[angle,length]` | a reaction arrow; types `->` `<-` `<=>` `<->>` `<<->` `<->` `=>` `-/>` `0`; labels are LaTeX text with `$…$` and `\ce{…}`, or a `\chemfig{…}` drawn |
| `\chemmove[opts]{\draw[opts] (a)..controls +(θ:l) and +(θ:l)..(b); …}` | curved arrows between hooks; also `to[out=θ,in=θ]` and `to[bend left=θ]`; `-left to`, `harpoon`, `half`, `fishhook` make a one-electron arrow; lengths in cm, mm, pt, bp, in, ex, em |

## Reading

A structure is read by its molecular formula (C-FOR-1) and its rings (C-FOR-2), with its charge; a name given with `\chemname` comes first. A scheme is read in order: compounds, "plus", and each arrow by its meaning ("reacts to give", "in equilibrium with", "in resonance with", "is made from", "does not react to give"…) with its conditions in parentheses. A mechanism arrow is read as "curved arrow: an electron pair moves from X to Y", by the hooked atoms' labels. The phrases are `chem.*` in every locale file.

The linear text writes each structure by its formula (`C₆H₆`), the arrows by their sign with the labels in brackets. Braille transcribes the same row: formulas, `+`, arrows with their labels.


## Formula

### C-FOR-1 · Molecular formula from the graph

A vertex without a label is a carbon, with 4 minus the orders of its bonds in hydrogens (a dashed or invisible bond counts 0). A vertex of a ring drawn aromatic (`**n`) shares half of the delocalized bond on each side, so it counts one more. A label counts what it writes: element symbols (the 118 and D, T), subscripts, parentheses with a count (`(CH_2)_3`), charges (`^+`, `^{2-}`, `\oplus`, `\ominus`). Group abbreviations count as what they stand for and take precedence over the element they look like: Me, Et, Pr, iPr, Bu, tBu, Ph, Bn, Ac, Bz, Ts, Ms, Tf, Boc, Cbz, TMS. Anything else (R, R′, X, Nu) is kept as written. The formula is written in Hill order: C, H, then alphabetically; without carbon, all alphabetically.

### C-FOR-2 · Aromatic rings in the speech

A ring is read as aromatic when it is drawn with its circle, or when it is a six-membered ring whose atoms each carry exactly one double bond (a Kekulé drawing), or a five-membered ring like that except for one N, O or S.

## Layout

Everything is drawn at text size, 1000 units per em, with the items every equation uses (strokes, ellipses, glyphs: a curve is sampled into short strokes, a filled wedge is a fan of strokes); the drawing is then centered on the math axis and scaled to the style it sits in. C-FOR-1 is checked against known formulas in `crates/core/tests/chemfig.rs`; the layout rules by the README gallery (`crates/core/tests/examples.rs`).

### C-LAY-1 · Bond length

A standard bond is 2.2 em of the label type. chemfig's default is 3 em, which spreads structures wider than journals do; the ACS style is 1.44 em, which crowds the labels mathra sets at text size.

### C-LAY-2 · Strokes

Strokes are 0.055 em wide with round ends. The second line of a double bond is 0.22 em from the first; the lines of a triple bond, 0.2 em apart. A wedge is 0.22 em wide at its wide end; a hashed wedge has a line every 0.13 em.

### C-LAY-3 · Positions

The first atom is at the origin. Each other atom is placed by the first bond that reaches it: from the piece of the label it leaves from, in the bond's direction, one bond length times the bond's length factor. A bond that reaches an atom already placed (a ring closing, a `?` bond) is drawn between the two.

### C-LAY-4 · Labels

A label is typeset by mathra in upright type, piece by piece, and placed so that the middle of the piece the bond arrives at sits on the atom's position. A bond leaves a label from its last piece going right, its first going left, and its first non-hydrogen piece going vertically; it arrives at the first piece from the left, the last from the right, the first non-hydrogen piece vertically (so `H_3C` above a ring attaches at C). A bond stops 0.09 em from the letters of the piece it touches.

### C-LAY-5 · Aromatic circle

The circle of an aromatic ring is centered on the ring's atoms, with a radius of 0.6 of its inner radius.

### C-LAY-6 · Double bonds

In a ring, the second line of a double bond goes inside the ring and is shortened at both ends by 16% of the bond (at most 0.4 em). Elsewhere the two lines are centered on the bond.

### C-LAY-7 · Lewis marks

A Lewis mark sits in its direction (0 to 7, steps of 45°), 0.17 em beyond the letters of the atom's piece (0.08 em more above and below): two dots 0.18 em apart for a pair, one dot for a radical, a 0.3 em line for a pair line, a small square for a vacancy.

## Schemes

### C-SCH-1 · Compounds along the arrows

The first compound is centered on the origin. A compound after an arrow is placed in the arrow's direction: the arrow leaves the previous compound's box 0.45 em away, runs its length (2.6 em, or the widest label plus 1 em, times its length factor), and the next compound's box starts 0.45 em beyond. After `+`, the sign follows the previous compound 0.35 em away and the next compound follows it; compounds side by side share their vertical middle.

### C-SCH-2 · Arrows

`->` a shaft and a head; `<-` the head at the start; `<->` a head at each end; `<=>` two harpoons 0.15 em apart, the upper one forward with its barb up, the lower one back with its barb down; `<->>` and `<<->` the same with the weaker direction 22% shorter at each end; `=>` an open double arrow; `-/>` a forward arrow crossed in the middle; `0` nothing. Heads are two curved barbs, 0.3 em long and 0.14 em to each side.

### C-SCH-3 · Arrow labels

The label above an arrow goes on its left side, looking along it, and the label below on its right side, 0.24 em from the shaft, centered on the arrow's middle. For a horizontal arrow, above is above. A structure as a label is drawn at 0.85 of its size.

### C-SCH-4 · Mechanism arrows

A curved arrow goes from one hooked atom to another, as a cubic Bézier: it leaves in its `out` direction (TikZ's first control point, or `out=`), arrives from its `in` direction, and each control point reaches the length given (or 0.45 of the distance between the atoms). Without directions it bends 45° to the left (`bend left`, or the angle given). It starts and ends 0.12 em beyond the atoms' letters; a fishhook has one barb.

## Not yet

- SMILES input, with automatic 2D coordinates (rings first, then chains), so structures can come from databases
- schemes on several rows, `\subscheme` branching, named compounds as arrow targets (`\arrow(a--b)`)
- the bond attachment and the charge placement chemists tune by hand (`\chemabove` is read, chemfig's `[,,,,]` atom choice too, but labels like `HO^-` with Lewis marks put the charge where mathra puts a superscript)
