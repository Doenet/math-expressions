import mmlToLatexObj from "./mml-to-latex";
import latexToAstObj from "./latex-to-ast";
import astToGuppyObj from "./ast-to-guppy";

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
