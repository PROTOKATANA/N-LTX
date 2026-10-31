# Corpus

Hand-made circuits, one folder per topic, used by the gallery of the site and by `crates/core/tests/corpus.rs`, which checks that each converts without warnings. Each file is a complete input for `voltik`, and each can be drawn by CircuiTikZ itself with `tools/oracle/tex.sh` to compare.

| Folder | What its figures show |
|---|---|
| `classics/` | Circuits every textbook has: rectifiers, op-amp stages, transistor pairs, oscillators, CMOS gates, latches, counters, converters, and the infinite networks (ladders, grids, the resistor cube) |
| `circuits/` | Small whole circuits: filters, dividers, ladders, resonant tanks |
| `components/` | The components of each family, side by side: resistive bipoles, American and European resistors, capacitors, inductors |
| `labels/` | Labels and annotations: sides, units, straight, rotated and smart alignment |
| `styles/` | American and European styles, `\ctikzset` settings, `mirror` and `invert`, fills and poles |

Add a figure by dropping a `.tex` file in its folder (the name, without `.tex`, becomes its title in the gallery: `voltage-divider.tex` is "Voltage divider"); the test and the gallery pick it up. Names are unique across folders.
