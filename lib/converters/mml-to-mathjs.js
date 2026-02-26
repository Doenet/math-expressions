import mmlToLatexObj from "./mml-to-latex.js";
import latexToAstObj from "./latex-to-ast.js";
import astToMathjsObj from "./ast-to-mathjs.js";

class mmlToMathjs {
  constructor() {
    this.mmlToLatex = new mmlToLatexObj();
    this.latexToAst = new latexToAstObj();
    this.astToMathjs = new astToMathjsObj();
  }

  convert(mml) {
    return this.astToMathjs.convert(
      this.latexToAst.convert(this.mmlToLatex.convert(mml)),
    );
  }
}

export default mmlToMathjs;
