"""Where the red ink is, from the black ink's box, in TeX's drawing and in voltik's (in pt).

    python3 -I tools/oracle/ink.py a.tex b.tex ...

For labels written in red (`l=\\textcolor{red}{$R$}`): each line gives the red box's left, right, top
and bottom edges from the center of the black drawing (y up), for TeX and for voltik, and the
difference. Both drawings must be there at 200 dpi (tools/oracle/tex.sh, and voltik's
<file>.voltik.png).
"""

import pathlib
import sys

import numpy as np
from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parents[2]
PT = 72.27 / 200


def boxes(path):
    a = np.asarray(Image.open(path).convert("RGB")).astype(int)
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    red = (r > 150) & (g < 120) & (b < 120)
    black = (r < 110) & (g < 110) & (b < 110)

    def box(m):
        ys, xs = np.nonzero(m)
        return xs.min(), xs.max(), ys.min(), ys.max()

    bx0, bx1, by0, by1 = box(black)
    cx, cy = (bx0 + bx1) / 2, (by0 + by1) / 2
    x0, x1, y0, y1 = box(red)
    return [(x0 - cx) * PT, (x1 - cx) * PT, (cy - y0) * PT, (cy - y1) * PT]


def main():
    print(f"{'figure':<14}{'':8}{'left':>8}{'right':>8}{'top':>8}{'bottom':>8}")
    for f in sys.argv[1:]:
        f = pathlib.Path(f)
        tex = boxes(ROOT / "tools/oracle/out" / f"{f.stem}.png")
        ours = boxes(f.with_suffix(".voltik.png"))
        for name, v in [("TeX", tex), ("voltik", ours), ("diff", [o - t for o, t in zip(ours, tex)])]:
            print(f"{f.stem if name == 'TeX' else '':<14}{name:<8}" + "".join(f"{x:8.2f}" for x in v))


if __name__ == "__main__":
    main()
