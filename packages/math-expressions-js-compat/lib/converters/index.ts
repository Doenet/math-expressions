// The `me.converters` namespace. The text/LaTeX converters are backed by the
// Rust core; the guppy, mathjs and MathML ones are pure-notation converters
// ported directly to TypeScript (math.js nodes, Guppy XML and MathML never
// reach Rust — MathML is reduced to LaTeX first and handed to the Rust parser).
import TextToAst from "./text-to-ast";
import LatexToAst from "./latex-to-ast";
import AstToText from "./ast-to-text";
import AstToLatex from "./ast-to-latex";
import AstToGuppy from "./ast-to-guppy";
import AstToMathjs from "./ast-to-mathjs";
import MathjsToAst from "./mathjs-to-ast";
import MmlToLatex from "./mml-to-latex";
import MmlToAst from "./mml-to-ast";

export const textToAstObj = TextToAst;
export const latexToAstObj = LatexToAst;
export const astToTextObj = AstToText;
export const astToLatexObj = AstToLatex;
export const astToGuppyObj = AstToGuppy;
export const astToMathjsObj = AstToMathjs;
export const mathjsToAstObj = MathjsToAst;
export const mmlToLatexObj = MmlToLatex;
export const mmlToAstObj = MmlToAst;

export {
  TextToAst,
  LatexToAst,
  AstToText,
  AstToLatex,
  AstToGuppy,
  AstToMathjs,
  MathjsToAst,
  MmlToLatex,
  MmlToAst,
};
