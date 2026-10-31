"""Draws every example with real TeX (pdflatex in a pinned TeX Live image), as the reference.

    python3 reference.py <examples.jsonl> <pgf source dir> <pgfplots source dir> <work dir> [--jobs N] [--image IMAGE]

For each example, <work dir>/ref/<hash>.png (100 dpi, white background) and <hash>.json (whether TeX
compiled it and how long pdflatex took). The hash is that of the document, so an unchanged example is
never compiled twice. Each example is compiled from its manual's doc folder, where the data files and
images it reads live.
"""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

IMAGE = "texlive/texlive:TL2025-historic"
DPI = 100

# Runs inside the container: one example per call, from its manual's folder.
COMPILE = r"""#!/bin/sh
h="$1"; job=/work/jobs/$h
case "$(cat $job/manual)" in tikz) cd /pgfdoc ;; *) cd /plotsdoc ;; esac
start=$(date +%s%N)
timeout 120 pdflatex -interaction=batchmode -halt-on-error -output-directory=$job $job/doc.tex >/dev/null 2>&1
status=$?
end=$(date +%s%N)
ok=false
if [ $status -eq 0 ] && [ -f $job/doc.pdf ]; then
  gs -q -dSAFER -dBATCH -dNOPAUSE -sDEVICE=png16m -r__DPI__ -dTextAlphaBits=4 -dGraphicsAlphaBits=4 -dFirstPage=1 -dLastPage=1 -o /work/ref/$h.png $job/doc.pdf >/dev/null 2>&1 && ok=true
fi
error=$(grep -m1 -A2 '^!' $job/doc.log 2>/dev/null | tr '\n' ' ' | tr -d '"\\' | cut -c1-300)
printf '{"ok": %s, "status": %d, "ms": %d, "error": "%s"}\n' $ok $status $(( (end - start) / 1000000 )) "$error" > /work/ref/$h.json
rm -rf $job
"""


def digest(tex):
    return hashlib.sha1(tex.encode("utf-8")).hexdigest()[:16]


def main():
    p = argparse.ArgumentParser()
    p.add_argument("examples")
    p.add_argument("pgf")
    p.add_argument("pgfplots")
    p.add_argument("work")
    p.add_argument("--jobs", type=int, default=os.cpu_count() or 4)
    p.add_argument("--image", default=IMAGE)
    a = p.parse_args()

    work = Path(a.work).resolve()
    (work / "ref").mkdir(parents=True, exist_ok=True)
    jobs = work / "jobs"
    shutil.rmtree(jobs, ignore_errors=True)
    jobs.mkdir()
    todo = []
    for line in open(a.examples, encoding="utf-8"):
        e = json.loads(line)
        h = digest(e["tex"])
        if (work / "ref" / f"{h}.json").exists():
            continue
        (jobs / h).mkdir(exist_ok=True)
        (jobs / h / "doc.tex").write_text(e["tex"], encoding="utf-8")
        (jobs / h / "manual").write_text(e["manual"])
        todo.append(h)
    print(f"{len(todo)} examples to compile with {a.image} ({a.jobs} at a time)", flush=True)
    if not todo:
        return
    (work / "compile.sh").write_text(COMPILE.replace("__DPI__", str(DPI)))
    (work / "todo.txt").write_text("\n".join(todo) + "\n")
    pgf_doc = Path(a.pgf).resolve() / "doc/generic/pgf"
    plots_doc = Path(a.pgfplots).resolve() / "doc/latex/pgfplots"
    cmd = [
        "docker", "run", "--rm",
        "-v", f"{work}:/work", "-v", f"{pgf_doc}:/pgfdoc:ro", "-v", f"{plots_doc}:/plotsdoc:ro",
        a.image, "sh", "-c", f"xargs -P {a.jobs} -n 1 sh /work/compile.sh < /work/todo.txt",
    ]
    subprocess.run(cmd, check=True)
    shutil.rmtree(jobs, ignore_errors=True)
    done = sum(1 for h in todo if (work / "ref" / f"{h}.json").exists())
    print(f"compiled {done} of {len(todo)}")
    sys.exit(0 if done == len(todo) else 1)


if __name__ == "__main__":
    main()
