import textToAstObj from "./text-to-ast";
import astToLatexObj from "./ast-to-latex";

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
