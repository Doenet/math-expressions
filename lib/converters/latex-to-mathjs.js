import latexToAstObj from "./latex-to-ast.js";
import astToMathjsObj from "./ast-to-mathjs.js";

class latexToMathjs {
  constructor() {
    this.latexToAst = new latexToAstObj();
    this.astToMathjs = new astToMathjsObj();
  }

  convert(latex) {
    return this.astToMathjs.convert(this.latexToAst.convert(latex));
  }
}

export default latexToMathjs;
