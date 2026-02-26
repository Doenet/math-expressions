import textToAstObj from "./text-to-ast.js";
import astToMathjsObj from "./ast-to-mathjs.js";

class textToMathjs {
  constructor() {
    this.textToAst = new textToAstObj();
    this.astToMathjs = new astToMathjsObj();
  }

  convert(text) {
    return this.astToMathjs.convert(this.textToAst.convert(text));
  }
}

export default textToMathjs;
