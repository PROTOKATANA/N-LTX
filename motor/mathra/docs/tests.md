# Tests

Every case here is an invented equation, written to exercise a construction or a rule.

## How to run

```sh
cargo test --workspace --exclude mathra-python              # core, CLI and wasm (native)
cargo test -p mathra --features latin-modern                # also the second font (tests/fonts.rs)
cargo test -p mathra --no-default-features                 # without structural chemistry (the `chem` feature)
cargo test -p mathra-wasm --target wasm32-unknown-unknown   # wasm on Node (needs wasm-bindgen-cli)
cargo clippy --workspace --all-targets -- -D warnings
```

Python, with a virtual environment that has `maturin` and `pytest`:

```sh
cd crates/python && maturin develop && python -m pytest tests
```

Coverage with [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov), with pytest in the same report:

```sh
eval "$(cargo llvm-cov show-env --export-prefix)"
cargo llvm-cov clean --workspace
cargo test --workspace --exclude mathra-python --features mathra/latin-modern
(cd crates/python && maturin develop && python -m pytest tests)
cargo llvm-cov report --summary-only        # or --html
```

## Benchmark

`cargo run --release -p mathra --example bench` converts the hand-written constructions in `corpus/` in strict and tolerant mode and prints how many convert and the time per equation. The README shows one run.

## What each file covers

