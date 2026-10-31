"""Turns every example and every component entry of the CircuiTikZ manual into a standalone document.

    python3 extract.py <circuitikz doc dir> <out.jsonl>

The doc dir is the `doc/` folder of the CircuiTikZ release (circuitikzmanual.tex and the manual's own
macros, ctikzmanutils.sty); nothing else of the release is read. Each line of the output is one item:
its id (line in the manual), the section it comes from, what kind it is, and the full document given
both to pdflatex (the reference) and to voltik.

Two kinds of items, both drawn by the manual itself:

- `example`: a `ctikzExample` block, the code the manual shows next to its result. `only code` blocks
  draw nothing and are skipped; the definitions of `exec outside` blocks stay in force for the rest of
  their group; `store`/`restore` prepend stored code, as the manual's macros do.
- `path` and `node`: an entry of the component list (`\\circuitdescbip` and `\\circuitdesc`), drawn as
  the manual draws it: `\\draw (0,0) to[<name>] (2,0);` or `\\draw (0,0) node[<name>]{<text>};`, filled
  with the manual's `fillcol` when the entry is fillable. The anchors the manual marks are left out.

The manual changes settings between examples (`\\ctikzset`, `\\tikzset`, `\\def`, `\\newcommand`, colors),
often inside `\\begingroup … \\endgroup`. Those settings are followed in order, group by group, and
every document starts with the ones in force at its place in the manual: every setting, and the
definitions the document uses (one it never uses would not change what TeX draws).
"""

import json
import re
import sys
from pathlib import Path

# The manual loads CircuiTikZ with these options (circuitikzmanual.tex), and its examples run in an
# article with T1 Latin Modern.
PREAMBLE = [
    r"\usepackage[T1]{fontenc}",
    r"\usepackage{lmodern}",
    r"\usepackage{amsmath,amssymb}",
    r"\usepackage[siunitx, RPvoltages]{circuitikz}",
]

# The commands that change settings for what follows; their arguments are read as TeX reads them.
SETUP = re.compile(r"\\(ctikzset|tikzset|ctikzsetstyle|ctikzloadstyle|ctikzsubcircuitdef|ctikzactivate(?:voltage|current|flow)directions|pgfkeys|usetikzlibrary|definecolor|colorlet|newcommand|renewcommand|providecommand|NewDocumentCommand|def|let)(?![A-Za-z@])")
EVENT = re.compile(
    r"\\begin\{([A-Za-z*]+)\}|\\end\{([A-Za-z*]+)\}|\\(begingroup|endgroup)(?![A-Za-z])"
    r"|\\(circuitdescbip|circuitdesc)(?![A-Za-z])|\\(section|subsection)\*?\{"
    r"|" + SETUP.pattern
)
# environments whose content is not run (listings) or is a figure of the text, not an example
SKIPPED = {"lstlisting", "verbatim", "comment", "circuitikz", "tikzpicture"}


def strip_comments(text):
    """Empties TeX comments, keeping the `%`: it still joins the line to the next, as in TeX."""
    return "\n".join(re.sub(r"(?<!\\)%.*", "%", line) for line in text.split("\n"))


def drop_inline_code(text):
    """Blanks `\\verb|…|` in the prose, outside the examples: code quoted in a sentence is not run."""
    parts = re.split(r"(\\begin\{ctikzExample\}.*?\\end\{ctikzExample\})", text, flags=re.S)
    verb = re.compile(r"\\verb\*?([^A-Za-z\s])(.*?)\1")
    return "".join(p if k % 2 else verb.sub(lambda m: " " * len(m.group(0)), p) for k, p in enumerate(parts))


def group(text, i, open_char="{", close_char="}"):
    """The content of the group opening at text[i] (after spaces), and the index after it; None if no group opens there."""
    while i < len(text) and text[i] in " \t\n":
        i += 1
    if i >= len(text) or text[i] != open_char:
        return None, i
    depth, j = 0, i
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
    raise ValueError(f"unbalanced group at {i}")


def star(text, i):
    j = i
    while j < len(text) and text[j] in " \t\n":
        j += 1
    return (True, j + 1) if j < len(text) and text[j] == "*" else (False, i)


def setup_statement(text, name, i):
    """The whole statement of a setting command whose name ends at text[i], and the index after it."""
    start = text.rfind("\\", 0, i)
    j = i
    if name in ("newcommand", "renewcommand", "providecommand"):
        _, j = star(text, j)
        target, j2 = group(text, j)
        if target is None:  # \newcommand\name
            m = re.compile(r"\s*\\[A-Za-z@]+").match(text, j)
            j = m.end()
        else:
            j = j2
        for _ in range(2):  # [n] and [default]
            arg, j2 = group(text, j, "[", "]")
            if arg is not None:
                j = j2
        _, j = group(text, j)
    elif name == "def":
        # \def\name<parameter text>{body}
        m = re.compile(r"\s*\\[A-Za-z@]+[^{]*").match(text, j)
        _, j = group(text, m.end())
    elif name == "let":
        j = re.compile(r"\s*\\[A-Za-z@]+\s*=?\s*\\[A-Za-z@]+").match(text, j).end()
    elif name == "NewDocumentCommand":
        for _ in range(3):  # name, argument specification, body
            _, j = group(text, j)
    elif name == "definecolor":
        arg, j2 = group(text, j, "[", "]")
        if arg is not None:
            j = j2
        for _ in range(3):
            _, j = group(text, j)
    elif name == "ctikzsubcircuitdef":
        for _ in range(3):  # name, anchors, path
            _, j = group(text, j)
    elif name == "colorlet":
        arg, j2 = group(text, j, "[", "]")
        if arg is not None:
            j = j2
        for _ in range(2):
            _, j = group(text, j)
    else:
        _, j = group(text, j)
    return text[start:j], j


