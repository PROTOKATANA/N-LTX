# Corpus

Hand-written LaTeX constructions used by the benchmark (`cargo run --release -p mathra --example bench`).

| File | Content |
|---|---|
| `constructions.json` | 97 constructions (`[name, latex]`): macros, environments, packages, old plain TeX syntax, and a few things mathra leaves out on purpose (TeX programming, diagrams). |
| `packages.json` | 19 constructions, one per supported package or accent case. |
| `reference.json` | The tables of the reference page (`site/reference.html`): AsciiMath with its LaTeX equivalent, LaTeX by topic, the science packages and MathML. Not part of the benchmark; `crates/core/tests/reference.rs` checks every row. |
