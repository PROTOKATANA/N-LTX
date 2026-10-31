# Languages

The tree of meaning is the same in every language; the locale only changes how it is **read** (decimal comma, open interval), **written** (decimal separator, function names) and **spoken**. Each language is one JSON file in `crates/core/src/locales/`, embedded in the binary.

| Code | Language | Decimal | Thousands | `]a, b[` | Written differently | Spoken |
|---|---|---|---|---|---|---|
| `en` | English | `.` | `,` | | | reference |
| `pt-BR` | Português (Brasil) | `,` | `.` | yes | `sen`, `tg`, `cotg`, `cossec`, `arcsen`, `arctg`, `senh`, `mdc`, `mmc` | reviewed |
| `es` | Español | `,` | `.` | | `sen`, `arcsen`, `senh`, `mcd`, `mcm` | to review |
| `fr` | Français | `,` | | yes | `cotan`, `sh`, `ch`, `th`, `pgcd`, `ppcm` | to review |
| `de` | Deutsch | `,` | `.` | yes | `ggT`, `kgV` | to review |
| `it` | Italiano | `,` | `.` | yes | `sen`, `tg`, `cotg`, `arcsen`, `arctg`, `senh`, `MCD`, `mcm` | to review |
| `ru` | Русский | `,` | | | `tg`, `ctg`, `cosec`, `arctg`, `sh`, `ch`, `th` | to review |
| `zh` | 中文（简体） | `.` | `,` | | | to review |
| `ja` | 日本語 | `.` | `,` | | | to review |
| `ko` | 한국어 | `.` | `,` | | | to review |
| `hi` | हिन्दी | `.` | `,` | | | to review |
| `ar` | العربية | `.` | `,` | | | to review |
| `id` | Bahasa Indonesia | `,` | `.` | | | to review |
| `tr` | Türkçe | `,` | `.` | | `ebob`, `ekok` | to review |
| `nl` | Nederlands | `,` | `.` | | `ggd`, `kgv` | to review |
| `pl` | Polski | `,` | | | `tg`, `ctg`, `arctg`, `NWD`, `NWW` | to review |

A code with a region (`es-MX`, `zh-CN`, `pt-PT`) uses its language. Regions with other conventions (Mexico writes a decimal point) are not distinguished yet.

**"To review"** means the phrases were written without a native speaker's review. Spoken math has school conventions that a translation misses; corrections are welcome as pull requests to the JSON file.

## The file

```json
{
 "code": "fr",
 "name": "Français",
 "decimal": ",",
 "thousands": null,
 "open_interval": true,
 "written": { "sinh": "sh", "gcd": "pgcd" },
 "fractions": { "numerals": […], "singular": […], "plural": […] },
 "speech": { "fraction_simple": "{0} sur {1}", "op.=": "égale", "greek.α": "alpha", … }
}
```

- `decimal`, `thousands`, `open_interval`: rules L-NUM-1 to L-NUM-3 and L-INT-1 (`rules.md`).
- `written`: function names that differ from LaTeX's (L-FUN-1). Missing ones are written as in LaTeX.
- `fractions` (optional): school names for 1/2 to 10/10, such as "trois quarts". Without it, `\frac{3}{4}` is read with `fraction_simple`.
- `units` (optional, L-UNI-3): the names of the siunitx units, by command name: `per`, `singular` (`one` or `below_two`: when the unit after a value is singular), `prefixes` (`"kilo": "quilo"`), `names` (`"meter": ["metro", "metros"]`, singular and plural) and `compounds`, for prefixed names that are not a plain concatenation (`"kilometer": ["quilômetro", "quilômetros"]`). Without it, units are read by their symbols. en and pt-BR have it.
- `double_struck` (optional, L-LET-2): `letter`, the template of any double-struck letter (`"{0} duplo"`); `sets`, the number sets by letter (`"R": "os reais"`); `contractions`, a preposition and an article joined into one word (`"a os": "aos"`). Without it, double-struck letters are read as plain letters. en and pt-BR have it.
- `optional` (optional): phrases a language may have, with no English fallback; without one, the general reading stays. `dot_product` is the dot between vectors (L-OPR-1). en and pt-BR have it.
- `speech`: every phrase, as a template where `{0}`, `{1}`… are the parts in order (`binomial` gets n and k, `long_division` gets dividend, divisor, quotient and remainder). The template decides the word order, so "k parmi n" (fr) and "2 分之 1" (zh, denominator first) need no code.

Keys: `op.<symbol>` for operators and relations, `big.<symbol>` for large operators, `greek.<letter>` for lowercase Greek (capitals use `capital`), `fn.<name>` for spoken function names (a function without one is read as written, without `function_of`), `accent.*`, `arrow.*`, and the structure keys in `en.json`.

A phrase missing from a language falls back to English. The test `every_language_has_every_phrase` (`crates/core/tests/locales.rs`) requires every file to be complete, and `every_language_converts_the_corpus` converts the whole corpus in every language.

## Adding a language

1. Copy `en.json` to `<code>.json` and translate `name` and every phrase.
2. Add the variant to `Locale` and its line to `SOURCES`, `ALL` and `code()` in `crates/core/src/locale.rs`.
3. Add a few phrases to `SPEECHES` in `crates/core/tests/locales.rs`, and a line to the table above.

## Limits

- The drawing uses STIX Two Math, which covers Latin, Greek and Cyrillic. Text in other scripts inside `\text{…}` (Chinese, Japanese, Arabic, Devanagari) is drawn as □ (E-REC-8); the speech, which lives in `<title>`, works in any script.
- The speech is a linear phrase built in the order of the equation. Languages that put the argument before the function (ja, ko, hi, tr) read `sin x` as "サイン x", the way it is read aloud in class, not with the grammatical order of a sentence.
- Right-to-left math notation (Arabic school notation) is out of scope: the equation is drawn left to right, as in Arabic scientific texts with Latin letters.
