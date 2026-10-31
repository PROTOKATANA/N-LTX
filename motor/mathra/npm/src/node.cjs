// Node: the wasm is read from disk when the module loads, so every function works right away.
const raw = require("./mathra_wasm.cjs");
const { wrap } = require("./wrap.cjs");

module.exports = { init: async () => {}, ...wrap(raw, () => true), version: "__VERSION__" };
