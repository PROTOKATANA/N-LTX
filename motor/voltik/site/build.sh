#!/bin/sh
# Builds what the playground loads into site/pkg. Open site/index.html straight from disk or serve site/.
# Needs the wasm32-unknown-unknown target and wasm-bindgen-cli at the version in Cargo.lock.
set -e
cd "$(dirname "$0")/.."
cargo build -p voltik-wasm --release --target wasm32-unknown-unknown
rm -rf site/pkg
wasm-bindgen --target no-modules --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/release/voltik_wasm.wasm
# file:// pages cannot fetch the .wasm, but they can run a classic script: the same bytes as base64
printf 'var VOLTIK_WASM_BASE64 = "%s";\n' "$(base64 < site/pkg/voltik_wasm_bg.wasm | tr -d '\n')" > site/pkg/voltik_wasm_bytes.js
# the gallery, every figure of corpus/ (checked by crates/core/tests/corpus.rs), and the banner, as a script
node -e '
  const fs = require("fs");
  // corpus/<topic>/<name>.tex, by topic and then name
  const figures = fs.readdirSync("corpus", { withFileTypes: true }).filter((d) => d.isDirectory()).map((d) => d.name).sort()
    .flatMap((topic) => fs.readdirSync("corpus/" + topic).filter((f) => f.endsWith(".tex")).sort()
      .map((f) => ({ name: f.slice(0, -4), topic, tex: fs.readFileSync(`corpus/${topic}/${f}`, "utf8") })));
  fs.writeFileSync("site/pkg/corpus.js", "var VOLTIK_CORPUS = " + JSON.stringify(figures) + ";\n" +
    "var VOLTIK_BANNER = " + JSON.stringify(fs.readFileSync("docs/banner.tex", "utf8")) + ";\n");
'
# the releases of CHANGELOG.md and the history of docs/progress.tsv, for the evolution page
python3 -I scripts/changelog.py js > site/pkg/changelog.js
# the icons, drawn by voltik itself from site/favicon.tex (1 cm wide: dpi = pixels × 2.54)
cargo build -q -p voltik-cli --release
voltik=target/release/voltik
$voltik site/favicon.tex -o site/pkg/favicon.svg
$voltik site/favicon.tex -o site/pkg/favicon-32.png --dpi 81.28 --transparent
$voltik site/favicon.tex -o site/pkg/apple-touch-icon.png --dpi 457.2 --transparent
