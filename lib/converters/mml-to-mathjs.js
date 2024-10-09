import mmlToLatexObj from "./mml-to-latex";
import latexToAstObj from "./latex-to-ast";
import astToMathjsObj from "./ast-to-mathjs";

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
