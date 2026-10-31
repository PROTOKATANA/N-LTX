"""Compares voltik's drawing of every item of the CircuiTikZ manual with TeX's, and writes the report.

    python3 compare.py <examples.jsonl> <work dir> [--docs docs/conformance.md]

Reads <work dir>/ref (TeX, from reference.py) and <work dir>/voltik (from the `conformance` example of
the core crate), and writes <work dir>/results.json, <work dir>/report.html (every figure that differs,
side by side) and the summary in Markdown (to --docs, or <work dir>/report.md).

The two drawings never agree pixel for pixel: TeX sets labels in Computer Modern and voltik in Latin
Modern Math (the same letters, without TeX's optical sizes), antialiasing differs, and so can the
bounding box by a hair. So a figure is compared by its shape, its color and its size, with the same
measures as tikra's conformance run:

- shape: after aligning the two images (the best shift of up to 6 px from each corner and from the
  center), their edges are compared: the share of voltik's edges with a TeX edge within 2 px (0.5 mm),
  and the other way round; their harmonic mean is the F1 score.
- pieces: each separate piece of TeX's drawing (a line, a word, a component) must be there in voltik's,
  and the other way round; one missing or extra piece is enough to not match.
- color: of voltik's ink near TeX's, the share with about the same color as some TeX ink 1 px around.
- ink: the share of each drawing's ink within 2 px of a mark of the other, the smaller of the two.
- size: width and height within 10% (or 4 px).

A figure *matches* with F1 ≥ 0.9, color ≥ 0.9, ink ≥ 0.95, the same size and no missing or extra
piece; it is *close* with F1 ≥ 0.75.
"""

import argparse
import hashlib
import html
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage, signal

RADIUS = 2.0
SHIFT = 6
INK = 40  # how far from white (0–255, on the darkest channel) a pixel must be to count as ink
COLOR = 90  # largest channel difference for two inks to count as the same color
BLUR = 0.7  # px: the blur both drawings get before their colors are compared
FAINT = 8  # how far from white a pixel must be to count as a mark, for the ink of the other drawing
MATCH, CLOSE, COLOR_OK, INK_OK = 0.90, 0.75, 0.90, 0.95
MIN_PIECE = 20  # pixels of outline for a piece to count
EDGE = 60  # color difference between neighbor pixels that makes an edge


def digest(tex):
    return hashlib.sha1(tex.encode("utf-8")).hexdigest()[:16]


def load(path):
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.int16)


def ink(img):
    return (255 - img).max(axis=2) > INK


def place(img, height, width, top, left):
    canvas = np.full((height, width, 3), 255, dtype=np.int16)
    canvas[top:top + img.shape[0], left:left + img.shape[1]] = img
    return canvas


