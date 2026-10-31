# Conformance with TikZ

How far tikra is from TikZ itself: every example of the TikZ manual (pgf 3.1.11a) and of the pgfplots
manual (1.18.2), drawn by pdflatex (TeX Live 2025) and by tikra, and compared. How it works and how
to run it: [tools/conformance](../tools/conformance/README.md).

**2371 of 2502 figures (94.8%) match TeX**; 117 are close, 14 differ and 0 fail in tikra. 126 of the ones that do not match came out with no warning.

**Our 100%: 2480 of 2486 (99.8%)**. It leaves out the 16 figures no converter can reproduce (9 with random numbers TeX draws anew on every run, 3 with an image file or external program, 4 with the day's date), and counts as done 112 that differ only by antialiasing. The list, checked by eye: [unreproducible.json](../tools/conformance/unreproducible.json).

| | Figures | Match | Close | Different | Fails | Mean F1 |
|---|---:|---:|---:|---:|---:|---:|
| All | 2502 | 2371 (94.8%) | 117 (4.7%) | 14 (0.6%) | 0 (0.0%) | 0.99 |
| TikZ manual | 1864 | 1745 (93.6%) | 109 (5.8%) | 10 (0.5%) | 0 (0.0%) | 0.98 |
| pgfplots manual | 638 | 626 (98.1%) | 8 (1.3%) | 4 (0.6%) | 0 (0.0%) | 0.99 |

*Match*: same shape (F1 ≥ 0.9 on the edges, with 0.5 mm of tolerance), no missing or extra piece, the same colors and size. *Close*: F1 ≥ 0.75.
*Fails*: tikra returned an error, panicked or took more than 10 s. Mean F1 counts a failure as 0.

Not counted: 137 where tex could not compile it, 19 where tex draws nothing visible, and 479 examples that are not figures (text, tables, key listings).

## Speed

The same 192 figures (a fixed sample of the manuals), one at a time, on Darwin arm64:

| | Median per figure | p95 |
|---|---:|---:|
| pdflatex (texlive/texlive:latest) | 647 ms | 1031 ms |
| tikra, TikZ to SVG | 0.83 ms | 7.06 ms |

Median ratio: tikra is **843× faster**. pdflatex time includes starting TeX and loading the libraries of the manual, which is what a user
waits for; tikra's is the conversion inside a running process: the first figure of a process also builds the engine, 18.8 ms more.

## By chapter

Sorted by how many figures do not match, the work left.

| Chapter | Figures | Match | Close | Different | Fails | Mean F1 |
|---|---:|---:|---:|---:|---:|---:|
| Shape Library <sub>tikz</sub> | 97 | 75 (77.3%) | 22 (22.7%) | 0 (0.0%) | 0 (0.0%) | 0.97 |
| Transparency <sub>tikz</sub> | 47 | 31 (66.0%) | 13 (27.7%) | 3 (6.4%) | 0 (0.0%) | 0.91 |
| Specifying Coordinates <sub>tikz</sub> | 68 | 56 (82.4%) | 12 (17.6%) | 0 (0.0%) | 0 (0.0%) | 0.96 |
| Coordinate, Canvas, and Nonlinear Transformations <sub>tikz</sub> | 38 | 29 (76.3%) | 9 (23.7%) | 0 (0.0%) | 0 (0.0%) | 0.93 |
| Circuit Libraries <sub>tikz</sub> | 64 | 56 (87.5%) | 6 (9.4%) | 2 (3.1%) | 0 (0.0%) | 0.97 |
| Syntax for Path Specifications <sub>tikz</sub> | 52 | 47 (90.4%) | 5 (9.6%) | 0 (0.0%) | 0 (0.0%) | 0.98 |
| Actions on Paths <sub>tikz</sub> | 72 | 68 (94.4%) | 4 (5.6%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Constructing Paths <sub>tikz</sub> | 26 | 23 (88.5%) | 3 (11.5%) | 0 (0.0%) | 0 (0.0%) | 0.97 |
| Axes <sub>tikz</sub> | 78 | 75 (96.2%) | 3 (3.8%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Decoration Library <sub>tikz</sub> | 81 | 78 (96.3%) | 3 (3.7%) | 0 (0.0%) | 0 (0.0%) | 0.97 |
| Pattern Library <sub>tikz</sub> | 8 | 5 (62.5%) | 3 (37.5%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Transformations <sub>tikz</sub> | 22 | 19 (86.4%) | 3 (13.6%) | 0 (0.0%) | 0 (0.0%) | 0.96 |
| Tutorial: A Picture for Karl's Students <sub>tikz</sub> | 46 | 43 (93.5%) | 3 (6.5%) | 0 (0.0%) | 0 (0.0%) | 0.98 |
| Smith Charts <sub>pgfplots</sub> | 12 | 9 (75.0%) | 3 (25.0%) | 0 (0.0%) | 0 (0.0%) | 0.96 |
| Axis Descriptions <sub>pgfplots</sub> | 77 | 74 (96.1%) | 2 (2.6%) | 1 (1.3%) | 0 (0.0%) | 0.99 |
| Using Paths <sub>tikz</sub> | 15 | 13 (86.7%) | 2 (13.3%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Declaring and Using Images <sub>tikz</sub> | 2 | 0 (0.0%) | 1 (50.0%) | 1 (50.0%) | 0 (0.0%) | 0.45 |
| Nodes and Shapes <sub>tikz</sub> | 12 | 10 (83.3%) | 2 (16.7%) | 0 (0.0%) | 0 (0.0%) | 0.96 |
| Quick Commands <sub>tikz</sub> | 3 | 1 (33.3%) | 2 (66.7%) | 0 (0.0%) | 0 (0.0%) | 0.87 |
| Style Sheets and Legends <sub>tikz</sub> | 85 | 83 (97.6%) | 2 (2.4%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Lindenmayer System Drawing Library <sub>tikz</sub> | 6 | 4 (66.7%) | 2 (33.3%) | 0 (0.0%) | 0 (0.0%) | 0.96 |
| Additional Mathematical Commands <sub>tikz</sub> | 3 | 1 (33.3%) | 0 (0.0%) | 2 (66.7%) | 0 (0.0%) | 0.70 |
| Animations <sub>tikz</sub> | 24 | 23 (95.8%) | 1 (4.2%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Creating Plots <sub>tikz</sub> | 7 | 6 (85.7%) | 0 (0.0%) | 1 (14.3%) | 0 (0.0%) | 0.96 |
| Polar Axes <sub>tikz</sub> | 11 | 10 (90.9%) | 1 (9.1%) | 0 (0.0%) | 0 (0.0%) | 0.98 |
| Automata Drawing Library <sub>tikz</sub> | 8 | 7 (87.5%) | 1 (12.5%) | 0 (0.0%) | 0 (0.0%) | 0.98 |
| Calendar Library <sub>tikz</sub> | 41 | 40 (97.6%) | 0 (0.0%) | 1 (2.4%) | 0 (0.0%) | 0.99 |
| Fitting Library <sub>tikz</sub> | 4 | 3 (75.0%) | 1 (25.0%) | 0 (0.0%) | 0 (0.0%) | 0.98 |
| Math Library <sub>tikz</sub> | 6 | 5 (83.3%) | 1 (16.7%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Arrows <sub>tikz</sub> | 104 | 103 (99.0%) | 1 (1.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Decorated Paths <sub>tikz</sub> | 31 | 30 (96.8%) | 1 (3.2%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Tutorial: A Lecture Map for Johannes <sub>tikz</sub> | 10 | 9 (90.0%) | 1 (10.0%) | 0 (0.0%) | 0 (0.0%) | 0.98 |
| Tutorial: A Petri-Net for Hagen <sub>tikz</sub> | 28 | 27 (96.4%) | 1 (3.6%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Fill between <sub>pgfplots</sub> | 29 | 28 (96.6%) | 1 (3.4%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Statistics <sub>pgfplots</sub> | 26 | 25 (96.2%) | 0 (0.0%) | 1 (3.8%) | 0 (0.0%) | 0.99 |
| Two Dimensional Plot Types <sub>pgfplots</sub> | 72 | 71 (98.6%) | 0 (0.0%) | 1 (1.4%) | 0 (0.0%) | 0.99 |
| Grid Options <sub>pgfplots</sub> | 18 | 17 (94.4%) | 0 (0.0%) | 1 (5.6%) | 0 (0.0%) | 0.97 |
| Scaling Options <sub>pgfplots</sub> | 22 | 21 (95.5%) | 1 (4.5%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Specifying the Plotted Range <sub>pgfplots</sub> | 13 | 12 (92.3%) | 1 (7.7%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Decorations <sub>tikz</sub> | 10 | 10 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.96 |
| Design Principles <sub>tikz</sub> | 7 | 7 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Layered Graphics <sub>tikz</sub> | 1 | 1 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Matrices <sub>tikz</sub> | 7 | 7 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Patterns <sub>tikz</sub> | 4 | 4 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| pgfmanual-en-base-scopes <sub>tikz</sub> | 19 | 19 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.97 |
| Shadings <sub>tikz</sub> | 6 | 6 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| pgfmanual-en-dv-examples <sub>tikz</sub> | 3 | 3 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.97 |
| Providing Data for a Data Visualization <sub>tikz</sub> | 8 | 8 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Introduction to Data Visualization <sub>tikz</sub> | 2 | 2 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Creating Data Visualizations <sub>tikz</sub> | 13 | 13 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Visualizers <sub>tikz</sub> | 18 | 18 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Three Dimensional Drawing Library <sub>tikz</sub> | 5 | 5 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Angle Library <sub>tikz</sub> | 6 | 6 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Babel Library <sub>tikz</sub> | 2 | 2 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Background Library <sub>tikz</sub> | 12 | 12 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Chains <sub>tikz</sub> | 14 | 14 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| To Path Library <sub>tikz</sub> | 13 | 13 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Entity-Relationship Diagram Drawing Library <sub>tikz</sub> | 7 | 7 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Paper-Folding Diagrams Library <sub>tikz</sub> | 18 | 18 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Matrix Library <sub>tikz</sub> | 14 | 14 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Mindmap Drawing Library <sub>tikz</sub> | 16 | 16 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Three Point Perspective Drawing Library <sub>tikz</sub> | 15 | 15 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Petri-Net Drawing Library <sub>tikz</sub> | 13 | 13 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Plot Handler Library <sub>tikz</sub> | 23 | 23 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Resource Description Framework Library <sub>tikz</sub> | 2 | 2 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Shadings Library <sub>tikz</sub> | 11 | 11 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Shadows Library <sub>tikz</sub> | 14 | 14 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Spy Library: Magnifying Parts of Pictures <sub>tikz</sub> | 11 | 11 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| SVG-Path Library <sub>tikz</sub> | 1 | 1 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Through Library <sub>tikz</sub> | 1 | 1 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.96 |
| Tree Library <sub>tikz</sub> | 7 | 7 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Turtle Graphics Library <sub>tikz</sub> | 4 | 4 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Views Library <sub>tikz</sub> | 4 | 4 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Tutorials and Guidelines <sub>tikz</sub> | 9 | 9 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.98 |
| Customizing the Mathematical Engine <sub>tikz</sub> | 2 | 2 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.94 |
| Mathematical Expressions <sub>tikz</sub> | 3 | 3 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Date and Calendar Utility Macros <sub>tikz</sub> | 1 | 1 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Repeating Things: The Foreach Statement <sub>tikz</sub> | 13 | 13 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Key Management <sub>tikz</sub> | 5 | 5 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Animation System Layer <sub>tikz</sub> | 1 | 1 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Specifying Graphs <sub>tikz</sub> | 159 | 159 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Matrices and Alignment <sub>tikz</sub> | 26 | 26 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Pics: Small Pictures on Paths <sub>tikz</sub> | 14 | 14 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Plots of Functions <sub>tikz</sub> | 30 | 30 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| pgfmanual-en-tikz-scopes <sub>tikz</sub> | 13 | 13 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Nodes and Edges <sub>tikz</sub> | 118 | 118 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Making Trees Grow <sub>tikz</sub> | 27 | 27 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Tutorial: Euclid's Amber Version of the Elements <sub>tikz</sub> | 14 | 14 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Tutorial: Diagrams as Simple Graphs <sub>tikz</sub> | 19 | 19 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Utilities and Basic Level Commands <sub>pgfplots</sub> | 7 | 7 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Decoration: Soft Clipping <sub>pgfplots</sub> | 1 | 1 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Grouping plots <sub>pgfplots</sub> | 8 | 8 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Patchplots Library <sub>pgfplots</sub> | 29 | 29 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Polar Axes <sub>pgfplots</sub> | 13 | 13 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.94 |
| Ternary Diagrams <sub>pgfplots</sub> | 13 | 13 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.98 |
| Units in Labels <sub>pgfplots</sub> | 2 | 2 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Components <sub>pgfplots</sub> | 1 | 1 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.94 |
| 3D Axis Configuration <sub>pgfplots</sub> | 13 | 13 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Three Dimensional Plot Types <sub>pgfplots</sub> | 85 | 85 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Alignment Options <sub>pgfplots</sub> | 8 | 8 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| The Axis Environments <sub>pgfplots</sub> | 21 | 21 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| The Picture's Size: Bounding Box and Clipping <sub>pgfplots</sub> | 5 | 5 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| pgfplots.reference.closingplots <sub>pgfplots</sub> | 6 | 6 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Skipping Or Changing Coordinates -- Filters <sub>pgfplots</sub> | 7 | 7 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Error Bars <sub>pgfplots</sub> | 9 | 9 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Fitting Lines -- Regression <sub>pgfplots</sub> | 5 | 5 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Markers, Linestyles, (Background) Colors and Colormaps <sub>pgfplots</sub> | 50 | 50 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Miscellaneous Options <sub>pgfplots</sub> | 7 | 7 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Number Formatting Options <sub>pgfplots</sub> | 6 | 6 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Style Options <sub>pgfplots</sub> | 2 | 2 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Symbolic Coordinates and User Transformations <sub>pgfplots</sub> | 5 | 5 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Tick Options <sub>pgfplots</sub> | 32 | 32 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Interoperability <sub>pgfplots</sub> | 3 | 3 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Transforming Coordinate Systems <sub>pgfplots</sub> | 4 | 4 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.98 |
| Memory and Speed considerations <sub>pgfplots</sub> | 4 | 4 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.99 |
| Solving a Real Use Case: Function Visualization <sub>pgfplots</sub> | 6 | 6 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Solving a Real Use Case: Scientific Data Analysis <sub>pgfplots</sub> | 7 | 7 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.98 |
| Use Cases involving Scatter Plots <sub>pgfplots</sub> | 5 | 5 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 1.00 |
| Solving a Real Use Case: Functions of Two Variables <sub>pgfplots</sub> | 5 | 5 (100.0%) | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0.98 |

## What tikra reported

Warnings, by how many figures they appear in (the varying part replaced by …):

| Figures | Warning |
|---:|---|
| 11 | `option … not supported: ignored` |
| 3 | `text outside a statement ignored` |
| 3 | `E-REC-N column N: … without a matching …; closed at the end` |
| 2 | `image not drawn: file … not found` |
| 2 | `\draw: figure: unknown coordinate: (…); statement ignored` |
| 2 | `key handler … not supported (…): ignored` |
| 2 | `fill between: soft clip … not understood: ignored` |
| 2 | `axis without data: drew the default window from N to N` |
| 1 | `environment colormixin not supported: content read as statements` |
| 1 | `\pgfpathcurvebetweentime not supported: … ignored` |
| 1 | `\pgfplotgnuplot not supported: … ignored` |
| 1 | `\pgfpathcircle: figure: not a point: \pgfpointborderrectangle; statement ignored` |
| 1 | `\pgfpathcircle: figure: not a point: \pgfpointborderellipse; statement ignored` |
| 1 | `\pgfpathcircle: figure: not a point: \pgfpointintersectionofcircles; statement ignored` |
| 1 | `\pgfqbox{\mybox}: the box is not drawn` |
| 1 | `\endpgftransparencygroup not supported: … ignored` |
| 1 | `E-REC-N column N: unsupported text command: \pgftransparencygroup; written as is` |
| 1 | `\pgftransparencygroup not supported: … ignored` |
| 1 | `E-REC-N column N: unsupported text command: \end; written as is` |
| 1 | `E-REC-N column N: unsupported text command: \endpgftransparencygroup; written as is` |
| 1 | `unrecognized path fragment: …` |
| 1 | `E-REC-N column N: unsupported text command: \pgfsetfillopacity; written as is` |
| 1 | `E-REC-N column N: unsupported text command: \begin; written as is` |
| 1 | `E-REC-N column N: unsupported text command: \node; written as is` |
| 1 | `\pgfmathparse: expression: unexpected character: …` |
| 1 | `E-REC-N column N: unsupported text command: \-; written as is` |
| 1 | `unknown color …` |
| 1 | `soft clip path … not understood: path drawn unclipped` |

Options tikra does not know, in figures that do not match, by how many figures they appear in:

| Figures | Option |
|---:|---|

Commands, environments and path operations tikra does not know, likewise:

| Figures | Command |
|---:|---|
| 1 | `colormixin` |
| 1 | `\pgfplotgnuplot` |
| 1 | `\pgftransparencygroup` |
| 1 | `\endpgftransparencygroup` |

Errors of the figures that fail:

| Figures | Error |
|---:|---|
