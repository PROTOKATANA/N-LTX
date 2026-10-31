# Changelog

What changed in each release of mathra: the crates `mathra` and `mathra-cli`, `@mathlabs/mathra` on npm and `mathra` on PyPI, which are released together. The site shows the same, with the equations drawn live: [the project's evolution](https://matheusmota-dev-br.github.io/mathra/changelog.html).

Each release has a headline (the line under its title), what changed, and the equations that show it (`Equations:`): a name from the gallery ([`corpus/gallery.json`](corpus/gallery.json)) or the LaTeX itself, drawn by the site with today's mathra. `scripts/release.sh` will not release a version without its section here, and makes the GitHub release from it.

## 1.9.2 - 2026-10-30

`\boldmath` in bold italic, and `\SI`'s values as siunitx prints them.

- `\SI{+10}{V}` leaves out the plus and `\SI{.25}{V}` gets its zero, as siunitx does ([#97](https://github.com/matheusmota-dev-br/mathra/issues/97)).
- `\boldmath` sets letters in bold italic, as TeX's bold math font does, and reaches the text of a `\rotatebox` ([#97](https://github.com/matheusmota-dev-br/mathra/issues/97)).

Equations: `\text{{\boldmath $x$}}`, `\text{{\boldmath \rotatebox{90}{$x$}}}`, `\SI{+10}{V}`, `\SI{.25}{V}`

## 1.9.1 - 2026-10-29

Italic text set as a text font sets it, and `\mathit`'s letters as one word.

- Inside `\text{…}`, `\textit{…}` and `\emph{…}` set their argument in italic and `\itshape`, `\it` and `\em` the rest of their group, with no italic correction between the letters or after a word; after `\textit{…}` and `\emph{…}` comes their last letter's correction, as LaTeX's `\/`, except before a period or a comma. `\textit{…}` in math is text in italic (L-TXT-5, [#93](https://github.com/matheusmota-dev-br/mathra/issues/93)).
- `\mathit{…}`'s letters make one word, with no italic correction between them, as in TeX's text font: since 1.9.0 each letter had one, as in math italic ([#92](https://github.com/matheusmota-dev-br/mathra/issues/92)).

Equations: `\text{an \emph{italic} word, {\itshape and more}}`, `\mathit{diff} \ne diff`

## 1.9.0 - 2026-10-29

Chemistry, color, fills and math environments inside text, and TeX's italic correction at the end of a formula.

- Inside `\text{…}`, mhchem's `\ce{…}` and `\pu{…}` are read as in math (`\text{water, \ce{H2O}}`), instead of being written as is (L-CHE-1, [#84](https://github.com/matheusmota-dev-br/mathra/issues/84)).
- Inside `\text{…}`, `\color{…}` (also `\color[model]{…}`) colors the rest of its group and `\textcolor{…}{…}` its argument, math included, keeping the text's bold, small capitals or monospace (L-TXT-4, [#85](https://github.com/matheusmota-dev-br/mathra/issues/85)).
- Inside `\text{…}`, `\hfill`, `\hfil` and `\hss` take no room, as in a box of natural width, and `\newline` and `\linebreak` are a space, as `\\` is (L-TXT-3, [#86](https://github.com/matheusmota-dev-br/mathra/issues/86)).
- Inside `\text{…}`, `\begin{equation}`, `align`, `gather`, `multline` and the other displays are read as math up to their `\end`, like `\[…\]`; `array`, `pmatrix`, `cases`, `aligned` and the other math-only environments too, with a warning (E-REC-1), since TeX refuses them in text (L-TXT-2, [#87](https://github.com/matheusmota-dev-br/mathra/issues/87)).
- A slanted letter that ends a math list gets its italic correction after it, as TeX does (Appendix G, rule 17). That covers the end of the formula, of a script, a fraction's parts, a radicand, a phantom, a lap and a table cell. In scripts, the base letter's correction stands in at the script's size. `$V$` was 5.83pt wide, TeX gives 8.06pt ([#89](https://github.com/matheusmota-dev-br/mathra/issues/89)).
- `layout::compose_list`: a math list composed on its own, with that correction.

Equations: `\text{water, \ce{H2O}, at \pu{20 mol}}`, `\text{the {\color{red}warm} and {\color{blue}cold} sides}`, `\text{so \begin{align*}a&=1\\b&=2\end{align*}}`, `V = f`, `V_0 + R_1`, `\frac{V}{f}`, `\sqrt{f}`

## 1.8.0 - 2026-10-29

Space and phantoms inside text, and text kerned as TeX kerns it.

- Inside `\text{…}`, `\hspace{…}` and `\hspace*{…}` add their space, and `\phantom`, `\hphantom` and `\vphantom` leave the room of their argument, instead of being written in red (L-TXT-3, [#79](https://github.com/matheusmota-dev-br/mathra/issues/79)).
- `Options::text_kerning`: text in `\text{…}` kerned by the pairs of TeX's OT1 Computer Modern (`cmr10`, `cmbx10`) or T1 Latin Modern (`ec-lmr10`, `ec-lmbx10`) fonts, as TeX's tables do (`va`, `AV`, `To`); none by default, as before (#81).
- Code that builds an `Options` or a `layout::Context` listing every field needs the new one (`..Default::default()`) (#81).

Equations: `\text{a\hspace{1cm}b}`, `\text{a\phantom{xy}b}`, `\text{AVA Today}`

## 1.7.1 - 2026-10-29

Bold text stays bold after math inside it.

- Inside `\text{…}`, the text after math (`$…$`, `\(…\)`, `\ensuremath`, `\textsubscript`, siunitx, graphicx boxes) keeps the bold, small capitals or monospace of its group: in `\textbf{Z $x$ Z}` the second Z was upright ([#77](https://github.com/matheusmota-dev-br/mathra/pull/77)).

Equations: `\text{\textbf{Z $x$ Z} and \textsc{Small $x$ Caps}}`

## 1.7.0 - 2026-10-29

Labels as LaTeX sets them: subscripts, siunitx and graphicx boxes inside text, and `\phi` in italic.

- `\phi` is in math italic, as TeX sets it, like `\varphi` and the other lowercase Greek letters; so are `\epsilon`, `\vartheta`, `\varkappa`, `\varrho` and `\varpi`, which were upright too ([#66](https://github.com/matheusmota-dev-br/mathra/issues/66)).
- Inside `\text{…}`: `\textsubscript{…}` and `\textsuperscript{…}` (`H\textsubscript{2}O`), `\ensuremath{…}`, and `\boldmath`, which sets the math that follows in bold until the end of its group; `\fontsize{…}{…}\selectfont` is ignored like the other sizes. A math command written in text (`\mathrm{V}`, `\frac{1}{2}`) is read as math, with a warning (E-REC-1), instead of being written as is ([#70](https://github.com/matheusmota-dev-br/mathra/issues/70)).
- siunitx ranges read as siunitx sets them: `\qtyrange{1}{5}{\volt}` is `1 V to 5 V`, with the language's word (`1 V a 5 V` in pt-BR), and an en dash only where the font cannot write it. The siunitx commands (`\SI`, `\qty`, `\si`, `\unit`, `\num`, the ranges, lists and products) work inside `\text{…}` as in math, and `\qty[options]{…}{…}` is siunitx's, not physics's ([#71](https://github.com/matheusmota-dev-br/mathra/issues/71)).
- graphicx's boxes, in math and inside `\text{…}`: `\scalebox{h}[v]` scales across and up apart, and a negative factor mirrors (`\scalebox{-1}[1]`) or turns upside down; `\rotatebox[origin=…]{angle}` turns the box; `\resizebox{width}{height}` (and `*`) scales it to a size, `!` keeping the ratio; `\raisebox{lift}[height][depth]` sets the height and depth asked for, so `\raisebox{0pt}[\height][0pt]` removes the depth. Lengths take `\width`, `\height`, `\depth` and `\totalheight` ([#69](https://github.com/matheusmota-dev-br/mathra/issues/69)).

Equations: `\phi \varphi \epsilon \vartheta \varrho \varpi` `\text{H\textsubscript{2}O and {\boldmath $x$} $x$}` `\text{from \qtyrange{1}{5}{\volt}}` `\rotatebox{90}{$I_0$} \scalebox{-1}[1]{Q1} \resizebox{1cm}{!}{abc}`

## 1.6.2 - 2026-10-28

The limits of an integral sit next to its sign.

- The limits of an integral sit next to the sign, as the font's italic correction asks: the upper one at the sign's advance width, the lower one moved left by the italic correction ([#65](https://github.com/matheusmota-dev-br/mathra/issues/65)).

Equations: `gaussian` `stokes` `\int_0^1 x\,dx`

## 1.6.1 - 2026-10-28

How to contribute, and how to report a vulnerability.

- A code of conduct (Contributor Covenant 2.1), a contributing guide and a security policy, in `.github/`. Vulnerabilities are reported privately through the repository's Security tab.
- The checks run on the contributor's machine: `scripts/check.sh` runs fmt, clippy and the Rust, wasm, npm and Python tests, and the git hooks in `.githooks/` run it before each commit and push. GitHub runs them again on `main`, not on pull requests.
- Conversion is unchanged: every SVG is the same as 1.6.0's, apart from the version it carries.

Equations: `euler` `quadratic`

## 1.6.0 - 2026-10-27

The equation writes itself, and reads the LaTeX of the packages it names.

- The writing animation: `--animate`, `{ animate: true }`, `animate=True`. Each glyph fades in, each rule grows along its length and each line is drawn, in the order the layout set them, all in about three seconds; off by default, and the SVG without it is unchanged byte for byte (S-SVG-10).
- The steps apart from the animation: `--steps`, `{ steps: true }`, `steps=True` number each element (`data-step`), for a page that walks through the equation.
- The playground walks through the equation step by step and downloads the animation as a GIF, a video or an animated SVG with its own controls; every equation of the gallery animates.
- Ink is never cut at the edge of the SVG: the view covers the ink that passes the box, such as the hook of a bold f at the end of an equation (S-SVG-1).
- Equilibrium and long arrows stretch under their labels, mhchem's `<=>` and `\xrightleftharpoons` included, built from the arrows the font can stretch.
- Packages read as LaTeX defines them: `\phase` (steinmetz), `\angl` (actuarialangle), arydshln's `:`, `\hdashline` and `\cdashline`, `\arraystretch`, the colons of mathtools and colonequals, `\varGamma` and family, `\intop`, `\injlim`, `\char`, `\reflectbox`, `\url`, `\href`, and TeX's `\mathchoice`, `\vcenter` and `\above`. Of the 1002 examples of KaTeX's support table, 925 convert in strict mode, up from 810; the rest are names LaTeX does not know.
- Fixes: `\circledcirc` is the operator ⊚, not a frame; vertical rules of `array` stay inside the table, an outer `|` at its edge; `\textbf` and `\texttt` in math are bold and typewriter; `\genfrac` draws the thickness it is given.
- This changelog, from 1.0.0 on; each release on GitHub is made from it, and the site's evolution page shows it.

Equations: `\ce{Zn^2+ <=>[+ 2OH-][+ 2H+] $\underset{\text{amphoteres Hydroxid}}{\ce{Zn(OH)2 v}}$}` `\rho \left( \frac{\partial \mathbf{u}}{\partial t} + (\mathbf{u} \cdot \nabla) \mathbf{u} \right) = - \nabla p + \mu \nabla^2 \mathbf{u} + \mathbf{f}` `V = 10\phase{-48\degree}\,\mathrm{V}` `\ddot{a}_{\angl{n}} = \frac{1 - v^n}{d}` `\def\arraystretch{1.5}\begin{array}{c:c:c} a & b & c \\ \hline d & e & f \\ \hdashline g & h & i \end{array}`

## 1.5.0 - 2026-10-26

In Latin Modern, scripts and limits sit where pdfTeX puts them.

- Scripts in the font's own forms for scripts (`ssty`), and `\ldots` and `\cdots` set as TeX sets them.
- In Latin Modern, scripts shift as pdfTeX shifts them, and the limits of an operator get the room above and below that TeX leaves.
- TeX's other delimiters, and the moustaches (`\lmoustache`, `\rmoustache`) built from brace halves in Latin Modern, which has no glyph for them.

Equations: `basel` `zeta` `\left\lmoustache \frac{a}{b} \right\rmoustache \quad x_1, \ldots, x_n`

## 1.4.0 - 2026-10-26

Structures and diagrams: chemfig molecules and commutative diagrams.

- Structural chemistry with chemfig, behind the `chem` feature (on by default): structures with rings, bonds, angles and Lewis marks, reaction schemes and mechanisms with curved arrows, each spoken by its molecular formula.
- Commutative diagrams from tikz-cd and xymatrix, besides amscd: arrows in any direction, bent, dashed, hooked and labelled.
- A gallery page on the site, by group, and more famous and big equations in the README, Navier–Stokes among them.
- The shipped wasm is built for size, and the embedded math fonts are subsets, a third smaller.

Equations: `caffeine` `sn2` `tikzcd` `xymatrix` `navierstokes`

## 1.3.0 - 2026-10-25

Meaning read aloud: number sets, units and the dot product.

- Double-struck letters are read as the number sets they name (L-LET-2), the dot between vectors as the dot product (L-OPR-1), and siunitx units by name in English and Portuguese (L-UNI-3).
- Braces and their labels placed by the ink of the brace; binomials say and write their parentheses once; large operators spaced from their operand in the plain text.
- The site gets one menu on every page, a documentation page, code samples colored by language, the 60-second demo video and a Sponsor link.

Equations: `units` `braces` `binomial`

## 1.2.0 - 2026-10-21

A second font: Latin Modern Math, the OpenType Computer Modern.

- Latin Modern Math as a second font, behind the `latin-modern` feature, chosen with `convert_in_font` and `typeset_in_font`.
- Words in text as far apart as the font says: Latin Modern Math's space is a third of an em.

Equations: `einstein`

## 1.1.1 - 2026-10-20

Braille checked against the codes.

- Nemeth and CMU signs settled against BANA 2022 and the 2006 CMU, and the review against MathCAT reported with the tool that repeats it; the reference page tells how far to trust the braille.
- MathJax's MathML and command arguments without braces read as meant.

Equations: `quadratic` `\frac{3}{4}`

## 1.1.0 - 2026-10-20

MathML and AsciiMath in, braille out.

- MathML and AsciiMath input, beside LaTeX.
- Every equation in braille, in the CMU and in Nemeth, the code chosen by the language.
- amssymb's negations, mhchem, physics, siunitx ranges and mathtools structures read.
- The site chooses LaTeX, MathML or AsciiMath, shows the braille and has a reference page; one script releases every package.

Equations: `chemistry` `\nleq \nsim \nRightarrow \lneqq` `\expval{H}{\psi} = \mel{\psi}{H}{\psi}`

## 1.0.1 - 2026-10-20

Sixteen languages, and mathra on npm and PyPI.

- Equations spoken and written in 16 languages.
- The npm package `@mathlabs/mathra` for Node, bundlers and browsers, and the Python package on PyPI with the same API.
- A browser playground on GitHub Pages, compared with KaTeX and MathJax; `render` returns every output as one JSON.
- `|x|` read as absolute value and `‖v‖` as norm; function scripts stay attached to the name in the linear text.

Equations: `euler` `ptbr` `\left| x \right| + \left\| v \right\|`

## 1.0.0 - 2026-10-17

Math LaTeX to a tree of meaning, accessible SVG, speech and text.

- The core: LaTeX read into a tree of meaning and drawn with explicit, numbered rules, as SVG that carries its speech, source and tree.
- The `mathra` command with `--explain`, `--batch` and the speech, text and tree outputs; `tex_to_svg` and `tex_to_tree` for Python, `texToSvg` and `texToTree` for wasm.
- TeX's quote ligatures in text become typographic quotes; a benchmark over a hand-written corpus; a README banner drawn by mathra itself.

Equations: `gaussian` `maxwell` `schrodinger`
