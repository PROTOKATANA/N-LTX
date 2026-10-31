"""Reads CHANGELOG.md, for the GitHub release and for the site's page of the project's evolution.

    python3 scripts/changelog.py notes <version>   the release's notes (exits 1 if it has no section)
    python3 scripts/changelog.py js                 the site's data: var MATHRA_CHANGELOG = {...};

A release is a section `## <version> - <date>` (or `## Unreleased`): its first paragraph is the
headline, the `- ` lines what changed, and `Equations: `a` `b`` the equations that show it, each a
name from corpus/gallery.json or the LaTeX itself. crates/core/tests/examples.rs checks that each one
converts in strict mode.
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


def gallery():
    """The named rows of the gallery: name -> {tex, locale, input, caption}."""
    groups = json.loads((ROOT / "corpus/gallery.json").read_text(encoding="utf-8"))
    return {r["name"]: r for g in groups for r in g["rows"] if r.get("name")}


def release(h, body, named):
    paragraphs = [p.strip() for p in body.split("\n\n") if p.strip()]
    headline = paragraphs[0] if paragraphs and not paragraphs[0].startswith(("- ", "**", "Equations:")) else ""
    listed = re.search(r"^Equations:(.*)$", body, re.M)
    equations = []
    for item in re.findall(r"`([^`]+)`", listed.group(1)) if listed else []:
        row = named.get(item)
        if row:
            equations.append({"tex": row["tex"], "locale": row.get("locale", "en"), "input": row.get("input", "latex"), "caption": row["caption"]})
        else:
            equations.append({"tex": item, "locale": "en", "input": "latex", "caption": ""})
    return {
        "version": h.group(2) or "Unreleased",
        "date": h.group(3) or "",
        "headline": headline,
        "items": [line[2:].strip() for line in body.splitlines() if line.startswith("- ")],
        "equations": equations,
    }


def main():
    if len(sys.argv) == 3 and sys.argv[1] == "notes":
        for h, body in sections():
            if h.group(2) == sys.argv[2]:
                print(body)
                return
        sys.exit(f"CHANGELOG.md has no section for {sys.argv[2]}: add `## {sys.argv[2]} - <date>` with what changed")
    if len(sys.argv) == 2 and sys.argv[1] == "js":
        named = gallery()
        data = {"releases": [release(h, body, named) for h, body in sections()]}
        print("var MATHRA_CHANGELOG = " + json.dumps(data, ensure_ascii=False) + ";")
        return
    sys.exit(__doc__)


if __name__ == "__main__":
    main()
