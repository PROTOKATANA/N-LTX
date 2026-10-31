"""TeX's drawing next to voltik's, one row per figure, for a look by eye.

    python3 -I tools/oracle/side.py out.png a.tex b.tex ...

Each figure must have been drawn by tools/oracle/tex.sh (tools/oracle/out/<name>.png) and by voltik
(<file>.voltik.png next to the .tex, at 200 dpi, as `voltik a.tex -o a.voltik.png --dpi 200`).
"""

import pathlib
import sys

from PIL import Image, ImageDraw, ImageOps

ROOT = pathlib.Path(__file__).resolve().parents[2]
PAD = 16


def trimmed(path):
    image = Image.open(path).convert("RGB")
    box = ImageOps.invert(image).getbbox()
    return image.crop(box) if box else image


def main():
    target, files = sys.argv[1], sys.argv[2:]
    rows = []
    for f in files:
        f = pathlib.Path(f)
        tex = trimmed(ROOT / "tools/oracle/out" / f"{f.stem}.png")
        ours = trimmed(f.with_suffix(".voltik.png"))
        rows.append((f.stem, tex, ours))
    width = max(t.width for _, t, _ in rows) + max(o.width for _, _, o in rows) + 3 * PAD
    height = sum(max(t.height, o.height) + 2 * PAD + 14 for _, t, o in rows)
    sheet = Image.new("RGB", (width, height), "white")
    draw = ImageDraw.Draw(sheet)
    left = max(t.width for _, t, _ in rows) + 2 * PAD
    y = 0
    for name, tex, ours in rows:
        draw.text((PAD, y + 2), f"{name}: TeX", fill="gray")
        draw.text((left, y + 2), "voltik", fill="gray")
        sheet.paste(tex, (PAD, y + 14 + PAD))
        sheet.paste(ours, (left, y + 14 + PAD))
        y += max(tex.height, ours.height) + 2 * PAD + 14
        draw.line([(0, y - 1), (width, y - 1)], fill="#ddd")
    sheet.save(target)


if __name__ == "__main__":
    main()