def definitions(code):
    """The setting statements of a block run outside an example, without what it draws."""
    found, i = [], 0
    while (m := SETUP.search(code, i)) is not None:
        try:
            statement, i = setup_statement(code, m.group(1), m.end())
        except (AttributeError, TypeError, ValueError):
            i = m.end()
            continue
        found.append(statement)
    return found


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


def defined_name(statement):
    """What a definition defines, as a document would use it (`\\mydiv`, `\\divider`, a color); None for a setting."""
    m = re.match(r"\\(?:newcommand|renewcommand|providecommand|NewDocumentCommand)\*?\s*\{?\s*(\\[A-Za-z@]+)", statement) or re.match(r"\\(?:def|let)\s*(\\[A-Za-z@]+)", statement)
    if m:
        return m.group(1)
    m = re.match(r"\\ctikzsubcircuitdef\s*\{([^}]*)\}", statement)
    if m:
        return "\\" + m.group(1).strip()
    m = re.match(r"\\(?:definecolor|colorlet)\s*(?:\[[^]]*\])?\s*\{([^}]*)\}", statement)
    return m.group(1).strip() if m else None


def needed(setup, body):
    """The settings in force, and only the definitions the body uses (directly or through another
    definition it uses): a definition that is never used does not change what TeX draws."""
    keep = [defined_name(s) is None for s in setup]
    text = body + "\n".join(s for s, k in zip(setup, keep) if k)
    changed = True
    while changed:
        changed = False
        for i, s in enumerate(setup):
            if keep[i]:
                continue
            name = defined_name(s)
            # a macro `vif queue add={name}` will call is named there without its backslash
            use = re.escape(name) + r"(?![A-Za-z@])|vif queue add=\{" + re.escape(name[1:]) + r"\}" if name.startswith("\\") else re.escape(name) + r"\b"
            if re.search(use, text):
                keep[i], changed = True, True
                text += "\n" + s
    return [s for s, k in zip(setup, keep) if k]


def manual_macros(utils):
    """The manual's macros that examples use (anchor marks, names, `fillcol`), read from ctikzmanutils.sty:
    (the names a document must use for it to be loaded, the definition)."""
    keep = []
    for names, pattern in (
        ((), r"\\usetikzlibrary\{[^}]*\}"),
        ((r"\upgreek", r"\up"), r"\\RequirePackage\{upgreek\}"),
        ((r"\TikZ", r"\Circuitikz"), r"\\def\\TikZ\{[^\n]*"),
        ((r"\Circuitikz",), r"\\def\\Circuitikz\{[^\n]*"),
        (("fillcol",), r"\\colorlet\{fillcol\}\{[^}]*\}"),
        ((r"\showcoord", r"\showanchors", r"\geolrcoord", r"\geocoord"), r"\\makeatletter % we use the internal.*?\\makeatother"),
        ((r"\showanchors", r"\geolrcoord", r"\geocoord"), r"\\NewDocumentCommand\{\\showanchors\}.*?\n\}\n"),
        ((r"\geolrcoord",), r"\\newcommand\{\\geolrcoord\}.*?\n\}\n"),
        ((r"\geocoord",), r"\\newcommand\{\\geocoord\}.*?\n\}\n"),
    ):
        m = re.search(pattern, utils, re.S)
        if m is None:
            raise SystemExit(f"ctikzmanutils.sty: not found: {pattern}")
        keep.append((names, m.group(0).replace(r"\RequirePackage", r"\usepackage").strip()))
    return keep


def used(macros, code):
    """The macros a document uses: the ones every document loads, and those whose names it mentions."""
    return [m for names, m in macros if not names or any(n in code for n in names)]


def document(macros, setup, body):
    setup = needed(setup, body)
    return "\n".join([
        r"\documentclass[border=1pt,varwidth=40cm]{standalone}",
        *PREAMBLE,
        *used(macros, "\n".join(setup) + body),
        r"\begin{document}",
        *setup,
        body.strip("\n"),
        r"\end{document}",
        "",
    ])


def is_figure(code):
    return re.search(r"\\begin\{(circuitikz|tikzpicture)\}|\\tikz\b|\\ctikz", code) is not None


