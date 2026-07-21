// `new latexToAst(params).convert(latex)` → JS AST array, via the wasm
// `parse_latex` / `parse_latex_with_options`.
import wasm from "../_wasm";

export default class LatexToAst {
  constructor(params) {
    this.params = params || {};
  }
  convert(latex) {
    const handle =
      Object.keys(this.params).length > 0
        ? wasm.parse_latex_with_options(latex, JSON.stringify(this.params))
        : wasm.parse_latex(latex);
    return JSON.parse(handle.tree_json());
  }
}
