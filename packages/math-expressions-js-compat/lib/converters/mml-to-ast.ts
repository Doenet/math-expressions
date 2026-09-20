/*
 * Presentation MathML → math-expressions AST.
 *
 * Ported from the legacy `lib/converters/mml-to-ast.js`, which is nothing more
 * than the composition of the MathML→LaTeX converter with the LaTeX parser —
 * there is no separate MathML grammar. The LaTeX half is the Rust core's.
 */
import mmlToLatexObj from "./mml-to-latex";
import latexToAstObj from "./latex-to-ast";

class mmlToAst {
  mmlToLatex: mmlToLatexObj;
  latexToAst: latexToAstObj;

  constructor() {
    this.mmlToLatex = new mmlToLatexObj();
    this.latexToAst = new latexToAstObj();
  }

  convert(mml: string) {
    return this.latexToAst.convert(this.mmlToLatex.convert(mml));
  }
}

export default mmlToAst;
