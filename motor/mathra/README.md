<p align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/banner.svg" alt="mathra" width="100%"></p>

# mathra

Accessible SVG equations from LaTeX, chemistry included, without a TeX install, for **JavaScript, Python, Rust and the terminal**. **[Try it in the browser](https://matheusmota-dev-br.github.io/mathra/)**, or **[watch the 60-second demo](https://youtu.be/pXuaCwmWaDg)**.

<p align="center"><a href="https://youtu.be/pXuaCwmWaDg"><img src="https://img.youtube.com/vi/pXuaCwmWaDg/maxresdefault.jpg" alt="mathra in 60 seconds: watch the demo on YouTube" width="720"></a></p>

mathra reads math LaTeX (and chemfig structures, reaction schemes and commutative diagrams), builds a **tree of meaning** (what each piece is) and draws it with **explicit rules**, numbered and tested. Every SVG carries the phrase a screen reader says, in 16 languages, and the locale decides the form: in pt-BR `3,14` is a single number and `\sin` comes out as `sen`. All four packages run the same core, so an equation gives the same SVG, byte for byte, wherever it is converted.

```console
$ mathra '3,14' --locale pt-BR --explain
input    3,14
locale   pt-BR
text     3,14
speech   3,14
braille  ⠼⠉⠂⠁⠙ (cmu)
tree     {"type":"row","children":[{"type":"number","value":"3.14"}]}
rules
  L-NUM-1   '3,14': comma between digits read as decimal separator (3.14)
  L-NUM-2   3.14 written with ',' as decimal separator (pt-BR)

$ mathra 'f(1, 5) = \sin x - 2.5' -o function.svg
```

## Examples

Every image below is an SVG written by mathra; the LaTeX for each one is in [`crates/core/tests/examples.rs`](https://github.com/matheusmota-dev-br/mathra/blob/main/crates/core/tests/examples.rs), which also checks that the files match the code. They are shown on a white card so they read well on dark pages too.

### Famous equations

<table>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/euler.svg" alt="Euler's identity"><br><sub>Euler's identity</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/gaussian.svg" alt="Gaussian integral"><br><sub>Gaussian integral</sub></td></tr>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/basel.svg" alt="Basel problem"><br><sub>Basel problem</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/quadratic.svg" alt="Quadratic formula"><br><sub>Quadratic formula</sub></td></tr>
<tr><td align="center" colspan="2"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/maxwell.svg" alt="Maxwell's equations"><br><sub>Maxwell's equations</sub></td></tr>
<tr><td align="center" colspan="2"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/schrodinger.svg" alt="Schrödinger equation"><br><sub>Schrödinger equation</sub></td></tr>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/einstein.svg" alt="Einstein field equations"><br><sub>Einstein field equations</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/cauchy.svg" alt="Cauchy's integral formula"><br><sub>Cauchy's integral formula</sub></td></tr>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/fourier.svg" alt="Fourier transform"><br><sub>Fourier transform</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/bayes.svg" alt="Bayes' theorem"><br><sub>Bayes' theorem</sub></td></tr>
<tr><td align="center" colspan="2"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/navierstokes.svg" alt="Navier–Stokes equations (incompressible)"><br><sub>Navier–Stokes equations (incompressible)</sub></td></tr>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/stokes.svg" alt="Stokes' theorem"><br><sub>Stokes' theorem</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/eulerlagrange.svg" alt="Euler–Lagrange equation"><br><sub>Euler–Lagrange equation</sub></td></tr>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/zeta.svg" alt="Riemann zeta as an Euler product"><br><sub>Riemann zeta as an Euler product</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/dirac.svg" alt="Dirac equation"><br><sub>Dirac equation</sub></td></tr>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/planck.svg" alt="Planck's law"><br><sub>Planck's law</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/ramanujan.svg" alt="Ramanujan's series for 1/π"><br><sub>Ramanujan's series for 1/π</sub></td></tr>
<tr><td align="center" colspan="2"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/blackscholes.svg" alt="Black–Scholes equation"><br><sub>Black–Scholes equation</sub></td></tr>
</table>

### Big equations

Systems, long derivations and dense tensor notation, each in one call, in strict mode:

<table>
<tr><td align="center" colspan="2"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/navierstokes3d.svg" alt="Navier–Stokes in Cartesian components, with continuity"><br><sub>Navier–Stokes in Cartesian components, with continuity</sub></td></tr>
<tr><td align="center" colspan="2"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/compressible.svg" alt="Compressible Navier–Stokes, with the viscous stress tensor"><br><sub>Compressible Navier–Stokes, with the viscous stress tensor</sub></td></tr>
<tr><td align="center" colspan="2"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/riemann.svg" alt="Riemann curvature tensor and Christoffel symbols"><br><sub>Riemann curvature tensor and Christoffel symbols</sub></td></tr>
<tr><td align="center" colspan="2"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/qed.svg" alt="Lagrangian of quantum electrodynamics"><br><sub>Lagrangian of quantum electrodynamics</sub></td></tr>
<tr><td align="center" colspan="2"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/gaussproof.svg" alt="The Gaussian integral, step by step in polar coordinates"><br><sub>The Gaussian integral, step by step in polar coordinates</sub></td></tr>
<tr><td align="center" colspan="2"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/vandermonde.svg" alt="Vandermonde determinant"><br><sub>Vandermonde determinant</sub></td></tr>
</table>

### Beyond the basics

Continued fractions, matrices with dots, `cases`, binomials, labeled braces, chemistry (`\ce`), units (`\SI`), commutative diagrams (amscd, tikz-cd, xymatrix), the pt-BR locale and computed long division:

<table>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/golden.svg" alt="Golden ratio as a continued fraction"><br><sub>Golden ratio as a continued fraction</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/matrix.svg" alt="m × n matrix with dots"><br><sub>m × n matrix with dots</sub></td></tr>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/piecewise.svg" alt="Piecewise definition"><br><sub>Piecewise definition</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/binomial.svg" alt="Binomial theorem"><br><sub>Binomial theorem</sub></td></tr>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/braces.svg" alt="Labeled brace"><br><sub>Labeled brace</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/chemistry.svg" alt="Chemical equation (mhchem)"><br><sub>Chemical equation (mhchem)</sub></td></tr>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/units.svg" alt="Quantity with units (siunitx)"><br><sub>Quantity with units (siunitx)</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/diagram.svg" alt="Commutative diagram (amscd)"><br><sub>Commutative diagram (amscd)</sub></td></tr>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/tikzcd.svg" alt="Pullback with bent arrows (tikz-cd)"><br><sub>Pullback with bent arrows (tikz-cd)</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/xymatrix.svg" alt="Short exact sequence (xymatrix)"><br><sub>Short exact sequence (xymatrix)</sub></td></tr>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/ptbr.svg" alt="pt-BR locale: sen, decimal comma, open interval"><br><sub>pt-BR locale: sen, decimal comma, open interval</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/division.svg" alt="Long division, computed (pt-BR)"><br><sub>Long division, computed (pt-BR)</sub></td></tr>
</table>

### Structural chemistry

Structures, reaction schemes and mechanisms in chemfig's language, inside any equation, read aloud by the molecular formula mathra counts from each structure (`spec/chemistry.md`):

<table>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/caffeine.svg" alt="Caffeine: fused rings (chemfig)"><br><sub>Caffeine: fused rings (chemfig)</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/sn2.svg" alt="SN2 mechanism: lone pairs and curved arrows"><br><sub>SN2 mechanism: lone pairs and curved arrows</sub></td></tr>
<tr><td align="center" colspan="2"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/nitration.svg" alt="Nitration of benzene: structures, mhchem and conditions on the arrow"><br><sub>Nitration of benzene: structures, mhchem and conditions on the arrow</sub></td></tr>
<tr><td align="center" colspan="2"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/esterification.svg" alt="Esterification and reduction: equilibrium, a reagent drawn over a vertical arrow (pt-BR)"><br><sub>Esterification and reduction: equilibrium, a reagent drawn over a vertical arrow (pt-BR)</sub></td></tr>
</table>

### What a screen reader hears

Each SVG carries the equation as speech in its `<title>`:

| Equation | Speech (`--speech`) |
|---|---|
| `x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}` | x equals fraction, negative b plus or minus square root of b squared minus 4 a c, end root, over 2 a, end fraction |
| `\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}` | sum from n equals 1 to infinity of 1 over n squared equals pi squared over 6 |
| `\lvert a \rvert + \lvert b \rvert \ge \lvert a + b \rvert` | absolute value of a plus absolute value of b is greater than or equal to absolute value of, a plus b, end absolute value |
| `\divchave{247}{5}` with `--locale pt-BR` | divisão armada de 247 por 5: quociente 49, resto 2 |
| `\begin{tikzcd} A \arrow[r, "f"] \arrow[d, "g"'] & B \\ C \end{tikzcd}` | diagram: row 1: A, B; row 2: C; arrow from A to B (f); arrow from A to C (g) |
| `\chemfig{H_3C-*6(=-=-=-)}` | structure of C7 H8, with 1 aromatic ring |
| `\schemestart \ce{N2 + 3H2} \arrow{<=>[Fe]} \ce{2NH3} \schemestop` | N sub 2 plus 3 H sub 2 in equilibrium with (Fe over the arrow) 2 N H sub 3 |

## Broken LaTeX

By default mathra draws what it can and returns **warnings** for what it had to fix (`E-REC-1` to `E-REC-10`, in `spec/tolerance.md`): unknown commands, unbalanced braces, double scripts, leftover JSON escapes such as `\n` or `\u00b2`, unknown MathML elements. Warnings show up in `--explain`, on stderr, in `Output::warnings` and in the SVG's `<metadata>`. With `--strict`, mathra refuses invalid input instead, like TeX.

```console
$ mathra '\vertA\vert = C_{16,2}' --text
mathra: warning E-REC-1: column 1: unsupported command: \vertA; read as \vert A
|A| = C_(16, 2)
```

## LaTeX, MathML and AsciiMath

LaTeX is the default. MathML (what Word, Google Docs, MathType and MathJax export) and AsciiMath (`sum_(i=1)^n i^2`) are read into the same tree of meaning, so they come out with the same drawing and the same speech. MathML passed as LaTeX, starting with `<math`, is recognized on its own. What each notation covers is in [`spec/input.md`](https://github.com/matheusmota-dev-br/mathra/blob/main/spec/input.md), and the [reference page](https://matheusmota-dev-br.github.io/mathra/reference.html) shows every construction, in AsciiMath (the tables of asciimath.org), LaTeX (by topic, as Overleaf's Mathematics articles), the science packages and MathML, each drawn and in braille, live in the browser.

```console
$ mathra 'sum_(i=1)^n i^2 = (n(n+1)(2n+1))/6' --from asciimath -o sum.svg
$ mathra '<math><msqrt><mi>x</mi></msqrt></math>' --speech      # square root of x
```

## Braille

Every equation also comes out in braille, as Unicode braille cells: in the **CMU** (Código Matemático Unificado) for Portuguese and Spanish, and in **Nemeth** for the other languages, or in either one by choice. Both follow their official documents, and the tests check them against the documents' own examples; what is covered and what is still open is in [`spec/braille.md`](https://github.com/matheusmota-dev-br/mathra/blob/main/spec/braille.md). The braille also goes into the SVG's `<metadata>`. It was compared with MathCAT row by row, with the official documents deciding every disagreement ([`docs/braille-review.md`](https://github.com/matheusmota-dev-br/mathra/blob/main/docs/braille-review.md), and the Braille tab of the [reference page](https://matheusmota-dev-br.github.io/mathra/reference.html#braille)); it still needs a braille transcriber's review before it reaches students.

```console
$ mathra '\frac{3}{4} + \sqrt{x}' --locale pt-BR --braille     # ⠼⠒⠙⠖⠫⠱⠭
$ mathra 'x^2 + 1' --braille                                     # ⠭⠘⠆⠐⠬⠂ (Nemeth)
$ mathra 'x^2 + 1' --braille --braille-code cmu                  # ⠭⠡⠼⠃⠖⠼⠁
```

## Locales

16 languages: `en` (default), `pt-BR`, `es`, `fr`, `de`, `it`, `ru`, `zh`, `ja`, `ko`, `hi`, `ar`, `id`, `tr`, `nl` and `pl`, with `--locale` (`locale="fr"` in Python, `{ locale: "fr" }` in JavaScript). The locale changes how the equation is read, written and spoken: in pt-BR `3,14` is one number with a decimal comma, `1.000` is one thousand, `\sin` is written `sen` and `]a, b[` is an open interval; in fr `\frac{3}{4}` is spoken "trois quarts"; in zh `\frac{1}{2}` is "2 分之 1". Each language is a JSON file in `crates/core/src/locales/`, described in [`spec/locales.md`](https://github.com/matheusmota-dev-br/mathra/blob/main/spec/locales.md) with how to add one. English and pt-BR are reviewed; the other languages still need a native speaker's review of the speech. Every rule that depends on the locale is numbered in `spec/rules.md`.

## What the SVG carries

Besides the outlines, every SVG follows the contract in `spec/svg.md`: speech for screen readers, the source equation, the tree and the braille in `<metadata>`, color inherited from the page, reused glyphs, a text layer for selection and unique ids per equation. The same input always produces the same bytes.

### The equation writes itself

With `--animate` (`{ animate: true }` in JavaScript, `animate=True` in Python, `animate: true` in Rust) the SVG writes itself in the browser, with CSS alone: each glyph fades in, each rule grows along its length and each line is drawn, in the order the layout set them, in about three seconds. `--steps` numbers each element (`data-step`) with no CSS, for a page that walks through the equation; nothing moves with `prefers-reduced-motion`, and without either option the SVG is unchanged byte for byte (S-SVG-10). The [playground](https://matheusmota-dev-br.github.io/mathra/) walks through an equation step by step and downloads the animation as a GIF, a video or an animated SVG with its own play bar; every equation of the [gallery](https://matheusmota-dev-br.github.io/mathra/gallery.html) animates too.

```console
$ mathra 'x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}' --animate -o quadratic.svg
```

What changed in each release is in [`CHANGELOG.md`](CHANGELOG.md), and on the site's [evolution page](https://matheusmota-dev-br.github.io/mathra/changelog.html) with the equations drawn live.

## Install and use

| Where | Install | Registry |
|---|---|---|
| JavaScript and TypeScript: Node, bundlers, browsers | `npm install @mathlabs/mathra` | [npm](https://www.npmjs.com/package/@mathlabs/mathra) |
| Python 3.9+: wheels for Linux, macOS and Windows | `pip install mathra` | [PyPI](https://pypi.org/project/mathra/) |
| Terminal | `cargo install mathra-cli` | [crates.io](https://crates.io/crates/mathra-cli) |
| Rust | `cargo add mathra` | [crates.io](https://crates.io/crates/mathra) · [docs.rs](https://docs.rs/mathra) |

### JavaScript and TypeScript

Types included. In Node it works right away; in browsers and bundlers, await `init()` once. More in [`npm/README.md`](https://github.com/matheusmota-dev-br/mathra/blob/main/npm/README.md).

```js
import { init, texToSvg, render } from "@mathlabs/mathra";

await init();                                                    // browsers and bundlers; Node needs nothing
const svg = texToSvg("3,14", { locale: "pt-BR" });
const { speech, warnings } = render(String.raw`\frac{3}{4}`, { locale: "fr" });   // "trois quarts"
render("sqrt(x)/2", { input: "asciimath" });                     // also "mathml"
render("x^2", { locale: "pt-BR" }).braille;                      // "⠭⠡⠼⠃" (CMU); { braille: "nemeth" } forces the code
```

### Python

```python
import mathra

svg = mathra.tex_to_svg(r"\sin x = 0.5")                 # <title>…sine of x equals 0.5</title>
r = mathra.render(r"\frac{3}{4}", locale="fr")            # dict: svg, speech, text, braille, braille_code, tree, applied, warnings
r["speech"]                                               # 'trois quarts'
mathra.tex_to_svg(r"\vertA\vert", strict=True)            # ValueError, like TeX
mathra.render("sqrt(x)/2", input="asciimath")             # also input="mathml"
mathra.render("x^2", locale="pt-BR")["braille"]           # '⠭⠡⠼⠃' (CMU); braille="nemeth" forces the code
```

Types included; more on [PyPI](https://pypi.org/project/mathra/).

### Terminal

```console
$ mathra 'x \le 2.75; y \ge 1' -o inequality.svg
$ mathra '3,14 + \sin x' --locale pt-BR
$ mathra 'x = -1' --speech        # x equals negative 1
$ echo 'a + b' | mathra --tree
$ mathra 'a/b + sqrt(c)' --from asciimath
$ mathra '\frac{3}{4}' --locale pt-BR --braille   # ⠼⠒⠙
$ mathra --batch -o out/ < equations.txt
```

### Rust

```rust
let r = mathra::convert("3.14", &mathra::Options::default())?;
println!("{}", r.svg);
println!("{}", r.speech);
```

Structural chemistry is the `chem` feature, on by default; `mathra = { version = "…", default-features = false }` leaves it out of a build that only needs equations.

## What it converts

Layout follows the parameters of the embedded font's MATH table. The font is STIX Two Math; with the
`latin-modern` feature, `convert_in_font` and `typeset_in_font` can also set an equation in Latin
Modern Math, the OpenType Computer Modern of TeX's own documents (about 430 KB more in the binary).

- Fractions (`\frac`, `\dfrac`, `\tfrac`, `\cfrac`, `\over`), binomials, roots with an index
- Scripts, primes (`f'`), limits (`\lim`, `\sum`, `\prod`, `\int` and family, `\limits`)
- `\left … \middle … \right`, `\big` and family, delimiters assembled from parts
- Matrices (`matrix`, `pmatrix`, `bmatrix`, `Bmatrix`, `vmatrix`, `Vmatrix`, `smallmatrix`), `cases`, `rcases`, `aligned`, `align`, `gathered`, `split`, `array` with `|`, `@{}`, `\hline`, `\cline`, arydshln's dashed `:`, `\hdashline` and `\cdashline`, and `\arraystretch`
- What LaTeX's own packages add to the KaTeX list: the colons of mathtools and colonequals (`\coloneqq`, `\Colonsim`…), `\varGamma` and family, `\injlim`, `\intop`, `\lgroup`, `\char`, `\xtwoheadrightarrow` and family, text symbols (`\AE`, `\H{o}`, `\textdollar`), `\url` and `\href`; TeX's `\mathchoice`, `\vcenter` and `\above`, and graphicx's `\reflectbox`
- Accents (`\hat`, `\vec`, `\bar`, `\dot`, `\widehat`, `\overrightarrow`, `\arc`…), `\underrightarrow` and family, `\overline`, `\underline`, `\overbrace`/`\underbrace` and `\overbracket`/`\underbracket` with labels, `\xrightarrow` and family, `\overset`, `\underset`, `\stackrel`
- The amssymb symbols, negations included (`\nleq`, `\nsim`, `\nRightarrow`, `\lneqq`…), and integrals up to `\iiiint`, `\idotsint` and `\varoiint`
- `\text` with `$…$`, `\color` and `\textcolor`, math environments (`align*`) and old-style accents (`\'e`, `\c{c}`), `\textsc`, alphabets (`\mathbb`, `\mathcal`, `\mathfrak`, `\mathbf`, `\boldsymbol`, `\mathsf`, `\mathtt`), old `\bf`/`\rm`/`\tt`
- `\cancel`, `\boxed`, `\textcircled`, `\phase` (steinmetz), `\angl` (actuarialangle), `\color`, `\colorbox`, `\fcolorbox`, `\phantom`, `\pmod`, `\operatorname`, spaces (`\,` `\quad` `\hspace`)
- siunitx (`\SI`, `\qty`, `\si`, `\num`, `\ang` with minutes and seconds, ranges, lists and products), HTML entities coming from editors
- structural chemistry (chemfig, the `chem` feature, on by default): `\chemfig{…}` with labels split into atoms, single, double and triple bonds, wedges and hashes, angles (`[2]`, `[:30]`, `[::60]`), branches, rings (aromatic, fused), distant bonds (`?`), Lewis marks (`\lewis`); reaction schemes (`\schemestart`, `\arrow{->[above][below]}` with every chemfig arrow and any angle, `\+`, `\chemname`); mechanisms (`\chemmove` with curved arrows and fishhooks between `@{hooks}`). Speech and text give each structure's molecular formula, counted from the graph with its implicit hydrogens
- mhchem (`\ce`, `\pu`): formulas, charges, radicals, isotopes, hydrates, bonds, decimal and fractional coefficients, arrows with labels (`->[\Delta]`, `<=>[K_w]`), `\frac` with formulas
- Substitution macros: `\newcommand`, `\renewcommand`, `\def`, `\let`, `\DeclareMathOperator`, `\DeclarePairedDelimiter`, `\newenvironment`
- Packages: physics (derivatives, `\dd`, `\eval`, `\expval`, `\mel`, `\comm`, `\grad`, `\curl`, `\laplacian`, `\vb`, `\vu`, `\order`, `\Tr`, `\qty`), braket (`\ket`, `\braket`, `\Set`), tensor (`\tensor`, `\indices`), mathtools (`\prescript`, `rcases`, `\overbracket`), xfrac, units, `\genfrac`, `\substack`, `\sideset`, `\smash`, `\mathclap`, `\multicolumn`, `\intertext`, `\hdotsfor`, `\raisebox`, `\scalebox`, `\rule`, `\verb`; old syntax `\cases`, `\matrix`, `\eqalign`
- commutative diagrams: amscd (`\begin{CD}`), tikz-cd (`\begin{tikzcd}`, `\arrow[dr, "f"', dashed, bend left]`) and xymatrix (`\xymatrix{A \ar[r]^f & B}`): arrows in any direction, bent or shifted, with labels beside or on them, dashed, dotted, double, squiggly, hooked, two-headed, maps-to; pullback corners (`\lrcorner`)
- mathra extensions: `\divchave{247}{5}` (long division, Brazilian layout) and `\contaarmada{+}{347}{285}` (column arithmetic), both computed

Out of scope, by the rule in `spec/boundary.md` (mathra is for equations): TikZ and TeX programming (`\ifx`, `\catcode`). Not yet: automatic line breaking, MathML and PNG output (MathML input is supported), content MathML, mhchem's text mode (`\ce{Hydrogen + Oxygen -> Water}` is read as formulas).

## Benchmark

`corpus/` holds 116 hand-written constructions, one or a few per command family and package. One run of `cargo run --release -p mathra --example bench` (arm64, Apple M5, rustc 1.99.1):

| Measure | Value |
|---|---|
| Constructions | 116 |
| Strict: converted | 112 (96.6%) |
| Tolerant: no warning | 112 (96.6%) |
| Tolerant: with warnings | 4 (3.4%) |
| Tolerant: error | 0 (0.0%) |
| Time per equation, mean | 17.9 µs |
| Time per equation, p95 | 43.8 µs |

Times vary by a few microseconds between runs. The 4 that strict mode refuses are not equations: TeX programming (`\ifx`, `\expandafter`, `\catcode`) and a drawing with coordinates (`\tikz`). Tolerant mode draws them as text with a warning.

### Compared with KaTeX and MathJax

`bench/compare.mjs` runs the three libraries on the same equations in Node (`cd bench && npm install && npm run compare`; Node 24, Apple M5, katex 0.16.22, mathjax-full 3.2.2). "Clean" means no error and no command drawn in red. The [playground](https://matheusmota-dev-br.github.io/mathra/#benchmark) runs the same comparison in your browser.

| | mathra (wasm) | MathJax (SVG) | KaTeX (HTML) |
|---|---|---|---|
| Playground examples, 18: clean | 18 | 15 | 15 |
| Playground examples: time per equation, mean | 135 µs | 578 µs | 65 µs |
| Corpus, 116 constructions: clean | 112 | 70 | 62 |
| Output per equation, gzip | 2.42 KB | 2.43 KB | 0.95 KB + CSS and fonts |
| Download to render in a browser, gzip | 782 KB | 660 KB | 77 KB + up to 254 KB of fonts |

KaTeX is faster and lighter to ship, but its output is HTML that needs its stylesheet and fonts, not a standalone image. Against MathJax's SVG, the closest equivalent, mathra is about 4× faster with output of similar size. mathra's download is the wasm and its JavaScript glue; almost half of it is the embedded font (356 KB gzip), cut down to what mathra draws (`tools/fonts/subset.sh`). Structural chemistry adds about 50 KB, and a build without the `chem` feature leaves it out. The corpus was written while building mathra, so it favors it; the three playground examples the others miss use `\SI`, `\sen` and `\divchave`.

## Layout

```
crates/core          tree, rules, layout, embedded fonts, outputs
crates/core/src/input     readers: LaTeX, MathML, AsciiMath (spec/input.md)
crates/core/src/output    SVG, speech, text and braille (CMU and Nemeth, spec/braille.md)
crates/core/src/locales   the 16 languages, one JSON file each (spec/locales.md)
crates/core/src/chem      structural chemistry: chemfig, schemes, mechanisms (spec/chemistry.md, `chem` feature)
crates/core/examples bench (the benchmark above) and banner (the image above)
crates/cli           `mathra` binary
crates/python        Python module (PyO3 + maturin)
crates/wasm          WebAssembly bindings (wasm-bindgen), used by npm/ and site/
npm/                 the npm package: JavaScript API, types, build and tests
site/                the playground and docs on GitHub Pages
bench/               mathra next to KaTeX and MathJax
spec/                numbered rules (rules.md), the SVG contract (svg.md), inputs, braille, languages
corpus/              hand-written constructions used by the benchmark and the reference page
tools/braille-compare  mathra's braille next to MathCAT's (docs/braille-review.md)
tools/fonts          the original fonts and subset.sh, which writes the embedded ones in crates/core/fonts
scripts/release.sh   tags every package version not yet on its registry
docs/                tests (tests.md), the braille review, the banner
docs/examples/       the README gallery, generated by tests/examples.rs
```

## Development

```console
$ cargo test                 # one test per rule, with the rule id in its name
$ cargo clippy --all-targets
$ cargo run -p mathra-cli -- '3,14' --locale pt-BR --explain
$ npm/build.sh && npm/test/run.sh     # the npm package, packed and installed as users get it
$ site/build.sh                       # then open site/index.html
$ scripts/release.sh --dry-run        # what a release would publish
```

**Releasing.** Each package has its own version: `Cargo.toml` (crates `mathra` and `mathra-cli`), `npm/package.json` and `crates/python/Cargo.toml`. Bump the ones to release in a pull request, merge it, then run `scripts/release.sh` on `main`: it tags every version not yet on its registry (`v*`, `npm-v*`, `py-v*`) and pushes the tags at once; each tag runs its workflow, which publishes with trusted publishing. A rerun of a half-done release skips what is already out.

The checks run on your machine: `git config core.hooksPath .githooks` once, and every commit runs fmt, clippy and the Rust tests, every push the wasm, npm and Python tests (`scripts/check.sh` runs them all by hand). `.github/workflows/ci.yml` runs the same checks again on `main` after each merge.

To change behavior, write the rule in `spec/`, cite its id in the code and add a test with the same id.

The tests cover 97.4% of the lines of the core, the CLI and the Python module (100% of the CLI, the Python module and the SVG), measured on 1.2.0, before structural chemistry; the gaps are in the MathML and AsciiMath readers and the braille codes. [docs/tests.md](https://github.com/matheusmota-dev-br/mathra/blob/main/docs/tests.md) explains how to run each suite (including wasm on Node and pytest), what each file checks, the coverage per file and the problem inputs already tested.

The banner above is drawn by mathra itself (`cargo run -p mathra --example banner`); `tests/banner.rs` fails if it goes stale.

## Sponsor

mathra is free and open source. If it is useful to you, you can support its work on [GitHub Sponsors](https://github.com/sponsors/matheusmota-dev-br).

## Licenses

Code: your choice of [MIT](https://github.com/matheusmota-dev-br/mathra/blob/main/LICENSE-MIT) or [Apache-2.0](https://github.com/matheusmota-dev-br/mathra/blob/main/LICENSE-APACHE), as is common in the Rust ecosystem. Embedded font: STIX Two Math, SIL Open Font License 1.1 (`crates/core/fonts/OFL-STIXTwoMath.txt`). With the `latin-modern` feature, also Latin Modern Math, GUST Font License (`crates/core/fonts/GUST-FONT-LICENSE-LatinModernMath.txt`).
