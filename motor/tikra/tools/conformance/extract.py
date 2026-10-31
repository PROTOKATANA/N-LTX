"""Turns every example of the TikZ and pgfplots manuals into a standalone document.

    python3 extract.py <pgf source dir> <pgfplots source dir> <out.jsonl>

Each line of the output is one example: its id (manual, file, line), the chapter it comes from, what
kind of code it is, and the full document given both to pdflatex (the reference) and to tikra.

The manuals typeset `\\begin{codeexample}[options] … \\end{codeexample}` blocks and run their code. The
options decide what runs: `code only`, `hidden` and `multipage` are not drawn; `setup code` is run
without being shown and stays defined for the rest of the file; `preamble`, `pre`, `post` and
`render instead` change what is compiled. They are honored here the way the manual's macros honor them
(pgfmanual-en-macros.tex), so each document draws what the manual shows.
"""

import json
import re
import sys
from pathlib import Path

# The libraries the TikZ manual loads for every example (pgfmanual-en-main-preamble.tex), minus
# graphdrawing (LuaTeX only), external and animations (no static drawing). One line, so tikra reads
# it as a preamble command.
PGF_LIBRARIES = (
    "3d,arrows,arrows.spaced,arrows.meta,bending,babel,calc,fit,patterns,patterns.meta,plotmarks,"
    "shapes.geometric,shapes.misc,shapes.symbols,shapes.arrows,shapes.callouts,shapes.multipart,"
    "shapes.gates.logic.US,shapes.gates.logic.IEC,circuits.logic.US,circuits.logic.IEC,circuits.logic.CDH,"
    "circuits.ee.IEC,datavisualization,datavisualization.polar,datavisualization.formats.functions,er,"
    "automata,backgrounds,chains,topaths,trees,petri,mindmap,matrix,calendar,folding,fadings,shadings,spy,"
    "through,turtle,positioning,scopes,decorations.fractals,decorations.shapes,decorations.text,"
    "decorations.pathmorphing,decorations.pathreplacing,decorations.footprints,decorations.markings,"
    "shadows,lindenmayersystems,intersections,fixedpointarithmetic,fpu,svg.path,graphs,graphs.standard,"
    "quotes,math,angles,views,rdf,perspective"
)

# The pgfplots manual (pgfplots.tex and pgfplots.preamble.tex). No `compat` key, as in the manual.
PGFPLOTS_PREAMBLE = "\n".join([
    r"\usepackage{pgfplots}",
    r"\usepackage{pgfplotstable}",
    r"\usetikzlibrary{spy,decorations.markings,decorations.footprints,shapes.arrows,matrix,positioning}",
    r"\usepgfplotslibrary{dateplot,units,groupplots,statistics,fillbetween,ternary,smithchart,patchplots,polar,colormaps,colorbrewer,colortol}",
    r"\pgfplotsset{table/search path={.,plotdata}}",
])

BEGIN = r"\begin{codeexample}"
END = r"\end{codeexample}"


def balanced(text, i, open_char, close_char):
    """The content of the group opening at text[i], and the index after it."""
    depth = 0
    j = i
    while j < len(text):
        c = text[j]
        if c == "\\":
            j += 2
            continue
        if c == open_char:
            depth += 1
        elif c == close_char:
            depth -= 1
            if depth == 0:
                return text[i + 1:j], j + 1
        j += 1
    raise ValueError("unbalanced group")


def split_options(options):
    """`a, b={x,y}, c=d` → {"a": None, "b": "x,y", "c": "d"}, splitting only at top-level commas."""
    parts, depth, current = [], 0, ""
    for c in options:
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
        if c == "," and depth == 0:
            parts.append(current)
            current = ""
        else:
            current += c
    parts.append(current)
    result = {}
    for p in parts:
        p = p.strip()
        if not p:
            continue
        key, _, value = p.partition("=")
        value = value.strip()
        if value.startswith("{") and value.endswith("}"):
            value = value[1:-1]
        result[key.strip()] = value if _ else None
    return result


def strip_comments(text):
    """Drops TeX comments (`%` to the end of the line, unless escaped)."""
    return "\n".join(re.sub(r"(?<!\\)%.*", "", line) for line in text.split("\n"))


