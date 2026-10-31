<p align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/banner.svg" alt="tikra: TikZ to SVG and PNG, no TeX install" width="100%"></p>

# tikra

TikZ and pgfplots figures to SVG and PNG, without a TeX install: no LaTeX, no fonts on the system, no external processes. It reimplements TikZ, pgf and pgfplots, and labels are set by [mathra](https://github.com/matheusmota-dev-br/mathra) in Latin Modern, TeX's own Computer Modern. **[Try it in the browser](https://matheusmota-dev-br.github.io/tikra/)** · **[See the gallery](https://matheusmota-dev-br.github.io/tikra/gallery.html)**.

```sh
pip install tikra
```

Wheels for Linux, macOS and Windows, for Python 3.9 and later; nothing else to install.

## Usage

```python
import tikra

svg = tikra.tikz_to_svg(r"""
\begin{tikzpicture}
  \draw[->] (0,0) -- (2,1) node[right] {$\vec v$};
\end{tikzpicture}
""")
open("v.svg", "w", encoding="utf-8").write(svg)
open("v.png", "wb").write(tikra.png(svg, dpi=300))

r = tikra.render(r"\tikz \draw (0,0) circle (1);")
r["warnings"]      # [] — what tikra skips or approximates is listed here, and the rest is drawn
r["alt"]           # 'Drawing of one shape, without text' — the figure in words, for those who cannot see it
```

| | |
|---|---|
| `tikz_to_svg(code, *, locale="en")` | The SVG as a string. `locale` (`en`, `pt-BR`, `es`, `fr`, `de`, `it`, `ru`, `zh`, `ja`, `ko`, `hi`, `ar`, `id`, `tr`, `nl`, `pl`) is the language of the figure in words; the drawing is the same in all. |
| `render(code, *, locale="en")` | A dict, the words in `locale`: `svg`; `alt`, the figure in one sentence (the SVG's `<title>`, for an image's alt text); `description`, the figure in words (the SVG's `<desc>`: a chart's axes and data, a diagram's nodes and connections); and `warnings` (a list of strings). When the picture has its author's text (`\begin{tikzpicture}[alt={…}]`), `alt` and `description` are that text. The words tikra writes are not perfect (they say what is in the drawing, not what it means, and little about free drawings): review them before publishing. |
| `png(svg, *, dpi=300, background=True)` | PNG bytes; `background=False` keeps it transparent instead of white. |

`code` is a full `standalone` document, a bare `tikzpicture`, an inline `\tikz`, an `axis` environment or loose TikZ statements. Errors (no figure in the input, an invalid dpi, a PNG past 100 megapixels) are `ValueError`. Types are included (`py.typed`).

## What it draws

Every image below is an SVG written by tikra from a figure in the repository's corpus, on a white card; the [gallery](https://matheusmota-dev-br.github.io/tikra/gallery.html) has all 227. Of the 2502 figures of the TikZ and pgfplots manuals, tikra draws 93.1% as TeX does (95.4% of the ones a converter can reproduce): [docs/conformance.md](https://github.com/matheusmota-dev-br/tikra/blob/main/docs/conformance.md).

<table>
<tr><td align="center"><a href="https://github.com/matheusmota-dev-br/tikra/blob/main/corpus/plots-3d/lorenz-attractor.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/examples/lorenz-attractor.svg" alt="The Lorenz attractor, colored by height" width="380"></a><br><sub>The Lorenz attractor, colored by height</sub></td><td align="center"><a href="https://github.com/matheusmota-dev-br/tikra/blob/main/corpus/charts/nightingale-rose.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/examples/nightingale-rose.svg" alt="Nightingale's rose of the causes of mortality" width="380"></a><br><sub>Nightingale's rose of the causes of mortality</sub></td></tr>
<tr><td align="center"><a href="https://github.com/matheusmota-dev-br/tikra/blob/main/corpus/plots-2d/normal-distribution.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/examples/normal-distribution.svg" alt="The bell curve and the 68–95–99.7 rule" width="380"></a><br><sub>The bell curve and the 68–95–99.7 rule</sub></td><td align="center"><a href="https://github.com/matheusmota-dev-br/tikra/blob/main/corpus/diagrams/petersen-graph.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/examples/petersen-graph.svg" alt="The Petersen graph" width="380"></a><br><sub>The Petersen graph</sub></td></tr>
<tr><td align="center"><a href="https://github.com/matheusmota-dev-br/tikra/blob/main/corpus/science/feynman-diagram.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/examples/feynman-diagram.svg" alt="A Feynman diagram, with decorations" width="380"></a><br><sub>A Feynman diagram, with decorations</sub></td><td align="center"><a href="https://github.com/matheusmota-dev-br/tikra/blob/main/corpus/geometry/golden-spiral.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/examples/golden-spiral.svg" alt="The golden spiral" width="380"></a><br><sub>The golden spiral</sub></td></tr>
<tr><td align="center"><a href="https://github.com/matheusmota-dev-br/tikra/blob/main/corpus/nodes-and-text/periodic-table-corner.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/examples/periodic-table-corner.svg" alt="The periodic table, by family" width="380"></a><br><sub>The periodic table, by family</sub></td><td align="center"><a href="https://github.com/matheusmota-dev-br/tikra/blob/main/corpus/plots-3d/surface-3d.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/examples/surface-3d.svg" alt="A pgfplots surface and its colorbar" width="380"></a><br><sub>A pgfplots surface and its colorbar</sub></td></tr>
</table>

Paths, nodes and every shape library, matrices, trees, graphs, mind maps, automata, circuits, arrow tips, shadings, fadings, patterns, decorations, pgfplots in 2D and 3D (polar, ternary, Smith charts, fill between, colormaps) and data visualization. What tikra does not understand becomes a warning, never a failure: the full subset is in [spec/subset.md](https://github.com/matheusmota-dev-br/tikra/blob/main/spec/subset.md).

## Also for JavaScript, Rust and the terminal

The same core is an npm package (`@mathlabs/tikra`), a Rust crate and a command-line tool: [github.com/matheusmota-dev-br/tikra](https://github.com/matheusmota-dev-br/tikra#install-and-use).

## License

The code is MIT or Apache-2.0; the fonts embedded through mathra are Latin Modern Math, under the GUST Font License, and STIX Two Math, under the SIL Open Font License 1.1; tikra's own TeX Gyre Termes (labels in Times) is under the GUST Font License.
