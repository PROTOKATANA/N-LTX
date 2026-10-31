import { test } from "node:test";
import assert from "node:assert/strict";
import { init, texToSvg, render, locales, version } from "@mathlabs/mathra";

test("works right away in Node, init is optional", async () => {
  assert.match(texToSvg("x^2"), /^<svg/);
  await init();
});

test("svg carries the speech", () => {
  assert.match(texToSvg(String.raw`\frac{1}{2}`), /1 over 2/);
});

test("options and the old locale string", () => {
  assert.match(texToSvg("3,14", { locale: "pt-BR" }), /3,14/);
  assert.equal(texToSvg("3,14", "pt-BR"), texToSvg("3,14", { locale: "pt-BR" }));
  assert.notEqual(texToSvg(String.raw`\sum_{n} x`, { display: false }), texToSvg(String.raw`\sum_{n} x`));
});

test("render returns every output as an object", () => {
  const r = render(String.raw`\vertA\vert`, { locale: "fr" });
  assert.match(r.svg, /^<svg/);
  assert.equal(r.speech, "valeur absolue de A");
  assert.equal(r.warnings[0].rule, "E-REC-1");
  assert.equal(typeof r.tree, "object");
});

test("strict refuses and errors are Error objects", () => {
  assert.throws(() => texToSvg(String.raw`\vertA\vert`, { strict: true }), Error);
  assert.throws(() => texToSvg("x", { locale: "xx" }), /unknown locale/);
});

test("locales and version", () => {
  const list = locales();
  assert.equal(list.length, 16);
  assert.deepEqual(list[0], { code: "en", name: "English" });
  assert.match(version, /^\d+\.\d+\.\d+/);
});

test("MathML and AsciiMath input", () => {
  assert.equal(render("sqrt(x)/2", { input: "asciimath" }).speech, render(String.raw`\frac{\sqrt{x}}{2}`).speech);
  assert.equal(render("<math><msup><mi>x</mi><mn>2</mn></msup></math>", { input: "mathml" }).text, "x^2");
  // MathML passed as LaTeX is recognized
  assert.equal(render("<math><mi>y</mi></math>").text, "y");
  assert.throws(() => texToSvg("x", { input: "word" }), /unknown input format/);
});

test("braille", () => {
  const r = render(String.raw`\frac{3}{4}`, { locale: "pt-BR" });
  assert.equal(r.braille, "⠼⠒⠙");
  assert.equal(r.braille_code, "cmu");
  assert.equal(render(String.raw`\frac{3}{4}`, { braille: "nemeth" }).braille, "⠹⠒⠌⠲⠼");
  assert.throws(() => render("x", { braille: "ueb" }), /unknown braille code/);
});

test("the writing animation and the steps", () => {
  assert.doesNotMatch(texToSvg("x+1"), /data-step/);
  assert.match(texToSvg("x+1", { animate: true }), /@keyframes mr-f/);
  const steps = texToSvg("x+1", { steps: true });
  assert.match(steps, /data-steps="3"/);
  assert.doesNotMatch(steps, /@keyframes/);
  assert.match(render("x", { animate: true }).svg, /mr-f/);
});
