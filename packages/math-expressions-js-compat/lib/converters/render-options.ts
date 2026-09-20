// Translate a legacy converter's constructor params into the options JSON the
// wasm `to_text_with_options` / `to_latex_with_options` entry points read.
//
// Only the keys the Rust printers understand are forwarded — a converter's
// params object may carry parser-side settings (and, in DoenetML's use,
// non-serializable values) that have no business in a render call.

/** Legacy spelling → the name the Rust side reads. */
const RENAMED: Record<string, string> = {
  output_unicode: "unicode",
};

const FORWARDED = [
  "unicode",
  "notation",
  "padToDigits",
  "padToDecimals",
  "showBlanks",
  "explicitMultiplicationSymbols",
  "avoidScientificNotation",
  "matrixEnvironment",
];

export function renderOptions(params: Record<string, unknown> | undefined) {
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(params || {})) {
    if (v === undefined || v === null) continue;
    const name = RENAMED[k] ?? k;
    if (FORWARDED.includes(name)) out[name] = v;
  }
  return JSON.stringify(out);
}
