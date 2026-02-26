import mmlToLatexObj from "./mml-to-latex.js";
import latexToAstObj from "./latex-to-ast.js";
import astToTextObj from "./ast-to-text.js";

class mmlToText {
  constructor() {
    this.mmlToLatex = new mmlToLatexObj();
    this.latexToAst = new latexToAstObj();
    this.astToText = new astToTextObj();
  }

  convert(mml) {
    return this.astToText.convert(
      this.latexToAst.convert(this.mmlToLatex.convert(mml)),
    );
  }
}

export default mmlToText;
