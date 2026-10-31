/** A language code: "en" (default), "pt-BR", "es", "fr", "de", "it", "ru", "zh", "ja", "ko", "hi", "ar", "id", "tr", "nl" or "pl". A region is accepted ("es-MX", "zh-CN"). */
export type Locale = "en" | "pt-BR" | "es" | "fr" | "de" | "it" | "ru" | "zh" | "ja" | "ko" | "hi" | "ar" | "id" | "tr" | "nl" | "pl" | (string & {});

export interface Options {
  /** How the equation is read, written and spoken. Default "en". */
  locale?: Locale;
  /** Display style, as in `$$…$$` (default), or inline with the text, as in `$…$` (false). */
  display?: boolean;
  /** Refuse invalid LaTeX, like TeX does, instead of recovering with warnings. Default false. */
  strict?: boolean;
  /** The notation of the equation: "latex" (default), "mathml" or "asciimath". MathML passed as LaTeX (starting with `<math`) is recognized on its own. */
  input?: "latex" | "mathml" | "asciimath";
  /** The braille code of `Result.braille`: "cmu" or "nemeth". Default: the locale's (CMU in pt-BR and es, Nemeth elsewhere). */
  braille?: "cmu" | "nemeth";
  /** The SVG writes itself in the browser, glyph by glyph and stroke by stroke, in the order the layout set it (CSS in the SVG). Default false; without it the SVG is unchanged. */
  animate?: boolean;
  /** Each element of the SVG numbered by that order (`data-step`, and `data-steps` on the `<svg>`), with no CSS: for a page that walks through the equation itself. Default false. */
  steps?: boolean;
}

/** A numbered rule from spec/rules.md or spec/tolerance.md, and what it did to this equation. */
export interface Rule {
  rule: string;
  description: string;
}

export interface Result {
  /** Standalone SVG with the speech in `<title>` and the source and tree in `<metadata>`. */
  svg: string;
  /** The phrase a screen reader says, in the locale's language. */
  speech: string;
  /** Linear text, close to UnicodeMath. */
  text: string;
  /** The equation in braille, as Unicode braille cells. */
  braille: string;
  /** The braille code used: "cmu" or "nemeth". */
  braille_code: "cmu" | "nemeth";
  /** The tree of meaning. */
  tree: unknown;
  /** Rules that changed something in this equation, in the order they acted. */
  applied: Rule[];
  /** Input problems the tolerant mode worked around (E-REC-*); empty in strict mode. */
  warnings: Rule[];
}

/** Loads the WebAssembly module: await it once before converting in browsers and bundlers. In Node it does nothing. */
export function init(input?: RequestInfo | URL | Response | BufferSource | WebAssembly.Module): Promise<void>;

/** Converts an equation (LaTeX unless `options.input` says otherwise) to SVG. Throws on input it cannot read (or on any invalid input with `strict`). */
export function texToSvg(tex: string, options?: Options | Locale): string;

/** Converts an equation and returns every output at once. */
export function render(tex: string, options?: Options | Locale): Result;

/** The supported languages, English first. */
export function locales(): { code: string; name: string }[];

/** The version of mathra's core inside this package (the npm package has its own version). */
export const version: string;
