// The figure as one HTML page that stands alone, its points dragged (interactive.js): tikra's
// WebAssembly goes inside it, so it opens from disk, with no network. The site's "Interactive HTML"
// button and scripts/interactive-html.mjs write it from the same parts: the wasm-bindgen script,
// the wasm as base64 (`var TIKRA_WASM_BASE64 = …`), params.js and interactive.js.
var tikraInteractivePage = ({ code, title, glue, wasm, params, interactive }) => {
  const escape = (s) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  // a script's text can hold anything but its own closing tag
  const inline = (js) => `<script>${js.replace(/<\/script/gi, "<\\/script")}</script>`;
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escape(title)}</title>
<style>
  :root { --fg: #1d1d1f; --muted: #86868b; --accent: #d9480f; --chip: #f1f1f3; --line: #d2d2d7; }
  body { margin: 0; min-height: 100vh; display: grid; place-items: center; background: #fff; color: var(--fg); font: 14px system-ui, sans-serif; }
  #box { position: relative; width: min(92vw, 900px); padding: 16px; box-sizing: border-box; }
  .ix-stage { position: relative; padding: 12px; border: 1px solid var(--line); border-radius: 12px; line-height: 0; }
  .ix-figure svg { width: 100%; height: auto; max-height: 78vh; display: block; overflow: visible; touch-action: none; user-select: none; }
  .ix-chips { display: flex; flex-wrap: wrap; justify-content: center; gap: 6px; padding-top: 10px; line-height: 1; }
  .ix-chips:empty { display: none; }
  .ix-chip { font: 12px/1 ui-monospace, monospace; border: 1px solid var(--line); border-radius: 999px; padding: 6px 10px; background: var(--chip); color: var(--fg); cursor: ew-resize; touch-action: none; }
  .ix-chip.ix-active, .ix-chip:hover { border-color: var(--accent); background: #fff4e5; }
  .ix-play { position: absolute; top: 8px; right: 8px; width: 30px; height: 30px; border-radius: 50%; border: 1px solid var(--line); background: #fff; color: var(--fg); font: 12px/1 system-ui, sans-serif; cursor: pointer; }
  .ix-play:hover { border-color: var(--accent); }
  .ix-play[hidden] { display: none; }
  .ix-handle { cursor: grab; outline: none; }
  .ix-halo { fill: var(--accent); fill-opacity: 0; transition: fill-opacity .15s; }
  .ix-handle:hover .ix-halo, .ix-handle:focus-visible .ix-halo, .ix-active .ix-halo { fill-opacity: .2; }
  .ix-ring { fill: #fff; stroke: var(--accent); stroke-width: 2; vector-effect: non-scaling-stroke; }
  .ix-cursor { fill: var(--fg); stroke: #fff; stroke-width: 1.2; vector-effect: non-scaling-stroke; pointer-events: none; }
  .ix-active { cursor: grabbing; }
  .ix-readout { position: absolute; z-index: 2; pointer-events: none; transform: translate(-50%, -170%); background: var(--fg); color: #fff; font: 12px ui-monospace, monospace; padding: 3px 8px; border-radius: 6px; white-space: pre; }
  .hint { position: fixed; bottom: 12px; left: 0; right: 0; text-align: center; color: var(--muted); font-size: 12px; }
</style>
</head>
<body>
<div id="box"></div>
<div class="hint">Drag the marked points (or Tab and the arrow keys), and the chips under the figure sideways. ▶ shows it done.</div>
${inline(glue)}
${inline(wasm)}
${inline(params)}
${inline(interactive)}
${inline(`
  // async: browsers refuse to compile a module this size on the page's own thread
  wasm_bindgen({ module_or_path: Uint8Array.from(atob(TIKRA_WASM_BASE64), (c) => c.charCodeAt(0)) }).then(() => {
    const render = (code) => JSON.parse(wasm_bindgen.render(code, "en", false, false));
    // it shows itself being dragged until a hand is on it
    tikraInteractive.mount(document.getElementById("box"), { code: ${JSON.stringify(code)}, render }).play();
  });
`)}
</body>
</html>
`;
};
