// `new textToAst(params).convert(text)` → JS AST array. Backed by the wasm
// `parse_text` / `parse_text_with_options` (option keys are the JS spellings:
// splitSymbols, unsplitSymbols, functionSymbols, appliedFunctionSymbols,
// operatorSymbols, allowSimplifiedFunctionApplication, parseLeibnizNotation,
// parseScientificNotation).
import wasm from "../_wasm";

export default class TextToAst {
  constructor(params) {
    this.params = params || {};
  }
  convert(text) {
    const handle =
      Object.keys(this.params).length > 0
        ? wasm.parse_text_with_options(text, JSON.stringify(this.params))
        : wasm.parse_text(text);
    try {
      return JSON.parse(handle.tree_json());
    } finally {
      handle.free(); // throwaway: created here, never handed to the caller
    }
  }
}
