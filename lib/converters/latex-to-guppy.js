import latexToAstObj from "./latex-to-ast.js";
import astToGuppyObj from "./ast-to-guppy.js";

class latexToGuppy {
  constructor() {
    this.latexToAst = new latexToAstObj();
    this.astToGuppy = new astToGuppyObj();
  }

  convert(latex) {
    return this.astToGuppy.convert(this.latexToAst.convert(latex));
  }
}

export default latexToGuppy;