def align(ref, tik):
    """Both images on one canvas, voltik's shifted where it overlaps TeX's ink the most."""
    h = max(ref.shape[0], tik.shape[0]) + 2 * SHIFT
    w = max(ref.shape[1], tik.shape[1]) + 2 * SHIFT
    ref_c = place(ref, h, w, SHIFT, SHIFT)
    # dilated, so a hairline that is off by a pixel still overlaps
    ref_wide = ndimage.binary_dilation(ink(ref_c), iterations=1).astype(np.float32)
    tik_ink = ink(tik).astype(np.float32)
    th, tw = tik_ink.shape
    # overlap[top + th - 1, left + tw - 1] = ink in common with voltik's image placed at (top, left)
    overlap = signal.fftconvolve(ref_wide, tik_ink[::-1, ::-1], mode="full")
    anchors = {
        (SHIFT, SHIFT),
        (SHIFT, SHIFT + ref.shape[1] - tw),
        (SHIFT + ref.shape[0] - th, SHIFT),
        (SHIFT + ref.shape[0] - th, SHIFT + ref.shape[1] - tw),
        (SHIFT + (ref.shape[0] - th) // 2, SHIFT + (ref.shape[1] - tw) // 2),
    }
    best, best_pos = -1.0, (SHIFT, SHIFT)
    for top0, left0 in anchors:
        for top in range(max(0, top0 - SHIFT), min(h - th, top0 + SHIFT) + 1):
            for left in range(max(0, left0 - SHIFT), min(w - tw, left0 + SHIFT) + 1):
                v = overlap[top + th - 1, left + tw - 1]
                if v > best + 0.5:
                    best, best_pos = v, (top, left)
    return ref_c, place(tik, h, w, *best_pos)


def edges(img):
    """Where the color changes: the outline of a fill, both sides of a line, and text or a line drawn
    on top of a fill, which an ink mask alone would hide."""
    e = np.zeros(img.shape[:2], dtype=bool)
    dx = np.abs(img[:, 1:] - img[:, :-1]).max(axis=2) > EDGE
    dy = np.abs(img[1:] - img[:-1]).max(axis=2) > EDGE
    e[:, 1:] |= dx
    e[:, :-1] |= dx
    e[1:] |= dy
    e[:-1] |= dy
    return e


def pieces(edge, other_ink):
    """The separate pieces of a drawing (a line, a word, a node; edges 2 px apart join) with fewer than
    half of their pixels near the other drawing's ink: what one drawing has and the other does not."""
    labels, n = ndimage.label(ndimage.binary_dilation(edge, iterations=2), structure=np.ones((3, 3)))
    if n == 0:
        return 0, 0
    labels = np.where(edge, labels, 0)
    near = ndimage.distance_transform_edt(~other_ink) <= RADIUS
    size = np.bincount(labels.ravel(), minlength=n + 1)
    hit = np.bincount(labels.ravel(), weights=near.ravel(), minlength=n + 1)
    big = size[1:] >= MIN_PIECE
    missing = big & (hit[1:] < 0.5 * size[1:])
    return int(big.sum()), int(missing.sum())


def measure(ref_path, tik_path):
    ref, tik = load(ref_path), load(tik_path)
    size_ok = all(abs(a - b) <= max(4, 0.1 * a) for a, b in zip(ref.shape[:2], tik.shape[:2]))
    ref_c, tik_c = align(ref, tik)
    ref_ink, tik_ink = ink(ref_c), ink(tik_c)
    base = {"size_ok": size_ok, "ref_size": list(ref.shape[1::-1]), "voltik_size": list(tik.shape[1::-1])}
    if not ref_ink.any() or not tik_ink.any():
        same = float(ref_ink.any() == tik_ink.any())
        return {**base, "f1": same, "precision": same, "recall": same, "color": 1.0, "pieces": 0, "missing": 0, "extra": 0}
    ref_e, tik_e = edges(ref_c), edges(tik_c)
    if not ref_e.any() or not tik_e.any():
        same = float(ref_e.any() == tik_e.any())
        return {**base, "f1": same, "precision": same, "recall": same, "color": 1.0, "pieces": 0, "missing": 0, "extra": 0}
    d_ref, nearest = ndimage.distance_transform_edt(~ref_e, return_indices=True)
    d_tik = ndimage.distance_transform_edt(~tik_e)
    near = tik_e & (d_ref <= RADIUS)
    precision = np.count_nonzero(near) / np.count_nonzero(tik_e)
    recall = np.count_nonzero(ref_e & (d_tik <= RADIUS)) / np.count_nonzero(ref_e)
    f1 = 0.0 if precision + recall == 0 else 2 * precision * recall / (precision + recall)
    # color: voltik's ink against the TeX ink nearest to it
    tik_ink_near = tik_ink & (ndimage.distance_transform_edt(~ref_ink) <= RADIUS)
    if tik_ink_near.any():
        # the same color as some TeX ink in the 3×3 pixels around: the two antialias a line's edge at different
        # phases, so the single nearest TeX pixel can be the light rim of a line voltik draws dark there
        # both drawings a little blurred: Ghostscript and resvg spread a thin line's ink over the
        # pixels differently (one dark pixel, or two lighter ones), which is not a color difference
        ref_b = ndimage.gaussian_filter(ref_c.astype(np.float32), sigma=(BLUR, BLUR, 0))
        tik_b = ndimage.gaussian_filter(tik_c.astype(np.float32), sigma=(BLUR, BLUR, 0))
        same = np.zeros(tik_ink.shape, dtype=bool)
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                ref_s = np.roll(ref_b, (dy, dx), axis=(0, 1))
                ink_s = np.roll(ref_ink, (dy, dx), axis=(0, 1))
                same |= ink_s & (np.abs(ref_s - tik_b).max(axis=2) <= COLOR)
        color = float(np.count_nonzero(same & tik_ink_near) / np.count_nonzero(tik_ink_near))
    else:
        color = 0.0
    n_ref, missing = pieces(ref_e, tik_e)
    _, extra = pieces(tik_e, ref_e)
    # ink: how much of each drawing's ink lies near the other's (a fill one has and the other does
    # not is mostly away from the edges, where the scores above do not look)
    # (near any mark of the other at all: a light line can be ink in one and just under INK in the other)
    near_ref = ndimage.distance_transform_edt(~((255 - ref_c).max(axis=2) > FAINT)) <= RADIUS
    near_tik = ndimage.distance_transform_edt(~((255 - tik_c).max(axis=2) > FAINT)) <= RADIUS
    ink_share = min(np.count_nonzero(tik_ink & near_ref) / np.count_nonzero(tik_ink), np.count_nonzero(ref_ink & near_tik) / np.count_nonzero(ref_ink))
    return {**base, "f1": round(f1, 4), "precision": round(precision, 4), "recall": round(recall, 4), "color": round(color, 4),
            "ink": round(float(ink_share), 4), "pieces": n_ref, "missing": missing, "extra": extra}


def category(r):
    if r["f1"] >= MATCH and r["color"] >= COLOR_OK and r.get("ink", 1.0) >= INK_OK and r["size_ok"] and r["missing"] == 0 and r["extra"] == 0:
        return "match"
    if r["f1"] >= CLOSE:
        return "close"
    return "different"


def warning_key(w):
    """`\\pic is not supported: ignored` stays; the varying part of a warning (what was ignored, numbers) goes."""
    w = re.sub(r"ignored: .*", "ignored", w)
    w = re.sub(r"'[^']*'|\"[^\"]*\"", "…", w)
    w = re.sub(r"\(.*?\)", "(…)", w)
    w = re.sub(r"\d+(\.\d+)?", "N", w)
    return w[:90]


def pct(n, d):
    return f"{100 * n / d:.1f}%" if d else "—"

CATS = ["match", "close", "different", "fails"]


def unknown_names(warnings):
    """The components or options voltik said it does not know."""
    for w in warnings:
        m = re.search(r"unknown component (.+?); drawn as", w) or re.match(r"option '([^']*)' not supported", w)
        if m:
            yield m.group(1)


def knows(component, warnings):
    return component not in set(unknown_names(warnings))
KINDS = {"example": "Examples of the manual", "path": "Path-style components", "node": "Node-style components"}


def main():
    p = argparse.ArgumentParser()
    p.add_argument("examples")
    p.add_argument("work")
    p.add_argument("--docs")
    a = p.parse_args()
    work = Path(a.work)
    voltik = {}
    for line in open(work / "voltik/results.jsonl", encoding="utf-8"):
        r = json.loads(line)
        voltik[r["hash"]] = r

    results = []
    for line in open(a.examples, encoding="utf-8"):
        e = json.loads(line)
        h = digest(e["tex"])
        ref_meta = json.loads((work / "ref" / f"{h}.json").read_text())
        t = voltik.get(h, {"status": "missing"})
        r = {k: e.get(k) for k in ("id", "line", "section", "chapter", "kind", "component")}
        r.update(hash=h, tex_ms=ref_meta["ms"], voltik_status=t["status"], warnings=t.get("warnings", []), svg_us=t.get("svg_us"))
        ref_png = work / "ref" / f"{h}.png"
        if not ref_meta["ok"] or not ref_png.exists():
            r.update(category="excluded", reason="TeX could not compile it", tex_error=ref_meta.get("error", ""))
        elif not ink(load(ref_png)).any():
            r.update(category="excluded", reason="TeX draws nothing visible")
        elif t["status"] != "drawn" or t.get("png_error"):
            r.update(category="fails", reason=t.get("error") or t.get("png_error") or t["status"])
        else:
            r.update(measure(ref_png, work / "voltik" / f"{h}.png"))
            r["category"] = category(r)
            if r["component"] and not knows(r["component"], r["warnings"]):
                # drawn as a plain wire: whatever the score, the component is not there
                r["category"] = "different"
        results.append(r)
    (work / "results.json").write_text(json.dumps(results, indent=1, ensure_ascii=False))
    summary = report_md(results)
    Path(a.docs or work / "report.md").write_text(summary)
    report_html(results, work)
    print(summary.split("\n## ")[0])


def table_row(name, rs):
    n = len(rs)
    c = Counter(r["category"] for r in rs)
    f1 = [r["f1"] for r in rs if "f1" in r] + [0.0] * c["fails"]
    score = sum(f1) / len(f1) if f1 else 0
    return f"| {name} | {n} | " + " | ".join(f"{c[k]} ({pct(c[k], n)})" for k in CATS) + f" | {score:.2f} |"


HEADER = ["| | Items | Match | Close | Different | Fails | Mean F1 |", "|---|---:|---:|---:|---:|---:|---:|"]


def report_md(results):
    counted = [r for r in results if r["category"] != "excluded"]
    excluded = Counter((r["kind"], r["reason"]) for r in results if r["category"] == "excluded")
    c = Counter(r["category"] for r in counted)
    silent = [r for r in counted if r["category"] != "match" and r["voltik_status"] == "drawn" and not r["warnings"]]
    components = [r for r in counted if r["kind"] in ("path", "node")]
    drawn = [r for r in components if r["category"] == "match"]
    examples = [r for r in counted if r["kind"] == "example"]
    out = [
        "# Conformance with CircuiTikZ",
        "",
        "How far voltik is from CircuiTikZ, measured by its manual (CircuiTikZ 1.8.7): every example the manual",
        "shows and every entry of its list of components, drawn by pdflatex (TeX Live, CircuiTikZ 1.8.7) and by",
        "voltik, and compared. How it works and how to run it: [tools/conformance](../tools/conformance/README.md).",
        "",
        f"**{len(drawn)} of {len(components)} components ({pct(len(drawn), len(components))}) and "
        f"{sum(r['category'] == 'match' for r in examples)} of {len(examples)} examples "
        f"({pct(sum(r['category'] == 'match' for r in examples), len(examples))}) match TeX.** "
        f"Of all {len(counted)} items, {c['close']} are close, {c['different']} differ and {c['fails']} fail in voltik; "
        f"{len(silent)} of the ones that do not match came out with no warning.",
        "",
        *HEADER,
        table_row("All", counted),
        *[table_row(name, [r for r in counted if r["kind"] == k]) for k, name in KINDS.items()],
        "",
        "*Match*: same shape (F1 ≥ 0.9 on the edges, with 0.5 mm of tolerance), no missing or extra piece, the same colors and size. *Close*: F1 ≥ 0.75.",
        "*Fails*: voltik returned an error, panicked or took more than 10 s. Mean F1 counts a failure as 0.",
        "",
        "A component is one entry of the manual's list, drawn as the manual draws it: `\\draw (0,0) to[<name>] (2,0);` "
        "for a path-style one, `\\draw (0,0) node[<name>]{};` for a node-style one, filled when the manual fills it. "
        "A component voltik says it does not know counts as different, even when the wire it draws instead scores close.",
        "",
    ]
    if excluded:
        out += ["Not counted: " + ", ".join(f"{n} {KINDS[k].lower()} where {reason.lower()}" for (k, reason), n in excluded.most_common()) + ".", ""]
    out += ["## By section", "", "Sorted by how many items do not match, the work left.", "",
            "| Section | Items | Match | Close | Different | Fails | Mean F1 |", "|---|---:|---:|---:|---:|---:|---:|"]
    chapters = defaultdict(list)
    for r in counted:
        chapters[(r["chapter"], r["kind"])].append(r)
    for (title, kind), rs in sorted(chapters.items(), key=lambda kv: (-sum(r["category"] != "match" for r in kv[1]), kv[0])):
        out.append(table_row(f"{title} <sub>{kind}</sub>", rs))
    out += ["", "## Components", "",
            "The manual's list of components, group by group, with the ones voltik draws as TeX does (and, in italics, the close ones).", "",
            "| Group | Components | Match | Close | Drawn as TeX does |", "|---|---:|---:|---:|---|"]
    groups = defaultdict(list)
    for r in components:
        groups[r["chapter"]].append(r)
    for title, rs in groups.items():
        cc = Counter(r["category"] for r in rs)
        names = [f"`{r['component']}`" for r in rs if r["category"] == "match"] + [f"*`{r['component']}`*" for r in rs if r["category"] == "close"]
        out.append(f"| {title} | {len(rs)} | {cc['match']} | {cc['close']} | {' '.join(names)} |")
    warn = Counter()
    for r in counted:
        for k in {warning_key(w) for w in r["warnings"]}:
            warn[k] += 1
    names = {r["component"] for r in components}
    unknown, options = Counter(), Counter()
    for r in counted:
        for name in set(unknown_names(r["warnings"])):
            (unknown if name in names else options)[name] += 1
    errors = Counter(warning_key(r["reason"]) for r in counted if r["category"] == "fails")
    out += ["", "## What voltik reported", "", "Warnings, by how many items they appear in (the varying part replaced by …):", "",
            "| Items | Warning |", "|---:|---|"]
    out += [f"| {n} | `{k.replace('|', '¦')}` |" for k, n in warn.most_common(40)]
    out += ["", "Components voltik does not know yet, by how many items use them:", "", "| Items | Component |", "|---:|---|"]
    out += [f"| {n} | `{k.replace('|', '¦')}` |" for k, n in unknown.most_common(60)]
    out += ["", "Other options voltik does not know, likewise:", "", "| Items | Option |", "|---:|---|"]
    out += [f"| {n} | `{k.replace('|', '¦')}` |" for k, n in options.most_common(40)]
    out += ["", "Errors of the items that fail:", "", "| Items | Error |", "|---:|---|"]
    out += [f"| {n} | `{k.replace('|', '¦')}` |" for k, n in errors.most_common(20)]
    out.append("")
    return "\n".join(out)


def report_html(results, work):
    rows = []
    order = {"fails": 0, "different": 1, "close": 2, "match": 3}
    shown = [r for r in results if r["category"] in order]
    for r in sorted(shown, key=lambda r: (order[r["category"]], r["line"])):
        score = f"F1 {r['f1']:.2f} · color {r['color']:.2f} · missing {r['missing']} of {r['pieces']} pieces · extra {r['extra']}" + ("" if r.get("size_ok") else " · size differs") if "f1" in r else html.escape(r.get("reason", ""))
        warnings = "".join(f"<li>{html.escape(w)}</li>" for w in r["warnings"][:8])
        tik = f'<img loading="lazy" src="voltik/{r["hash"]}.png">' if r["voltik_status"] == "drawn" and "f1" in r else "<em>no drawing</em>"
        name = f" · <code>{html.escape(r['component'])}</code>" if r.get("component") else ""
        rows.append(
            f'<section class="{r["category"]}"><h3>{html.escape(r["id"])} <span>{r["category"]}</span></h3>'
            f'<p>{html.escape(r["chapter"])} · {r["kind"]}{name} · {score}</p>'
            f'<div class="pair"><figure><img loading="lazy" src="ref/{r["hash"]}.png"><figcaption>TeX</figcaption></figure>'
            f'<figure>{tik}<figcaption>voltik</figcaption></figure></div><ul>{warnings}</ul></section>')
    counts = Counter(r["category"] for r in shown)
    buttons = "".join(f'<button data-c="{c}">{c} ({counts[c]})</button>' for c in ["fails", "different", "close", "match"])
    page = f"""<!doctype html><meta charset="utf-8"><title>voltik conformance</title>
<style>
body{{font:14px system-ui;margin:24px;background:#f6f6f6}} section{{background:#fff;border:1px solid #ddd;border-radius:8px;padding:10px 14px;margin:10px 0}}
h3{{margin:0;font:600 13px ui-monospace,monospace}} h3 span{{color:#a50}} p{{margin:4px 0;color:#555}} ul{{margin:4px 0;color:#753;font:12px ui-monospace,monospace}}
.pair{{display:flex;gap:16px;flex-wrap:wrap}} figure{{margin:0}} figure img{{max-width:46vw;border:1px solid #eee}} figcaption{{color:#888;font-size:12px}}
button{{margin-right:6px}} .hide{{display:none}}
</style>
<p>{buttons}</p>
{"".join(rows)}
<script>for(const b of document.querySelectorAll("button"))b.onclick=()=>{{for(const s of document.querySelectorAll("section"))s.classList.toggle("hide",!s.classList.contains(b.dataset.c))}};document.querySelector("button").click();</script>
"""
    (work / "report.html").write_text(page)


if __name__ == "__main__":
    main()
