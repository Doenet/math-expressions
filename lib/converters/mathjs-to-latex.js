import mathjsToAstObj from "./mathjs-to-ast.js";
import astToLatexObj from "./ast-to-latex.js";

class mathjsToLatex {
  constructor() {
    this.mathjsToAst = new mathjsToAstObj();
    this.astToLatex = new astToLatexObj();
  }

  convert(mathjs) {
    return this.astToLatex.convert(this.mathjsToAst.convert(mathjs));
  }
}

export default mathjsToLatex;
