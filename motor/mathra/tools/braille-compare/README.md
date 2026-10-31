# braille-compare

Compares mathra's braille with [MathCAT](https://github.com/daisy/MathCAT) (DAISY) on the same MathML, in
Nemeth and in CMU, for every row of `corpus/reference.json`. It is a review aid, not a test: where the two
disagree, the official document decides (see `docs/braille-review.md`). MathCAT's CMU follows the Spanish
ONCE guide; mathra's follows the Brazilian 2006 edition, so some differences are expected.

```sh
cd tools/braille-compare
npm install                                   # MathJax, to turn the LaTeX rows into MathML
node tex2mml.mjs ../../corpus/reference.json rows.json
cp -R "$(dirname "$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name=="mathcat") | .manifest_path')")/Rules" rules
cargo run --release -- rows.json "$PWD/rules" result.json
```

`result.json` has one entry per row and code: both braille strings, as cells and as dot numbers. MathCAT
needs an absolute path to its `rules` folder.