| File | Tests | What it checks |
|---|---:|---|
| `crates/core/tests/rules.rs` | 34 | One test per rule in `spec/rules.md` and `spec/svg.md`; the name starts with the rule id. S-SVG-10 (steps and the writing animation) has five: off by default and byte for byte, every element numbered in order, how each kind enters, the timing and the same `<style>` in every equation, valid XML. |
| `crates/core/tests/structures.rs` | 24 | Fractions, roots, matrices, cases, alignments: what sits above, below or beside what. |
| `crates/core/tests/latex_reader.rs` | 100 | Every command and environment the reader knows (including every amssymb symbol and negation, drawn in strict mode so the font must have it), every recovery path (arrow diagrams included) and, at the end (`p_*`), the problem inputs. |
| `crates/core/tests/layout.rs` | 72 | From the tree to boxes: geometric properties (width, height, alignment, stretching), not exact coordinates. Bold, small capitals and monospace go on after math inside their group. Italic text has no italic correction inside it, `\mathit`'s letters make a word. Includes the layout problem cases and arrow diagrams (shaft on the axis, diagonal and bent arrows, strokes per style, labels that widen a column or break a line). |
| `crates/core/tests/outputs.rs` | 16 | Screen reader speech in English and pt-BR, linear text, public shortcuts, error messages, locales. |
| `crates/core/tests/locales.rs` | 8 | The 16 languages: every phrase present, the whole corpus, the chemistry, physics and siunitx constructions and the MathML and AsciiMath inputs in each one, region codes, speech and number conventions per language. |
| `crates/core/tests/hostile_output.rs` | 7 | Hostile input, in LaTeX, MathML and AsciiMath: the SVG stays valid XML and injects no markup; an XML entity bomb stays bounded. |
| `crates/core/tests/macros_and_packages.rs` | 22 | Substitution macros (L-MAC-1), package commands (physics, braket, siunitx ranges and lists, tensor, mathtools), tikz-cd and xymatrix (arrows, styles, labels, positions) and the boundary (`spec/boundary.md`). |
| `crates/core/tests/compatibility.rs` | 27 | LaTeX compatibility: mhchem equilibria whose arrows stretch under their labels, ink past the box never cut by the view (S-SVG-1, over the whole corpus, display and inline), `\phase` and `\angl` as steinmetz and actuarialangle draw them, the section-3 block (`\usepackage`, `\arraystretch`, `c:c:c`, `\hline`, `\hdashline`), rules inside their table, each package on its own, the LaTeX commands of the KaTeX list, `\mathchoice` (the branch of each style), `\vcenter`, `\above` and `\genfrac` thicknesses, `\reflectbox` (mirrored glyphs), `\url` and typewriter text, and KaTeX-only names refused. |
| `crates/core/tests/chemfig.rs` | 9 | Structural chemistry (L-CHE-2, `chem` feature): molecular formulas of known molecules (C-FOR-1), fused rings sharing atoms and bonds, bond kinds and angles, branches, label pieces, distant bonds, Lewis marks and hooks, schemes and every arrow kind, speech in English and pt-BR, text and braille, mechanism arrows, chemistry inside an equation and in a subscript, recoveries. |
| `crates/core/tests/chemistry.rs` | 17 | mhchem (L-CHE-1): arrows with labels, hydrates, coefficients, bonds, isotopes, charges and radicals, `\frac` inside `\ce`, `\pu`, both inside text. |
| `crates/core/tests/tolerance.rs` | 15 | Tolerant versus strict mode (`spec/tolerance.md`). |
| `crates/core/tests/mathml.rs` | 15 | MathML input (`spec/input.md`): the same speech and text as the LaTeX of each equation, every element, fences around tables, decorations, entities and namespaces, MathJax's conventions, L-ENT-4, E-REC-10 and invalid XML. |
| `crates/core/tests/asciimath.rs` | 8 | AsciiMath input (`spec/input.md`): the same speech and text as the LaTeX of each equation, brackets removed from fractions and scripts, intervals, matrices and cases, commands, unpaired brackets (E-REC-5). |
| `crates/core/tests/braille.rs` | 17 | Braille (`spec/braille.md`): the examples of the Nemeth code (BANA 2022) and of the CMU (MEC 2006), each cited by section or example number; the code chosen by the locale; braille in the SVG and the JSON; S-BRL-1; the whole corpus in both codes; the signs settled by the review against MathCAT (`docs/braille-review.md`). |
| `crates/core/tests/reference.rs` | 5 | The reference page (`site/reference.html`, data in `corpus/reference.json`): every row converts in strict mode with no warning, each AsciiMath and MathML row has the same speech as its LaTeX column, every row has braille in both codes. |
| `crates/core/tests/fonts.rs` | 6 | The font an equation is set in: STIX Two Math by default; with the `latin-modern` feature, Latin Modern Math with its own metrics and its own word space (a third of an em). The embedded fonts against the originals in `tools/fonts`: every character keeps its glyph ID, outline, advance and MATH data, and so do its size variants and assembly parts. |
| `crates/core/tests/banner.rs` | 1 | The README banner (`examples/banner.rs`) is deterministic and matches `docs/banner.svg`. |
| `crates/core/tests/examples.rs` | 4 | The README gallery (famous equations, big equations, constructions): every equation is accepted in strict mode, each `docs/examples/*.svg` matches what the code draws, and no stray file sits in that folder. The site's gallery page (`corpus/gallery.json`): every README example is on it, as the same equation, and every row converts in strict mode. `CHANGELOG.md`: releases newest first, each with a date and a headline, and every equation it shows a gallery name or LaTeX that converts in strict mode. |
| `crates/core/src/input/asciimath.rs` (unit) | 1 | Every AsciiMath symbol's LaTeX spelling is valid LaTeX in strict mode. |
| `crates/core/src/chem/chemfig.rs` (unit) | 3 | The chemfig reader on its own: labels split into atoms, angle syntax, Lewis marks. |
| `crates/core/src/chem/formula.rs` (unit) | 2 | Molecular formulas: atoms counted in labels, Hill order without carbon. |
| `crates/core/src/font.rs` (unit) | 2 | A glyph without an outline and a quadratic curve, which the embedded font does not have. |
| `crates/cli/tests/cli.rs` | 18 | The binary run as a process: arguments, `--from`, `--braille`, stdin, `--batch`, writing files, exit codes. |
| `crates/wasm/tests/native.rs` | 7 | Exported functions outside the browser, and the errors before they become a `JsError`. |
| `crates/wasm/tests/web.rs` | 2 | The same functions inside wasm, on Node, including the `JsError`. |
| `npm/test/*.test.*` | 10 | The npm package as published: packed, installed in a scratch project, imported with ESM and `require`; options, `render`, strict errors, locales, input notations, braille. Run with `npm/build.sh && npm/test/run.sh`. |
| `crates/python/tests/test_mathra.py` | 14 | The installed Python package: SVG, tree, `render`, `locales`, `display`, `strict`, `input` and `braille`, locale, `ValueError`, versions. CI runs it on the wheel of each platform and on the sdist (`.github/workflows/pypi.yml`). |

