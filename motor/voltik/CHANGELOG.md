# Changelog

What changed in each release of voltik: the crates `voltik` and `voltik-cli`, `@mathlabs/voltik` on npm and `voltik` on PyPI, which are released together. The local site (`site/build.sh`, then open `site/changelog.html`) shows the same, with the circuits drawn live.

Each release has a headline (the line under its title), what changed, and the figures of [`corpus/`](corpus) that show it (`Figures:`), drawn by the site with today's voltik. `scripts/release.sh` will not release a version without its section here, and makes the GitHub release from it.

## 0.16.0 - 2026-10-30

The rest of the CircuiTikZ manual: 1053 of 1055 figures match TeX, no option of the manual is ignored any more, and the conformance run now counts the 9 it could not compile.

- Every look of the arrows: `opto`, `switch`, `tunable`, `wiper` and `gto gate` `start arrow` and `end arrow`, and `opto arrows/…` and `switch arrows/…` (`color`, `dash`, `relative thickness`), also on phototransistors, cute switches and rotary switches.
- Transistors: `tripoles/mos style=arrows`, `tripoles/pmos style/emptycircle` and `nocircle`, `arrow pos=end` on a MOSFET, `transistors/gate thickness`, `rounded caps`, `tr gap fill`, `depletion color`, `transistor solderdot scale`, and the HEMT's `split gate`.
- Sources and meters: `bipoles/dcisource/angle`, `bipoles/isourcesin/angle`, an American source's `inner plus`, `inner minus` and `margin`, `sources/symbol/sign rotation`, and `index position`.
- Blocks: `to[…, dashed]` dashes their box, with `inner blocks dashed` and `dashed blocks pattern`; `blocks dc segments` (and `in`, `out`); `blocks/filter grid/…`, `blocks/filter plot/…`, and `ideal filter numbers`.
- Logic ports: `american or shape=roundy`, and `ieeestd ports/` `not radius`, `xor bar distance` and `xor leads in`.
- `crossing vertical/…`, `transformer core/…`, `en amp text A`, `nodes width` on a `to`, `bipole/nodes/left` and `right`, poles given by a macro or a style, and an oscilloscope's own waveform (`bipoles/oscope/waveform/name/.code`).
- `to[amp, >]`: the arrow into a block.
- A transformer's coils have their `midtap` (`T-L2.midtap`).
- A cute choke's `bipoles/cutechoke/cthick`, `twolineschoke` and `onelinechoke`.
- The poles of a `to` under `transform shape` are as big as the `ocirc` nodes (they were scaled twice).
- Partial borders of transistors' circles and tubes (`partial borders`, `partial border dash`), and the anchors `circle top`, `bottom`, `left` and `right`.
- The body diode's `color`, `dash`, `relative thickness`, `scale` and `dot scale`.
- `open poles fill` and `open poles opacity`.
- `amplifiers/plus`, `amplifiers/minus`, `amp plus`, `amp minus` and `amp symbol font`.
- `monopoles/vcc/arrow` and `monopoles/vee/arrow`, with `\\scaledwidth`.
- `gridlines`, `tripoles/schottky base size`, `no schottky base`, `tripoles/igbt/outer base height` and `thickness`, `monopoles/msrstub/radius`, `proximeter/hlines position` and `thickness`, `current arrow scale` and `logic ports draw input leads`.
- `rmeterwa`'s arrow leaves room for its text; a label on a line turned by `transform shape` keeps its side; `genericsplitter`'s and `mslstub`'s texts sit where TeX puts them; `\\SI{+10}{V}` leaves out the plus.
- `\ctikzflipx`, `\ctikzflipy` and `\ctikzflipxy` flip their text back; a European port's symbol stays readable in a mirrored node.
- A split two-port is drawn upright whichever way its path goes, with `t1` and `t2` where TeX puts them.
- A double bipole's source follows `voltage dir`; the texts of `smeter` on vertical lines and of `qgenerator`; `\SI{.25}{…}` gets its zero.
- `vif queue add` runs its macros at the end of the picture, or at `\ctikzprocessvif`, as TeX does, so they can use the component's nodes (`(R1voltage.center)`).
- `\ctikzset{my v/.default=blue}` reaches the styles a `to` uses.
- Inductive sensors (`sL`) keep their labels clear of their arrow.
- An American voltage source under `bipole/override source vif` keeps a source's label.
- `voltage=raised` keeps its height under a class's scale (`diodes/scale`).
- `pin distance` and `label distance` among a pin's or a label's own options (tikra), and `\boldmath` math in bold italic, into `\rotatebox` too (mathra).