def blocks(source):
    """Every codeexample block of a file: (line, options, body)."""
    at = 0
    while True:
        i = source.find(BEGIN, at)
        if i < 0:
            return
        j = i + len(BEGIN)
        options = ""
        k = j
        while k < len(source) and source[k] in " \t":
            k += 1
        if k < len(source) and source[k] == "[":
            options, j = balanced(source, k, "[", "]")
        end = source.find(END, j)
        if end < 0:
            return
        yield source.count("\n", 0, i) + 1, strip_comments(options), source[j:end]
        at = end + len(END)


def kind(code):
    """What the example draws with: TikZ (tikra's target), the pgf basic layer, or neither."""
    if re.search(r"\\begin\{tikzpicture\}|\\tikz\b|\\begin\{(axis|semilogxaxis|semilogyaxis|loglogaxis|polaraxis|ternaryaxis|smithchart|groupplot)\}", code):
        return "tikz"
    if re.search(r"\\begin\{pgfpicture\}|\\pgfpicture\b", code):
        return "pgf"
    return "other"


def chapter(source, path):
    m = re.search(r"\\(?:section|chapter|part)\*?\{((?:[^{}]|\{[^{}]*\})*)\}", source)
    title = m.group(1) if m else path.stem
    return re.sub(r"\\[a-zA-Z]+\s*|[{}|]", "", title).strip()


def file_libraries(source):
    """`\\usetikzlibrary`, `\\usepgflibrary` and `\\usepgfplotslibrary` outside the examples: they stay loaded for the rest of the file."""
    outside = re.sub(re.escape(BEGIN) + r".*?" + re.escape(END), "", source, flags=re.S)
    outside = strip_comments(outside)
    found = []
    for m in re.finditer(r"^\s*\\(usetikzlibrary|usepgflibrary|usepgfplotslibrary)\s*\{([^}]*)\}", outside, re.M):
        names = ",".join(n.strip() for n in m.group(2).split(",") if n.strip())
        if names:
            found.append(f"\\{m.group(1)}{{{names}}}")
    return found


def document(preamble, setup, pre, code, post):
    body = "\n".join(s for s in setup) + ("\n" if setup else "") + pre + code + post
    return "\n".join([
        r"\documentclass[border=1pt,varwidth=40cm]{standalone}",
        r"\usepackage{amsmath,amssymb}",
        *preamble,
        r"\begin{document}",
        body.strip("\n"),
        r"\end{document}",
        "",
    ])


def examples(manual, directory, base_preamble, pattern):
    for path in sorted(Path(directory).glob(pattern)):
        source = path.read_text(encoding="latin-1")
        if BEGIN not in source:
            continue
        title = chapter(source, path)
        preamble = base_preamble + file_libraries(source)
        setup = []
        for line, raw_options, body in blocks(source):
            options = split_options(raw_options)
            if "setup code" in options:
                setup.append(body.strip("\n"))
                continue
            if any(k in options for k in ("code only", "hidden", "multipage", "animation list", "from file")):
                continue
            code = options.get("render instead") or body
            extra = [p for p in [options.get("preamble")] if p]
            pre, post = options.get("pre") or "", options.get("post") or ""
            if "vbox" in options:
                pre, post = r"\vbox\bgroup\hsize=12cm\relax" + pre, post + r"\egroup"
            yield {
                "id": f"{manual}/{path.stem}:{line}",
                "manual": manual,
                "file": path.name,
                "line": line,
                "chapter": title,
                "kind": kind(code),
                "tex": document(preamble + extra, setup, pre, code.strip("\n") + "\n", post),
            }


def main():
    pgf_dir, pgfplots_dir, out = sys.argv[1:4]
    pgf_preamble = [r"\usepackage{tikz}", rf"\usetikzlibrary{{{PGF_LIBRARIES}}}"]
    found = list(examples("tikz", Path(pgf_dir) / "doc/generic/pgf", pgf_preamble, "pgfmanual-en-*.tex"))
    found += list(examples("pgfplots", Path(pgfplots_dir) / "doc/latex/pgfplots", PGFPLOTS_PREAMBLE.split("\n"), "pgfplots*.tex"))
    with open(out, "w", encoding="utf-8") as f:
        for e in found:
            f.write(json.dumps(e, ensure_ascii=False) + "\n")
    by_kind = {}
    for e in found:
        key = (e["manual"], e["kind"])
        by_kind[key] = by_kind.get(key, 0) + 1
    for (manual, k), n in sorted(by_kind.items()):
        print(f"{manual:9} {k:6} {n}")
    print(f"total     {len(found)}")


if __name__ == "__main__":
    main()
