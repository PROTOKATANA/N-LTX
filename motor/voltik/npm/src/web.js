// Browsers and bundlers (Vite, webpack, esbuild): the wasm is fetched next to this file by `init()`.
import initWasm, * as raw from "./voltik_wasm.js";
import { wrap } from "./wrap.js";

let ready = false;
let loading;

/** Loads the WebAssembly module. Call it once and await it; later calls return the same promise. */
export function init(input) {
  loading ??= initWasm(input === undefined ? undefined : { module_or_path: input }).then(() => {
    ready = true;
  });
  return loading;
}

export const { circuitikzToSvg, render, png, version } = wrap(raw, () => ready);
