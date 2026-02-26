import textToAstObj from "./text-to-ast.js";
import astToLatexObj from "./ast-to-latex.js";

class textToLatex {
  constructor() {
    this.textToAst = new textToAstObj();
    this.astToLatex = new astToLatexObj();
  }

  convert(text) {
    return this.astToLatex.convert(this.textToAst.convert(text));
  }
}

export default textToLatex;
