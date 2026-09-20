// `new textToAst(params).convert(text)` → JS AST array. Backed by the wasm
// `parse_text` / `parse_text_with_options` (option keys are the JS spellings:
// splitSymbols, unsplitSymbols, functionSymbols, appliedFunctionSymbols,
// operatorSymbols, allowSimplifiedFunctionApplication, parseLeibnizNotation,
// parseScientificNotation).
import wasm from "../_wasm";
import { jsonToAst } from "./ast-json";

export default class TextToAst {
  /** Parser options, passed through as JSON when non-empty. */
  params: Record<string, unknown>;
  constructor(params?: Record<string, unknown>) {
    this.params = params || {};
  }
  convert(text) {
    const handle =
      Object.keys(this.params).length > 0
        ? wasm.parse_text_with_options(text, JSON.stringify(this.params))
        : wasm.parse_text(text);
    try {
      // `jsonToAst`, not bare `JSON.parse`: `oo` parses to the wire tag
      // `{"$":"Inf"}` and callers expect the legacy scalar `Infinity`.
      return jsonToAst(handle.tree_json());
    } finally {
      handle.free(); // throwaway: created here, never handed to the caller
    }
  }
}
