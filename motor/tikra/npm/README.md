<p align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/banner.svg" alt="tikra: TikZ to SVG and PNG, no TeX install" width="100%"></p>

# @mathlabs/tikra

TikZ and pgfplots figures to SVG and PNG, without a TeX install: no LaTeX, no fonts on the system, no external processes. It reimplements TikZ, pgf and pgfplots, and labels are set by [mathra](https://github.com/matheusmota-dev-br/mathra) in Latin Modern, TeX's own Computer Modern. **[Try it in the browser](https://matheusmota-dev-br.github.io/tikra/)** · **[See the gallery](https://matheusmota-dev-br.github.io/tikra/gallery.html)**.

```sh
npm install @mathlabs/tikra
```

## What it draws

Every image below is an SVG written by tikra from a figure in the repository's corpus, on a white card; the [gallery](https://matheusmota-dev-br.github.io/tikra/gallery.html) has all 227. Of the 2502 figures of the TikZ and pgfplots manuals, tikra draws 93.1% as TeX does (95.4% of the ones a converter can reproduce): [docs/conformance.md](https://github.com/matheusmota-dev-br/tikra/blob/main/docs/conformance.md).

<table>
<tr><td align="center"><a href="https://github.com/matheusmota-dev-br/tikra/blob/main/corpus/plots-3d/lorenz-attractor.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/examples/lorenz-attractor.svg" alt="The Lorenz attractor, colored by height" width="380"></a><br><sub>The Lorenz attractor, colored by height</sub></td><td align="center"><a href="https://github.com/matheusmota-dev-br/tikra/blob/main/corpus/charts/nightingale-rose.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/examples/nightingale-rose.svg" alt="Nightingale's rose of the causes of mortality" width="380"></a><br><sub>Nightingale's rose of the causes of mortality</sub></td></tr>
<tr><td align="center"><a href="https://github.com/matheusmota-dev-br/tikra/blob/main/corpus/plots-2d/normal-distribution.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/examples/normal-distribution.svg" alt="The bell curve and the 68–95–99.7 rule" width="380"></a><br><sub>The bell curve and the 68–95–99.7 rule</sub></td><td align="center"><a href="https://github.com/matheusmota-dev-br/tikra/blob/main/corpus/diagrams/petersen-graph.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/examples/petersen-graph.svg" alt="The Petersen graph" width="380"></a><br><sub>The Petersen graph</sub></td></tr>
<tr><td align="center"><a href="https://github.com/matheusmota-dev-br/tikra/blob/main/corpus/science/feynman-diagram.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/examples/feynman-diagram.svg" alt="A Feynman diagram, with decorations" width="380"></a><br><sub>A Feynman diagram, with decorations</sub></td><td align="center"><a href="https://github.com/matheusmota-dev-br/tikra/blob/main/corpus/geometry/golden-spiral.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/examples/golden-spiral.svg" alt="The golden spiral" width="380"></a><br><sub>The golden spiral</sub></td></tr>
<tr><td align="center"><a href="https://github.com/matheusmota-dev-br/tikra/blob/main/corpus/nodes-and-text/periodic-table-corner.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/examples/periodic-table-corner.svg" alt="The periodic table, by family" width="380"></a><br><sub>The periodic table, by family</sub></td><td align="center"><a href="https://github.com/matheusmota-dev-br/tikra/blob/main/corpus/plots-3d/surface-3d.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/tikra/main/docs/examples/surface-3d.svg" alt="A pgfplots surface and its colorbar" width="380"></a><br><sub>A pgfplots surface and its colorbar</sub></td></tr>
</table>

## Node

Works right away:

```js
import { tikzToSvg, render, png } from "@mathlabs/tikra";      // or: const { tikzToSvg } = require("@mathlabs/tikra")
import { writeFileSync } from "node:fs";

const svg = tikzToSvg(String.raw`\begin{tikzpicture}\draw[->] (0,0) -- (2,1) node[right] {$\vec v$};\end{tikzpicture}`);
writeFileSync("v.svg", svg);
writeFileSync("v.png", png(svg, { dpi: 300 }));
const { warnings, alt } = render(String.raw`\tikz \draw (0,0) circle (1);`);   // [], "Drawing of one shape, without text"
```

## Browsers and bundlers (Vite, webpack, esbuild)

Await `init()` once; it fetches `tikra.wasm` next to the module:

```js
import { init, tikzToSvg } from "@mathlabs/tikra";

await init();
document.querySelector("#figure").innerHTML = tikzToSvg(String.raw`\tikz \draw (0,0) circle (1);`);
```

Without a bundler, from a CDN:

```html
<script type="module">
  import { init, tikzToSvg } from "https://cdn.jsdelivr.net/npm/@mathlabs/tikra/web/index.js";
  await init();
  document.body.innerHTML = tikzToSvg(String.raw`\tikz \draw[->] (0,0) -- (1,1);`);
</script>
```

## API

| | |
|---|---|
| `init(input?)` | Loads the wasm. Needed once in browsers and bundlers; does nothing in Node. `input` overrides where the wasm comes from (a URL, a `Response`, bytes or a `WebAssembly.Module`). |
| `tikzToSvg(code, options?)` | The SVG as a string. `options.locale` (`"en"` by default, or `"pt-BR"`, `"es"`, `"fr"`, `"de"`, `"it"`, `"ru"`, `"zh"`, `"ja"`, `"ko"`, `"hi"`, `"ar"`, `"id"`, `"tr"`, `"nl"`, `"pl"`) is the language of the figure in words; the drawing is the same in all. |
| `render(code, options?)` | `{ svg, alt, description, warnings }`, the words in `options.locale`: `alt` is the figure in one sentence (the SVG's `<title>`, for an image's `alt`), `description` the figure in words (the SVG's `<desc>`: a chart's axes and data, a diagram's nodes and connections), and `warnings` lists, as strings, what was ignored or approximated. When the picture has its author's text (`\begin{tikzpicture}[alt={…}]`), `alt` and `description` are that text. The words tikra writes are not perfect (they say what is in the drawing, not what it means, and little about free drawings): review them before publishing. |
| `png(svg, options?)` | PNG bytes as a `Uint8Array`. Options: `dpi` (300 by default) and `background` (`true`, white, by default; `false` for transparent). |
| `version()` | The version of tikra's core inside the package (the npm package has its own version). |

`code` is a full `standalone` document, a bare `tikzpicture`, an inline `\tikz`, an `axis` environment or loose TikZ statements. Errors (no figure in the input, an invalid dpi, a PNG past 100 megapixels) are thrown as `Error`.

## Warnings, not errors

What tikra does not understand becomes a warning, and the figure is drawn with the rest: unknown commands, environments and options, TikZ escaped twice or for Markdown (undone), labels with broken LaTeX. The full subset is in [spec/subset.md](https://github.com/matheusmota-dev-br/tikra/blob/main/spec/subset.md).

## Also for Python, Rust and the terminal

The same core is a Python module, a Rust crate and a command-line tool, and every one gives the same SVG, byte for byte. How to install each: [github.com/matheusmota-dev-br/tikra](https://github.com/matheusmota-dev-br/tikra#install-and-use).

## License

The code is MIT or Apache-2.0; the fonts embedded through mathra are Latin Modern Math, under the GUST Font License, and STIX Two Math, under the SIL Open Font License 1.1; tikra's own TeX Gyre Termes (labels in Times) is under the GUST Font License.
