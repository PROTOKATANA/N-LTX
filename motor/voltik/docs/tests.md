# Tests

Every circuit in the tests and in `corpus/` was written for this repository to exercise voltik.

## How to run

```console
$ cargo test                                    # core, CLI and wasm (natively), 43 tests, under 3 s
$ cargo llvm-cov --workspace --exclude voltik-python --summary-only   # coverage (cargo install cargo-llvm-cov)
$ cargo clippy --all-targets -- -D warnings
$ VOLTIK_UPDATE_BANNER=1 cargo test -p voltik --test banner   # redraws docs/banner.svg and docs/banner.png
$ cargo test -p voltik-wasm                                   # the wasm exports, natively
$ CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
    cargo test -p voltik-wasm --target wasm32-unknown-unknown  # the same exports inside wasm, on Node
$ npm/build.sh && npm/test/run.sh                             # the npm package, packed and installed
$ (cd crates/python && maturin develop && python -m pytest tests)   # the Python module, in a virtualenv
$ scripts/check.sh                                            # all of the above that CI runs
```

voltik depends on the published tikra crate (or a local checkout, see "Development" in the README), and tikra on mathra. A change in either can change a test that checks a path or a label.

## What each file covers

Integration, in `crates/core/tests/`:

| File | Tests | What it exercises |
|---|---:|---|
| `bipoles.rs` | 14 | Every path-style component draws without warnings, in every style and filled; a component is its outline and two leads; `short` and `open` have no outline; `american`/`european` choose the resistor, the variable and the sensing components; `fill` fills the fillable components only; named components have anchors (`wiper`, `tip`, `label`, `midtap`…); poles by shorthand (`d-d`) and `bipole nodes`; class settings (`zigs`, `coils`, `width`) change the shape, the same in `\ctikzset`, in a `to[…]` and in a style defined with `\ctikzset`; labels and the siunitx shorthand; poles drawn over the wire; `mirror` and `invert` keep the leads outside; components inside a plain `tikzpicture`. |
| `vif.rs` | 11 | Every voltage, current and flow variant draws in every voltage style and both directions, on a component and an open circuit; a current is an arrow head on the lead after the component, or before it with `>_`; the European arrow points by `voltage dir`, and `\usepackage[RPvoltages]{circuitikz}` sets it; American voltages are signs; the anchors (`R-Vfrom`, `R-Ipos`…), the labels' nodes and `\ctikzgetanchor`; a valueless `v` and `no v symbols`; `voltage shift`, `bump b`, `distance from node`, `current/distance` move them; two-line labels and `\SI`, `\qty`; `\ctikzgetanchor` and `\ctikzgetdirection` answer what TeX does for labels by the line's way and `label/align`; a named component is a node with its name prefix; `vif queue add` and `\ctikzactivatevoltagedirections`; the style files. |
| `nodes.rs` | 6 | Every node component draws, filled, with its compass anchors; placed by its own anchors (a BNC by `zero`), a ground hangs from its point; terminals full, white or filled, sized by `nodes width`; the classes scale their components, and a class's scale is not the picture's; the supplies' text counts in the picture's size, the connectors' does not; buses, crossings and the IEC connector on paths, with short anchors. |
| `sources.rs` | 4 | Every component key draws with a voltage and a current in both conventions; a source's value is its voltage or its current (`V=`, `I<=`), a resistor's its label; a voltage source's voltage is a straight arrow to its end, a curved one with `bipole/is voltage=false`; an instrument's `t=` text, the `dc symbol` and `ac symbol` nodes. |
| `diodes.rs` | 4 | Every diode key draws in the three diode styles, with a voltage and a label; `D` is `Do`, or `D*`, `D-` by the style and by `\usepackage[fulldiode]`; `thyristor style=compact`, `led arrows from cathode`, `pd arrows to cathode` and `diode straight whiskers` change the shapes (in a `to` too), a laser's arrows stay; a thyristor's gate and anode anchors; a voltage above an LED goes over its arrows. |
| `switches.rs` | 6 | Every switch, button and jumper key draws, named with `n=`, with its anchors (`out 1`, `tap down`…); a cute switch's poles are white unless filled; a three-pins jumper takes its `tjumper connections`; the node-style `spdt`, `cute spdt …` and `proximeter` have their anchors.; rotary switches with their channels, wiper, arrows and anchors, `rotary switch` keeping the arrow in force; the poles of cute switches and named jumpers as nodes (`S-in.s`, `J-tap`), and `bipoles/jumper/shape`, `cuteswitch/height` and `thickness`. |
| `transistors.rs` | 8 | Every transistor shape draws with its terminals and text anchor; `bodydiode`, `tr circle`, `bulk`, `doublegate`, `emitters=2` choose their variants, an unmeasured combination is drawn plain with a warning; the text does not count in the picture's size; a depletion channel is dark unless filled; transistors on a path, named, with their gate, and turned round by `invert`.; the tubes draw with their anchors (`control`, `screen`, `cathode1`…) and options (`filament`, `fullcathode`, `nixieanode`…).; a transistor's text up, down or as it used to be, the text helpers (`\ctikzflipx`, `\killdepth`…), `ferroel gate` (nothing on a shape without it), `arrow pos=end`, `jfet gate height`, `scale circle radius`, `mos style/arrows`; a tube's sizes, wider than tall, its class's thickness, its filament, ten anodes, the dynode's arc. |
| `amplifiers.rs` | 4 | Every amplifier draws with its anchors (`+`, `-`, `out`, `out +`, `ra+`, `bin`…); its signs are drawn, and `noinv input up` swaps them and their anchors; a transformer's coils follow `inductor` and `quadpoles style=inline`, the double bipoles have `A1`…`B2`; an `elmech` is white unless filled, and `Telmech` is one on a path. |
| `customizations.rs` | 7 | An amplifier's and a transformer's sizes change their shape, and an amplifier with no ports has no leads; a class's `fill` fills nodes and paths; `component text=left` and `en amp text` move and replace an amplifier's text; siunitx and `\scalebox` in any text, in math or not; `transform shape` scales components on paths; bipoles as nodes (`resistorshape`, `oosourceshape` turned and scaled) and in a double bipole; a transformer's coils as nodes `T-L1.a`, and drawn by `transformer L1`. |
| `logic.rs` | 4 | Every logic port draws in its three styles with 2 to 8 inputs and without input leads, with its anchors (`in 8`, `out`); the schmitt ports, transmission gates and `notcirc`; `and port` follows `logic ports`; the gates on a path draw with their anchors and follow `logic ports`; the European negation symbols and xnor style, `european` choosing the ports, the measured sizes and `inner inputs` (another size warns), `tgate` and `blank port`, `logic ports origin=center` and `component text=left`. |
| `chips.rs` | 3 | Every DIP and QFP chip draws with 4 to 44 pins and each option, with its anchors (`pin 44`, `bpin 1`, `dot`); a pin is at its lead's end, on the border without a lead or when `draw only pins` leaves it out, and stays put with `no input leads`; more pins or a wider chip change the shape; the numbers are texts, gone with `hide numbers`; a chip is filled and scaled by `chips/scale`. |
| `muxdemux.rs` | 3 | Every multiplexer draws with every anchor, by its def, the package's shapes, without leads or with some pins only; the inversion marks; the pins are where the def puts them (the first left one on top, an odd one at the inset's tip), stick out by a lead or to one line with `square pins`, and stay put without leads; the labels of every kind are texts, a local `muxdemux label` replaces a key of the style, `0` takes a symbol away, a negation circle is a node (`M-nL2`); the border labels along the sides and `bgpicture` warn. |
| `analog.rs` | 5 | The transformer sources' three-phase windings of every kind, turned and scaled, their extra winding and anchors, `ootrans size=large`; a waveform source's symbol turned and thicker; round circles of a double-circle source of its own size; a noise source's fill or hatching; every oscilloscope waveform, an unknown one warns, its grid half transparent, its own size as a node; instruments upright unless `rotated instruments`; the dc symbol's dashes and the symbols' text not counted in the picture's size; a wiggly fuse without poles or with others; a microphone's bar, a barrier's cross, a spark gap's gap and tips; a current tap's dot; a transistor circle's color and dashes; `\ctikztunablearrow`; a subcircuit placed by an anchor. |
| `flipflops.rs` | 3 | Every flip-flop of the package and by `flipflop def` draws with its anchors, turned or not, and the `clockwedge`; the pins are a spacing apart and stick out by a lead, only the ones with a text, clock or negation drawn; a negation circle is a node (`F-N5`), a later def wins, the clock wedge and the width follow their settings; seven segment displays light the segments of their value or bits, in a loop too, the dot and box widen them. |
| `blocks.rs` | 3 | Every block draws on a path, filled and named, and every block and RF node, filled, with its anchors; `boxed`, `box only`, `ideal filter` and a bare transmission line change them; the lateral anchors follow `block left anchors pos`, a round block's `geocenter` and border, a Wilkinson's and a coupler's ports; the split two-port's texts in its halves or by its corners, a block's own width, a Wilkinson's size, the inner color. |
| `corpus.rs` | 1 | Every circuit in `corpus/<topic>/` converts without warnings to SVG and PNG. |
| `banner.rs` | 1 | `docs/banner.svg` and `docs/banner.png` are what the code draws today from `docs/banner.tex`. |

