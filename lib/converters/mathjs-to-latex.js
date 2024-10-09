import mathjsToAstObj from "./mathjs-to-ast";
import astToLatexObj from "./ast-to-latex";

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
