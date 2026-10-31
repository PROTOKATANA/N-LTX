// The JavaScript API over the raw wasm-bindgen exports, shared by the Node and the browser builds.

export function wrap(raw, isReady) {
  const check = () => {
    if (!isReady()) throw new Error("voltik: call `await init()` once before converting (only needed outside Node)");
  };
  return {
    circuitikzToSvg(code, options) {
      check();
      return raw.circuitikzToSvg(code, options?.locale ?? null, options?.animate ?? null, options?.steps ?? null);
    },
    render(code, options) {
      check();
      return JSON.parse(raw.render(code, options?.locale ?? null, options?.animate ?? null, options?.steps ?? null));
    },
    png(svg, options) {
      check();
      const o = options ?? {};
      return raw.png(svg, o.dpi ?? null, o.background ?? null);
    },
    version() {
      check();
      return raw.version();
    },
  };
}
