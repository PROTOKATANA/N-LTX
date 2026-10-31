# Braille review: mathra against MathCAT and the official documents

This is the first review of mathra's braille, before it goes to a human transcriber. It compared mathra with [MathCAT](https://github.com/daisy/MathCAT) (DAISY), the open-source reference for accessible math, on the same input. Wherever the two disagreed, the official document decided. It does not replace a transcriber's review; it narrows what a transcriber has to look at.

## Method

- **Input:** the 339 rows of `corpus/reference.json` (the reference page), turned into MathML by MathJax 3.2.2. mathra and MathCAT read the same MathML. 29 rows use commands only mathra knows (`\divchave`, `\SI`, `\ce` arrows MathJax draws in red) and were skipped.
- **MathCAT** 0.7.7-alpha.1, braille codes Nemeth and CMU. MathCAT's CMU follows the Spanish ONCE guide. mathra's follows the Brazilian *Código Matemático Unificado* (MEC/SEESP 2006), so CMU differences are expected where the two editions differ.
- **Sources:** for Nemeth, BANA, *The Nemeth Braille Code for Mathematics and Science Notation* (2022). For CMU, the 2006 edition as transcribed by the Instituto Benjamin Constant, with the CIB *Código matemático unificado para Iberoamérica* (2024) as a second reference.
- **Tool:** `tools/braille-compare/` (see its README) repeats the comparison.

## Results

| | Before the review | After | Different, explained below |
|---|---:|---:|---:|
| Nemeth, rows equal to MathCAT | 177 / 310 | 240 / 310 | 70 |
| CMU, rows equal to MathCAT | 163 / 310 | 177 / 310 | 133 |

## What the review fixed in mathra

Every change cites its source and has a test in `crates/core/tests/braille.rs` or `mathml.rs`.

**Nemeth**
- **Symbols from the Rule 20 and Rule 21 tables:**
  - operations: ∗ ⋆ ∖ ∘ ∧ ∨ ⊙ ⊕ ⊖ † ‡;
  - relations: ≺ ≻ ⊃ ⊇ ∋ ∥ ⊥ ≪ ≫ ≅ ≃ ≦ ≧;
  - arrows: ↑ ↓ ↕, the equilibrium arrows ⇌ ⥂ ⥄, ⇐;
  - negations by §21.1.
- **Other signs:**
  - half brackets for floor and ceiling (§19.4);
  - ∵ and ∴, spaced (§23.18);
  - signs of shape followed by a space (Rule 17);
  - ℏ, ℵ;
  - long arrows with three shaft cells (§22.5.3).
- **The space after a comparison keeps the level (§14.8.7).** `f(x)|ₓ₌₄` is now ⠳⠰⠭⠀⠰⠨⠅⠀⠼⠲, as in the source's example.
- **Systems and matrices:**
  - what precedes and follows a system goes on its top line, and the other rows align below it (Ex. 19-30);
  - `\left[ … \right]` around an array becomes enlarged brackets on every row.
- **Typeforms:**
  - German (Fraktur) letters take ⠸ (Ex. 6-2);
  - sans-serif takes ⠠⠨⠰;
  - bold Greek takes ⠸ and the Greek indicator, not the English-letter one;
  - the capital Φ is no longer written as an alternative form.
- **Modifiers:**
  - cancellation ⠪…⠻ (Rule 12);
  - contracted bar under a letter, ⠭⠩⠱ (§15.2.3);
  - horizontal braces and brackets without a label (Ex. 19-17 to 19-20);
  - modified comparison signs spaced as a whole;
  - a modified arrow in its uncontracted form.
- **Other:**
  - a word-sized space is a blank cell;
  - a unit is spaced from its number and takes no English-letter indicator (an abbreviation, §6.3.1 c);
  - a numeric subscript of an upright letter (H₂) needs no indicator;
  - the backslash ⠸⠡ (Rule 20);
  - the arc as a modifier (Ex. 15-54);
  - the degree sign at superscript level even when print does not raise it (§23.10);
  - the dash of ranges ⠤⠤, spaced (Rule 8, §20.1.1);
  - ⇌ with a label, in its own compounded form.

**CMU**
- **Left indices** (¹⁴₆C) are written after the letter, with ⠠⠌ and ⠈⠡ (§2.2).
- **Positions directly over and under:** `\overset` and `\underset` use the "sobrescrito" ⠡⠡ and the "subscrito" ⠌⠌ (§2.2), and so does a limit under max/min.
- **Symbols:**
  - ellipsis ⠄⠄⠄ (§3.2.1);
  - composition ⠠⠆ (§7.1);
  - "portanto" ⠠⠡ and "posto que" ⠈⠌, with a blank on each side (§6.3);
  - large ∧ ⠸⠢ and ∨ ⠸⠊ (§6.2);
  - alef ⠠⠳ and "equivale a" ⠐⠢⠄ (§6.1);
  - angle, triangle and square before the points (§8.1);
  - nabla.
