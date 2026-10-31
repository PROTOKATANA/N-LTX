# Boundary: what goes into mathra

mathra renders **equations**. The goal is to convert 100% of equations; what is not an equation goes to another tool. To keep it from growing into a rewritten TeX, every new construction goes through one rule:

> **The core takes what becomes boxes on a line or in a grid, and arrows between the cells of a grid.** Whatever needs free coordinates or running a program stays out.

| In | Out | Why |
|---|---|---|
| Fractions, scripts, roots, delimiters | — | inline typography |
| Matrices, `array`, `cases`, alignments, `\multicolumn` | — | grid |
| Column arithmetic (`\divchave`, `\contaarmada`), factoring with `array` | — | grid of digits |
| Commutative diagrams: amscd (`CD`), `tikz-cd`, `xymatrix` | `tikz-cd`'s TikZ keys beyond arrows (`\tikzcdset`, nodes placed by coordinates, `\draw` inside the diagram) | a grid of cells plus arrows between them: each arrow is fixed by its two cells, its bend and its style, so no coordinate comes from the author |
| Substitution macros: `\newcommand`, `\renewcommand`, `\providecommand`, `\def` with `#1…#9`, `\let`, `\DeclareMathOperator`, `\DeclarePairedDelimiter`, `\newenvironment` without arguments | `\ifx`, `\expandafter`, `\catcode`, `\def` with delimited parameters | replacing text is reading; conditionals and category changes are running a program |
| — | TikZ, pgfplots, `circuitikz` | drawing with coordinates |

## The exception: chemistry

Chemical equations are equations, and chemists write them with structures as much as with one-line formulas. So structural chemistry comes in, although it is drawn by directions and not on a line or a grid: chemfig structures (rings, wedges, Lewis marks), reaction schemes (`\schemestart`, `\arrow`, `\+`) and mechanism arrows between atoms (`\chemmove`). The exception is narrow:

- it is a graph of atoms and bonds whose geometry comes from chemfig's angles and fixed rules, never from coordinates the author gives;
- it lives in its own module (`crates/core/src/chem`, spec/chemistry.md), behind the `chem` feature, on by default, so the reader of equations does not grow drawing commands;
- inside `\chemmove` only `\draw` between two atoms is read: TikZ itself stays out.

| In (`chem`) | Out | Why |
|---|---|---|
| Structural formulas, rings, stereo bonds, Lewis marks, charges | 3D models, orbital surfaces | a structure is a graph; a surface is a picture |
| Reaction schemes with every chemfig arrow and its conditions | lab apparatus (`chemplants`), energy diagrams with axes | a scheme connects compounds; an apparatus or a plot has coordinates |
| Mechanism arrows between named atoms | any other TikZ inside `\chemmove` | an electron arrow goes from one atom to another |

## If figures ever come in

Figures would live in a separate workspace package (for example `crates/figures`), with a meaning-based language of their own: number line, Cartesian plane, Venn diagram, probability tree, labeled geometric figures. It would depend on the core to draw math labels and would use the same SVG contract. The core would never depend on it, and the LaTeX reader would not gain any drawing command.