The CLI, in `crates/cli/tests/cli.rs` (7 tests): `--help` and `--version`, stdin to stdout, a file to SVG and to PNG (`--dpi`, `--transparent`), warnings on stderr or listed with `--warnings`, `--describe` and `--locale`, input errors (a missing file, input with no figure) and output errors. The test runs the real binary (`CARGO_BIN_EXE_voltik`), and `cargo llvm-cov` counts that child process's coverage.

Packages:

| File | Tests | What it exercises |
|---|---:|---|
| `crates/wasm/tests/native.rs` | 4 | The functions behind the JavaScript API, natively: SVG, the JSON of `render` with its warnings, PNG defaults, errors as text. |
| `crates/wasm/tests/web.rs` | 2 | The same exports compiled to wasm and run on Node; errors become a `JsError`. |
| `npm/test/*.test.*` | 7 | The packed npm package, installed in a scratch project: ESM and `require`, every input form, `render` and its figure in words, `png`, errors as `Error`, `version()`. |
| `crates/python/tests/test_voltik.py` | 8 | The Python module as installed: every input form, `render` and its figure in words, `png` and its errors, `ValueError`, versions. `pypi.yml` runs it against each built wheel and the sdist. |

Unit tests, inside the modules: `label.rs` (the siunitx shorthand and `\SI`, `\qty`, `\si`, `\num`), `vif.rs` (reading a variant's side, direction and end). Plus the documentation example in `lib.rs`.

## Coverage

`cargo llvm-cov --workspace --exclude voltik-python --summary-only`:

| File | Regions | Functions | Lines |
|---|---:|---:|---:|
| `cli/src/main.rs` | 98.39% | 100.00% | 100.00% |
| `core/src/bipole.rs` | 95.08% | 97.30% | 95.65% |
| `core/src/components.rs` | 70.83% | 59.09% | 69.66% |
| `core/src/label.rs` | 82.27% | 100.00% | 88.16% |
| `core/src/lib.rs` | 89.68% | 100.00% | 91.57% |
| `core/src/vif.rs` | 97.59% | 100.00% | 98.25% |
| `wasm/src/lib.rs` | 71.88% | 57.14% | 71.43% |
| **Total** | **90.71%** | **86.78%** | **91.65%** |

The gaps of `components.rs` are the builders of its table of components (`bipole`, `band`, `anchors`…), `const fn`s that run when the crate compiles, where coverage does not see them. The gaps of `label.rs` and `lib.rs` are mostly the long tables of siunitx units and of package options, one line each. The wasm crate's gaps are the `#[wasm_bindgen]` wrappers, which only `web.rs` calls, and it runs inside wasm, where `cargo llvm-cov` does not reach.

## Comparing with TeX

The tests check that a circuit draws and how it is built; whether it looks like CircuiTikZ is checked against TeX with the tools in `tools/oracle/` (see "How close to CircuiTikZ" in the README). They need Docker and are not part of `cargo test`; neither is the conformance run over the whole manual, `tools/conformance/run.sh` ([docs/conformance.md](conformance.md)).
