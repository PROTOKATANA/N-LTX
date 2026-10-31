// The construction animation on the site: an animated SVG from voltik played in a loop, as a GIF
// plays, and turned into a real GIF or a video, frame by frame, in the browser.
var voltikAnimation = (() => {
  const HOLD = 1.6; // seconds the finished figure stays before the loop starts over
  const DRAW = 0.9; // the longest step (a stroke drawn along its length), as in voltik's CSS

  // the seconds between two steps, from voltik's CSS
  const stepTime = (svg) => Number(/calc\(var\(--i\)\*([\d.]+)s\)/.exec(svg)?.[1] ?? 0);

  // how long the figure takes to build: the last step's delay plus its own length, in seconds
  function duration(svg) {
    const count = Number(/data-steps="(\d+)"/.exec(svg)?.[1] ?? 0);
    return Math.max(0, count - 1) * stepTime(svg) + DRAW;
  }

  // plays `svg` in `box` over and over; the returned function stops it and leaves `still` there
  function loop(box, svg, still) {
    let timer = 0;
    const play = () => {
      box.innerHTML = svg;
      timer = setTimeout(play, (duration(svg) + HOLD) * 1000);
    };
    play();
    return () => {
      clearTimeout(timer);
      box.innerHTML = still;
    };
  }

  // the figure at `t` seconds: every step paused where it is at that moment
  function frameSvg(svg, t, step) {
    const pause = `<style>.tr-d,.tr-f{animation-play-state:paused!important;animation-delay:calc(var(--i)*${step}s - ${t}s)!important}</style>`;
    return svg.replace("</svg>", pause + "</svg>");
  }

  // The controls an animated SVG carries when it is downloaded: play and pause, a bar to drag and
  // the step it is at, below the figure. They run as the SVG's own script, so they show where a
  // browser opens the file or a page puts it inline; as an <img> scripts do not run, and the
  // figure builds itself once, with no bar. Written out by its source, with nothing from outside.
  function controls() {
    const NS = "http://www.w3.org/2000/svg";
    const svg = document.currentScript?.closest("svg") ?? document.documentElement;
    const animations = svg.getAnimations({ subtree: true }).filter((a) => a.effect);
    // reduced motion: voltik's CSS plays nothing, and there is nothing to control
    if (!animations.length) return;
    const end = Math.max(...animations.map((a) => a.effect.getComputedTiming().endTime));
    const starts = animations.map((a) => a.effect.getTiming().delay).sort((a, b) => a - b);
    // room for the bar under the figure, in the figure's units: 24pt, or more for a large figure
    const [x, y, w, h] = svg.getAttribute("viewBox").split(/\s+/).map(Number);
    const bar = Math.max(24, w * 0.045), pad = bar * 0.25;
    svg.setAttribute("viewBox", `${x} ${y} ${w} ${h + bar}`);
    if (svg.hasAttribute("height")) svg.setAttribute("height", `${h + bar}pt`);
    const el = (name, attrs, parent) => {
      const e = document.createElementNS(NS, name);
      for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
      parent.append(e);
      return e;
    };
    const g = el("g", { transform: `translate(${x} ${y + h})`, "font-family": "system-ui, sans-serif", "font-size": bar * 0.42 }, svg);
    el("rect", { width: w, height: bar, fill: "#f4f1ea" }, g);
    // the button: a triangle to play, two bars to pause, an arrow to start over
    const button = el("g", { cursor: "pointer", role: "button", tabindex: "0", "aria-label": "Play" }, g);
    el("rect", { width: bar, height: bar, fill: "transparent" }, button);
    const icon = el("path", { fill: "#0e1726" }, button);
    const s = bar * 0.3, c = bar / 2;
    const ICONS = {
      play: `M${c - s * 0.7} ${c - s}L${c + s} ${c}L${c - s * 0.7} ${c + s}Z`,
      pause: `M${c - s * 0.8} ${c - s}h${s * 0.55}v${2 * s}h${-s * 0.55}ZM${c + s * 0.25} ${c - s}h${s * 0.55}v${2 * s}h${-s * 0.55}Z`,
      again: `M${c} ${c - s}A${s} ${s} 0 1 1 ${c - s} ${c}h${s * 0.4}A${s * 0.6} ${s * 0.6} 0 1 0 ${c} ${c - s * 0.6}v${s * 0.45}l${-s * 0.7} ${-s * 0.65}l${s * 0.7} ${-s * 0.65}Z`,
    };
    // the bar to drag, and how many steps have started
    const label = el("text", { x: w - pad, y: bar * 0.66, "text-anchor": "end", fill: "#5d6b80" }, g);
    label.textContent = `${starts.length}/${starts.length}`;
    const left = bar + pad * 0.5, right = w - pad * 2 - label.getComputedTextLength(), track = Math.max(bar, right - left);
    const hit = el("rect", { x: left, y: 0, width: track, height: bar, fill: "transparent", cursor: "pointer" }, g);
    el("rect", { x: left, y: bar / 2 - 1.5, width: track, height: 3, rx: 1.5, fill: "#e5dfd2", "pointer-events": "none" }, g);
    const done = el("rect", { x: left, y: bar / 2 - 1.5, width: 0, height: 3, rx: 1.5, fill: "#f2b544", "pointer-events": "none" }, g);
    const knob = el("circle", { cy: bar / 2, r: bar * 0.18, fill: "#f2b544", "pointer-events": "none" }, g);

    // the clock is the script's: every animation paused, and set to the same moment
    let t = 0, playing = false, last = 0;
    animations.forEach((a) => a.pause());
    function show() {
      animations.forEach((a) => (a.currentTime = t));
      const f = end ? t / end : 1;
      done.setAttribute("width", f * track);
      knob.setAttribute("cx", left + f * track);
      label.textContent = `${starts.filter((d) => d <= t).length}/${starts.length}`;
      const state = playing ? "pause" : t >= end ? "again" : "play";
      icon.setAttribute("d", ICONS[state]);
      button.setAttribute("aria-label", { play: "Play", pause: "Pause", again: "Play again" }[state]);
    }
    function tick(now) {
      if (!playing) return;
      t = Math.min(end, t + (now - last));
      last = now;
      if (t >= end) playing = false;
      show();
      if (playing) requestAnimationFrame(tick);
    }
    function play() {
      if (t >= end) t = 0;
      playing = true;
      last = performance.now();
      requestAnimationFrame(tick);
      show();
    }
    const toggle = () => (playing ? ((playing = false), show()) : play());
    button.addEventListener("click", toggle);
    button.addEventListener("keydown", (e) => { if (e.key === " " || e.key === "Enter") { e.preventDefault(); toggle(); } });
    // dragging the bar: the figure at that moment, paused
    const seek = (e) => {
      const p = new DOMPoint(e.clientX, e.clientY).matrixTransform(g.getScreenCTM().inverse());
      t = Math.max(0, Math.min(1, (p.x - left) / track)) * end;
      show();
    };
    hit.addEventListener("pointerdown", (e) => { playing = false; hit.setPointerCapture(e.pointerId); seek(e); });
    hit.addEventListener("pointermove", (e) => { if (hit.hasPointerCapture(e.pointerId)) seek(e); });
    // it starts playing, as the plain animated SVG does
    play();
  }

  // the animated SVG with its controls, for a file to download
  const withControls = (svg) => svg.replace(/<\/svg>$/, `<script><![CDATA[(${controls})();]]></script></svg>`);

  const loadImage = (svg) => new Promise((resolve, reject) => {
    const img = new Image();
    const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
    img.onload = () => { URL.revokeObjectURL(url); resolve(img); };
    img.onerror = () => { URL.revokeObjectURL(url); reject(new Error("the SVG could not be drawn")); };
    img.src = url;
  });

  // the frame size for `width` pixels, from the SVG's viewBox
  function size(svg, width) {
    const box = /viewBox="([-\d.]+) ([-\d.]+) ([\d.]+) ([\d.]+)"/.exec(svg);
    const ratio = box ? Number(box[4]) / Number(box[3]) : 1;
    // even sides: video encoders want them
    const even = (v) => Math.max(16, 2 * Math.round(v / 2));
    return [even(width), even(width * ratio)];
  }

  // the animated SVG as GIF bytes: `width` pixels wide on white, `fps` frames a second, looping;
  // `progress(fraction)` is told as the frames are drawn
  async function gif(svg, { width = 560, fps = 15, progress = () => {} } = {}) {
    const step = stepTime(svg);
    const total = duration(svg);
    const [w, h] = size(svg, width);
    const canvas = document.createElement("canvas");
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext("2d", { willReadFrequently: true });
    const count = Math.max(2, Math.ceil(total * fps));
    const frames = [];
    for (let k = 0; k <= count; k++) {
      const img = await loadImage(frameSvg(svg, (k / count) * total, step));
      ctx.fillStyle = "#fff";
      ctx.fillRect(0, 0, w, h);
      ctx.drawImage(img, 0, 0, w, h);
      frames.push(ctx.getImageData(0, 0, w, h).data);
      progress((k + 1) / (count + 1) * 0.8);
      await new Promise((r) => setTimeout(r, 0));
    }
    const delay = Math.round(100 / fps);
    const delays = frames.map((_, k) => (k === frames.length - 1 ? Math.round(HOLD * 100) : delay));
    return encode(w, h, frames, delays, progress);
  }

  // the formats a browser records, the first it knows: WebM in Chrome and Firefox, MP4 in Safari
  const FORMATS = [["video/webm;codecs=vp9", "webm"], ["video/webm", "webm"], ["video/mp4", "mp4"]];
  const recordable = () => typeof MediaRecorder === "function" && FORMATS.some(([t]) => MediaRecorder.isTypeSupported(t));

  // the animated SVG as a video, `{ blob, ext }`: recorded in real time, so it takes as long as
  // the animation plus the hold at the end; each frame is drawn while the one before is shown
  async function video(svg, { width = 960, fps = 30, progress = () => {} } = {}) {
    const [type, ext] = FORMATS.find(([t]) => MediaRecorder.isTypeSupported(t)) ?? [];
    if (!type) throw new Error("this browser does not record video");
    const step = stepTime(svg);
    const total = duration(svg);
    const [w, h] = size(svg, width);
    const canvas = document.createElement("canvas");
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext("2d");
    const paint = (img) => { ctx.fillStyle = "#fff"; ctx.fillRect(0, 0, w, h); ctx.drawImage(img, 0, 0, w, h); };
    const count = Math.max(2, Math.ceil(total * fps));
    const frame = (k) => loadImage(frameSvg(svg, (Math.min(k, count) / count) * total, step));
    // the first frame on the canvas before the recording starts: the video opens on it
    paint(await frame(0));
    const stream = canvas.captureStream(fps);
    const recorder = new MediaRecorder(stream, { mimeType: type, videoBitsPerSecond: 6e6 });
    const chunks = [];
    recorder.ondataavailable = (e) => { if (e.data.size) chunks.push(e.data); };
    const done = new Promise((resolve) => (recorder.onstop = resolve));
    recorder.start();
    const start = performance.now();
    let next = frame(1);
    for (let k = 1; k <= count; k++) {
      const img = await next;
      next = frame(k + 1);
      // on time: wait for the frame's moment, then show it
      const wait = start + (k * 1000) / fps - performance.now();
      if (wait > 0) await new Promise((r) => setTimeout(r, wait));
      paint(img);
      progress((k / count) * (total / (total + HOLD)));
    }
    await new Promise((r) => setTimeout(r, HOLD * 1000));
    recorder.stop();
    await done;
    stream.getTracks().forEach((t) => t.stop());
    progress(1);
    return { blob: new Blob(chunks, { type: type.split(";")[0] }), ext };
  }

  // the colors of the finished figure, the most frequent first: up to 256, the palette of every frame
  function palette(pixels) {
    const counts = new Map();
    for (let i = 0; i < pixels.length; i += 4) {
      const key = ((pixels[i] >> 3) << 10) | ((pixels[i + 1] >> 3) << 5) | (pixels[i + 2] >> 3);
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
    const top = [...counts.entries()].sort((a, b) => b[1] - a[1]).slice(0, 256).map(([k]) => k);
    // 5 bits back to 8, with 31 as 255: the white of the page stays white
    const full = (v) => Math.round((v * 255) / 31);
    const colors = top.map((k) => [full((k >> 10) & 31), full((k >> 5) & 31), full(k & 31)]);
    while (colors.length < 256) colors.push([0, 0, 0]);
    return colors;
  }

  // GIF89a: one global palette, a frame per image with its delay, looping forever
  function encode(w, h, frames, delays, progress) {
    const colors = palette(frames[frames.length - 1]);
    const out = [];
    const bytes = (...b) => { for (const x of b) out.push(x & 255); };
    const word = (v) => bytes(v, v >> 8);
    const text = (s) => { for (const c of s) out.push(c.charCodeAt(0)); };
    text("GIF89a");
    word(w); word(h);
    bytes(0xf7, 0, 0);
    for (const [r, g, b] of colors) bytes(r, g, b);
    bytes(0x21, 0xff, 11); text("NETSCAPE2.0"); bytes(3, 1, 0, 0, 0);
    const nearest = new Map();
    const index = (r, g, b) => {
      const key = (r << 16) | (g << 8) | b;
      let best = nearest.get(key);
      if (best === undefined) {
        let d = Infinity;
        for (let i = 0; i < colors.length; i++) {
          const [cr, cg, cb] = colors[i];
          const e = (r - cr) ** 2 + (g - cg) ** 2 + (b - cb) ** 2;
          if (e < d) { d = e; best = i; }
        }
        nearest.set(key, best);
      }
      return best;
    };
    const indices = new Uint8Array(w * h);
    frames.forEach((px, f) => {
      for (let i = 0, j = 0; j < indices.length; i += 4, j++) indices[j] = index(px[i], px[i + 1], px[i + 2]);
      bytes(0x21, 0xf9, 4, 0); word(delays[f]); bytes(0, 0);
      bytes(0x2c); word(0); word(0); word(w); word(h); bytes(0);
      bytes(8);
      const data = lzw(indices);
      for (let i = 0; i < data.length; i += 255) {
        const block = data.subarray(i, i + 255);
        bytes(block.length);
        for (const x of block) out.push(x);
      }
      bytes(0);
      progress(0.8 + 0.2 * (f + 1) / frames.length);
    });
    bytes(0x3b);
    return new Uint8Array(out);
  }

  // GIF's LZW for 8-bit indices: codes of 9 to 12 bits, least significant bit first
  function lzw(indices) {
    const CLEAR = 256, END = 257;
    const out = [];
    let acc = 0, bits = 0, size = 9, next = 258;
    let dict = new Map();
    const emit = (code) => {
      acc |= code << bits;
      bits += size;
      while (bits >= 8) { out.push(acc & 255); acc >>>= 8; bits -= 8; }
    };
    emit(CLEAR);
    let prefix = indices[0];
    for (let i = 1; i < indices.length; i++) {
      const k = indices[i];
      const key = (prefix << 8) | k;
      const known = dict.get(key);
      if (known !== undefined) { prefix = known; continue; }
      emit(prefix);
      if (next < 4096) {
        dict.set(key, next++);
        if (next > (1 << size) && size < 12) size++;
      } else {
        emit(CLEAR);
        dict = new Map();
        next = 258;
        size = 9;
      }
      prefix = k;
    }
    emit(prefix);
    emit(END);
    if (bits > 0) out.push(acc & 255);
    return new Uint8Array(out);
  }

  return { duration, loop, gif, video, recordable, withControls };
})();
