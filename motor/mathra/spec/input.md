# Input notations

mathra reads three notations into the same tree of meaning: **LaTeX** (the default), **MathML** and **AsciiMath**. Everything after the tree (layout, speech, linear text, SVG) is shared, so the same equation gives the same drawing and the same phrase whatever notation it came in. The notation is chosen with `Options::input` (Rust), `input` (JavaScript and Python) or `--from` (CLI), and `<mathra:source type="…">` in the SVG says which one was read (S-SVG-3).

## MathML

Presentation MathML, as Word, Google Docs, LibreOffice, MathType and MathJax export it. The input is XML: it must be well formed in both modes, because there is no tree to recover from broken XML. The root may be `<math>` with or without a namespace prefix (`<m:math>`), an XML prolog or comments; any other element is read as the content of a `<math>`.

| Element | Becomes |
|---|---|
| `mi` | one letter: a variable, in italic (uppercase Greek upright, as in TeX); several letters: a function (`sin`, `lim`) or an upright word; `mathvariant` chooses the alphabet |
| `mn` | a number, with the locale's decimal comma and thousands separator (L-NUM-1, L-NUM-3) |
| `mo` | the character with its TeX class (`≰` a relation, `⊗` a binary operator); `∑ ∫ ∏…` large operators; words (`lim`, `max`) functions; invisible operators (U+2061–U+2064) draw nothing |
| `mtext`, `ms`, `mspace` | text; quoted text; a space (`em`, `ex`, `pt`, `px`, `mu` and the named spaces) |
| `mrow`, `mstyle`, `mpadded` | a row; `mstyle` carries `displaystyle`, `mathvariant`, `mathcolor` and `mathbackground` |
| `mfrac`, `msqrt`, `mroot` | fraction (`linethickness="0"` without the rule), roots |
| `msub`, `msup`, `msubsup`, `mmultiscripts` | scripts; prescripts (`<mprescripts/>`) on an empty base, as in isotopes |
| `munder`, `mover`, `munderover` | limits over a large operator or a function; an accent, a bar, an arrow, a brace or a bracket when the script is that single mark (`→` over a letter is the vector accent); a brace's label in the outer element, as MathJax writes it; anything else stacked |
| `mtable`, `mtr`, `mlabeledtr`, `mtd` | a table; `columnalign`, `columnlines` and `rowlines` are kept; the label of `mlabeledtr` goes at the end of its row |
| `mfenced`, fences in `mrow` | an `mrow` that starts with an opening fence and ends with a closing one stretches them (`\left…\right`) when the inside is balanced; around a table they make a matrix (`(` `[` `|` `‖` `{…}`), `{` alone makes `cases` and `}` alone `rcases` |
| `menclose` | `box`/`roundedbox`, `circle`, the strikes, `top`, `bottom`, `radical` |
| `semantics`, `maction`, `merror`, `mphantom` | the presentation child (annotations are ignored); the selected child; content in red; space without drawing |

### L-ENT-4 · MathML pasted as LaTeX

In LaTeX mode, an input that starts with a `<math>` element (after an optional prolog or comments) is read as MathML. LaTeX never starts that way, and editors put MathML on the clipboard.

### Entities

XML knows only `&lt;`, `&gt;`, `&amp;`, `&quot;`, `&apos;` and numeric references. MathML's named entities (`&InvisibleTimes;`, `&times;`, `&alpha;`, `&rarr;`…) become their characters before parsing. An unknown entity is an error in strict mode and is dropped with a warning (E-REC-10) in tolerant mode.

### Out of scope

Content MathML (`<apply>`, `<ci>`, `<cn>`) describes meaning without a drawing; it is reported as such. Elementary math (`<mlongdiv>`, `<mstack>`) is read as rows; for long division and column arithmetic there are `\divchave` and `\contaarmada`.

## AsciiMath

The notation of asciimath.org: `sum_(i=1)^n i^2 = (n(n+1)(2n+1))/6`.

- **Grammar.** An expression is a sequence of `I` or `I/I`; `I` is `S`, `S_S`, `S^S` or `S_S^S`; `S` is a symbol, a bracketed expression, `unary S` or `binary S S`. The brackets around a fraction's parts, a script and an argument are removed: `(a+b)/(c+d)` has no parentheses, `x^(2n)` neither, `f(x)` keeps them.
- **Symbols.** Each AsciiMath symbol is given by its LaTeX spelling (`xx` is `\times`, `!=` is `\ne`, `RR` is `\mathbb{R}`, `sum` is `\sum`) and read by the LaTeX reader, so both notations draw and read a symbol the same way. The tokenizer takes the longest symbol; a word that is not a symbol is read letter by letter (`speed` is s·p·e·e·d), as asciimath.org does.
- **Brackets.** `(` `[` `{` `(:` `<<` and the invisible `{:` open; any closing bracket closes, so `(a,b]` is a half-open interval. `|x|` is an absolute value when a matching bar follows.
- **Matrices.** A bracket holding two or more bracketed rows of the same length, separated by commas, is a matrix: `[[a,b],[c,d]]`, `((1,2),(3,4))`. With `{` and the invisible `:}` it is `cases`: `{(x, x>0),(0, x<=0):}`.
- **Commands.** `sqrt`, `root(n)(x)`, `frac(a)(b)`, `abs`, `norm`, `floor`, `ceil`, accents (`hat`, `bar`, `vec`, `dot`, `ddot`, `tilde`, `ul`, `overarc`), braces with a label (`ubrace(…)_(…)`), `cancel`, alphabets (`bb`, `bbb`, `cc`, `tt`, `fr`, `sf`), `stackrel`, `overset`, `underset`, `color(red)(x)`, `text(…)` and `"quoted text"`.
- **Numbers.** A dot between digits is decimal; in the languages with a decimal comma, a comma between digits is too, and a comma followed by a space separates (L-NUM-1).

An unpaired bracket is an error in strict mode and is drawn as written, with a warning (E-REC-5), in tolerant mode.
