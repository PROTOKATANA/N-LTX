#!/bin/sh
# Builds what the playground loads into site/pkg. Open site/index.html straight from disk or serve site/.
# Needs the wasm32-unknown-unknown target and wasm-bindgen-cli at the version in Cargo.lock.
set -e
cd "$(dirname "$0")/.."
cargo build -p tikra-wasm --release --target wasm32-unknown-unknown
rm -rf site/pkg
wasm-bindgen --target no-modules --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/release/tikra_wasm.wasm
# file:// pages cannot fetch the .wasm, but they can run a classic script: the same bytes as base64
printf 'var TIKRA_WASM_BASE64 = "%s";\n' "$(base64 < site/pkg/tikra_wasm_bg.wasm | tr -d '\n')" > site/pkg/tikra_wasm_bytes.js
# the gallery, every figure of corpus/ (checked by crates/core/tests/corpus.rs), and the banner, as a script
node -e '
  const fs = require("fs");
  // corpus/<topic>/<name>.tex, by topic and then name
  const figures = fs.readdirSync("corpus", { withFileTypes: true }).filter((d) => d.isDirectory()).map((d) => d.name).sort()
    .flatMap((topic) => fs.readdirSync("corpus/" + topic).filter((f) => f.endsWith(".tex")).sort()
      .map((f) => ({ name: f.slice(0, -4), topic, tex: fs.readFileSync(`corpus/${topic}/${f}`, "utf8") })));
  fs.writeFileSync("site/pkg/corpus.js", "var TIKRA_CORPUS = " + JSON.stringify(figures) + ";\n" +
    "var TIKRA_BANNER = " + JSON.stringify(fs.readFileSync("docs/banner.tex", "utf8")) + ";\n");
'
# the releases of CHANGELOG.md and the history of docs/progress.tsv, for the evolution page
python3 -I scripts/changelog.py js > site/pkg/changelog.js
# the conformance numbers of docs/conformance.md, for the "How close to TikZ" section
python3 -I - > site/pkg/conformance.js <<'PY'
import json, re
md = open("docs/conformance.md", encoding="utf-8").read()
exact = re.search(r"\*\*(\d+) of (\d+) figures \(([\d.]+%)\) match TeX\*\*", md)
ours = re.search(r"\*\*Our 100%: (\d+) of (\d+) \(([\d.]+%)\)\*\*.*?the (\d+) figures no converter.*?counts as done (\d+)", md)
data = {
    "exact": {"done": int(exact[1]), "of": int(exact[2]), "pct": exact[3]},
    "ours": {"done": int(ours[1]), "of": int(ours[2]), "pct": ours[3]},
    "left_out": int(ours[4]), "aliased": int(ours[5]),
}
print("var TIKRA_CONFORMANCE = " + json.dumps(data) + ";")
PY
# the icons, drawn by tikra itself from site/favicon.tex (1 cm wide: dpi = pixels × 2.54)
cargo build -q -p tikra-cli --release
tikra=target/release/tikra
$tikra site/favicon.tex -o site/pkg/favicon.svg
$tikra site/favicon.tex -o site/pkg/favicon-32.png --dpi 81.28 --transparent
$tikra site/favicon.tex -o site/pkg/apple-touch-icon.png --dpi 457.2 --transparent
