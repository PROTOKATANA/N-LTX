# Corpus

Hand-made figures, one folder per topic, used by the gallery of the site, by the benchmark (`cargo run --release -p tikra --example bench`) and by `crates/core/tests/corpus.rs`, which checks that each converts without warnings. Each file is a complete input for `tikra`.

| Folder | What its figures show |
|---|---|
| `parameters/` | Figures with numbers to change (`\def\n{6} % 3..12`): the site gives each a slider (a step after a colon: `-4..4:0.1`), and each point the code names (`\coordinate (C) at (\cx,\cy);`) that they move a handle to drag on the figure |
| `geometry/` | Points, lines, circles, angles, transformations and coordinates |
| `plots-2d/` | TikZ plots and pgfplots axes in two dimensions |
| `plots-3d/` | Surfaces, meshes, curves and heat maps with pgfplots |
| `charts/` | Bars, pies, histograms, error bars and other statistics |
| `special-axes/` | Polar, ternary and Smith charts, and data visualization |
| `diagrams/` | Flowcharts, graphs, trees, mind maps, automata and Petri nets |
| `science/` | Circuits, logic gates, physics, optics and chemistry |
| `decorations/` | Decorations, markings, text along paths, arrow tips and fractals |
| `fills-and-colors/` | Patterns, shadings, fadings, opacity, blending and clipping |
| `nodes-and-text/` | Node shapes, labels, matrices, calendars and text layout |

Add a figure by dropping a `.tex` file in its folder (the name, without `.tex`, becomes its title in the gallery: `polar-rose.tex` is "Polar rose"); the test, the benchmark and the gallery pick it up. Names are unique across folders.
