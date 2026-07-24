// Generate supplementary edge-case fixtures by running the JS parsers as the
// oracle. Unlike extract-fixtures.mjs (which copies expectations out of the
// spec files), this script records whatever the JS implementation actually
// does — for inputs the upstream specs never cover. Targets: lexer rules with
// no spec coverage (unicode operator aliases, keyword boundaries, \big
// delimiter families, \var* substitutions, sci-notation lookahead edges) and
// error paths.
//
// Re-run when the corpus or the JS implementation changes:
//   node scripts/generate-edge-fixtures.mjs

import { writeFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";
import textToAst from "../../../tmp/js-legacy/lib/converters/text-to-ast.js";
import latexToAst from "../../../tmp/js-legacy/lib/converters/latex-to-ast.js";

const here = dirname(fileURLToPath(import.meta.url));
const outDir = join(here, "../tests/fixtures");

const textCorpus = [
  // unicode multiplication aliases (rule table entries with no spec coverage)
  "a·b",
  "a⋅b",
  "a•b",
  "a×b",
  "a**b",
  // unicode minus / hyphen family
  "a−b",
  "a–b",
  "a—b",
  "a‐b",
  "x₋"    ,
  // unicode relations, logic, arrows
  "a≠b",
  "a≤b≤c",
  "a≥b",
  "¬p",
  "p∧q∨r",
  "a᐀b",
  "p→q",
  "p←q",
  "p↔q",
  "p⟹q",
  "p⟸q",
  "p⟺q",
  // unicode set relations
  "a∈B",
  "a∉B",
  "A∋b",
  "A∌b",
  "A⊆B",
  "A⊈B",
  "A⊇B",
  "A⊉B",
  "A⊄B",
  "A⊅B",
  "A∪B∩C",
  // unicode misc
  "x⟂y",
  "x∥y",
  "∠ABC",
  "∫x dx",
  "∞",
  "-∞",
  "∅",
  "ℯ^x",
  "α+β",
  "µ",
  "♠+♡",
  "x‸2",
  "xʌ2",
  "f′(x)",
  // infinity keyword forms
  "infinity",
  "Infinity",
  "infty",
  "OO",
  "oo+1",
  // keyword boundaries (rule requires (?![a-zA-Z0-9]))
  "andy",
  "oo2",
  "pix",
  "plusminusx",
  "intx",
  "subsetx",
  "forallx",
  "not x",
  "nota",
  // ampersand forms
  "a&&b",
  "a&b",
  // scientific notation lookahead edges
  "1E5",
  "1E+5",
  "1E-5",
  "(1E5)",
  "1E5x",
  "1E5 x",
  "[1E5]",
  "{1E5}",
  "1E5,2",
  "2E-3+1",
  "1.E3",
  "3.",
  ".5",
  "1.2.3",
  // units
  "50%",
  "$5",
  "5$",
  "%x",
  "3 deg + 2",
  // subscript/superscript interactions
  "x_2^3",
  "x^3_2",
  "C_-^+",
];

const latexCorpus = [
  // \big... delimiter families (zero spec coverage)
  "\\bigl( x \\bigr)",
  "\\Bigl[ x \\Bigr]",
  "\\biggl( x+1 \\biggr)",
  "\\Biggl( x \\Biggr)",
  "\\bigl\\{ x \\bigr\\}",
  "\\big| x \\big|",
  "\\Bigg| x \\Bigg|",
  "\\bigl| x \\bigr|",
  "\\bigl\\lfloor x \\bigr\\rfloor",
  "\\Bigl\\lceil x \\Bigr\\rceil",
  "\\biggl\\langle x, y \\biggr\\rangle",
  // \var* substitutions
  "\\varepsilon + \\vartheta",
  "\\varnothing",
  "\\varrho",
  "\\varphi",
  // \asin family substitutions
  "\\asin(x)",
  "\\acos(x)",
  "\\atan(x)",
  // arrows and synonyms
  "p \\to q",
  "p \\gets q",
  "p \\Longrightarrow q",
  "p \\Longleftarrow q",
  "p \\Longleftrightarrow q",
  "x \\bot y",
  // \not combinations
  "a \\ne b",
  "a \\not= b",
  "a \\not \\in B",
  "A \\not\\subseteq B",
  "A \\not \\supset B",
  // integrals
  "\\int x dx",
  "\\int_0^1 x^2 dx",
  "\\int x\\,dx",
  // spacing commands
  "x \\qquad y",
  "x\\;y",
  "x\\:y",
  "x\\>y",
  "x\\!y",
  "a\\ b",
  // operatorname edges
  "\\operatorname{foo}^2",
  "\\operatorname{a+b}",
  "\\operatorname{ sin2 }(x)",
  // \circ degree unit
  "45^\\circ",
  "x^\\circ + 90",
  "x^{\\circ}",
  // sci notation with latex delimiters
  "x^{1E2}",
  "\\begin{matrix}1E2 & 2\\end{matrix}",
  "1E2\\\\3",
  // matrix edges
  "\\begin{matrix} & 1 \\\\ 2 \\end{matrix}",
  "\\begin{matrix}1 & 2 & \\\\ 3 \\end{matrix}",
  // frac without braces
  "\\frac[1]{2}",
  "\\frac\\pi2",
  // sqrt edges
  "\\sqrt[3]{8}y",
  "\\sqrt2",
  // error paths
  "\\mathbb{R}",
  "\\begin{foo}x\\end{foo}",
  "\\begin{matrix}1\\end{pmatrix}",
  "\\sin \\left( x \\right]",
];

function generate(name, corpus, Converter) {
  const trees = [];
  const errors = [];
  for (const input of corpus) {
    const c = new Converter();
    try {
      trees.push({ input, tree: encodeSpecials(c.convert(input)) });
    } catch (e) {
      errors.push({ input, error: e.message });
    }
  }
  writeFileSync(
    join(outDir, `${name}-edge.json`),
    JSON.stringify(trees, null, 1) + "\n",
  );
  if (errors.length > 0) {
    writeFileSync(
      join(outDir, `${name}-edge-errors.json`),
      JSON.stringify(errors, null, 1) + "\n",
    );
  }
  console.log(`${name}-edge: ${trees.length} trees, ${errors.length} errors`);
}

function encodeSpecials(tree) {
  if (Array.isArray(tree)) return tree.map(encodeSpecials);
  if (typeof tree === "number") {
    if (tree === Infinity) return { $: "Inf" };
    if (tree === -Infinity) return { $: "-Inf" };
    if (Number.isNaN(tree)) return { $: "NaN" };
  }
  return tree;
}

generate("text-to-ast", textCorpus, textToAst);
generate("latex-to-ast", latexCorpus, latexToAst);
