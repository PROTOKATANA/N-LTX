// A slider for each number a figure's code defines: \def\r{0.7}, \pgfmathsetmacro{\r}{0.7},
// \newcommand{\r}{0.7}. Moving one writes the number in the code, and the page draws it again.
// A comment on the same line can give the slider's range: \def\n{6} % sides, 3..12
var voltikParams = (() => {
  const PARAM = /\\(?:def\s*(\\[a-zA-Z@]+)|pgfmathsetmacro\s*\{?\s*(\\[a-zA-Z@]+)\s*\}?|(?:re)?newcommand\s*\{?\s*(\\[a-zA-Z@]+)\s*\}?)\s*\{\s*(-?\d*\.?\d+)\s*\}/g;
  const HINT = /^[^\n%]*%[^\n]*?(-?\d*\.?\d+)\s*\.\.\s*(-?\d*\.?\d+)/;
  // the code with its comments blanked out, the same length: a \def in a comment is no number
  const strip = (src) => src.replace(/(^|[^\\])%[^\n]*/g, (m, c) => c + " ".repeat(m.length - c.length));

  // the numbers, once per name: where the number is in the code, and the range its comment gives
  function find(code) {
    const out = [];
    for (const m of strip(code).matchAll(PARAM)) {
      const name = m[1] ?? m[2] ?? m[3];
      if (out.some((p) => p.name === name)) continue;
      const at = m.index + m[0].lastIndexOf(m[4]);
      const hint = HINT.exec(code.slice(m.index + m[0].length));
      const lo = hint && Number(hint[1]), hi = hint && Number(hint[2]);
      out.push({ name, value: Number(m[4]), at, length: m[4].length, text: m[4], hint: hint && lo < hi ? [lo, hi] : null });
    }
    return out;
  }

  // the range: the one the comment gives, or from 1 (0 for a fraction) to twice the number, the
  // other way for a negative one; whole steps for whole numbers over a range of 6 or more
  function initial(p) {
    const v = p.value, whole = Number.isInteger(v);
    let [lo, hi] = p.hint ?? (v === 0 ? [-1, 1] : v > 0 ? [whole && v >= 3 ? 1 : 0, 2 * v] : [2 * v, 0]);
    const step = whole && Number.isInteger(lo) && Number.isInteger(hi) && hi - lo >= 6 ? 1 : (hi - lo) / 200;
    return { lo, hi, step };
  }
  // the number as the slider gives it, with no more decimals than its step has
  const decimals = (step) => (step >= 1 ? 0 : Math.min(4, Math.ceil(-Math.log10(step))));

  // the sliders of `code()` in `box`; `change(code)` is told the code with the new number. Returns
  // the function to call after the code changed elsewhere (typed in an editor).
  function mount(box, { code, change }) {
    const ranges = new Map();
    let key = "";
    const range = (p) => {
      if (!ranges.has(p.name)) ranges.set(p.name, initial(p));
      const r = ranges.get(p.name);
      r.lo = Math.min(r.lo, p.value);
      r.hi = Math.max(r.hi, p.value);
      return r;
    };
    function refresh() {
      const params = find(code());
      box.hidden = !params.length;
      // the same names: only the values move (rebuilding would drop the slider being dragged)
      if (params.map((p) => p.name).join(" ") === key) {
        for (const p of params) {
          const row = [...box.children].find((r) => r.dataset.name === p.name);
          if (!row || document.activeElement === row.querySelector("input")) continue;
          const r = range(p);
          Object.assign(row.querySelector("input"), { min: r.lo, max: r.hi, step: r.step, value: p.value });
          row.querySelector("output").textContent = p.text;
        }
        return;
      }
      key = params.map((p) => p.name).join(" ");
      box.replaceChildren();
      for (const p of params) {
        const r = range(p);
        const row = document.createElement("label");
        row.className = "param";
        row.dataset.name = p.name;
        row.append(Object.assign(document.createElement("span"), { textContent: p.name }));
        const input = Object.assign(document.createElement("input"), { type: "range", min: r.lo, max: r.hi, step: r.step, value: p.value });
        const output = document.createElement("output");
        output.textContent = p.text;
        row.append(input, output);
        input.oninput = () => {
          const src = code();
          const now = find(src).find((q) => q.name === p.name);
          if (!now) return;
          const text = String(Number(Number(input.value).toFixed(decimals(r.step))));
          output.textContent = text;
          change(src.slice(0, now.at) + text + src.slice(now.at + now.length));
        };
        box.append(row);
      }
    }
    refresh();
    return refresh;
  }

  return { find, mount };
})();
