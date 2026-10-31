# Conformance with CircuiTikZ

How far voltik is from CircuiTikZ, measured by its manual. Every example of the CircuiTikZ manual and
every entry of its list of components is drawn by pdflatex and by voltik, and the two drawings are
compared. The result is [docs/conformance.md](../../docs/conformance.md): how many components and
examples match, per section, and what voltik reported.

```console
$ python3 -m pip install numpy scipy pillow
$ tools/conformance/run.sh             # the whole run; TeX's drawings are cached by content
$ tools/conformance/run.sh --voltik    # after a change in voltik: redraw with voltik and compare again
$ open target/conformance/report.html  # every item, TeX next to voltik
```

## The items

The manual is the specification. `run.sh` downloads the manual of the CircuiTikZ release that TeX Live
carries (1.8.7): only `doc/circuitikzmanual.tex` and the manual's own macros, `doc/ctikzmanutils.sty`.
The package's code is not read. `extract.py` writes one standalone document per item:

- **examples**: each `ctikzExample` block, the code the manual shows next to its result. `only code`
  blocks draw nothing and are skipped; the definitions of `exec outside` blocks stay in force for the
  rest of their group; `store` and `restore` prepend stored code, as the manual's macros do.
- **components**: each entry of the list of components, `\circuitdescbip` (path-style) and
  `\circuitdesc` (node-style), drawn as the manual draws it: `\draw (0,0) to[<name>] (2,0);` or
  `\draw (0,0) node[<name>]{<text>};`, filled with the manual's `fillcol` when the entry is fillable.
  The anchors the manual marks around some components are left out.

The manual changes settings as it goes (`\ctikzset`, `\tikzset`, `\def`, `\newcommand`, colors,
`\ctikzsubcircuitdef`), often inside `\begingroup … \endgroup`. They are followed in order, group by
group, and every document starts with the ones in force at its place in the manual. Every document
loads CircuiTikZ as the manual does (`siunitx, RPvoltages`), with T1 Latin Modern, and the manual's
macros for marking anchors.

A macro given to `vif queue add` counts as used, though the document names it without its backslash.

The same document goes to pdflatex and to voltik.

## The reference

`reference.py` compiles every document with pdflatex in TeX Live, the image pinned by digest (CircuiTikZ
1.8.7, so the reference does not move), and turns the first page into a PNG at 100 dpi with
Ghostscript. A document that TeX itself cannot compile is left out of the score.

## The comparison

The `conformance` example of the core crate draws each document with voltik, at the same 100 dpi on
white, and records warnings, errors, panics and figures that take longer than 10 s. `compare.py`
compares each pair with the same measures as tikra's conformance run:

- **shape**: after aligning the two images, their edges are compared: the share of voltik's edges with
  a TeX edge within 2 px (0.5 mm), and the other way round; their harmonic mean is the F1 score.
- **pieces**: each separate piece of TeX's drawing (a line, a word, a component) must be there in
  voltik's, and the other way round; one missing or extra piece is enough to not match.
- **color**: of voltik's ink near TeX's, the share with about the same color as some TeX ink nearby.
- **ink**: the share of each drawing's ink that lies within 2 px of a mark of the other.
- **size**: width and height within 10% (or 4 px).

A figure **matches** with F1 ≥ 0.9, color ≥ 0.9, ink ≥ 0.95, the same size and no missing or extra
piece; it is **close** with F1 ≥ 0.75.

The tolerance absorbs what is expected to differ: labels in Latin Modern Math (Computer Modern from a
math font, without TeX's optical sizes or text italic), antialiasing, a bounding box off by a hair. An
item that does not match although voltik gave no warning is counted apart: that is voltik being wrong
without saying so.

Each item is compiled once, but the ones with tikzmark's `pic anchor`, which needs a second run to find its place.
