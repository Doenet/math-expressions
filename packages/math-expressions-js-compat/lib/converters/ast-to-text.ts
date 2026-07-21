// `new astToText(params).convert(ast)` → text string, via wasm `from_ast` +
// `to_text`. The wasm emitter is fixed-behavior, so emitter options
// (output_unicode, padToDigits/padToDecimals, avoidScientificNotation,
// showBlanks) are NOT honored — those option-specific spec cases will differ.
import wasm from "../_wasm";

export default class AstToText {
  constructor(params) {
    this.params = params || {};
  }
  convert(ast) {
    return wasm.from_ast(JSON.stringify(ast)).to_text();
  }
}
