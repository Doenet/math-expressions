/**
 * Compile-only usage of the published declarations. Never executed and never
 * shipped as JavaScript — `npm run typecheck` type-checks it and stops.
 *
 * Each block is a call shape a real consumer makes, so a declaration that stops
 * admitting one fails here rather than in a consumer's build. The list is
 * drawn from DoenetML's call sites, which are the only large body of code
 * written against this API.
 */
import me, {
  dopri,
  isTree,
  setWasmModule,
  type Complex,
  type Expression,
  type OdeState,
  type Tree,
} from "math-expressions";

// --- parsing, the three entry points -------------------------------------
const fromText: Expression = me.fromText("x^2 + 2x + 1");
const fromLatex: Expression = me.fromLatex("\\frac{x+1}{2}");
const fromAst: Expression = me.fromAst(["+", 1, "x", 3]);

// `fromAst` accepts an `Expression` where a tree is expected, at any depth —
// a math-valued state variable holds one, and code that re-wraps it hands it
// straight back.
me.fromAst(["+", fromText as unknown as Tree, 1]);

// --- the AST round trip ---------------------------------------------------
const tree: Tree = fromAst.tree;
if (isTree(tree)) {
  // `isTree` is a type guard, so `tree` narrows.
  const _narrowed: Tree = tree;
  void _narrowed;
}

// --- rendering ------------------------------------------------------------
const _text: string = fromText.toString();
const _latex: string = fromText.toLatex();
void _text;
void _latex;

// --- the grading surface --------------------------------------------------
const _equal: boolean = fromText.equals(fromLatex);
const _fuzzy: boolean = fromText.equals(fromLatex, {
  allowed_error_in_numbers: 0.001,
  include_error_in_number_exponents: false,
});
// Exactly two shapes, and `null` is not one of them: a `number` (with `NaN`
// standing for "no numeric value", as in legacy), or a math.js `Complex` for a
// non-real value — `fromText("i")` is `{re: 0, im: 1}`. Annotating it
// `number | null` is what let a `{re, im}` object out of two DoenetML functions
// that promised a number, and the `null` it used to answer for `x+1` is what
// let unevaluable expressions read as zero.
const _constant: number | Complex = fromText.evaluate_to_constant();
// The narrowing that is actually sound, and which `!== null` never was.
const _asNumber: number =
  typeof _constant === "number" && !Number.isNaN(_constant) ? _constant : NaN;
void _asNumber;
void _equal;
void _fuzzy;
void _constant;

// --- symbolic operations --------------------------------------------------
const _simplified: Expression = fromText.simplify();
const _expanded: Expression = fromText.expand();
const _derivative: Expression = fromText.derivative("x");
const _substituted: Expression = fromText.substitute({ x: me.fromText("2y") });
const _vars: string[] = fromText.variables();
void _simplified;
void _expanded;
void _derivative;
void _substituted;
void _vars;

// --- context-level operations (`me.simplify(e)` alongside `e.simplify()`) --
const _ctxSimplified: Expression = me.simplify(fromText);

// --- numeric compilation --------------------------------------------------
const _fn = fromText.f();
const _atOne: number = _fn({ x: 1 }) as number;
void _ctxSimplified;
void _atOne;

// --- the ODE integrator ---------------------------------------------------
const scalarSolution = dopri(0, 1, 1, (_x: number, y: OdeState) => y as number);
const _atHalf: OdeState = scalarSolution.at(0.5);
const _early: boolean = scalarSolution.terminatedEarly;
scalarSolution.free();
void _atHalf;
void _early;

const vectorSolution = dopri(0, 1, [1, 0], (_x: number, y: OdeState) => [
  (y as number[])[1],
  -(y as number[])[0],
]);
const _states: OdeState[] = vectorSolution.y;
void _states;

// --- the browser/worker injection point -----------------------------------
declare const initializedWebWasm: Record<string, unknown>;
setWasmModule(initializedWebWasm);
