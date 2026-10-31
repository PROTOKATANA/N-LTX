"""Writes docs/examples/<name>.svg, the corpus circuits shown in the READMEs, on a white card.

    cargo build --release -p voltik-cli && python3 -I scripts/examples.py
"""

import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXAMPLES = ["ladder-network", "voltage-divider", "rlc-circuit", "labels-around", "units-and-annotations", "european-styles"]


def main():
    out = ROOT / "docs/examples"
    out.mkdir(parents=True, exist_ok=True)
    corpus = {p.stem: p for p in (ROOT / "corpus").glob("*/*.tex")}
    for name in EXAMPLES:
        svg = subprocess.run([ROOT / "target/release/voltik", corpus[name]], capture_output=True, text=True, check=True).stdout
        # a white card under the drawing, so it reads on dark pages too
        box = re.search(r'viewBox="([-\d.]+) ([-\d.]+) ([-\d.]+) ([-\d.]+)"', svg)
        x, y, w, h = box.groups()
        svg = re.sub(r"(<svg[^>]*>)", rf'\1<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="#ffffff"/>', svg, count=1)
        (out / f"{name}.svg").write_text(svg, encoding="utf-8")
        print(f"docs/examples/{name}.svg")


if __name__ == "__main__":
    main()