## 0.15.0 - 2026-10-29

The CircuiTikZ manual's figures drawn as TeX draws them: 1041 of 1046 match, from 1010.

- Labels and voltages placed as TeX places them:
  - labels on slanted lines;
  - two-line labels (`l2`, `a2`), set as the manual's `tabular`;
  - raised and shifted voltages and sources' voltage arrows;
  - a voltage above a switch or on a mirrored component (`v^`, `mirror`);
  - `bipoles/<shape>/voltage/additional shift`;
  - a `short`'s current and flow at its middle;
  - math labels as wide as TeX's (mathra#89).
- Logic ports:
  - `tripoles/american … port/port width` and `tripoles/thickness`;
  - `european ports font`;
  - `inline not=I1`;
  - `logic ports/fill=none` in a node's own options.
- Multiplexer labels: `TL1`/`TR1`/`BL1`/`BR1` are drawn, and the labels of a turned chip sit where TeX puts them.
- Transistor texts placed by their circle (`tr circle`, `scale circle radius`).
- Dynode arcs at any `arc angle`.
- `en amp text`, mirrored and turned with its node.
- The plain sparkgap's labels.
- `oosourcetrans` at `ootrans size=large`.
- Three-phase symbols upright whichever way their source points.
- A bipole drawn as a node continues its path from the node's own point.
- The jack taps' arrow shorthands `^`, `^f`, `v` and `vf`, and `\pgfcircversion`.
- From tikra#112:
  - `pin=\a:\a` in a `\foreach`;
  - `+` points after a `to[…]`;
  - colors and options inside a path;
  - `current point is local`;
  - `varwidth`;
  - `example-image-a`;
  - multi-line nodes on TeX's baseline.

Figures: `voltages-and-currents`, `stacked-and-units`, `mirror-invert`, `voltage-styles`

## 0.14.0 - 2026-10-29

Chips, multiplexers, flip-flops, displays, block diagrams and RF components; the manual's own voltages and style files.