def items(manual, utils):
    macros = manual_macros(utils)
    start = manual.index(r"\begin{document}") + len(r"\begin{document}")
    end = manual.index(r"\section{Changelog")
    line0 = manual.count("\n", 0, start)
    text = drop_inline_code(strip_comments(manual[start:end]))
    frames = [[]]  # the settings of each open group, outermost first
    storage = {}
    section, subsection = "", ""
    i = 0

    def place():
        return f"{section} › {subsection}" if subsection else section

    def line(at):
        return line0 + text.count("\n", 0, at) + 1

    def setup():
        return [s for f in frames for s in f]

    while True:
        m = EVENT.search(text, i)
        if m is None:
            return
        at, i = m.start(), m.end()
        begin, finish, grp, desc, sec = m.group(1), m.group(2), m.group(3), m.group(4), m.group(5)
        if begin == "ctikzExample":
            raw, i = group(text, i, "[", "]")
            options = split_options(raw or "")
            close = text.index(r"\end{ctikzExample}", i)
            body = text[i:close].strip("\n")
            i = close + len(r"\end{ctikzExample}")
            for name in [options.get("clear storage")] if options.get("clear storage") else []:
                storage.pop(name, None)
            if options.get("store"):
                storage[options["store"]] = storage.get(options["store"], "") + body + "\n"
            if "exec outside" in options:
                frames[-1].extend(definitions(body))
                continue
            if "only code" in options:
                continue
            if options.get("restore"):
                body = storage.get(options["restore"], "") + body
            yield {"id": f"manual:{line(at)}", "line": line(at), "section": section, "chapter": place(),
                   "kind": "example" if is_figure(body) else "other", "tex": document(macros, setup(), body)}
        elif begin in SKIPPED:
            close = text.find(rf"\end{{{begin}}}", i)
            i = len(text) if close < 0 else close + len(rf"\end{{{begin}}}")
        elif begin:
            if begin != "document":
                frames.append([])
        elif finish:
            if len(frames) > 1:
                frames.pop()
        elif grp == "begingroup":
            frames.append([])
        elif grp == "endgroup":
            if len(frames) > 1:
                frames.pop()
        elif sec:
            title, i = group(text, i - 1)
            title = re.sub(r"\\[a-zA-Z]+\s*|[{}|]", "", title).strip()
            if sec == "section":
                section, subsection = title, ""
            else:
                subsection = title
        elif desc:
            item, i = component(text, i, desc)
            fill = ", fill=fillcol" if item["fillable"] else ""
            if desc == "circuitdescbip":
                draw = rf"\draw (0,0) to[{item['name']}, name=B{fill}] (2,0);"
            else:
                draw = rf"\draw (0,0) node[{item['name']},scale={item['scale']}{fill}](N){{{item['text']}}};"
            body = "\\begin{circuitikz}\n" + draw + "\n\\end{circuitikz}"
            yield {"id": f"manual:{line(at)}", "line": line(at), "section": section, "chapter": place(),
                   "kind": "path" if desc == "circuitdescbip" else "node", "component": item["name"],
                   "tex": document(macros, setup(), body)}
        else:
            try:
                statement, i = setup_statement(text, m.group(6), i)
            except (AttributeError, TypeError, ValueError):
                continue  # the name of a command in a sentence, not a statement
            frames[-1].append(statement)


def component(text, i, desc):
    """The arguments of `\\circuitdesc` (s O{1} m m m d() d[]) or `\\circuitdescbip` (s o m d<> m m d() d[])."""
    fillable, i = star(text, i)
    if desc == "circuitdesc":
        scale, j = group(text, i, "[", "]")
        if scale is not None:
            i = j
        name, i = group(text, i)
        _, i = group(text, i)  # description
        node_text, i = group(text, i)
        result = {"name": name.strip(), "scale": (scale or "1").strip(), "text": node_text, "fillable": fillable}
    else:
        main, j = group(text, i, "[", "]")
        if main is not None:
            i = j
        name, i = group(text, i)
        shape, j = group(text, i, "<", ">")
        if shape is not None:
            i = j
        _, i = group(text, i)  # description
        _, i = group(text, i)  # aliases
        result = {"name": name.strip(), "fillable": fillable}
    for o, c in (("(", ")"), ("[", "]")):  # anchors and internal nodes
        arg, j = group(text, i, o, c)
        if arg is not None:
            i = j
    return result, i


def main():
    doc, out = Path(sys.argv[1]), sys.argv[2]
    manual = (doc / "circuitikzmanual.tex").read_text(encoding="utf-8")
    utils = (doc / "ctikzmanutils.sty").read_text(encoding="utf-8")
    found = list(items(manual, utils))
    with open(out, "w", encoding="utf-8") as f:
        for e in found:
            f.write(json.dumps(e, ensure_ascii=False) + "\n")
    by_kind = {}
    for e in found:
        by_kind[e["kind"]] = by_kind.get(e["kind"], 0) + 1
    for k, n in sorted(by_kind.items()):
        print(f"{k:8} {n}")
    print(f"total    {len(found)}")


if __name__ == "__main__":
    main()
