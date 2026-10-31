"""How Ghostscript draws CMYK colors, as a table for tikra (crates/core/src/color/cmyk.bin).

    python3 -I tools/cmyk/table.py [--image IMAGE]

pdflatex writes a cmyk color (`cyan`, `\\definecolor{x}{cmyk}{…}`, the dvipsnames) as such in the PDF,
and Ghostscript, which draws the conformance references, turns it into RGB through its default ICC
profiles: `cyan` is (0,174,239), not (0,255,255). This draws a grid of N⁴ cmyk patches (N levels
of each ink, from 0 to 1) with that Ghostscript and writes their RGB, in c, m, y, k order (k fastest),
three bytes each; tikra interpolates between them.

Needs Docker, Python 3 and Pillow.
"""

import argparse
import subprocess
import tempfile
from pathlib import Path

from PIL import Image

IMAGE = "texlive/texlive:TL2025-historic"  # as tools/conformance/reference.py
N = 13
PATCH = 2  # pt; 4 px at 144 dpi


def pdf(path):
    levels = [i / (N - 1) for i in range(N)]
    ops = []
    for ci, c in enumerate(levels):
        for mi, m in enumerate(levels):
            for yi, y in enumerate(levels):
                for ki, k in enumerate(levels):
                    x, z = (ci * N + mi) * PATCH, (yi * N + ki) * PATCH
                    ops.append(f"{c:.6f} {m:.6f} {y:.6f} {k:.6f} k {x} {z} {PATCH} {PATCH} re f")
    content = "\n".join(ops).encode()
    side = N * N * PATCH
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {side} {side}] /Contents 4 0 R >>".encode(),
        b"<< /Length %d >>\nstream\n" % len(content) + content + b"\nendstream",
    ]
    out = b"%PDF-1.5\n"
    offsets = []
    for i, o in enumerate(objs):
        offsets.append(len(out))
        out += f"{i + 1} 0 obj\n".encode() + o + b"\nendobj\n"
    xref = len(out)
    out += f"xref\n0 {len(objs) + 1}\n0000000000 65535 f \n".encode()
    out += b"".join(f"{o:010d} 00000 n \n".encode() for o in offsets)
    out += f"trailer\n<< /Size {len(objs) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    path.write_bytes(out)


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--image", default=IMAGE)
    a = p.parse_args()
    target = Path(__file__).resolve().parents[2] / "crates/core/src/color/cmyk.bin"
    with tempfile.TemporaryDirectory() as d:
        work = Path(d)
        pdf(work / "grid.pdf")
        subprocess.run(
            ["docker", "run", "--rm", "-v", f"{work}:/w", "-w", "/w", a.image,
             "gs", "-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=png16m", "-r144", "-o", "grid.png", "grid.pdf"],
            check=True,
        )
        im = Image.open(work / "grid.png").convert("RGB")
        px = PATCH * 2
        height = im.size[1]
        table = bytearray()
        for ci in range(N):
            for mi in range(N):
                for yi in range(N):
                    for ki in range(N):
                        x, z = (ci * N + mi) * px + px // 2, (yi * N + ki) * px + px // 2
                        table += bytes(im.getpixel((x, height - 1 - z)))
    target.write_bytes(bytes(table))
    print(f"{target}: {N}^4 colors")


if __name__ == "__main__":
    main()
