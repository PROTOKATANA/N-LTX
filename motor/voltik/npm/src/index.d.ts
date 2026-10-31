export interface Result {
  /** Standalone SVG, with math labels typeset by mathra (through tikra). */
  svg: string;
  /** The figure in a sentence, for readers who cannot see it: the SVG's `<title>`, an image's `alt`. */
  alt: string;
  /** The figure in words (axes, data, nodes and their connections): the SVG's `<desc>`. */
  description: string;
  /** What was ignored or approximated: unsupported commands, labels with broken LaTeX. */
  warnings: string[];
}

/** mathra's languages: the figure in words (`alt`, `description`) is told in them; the drawing is the same. */
export type Locale = "en" | "pt-BR" | "es" | "fr" | "de" | "it" | "ru" | "zh" | "ja" | "ko" | "hi" | "ar" | "id" | "tr" | "nl" | "pl";

export interface Options {
  /** The language of the figure in words. Default `"en"`. A code with a region (`"es-MX"`) takes its language. */
  locale?: Locale | (string & {});
  /** The construction animation: in a browser, the figure draws itself path by path, in the order TikZ drew it. Default `false`. */
  animate?: boolean;
  /** Each element numbered by its step (`data-step`, and `data-steps` on the `<svg>`), with no CSS: for a page that walks through the figure itself. Default `false`. */
  steps?: boolean;
}

export interface PngOptions {
  /** Resolution. Default 300, for print. */
  dpi?: number;
  /** Paint a white background (default) or keep it transparent (false). */
  background?: boolean;
}

/** Loads the WebAssembly module: await it once before converting in browsers and bundlers. In Node it does nothing. */
export function init(input?: RequestInfo | URL | Response | BufferSource | WebAssembly.Module): Promise<void>;

/** Converts a CircuiTikZ circuit (a `standalone` document, a `circuitikz`, a `tikzpicture` with its components, or loose statements) to SVG, with the figure in words in `options.locale`. Throws when there is no figure to draw or the locale is unknown. */
export function circuitikzToSvg(code: string, options?: Options): string;

/** Converts a CircuiTikZ circuit and returns the SVG, the figure in words and the warnings. */
export function render(code: string, options?: Options): Result;

/** Rasterizes an SVG written by voltik to PNG bytes. Throws on an invalid SVG or dpi, or past 100 megapixels. */
export function png(svg: string, options?: PngOptions): Uint8Array;

/** The version of voltik's core inside this package (the npm package has its own version). */
export function version(): string;
