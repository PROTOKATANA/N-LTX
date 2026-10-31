# Braille

Every conversion also writes the equation in braille, as Unicode braille cells (U+2800–U+28FF): `Output::braille` (Rust), `braille` in `render()` (JavaScript and Python), `--braille` (CLI), and `<mathra:braille code="…">` in the SVG (S-SVG-9). It is built from the tree of meaning, so LaTeX, MathML and AsciiMath give the same braille.

Two codes, chosen by the language unless `braille` (`--braille-code`) says otherwise:

| Code | Used in | Default for |
|---|---|---|
| **CMU**, Código Matemático Unificado | Brazil, Portugal, the Spanish-speaking countries | `pt-BR`, `es` |
| **Nemeth** Braille Code for Mathematics | United States, Canada | every other language |

> The braille is meant for blind readers. Each sign below comes from the official document and is checked by a test with the source's own examples (`crates/core/tests/braille.rs`). Even so, it should be reviewed by a braille transcriber before it is put in front of students; the open points are listed at the end.

## Sources

- **Nemeth**: BANA, *The Nemeth Braille Code for Mathematics and Science Notation*, 2022 edition (brailleauthority.org), which replaces the 1972 revision. Its braille is printed in North American Braille ASCII and was converted to dots with the standard table. Section numbers (§) in `output/braille/nemeth.rs` refer to it. The October 2025 errata were not reviewed.
- **CMU**: *Código Matemático Unificado para a Língua Portuguesa*, MEC/SEESP, 1st ed., 2006 (ISBN 978-85-60331-04-8), from the braille-ready transcription by the Instituto Benjamin Constant (the MEC PDF was not reachable). Its braille is written in Braille Fácil ASCII and was decoded with the dot descriptions the document gives itself. Each sign was cross-checked with the Consejo Iberoamericano del Braille, *Código matemático unificado para Iberoamérica*, 2nd ed., 2024. Section numbers refer to the 2006 edition.

## Nemeth

| Construction | Rule |
|---|---|
| Numbers | Digits in the lower part of the cell, decimal point ⠨. The numeric indicator ⠼ at the start, after a space, a punctuation mark, or a minus that follows a space, and on every number of a matrix; not inside an enclosed list (§3) |
| Letters | Capitals ⠠, Greek ⠨ (alternative forms ⠨⠈), bold ⠸⠰, double-struck ⠠⠸⠰, script ⠈⠰. The English-letter indicator ⠰ before a single letter that stands alone, except next to a comparison sign, after a function name, in a matrix or an enclosed list (§6) |
| Operations and comparisons | Comparison signs with a space on each side; negation is ⠌ before the sign (§20–22) |
| Fractions | Simple ⠹ ⠌ ⠼, complex and hypercomplex with ⠠ per order, mixed numbers ⠸⠹ ⠸⠼, binomial coefficient with ⠩ (§13, §15.6) |
| Levels | Superscript ⠘, subscript ⠰, nested levels spelled from the baseline, return with ⠐ (not before a space or punctuation); a numeric subscript of a letter needs no indicator; primes ⠄ before the scripts; ⠪ as a comma at script level (§14) |
| Radicals | ⠣ index ⠜ … ⠻, inner radicals with ⠨ per depth (§16) |
| Functions | The name's letters followed by a space (§18) |
| Modified expressions | ⠐ expression ⠩ under ⠣ over ⠻ for Σ, Π, lim, accents and stacked labels; contracted bar x̄ = ⠭⠱ (§15); integrals with their bounds as scripts (§23) |
| Matrices and systems | A braille line per row, enlarged grouping signs on every line, a blank cell between entries (§19.6, §25.8) |

## CMU

| Construction | Rule |
|---|---|
| Numbers | ⠼ and the digits as the letters a–j; decimal comma ⠂; thousands point ⠄ (§3.1–3.2) |
| Fractions | A numeric fraction writes the numerator in the lower part of the cell after ⠼ and the denominator in the upper part: 3/4 = ⠼⠒⠙ (§3.2.2). Other fractions: numerator ⠲ denominator (§5.1) |
| Auxiliary parentheses | ⠢ … ⠔ around a numerator, denominator, exponent, index, radicand or marked expression with more than one term, unless it is already in ( ) [ ] { } (§1.3) |
| Letters | Capitals ⠨, Greek ⠈ and Greek capitals ⠘ (§1.2), other typographic variants ⠠ / ⠰, number sets ⠸ + letter (§3.5); a letter a–j right after a number takes ⠐ (§1.1) |
| Operations and relations | No spaces; negation is ⠘ before the relation (§4.3); set signs (§6) |
| Powers and indices | Exponent ⠡, subscript ⠌, the subscript first; primes ⠳ (§2, §5.2) |
| Roots | ⠫ index ⠱ radicand; the square root omits the index (§5.3) |
| Functions | Brazilian names (sen, tg, cossec, sh…) ended by ⠄; log base: log⠄ base ⠱; lim⠄ condition ⠱ (§7.2, §7.5) |
| Σ, Π, ∫ | The sign, the lower bound, ⠒, the upper bound, ⠱ (§7.4, §7.5.1) |
| Marks | Written before the letter: bar ⠈⠉, vector ⠒⠂, left arrow ⠐⠒, tilde ⠐⠢, arc ⠈⠒, dot ⠈ (§2.3.2, §8.1) |
| Matrices | A line per row, in the print positions (§7.5.1) |

## S-BRL-1 · A symbol without a sign

A symbol the code has no sign for (or that mathra does not know in that code yet) is written as the full cell ⠿, and the conversion records a note `S-BRL-1` naming the symbol, in `applied` (it is not a problem with the input, so it is not a warning).

## Open points

The comparison with MathCAT (`docs/braille-review.md`, `tools/braille-compare/`) settled several of the points this section used to list: ≈ (the double tilde of the Rule 21 table), ⇒/⟹ with two shaft cells (Ex. 22-19), the enlarged double bar ⠠⠳⠠⠳ (Rule 19 table), floor and ceiling (§19.4). What is still open:

- **Inferred from a rule, with no printed example.** Nemeth: ⊗ as the circle with an interior cross (§15.10 applied to the cross), ⪯ ⪰ as a bar under the curved comparison signs (§21.9), \ddot as two dot modifiers, \boxed as the rectangle shape with the expression inside (§15.10), ∭ as three integral signs. CMU: the degree sign ⠴ (only in the 2024 Iberoamerican edition), nabla ⠈⠻ (the 2006 text has `^ï`; the CIB 2024 and MathCAT print 4-12456), the brace that opens each line of a system (cases are not covered by the 2006 edition), a limit under max/min as the "subscrito" ⠌⠌.
- **No sign in the source** (they come out as ⠿ with S-BRL-1): Nemeth: ¬, ⋮, ⋱, ↦, ⊤, ⊢, ⊨, ⋉, ⋊, ⋈, ℓ, a brace with a label (§19.2: it must be drawn). CMU: the hat, cancellation, a framed expression, horizontal braces and brackets, floor and ceiling, ≅ (§7.6 leaves the tilde-like relations to the author), ↑ ↓, ≺ ≻, labeled arrows.
- **Questions for a transcriber**, listed with what mathra does now in `docs/braille-review.md`.
- **Context the tree does not keep.** Print spacing is lost in the tree, so Nemeth spaces follow the usual layout (after a comma, around comparison signs). Words inside an equation are written letter by letter, without contractions. Nemeth's rules for a letter inside a print sentence (UEB context, opening and closing Nemeth indicators) are outside the equation and not applied.
