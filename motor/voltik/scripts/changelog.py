"""Reads CHANGELOG.md, for the GitHub release and for the site's page of the project's evolution.

    python3 scripts/changelog.py notes <version>   the release's notes (exits 1 if it has no section)
    python3 scripts/changelog.py js                 the site's data: var VOLTIK_CHANGELOG = {...};

A release is a section `## <version> - <date>` (or `## Unreleased`): its first paragraph is the
headline, the `- ` lines what changed, `**Matches TeX:** ...` how it compares with CircuiTikZ, and
`Figures: `a` `b`` the names of corpus/ figures that show it. The site's chart reads docs/progress.tsv
(date, components drawn, corpus figures).
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HEADING = re.compile(r"^## (Unreleased|(\d+\.\d+\.\d+) - (\d{4}-\d{2}-\d{2}))\s*$", re.M)


def sections():
    text = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    heads = list(HEADING.finditer(text))
    for k, h in enumerate(heads):
        end = heads[k + 1].start() if k + 1 < len(heads) else len(text)
        yield h, text[h.end():end].strip()


def release(h, body):
    paragraphs = [p.strip() for p in body.split("\n\n") if p.strip()]
    headline = paragraphs[0] if paragraphs and not paragraphs[0].startswith(("- ", "**", "Figures:")) else ""
    figures = re.search(r"^Figures:(.*)$", body, re.M)
    matches = re.search(r"^\*\*Matches TeX:\*\*\s*(.*)$", body, re.M)
    return {
        "version": h.group(2) or "Unreleased",
        "date": h.group(3) or "",
        "headline": headline,
        "items": [line[2:].strip() for line in body.splitlines() if line.startswith("- ")],
        "matches": matches.group(1).strip() if matches else "",
        "figures": re.findall(r"`([^`]+)`", figures.group(1)) if figures else [],
    }


def progress():
    rows = (ROOT / "docs/progress.tsv").read_text(encoding="utf-8").split("\n")[1:]
    return [{"date": d, "components": int(c), "figures": int(f)} for d, c, f in (r.split("\t") for r in rows if r.strip())]


def main():
    if len(sys.argv) == 3 and sys.argv[1] == "notes":
        for h, body in sections():
            if h.group(2) == sys.argv[2]:
                print(body)
                return
        sys.exit(f"CHANGELOG.md has no section for {sys.argv[2]}: add `## {sys.argv[2]} - <date>` with what changed")
    if len(sys.argv) == 2 and sys.argv[1] == "js":
        data = {"releases": [release(h, body) for h, body in sections()], "progress": progress()}
        print("var VOLTIK_CHANGELOG = " + json.dumps(data, ensure_ascii=False) + ";")
        return
    sys.exit(__doc__)


if __name__ == "__main__":
    main()