## Coverage

Measured with the procedure above (core with the `latin-modern` feature, CLI, native wasm and pytest together), on version 1.2.0.

| File | Lines | Regions | Functions |
|---|---:|---:|---:|
| `cli/src/main.rs` | 100% | 98.7% | 100% |
| `core/src/font.rs` | 100% | 99.6% | 100% |
| `core/src/input/asciimath.rs` | 92.7% | 91.5% | 95.7% |
| `core/src/input/latex.rs` | 99.2% | 96.1% | 100% |
| `core/src/input/mathml.rs` | 92.9% | 85.0% | 96.0% |
| `core/src/input/symbols.rs` | 100% | 87.1% | 100% |
| `core/src/layout/alphabet.rs` | 100% | 100% | 100% |
| `core/src/layout/arithmetic.rs` | 100% | 96.0% | 100% |
| `core/src/layout/math_box.rs` | 100% | 97.9% | 100% |
| `core/src/layout/mod.rs` | 99.9% | 97.9% | 100% |
| `core/src/layout/stretch.rs` | 100% | 98.7% | 100% |
| `core/src/layout/table.rs` | 100% | 98.4% | 100% |
| `core/src/lib.rs` | 100% | 99.5% | 100% |
| `core/src/locale.rs` | 98.1% | 97.7% | 93.1% |
| `core/src/output/braille/cmu.rs` | 91.8% | 92.0% | 100% |
| `core/src/output/braille/mod.rs` | 100% | 100% | 100% |
| `core/src/output/braille/nemeth.rs` | 94.0% | 94.5% | 96.7% |
| `core/src/output/mod.rs` | 100% | 100% | 100% |
| `core/src/output/speech.rs` | 99.2% | 99.4% | 100% |
| `core/src/output/svg.rs` | 100% | 100% | 100% |
| `core/src/output/text.rs` | 100% | 100% | 100% |
| `core/src/rules.rs` | 100% | 100% | 100% |
| `core/src/tree.rs` | 100% | 100% | 100% |
| `python/src/lib.rs` | 100% | 90.5% | 100% |
| `wasm/src/lib.rs` | 74.3% | 78.6% | 66.7% |
| **Total** | **97.4%** | **94.7%** | **98.3%** |

About what is missing:

- **Lines below 100%** are in the readers added in 1.1.0 (`mathml.rs`, `asciimath.rs`) and in the braille codes (`cmu.rs`, `nemeth.rs`): branches no test reaches yet, mostly rare elements, attributes and sign combinations. `cargo llvm-cov report --html` shows each one.
- **Regions below 100%** are mostly arms of symbol tables (`symbols.rs`, `locale.rs`) that share a line with others: the line counts as covered when any arm on it runs. Every command in those tables has at least one test; not every combination of symbol and locale does.
- **`wasm/src/lib.rs`**: the missing lines are the code `#[wasm_bindgen]` generates for the JavaScript boundary. It only runs inside wasm, and `cargo-llvm-cov` does not instrument wasm. That path is tested in `tests/web.rs`, on Node, just without measurement.

## Problem cases

Every row has a test. In no case is there a panic, a hang, a NaN or a negative dimension in the SVG.

### Reader (`latex_reader.rs`, `p_*`)

