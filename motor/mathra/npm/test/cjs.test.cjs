const { test } = require("node:test");
const assert = require("node:assert/strict");
const mathra = require("@mathlabs/mathra");

test("require works too", () => {
  assert.match(mathra.texToSvg("x"), /^<svg/);
  assert.equal(mathra.render("x = 1", { locale: "es" }).speech, "x es igual a 1");
});
