# Conformance with TikZ

How far tikra is from TikZ itself. Every example of the TikZ manual and of the pgfplots manual is drawn
by pdflatex and by tikra, and the two drawings are compared. The result is
[docs/conformance.md](../../docs/conformance.md): how many figures match, per chapter, what tikra
reported, and the speed of each.

```console
$ python3 -m pip install numpy scipy pillow
$ tools/conformance/run.sh            # the whole run; TeX's drawings are cached by content
$ tools/conformance/run.sh --tikra    # after a change in tikra: redraw with tikra and compare again
$ open target/conformance/report.html # every figure that differs, TeX next to tikra
```

## The examples

The manuals are the specification: each `codeexample` block is code that the manual runs and shows
next to its result. `extract.py` reads the sources of the versions inside the TeX Live image (pgf
3.1.11a, pgfplots 1.18.2), downloaded from their GitHub tags, and writes one standalone document per
example, honoring the block's options as the manual's macros do:

- `code only`, `hidden`, `multipage`, animations: nothing is drawn, so they are skipped;
- `setup code`: run without being shown, and kept for the rest of its file;
- `preamble`, `pre`, `post`, `render instead`, `vbox`: applied as in the manual;
- every document loads the libraries that the manual loads for all its examples, on one line.

The same document goes to pdflatex and to tikra. Only figures count (code with a `tikzpicture`,
`\tikz`, a pgfplots axis or the basic layer's `pgfpicture`); examples of text or of tables are listed
apart.

## The reference

`reference.py` compiles every document with pdflatex in `texlive/texlive:TL2025-historic` (pinned, so
the reference does not move), each from its manual's doc folder, where the data files and images it
reads live, and turns the first page into a PNG at 100 dpi with Ghostscript. A document that TeX itself
cannot compile (it needs shell escape, LuaTeX or a file that is not there) is left out of the score.

`timing.py` times a fixed sample of 200 figures with pdflatex one at a time, in a TeX Live image native
to the machine (the pinned one is amd64 only and may run emulated).

## The comparison

The `conformance` example of the core crate draws each document with tikra, at the same 100 dpi on
white, and records warnings, errors, panics and figures that take longer than 10 s. `compare.py`
compares each pair by:

- **shape**: after aligning the two images (the best shift of up to 6 px from each corner and from the
  center), their edges (where the color changes, so a fill counts by its outline and a label on a fill
  still shows) are compared: the share of tikra's edges with a TeX edge within 2 px (0.5 mm), and the
  other way round; their harmonic mean is the F1 score.
- **pieces**: each separate piece of TeX's drawing (a line, a word, a node) must be there in tikra's, and
  the other way round; one missing or extra piece is enough to not match.
- **color**: of tikra's ink near TeX's, the share with about the same color as some TeX ink in the 3×3
  pixels around it, both drawings blurred by 0.7 px first. Ghostscript and resvg antialias a line's
  edge at different phases, and spread a thin line's ink differently (one dark pixel, or two lighter
  ones); neither is a color difference.
- **ink**: the share of each drawing's ink that lies within 2 px of a mark of the other (the smaller
  of the two). A fill one drawing has and the other does not is mostly away from the edges, where the
  scores above do not look.
- **size**: width and height within 10% (or 4 px).

A figure **matches** with F1 ≥ 0.9, color ≥ 0.9, ink ≥ 0.95, the same size and no missing or extra piece; it is
**close** with F1 ≥ 0.75.
**Our 100%** is the share of the figures a converter can reproduce that tikra draws as TeX does. It
leaves out, by an explicit list checked by eye ([unreproducible.json](unreproducible.json)), the
figures with random numbers and no `\pgfmathsetseed` (TeX draws new ones on every run), with an image
file or an external program (gnuplot), and with the day's date; and it counts as done the figures that
differ from TeX's only by antialiasing (Ghostscript, which renders TeX's PDF, and tikra put lines,
fills and letters on the pixels differently: Ghostscript inks a line wider than it is and small labels
heavier, bands a shading and draws pattern tiles without antialiasing), as long as they stay at least
close. A figure joins the list only
after looking at both drawings; one that later differs again is named in the report.

The tolerance absorbs what is expected to differ: labels in Latin Modern Math (Computer Modern
from a math font, without TeX's optical sizes or text italic), antialiasing, a bounding box off by a hair. A figure that does not match although tikra gave no
warning is counted apart: that is tikra being wrong without saying so.