| Input | Strict | Tolerant |
|---|---|---|
| Unbalanced braces | error | draws, E-REC-4 |
| `\left` without `\right`, `\begin` without `\end` and the other way around | error | draws, E-REC-5 |
| Loose `&` and `\\` outside an environment | implicit alignment | same |
| `\newcommand\a{\a}`, `\def\a{\a\a}` | "too many macros" | discards, a single E-REC-1 warning |
| Exponential expansion (26 levels) | finishes in seconds | same |
| 10,000 levels of `{`, `\frac`, `\sqrt`, `\left`, `^{`, `\not` | nesting error | same; fits in a 1 MB stack |
| 1 MB input | reads, in linear time | same |
| Empty, only spaces, `$$`, `\[\]` | empty equation | same |
| NUL and other control characters | error | ignores, E-REC-9 |
| Emoji, a lone combining accent, Arabic, Hebrew, ZWJ, BOM | reads | reads; missing glyphs become □ (E-REC-8) |
| Half a surrogate pair from JSON (`\ud83d`) | error | no invalid character produced |
| `^^`, `^_^_`, `x^1^2^3` | error | draws, E-REC-3 |
| Unclosed `\sqrt[`, missing argument at the end | error | draws, E-REC-7 |
| `\hdotsfor{10^20}`, `*{50}{*{50}{*{50}{c}}}`, a 10,000-digit number | error or limit | finishes; columns stop at 1,000 |
| `\color{red" onload="x}{a}` | error | draws without color, E-REC-9 |
| `\divchave{a}{0}`, arithmetic with more than 30 digits | error | inline calculation, E-REC-9 |
| `\newcommand\a[1]{#0}` | error | warning (it used to panic) |

### Layout (`layout.rs`)

| Input | Result |
|---|---|
| 1×1 matrix, empty matrix, rows with different column counts | composed by the longest row |
| 200×200 matrix | composed in under 5 s in debug |
| `\left(` around 50 rows | the parenthesis is assembled from font parts; `⟨`, which has no parts, stops at the largest variant |
| `\sqrt` of 60 rows | the rule covers the radicand |
| 30 nested fractions, 20 levels of scripts | the size stops at scriptscript, without shrinking further |
| `\overbrace` over long text | ok |
| `\hline`/`\cline` out of range or reversed, `\multicolumn` wider than the table or `{0}` | no negative rule |
| Hand-built tree with a zero divisor or arithmetic with wrong terms | an error, not a panic |
| `\mathbb{1}`, `\mathbb{\alpha}`, `\mathbb{ж}` | double-struck form when it exists; otherwise the character itself |
| Emoji, U+10FFFF in a script, fraction or root | □ with a warning; in strict mode, `Error::MissingGlyph` |
| Accent, brace or frame over nothing | takes the width of its own drawing |

### Output (`hostile_output.rs`, `outputs.rs`)

| Input | Result |
|---|---|
| `\text{</title><script>…}`, `]]>`, `<!--`, `<?xml` | escaped; no element or `on…` attribute injected |
| NUL, ESC, U+FFFE in the source | become U+FFFD in the metadata (S-SVG-8); they used to produce invalid XML |
| Emoji, RTL, zero-width, CJK | valid XML |
| 40,000 characters of source | valid XML |
| The same equation twice / different equations | same ids / different ids |
| Hand-built tree (impossible `LongDivision`, large operator without limits) | speech and text come out without a panic |

### Command line (`cli.rs`)

| Input | Result |
|---|---|
| `--batch -o` to a missing folder | every line says `error	could not write …` (it used to say `ok` and write nothing) |
| `--batch` with a line that is not UTF-8 | that line says `error`, the following ones go on (the batch used to stop silently) |
| `--batch` with CRLF and empty lines | the `\r` goes; file numbering is kept |
| `-o` pointing to a folder | clear error, exit code 1 |
| Empty equation | empty, valid SVG |
| A 100,000-character argument | converts |
