<p align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/banner.svg" alt="mathra" width="100%"></p>

# mathra

Math LaTeX to accessible SVG, without a TeX install, chemistry (chemfig structures, reaction schemes, mechanisms) and commutative diagrams (tikz-cd, xymatrix) included. Every SVG carries the phrase a screen reader says, in 16 languages, and draws with the STIX Two Math font embedded, so it looks the same everywhere. **[Try it in the browser](https://matheusmota-dev-br.github.io/mathra/)**.

```sh
pip install mathra
```

Wheels for Linux, macOS and Windows, for Python 3.9 and later; nothing else to install.

## Usage

```python
import mathra

svg = mathra.tex_to_svg(r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}")
open("quadratic.svg", "w", encoding="utf-8").write(svg)

r = mathra.render(r"\frac{3}{4}", locale="fr")
r["speech"]        # 'trois quarts'
r["warnings"]      # [] — broken LaTeX is drawn anyway and reported here (E-REC-*)

mathra.tex_to_svg(r"\sum_{n=1}^{\infty} \frac{1}{n^2}", display=False)   # inline style
mathra.tex_to_svg(r"\vertA\vert", strict=True)                           # ValueError, like TeX
mathra.render("sqrt(x)/2", input="asciimath")["speech"]                   # also input="mathml"
mathra.render(r"\chemfig{H_3C-*6(=-=-=-)}")["speech"]                     # 'structure of C7 H8, with 1 aromatic ring'
```

| | |
|---|---|
| `tex_to_svg(tex, locale="en", *, display=True, strict=False, input="latex", braille=None, animate=False, steps=False)` | The SVG as a string. |
| `render(tex, locale="en", *, display=True, strict=False, input="latex", braille=None, animate=False, steps=False)` | A dict: `svg`, `speech`, `text`, `braille`, `braille_code`, `tree`, `applied`, `warnings`. |
| `tex_to_tree(tex, locale="en", *, display=True, strict=False, input="latex", braille=None, animate=False, steps=False)` | The tree of meaning as JSON. |
| `locales()` | `[{"code": "en", "name": "English"}, …]`: en, pt-BR, es, fr, de, it, ru, zh, ja, ko, hi, ar, id, tr, nl, pl. |

`input` is `"latex"`, `"mathml"` or `"asciimath"`; MathML passed as LaTeX, starting with `<math`, is recognized on its own. `braille` is `"cmu"` or `"nemeth"`; by default the locale's (CMU in pt-BR and es, Nemeth elsewhere). Errors are `ValueError`. Types are included (`py.typed`).

## What it draws

Every image below is an SVG written by mathra, on a white card so it reads well on dark pages.

<table>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/quadratic.svg" alt="Quadratic formula"><br><sub>Quadratic formula</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/gaussian.svg" alt="Gaussian integral"><br><sub>Gaussian integral</sub></td></tr>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/matrix.svg" alt="m × n matrix with dots"><br><sub>Matrix with dots</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/piecewise.svg" alt="Piecewise definition"><br><sub>Piecewise definition</sub></td></tr>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/chemistry.svg" alt="Chemical equation (mhchem)"><br><sub>Chemistry (<code>\ce</code>)</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/units.svg" alt="Quantity with units (siunitx)"><br><sub>Units (<code>\SI</code>)</sub></td></tr>
<tr><td align="center" colspan="2"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/maxwell.svg" alt="Maxwell's equations"><br><sub>Maxwell's equations</sub></td></tr>
<tr><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/caffeine.svg" alt="Caffeine: fused rings (chemfig)"><br><sub>Caffeine (<code>\chemfig</code>)</sub></td><td align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/tikzcd.svg" alt="Pullback with bent arrows (tikz-cd)"><br><sub>Commutative diagram (tikz-cd)</sub></td></tr>
<tr><td align="center" colspan="2"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/mathra/main/docs/examples/nitration.svg" alt="Nitration of benzene: structures, mhchem and conditions on the arrow"><br><sub>Reaction scheme (<code>\schemestart</code>)</sub></td></tr>
</table>

## Chemistry and diagrams

chemfig works inside any equation: structures (rings, wedges, Lewis marks), reaction schemes (`\schemestart … \arrow{->[above][below]} … \schemestop`) and mechanism arrows (`\chemmove`). The speech gives each structure's molecular formula, counted from the drawing. Commutative diagrams come in amscd, tikz-cd and xymatrix. Every construction, drawn live, is in the [gallery](https://matheusmota-dev-br.github.io/mathra/gallery.html) and the [reference](https://matheusmota-dev-br.github.io/mathra/reference.html).

## What the SVG carries

Speech in `<title>` linked with `aria-labelledby`, the source equation and the tree of meaning in `<metadata>`, color from the page (`currentColor`), size in `em`, a selectable text layer and ids unique to each equation. The same input always gives the same bytes, in Python and in every other package.

With `animate=True` the equation writes itself in the browser, with CSS alone: each glyph fades in, each rule grows and each line is drawn, in the order the layout set them, in about three seconds (nothing moves with `prefers-reduced-motion`); `steps=True` numbers each element by that order (`data-step`) with no animation. Both are off by default, and without them the SVG is unchanged byte for byte.

## Also for JavaScript, Rust and the terminal

The same core is an npm package (`@mathlabs/mathra`), a Rust crate and a command-line tool: [github.com/matheusmota-dev-br/mathra](https://github.com/matheusmota-dev-br/mathra#install-and-use). Rules, languages and the full list of supported commands are there too.

## License

The code is MIT or Apache-2.0; the embedded STIX Two Math font is under the SIL Open Font License 1.1.
