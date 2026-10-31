#!/bin/sh
# How far voltik is from CircuiTikZ, measured by its manual: draws every example and every component
# entry of the CircuiTikZ manual with pdflatex and with voltik, compares them and writes
# docs/conformance.md (summary) and target/conformance/report.html (every figure side by side).
# See tools/conformance/README.md.
#
#   tools/conformance/run.sh            the whole run (the TeX references are cached by content)
#   tools/conformance/run.sh --voltik   only redraw with voltik and compare, reusing the references
#
# Needs Docker (for TeX Live), Python 3 with numpy, scipy and Pillow, and cargo.
set -e
cd "$(dirname "$0")/../.."
# the CircuiTikZ inside the TeX Live image, so the manual's examples are the ones that TeX knows
CIRCUITIKZ=1.8.7
work=target/conformance
doc=$work/src/circuitikz-$CIRCUITIKZ/doc
mkdir -p "$work/src"
# only the manual: circuitikzmanual.tex and its macros, ctikzmanutils.sty (nothing of the package's code)
[ -f "$doc/circuitikzmanual.tex" ] || curl -sfL "https://github.com/circuitikz/circuitikz/archive/refs/tags/v$CIRCUITIKZ.tar.gz" |
  tar xz -C "$work/src" "circuitikz-$CIRCUITIKZ/doc/circuitikzmanual.tex" "circuitikz-$CIRCUITIKZ/doc/ctikzmanutils.sty"

python3 -I tools/conformance/extract.py "$doc" "$work/examples.jsonl"
if [ "${1:-}" != "--voltik" ]; then
  python3 -I tools/conformance/reference.py "$work/examples.jsonl" "$work"
fi

# voltik reads the same documents
rm -rf "$work/voltik-in" "$work/voltik"
python3 -I -c '
import hashlib, json, sys, pathlib
out = pathlib.Path(sys.argv[2]); out.mkdir(parents=True)
for line in open(sys.argv[1], encoding="utf-8"):
    tex = json.loads(line)["tex"]
    (out / (hashlib.sha1(tex.encode()).hexdigest()[:16] + ".tex")).write_text(tex, encoding="utf-8")
' "$work/examples.jsonl" "$work/voltik-in"
cargo run -q --release -p voltik --example conformance -- "$work/voltik-in" "$work/voltik"

python3 -I tools/conformance/compare.py "$work/examples.jsonl" "$work" --docs docs/conformance.md
echo "Side by side: $work/report.html"
