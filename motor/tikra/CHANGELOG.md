# Changelog

What changed in each release of tikra: the crates `tikra` and `tikra-cli`, `@mathlabs/tikra` on npm and `tikra` on PyPI, which are released together. The site shows the same, with the figures drawn live: [the project's evolution](https://matheusmota-dev-br.github.io/tikra/changelog.html).

Each release has a headline (the line under its title), what changed, and the figures of [`corpus/`](corpus) that show it (`Figures:`), drawn by the site with today's tikra. `scripts/release.sh` will not release a version without its section here, and makes the GitHub release from it.

## 1.8.1 - 2026-10-30

The distance of a pin or a label among its own options, tikzmark's `pic anchor`, what extensions need to draw, and labels set by mathra 1.9.2.

- A shape path of an extension can have an `opacity` of its own: code listing every field of `ShapePath` needs it ([#146](https://github.com/matheusmota-dev-br/tikra/issues/146)).
- `>` with no value runs `bare arrow/.try`, for libraries to give it a meaning ([#146](https://github.com/matheusmota-dev-br/tikra/issues/146)).
- tikzmark's `pic anchor=(-out)`: a pic placed by a coordinate of its own ([#146](https://github.com/matheusmota-dev-br/tikra/issues/146)).
- `Context::is_dashed`, for an extension to draw by the path's dashes ([#146](https://github.com/matheusmota-dev-br/tikra/issues/146)).
- `pin={[pin distance=…]…}` and `label={[label distance=…]…}`: the distance among a pin's or a label's own options is the one used ([#146](https://github.com/matheusmota-dev-br/tikra/issues/146)).
- Labels are set by mathra 1.9.2: `\boldmath` sets letters in bold italic and reaches the text of a `\rotatebox`, and `\SI{+10}{V}` and `\SI{.25}{V}` print their values as siunitx does.

## 1.8.0 - 2026-10-29

The figures of texample.net drawn as TeX draws them, the CircuiTikZ manual's last figures, multi-line nodes on TeX's baseline, and labels set by mathra 1.9.1.

- `pin=\a:\a`, `label=\a:…`: an angle held in a macro (a `\foreach` variable) is expanded instead of ignored ([#112](https://github.com/matheusmota-dev-br/tikra/issues/112)).
- A `to` path keeps the `+` reference point: after `to[…] +(2,0)`, `+(0,0)` is still the start ([#112](https://github.com/matheusmota-dev-br/tikra/issues/112)).
- In the text around pictures: `\emph`, `\textit` and the like, the control space `\ `, and `\pgfversion` (3.1.12) ([#112](https://github.com/matheusmota-dev-br/tikra/issues/112)).
- `\pgfuseimage` and `\includegraphics` of mwe's `example-image-a` draw TeX Live's placeholder ([#112](https://github.com/matheusmota-dev-br/tikra/issues/112)).
- A color set inside a path's `{…}` group colors only its text; options in the middle of a path (`[dashed, red]`) also apply to the nodes drawn earlier on it ([#112](https://github.com/matheusmota-dev-br/tikra/issues/112)).
- standalone's `varwidth=`: a `\color` at the document's top level makes the page that wide, as TeX does ([#112](https://github.com/matheusmota-dev-br/tikra/issues/112)).
- A node's lines (`align=`) sit on the last line's baseline, as TeX's `\vbox` does (in a `text width`, on the first), and a `tabular` on the math axis; `@{}` next to a column of a tabular takes its `\tabcolsep` away ([#112](https://github.com/matheusmota-dev-br/tikra/issues/112)).
- Code that builds a `scene::Label` listing every field needs the new `lift` ([#112](https://github.com/matheusmota-dev-br/tikra/issues/112)).
- xcolor's named color sets: every color of the dvipsnames (`BrickRed`, `MidnightBlue`), svgnames (`DimGray`, `Gold`) and x11names (`Red3`, `Gold2`), with the values TeX gives them. A set loaded with `\usepackage[…]{xcolor}`, a class option or `\PassOptionsToPackage{…}{xcolor}` defines its colors over those before it, as in xcolor (`Gray` is the svgnames' after `[dvipsnames,svgnames]`); in a document that loads no set, where TeX stops on the name, the name is still drawn, the dvipsnames' first (#56).
- `\xdefinecolor` defines a color as `\definecolor` does (#57).
- `\the<counter>` of a `\newcounter` is the counter's value, as LaTeX prints it: in an expression (`(\thea/100*360:1)`), a color (`red!\thedensity`) and a node's text (#66).
- `\pgfmathsetcounter`, `\pgfmathaddtocounter` and `\pgfmathaddtolength`, as pgfmath does them: the value truncated toward zero for a counter, set locally (a `\foreach` body keeps it to itself, unlike `\setcounter`). A `\whiledo` on a counter they move ends (#67).
- `\pgfmathsetlength` reads a plain number as points, as pgfmath does: `2cm+3` is 59.9pt, not 5cm (#68).
- `pin edge={…}` given through a style of the pin or `every pin` sets the pin's edge, the last one given winning, as in TeX (#70).
- `samples at={…}`: a plot at the values of a `\foreach` list (`0,0.25,...,6`); a later `samples` or `domain` takes over again (#71).
- `edge from parent[…]` of a child that has children of its own is no longer lost: `edge from parent[draw=none]` hides that edge (#72).
- The keys of PGF 1.x's `snakes` library: `snake=coil` (any decoration) decorates each straight line of the path, and `line before snake`, `line after snake`, `gap before snake`, `gap after snake`, `gap around snake`, `raise snake` and `mirror snake`; `segment amplitude` also sets the shape height without a warning (#73).
- PGF 1.x's `set style={{name}+=[options]}` adds to a style and `set style={{name}=[options]}` defines one (#74).
- `\tikzstyle name=[options]` with the name not in braces, as `\tikzstyle directed=[…]` (#75).
- `dash pattern=on5pt off3pt`: no space is needed after `on` and `off` (#76).
- `right of=a` and its kin shift by a `node distance` given after them in the same options too (#77).
- pgfmath functions without parentheses, read as TeX reads them: `sin 30`, `cos \x`, `sin 2*30` (sin(2) times 30), `sin 30^2` (#59).
- An array of strings in pgfmath, `{"orange","blue"}[\i]`, gives the string in `\pgfmathparse` and `\pgfmathsetmacro` (#60).
- Lengths in an expression count in points, as pgfmath counts them: `1/10pt` is 0.1pt (it was 2.8cm) and `3pt*2cm` is 170.7pt (#61).
- pgfmath's inner commands, `\pgfmathadd@`, `\pgfmathmultiply@`, `\pgfmathsin@`…, used with `\makeatletter` (#62).
- pgfmath's random numbers: `random()`, `random(m)` and `random(a,b)` from the same seed as TeX, `\pgfmathrand` and `\pgfmathrnd` (also inside a point argument, before the point), `\pgfmathrandominteger`, `\pgfmathdeclarerandomlist` and `\pgfmathrandomitem` (#63).
- `\pgfdecorationsegmentaspect` in decoration code: the value of `aspect` (#64).
- `join=with a by {options}`: the braces only group the options (#65).
- A node's text breaks into lines only at the `\\` of the text itself. A `\\` inside `\[ … \]`, `\( … \)`, `$$ … $$` or an environment (`align*`, `matrix`, `tabular`) belongs to it and goes to mathra whole, and `text width` keeps those in one piece when it wraps (#80).
- `\textcolor{…}{…}` inside math in a node colors that part of the formula instead of breaking it (#81).
- The repair of TikZ escaped for Markdown no longer fires on display math (`\[ … \]`) or on a `\\` before `\end`: it now undoes `\[` only after a TikZ command (`\draw\[red\]`) or before one (#82).
- amsmath's `\DeclareMathOperator{\p}{p}` (and starred) defines `\p` for labels (#83).
- `\newline` in a node's text ends the line in a `text width`, as `\\` does; out of one it does nothing, as in TeX (#84).
- `text width` wraps text in another family (`font=\sffamily`, `\ttfamily`, `\itshape`), which was set on one line (#85).
- A node's text that is all a `\parbox[t]{2cm}{…}` or a `minipage` wraps to that width, justified unless it begins with `\centering`, `\raggedright` or `\raggedleft` (#86).
- `\geometry{…}` is no longer printed, and `\resizebox{7cm}{!}{…}` (and `\resizebox*`) around pictures scales them to that size as graphicx does, instead of printing its arguments (#87).
- `\calendar[name=cal, …]` names its days `cal-2000-01-20`, as `\calendar (cal)` does (#99).
- `\ifdate` and a calendar's `if (…)` run the tests a document defines as `/pgf/calendar/<name>/.code`, with the date in `\pgfcalendarifdateyear`, `…month`, `…day`, `…weekday`, `…julian` and `\pgfcalendarmatchestrue` (#100).
- Calendar layouts of the document's own can use pgf's `\c@pgf@counta` … `\c@pgf@countd` and the library's `\tikz@lib@cal@xshift`, `…yshift`, `…month@xshift`, `…month@yshift`, `…width`; such a layout no longer stops tikra with "nesting went past 64 levels" (#101).
- `\pgfdeclareshape` runs a shape's code for each node as pgf does: `\savedanchor`, `\saveddimen` and `\savedmacro` (from the node's text box `\pgfnodeparttextbox` and its sizes `\pgfshapeminwidth`, …), `\anchor{name}{code}` (also as `\pgf@anchor@<shape>@<name>`), the paths of every layer (`\behindbackgroundpath`, `\beforebackgroundpath`, …) with what their code fills, strokes and writes (`\pgftext`), and the rectangle's border around the shape's own corners; only the paths it inherits are drawn, and its name works as an option. Also `\pgf@process`, `\pgfextractx`, `\pgfextracty`, `\pgfclosepath` and `\c@pgf@counta`, and a style that appends to `\tikz@textfont` no longer loops (#103).
- `\tikzoption{name}[default]{code}` defines the key with its default value (#104).
- An anchor that is an expression, `(a.200+360/5)`, is the angle it gives, as pgf evaluates it (#105).
- The `svg` path operation takes its data in quotes too: `svg "M0,0 L20,10"` (#106).
- `\g@addto@macro`, etoolbox's `\appto`, `\gappto`, `\eappto`, `\xappto`, `\preto` and kin, and babel's `\addto` add to a macro as TeX does (#89).
- `\numexpr …\relax` and `\dimexpr …\relax` in a pgfmath expression or a coordinate are read as TeX reads them (#90).
- `\newcounter{name}` defines `\thename` (#66).
- `\pgfgetlastxy{\x}{\y}`: the last point of the last path, in pt (#91).
- etoolbox's `\ifstrempty`, `\ifblank` and `\ifdefempty` (#92), and its toggles: `\newtoggle`, `\providetoggle`, `\toggletrue`, `\togglefalse`, `\settoggle`, `\iftoggle`, `\nottoggle` (#93).
- xstring's `\StrLen`, `\StrLeft`, `\StrRight`, `\StrMid`, `\StrChar`, `\StrBefore`, `\StrBehind`, `\StrCut`, `\StrBetween`, `\StrSubstitute`, `\StrDel`, `\StrCount`, `\IfSubStr`, `\IfStrEq`, `\IfBeginWith` and `\IfEndWith` (#94).
- pgfkeys' `/K/.is family`: the key `/K` goes to that path (#95).
- TikZ's `\tikz@checkunit` (with `\iftikz@isdimension`), `\pgf@xx`, `\pgf@xy`, `\pgf@yx`, `\pgf@yy` in an expression, and `\tikz@textfont` (#96).
- LaTeX's page lengths: `\paperwidth` and `\paperheight` (letter paper, or the size of the class or geometry's `a4paper`, `a5paper`… and `landscape`), `\textheight`, `\oddsidemargin`, `\topmargin`, `\headheight`, `\parskip`, `\columnsep` and the others `article` sets, and the caption package's `\captionmargin`: usable in coordinates, and `\setlength` no longer warns about them (#97).
- LaTeX's boxes: `\newsavebox`, `\savebox`, `\sbox` and `lrbox` keep what they typeset (a picture, a `tabular` of pictures and words), and `\usebox` draws it in a node's text or in the document, the node as large as the box; an `overlay` picture is a box of no size, drawn around the node's center (#108).
- With the active preview package's `\PreviewEnvironment{tikzpicture}`, a `\scalebox`, `\resizebox` or `\fbox` around a picture no longer scales or frames it: each picture is a page of its own, as TeX makes it (#109).
- The words of a `tabular` cell in the document keep the spaces between them (#110).
- A node or pic placed `at +(x,y)` (or `++`) is placed from the path's last point, as TikZ does, instead of being dropped with "at without a point"; `pic foreach … at +(…)` too (#114).
- `node (a) (b) {…}` is named b, as in TikZ, instead of ending after its first name (#115).
- In a shifted, scaled or turned scope, `let`'s `\x1` and `\y1` and the vectors `x=(node)`, `y=(node)` are in the scope's coordinates, as TeX gives them, so `(\x1,0)` lands below the point; `\p{\name}` expands its name (#116).
- calc's `($(A)!1cm!(B)$)` measures the distance before the transformation: 3cm on the page under `scale=3`, as TeX (#117).
- A node's text runs its `\def`, `\let`, register assignments, `\@ifnextchar` and `\setkeys` as TeX does while typesetting it, and draws a picture made by a macro after them (its `\foreach \number` included); `\relax` typesets nothing; pgf's `\c@pgf@counta`… counters and `\pgfmathloop … \repeatpgfmathloop` (#118).
- The `pic text` and `pic text options` keys set `\tikzpictext` and `\tikzpictextoptions` (#119).
- `\foreach \a in {\pa,\pb}` keeps its items as they are written, so a macro holding a whole path with commas is one item, as in TeX; the ends of a `...` range are still read (#120).
- A macro in a `declare function` body is read when the function is used (#121).
- `parabola height` is scaled with the picture, as the bend is a point of it (#123).
- `\spy on (point)` expands the macros in its point when the spy scope ends (#124).
- `let \p1=…`: `\x1` and `\y1` are in the picture's coordinates, so `(\x1,\y1)` under `scale` is the point again (#125).
- `xyz polar cs` and `polar cs` read `radius`, `x radius` and `y radius` in order: the last one given wins (#126).
- `to[out=…,in=…]` and `bend` curves are computed in the picture's coordinates: they turn, slant and stretch with it (#127).
- A functional shading on a path is sampled only under the path: more than twice as fine (#128).
- Nothing after `\end{document}` is typeset (#129).
- `\title`, `\author` and `\date` are no longer typeset as text (#130).
- `below of=…` and the other old placement keys take the node distance the node ends up with, also when it is given after them (#131).
- With `text width`, a word too long for the last line goes to a line of its own, as TeX's last pass does (#132).
- calc: a distance toward the point itself (`$(a)!1cm!(a)$`) goes up, as TeX's direction of no length (#133).
- `\chainin [options]` keeps the `after node path` its options set (joins of one's own), and `\pgfkeysnovalue` expands to nothing instead of forever (#134).
- Matrix delimiters no longer take the cells' `nodes={…}` style (#135).
- pgfmath: an array of strings, `{"red","blue"}[1]`, gives its item (#136).
- The angle of `label=` and `pin=` may come from a macro (`label=\a+15:text`), instead of silently becoming 90 (#137).
- `\draw[circle connection bar] (a) edge (b)` is outlined and `\fill[color,…]` fills it in that color; a mind map's edges stay filled with the concept colors (#138).
- Labels are set by mathra 1.9.1: a slanted letter at the end of math gets its italic correction, as in TeX (`$V$`, `$f$` are wider), and italic text (`\itshape`, `\textit`, `\emph`) is mathra's italic text, without those corrections, as wide as TeX sets it (#143).
- Our 100% (the figures a converter can reproduce) is 2480 of 2486, as in 1.7.0.

**Matches TeX:** 2371 of 2502 figures of the manuals (94.8%).

Figures: `labels-multiline` `labels-pins-quotes` `text-alignment-boxes`

## 1.7.0 - 2026-10-29

Figures you drag by their points, labels kerned as TeX kerns them, and cmyk colors as TeX's PDF shows them.

- In the playground, each point the code names gets a handle on the figure: dragging it rewrites the numbers that move it, in the code. A demo drags the figure by itself; "Interactive HTML" downloads the figure as one page that works offline, as does `scripts/interactive-html.mjs`. A range comment can give a step: `% -4..4:0.1` (#40).
- `Output::points`: every node and coordinate the author named, where it is in the SVG, and `Output::origin`, where the picture's (0,0) landed; both are in the wasm `render` JSON (#40).
- A color ending in `!` is read as xcolor reads it: `red!50!` is `red!50!white`, and `red!` is red (#41).
- The preview package's setup (`\PreviewEnvironment{tikzpicture}`, `\setlength\PreviewBorder{5pt}`) is no longer drawn as text or warned about (#42).
- pgfmath's `Mod` (never negative: `Mod(-7,3)` is 2) and `\pgfmathMod`, and a quoted string as the value of `?:` (`Mod(\k,2) ? "pink" : "purple!50"`), also in `\pgfmathsetmacro` (#43).
- A `rectangle callout` narrower than its pointer is drawn instead of stopping tikra (#44).
- `shift={(0,\R)}` reads a length register as a point on a path does (#45).
- In a `\matrix` cell, a named point is read in the cell's own coordinates, as TeX sets the cell before moving it into place: `(A)` from outside the matrix moves with the cell, and a node from another cell is where it was in that cell (#47).
- `\pagecolor{…}` (and `\pagecolor[model]{…}`) paints the page behind the figure, margin included, as TeX does, instead of being drawn as text; `\nopagecolor` takes it away (#50).
- `tikra::extension`: a shape's text can be turned around its point (`ShapeText::turn`, a label along a side) and kept upright when its node turns (`ShapeText::upright`, a turned chip's pin numbers) (#39).
- `left=30`, `above=.2`…: a bare number is in points, as TikZ reads it, and in the x and y vectors' units only with the `positioning` library (#52).
- Code that builds a `ShapeText` listing every field needs `..Default::default()` for the new ones (#39).
- Labels in Latin Modern are kerned as TeX kerns them: by the OT1 fonts (`cmr10`, `cmbx10`) by default and the T1 ones (`ec-lmr10`, `ec-lmbx10`) after `\usepackage[T1]{fontenc}` (`va`, `AV`, `To`) (#48; mathra#81). 3 more figures match TeX.
- LaTeX's `picture` environment in a text: `\begin{picture}(w,h)(x,y)` with `\put`, `\multiput`, `\line`, `\circle`, `\circle*`, `\thinlines`, `\thicklines` and `\linethickness`, in `\unitlength`, drawn as LaTeX draws it (#53).
- Images in a text: `\pgfdeclareimage[width=…, height=…]{name}{file}` and `\pgfuseimage{name}`, `\pgfimage` and `\includegraphics` in a node or `\pgftext`, a PNG file in a box of its size (#53).
- `\begingroup`, `\endgroup` and `\relax` in a text no longer leave the space after them.
- `\NewDocumentCommand` and its kin (`Renew`, `Provide`, `Declare`, `Expandable`): commands with arguments `m`, `o`, `O{…}`, `s`, `t`, `d`, `D`, `r` and `R`, and `\IfNoValueTF`, `\IfValueTF`, `\IfBooleanTF` and their `T` and `F` forms (#53).
- The `Jack Tap` arrow tip of CircuiTikZ: a step, then half a triangle, 0.3cm by 0.15cm whatever the line width; `swap`, `reversed`, `width`, `length`, `fill`, `open` and the line cap and join (#53).
- `\scalebox{0.7}{…}` (and `\scalebox{2}[1]`) around pictures in a document scales them, their lines and texts with them, as graphicx does.
- Colors in xcolor's cmyk (`cyan`, `magenta`, `yellow`, `olive`, the dvipsnames, `\definecolor{x}{cmyk}{…}`) are drawn as Ghostscript draws them from TeX's PDF, through its ICC profiles: `cyan` is #00aeef, `cyan!30!white` #abe1fa (`tools/cmyk/table.py`). Mixes are in their first color's model, as xcolor mixes them (`white!70!cyan` is cmyk, `red!50!cyan` rgb), and so are complements and the extended syntax's `cmyk,1:…` and `gray,1:…` (#53).
- `tikra::color::Color` has a `model`: code that builds one with `Color { r, g, b }` uses `Color::rgb(r, g, b)` (#53).
- `tikra::extension`: `Context::measure` runs code without drawing it and gives the box of what it drew, with what a closure read before its nodes are forgotten (a subcircuit placed by one of its anchors) (#53).
- Labels are set by mathra 1.8.0: inside text, `\hspace{…}` adds its space and `\phantom`, `\hphantom` and `\vphantom` leave the room of their argument, and text is kerned by TeX's OT1 or T1 fonts.
- Our 100% (the figures a converter can reproduce) is 2480 of 2486, down from 2481.

**Matches TeX:** 2370 of 2502 figures of the manuals (94.7%).

Figures: `inscribed-angle-theorem` `bezier-control-points` `parallelogram-rule`

## 1.6.0 - 2026-10-29

Labels in Times, fills without seams, and library shapes with texts and styles of their own.

- Labels in `\fontfamily{ptm}` are drawn in Times (TeX Gyre Termes, in its four styles), as wide as TeX sets them and kerned as TeX kerns them (#35).
- In bold text, `\emph` and `\textit` are bold italic, as in TeX, and the text after a formula stays bold (#35; mathra 1.7.1).
- Fills side by side no longer show a seam of the background along the edge they share: grids of cells, bars and faces of solids are solid (#35).
- A path filled and stroked at different opacities is drawn as PDF draws it: under a translucent stroke you see what is under the path, not its fill (#35).
- `tikra::extension`: a library shape's paths may have a color, a dash pattern and line caps of their own (#32). Its shapes may draw texts of their own, turned and mirrored with the node, in its size and its words (#33). A shape knows its node's name, its fill and line colors and its transformation, and a key knows whether `transform shape` is on and can define TeX as tokens (`Context::define_tex`) (#34).
- Code that builds a `ShapePath` or a `NodeShape` listing every field needs `..Default::default()` for the new ones (#36).
- Colors read xcolor's extended syntax (`rgb,1:red,1;green,0.5;blue,0`).
- Labels are set by mathra 1.7.1: the text after a formula keeps its bold, small capitals or monospace; `\phi` (and `\epsilon`, `\vartheta`, `\varrho`, `\varpi`) in math italic as TeX sets them; `\textsubscript`, `\textsuperscript`, `\ensuremath` and `\boldmath` inside text; siunitx ranges; graphicx's `\scalebox`, `\rotatebox`, `\resizebox` and `\raisebox`, drawn turned and scaled in the label.
- Our 100% (the figures a converter can reproduce) is 2481 of 2486, up from 2476.

**Matches TeX:** 2368 of 2502 figures of the manuals (94.6%).

Figures: `spotlight-clipping` `fading-directions`

## 1.5.0 - 2026-10-28

Node shapes drawn by a library.

- `tikra::extension`: a library declares node shapes with a Rust function (`Context::declare_shape`), computed for each node from its text box: its anchors, its border, where its text goes, its paths (stroked as wide as it says, filled with the node's fill, with its fill or white, or solid in its line color) and what else counts in the picture's size. One function may draw several shapes, told apart by name. They are nodes as any other: placed by their anchors (`anchor=`), turned with the node, with lines stopping at their border (#29).
- Drawing is unchanged: every figure is the same as 1.4.2's.

**Matches TeX:** 2365 of 2502 figures of the manuals (94.5%).

## 1.4.2 - 2026-10-28

How to contribute, and how to report a vulnerability.

- A code of conduct (Contributor Covenant 2.1), a contributing guide and a security policy, in `.github/`. Vulnerabilities are reported privately through the repository's Security tab.
- The checks run on the contributor's machine: `scripts/check.sh` runs fmt, clippy and the Rust, wasm, npm and Python tests, and the git hooks in `.githooks/` run it before each commit and push. GitHub runs them again on `main`, not on pull requests.
- Drawing is unchanged: every figure is the same as 1.4.1's.

**Matches TeX:** 2365 of 2502 figures of the manuals (94.5%).

## 1.4.1 - 2026-10-27

The engine opens to libraries from outside tikra.

- `tikra::extension`: a package drawn on TikZ can live in a crate of its own, as TeX definitions run once and keys written in Rust, without a copy of the engine. `tikra::convert_with_libraries` converts with them defined next to TikZ's own.
- TikZ's circuit libraries go through the same opening, which shows it is enough for them; they draw as before.
- The engine's internal modules are hidden from the documentation: they are not an API and change with any release. What is kept stable is `convert`, `png` and `extension`.
- Nothing changes for the CLI, npm and PyPI.

**Matches TeX:** 2365 of 2502 figures of the manuals (94.5%).

## 1.4.0 - 2026-10-27

The figure draws itself, in the order TikZ drew it.

- The construction animation: `--animate`, `{ animate: true }`, `animate=True`. Each stroke is drawn along its length and its fill follows, all in about six seconds; off by default, and the SVG without it is unchanged byte for byte.
- The steps apart from the animation: `--steps`, `{ steps: true }`, `steps=True` number each element by the order TikZ drew it, for a page that walks through the figure.
- The playground walks through the figure step by step, gives a slider to each number the code defines (`\def`, `\pgfmathsetmacro`, `\newcommand`), and downloads the animation as a GIF, a video or an animated SVG with its own controls.
- The gallery animates every figure, and figures with numbers to change have their sliders.
- Five figures in the corpus with numbers to change, a topic of their own; the corpus has 241 figures.

**Matches TeX:** 2365 of 2502 figures of the manuals (94.5%).

Figures: `sine-wave-controls` `polar-rose-petals` `lissajous-figure` `regular-polygon-sides` `projectile-launch`

## 1.3.2 - 2026-10-27

How tikra grew, on the site and in this changelog.

- The site's [evolution page](https://matheusmota-dev-br.github.io/tikra/changelog.html): every release with its figures drawn live, and the conformance over time as a chart tikra draws.
- This changelog, from 1.0.0 on; each release on GitHub is made from it.
- The playground's editor has a maximum height and scrolls inside: a figure with a lot of data, such as the Lorenz attractor, no longer stretches the page ([#17](https://github.com/matheusmota-dev-br/tikra/issues/17)).
- pgf's and pgfplots' key definitions written anew, clean-room, with credits to the original authors.

**Matches TeX:** 2365 of 2502 figures of the manuals (94.5%).

## 1.3.1 - 2026-10-26

Molecules and commutative diagrams inside the drawing.

- The bonds of a molecule in a label take the label's color, as its letters do.
- Lines of a label too tall for the distance between baselines are a `\lineskip` apart, as in TeX.
- A function declared with `\pgfmathdeclarefunction` is plotted, and a plot that fails says why.
- Three figures in the gallery that only tikra draws: molecules and commutative diagrams in nodes.

**Matches TeX:** 2365 of 2502 figures of the manuals (94.5%).

Figures: `methylxanthines` `adjunctions` `substitution-energy-profile`

## 1.3.0 - 2026-10-26

Our 100% at 99.6%: of the figures a converter can reproduce, almost every one is drawn as TeX draws it.

- Snapshots of the system layer's animations, and of `whom:` with several attributes.
- pgfplots computes as pgf's fpu does: `exp`, square roots, log ticks and log samples.
- `shader=interp` shades smoothly, without seams, in surfaces and matrix plots; `\addplot graphics` draws a PNG image.
- Rounded corners as pgf makes them, curves included; Smith charts, callouts and patterns as pgf draws them.
- A path filled and stroked at one opacity is painted as one group, as in PDF.
- Labels set by mathra 1.5.0.

**Matches TeX:** 2365 of 2502 figures of the manuals (94.5%), up from 93.1%.

Figures: `clock-snapshots` `smooth-shaded-surface` `interpolated-heat-map` `matrix-with-moustaches` `rounded-corners-on-curves` `smith-chart-scaled`

## 1.2.1 - 2026-10-25

The 60-second demo.

- The [demo video](https://youtu.be/vTX69CwtjB4) in the README and on the site.
- The GitHub Sponsors button on the repository.

**Matches TeX:** 2330 of 2502 figures of the manuals (93.1%).

## 1.2.0 - 2026-10-24

The figure in words, for readers who cannot see it.

- Every figure comes with its alt text and a description of what it shows (axes, data, nodes and their connections), written into the SVG's `<title>` and `<desc>`.
- In 16 languages: `render()` in JavaScript and Python, `--describe` and `--locale` in the terminal.
- The playground tells the figure in words.

**Matches TeX:** 2330 of 2502 figures of the manuals (93.1%).

Figures: `petersen-graph` `normal-distribution`

## 1.1.1 - 2026-10-24

Every registry shows the new README.

- The READMEs of crates.io, npm and PyPI up to date, with famous figures as examples.

**Matches TeX:** 2330 of 2502 figures of the manuals (93.1%).

Figures: `golden-spiral`

## 1.1.0 - 2026-10-24

An engine that reads TikZ the way TikZ does, and almost every library of TikZ and pgfplots.

- A new engine: a small TeX that reads the figure as TeX reads it, pgfkeys, and pgf's basic layer.
- Almost every TikZ library: shapes, arrows, decorations, matrices, trees, graphs, mind maps, circuits, automata, calendars, fadings, shadings, patterns, spy, 3d and more.
- pgfplots: 2D and 3D axes, surfaces, colormaps and colorbars, polar, ternary and Smith charts, statistics, groupplots.
- Labels in Latin Modern Math, TeX's Computer Modern, at the widths of TeX's own fonts.
- The conformance report: every example of the TikZ and pgfplots manuals, drawn by TeX and by tikra and compared. 3.5% matched on its first day, October 20.
- The corpus grew from 25 figures to 227.

**Matches TeX:** 2330 of 2502 figures of the manuals (93.1%).

Figures: `lorenz-attractor` `nightingale-rose` `feynman-diagram` `periodic-table-corner` `surface-3d`

## 1.0.0 - 2026-10-21

The first release: TikZ and pgfplots figures to SVG and PNG, with no TeX installation.

- A subset of TikZ and pgfplots drawn in Rust, labels typeset by [mathra](https://github.com/matheusmota-dev-br/mathra).
- Everywhere at once: the `tikra` crate and command on crates.io, `@mathlabs/tikra` on npm (WebAssembly, for Node, bundlers and browsers) and `tikra` on PyPI, with the same API.
- The site: a playground, a gallery and the docs.
- A corpus of 25 hand-made figures.

Figures: `triangle-angles` `unit-circle` `flowchart` `bar-chart`
