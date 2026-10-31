// LaTeX (and the reference's MathML rows) to MathML with MathJax, so mathra and MathCAT read the same input.
import fs from "node:fs";
import { mathjax } from "mathjax-full/js/mathjax.js";
import { TeX } from "mathjax-full/js/input/tex.js";
import { liteAdaptor } from "mathjax-full/js/adaptors/liteAdaptor.js";
import { RegisterHTMLHandler } from "mathjax-full/js/handlers/html.js";
import { AllPackages } from "mathjax-full/js/input/tex/AllPackages.js";
import { SerializedMmlVisitor } from "mathjax-full/js/core/MmlTree/SerializedMmlVisitor.js";
import { STATE } from "mathjax-full/js/core/MathItem.js";

const adaptor = liteAdaptor();
RegisterHTMLHandler(adaptor);
const tex = new TeX({ packages: AllPackages.filter((p) => p !== "bussproofs") });
const doc = mathjax.document("", { InputJax: tex });
const visitor = new SerializedMmlVisitor();
const toMml = (src) => visitor.visitTree(doc.convert(src, { display: true, end: STATE.CONVERT }));

const ref = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const out = [];
for (const [group, sections] of Object.entries(ref)) {
  for (const s of sections) {
    s.rows.forEach((r, i) => {
      const id = `${s.id}#${i + 1}`;
      if (r.mml) { out.push({ id, group, tex: r.tex, mml: r.mml }); return; }
      try {
        const mml = toMml(r.tex);
        if (/<merror/.test(mml)) out.push({ id, group, tex: r.tex, skip: "MathJax error" });
        else out.push({ id, group, tex: r.tex, mml });
      } catch (e) {
        out.push({ id, group, tex: r.tex, skip: String(e.message ?? e) });
      }
    });
  }
}
fs.writeFileSync(process.argv[3], JSON.stringify(out, null, 1));
console.log(out.length, "rows,", out.filter((o) => o.skip).length, "skipped");
