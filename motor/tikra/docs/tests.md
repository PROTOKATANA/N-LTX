# Tests

Every figure in the tests and in `corpus/` was written for this repository to exercise the interpreter.

## How to run

```console
$ cargo test                                    # core and CLI, 168 tests, under 3 s
$ cargo llvm-cov --workspace --summary-only     # coverage (install with cargo install cargo-llvm-cov)
$ cargo llvm-cov --workspace --html             # browsable report in target/llvm-cov/html
$ cargo clippy --all-targets -- -D warnings
$ TIKRA_UPDATE_BANNER=1 cargo test -p tikra --test banner   # redraws docs/banner.svg and docs/banner.png
$ cargo run --release -p tikra --example bench              # benchmark over corpus/ (README)
$ cargo test -p tikra-wasm                                  # the wasm exports, natively
$ CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
    cargo test -p tikra-wasm --target wasm32-unknown-unknown # the same exports inside wasm, on Node
$ npm/build.sh && npm/test/run.sh                           # the npm package, packed and installed
$ (cd crates/python && maturin develop && python -m pytest tests)   # the Python module, in a virtualenv
```

tikra depends on the published mathra crate (or a local checkout, see "Development" in the README). Labels go through it, so a change in mathra can change a test that checks label text or size. Math follows mathra's default locale, which is English (`\sin` reads "sin", decimal point).

## What each file covers

Integration, in `crates/core/tests/`:

| File | Tests | What it exercises |
|---|---:|---|
| `input.rs` | 26 | The forms `convert` accepts: `standalone` with and without `border`, `tikzpicture` with options, inline `\tikz`, a bare `axis`, loose statements, input with no figure. Repairs (doubled backslashes, Markdown), a preamble with colors and styles, xcolor's named sets loaded by the preamble, `\pagecolor`, comments, SVG escaping. |
| `paths.rs` | 48 | `--`, `-\|`, `\|-`, `+` and `++`, polar coordinates, lengths with units, names and anchors, `calc` (`!t!`, sums, factors, rotation), `.. controls ..`, `to`/`edge` with `out`/`in`/`bend`/`looseness`, `rectangle`, `circle`, `ellipse`, `arc` (parentheses and options), `grid` (with the path's own `step` or its own), `plot` (expression, coordinates, `smooth`), decorations, rounded corners, `shorten`, `\clip`, polar coordinates with a braced angle, nodes in the middle of a segment, `bend left` given to the whole statement, `current point is local`. Each operation also with an invalid coordinate. |
| `nodes.rs` | 46 | Shapes (`circle`, `ellipse`, `diamond`, `shape=`), minimum sizes, `inner sep`, `aspect`, named and angle anchors, baseline anchors (`base`, `base west`, `base east`, `text`), `anchor`, `above`/`below left=…`, `positioning` (`below=2cm of a`, `and`), `node distance`, `midway`/`pos=`, `label=`, text color, `font`, `text width`, `align`, `rotate`, `scale` (text included). |
| `styles_and_colors.rs` | 20 | Dashes, widths, opacities, hatching, arrow tips (every family and `>=`), xcolor colors (mixes, complement, names and the dvipsnames, svgnames and x11names, `\definecolor` in rgb/RGB/gray/cmyk/HTML, `\colorlet`), `\tikzset` with `.style` and `.append style`, `\tikzstyle`, `every node`/`every path`, option list parsing, hatching as the fill, the `->>` tip. |
| `control.rs` | 25 | `\foreach` (ellipsis, `count`, several variables, nested, no leaking variables), `scope` with `shift`/`scale`/`rotate`/`x=`/`y=`, `\def` and relatives, `\pgfmathsetmacro`, `declare function`, unknown environments, `\end` without `\begin`, ignored commands. |
| `expressions.rs` | 8 | The pgfmath evaluator through the public API: every function (`sin` … `veclen`, `atan2`, `log10`), constants, scientific notation, units, `^` associativity, syntax errors. Arrays of strings. |
| `plots.rs` | 25 | pgfplots: `axis lines` (left, middle, box, none), the window from the data, `domain`/`samples` on the axis and on the series, `\addplot+`, marks, a legend with `at=` and a frame, log variants read as linear, `axis` without `\end`, `only marks`, `mark=`, `no marks`. |
| `svg_and_png.rs` | 14 | The generated SVG (opacities, dashes, drawn nodes, multiline labels and alignments, mathra's rules, lines and circles, nested clips, an empty scene) and the PNG (background, dpi, invalid SVG, zero/negative/infinite dpi, pixel cap). |
| `figures.rs` | 12 | Whole figures that combine several features: a number line, a folding fan in a rotated scope, a flowchart, stacked waves from a declared function, a signpost with hatching and inherited label color, a plot with a legend. |
| `describe.rs` | 23 | The figure in words (`alt`, `description`, the SVG's `<title>` and `<desc>`, with ids of each figure's own), in other languages with the same drawing; the author's `alt={…}` told instead: drawings with and without text, math spoken, flowcharts with directions and labels, connections by anchors and through coordinates, loops, `edge` with quotes, paths that are not drawn, trees and mind maps, long lists cut, bar, horizontal bar, histogram, line, scatter, area, surface, mesh, stem, quiver, contour and box plots, log, polar, ternary and Smith axes, several axes, data visualizations. |
| `extension.rs` | 18 | Libraries through the public API: their keys and definitions, points and code, warnings, the token helpers, TeX defined as tokens, whether `transform shape` is on; node shapes they declare (one function for several, by name), painted as they say (strokes of their own width, the node's fill, its fill or white, solid; in a color, dash pattern and caps of their own, an unknown color warned), with texts of their own at each anchor, turned and mirrored with the node, in its words and its size; what they know of their node (name with its prefix, fill and line colors as xcolor expressions, its transformation); with their anchors, placed by them, turned with the node, lines stopping at their outline, rectangle or point, room made for their extent; unknown without the library. |
| `corpus.rs` | 1 | Every figure in `corpus/<topic>/` converts without warnings, with no `NaN`/`inf`, to SVG and PNG. |
| `problematic.rs` | 19 | Input built to break the interpreter (section below), including groups that never close at every place a group is read. |
| `banner.rs` | 1 | The README banner: `docs/banner.svg` must be byte for byte what tikra draws from `docs/banner.tex`, and `docs/banner.png` must be 1280 px wide. |
| `examples.rs` | 3 | The README gallery: each `docs/examples/*.svg` must be byte for byte what tikra draws from its corpus figure, on a white card. |

Command line, in `crates/cli/tests/cli.rs` (7 tests): help and version, stdin → stdout, file → SVG and PNG, `--dpi`, `--transparent`, `--warnings`, `--describe`, `--locale` (and an unknown one), a missing file, a file and a stdin that are not UTF-8, input with no figure, output into a missing folder, a PNG that is too large. The test runs the real binary (`CARGO_BIN_EXE_tikra`), and `cargo llvm-cov` counts that child process's coverage.

Packages, run by `.github/workflows/ci.yml` on every pull request:

| File | Tests | What it exercises |
|---|---:|---|
| `crates/wasm/tests/native.rs` | 4 | The functions behind the JavaScript API, natively: SVG, the JSON of `render` (with the figure in words) with and without warnings, PNG defaults (white background, 300 dpi), errors as text. |
| `crates/wasm/tests/web.rs` | 2 | The same exports compiled to wasm and run on Node; errors become a `JsError`. |
| `npm/test/*.test.*` | 8 | The packed npm package, installed in a scratch project: ESM and `require`, every input form, `render` and its figure in words, `png` options, errors as `Error`, `version()`. |
| `crates/python/tests/test_tikra.py` | 8 | The Python module as installed: every input form, `render` and its figure in words, `png` (bytes, `dpi`, `background`), `ValueError`, versions. `pypi.yml` runs it against each built wheel and the sdist. |

Unit tests, inside the modules: `describe.rs` (numbers, text and math as spoken, an axis with nothing to tell, plural forms, every language's phrases against English's placeholders, a chart in Portuguese), `color.rs` (mixes), `expr.rs` (basic expressions), `label.rs` (wrapping and font size), `tex/tests.rs` (TeX itself: macros, registers, conditionals, LaTeX commands, etoolbox, xstring, the page lengths). Plus the documentation example in `lib.rs`.

## Coverage

`cargo llvm-cov --workspace --summary-only`:

| File | Regions | Functions | Lines |
|---|---:|---:|---:|
| `cli/src/main.rs` | 100.00% | 100.00% | 100.00% |
| `core/src/axes.rs` | 98.55% | 100.00% | 100.00% |
| `core/src/color.rs` | 100.00% | 100.00% | 100.00% |
| `core/src/expr.rs` | 99.80% | 100.00% | 100.00% |
| `core/src/label.rs` | 98.23% | 100.00% | 100.00% |
| `core/src/lib.rs` | 99.44% | 100.00% | 100.00% |
| `core/src/scene.rs` | 100.00% | 100.00% | 100.00% |
| **Total** | **99.54%** | **100.00%** | **100.00%** |

The missing regions are almost all the error side of a `?` whose call cannot fail at that point (a point already validated earlier, for example) and the implicit `else` of an `if let` that would only fail with impossible state. Covering them would mean fabricating state from outside the API.

## Problematic cases

`tests/problematic.rs` runs each input on a thread with a 5-second deadline. An infinite loop fails the test instead of hanging the suite, and a panic shows up as a failure. Each case must end with a clear error or a warning, with no `NaN` or `inf` in the SVG.

| Input | Behavior |
|---|---|
| `\draw[step=0] (0,0) grid (1,1);` or `step=-1` | warning; grid ignored |
| `grid [step=0.000001] (100,100)` | warning; cap of 10,000 lines per direction |
| `decoration={zigzag, segment length=0pt}` | one step; cap of 10,000 |
| `\draw (1,0) arc (0:1000000000:1);` | one turn plus the rest, same end point |
| `plot[samples=1000000000]` | cap of 2,000 samples |
| `\def\a{\a\a}` and using `\a` | expansion stops at 100,000 characters; statement ignored with a warning |
| `a/.style={b}, b/.style={a}`, styles that multiply by 4 over 20 levels | cycle detected; cap of 10,000 options |
| `declare function={f(\x)=f(\x)+1;}` | error "nesting went past 200 levels", reported as a warning |
| 10,000 nested `(`, 10,000 `-` in a row | the same nesting error |
| `1^1^…^1` with 10,000 terms, `-` × 100,000 before a color | read in a loop, no deep recursion |
| `\foreach` 10,000 × 10,000 | error "figure too large: more than 100000 statements" |
| 5,000 nested `scope`, 200 nested `\foreach` | nesting error (cap of 64) |
| `\def` at the end of the text | ignored |
| `{1/0}`, `{0/0}`, `ln(0)`, `sqrt(-1)` in a coordinate, radius or angle | warning "coordinate is not finite"; path or node ignored |
| `\pgfmathsetmacro{\z}{1/0}` | warning "non-finite result" |
| `(1e300,1e300)`, `(200000,0)`, `minimum width=1e9cm` | scientific notation is read; warning beyond ±100,000 cm |
| an axis with `xmin=1e17, xmax=1e17+16` | ticks by index, cap of 50 |
| `--dpi 1000000`, a 1 km figure at 300 dpi | error "image too large" (cap of 100 megapixels) |
| `--dpi 0`, negative, `NaN` | error "invalid dpi: …" |
| stdin that is not UTF-8 | error "could not read standard input" |
| `\foreach \k in {1} {}` and using `\k` afterwards | the loop variable does not leak |
| `\end{ção}` (a name with accents) | read correctly |

Also tested, with no change needed: empty input or only comments (clear error), unbalanced braces, brackets and parentheses (warning), a node with no text, missing node names, Unicode and emoji in labels and node names, `samples=0`, `samples=1` and negative, an inverted or empty domain, `xmin=xmax` in pgfplots, a one-point path, `cycle` with no path, `\draw;`, `;;;;`, a zero or negative radius, a 0° arc, a missing file in the CLI, a label mathra refuses even in tolerant mode (nesting over 64 levels: the label goes away, with a warning).