- **Variant letters:** a capital in another typographic variant is ⠰ and the letter, without ⠨ (§1.1).
- **Set difference and negations:** the backslash as set difference ⠐⠄ (§6.1); ≁ and the negated double arrows by the ⠘ prefix (§4.3).
- **No more silent omissions:** cancellation, frames and horizontal braces have no sign in the 2006 code. They used to disappear; now they come out as ⠿ with the S-BRL-1 note.

**MathML reader** (bugs found because MathJax's MathML differs from hand-written MathML)
- Consecutive relations in one `<mo>≤≥≠</mo>` were read as a function name and lost.
- The no-break spaces of `<mtext>&#xA0;and&#xA0;</mtext>` were dropped.
- A multi-letter `<mi>` (`rank`) is a function name (MathML 3.2.3).
- `cases` with `columnalign` was not recognized.
- MathJax writes `\|` as the parallel sign ∥, so a pair of them around content is read as a norm.

## Where MathCAT is wrong, according to the source (Nemeth)

These could be reported to the MathCAT project.

| MathCAT writes | The source says | Reference |
|---|---|---|
| ⟹ with three shaft cells ⠫⠶⠶⠶⠕ | two: ⠫⠶⠶⠕ | Ex. 22-19 |
| Fraktur g as ⠸⠸⠛ | ⠸⠛ | Ex. 6-2 |
| Enlarged bracket as ⠠⠈⠷ | ⠈⠠⠷ | Rule 19 table |
| Matrices on one line, letters with the English-letter indicator | a braille line per row, no English-letter indicator in a matrix | §19.6, §6.4.4 |
| Enlarged braces only on the first line of a system | on every line | §19.6 |
| `\neg p` as ⠰⠏ (the sign is dropped) | — (no sign in the source; mathra writes ⠿ and a note) | |
| ⋉, ⋈, ⊨, ⋮, ⋱ spelled out as their Unicode code | — | |

## Expected differences in CMU (Spanish ONCE guide against the Brazilian 2006 edition)

- **Function names:** Spanish `sin`, `tan` against Brazilian `sen`, `tg`, `cossec`, `sh`.
- **Numeric fractions:** the 2006 edition writes the numerator in the lower part of the cell and the denominator in the upper part (3/4 = ⠼⠒⠙, §3.2.2). The newer Iberoamerican text reverses them.
- **Different signs:**

  | Symbol | mathra (Brazil 2006) | MathCAT (ONCE) |
  |---|---|---|
  | ∧ | ⠰⠂ | ⠸⠢ |
  | ∨ | ⠰⠄ | ⠸⠊ |
  | ≈ | ⠈⠶ | ⠐⠶⠄ |
  | → | ⠒⠂ | ⠒⠒⠂ |
  | · | ⠄ | ⠰⠄ |

  Alternative Greek forms are ⠈⠬ in ONCE and do not exist in 2006.
- **Layout:** the comma is followed by a blank in the 2006 examples (`x, y, z` = ⠭⠂⠀⠽⠂⠀⠵). Matrices and systems are written as print shows them, a line per row (§7.5.1).

## On the site

The reference page has a **Braille** tab (`site/reference.html#braille`) with the sources, these numbers, the signs inferred from a rule and a live list of the symbols each code writes as ⠿. In the other tabs, a braille cell with such a symbol is highlighted, and hovering over it gives the reason. The playground's Braille tab links there.

## Questions for a transcriber

What mathra does now is in brackets.

**Nemeth**
1. A single letter between grouping signs, `(x)`: [⠷⠭⠾]. §6.3.1 asks for a space or punctuation on both sides; MathCAT writes ⠷⠰⠭⠾.
2. A word in bold or double-struck letters (`\mathbf{AB}`): [a typeform indicator before each letter]. §7 also has single-word typeform indicators.
3. Bold italic Greek (`\boldsymbol\alpha`): [⠸⠨⠁]. MathCAT writes ⠸⠨⠨⠁.
4. The signs inferred from a rule (⊗, ⪯ ⪰, \ddot, \boxed, ∭), listed in `spec/braille.md`.
5. Chemistry: H₂O as ⠠⠓⠆⠠⠕ (a numeric subscript of an upright letter, no indicator) and bonds ⠸⠇ (≡) spaced as comparison signs. Is the transcription of chemistry in Nemeth the right model, or should it follow the BANA chemistry guidelines?

**CMU**
1. The signs the 2006 edition does not have and mathra writes as ⠿: hat, cancellation, framed expression, horizontal braces, floor and ceiling, ≅, ↑ ↓, ≺ ≻, labeled arrows. Is there a Brazilian practice for them? Should mathra take the CIB 2024 signs?
2. Systems (`cases`): [⠐⠇ at the start of each line]. The 2006 edition does not cover them.
3. A limit under max/min: [the "subscrito" ⠌⠌, as for a mark directly under].
4. The degree sign: [⠴, from the CIB 2024].
5. The evaluation bar `|ₓ₌₀`: [⠸, a blank, then the subscript]. Is the blank right before an index?
6. Nabla: [⠈⠻]. Does the `ï` of the 2006 text mean 12456?
7. Do variant letters inside a word (`\mathbb{AaBbCc}`) each take ⠠ or ⠰ [yes]? Is a double-struck C inside a word still ℂ ⠸⠉ [it is, today]?
