#!/bin/sh
# Builds the npm package into npm/pkg: one wasm, loaded from disk in Node and fetched in browsers.
# Needs the wasm32-unknown-unknown target and wasm-bindgen-cli at the version in Cargo.lock.
set -e
cd "$(dirname "$0")/.."
cargo build -p voltik-wasm --release --target wasm32-unknown-unknown
wasm=target/wasm32-unknown-unknown/release/voltik_wasm.wasm
out=npm/pkg
tmp=$(mktemp -d)
rm -rf "$out"
mkdir -p "$out/web" "$out/node"
wasm-bindgen --target web --no-typescript --out-dir "$tmp/web" "$wasm"
wasm-bindgen --target nodejs --no-typescript --out-dir "$tmp/node" "$wasm"
cp "$tmp/web/voltik_wasm_bg.wasm" "$out/voltik.wasm"
# both glue files point at the single wasm at the package root
sed "s|new URL('voltik_wasm_bg.wasm', import.meta.url)|new URL('../voltik.wasm', import.meta.url)|" "$tmp/web/voltik_wasm.js" > "$out/web/voltik_wasm.js"
sed 's|`${__dirname}/voltik_wasm_bg.wasm`|`${__dirname}/../voltik.wasm`|' "$tmp/node/voltik_wasm.js" > "$out/node/voltik_wasm.cjs"
grep -q "'../voltik.wasm'" "$out/web/voltik_wasm.js" && grep -q '/../voltik.wasm' "$out/node/voltik_wasm.cjs" || { echo "wasm-bindgen output changed: fix the paths in npm/build.sh" >&2; exit 1; }
cp npm/src/web.js "$out/web/index.js"
cp npm/src/wrap.js "$out/web/wrap.js"
cp npm/src/node.cjs "$out/node/index.cjs"
cp npm/src/node.mjs "$out/node/index.mjs"
sed -e 's/^export function wrap/function wrap/' npm/src/wrap.js > "$out/node/wrap.cjs"
echo 'module.exports = { wrap };' >> "$out/node/wrap.cjs"
cp npm/src/index.d.ts "$out/index.d.ts"
cp npm/src/index.d.ts "$out/index.d.cts"
cp npm/README.md LICENSE-MIT LICENSE-APACHE crates/python/licenses/OFL-STIXTwoMath.txt crates/python/licenses/GUST-FONT-LICENSE-LatinModernMath.txt "$out/"
cp npm/package.json "$out/package.json"
rm -rf "$tmp"
echo "npm/pkg: $(node -p "require('./npm/package.json').name + '@' + require('./npm/package.json').version")"
