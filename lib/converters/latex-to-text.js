import latexToAstObj from "./latex-to-ast";
import astToTextObj from "./ast-to-text";

class latexToText {
  constructor() {
    this.latexToAst = new latexToAstObj();
    this.astToText = new astToTextObj();
  }

  convert(latex) {
    return this.astToText.convert(this.latexToAst.convert(latex));
  }
}

export default latexToText;
