import { test } from "node:test";
import assert from "node:assert/strict";
import { init, tikzToSvg, render, png, version } from "@mathlabs/tikra";

const FIGURE = String.raw`\begin{tikzpicture}\draw[->] (0,0) -- (1,1) node[right] {$x$};\end{tikzpicture}`;

test("works right away in Node, init is optional", async () => {
  assert.match(tikzToSvg(FIGURE), /^<svg/);
  await init();
});

test("every input form", () => {
  for (const code of [
    String.raw`\documentclass[tikz]{standalone}\begin{document}\begin{tikzpicture}\draw (0,0)--(1,0);\end{tikzpicture}\end{document}`,
    String.raw`\tikz \draw (0,0) circle (1);`,
    String.raw`\begin{axis}\addplot {x^2};\end{axis}`,
    String.raw`\draw (0,0) -- (1,1);`,
  ]) assert.match(tikzToSvg(code), /^<svg/);
});

test("render returns the svg and the warnings", () => {
  assert.deepEqual(render(FIGURE).warnings, []);
  const r = render(String.raw`\begin{tikzpicture}\pic {x}; \draw (0,0) -- (1,0);\end{tikzpicture}`);
  assert.match(r.svg, /^<svg/);
  assert.ok(r.warnings.length > 0);
  assert.equal(typeof r.warnings[0], "string");
});

test("render describes the figure in words", () => {
  const r = render(FIGURE);
  assert.equal(r.alt, "Drawing of one shape with the text “x”");
  assert.equal(r.description, "Text in the figure: x.");
  assert.equal(render(FIGURE, { locale: "pt-BR" }).description, "Texto na figura: x.");
  assert.match(tikzToSvg(FIGURE, { locale: "de" }), /Text in der Abbildung: x\./);
  assert.throws(() => render(FIGURE, { locale: "xx" }), /unknown locale/);
});

test("errors are Error objects", () => {
  assert.throws(() => tikzToSvg("just text"), /no tikzpicture/);
  assert.throws(() => png("not svg"), Error);
  assert.throws(() => png(tikzToSvg(FIGURE), { dpi: 0 }), /invalid dpi/);
});

test("png", () => {
  const svg = tikzToSvg(FIGURE);
  const opaque = png(svg);
  assert.ok(opaque instanceof Uint8Array);
  assert.deepEqual([...opaque.slice(0, 4)], [0x89, 0x50, 0x4e, 0x47]);
  assert.notDeepEqual(png(svg, { background: false }), opaque);
  assert.ok(png(svg, { dpi: 72 }).length < opaque.length);
});

test("version", () => {
  assert.match(version(), /^\d+\.\d+\.\d+/);
});
