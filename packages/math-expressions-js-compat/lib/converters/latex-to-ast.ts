// `new latexToAst(params).convert(latex)` → JS AST array, via the wasm
// `parse_latex` / `parse_latex_with_options`.
import wasm from "../_wasm";
import { jsonToAst } from "./ast-json";

export default class LatexToAst {
  /** Parser options, passed through as JSON when non-empty. */
  params: Record<string, unknown>;
  constructor(params?: Record<string, unknown>) {
    this.params = params || {};
  }
  convert(latex) {
    const handle =
      Object.keys(this.params).length > 0
        ? wasm.parse_latex_with_options(latex, JSON.stringify(this.params))
        : wasm.parse_latex(latex);
    try {
      // `jsonToAst`, not bare `JSON.parse`: `\infty` parses to the wire tag
      // `{"$":"Inf"}` and callers expect the legacy scalar `Infinity`.
      return jsonToAst(handle.tree_json());
    } finally {
      handle.free(); // throwaway: created here, never handed to the caller
    }
  }
}
