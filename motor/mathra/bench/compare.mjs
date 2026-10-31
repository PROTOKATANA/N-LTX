// mathra next to KaTeX and MathJax: how many equations each converts cleanly, how long a conversion
// takes and how big the output is. `npm install && npm run compare`; the numbers go to the site's
// SNAPSHOT in site/index.html by hand.
//
// "Clean" means no error and no command drawn in red: mathra without warnings, MathJax without
// `merror` (all TeX packages except `noundefined`, which would draw unknown commands in red instead),
// KaTeX without throwing (with mhchem).

import { createRequire } from "module";
import fs from "fs";
import zlib from "zlib";

const require = createRequire(import.meta.url);
const katex = require("katex");
require("katex/contrib/mhchem");
const mathra = require("./pkg/mathra_wasm.js");
const { mathjax } = require("mathjax-full/js/mathjax.js");
const { TeX } = require("mathjax-full/js/input/tex.js");
const { SVG } = require("mathjax-full/js/output/svg.js");
const { liteAdaptor } = require("mathjax-full/js/adaptors/liteAdaptor.js");
const { RegisterHTMLHandler } = require("mathjax-full/js/handlers/html.js");
const { AllPackages } = require("mathjax-full/js/input/tex/AllPackages.js");

const adaptor = liteAdaptor();
RegisterHTMLHandler(adaptor);
const packages = AllPackages.filter((p) => p !== "noundefined");
const mj = mathjax.document("", { InputJax: new TeX({ packages }), OutputJax: new SVG({ fontCache: "local" }) });

/// Conversions of each equation timed per sample, after one warm-up pass over the whole set.
const REPEAT = 50;

const json = (path) => JSON.parse(fs.readFileSync(new URL(path, import.meta.url), "utf8"));
const sets = {
  "gallery (18)": json("./famous.json"),
  "corpus (116)": [...json("../corpus/constructions.json"), ...json("../corpus/packages.json")],
};

const engines = {
  mathra: (tex) => {
    const r = JSON.parse(mathra.render(tex, "en", true, false));
    return { out: r.svg, clean: r.warnings.length === 0 };
  },
  "MathJax (SVG)": (tex) => {
    const out = adaptor.outerHTML(mj.convert(tex, { display: true }));
    return { out, clean: !/merror|data-mjx-error/.test(out) };
  },
  "KaTeX (HTML)": (tex) => {
    try {
      return { out: katex.renderToString(tex, { displayMode: true, throwOnError: true }), clean: true };
    } catch {
      return { out: katex.renderToString(tex, { displayMode: true, throwOnError: false }), clean: false };
    }
  },
};

// KaTeX and MathJax print warnings about LaTeX-incompatible input; the table is what matters here
console.warn = () => {};

for (const [set, list] of Object.entries(sets)) {
  console.log(`\n### ${set}\n`);
  console.log("| Library | Clean | Mean | p95 | Output (gzip) |");
  console.log("|---|---|---|---|---|");
  for (const [name, convert] of Object.entries(engines)) {
    const run = (tex) => { try { return convert(tex); } catch { return { out: "", clean: false }; } };
    list.forEach(([, tex]) => run(tex));
    let clean = 0, gzip = 0;
    const times = [];
    for (const [, tex] of list) {
      const r = run(tex);
      if (r.clean) clean++;
      gzip += zlib.gzipSync(r.out).length;
      const start = process.hrtime.bigint();
      for (let i = 0; i < REPEAT; i++) run(tex);
      times.push(Number(process.hrtime.bigint() - start) / 1e3 / REPEAT);
    }
    times.sort((a, b) => a - b);
    const mean = times.reduce((a, b) => a + b, 0) / times.length;
    const p95 = times[Math.min(times.length, Math.ceil(times.length * 0.95)) - 1];
    console.log(`| ${name} | ${clean}/${list.length} | ${mean.toFixed(1)} µs | ${p95.toFixed(1)} µs | ${(gzip / list.length / 1024).toFixed(2)} KB |`);
  }
}
