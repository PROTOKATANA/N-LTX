const { test } = require("node:test");
const assert = require("node:assert/strict");
const tikra = require("@mathlabs/tikra");

test("require works too", () => {
  assert.match(tikra.tikzToSvg(String.raw`\tikz \draw (0,0) circle (1);`), /^<svg/);
  assert.deepEqual(tikra.render(String.raw`\tikz \draw (0,0) -- (1,0);`).warnings, []);
});
