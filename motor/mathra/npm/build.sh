#!/bin/sh
# Builds the npm package into npm/pkg: one wasm, loaded from disk in Node and fetched in browsers.
# Needs the wasm32-unknown-unknown target and wasm-bindgen-cli at the version in Cargo.lock.
set -e
cd "$(dirname "$0")/.."
# the npm package has its own version (npm/package.json); `version` in the API is the one of mathra's core
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
cargo build -p mathra-wasm --profile wasm --target wasm32-unknown-unknown
wasm=target/wasm32-unknown-unknown/wasm/mathra_wasm.wasm
out=npm/pkg
tmp=$(mktemp -d)
rm -rf "$out"
mkdir -p "$out/web" "$out/node"
wasm-bindgen --target web --no-typescript --out-dir "$tmp/web" "$wasm"
wasm-bindgen --target nodejs --no-typescript --out-dir "$tmp/node" "$wasm"
cp "$tmp/web/mathra_wasm_bg.wasm" "$out/mathra.wasm"
# both glue files point at the single wasm at the package root
sed "s|new URL('mathra_wasm_bg.wasm', import.meta.url)|new URL('../mathra.wasm', import.meta.url)|" "$tmp/web/mathra_wasm.js" > "$out/web/mathra_wasm.js"
sed 's|`${__dirname}/mathra_wasm_bg.wasm`|`${__dirname}/../mathra.wasm`|' "$tmp/node/mathra_wasm.js" > "$out/node/mathra_wasm.cjs"
grep -q "'../mathra.wasm'" "$out/web/mathra_wasm.js" && grep -q '/../mathra.wasm' "$out/node/mathra_wasm.cjs" || { echo "wasm-bindgen output changed: fix the paths in npm/build.sh" >&2; exit 1; }
sed "s/__VERSION__/$version/" npm/src/web.js > "$out/web/index.js"
cp npm/src/wrap.js "$out/web/wrap.js"
sed "s/__VERSION__/$version/" npm/src/node.cjs > "$out/node/index.cjs"
cp npm/src/node.mjs "$out/node/index.mjs"
sed -e 's/^export function wrap/function wrap/' npm/src/wrap.js > "$out/node/wrap.cjs"
echo 'module.exports = { wrap };' >> "$out/node/wrap.cjs"
cp npm/src/index.d.ts "$out/index.d.ts"
cp npm/src/index.d.ts "$out/index.d.cts"
cp npm/README.md LICENSE-MIT LICENSE-APACHE crates/core/fonts/OFL-STIXTwoMath.txt "$out/"
cp npm/package.json "$out/package.json"
rm -rf "$tmp"
echo "npm/pkg: $(node -p "require('./npm/package.json').name + '@' + require('./npm/package.json').version") with mathra $version"
