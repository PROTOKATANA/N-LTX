<p align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/voltik/main/docs/banner.svg" alt="voltik: CircuiTikZ to SVG, no TeX install" width="100%"></p>

# @mathlabs/voltik

CircuiTikZ circuits to SVG and PNG, without a TeX install: no LaTeX, no fonts on the system, no external processes. voltik is a library of [tikra](https://github.com/matheusmota-dev-br/tikra), which draws the TikZ; voltik adds the CircuiTikZ components, and labels are set by [mathra](https://github.com/matheusmota-dev-br/mathra) in Latin Modern, TeX's own Computer Modern.

```sh
npm install @mathlabs/voltik
```

## What it draws

Path-style components placed with `to[…]`: `short`, `open`, `generic`, resistors (`R`, American and European), capacitors (`C`) and inductors (`L`); labels and annotations (`l`, `l_`, `a`, `R=label`), straight, rotated or smart, with the siunitx shorthand `20<\ohm>`; poles `o` and `*`; `mirror` and `invert`; `\ctikzset` and the class keys. The specification is the CircuiTikZ manual; what is drawn and what is not yet is in [spec/subset.md](https://github.com/matheusmota-dev-br/voltik/blob/main/spec/subset.md). Every image below is an SVG written by voltik.

<table>
<tr><td align="center"><a href="https://github.com/matheusmota-dev-br/voltik/blob/main/corpus/circuits/ladder-network.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/voltik/main/docs/examples/ladder-network.svg" alt="An RC ladder network" width="300"></a><br><sub>An RC ladder network</sub></td><td align="center"><a href="https://github.com/matheusmota-dev-br/voltik/blob/main/corpus/circuits/voltage-divider.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/voltik/main/docs/examples/voltage-divider.svg" alt="A voltage divider" width="300"></a><br><sub>A voltage divider</sub></td></tr>
<tr><td align="center"><a href="https://github.com/matheusmota-dev-br/voltik/blob/main/corpus/circuits/rlc-circuit.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/voltik/main/docs/examples/rlc-circuit.svg" alt="An RLC circuit" width="300"></a><br><sub>An RLC circuit</sub></td><td align="center"><a href="https://github.com/matheusmota-dev-br/voltik/blob/main/corpus/labels/labels-around.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/voltik/main/docs/examples/labels-around.svg" alt="Labels at every angle, smart alignment" width="300"></a><br><sub>Labels at every angle, smart alignment</sub></td></tr>
</table>

## Node

Works right away:

```js
import { circuitikzToSvg, render, png } from "@mathlabs/voltik";      // or: const { circuitikzToSvg } = require("@mathlabs/voltik")
import { writeFileSync } from "node:fs";

const svg = circuitikzToSvg(String.raw`\begin{circuitikz}\draw (0,0) to[R=$R_1$, o-*] (2,0);\end{circuitikz}`);
writeFileSync("r.svg", svg);
writeFileSync("r.png", png(svg, { dpi: 300 }));
const { warnings, alt } = render(String.raw`\begin{circuitikz}\draw (0,0) to[C] (2,0);\end{circuitikz}`);
```

## Browsers and bundlers (Vite, webpack, esbuild)

Await `init()` once; it fetches `voltik.wasm` next to the module:

```js
import { init, circuitikzToSvg } from "@mathlabs/voltik";

await init();
document.querySelector("#circuit").innerHTML = circuitikzToSvg(String.raw`\begin{circuitikz}\draw (0,0) to[L=$L$] (2,0);\end{circuitikz}`);
```

Without a bundler, from a CDN:

```html
<script type="module">
  import { init, circuitikzToSvg } from "https://cdn.jsdelivr.net/npm/@mathlabs/voltik/web/index.js";
  await init();
  document.body.innerHTML = circuitikzToSvg(String.raw`\begin{circuitikz}\draw (0,0) to[R] (2,0);\end{circuitikz}`);
</script>
```

## API

| | |
|---|---|
| `init(input?)` | Loads the wasm. Needed once in browsers and bundlers; does nothing in Node. `input` overrides where the wasm comes from (a URL, a `Response`, bytes or a `WebAssembly.Module`). |
| `circuitikzToSvg(code, options?)` | The SVG as a string. `options.locale` (`"en"` by default, or `"pt-BR"`, `"es"`, `"fr"`, `"de"`, `"it"`, `"ru"`, `"zh"`, `"ja"`, `"ko"`, `"hi"`, `"ar"`, `"id"`, `"tr"`, `"nl"`, `"pl"`) is the language of the figure in words; the drawing is the same in all. |
| `render(code, options?)` | `{ svg, alt, description, warnings }`: `alt` is the figure in one sentence (the SVG's `<title>`), `description` the figure in words (the SVG's `<desc>`), and `warnings` lists, as strings, what was ignored or approximated. The words are written by tikra from what it drew: review them before publishing. |
| `png(svg, options?)` | PNG bytes as a `Uint8Array`. Options: `dpi` (300 by default) and `background` (`true`, white, by default; `false` for transparent). |
| `version()` | The version of voltik's core inside the package (the npm package has its own version). |

`code` is a `circuitikz` environment, a `tikzpicture` that uses CircuiTikZ components, a full `standalone` document or loose statements. Errors (no figure in the input, an invalid dpi, a PNG past 100 megapixels) are thrown as `Error`.

## Warnings, not errors

What voltik does not draw yet becomes a warning, and the rest of the circuit is drawn: a component it does not know (a source, a diode), an option it does not read.

## Also for Python, Rust and the terminal

The same core is a Python module, a Rust crate and a command-line tool, and every one gives the same SVG, byte for byte. How to install each: [github.com/matheusmota-dev-br/voltik](https://github.com/matheusmota-dev-br/voltik#install-and-use).

## License

The code is MIT or Apache-2.0; the fonts embedded through mathra are Latin Modern Math, under the GUST Font License, and STIX Two Math, under the SIL Open Font License 1.1.
