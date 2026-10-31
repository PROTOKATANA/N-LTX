#!/bin/sh
# The reference: draws figures with CircuiTikZ itself, in TeX Live, as SVG (exact coordinates, from
# dvisvgm) and PNG (pdflatex and Ghostscript, at 200 dpi). It is what voltik is compared against; the
# behavior to implement comes from the CircuiTikZ manual.
#
#   tools/oracle/tex.sh corpus/resistor.tex [more.tex...]   writes tools/oracle/out/<name>.svg and .png
#
# Each file is a figure (a circuitikz or tikzpicture environment, with \ctikzset lines before it if
# needed), wrapped here in a standalone document. Needs Docker.
set -e
cd "$(dirname "$0")/../.."
# texlive:latest carries CircuiTikZ 1.8.7, the version of the manual
IMAGE=${VOLTIK_TEXLIVE:-texlive/texlive:latest}
out=tools/oracle/out
mkdir -p "$out/work"
for f in "$@"; do
  name=$(basename "$f" .tex)
  {
    printf '%s\n' '\documentclass[border=2pt]{standalone}' '\usepackage[siunitx]{circuitikz}' '\begin{document}'
    cat "$f"
    printf '%s\n' '\end{document}'
  } > "$out/work/$name.tex"
done
docker run --rm -v "$PWD/$out:/out" -w /out/work "$IMAGE" sh -c '
  for t in *.tex; do
    n=${t%.tex}
    latex -interaction=nonstopmode -halt-on-error "$t" > "$n.log" 2>&1 || { echo "TeX failed: $n (see tools/oracle/out/work/$n.log)"; continue; }
    dvisvgm --no-fonts --exact-bbox -o "/out/$n.svg" "$n.dvi" > /dev/null 2>&1
    pdflatex -interaction=nonstopmode -halt-on-error "$t" > /dev/null 2>&1 && gs -q -sDEVICE=png16m -r200 -o "/out/$n.png" "$n.pdf" || echo "no PNG: $n"
  done'
