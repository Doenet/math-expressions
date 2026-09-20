// `new astToLatex(params).convert(ast)` → LaTeX string, via wasm `from_ast` +
// `to_latex_with_options`. Emitter options are forwarded the same way as in
// `ast-to-text.ts`.
import wasm from "../_wasm";
import { astToJson } from "./ast-json";
import { renderOptions } from "./render-options";

export default class AstToLatex {
  /** Emitter options, forwarded through `renderOptions`. */
  params: Record<string, unknown>;
  constructor(params?: Record<string, unknown>) {
    this.params = params || {};
  }
  convert(ast) {
    const handle = wasm.from_ast(astToJson(ast));
    try {
      return handle.to_latex_with_options(renderOptions(this.params));
    } finally {
      handle.free(); // throwaway: created here, never handed to the caller
    }
  }
}
