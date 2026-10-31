# Embedded fonts

These are modified versions of two fonts, cut down to what mathra reads. They are not the original
fonts and are not meant to be installed or used outside mathra.

| File | Original | License |
|---|---|---|
| `STIXTwoMath-Regular-subset.otf` | STIX Two Math 2.12, [stipub/stixfonts](https://github.com/stipub/stixfonts) | SIL Open Font License 1.1 (`OFL-STIXTwoMath.txt`) |
| `latinmodern-math-subset.otf` | Latin Modern Math 1.959, [GUST](https://www.gust.org.pl/projects/e-foundry/lm-math) | GUST Font License (`GUST-FONT-LICENSE-LatinModernMath.txt`) |

Changes, made by [`tools/fonts/subset.sh`](../../../tools/fonts/subset.sh) from the originals kept in
`tools/fonts`: hinting removed; the GPOS, GDEF and DSIG tables removed, and of GSUB all but the `ssty`
feature (the forms for scripts), with the glyphs reached only through what was removed; glyph names removed; the `name` table reduced to copyright and license;
CFF subroutines expanded. Every character the original maps keeps its glyph, glyph ID, outline,
metrics and MATH data, which `tests/fonts.rs` checks against the originals.
