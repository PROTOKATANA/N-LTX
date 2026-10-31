"""Draws every item with real TeX (pdflatex with CircuiTikZ 1.8.7, in a pinned TeX Live image), as the reference.

    python3 reference.py <examples.jsonl> <work dir> [--jobs N] [--image IMAGE]

For each item, <work dir>/ref/<hash>.png (100 dpi, white background) and <hash>.json (whether TeX
compiled it and how long pdflatex took). The hash is that of the document, so an unchanged item is
never compiled twice.
"""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

# texlive/texlive:latest of 2026-10, with CircuiTikZ 1.8.7 (2026/09/12), pinned so the reference does not move
IMAGE = "texlive/texlive@sha256:e8148b126545105c4aeeaa212f8a7bd02d05b500351372881055e1e59ae67d33"
DPI = 100

# Runs inside the container: one item per call.
COMPILE = r"""#!/bin/sh
h="$1"; job=/work/jobs/$h
cd $job
start=$(date +%s%N)
timeout 120 pdflatex -interaction=batchmode -halt-on-error doc.tex >/dev/null 2>&1
status=$?
# tikzmark's `pic anchor` places a pic by what the first run wrote to the .aux
if [ $status -eq 0 ] && grep -q 'pic anchor' doc.tex; then
  timeout 120 pdflatex -interaction=batchmode -halt-on-error doc.tex >/dev/null 2>&1
  status=$?
fi
end=$(date +%s%N)
ok=false
if [ $status -eq 0 ] && [ -f doc.pdf ]; then
  gs -q -dSAFER -dBATCH -dNOPAUSE -sDEVICE=png16m -r__DPI__ -dTextAlphaBits=4 -dGraphicsAlphaBits=4 -dFirstPage=1 -dLastPage=1 -o /work/ref/$h.png doc.pdf >/dev/null 2>&1 && ok=true
fi
error=$(grep -m1 -A2 '^!' doc.log 2>/dev/null | tr '\n' ' ' | tr -d '"\\' | cut -c1-300)
printf '{"ok": %s, "status": %d, "ms": %d, "error": "%s"}\n' $ok $status $(( (end - start) / 1000000 )) "$error" > /work/ref/$h.json
cd / && rm -rf $job
"""


def digest(tex):
    return hashlib.sha1(tex.encode("utf-8")).hexdigest()[:16]


def main():
    p = argparse.ArgumentParser()
    p.add_argument("examples")
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
        if (work / "ref" / f"{h}.json").exists() or h in todo:
            continue
        (jobs / h).mkdir(exist_ok=True)
        (jobs / h / "doc.tex").write_text(e["tex"], encoding="utf-8")
        todo.append(h)
    print(f"{len(todo)} items to compile with {a.image} ({a.jobs} at a time)", flush=True)
    if not todo:
        return
    (work / "compile.sh").write_text(COMPILE.replace("__DPI__", str(DPI)))
    (work / "todo.txt").write_text("\n".join(todo) + "\n")
    cmd = ["docker", "run", "--rm", "-v", f"{work}:/work", a.image,
           "sh", "-c", f"xargs -P {a.jobs} -n 1 sh /work/compile.sh < /work/todo.txt"]
    subprocess.run(cmd, check=True)
    shutil.rmtree(jobs, ignore_errors=True)
    done = sum(1 for h in todo if (work / "ref" / f"{h}.json").exists())
    print(f"compiled {done} of {len(todo)}")
    sys.exit(0 if done == len(todo) else 1)


if __name__ == "__main__":
    main()