- The DIP and QFP chips (`dipchip`, `qfpchip`) with any number of pins (`num pins`), their leads or pads (`external pins width`, `external pad fraction`, `no input leads`, `draw only pins`), the orientation mark (`no topmark`) and the pins' numbers (`hide numbers`, `multipoles/font`), sized by `multipoles/dipchip/width`, `pin spacing` and `chips/scale`, with their anchors (`pin 1`, `bpin 1`, `dot`).
- The multiplexers (`muxdemux`) shaped by `muxdemux def`, with or without an inset, their pins on four sides (`square pins`, `draw only left pins`…) and all their anchors (`lpin 1`, `bbpin 2`, `inset top`, `narrow center`…); the package's `demux`, `one bit adder` and `ALU`; the class `muxdemuxes`.
- `muxdemux label`: texts by the pins, inside and outside (`L1`, `LU1`, `BR2`…), by the borders (`N`, `Si`), clock wedges (`cL2`), negation circles as nodes (`M-nL1`) and wedges (`wiB1`, `woR1`); the inversion marks `wedgeinv` and `circleinv`.
- The flip-flops (`flipflop`, `flipflop def` with texts, clock wedges and negation circles as nodes `FF-N4`), the package's `latch`, `flipflop SR`, `D`, `T` and `JK`, `add async SR`, `dot on notQ`, `\ctikztextnot` and `clockwedge`; their texts stay upright when they turn.
- The seven segment displays: `bare7seg`, `seven segment val=A dot off box on`, `seven segment bits=…`, and their sizes and colors.
- Logic ports: the European negation symbols (`tripoles/european not symbol`) and `european xnor style=direct`, `logic ports origin=center`, `inner inputs`, `component text=left`, `tgate`, `double tgate`, `blank port`, `blank not port`, and the sizes the manual shows (`tripoles/american or port/height`, `ieeestd ports/height`…); `american` and `european` set the ports too.
- The pins' numbers and labels stay upright on a turned chip or multiplexer (`straight numbers`, or turn with it by `rotated numbers`), and the multiplexers' border labels along the sides (`E`, `Wi`…) are turned a quarter (tikra#39).
- `\rotatebox`, `\raisebox` and `\scalebox` in any text turn, raise and scale it, as graphicx does.
- The transformer sources' three-phase windings (`prim=delta`, `sec=wye`, `tert=zig`, `eyw`, `xdelta`), turned and scaled (`sources/symbol/delta rot`, `prim rot`, `delta scale`), their extra winding (`oo upper winding`, `upper=…`) with its anchors, and `ootrans size=large`; `bipoles/oosource/width` and `height`; the waveform sources' `sources/symbol/rotate` and `thickness`; the noise sources' `fillcolor`, hatched with `dashed`.
- The oscilloscope's waveforms (`bipoles/oscope/waveform=sin`…), size (`bipoles/oscope/width`, `height`) and half-transparent grid, its text above it as a node; the instruments' screens kept upright (`straight instruments`, or `rotated instruments`); the current tap's `bipoles/currtap/…`.
- The `dc symbol` and `ac symbol` sized and with their text above them (`dc symbol segments`…); the wiggly fuse's poles (`bipoles/wfuse/dots`, `shape`), the microphone's `bar thickness`, the spark gap's `sparkgap end arrow` and `gap`, the barrier's cross; `transistor circle/color` and `dash`; the `latexslim` tip, `\ctikztunablearrow` and subcircuits (`\ctikzsubcircuitdef`, `\ctikzsubcircuitactivate`).
- A potentiometer drawn as a node has its `wiper`; a transformer's coils drawn as inductors (`transformer L1`, `L2`) curve toward its core; `\si{\%}` writes a percent sign.
- The block diagram components: on a path, the filters, amplifiers, generators, converters, attenuators and the radio and TV blocks (`lowpass`, `amp`, `adc`, `sacdc`, `trx`…), boxed (`amp, boxed`), the plot filters (`lowpassp`, `ideal filter`), with their texts (`t=`, `t1`, `t2`), sizes (`bipoles/amp/width`), inner color and lateral anchors (`block left anchors pos`); as nodes the mixers, adders, oscillators and circulators (`boxed`, `box only`), splitters, couplers, `mzm` and `gridnode`, and every block as a node shape (`powershape`).
- The RF components: antennas, `waves`, `harmonics`, the microstrips (`mstline`, `mslstub`, `msport`, `msrstub`), the transmission line `TL` (`bipoles/tline/bare`), `tlinestub` and `match`.
- `bgpicture` in `muxdemux def`: pgf's code drawn on the multiplexer, its x axis from -1 cm to 1 cm across it.
- Voltages and currents as CircuiTikZ places them: American signs on every voltage source but the American ones (which carry theirs), an American source's label closer to it; a voltage source's current reversed with electric-field voltages whatever its side (`i_=…`); `bipole/is voltage=false` after a source's value keeps its side and direction (the manual's mix); and the labels of voltages and currents with no outer sep, as TeX's.
- Labels kerned as TeX's T1 Latin Modern kerns them (tikra#48, mathra#81).
- `v`, `i` and `f` without a value draw nothing, as in TeX: only their anchors are there (`R1-Vfrom`…).
- A named component is a node, as in CircuiTikZ: `(R1)` is its center, and `name prefix` (and a `pic`'s name) names it. `\ctikzgetdirection` and `\ctikzgetanchor` answer what TeX does: directions in the picture's own coordinates from 0 to 360 (`333.43495`, a label's `333`), a current's arrow (`R1-Iarrow`), and the anchors of labels that keep straight (`base`, `north`, `mid west`, `-45`); a raised voltage's `Vlab` its side's anchor.
- Labels along a line turn in the picture's own coordinates (`label/align=rotate`, `smart`), so that `transform shape` turns them once; upside down only strictly between 90 and 270 degrees.
- The labels of LEDs and photodiodes clear their arrows.
- The manual's own voltages, currents and flows: `vif queue add={macro}{a}{b}{c}` runs `\macro{R1}{a}{b}{c}` once the component is drawn (naming it when it has no name), `\ctikzactivatevoltagedirections{vpms}` (and `current`, `flow`) gives a style its variants (`vpms^>`), and `use auto vif`; styles of two or more arguments in `\ctikzset`.
- The style files: `\ctikzloadstyle{example}`, `romano` and `legacy`, and their `… circuit style`.
- `bipole label style={…}` (and `annotation`, `voltage`, `current`, `flow`) in a `to` sets the style of its labels; the style comes after voltik's own settings of the label (`outer sep=5pt`); American voltage signs keep the color of the path.
- `fullgeneric` and `tfullgeneric`; `bipoles/fix tunable direction=false` turns the variable components' arrows as before 1.8; `tunable end arrow` ends them.
- `voltage shift sources adjust`: a source's voltage shifted by that times `voltage shift`.
- Colors of xcolor's cmyk model as TeX's PDF shows them (`cyan` is `#00aeef`; tikra#53), the picture environment (`\fourdots`), `\NewDocumentCommand`, the `Jack Tap` tip and pictures in a `\scalebox` (tikra#53); subcircuits placed by measuring them, without drawing them (tikra#53).
- Drawn by tikra 1.7.0 and mathra 1.8.0 (tikra#39, tikra#48, tikra#52, tikra#53, mathra#81). 540 of the manual's 541 components and 470 of its 505 examples match TeX.

Figures: `chips`, `multiplexers`, `flip-flops`, `three-phase-and-scopes`, `block-diagrams`

## 0.13.0 - 2026-10-29

Logic gates.

- The logic ports in their three styles, American, European and IEEE (`american and port`, `european xnor port`, `ieeestd nand port`…), with 2 to 8 inputs (`number inputs`), without leads (`no input leads`, `no output leads`, `no leads`, and back with `input leads`, `all leads`), and their anchors (`in 1`, `bin 1`, `out`, `bout`); the schmitt ports, the transmission gates (`ieee tgate`, `ieee double tgate`), `notcirc`, `schmitt symbol` and the European blank ports.
- The short keys (`and port`, `not port`…) follow `logic ports=american`, `european` or `ieee` (and `american ports`, `european ports`, `ieee ports`, the package options too).
- The gates on a path (`inline not`, `inline buffer`, `inline schmitt`, `inline invschmitt`, `inline tgate`, `inline double tgate`), in the style of `logic ports`.

Figures: `logic-gates`, `half-adder`

## 0.12.0 - 2026-10-29

Rotary switches, the poles of switches as nodes, and the customizations of transistors and tubes.

- The rotary switch (`rotaryswitch`, `rotary switch=8 in 120 wiper 40`, `rotary switch <->`…), with its channels, wiper, arrows, thickness and pole shape, all its anchors (`out 3`, `aout 3`, `sqout 3`, `ext north`), and its poles as nodes (`S-out 3.n`).
- The poles of cute switches and of named jumpers as nodes (`S1-in.s`, `J2-in.-135`, `J1-tap`); `bipoles/cuteswitch/height`, `thickness` and `shape`, and `bipoles/jumper/shape`.
- Bipoles drawn as nodes turn with the node (`yscale=-1`), their ends on their line, and count only what they draw.

- Transistors' texts up, down and as they used to be (`component text`, `legacy transistors text`); `ferroel gate`, `transistors/arrow pos=end`, `jfet gate height`, `mos style/arrows`, `scale circle radius` and the circle's `relative thickness`; a class's `thickness` on nodes.
- The tubes' sizes (`tubes/width`, `height`, `cathode width`, `anode width`, `filament angle`, `filament distance`), any number of anodes (`tubes/anodes`, as the manual's VFD and nixie tubes), and the dynode's `arc pos` and `top width`.
- The manual's text helpers, `\ctikzflipx`, `\killdepth`, `\rotatebox`, `\raisebox`, write their text.

Figures: `rotary-switches`

## 0.11.0 - 2026-10-29

Transformers and motors, the sizes of components, and bipoles as nodes.

- The double bipoles as nodes: `transformer` and `transformer core`, their coils by the `inductor` setting, `gyrator` and `double bipole`, each also `quadpoles style=inline`, with their anchors (`A1`…`B2`, the dots).
- The electro-mechanical device `elmech`, and `Telmech` on a path.
- The sizes of amplifiers and double bipoles (`tripoles/op amp/width`, `port width`, `input height`…, `quadpoles/transformer/width`, `height`, `inner`), drawn in between the measured ones.
- A class's `fill` (`\ctikzset{amplifiers/fill=cyan}`), `component text=left`, `left text distance` and `en amp text`.
- Every bipole as a node by its shape name (`node[resistorshape]`, `fulllediodeshape`…), and inside a double bipole (`double bipole L`, `R`, `R invert`, `every double bipole L`).
- A transformer's coils as nodes (`T1-L1.a`), drawn by `transformer L1` and `L2`.
- siunitx in any text, not only in labels (`node {\SI{12}{\volt}}`), and `\scalebox`; `transform shape` scales components on paths too.

Figures: `transformers` `sizes-and-shapes`

## 0.10.0 - 2026-10-29

Amplifiers.

- The amplifiers of the manual as nodes: `op amp`, `fd op amp`, `en amp`, `gm amp`, the instrumentation amplifiers, the plain amplifiers and `buffer`, with their anchors and their `+` and `-`, in the `amplifiers` class.
- `noinv input up`, `noinv input down`, `noinv output up` and `noinv output down`.

Figures: `amplifiers` `non-inverting-amplifier`

## 0.9.0 - 2026-10-28

Electronic tubes.

- The tubes of the manual as nodes: `diodetube`, `triode`, `tetrode`, `pentode`, `pentode suppressor to cathode`, `matube`, `magnetron` and `dynode`, with their anchors, in the `tubes` class.
- Their options `filament`, `nocathode`, `fullcathode`, `nixieanode`, `anodedot` and `nogrid`.

Figures: `tubes`

## 0.8.0 - 2026-10-28

Transistors.

- Every transistor of the manual as a node: bipolar (`npn`, `pnp`, multi-collector `bjtnpn`…), IGBTs, MOSFETs (`nmos`, `pmos`, depletion), FETs (`nfet`, `nigfete`…), HEMTs, JFETs, UJTs, ISFET and graphene FETs, with all their anchors, in the `transistors` class.
- Their options in the combinations the manual draws: `bodydiode`, `tr circle`, `bulk`, `fdsoi`, `doublegate`, `solderdot`, `schottky base`, `photo`, `nobase`, `arrowmos`, `emptycircle`, `nocircle`, `collectors`, `emitters`.
- Transistors on a path (`to[Tnmos=$M_1$, n=M]`), turned round by `invert`.
- Depletion channels dark unless filled; the text right of the transistor, not counting in the picture's size, as in CircuiTikZ.

Figures: `transistors` `common-emitter`

## 0.7.0 - 2026-10-28

Switches, buttons and jumpers.

- Every path-style switch of the manual: traditional (`spst`, `nos`, `ncs`, `cnos`…), push buttons, the toggle switch, the reed switch, the cute switches, the inline proximeter, the jumpers (`three-pins jumper` with `tjumper connections`) and the solder jumpers, with their anchors; their poles white unless filled.
- The node-style `spdt`, `cute spdt up`, `cute spdt mid`, `cute spdt down` (and their `… arrow`) and `proximeter`, with their anchors, in the `switches` class.
- `n=…`, the short form of a component's `name`.

Figures: `switches`

## 0.6.0 - 2026-10-28

Diodes and the thyristors.

- Every diode of the manual: plain, Schottky, Zener, ZZener, tunnel, photodiode, LED, laser, varcap, TVS, Shockley and bidirectional, empty, full and stroke (`D`, `D*`, `D-`…); the short keys follow the `diode` setting, the styles `empty diodes`, `full diodes`, `stroke diodes` and the package options `emptydiode`, `fulldiode`, `strokediode`.
- The thyristor-like tripoles (`Tr`, `Ty`, `PUT`, `GTO`, `GTOb`, `aGTOb`, `IGCT`), legacy or `thyristor style=compact`, with their gate and anode and cathode anchors.
- `led arrows from cathode`, `pd arrows to cathode` and `diode straight whiskers`, and their way back.
- Their voltages as CircuiTikZ draws them, on each side: a voltage above an LED clears its arrows.

Figures: `diodes` `rectifier-and-zener`

## 0.5.0 - 2026-10-28

Sources, instruments and the other bipoles.

- Every source of the manual: independent and controlled, voltage and current, American, European and cute (`american currents`, `european currents` and their package options), sinusoidal, square, triangle, noise, DC, the `oosource`s, `nullator` and `norator`; the batteries and solar cells.
- Their voltages and currents as CircuiTikZ draws a source's: `V=…` and `I<=…` are the voltage and the current, a voltage source's voltage is a straight arrow across it, a current source's turns the other way; `bipole/is voltage`, `bipole/is current` and `bipole/override source vif`.
- The instruments (`rmeter`, `smeter`, `ammeter`, `voltmeter`, `oscope`, the probes, `iloop`…) with `t=…`, the mechanical analogies (`spring`, `damper`, `mass`, `inerter`, `viscoe`), and the other bipoles: fuses, lamps, bulbs, spark gaps, loudspeaker, microphones, buzzers, `squid`, barriers, `relais`, `thermocouple`, surge arresters; the `dc symbol` and `ac symbol` nodes.
- A component's built-in text stays upright and in its middle when the component turns.

Figures: `sources` `instruments-and-more` `source-and-load`

## 0.4.0 - 2026-10-28

Components as nodes, with anchors of their own.

- Node-style components, each a node shape of its own (tikra's new library shapes): the ten grounds, `vcc` and `vee`, the terminals (`circ`, `ocirc`, `diamondpole`, `squarepole`… and `rectjoinfill`), the BNC and IEC connectors, the jump and plain crossings, and the arrows (`currarrow`, `inputarrow`, `trarrow`, `flowarrow`).
- Their anchors (`G.south`, `B.zero`, `S.socket center`), placing by them (`anchor=zero`), turning with the node, `fill` where fillable, and the picture's size as CircuiTikZ counts it.
- The classes `grounds`, `power supplies` and `connectors` (`…/scale`); every class of the manual has its settings, so `\ctikzset{diodes/scale=…}` no longer scales the picture.
- Path-style buses (`multiwire`, `bmultiwire`, `tmultiwire`), `crossing` and `iec connector`; named bipoles have the short anchors too (`R1.nw`).

Figures: `grounds-and-supplies` `terminals-and-connectors` `powered-divider`

## 0.3.0 - 2026-10-28

Voltages, currents and flows, as the manual draws them for every component so far.

- `v=`, `i=`, `f=` and their variants with a side (`^`, `_`) and a direction (`<`, `>`), on the lead before or after the component.
- Voltages in the four styles: European curved arrows, straight arrows, American signs and raised American signs, with `voltage shift`, `voltage/bump b`, `voltage/distance from node` and the American keys (`american plus`, `american label distance`…); an open circuit's voltage lies on the line.
- The `voltage dir` conventions (`noold`, the default, `EF`, `RP`, `old`), and the document's own `\usepackage[RPvoltages, american]{circuitikz}`.
- The anchors of named components' voltages, currents and flows (`R1-Vfrom`, `R1-Ipos`…), their labels' nodes (`R1voltage`, `R1label`), `\ctikzgetanchor`, `\ctikzgetdirection`, `no v symbols` and the `bipole … style` keys.
- Two-line labels `l2=… and …`, and siunitx in any label: `\SI`, `\qty`, `\si`, `\unit`, `\num`.
- Labels keep TikZ's inner sep, as CircuiTikZ's do, so pictures are their size.

Figures: `voltages-and-currents` `voltage-styles` `stacked-and-units`

## 0.2.0 - 2026-10-28

Every resistive bipole, capacitor and inductor of the manual, drawn as CircuiTikZ draws them.

- The resistive bipoles: the generic ones (crossed, slashed, sine, tunable, asymmetric), memristors, variable resistors, potentiometers with `wiper pos`, resistive sensors, light-dependent resistors, varistors, photoresistors and thermistors, American and European.
- The capacitors (curved, electrolytic, variable, sensing, piezoelectric, constant phase, ferroelectric) and the inductors in the three styles, cute, American and European, variable and sensing, and the cute choke.
- `fill=` on a `to` fills the components the manual makes fillable; `name=` gives a component its anchors as coordinates (`P.wiper`, `L1.midtap`, `L1.core west`).
- Poles `d`, and `bipole nodes={…}{…}` with circles, diamonds and squares; the class keys `height` and `modifier thickness`; the class keys in a `to[…]`, and styles defined with `\ctikzset`.
- Labels keep out of each component's own band, above and below: a potentiometer's wiper pushes the label up, not the annotation down.

Figures: `resistive-bipoles` `american-and-european-resistors` `capacitors` `inductors` `thermistor-bridge` `fills-and-poles` `class-settings`

## 0.1.0 - 2026-10-28

The first components of CircuiTikZ, drawn as a library of tikra.

- Path-style components: `short`, `open`, `generic`, resistors (American and European), capacitors and cute inductors, placed halfway along `to[…]` and turned with the line.
- Labels and annotations (`l`, `l_`, `a`, `R=label`), straight, rotated or smart, and the siunitx shorthand `20<\ohm>`.
- Poles `o` and `*`, `mirror` and `invert`, `\ctikzset` and the class keys (`resistors/zigs`, `inductors/coils`, `scale`, `width`, `thickness`), the `american` and `european` styles.
- The same core in Rust, the terminal, JavaScript (WebAssembly) and Python, and a site with a playground, a gallery and this page.

Figures: `ladder-network` `labels-around` `european-styles` `mirror-invert`
