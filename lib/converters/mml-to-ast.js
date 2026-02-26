import mmlToLatexObj from "./mml-to-latex.js";
import latexToAstObj from "./latex-to-ast.js";

class mmlToAst {
  constructor() {
    this.mmlToLatex = new mmlToLatexObj();
    this.latexToAst = new latexToAstObj();
  }

  convert(mml) {
    return this.latexToAst.convert(this.mmlToLatex.convert(mml));
  }
}

export default mmlToAst;
