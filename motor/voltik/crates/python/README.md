<p align="center"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/voltik/main/docs/banner.svg" alt="voltik: CircuiTikZ to SVG, no TeX install" width="100%"></p>

# voltik

CircuiTikZ circuits to SVG and PNG, without a TeX install: no LaTeX, no fonts on the system, no external processes. voltik is a library of [tikra](https://github.com/matheusmota-dev-br/tikra), which draws the TikZ; voltik adds the CircuiTikZ components, and labels are set by [mathra](https://github.com/matheusmota-dev-br/mathra) in Latin Modern, TeX's own Computer Modern.

```sh
pip install voltik
```

Python 3.9 and later; wheels for Linux, macOS and Windows; types included.

## What it draws

Path-style components placed with `to[…]`: `short`, `open`, `generic`, resistors (`R`, American and European), capacitors (`C`) and inductors (`L`); labels and annotations (`l`, `l_`, `a`, `R=label`), straight, rotated or smart, with the siunitx shorthand `20<\ohm>`; poles `o` and `*`; `mirror` and `invert`; `\ctikzset` and the class keys. The specification is the CircuiTikZ manual; what is drawn and what is not yet is in [spec/subset.md](https://github.com/matheusmota-dev-br/voltik/blob/main/spec/subset.md). Every image below is an SVG written by voltik.

<table>
<tr><td align="center"><a href="https://github.com/matheusmota-dev-br/voltik/blob/main/corpus/circuits/ladder-network.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/voltik/main/docs/examples/ladder-network.svg" alt="An RC ladder network" width="300"></a><br><sub>An RC ladder network</sub></td><td align="center"><a href="https://github.com/matheusmota-dev-br/voltik/blob/main/corpus/circuits/voltage-divider.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/voltik/main/docs/examples/voltage-divider.svg" alt="A voltage divider" width="300"></a><br><sub>A voltage divider</sub></td></tr>
<tr><td align="center"><a href="https://github.com/matheusmota-dev-br/voltik/blob/main/corpus/circuits/rlc-circuit.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/voltik/main/docs/examples/rlc-circuit.svg" alt="An RLC circuit" width="300"></a><br><sub>An RLC circuit</sub></td><td align="center"><a href="https://github.com/matheusmota-dev-br/voltik/blob/main/corpus/labels/labels-around.tex"><img src="https://raw.githubusercontent.com/matheusmota-dev-br/voltik/main/docs/examples/labels-around.svg" alt="Labels at every angle, smart alignment" width="300"></a><br><sub>Labels at every angle, smart alignment</sub></td></tr>
</table>

## Use

```python
import voltik

svg = voltik.circuitikz_to_svg(r"\begin{circuitikz}\draw (0,0) to[R=$R_1$, o-*] (2,0);\end{circuitikz}")
open("r.svg", "w").write(svg)
open("r.png", "wb").write(voltik.png(svg, dpi=300))      # background=False for transparent

r = voltik.render(code)              # dict: svg, alt, description, warnings
r["warnings"]                        # what was ignored or approximated
voltik.render(code, locale="pt-BR")  # the figure in words in another language; the drawing is the same
```

| | |
|---|---|
| `circuitikz_to_svg(code, *, locale="en", animate=False, steps=False)` | The SVG as a string. `locale` is the language of the figure in words (`en`, `pt-BR`, `es`, `fr`, `de`, `it`, `ru`, `zh`, `ja`, `ko`, `hi`, `ar`, `id`, `tr`, `nl`, `pl`). |
| `render(code, *, locale="en", animate=False, steps=False)` | A dict: `svg`, `alt` (the figure in one sentence), `description` (the figure in words) and `warnings`. |
| `png(svg, *, dpi=300, background=True)` | PNG bytes. |
| `__version__`, `core_version` | The package's version and the version of voltik's core inside it. |

`code` is a `circuitikz` environment, a `tikzpicture` that uses CircuiTikZ components, a full `standalone` document or loose statements. No figure in the input, an unknown locale, an invalid dpi or a PNG past 100 megapixels raise `ValueError`.

## Warnings, not errors

What voltik does not draw yet becomes a warning, and the rest of the circuit is drawn: a component it does not know (a source, a diode), an option it does not read.

## Also for JavaScript, Rust and the terminal

The same core is an npm package, a Rust crate and a command-line tool, and every one gives the same SVG, byte for byte: [github.com/matheusmota-dev-br/voltik](https://github.com/matheusmota-dev-br/voltik#install-and-use).

## License

The code is MIT or Apache-2.0; the fonts embedded through mathra are Latin Modern Math, under the GUST Font License, and STIX Two Math, under the SIL Open Font License 1.1.
