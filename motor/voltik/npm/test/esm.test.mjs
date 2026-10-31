import { test } from "node:test";
import assert from "node:assert/strict";
import { init, circuitikzToSvg, render, png, version } from "@mathlabs/voltik";

const FIGURE = String.raw`\begin{circuitikz}\draw (0,0) to[R=$R$] (2,0);\end{circuitikz}`;

test("works right away in Node, init is optional", async () => {
  assert.match(circuitikzToSvg(FIGURE), /^<svg/);
  await init();
});

test("every input form", () => {
  for (const code of [
    String.raw`\documentclass{standalone}\usepackage{circuitikz}\begin{document}\begin{circuitikz}\draw (0,0) to[R] (2,0);\end{circuitikz}\end{document}`,
    String.raw`\begin{circuitikz}\draw (0,0) to[C] (2,0);\end{circuitikz}`,
    String.raw`\begin{tikzpicture}\draw (0,0) to[L] (2,0);\end{tikzpicture}`,
    String.raw`\draw (0,0) to[R, o-*] (2,0);`,
  ]) assert.match(circuitikzToSvg(code), /^<svg/);
});

test("render returns the svg and the warnings", () => {
  assert.deepEqual(render(FIGURE).warnings, []);
  const r = render(String.raw`\begin{circuitikz}\draw (0,0) to[R, no such option] (2,0);\end{circuitikz}`);
  assert.match(r.svg, /^<svg/);
  assert.ok(r.warnings.length > 0);
  assert.equal(typeof r.warnings[0], "string");
});

test("render describes the figure in words", () => {
  const r = render(FIGURE);
  assert.equal(r.alt, "Drawing of 2 shapes with the text “R”");
  assert.equal(r.description, "Text in the figure: R.");
  assert.equal(render(FIGURE, { locale: "pt-BR" }).description, "Texto na figura: R.");
  assert.match(circuitikzToSvg(FIGURE, { locale: "de" }), /Text in der Abbildung: R\./);
  assert.throws(() => render(FIGURE, { locale: "xx" }), /unknown locale/);
});

test("errors are Error objects", () => {
  assert.throws(() => circuitikzToSvg("just text"), /no tikzpicture/);
  assert.throws(() => png("not svg"), Error);
  assert.throws(() => png(circuitikzToSvg(FIGURE), { dpi: 0 }), /invalid dpi/);
});

test("png", () => {
  const svg = circuitikzToSvg(FIGURE);
  const opaque = png(svg);
  assert.ok(opaque instanceof Uint8Array);
  assert.deepEqual([...opaque.slice(0, 4)], [0x89, 0x50, 0x4e, 0x47]);
  assert.notDeepEqual(png(svg, { background: false }), opaque);
  assert.ok(png(svg, { dpi: 72 }).length < opaque.length);
});

test("version", () => {
  assert.match(version(), /^\d+\.\d+\.\d+/);
});
