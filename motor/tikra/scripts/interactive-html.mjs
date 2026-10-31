#!/usr/bin/env node
// A figure as one HTML file you change by dragging its named points (site/interactive-page.js, the
// same page the site's "Interactive HTML" button writes). Needs site/pkg (site/build.sh).
//   node scripts/interactive-html.mjs figure.tex -o figure.html
import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";

const args = process.argv.slice(2);
const o = args.indexOf("-o");
const out = o >= 0 ? args.splice(o, 2)[1] : null;
const [input] = args;
if (!input) {
  console.error("usage: interactive-html.mjs figure.tex [-o figure.html]");
  process.exit(2);
}
const site = path.join(path.dirname(new URL(import.meta.url).pathname), "..", "site");
const read = (f) => fs.readFileSync(path.join(site, f), "utf8");
const page = vm.runInNewContext(read("interactive-page.js") + ";tikraInteractivePage");
const html = page({
  code: fs.readFileSync(input, "utf8"),
  title: path.basename(input, ".tex"),
  glue: read("pkg/tikra_wasm.js"),
  wasm: read("pkg/tikra_wasm_bytes.js"),
  params: read("params.js"),
  interactive: read("interactive.js"),
});
if (out) fs.writeFileSync(out, html);
else process.stdout.write(html);
