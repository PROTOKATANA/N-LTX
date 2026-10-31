#!/bin/sh
# How far tikra is from TikZ: draws every example of the TikZ and pgfplots manuals with pdflatex and
# with tikra, compares them and writes docs/conformance.md (summary) and target/conformance/report.html
# (every figure side by side). See tools/conformance/README.md.
#
#   tools/conformance/run.sh            the whole run (the TeX references are cached by content)
#   tools/conformance/run.sh --tikra    only redraw with tikra and compare, reusing the references
#
# Needs Docker (for TeX Live), Python 3 with numpy, scipy and Pillow, and cargo. The first run pulls two
# TeX Live images (about 5 GB) and compiles some 3,000 examples: a few minutes on 8 cores.
set -e
cd "$(dirname "$0")/../.."
# the versions inside the TeX Live image, so the manual's examples are the ones that TeX knows
PGF=3.1.11a
PGFPLOTS=1.18.2
# CONFORMANCE_WORK: another working directory (with `ref` and `src` linked from the main one), so
# that several checkouts can measure at once
work=${CONFORMANCE_WORK:-target/conformance}
src=$work/src
mkdir -p "$src"
[ -d "$src/pgf-$PGF" ] || curl -sfL "https://github.com/pgf-tikz/pgf/archive/refs/tags/$PGF.tar.gz" | tar xz -C "$src"
[ -d "$src/pgfplots-$PGFPLOTS" ] || curl -sfL "https://github.com/pgf-tikz/pgfplots/archive/refs/tags/$PGFPLOTS.tar.gz" | tar xz -C "$src"

python3 -I tools/conformance/extract.py "$src/pgf-$PGF" "$src/pgfplots-$PGFPLOTS" "$work/examples.jsonl"
if [ "${1:-}" != "--tikra" ]; then
  python3 -I tools/conformance/reference.py "$work/examples.jsonl" "$src/pgf-$PGF" "$src/pgfplots-$PGFPLOTS" "$work"
  python3 -I tools/conformance/timing.py "$work/examples.jsonl" "$src/pgf-$PGF" "$src/pgfplots-$PGFPLOTS" "$work"
fi

# tikra reads the same documents
rm -rf "$work/tikra-in" "$work/tikra"
python3 -I -c '
import hashlib, json, sys, pathlib
out = pathlib.Path(sys.argv[2]); out.mkdir(parents=True)
for line in open(sys.argv[1], encoding="utf-8"):
    tex = json.loads(line)["tex"]
    (out / (hashlib.sha1(tex.encode()).hexdigest()[:16] + ".tex")).write_text(tex, encoding="utf-8")
' "$work/examples.jsonl" "$work/tikra-in"
# data files (`table {plotdata/…}`) are read from the manuals' doc folders, as pdflatex compiled them
TIKRA_INPUT_DIR="$PWD/$src/pgfplots-$PGFPLOTS/doc/latex/pgfplots:$PWD/$src/pgf-$PGF/doc/generic/pgf" \
  cargo run -q --release -p tikra --example conformance -- "$work/tikra-in" "$work/tikra"

python3 -I tools/conformance/compare.py "$work/examples.jsonl" "$work" --docs docs/conformance.md
echo "Side by side: $work/report.html"
