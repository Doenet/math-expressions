// `new astToLatex(params).convert(ast)` → LaTeX string, via wasm `from_ast` +
// `to_latex`. Emitter options are not honored (see ast-to-text.js).
import wasm from "../_wasm";

export default class AstToLatex {
  constructor(params) {
    this.params = params || {};
  }
  convert(ast) {
    return wasm.from_ast(JSON.stringify(ast)).to_latex();
  }
}
