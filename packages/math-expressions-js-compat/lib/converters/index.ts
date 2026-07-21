// The `me.converters` namespace. The four ported converters are backed by the
// Rust core; the mathjs / guppy / MathML converters have no Rust equivalent and
// are stubs that throw when used (see the individual files).
import TextToAst from "./text-to-ast";
import LatexToAst from "./latex-to-ast";
import AstToText from "./ast-to-text";
import AstToLatex from "./ast-to-latex";

export const textToAstObj = TextToAst;
export const latexToAstObj = LatexToAst;
export const astToTextObj = AstToText;
export const astToLatexObj = AstToLatex;

// Present so `me.converters.mmlToAstObj` etc. exist; unsupported at runtime.
export class mmlToAstObj {
  convert() {
    throw new Error("math-expressions-js-compat: MathML parsing is not implemented");
  }
}

export { TextToAst, LatexToAst, AstToText, AstToLatex };
