"""How long pdflatex takes per figure, natively and one at a time, for the speed table of the report.

    python3 timing.py <examples.jsonl> <pgf source dir> <pgfplots source dir> <work dir> [--image IMAGE] [--sample N]

The reference run (reference.py) compiles many examples at once, possibly emulated (the pinned image is
amd64 only), so its times do not say how long a user waits. This compiles a fixed sample of the TikZ
figures one after the other, in a TeX Live image native to this machine, and writes <work dir>/timing.json.
"""

import argparse
import hashlib
import json
import subprocess
from pathlib import Path

SCRIPT = r"""
for h in $(cat /work/timing/todo.txt); do
  job=/work/timing/$h
  case "$(cat $job/manual)" in tikz) cd /pgfdoc ;; *) cd /plotsdoc ;; esac
  start=$(date +%s%N)
  timeout 120 pdflatex -interaction=batchmode -halt-on-error -output-directory=$job $job/doc.tex >/dev/null 2>&1
  status=$?
  end=$(date +%s%N)
  echo "$h $status $(( (end - start) / 1000000 ))" >> /work/timing/times.txt
done
"""


def main():
    p = argparse.ArgumentParser()
    p.add_argument("examples")
    p.add_argument("pgf")
    p.add_argument("pgfplots")
    p.add_argument("work")
    p.add_argument("--image", default="texlive/texlive:latest")
    p.add_argument("--sample", type=int, default=200)
    a = p.parse_args()
    work = Path(a.work).resolve()
    figures = [e for e in map(json.loads, open(a.examples, encoding="utf-8")) if e["kind"] == "tikz"]
    step = max(1, len(figures) // a.sample)
    sample = figures[::step][: a.sample]
    d = work / "timing"
    subprocess.run(["rm", "-rf", str(d)], check=True)
    d.mkdir(parents=True)
    hashes = []
    for e in sample:
        h = hashlib.sha1(e["tex"].encode("utf-8")).hexdigest()[:16]
        (d / h).mkdir(exist_ok=True)
        (d / h / "doc.tex").write_text(e["tex"], encoding="utf-8")
        (d / h / "manual").write_text(e["manual"])
        hashes.append(h)
    (d / "todo.txt").write_text("\n".join(hashes) + "\n")
    pgf_doc = Path(a.pgf).resolve() / "doc/generic/pgf"
    plots_doc = Path(a.pgfplots).resolve() / "doc/latex/pgfplots"
    subprocess.run(["docker", "run", "--rm", "-v", f"{work}:/work", "-v", f"{pgf_doc}:/pgfdoc:ro", "-v", f"{plots_doc}:/plotsdoc:ro",
                    a.image, "sh", "-c", SCRIPT], check=True)
    times = {}
    for line in (d / "times.txt").read_text().split("\n"):
        if line:
            h, status, ms = line.split()
            if status == "0":
                times[h] = int(ms)
    machine = subprocess.run(["uname", "-sm"], capture_output=True, text=True).stdout.strip()
    (work / "timing.json").write_text(json.dumps({"image": a.image, "machine": machine, "ms": times}, indent=1))
    subprocess.run(["rm", "-rf", str(d)], check=True)
    print(f"timed {len(times)} of {len(sample)} figures with {a.image}")


if __name__ == "__main__":
    main()
