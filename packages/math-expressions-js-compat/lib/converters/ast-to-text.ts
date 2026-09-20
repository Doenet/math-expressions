// `new astToText(params).convert(ast)` → text string, via wasm `from_ast` +
// `to_text_with_options`. The constructor's emitter options (output_unicode,
// padToDigits/padToDecimals, avoidScientificNotation, showBlanks,
// explicitMultiplicationSymbols) are forwarded to the Rust printer; anything
// else in `params` is dropped (see `render-options.ts`).
import wasm from "../_wasm";
import { astToJson } from "./ast-json";
import { renderOptions } from "./render-options";

export default class AstToText {
  /** Emitter options, forwarded through `renderOptions`. */
  params: Record<string, unknown>;
  constructor(params?: Record<string, unknown>) {
    this.params = params || {};
  }
  convert(ast) {
    const handle = wasm.from_ast(astToJson(ast));
    try {
      return handle.to_text_with_options(renderOptions(this.params));
    } finally {
      handle.free(); // throwaway: created here, never handed to the caller
    }
  }
}
