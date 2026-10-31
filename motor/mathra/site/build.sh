#!/bin/sh
# Builds the wasm the playground loads into site/pkg. Open site/index.html straight from disk or serve site/.
# Needs the wasm32-unknown-unknown target and wasm-bindgen-cli at the version in Cargo.lock.
set -e
cd "$(dirname "$0")/.."
cargo build -p mathra-wasm --profile wasm --target wasm32-unknown-unknown
rm -rf site/pkg
wasm-bindgen --target no-modules --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm/mathra_wasm.wasm
# file:// pages cannot fetch the .wasm, but they can run a classic script: the same bytes as base64
printf 'var MATHRA_WASM_BASE64 = "%s";\n' "$(base64 < site/pkg/mathra_wasm_bg.wasm | tr -d '\n')" > site/pkg/mathra_wasm_bytes.js
# the reference page's tables (corpus/reference.json, checked by crates/core/tests/reference.rs) as a script,
# so the page also works opened from disk
printf 'var MATHRA_REFERENCE = %s;\n' "$(cat corpus/reference.json)" > site/pkg/reference_data.js
# the gallery page's groups (corpus/gallery.json, checked by crates/core/tests/examples.rs), the same way
printf 'var MATHRA_GALLERY = %s;\n' "$(cat corpus/gallery.json)" > site/pkg/gallery_data.js
# the releases of CHANGELOG.md, for the evolution page
python3 -I scripts/changelog.py js > site/pkg/changelog.js
