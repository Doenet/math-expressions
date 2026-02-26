import mmlToLatexObj from "./mml-to-latex.js";
import latexToAstObj from "./latex-to-ast.js";
import astToGuppyObj from "./ast-to-guppy.js";

class mmlToGuppy {
  constructor() {
    this.mmlToLatex = new mmlToLatexObj();
    this.latexToAst = new latexToAstObj();
    this.astToGuppy = new astToGuppyObj();
  }

  convert(mml) {
    return this.astToGuppy.convert(
      this.latexToAst.convert(this.mmlToLatex.convert(mml)),
    );
  }
}

export default mmlToGuppy;
