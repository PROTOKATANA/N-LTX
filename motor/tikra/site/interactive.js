// A figure you change by dragging the figure itself. The numbers its code defines (\def\r{0.7}, as
// the sliders of params.js find them) are the knobs; the points the author named (\coordinate (tip)
// at (\a:\r);) are the handles. Nobody says which number moves which point: the page draws the
// figure again with each number nudged and sees where the points went. Dragging a handle then
// changes up to two numbers at once (an angle and a length, say) so the point follows the pointer,
// one Newton step at a time. A number no handle moves gets a chip to drag sideways.
// Needs `render(code)` → {svg, points: [{name, x, y}], origin: [x, y]}, and tikraParams.
var tikraInteractive = (() => {
  const NS = "http://www.w3.org/2000/svg";

  const decimals = (step) => (step >= 1 ? 0 : Math.min(4, Math.ceil(-Math.log10(step))));
  const format = (v, step) => String(Number(v.toFixed(decimals(step))));
  const clamp = (v, r) => Math.max(r.lo, Math.min(r.hi, v));

  // `code` with the numbers of `values` (by name) written in, the last first so places hold
  function withValues(code, values) {
    let out = code;
    for (const p of tikraParams.find(code).reverse()) {
      if (p.name in values) out = out.slice(0, p.at) + values[p.name] + out.slice(p.at + p.length);
    }
    return out;
  }

  // the solution of the small system A·x = b (Gauss with pivots); null when it has none
  function solve(A, b) {
    const n = b.length, M = A.map((row, i) => [...row, b[i]]);
    for (let c = 0; c < n; c++) {
      let pivot = c;
      for (let r = c + 1; r < n; r++) if (Math.abs(M[r][c]) > Math.abs(M[pivot][c])) pivot = r;
      if (Math.abs(M[pivot][c]) < 1e-12) return null;
      [M[c], M[pivot]] = [M[pivot], M[c]];
      for (let r = 0; r < n; r++) {
        if (r === c) continue;
        const f = M[r][c] / M[c][c];
        for (let k = c; k <= n; k++) M[r][k] -= f * M[c][k];
      }
    }
    return M.map((row, i) => row[n] / row[i]);
  }

  // a place on the screen in the SVG's coordinates: from its content box (inside any CSS padding)
  // and its viewBox, fitted and centered as `preserveAspectRatio` does by default. getScreenCTM()
  // is not trusted with padding around an outer <svg>.
  function toSvg(svg, clientX, clientY) {
    const r = svg.getBoundingClientRect(), cs = getComputedStyle(svg);
    const px = (k) => parseFloat(cs[k]) || 0;
    const left = r.left + px("paddingLeft") + px("borderLeftWidth"), top = r.top + px("paddingTop") + px("borderTopWidth");
    const w = r.width - px("paddingLeft") - px("paddingRight") - px("borderLeftWidth") - px("borderRightWidth");
    const h = r.height - px("paddingTop") - px("paddingBottom") - px("borderTopWidth") - px("borderBottomWidth");
    const vb = svg.viewBox.baseVal;
    const s = Math.min(w / vb.width, h / vb.height);
    return [vb.x + (clientX - left - (w - vb.width * s) / 2) / s, vb.y + (clientY - top - (h - vb.height * s) / 2) / s];
  }

  // the code with its numbers taken out: two codes alike but for them are the same figure
  const skeleton = (code) => withValues(code, Object.fromEntries(tikraParams.find(code).map((p) => [p.name, ""])));

  // the figure of `source` in `box`, its handles on it; `change(code)` is told the code with the
  // new numbers when a drag ends. Returns {code, update(code), show(), destroy()}: `update` takes
  // the same figure with other numbers (false when it is another figure, to mount anew), `show`
  // draws it again after something else filled `box`.
  function mount(box, { code: initial, render, change = () => {} }) {
    let source = initial;
    const params = tikraParams.find(source);
    const ranges = Object.fromEntries(params.map((p) => [p.name, tikraParams.initial(p)]));
    const values = Object.fromEntries(params.map((p) => [p.name, p.value]));
    // the code with the numbers as they are: at their steps, or, while a handle is held, finer, so
    // the point keeps up with the pointer instead of jumping from step to step
    const code = (fine = false) => withValues(source, Object.fromEntries(params.map((p) => [p.name, format(values[p.name], fine ? 1e-4 : ranges[p.name].step)])));
    // a drawing with `vals`, its points and box measured from the picture's origin: those stay
    // comparable from one drawing to the next, where the SVG's own coordinates slide
    const measure = (vals) => {
      const r = render(withValues(source, vals));
      const [ox, oy] = r.origin;
      const vb = /viewBox="([-\d.e]+) ([-\d.e]+) ([-\d.e]+) ([-\d.e]+)"/.exec(r.svg).slice(1).map(Number);
      return {
        points: Object.fromEntries(r.points.map((p) => [p.name, [p.x - ox, p.y - oy]])),
        box: [vb[0] - ox, vb[1] - oy, vb[0] + vb[2] - ox, vb[1] + vb[3] - oy],
      };
    };

    // how far each point moves per unit of each number, here: {point: {number: [dx, dy]}}
    function jacobian(base = measure(values).points) {
      const J = {};
      for (const p of params) {
        const r = ranges[p.name];
        let h = (r.hi - r.lo) / 200;
        if (values[p.name] + h > r.hi) h = -h;
        const moved = measure({ ...values, [p.name]: values[p.name] + h }).points;
        for (const [name, [x, y]] of Object.entries(base)) {
          if (moved[name]) (J[name] ??= {})[p.name] = [(moved[name][0] - x) / h, (moved[name][1] - y) / h];
        }
      }
      return J;
    }
    // how far a number takes a point over its whole range: what says the number moves it at all
    const reach = (J, point, name) => Math.hypot(...J[point][name]) * (ranges[name].hi - ranges[name].lo);
    // the numbers a handle moves: the one that moves it most, and the one that moves it most
    // across that direction, so the two together cover a plane; the others stay as they are
    function knobs(J, point) {
      const live = params.map((p) => p.name).filter((n) => J[point]?.[n] && reach(J, point, n) > 1);
      if (live.length < 2) return live;
      const first = live.reduce((a, b) => (reach(J, point, b) > reach(J, point, a) ? b : a));
      const [ux, uy] = J[point][first].map((v) => v / Math.hypot(...J[point][first]));
      const across = (n) => Math.abs(J[point][n][0] * uy - J[point][n][1] * ux) * (ranges[n].hi - ranges[n].lo);
      const second = live.filter((n) => n !== first).reduce((a, b) => (across(b) > across(a) ? b : a));
      return across(second) > 1 ? [first, second] : [first];
    }

    // the frame: every box the figure takes at the corners of its numbers' ranges, so it does not
    // jump about while it changes (up to 3 numbers: 8 drawings)
    let frame = null;
    if (params.length && params.length <= 3) {
      const corners = params.reduce((all, p) => all.flatMap((c) => [{ ...c, [p.name]: ranges[p.name].lo }, { ...c, [p.name]: ranges[p.name].hi }]), [{}]);
      for (const c of [{}, ...corners]) {
        try {
          const b = measure({ ...values, ...c }).box;
          frame = frame ? [Math.min(frame[0], b[0]), Math.min(frame[1], b[1]), Math.max(frame[2], b[2]), Math.max(frame[3], b[3])] : b;
        } catch { /* a corner the figure cannot be drawn at */ }
      }
    }

    // the figure on a card of its own, everything to touch on it: the drawing with its handles,
    // under it the chips of the numbers no handle holds, in its corner the demo's button
    let figure, chips, readout, playButton;
    function build() {
      box.innerHTML = `<div class="ix-stage"><div class="ix-figure"></div><div class="ix-chips"></div>`
        + `<button type="button" class="ix-play" title="Watch it dragged" aria-label="Watch it dragged">▶</button></div><div class="ix-readout" hidden></div>`;
      [figure, chips, readout, playButton] = [".ix-figure", ".ix-chips", ".ix-readout", ".ix-play"].map((c) => box.querySelector(c));
      playButton.onclick = () => (demo ? stop() : play());
      playButton.hidden = !params.length;
      showPlaying();
    }
    function showPlaying() {
      if (!playButton) return;
      playButton.textContent = demo ? "■" : "▶";
      playButton.title = playButton.ariaLabel = demo ? "Stop" : "Watch it dragged";
    }
    let current = null; // the drawing on show: its SVG, and its points in the SVG's coordinates
    let J = {};
    let drag = null; // {point, knobs} while a handle is held, {chip, start, x} while a chip is
    let demo = null; // while the figure shows itself being dragged: see play()
    let focused = null;

    // the numbers as drawn: at their steps, finer while they move (a drag, the demo)
    const tell = (names) => names.map((n) => `${n} = ${format(values[n], drag || demo ? Math.min(ranges[n].step, 0.01) : ranges[n].step)}`).join("   ");
    function showReadout(names, x, y) {
      readout.hidden = !names.length;
      readout.textContent = tell(names);
      const b = box.getBoundingClientRect();
      readout.style.left = `${x - b.left}px`;
      readout.style.top = `${y - b.top}px`;
    }

    function draw() {
      let r;
      try { r = render(code(!!drag || !!demo)); } catch { return; }
      figure.innerHTML = r.svg;
      const svg = figure.querySelector("svg");
      const [ox, oy] = r.origin;
      if (frame) {
        const [w, h] = [frame[2] - frame[0], frame[3] - frame[1]];
        svg.setAttribute("viewBox", `${frame[0] + ox} ${frame[1] + oy} ${w} ${h}`);
        svg.setAttribute("width", `${w}pt`);
        svg.setAttribute("height", `${h}pt`);
      }
      current = { svg, origin: r.origin, points: Object.fromEntries(r.points.map((p) => [p.name, [p.x, p.y]])) };
      const [x0] = toSvg(svg, 0, 0), [x1] = toSvg(svg, 1, 0);
      const unit = x1 - x0 || 1; // SVG units per pixel
      const held = new Set();
      for (const [name, [x, y]] of Object.entries(current.points)) {
        const ks = drag?.point === name ? drag.knobs : knobs(J, name);
        if (!ks.length) continue;
        ks.forEach((k) => held.add(k));
        const shown = demo?.track.point === name;
        const g = document.createElementNS(NS, "g");
        g.setAttribute("class", "ix-handle" + (drag?.point === name || shown ? " ix-active" : ""));
        g.setAttribute("tabindex", "0");
        g.setAttribute("role", "slider");
        g.setAttribute("aria-label", `${name}: drag, or use the arrow keys`);
        g.setAttribute("aria-valuetext", tell(ks));
        for (const [cls, radius] of [["ix-halo", 14], ["ix-ring", 5.5]]) {
          const c = document.createElementNS(NS, "circle");
          c.setAttribute("class", cls);
          c.setAttribute("cx", x);
          c.setAttribute("cy", y);
          c.setAttribute("r", radius * unit);
          g.append(c);
        }
        g.onpointerdown = (e) => {
          e.preventDefault();
          J = jacobian(); // the numbers this drag moves are fixed until it ends
          drag = { point: name, knobs: knobs(J, name) };
          focused = name;
          showReadout(drag.knobs, e.clientX, e.clientY);
          draw();
        };
        g.onpointerenter = (e) => { if (!drag) showReadout(ks, e.clientX, e.clientY); };
        g.onpointerleave = () => { if (!drag) readout.hidden = true; };
        g.onkeydown = (e) => keyed(e, name);
        if (shown) {
          // the pointer that drags it, drawn at the size of a real one
          const cursor = document.createElementNS(NS, "path");
          cursor.setAttribute("class", "ix-cursor");
          cursor.setAttribute("d", "M0 0 L0 17 L4.6 12.6 L7.6 19.4 L10.4 18.2 L7.4 11.6 L13.4 11.4 Z");
          cursor.setAttribute("transform", `translate(${x + 2 * unit} ${y + 2 * unit}) scale(${unit})`);
          g.append(cursor);
        }
        svg.append(g);
        if (focused === name && !demo) g.focus({ preventScroll: true });
        if (shown) {
          const ring = g.querySelector(".ix-ring").getBoundingClientRect();
          showReadout(ks, ring.x + ring.width / 2, ring.y + ring.height / 2);
        }
      }
      // the numbers no handle holds: a chip each, dragged sideways
      chips.replaceChildren();
      for (const p of params) {
        if (held.has(p.name)) continue;
        const chip = Object.assign(document.createElement("button"), { type: "button", className: "ix-chip" + (demo?.track.chip === p.name ? " ix-active" : ""), textContent: tell([p.name]), title: "Drag sideways" });
        chip.onpointerdown = (e) => {
          e.preventDefault();
          drag = { chip: p.name, start: values[p.name], x: e.clientX };
        };
        chips.append(chip);
      }
    }

    // Newton steps toward `target` (SVG units, the drawing on show): the change of the knobs that,
    // with how they move the point here, takes it there in the least squares. The steps measure
    // drawings off the page; only where they end is shown.
    function follow(point, ks, target) {
      const goal = [target[0] - current.origin[0], target[1] - current.origin[1]];
      const t0 = performance.now();
      for (let i = 0; i < 4 && performance.now() - t0 < 12; i++) {
        const here = measure(values).points;
        const at = here[point];
        if (!at) break;
        const e = [goal[0] - at[0], goal[1] - at[1]];
        if (Math.hypot(...e) < 0.1) break;
        J = jacobian(here); // the figure is not linear in its numbers: measured where it is now
        const cols = ks.map((k) => J[point]?.[k] ?? [0, 0]);
        const A = cols.map((a) => cols.map((b) => a[0] * b[0] + a[1] * b[1]));
        A.forEach((row, k) => (row[k] += 1e-9 + 1e-3 * row[k])); // damped: no leap where a knob barely moves it
        const d = solve(A, cols.map((c) => c[0] * e[0] + c[1] * e[1]));
        if (!d) break;
        ks.forEach((k, j) => (values[k] = clamp(values[k] + d[j], ranges[k])));
      }
      draw();
    }

    // each number at its step: a drag is continuous, what it leaves is not
    function snap() {
      for (const p of params) {
        const r = ranges[p.name];
        values[p.name] = clamp(r.lo + Math.round((values[p.name] - r.lo) / r.step) * r.step, r);
      }
    }
    function release() {
      snap();
      drag = null;
      readout.hidden = true;
      J = jacobian();
      draw();
      change(code());
    }

    // the arrow keys move the point 8 units (Shift: 24)
    function keyed(e, name) {
      const s = e.shiftKey ? 24 : 8;
      const d = { ArrowLeft: [-s, 0], ArrowRight: [s, 0], ArrowUp: [0, -s], ArrowDown: [0, s] }[e.key];
      if (!d) return;
      e.preventDefault();
      focused = name;
      const ks = knobs(J, name), [x, y] = current.points[name];
      follow(name, ks, [x + d[0], y + d[1]]);
      release();
    }

    // on the window: the SVG under the pointer is drawn anew at each step. One step a frame,
    // toward where the pointer is by then: the moves in between only update the goal
    let pointer = null, frameAsked = false;
    function step() {
      frameAsked = false;
      if (!drag || !pointer) return;
      const e = pointer;
      if (drag.chip) {
        const r = ranges[drag.chip];
        values[drag.chip] = clamp(drag.start + ((e.clientX - drag.x) / 240) * (r.hi - r.lo), r);
        draw();
        return;
      }
      follow(drag.point, drag.knobs, toSvg(current.svg, e.clientX, e.clientY));
      showReadout(drag.knobs, e.clientX, e.clientY);
    }
    const moved = (e) => {
      if (!drag) return;
      pointer = { clientX: e.clientX, clientY: e.clientY };
      if (!frameAsked) {
        frameAsked = true;
        requestAnimationFrame(step);
      }
    };
    const up = () => {
      if (!drag) return;
      if (pointer) step(); // the last move, if its frame has not come yet
      pointer = null;
      release();
    };
    window.addEventListener("pointermove", moved);
    window.addEventListener("pointerup", up);

    // The demo: the figure dragged by itself, a handle at a time, a pointer on it, around a loop that
    // leaves from where the numbers are and comes back there; then the chips, swung the same way.
    // A look at what can be dragged, for whoever has not tried. Anything done on the figure ends it.
    const LOOP = 4200; // ms a handle is shown
    function swing(name, from, t, j) {
      // the second number of a handle twice as fast: the point draws a figure eight, not a line
      const r = ranges[name], amp = 0.3 * (r.hi - r.lo), mid = clamp(from, { lo: r.lo + amp, hi: r.hi - amp });
      const phase = Math.asin(Math.max(-1, Math.min(1, (from - mid) / amp)));
      const eased = t - Math.sin(2 * Math.PI * t) / (2 * Math.PI); // still at both ends: no jolt between handles
      return clamp(mid + amp * Math.sin(2 * Math.PI * (j + 1) * eased + phase), r);
    }
    function play() {
      if (demo || drag) return !!demo;
      const tracks = Object.keys(current?.points ?? {}).map((point) => ({ point, knobs: knobs(J, point) })).filter((t) => t.knobs.length);
      for (const p of params) if (!tracks.some((t) => t.knobs.includes(p.name))) tracks.push({ chip: p.name, knobs: [p.name] });
      if (!tracks.length) return false;
      demo = { tracks, from: { ...values }, t0: null, frame: 0, track: tracks[0] };
      const tick = (now) => {
        if (!demo) return;
        // timed from the first frame: a frame's time can be earlier than the call that asked for it
        demo.t0 ??= now;
        const elapsed = Math.max(0, now - demo.t0);
        const k = Math.floor(elapsed / LOOP), t = (elapsed % LOOP) / LOOP;
        demo.track = demo.tracks[k % demo.tracks.length];
        Object.assign(values, demo.from);
        demo.track.knobs.forEach((n, j) => (values[n] = swing(n, demo.from[n], t, j)));
        draw();
        if (demo.track.chip) {
          const c = chips.querySelector(".ix-active")?.getBoundingClientRect();
          if (c) showReadout([demo.track.chip], c.x + c.width / 2, c.y + c.height + 34);
        }
        demo.frame = requestAnimationFrame(tick);
      };
      demo.frame = requestAnimationFrame(tick);
      showPlaying();
      return true;
    }
    // the demo ends where it was (`keep`: a hand took over, the point stays under it, at its step)
    // or where it began
    function stop({ keep = false, redraw = true } = {}) {
      if (!demo) return;
      cancelAnimationFrame(demo.frame);
      if (keep) snap();
      else Object.assign(values, demo.from);
      demo = null;
      readout.hidden = true;
      showPlaying();
      if (redraw) draw();
      if (keep) change(code());
    }
    box.addEventListener("pointerdown", (e) => { if (!e.target.closest(".ix-play")) stop({ keep: true }); }, true);
    box.addEventListener("keydown", () => stop({ keep: true }), true);

    // the same figure with other numbers (a slider moved, a number typed): no new frame to measure
    function update(next) {
      stop({ redraw: false });
      if (drag || skeleton(next) !== skeleton(source)) return false;
      const now = tikraParams.find(next);
      if (now.map((p) => p.name).join(" ") !== params.map((p) => p.name).join(" ")) return false;
      for (const p of now) {
        const r = ranges[p.name];
        [r.lo, r.hi] = [Math.min(r.lo, p.value), Math.max(r.hi, p.value)];
        values[p.name] = p.value;
      }
      source = next;
      J = jacobian();
      if (!figure.isConnected) build();
      draw();
      return true;
    }

    build();
    J = jacobian();
    draw();
    return {
      code,
      update,
      show: () => { build(); draw(); },
      play,
      stop,
      playing: () => !!demo,
      destroy: () => {
        stop({ redraw: false });
        window.removeEventListener("pointermove", moved);
        window.removeEventListener("pointerup", up);
      },
    };
  }

  return { mount, withValues };
})();
