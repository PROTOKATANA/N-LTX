const { test } = require("node:test");
const assert = require("node:assert/strict");
const voltik = require("@mathlabs/voltik");

test("require works too", () => {
  assert.match(voltik.circuitikzToSvg(String.raw`\begin{circuitikz}\draw (0,0) to[C] (2,0);\end{circuitikz}`), /^<svg/);
  assert.deepEqual(voltik.render(String.raw`\begin{circuitikz}\draw (0,0) to[R] (2,0);\end{circuitikz}`).warnings, []);
});
