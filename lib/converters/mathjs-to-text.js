import mathjsToAstObj from "./mathjs-to-ast.js";
import astToTextObj from "./ast-to-text.js";

class mathjsToText {
  constructor() {
    this.mathjsToAst = new mathjsToAstObj();
    this.astToText = new astToTextObj();
  }

  convert(mathjs) {
    return this.astToText.convert(this.mathjsToAst.convert(mathjs));
  }
}

export default mathjsToText;
