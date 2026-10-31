// The JavaScript API over the raw wasm-bindgen exports, shared by the Node and the browser builds.

/** `options` may be an object or, as in earlier versions, just the locale code. */
function args(options) {
  const o = typeof options === "string" ? { locale: options } : options ?? {};
  return [o.locale ?? null, o.display ?? null, o.strict ?? null, o.input ?? null, o.braille ?? null, o.animate ?? null, o.steps ?? null];
}

export function wrap(raw, isReady) {
  const check = () => {
    if (!isReady()) throw new Error("mathra: call `await init()` once before converting (only needed outside Node)");
  };
  return {
    texToSvg(tex, options) {
      check();
      return raw.texToSvg(tex, ...args(options));
    },
    render(tex, options) {
      check();
      return JSON.parse(raw.render(tex, ...args(options)));
    },
    locales() {
      check();
      return JSON.parse(raw.locales());
    },
  };
}
