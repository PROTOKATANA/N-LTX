// Node: the wasm is read from disk when the module loads, so every function works right away.
const raw = require("./voltik_wasm.cjs");
const { wrap } = require("./wrap.cjs");

module.exports = { init: async () => {}, ...wrap(raw, () => true) };
