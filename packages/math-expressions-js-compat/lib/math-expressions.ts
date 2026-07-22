// Drop-in replacement for the original `lib/math-expressions.js` default export
// (the `Context` factory + `Expression`), backed by the Rust/wasm core.
//
// Not every legacy method exists on the Rust side; those that don't are either
// approximated, or throw a clear "not implemented in js-compat" so the calling
// test fails cleanly (the suite still runs). See JS_TEST_COVERAGE_AUDIT.md.
import wasm from "./_wasm";
import math from "./mathjs";
import { match, flatten, unflattenLeft, unflattenRight } from "./trees/flatten";
import * as converters from "./converters/index";
import type { WasmExpression } from "math-expressions-rs-wasm";

/** The JS AST tree encoding (`["+", 1, "x", 3]`). */
export type Tree = number | string | boolean | Tree[];

/** Anything that can be coerced to an Expression. */
export type ExpressionLike = Expression | WasmExpression | Tree;

/** Legacy `.equals` grading options (snake_case or camelCase keys). */
export type EqualityOptions = Record<string, unknown>;

/** `.substitute` / `.evaluate` bindings. */
export type Bindings = Record<string, ExpressionLike>;

/** Type guard mirroring the original `isTree`. */
export function isTree(value: unknown): boolean {
  if (
    typeof value === "number" ||
    typeof value === "string" ||
    typeof value === "boolean"
  ) {
    return true;
  }
  if (Array.isArray(value) && value.length > 0 && typeof value[0] === "string") {
    return value.slice(1).every((item) => isTree(item));
  }
  return false;
}

function notImplemented(name: string): (...args: unknown[]) => never {
  return function () {
    throw new Error(`math-expressions-js-compat: ${name}() is not implemented`);
  };
}

/** Wrap a raw wasm Expression handle (or undefined) as a compat Expression. */
function wrap(handle: WasmExpression | undefined, context: Ctx): Expression | undefined {
  if (handle === undefined || handle === null) return undefined;
  return new Expression(handle, context);
}

/** Coerce a value (Expression | wasm handle | string | number | AST) → Expression. */
function toExpr(x: ExpressionLike, context?: Ctx): Expression {
  const ctx = context || Context;
  if (x instanceof Expression) return x;
  if (x && typeof (x as WasmExpression).tree_json === "function") {
    return new Expression(x as WasmExpression, ctx);
  }
  if (typeof x === "string") return ctx.fromText(x);
  return ctx.fromAst(x as Tree); // number or AST array
}

/** A variable argument may be a string name or an Expression of a symbol. */
function varName(v: string | Expression): string {
  if (typeof v === "string") return v;
  if (v instanceof Expression) return v.toString();
  return String(v);
}

/** The Context (`me`) shape, used for the back-reference on each Expression. */
type Ctx = typeof Context;

// Legacy `.equals` options are snake_case; the wasm `equals_with_options` takes
// camelCase JSON keys. Map the ones the Rust side understands; drop the rest.
const EQ_OPTION_KEYS: Record<string, string> = {
  relative_tolerance: "relativeTolerance",
  absolute_tolerance: "absoluteTolerance",
  tolerance_for_zero: "toleranceForZero",
  allowed_error_in_numbers: "allowedErrorInNumbers",
  include_error_in_number_exponents: "includeErrorInNumberExponents",
  allowed_error_is_absolute: "allowedErrorIsAbsolute",
  allow_blanks: "allowBlanks",
};
function mapEqOptions(opts: EqualityOptions): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(opts)) {
    if (EQ_OPTION_KEYS[k]) out[EQ_OPTION_KEYS[k]] = v;
    else if (Object.values(EQ_OPTION_KEYS).includes(k)) out[k] = v; // already camelCase
  }
  return out;
}

class Expression {
  _w: WasmExpression;
  context: Ctx;

  constructor(handle: WasmExpression, context?: Ctx) {
    this._w = handle;
    this.context = context || Context;
  }

  // ---- inspection / rendering ----
  get tree() {
    return JSON.parse(this._w.tree_json());
  }
  toString() {
    return this._w.to_text();
  }
  toText() {
    return this._w.to_text();
  }
  toLatex() {
    return this._w.to_latex();
  }
  tex() {
    return this._w.to_latex();
  }
  toJSON() {
    return JSON.parse(this._w.to_serialized());
  }
  variables() {
    return this._w.variables();
  }
  functions() {
    return this._w.functions();
  }

  // ---- equality ----
  equals(other, options) {
    const o = toExpr(other, this.context);
    if (options && Object.keys(options).length > 0) {
      return this._w.equals_with_options(o._w, JSON.stringify(mapEqOptions(options)));
    }
    return this._w.equals(o._w);
  }
  equalsViaReal(other) {
    return this._w.equals_via_real(toExpr(other, this.context)._w);
  }
  // via-complex is a numerical variant — the wasm `equals` (complex sampling).
  equalsViaComplex(other) {
    return this._w.equals(toExpr(other, this.context)._w);
  }
  // via-syntax is a *structural* comparison — NOT numerical. The wasm exposes it
  // through `structural_equality` with the `sameStructure` criterion, which
  // routes to Rust `equals_syntactic` (no sampling). This matches the original
  // `equalsViaSyntax` and never evaluates the expression at sample points.
  equalsViaSyntax(other) {
    return this._w.structural_equality(toExpr(other, this.context)._w, '"sameStructure"');
  }
  is_zero() {
    return this._w.is_zero();
  }
  isAnalytic(opts) {
    const o = opts || {};
    return this._w.is_analytic(!!o.allow_abs, !!o.allow_arg, !!o.allow_relation);
  }

  // ---- calculus ----
  derivative(v) {
    return wrap(this._w.derivative(varName(v)), this.context);
  }
  integrate(v) {
    return wrap(this._w.integrate(varName(v)), this.context);
  }

  // ---- normalization / simplification ----
  simplify() {
    const a = this.context._assumptionTexts;
    return wrap(
      a && a.length ? this._w.simplify_with_assumptions(a) : this._w.simplify(),
      this.context,
    );
  }
  simplify_logical() {
    return wrap(this._w.simplify_logical(), this.context);
  }
  expand() {
    return wrap(this._w.expand(), this.context);
  }
  factor() {
    return wrap(this._w.factor(), this.context);
  }
  evaluate_numbers(_opts) {
    return wrap(this._w.evaluate_numbers(), this.context);
  }
  collect_like_terms_factors() {
    return wrap(this._w.collect_like_terms_factors(), this.context);
  }
  simplify_ratios() {
    return wrap(this._w.simplify_ratios(), this.context);
  }
  reduce_rational() {
    return wrap(this._w.reduce_rational(), this.context);
  }
  together() {
    return wrap(this._w.together(), this.context);
  }
  normalize_function_names() {
    return wrap(this._w.normalize_function_names(), this.context);
  }
  constants_to_floats() {
    return wrap(this._w.constants_to_floats(), this.context);
  }

  // ---- structural conversions ----
  tuples_to_vectors() {
    return wrap(this._w.tuples_to_vectors(), this.context);
  }
  altvectors_to_vectors() {
    return wrap(this._w.altvectors_to_vectors(), this.context);
  }
  to_intervals() {
    return wrap(this._w.to_intervals(), this.context);
  }
  subscripts_to_strings() {
    return wrap(this._w.subscripts_to_strings(), this.context);
  }
  strings_to_subscripts() {
    return wrap(this._w.strings_to_subscripts(), this.context);
  }
  copy() {
    return wrap(this._w.copy(), this.context);
  }

  // ---- units ----
  remove_units(scaleBasedOnUnit) {
    return wrap(this._w.remove_units(!!scaleBasedOnUnit), this.context);
  }
  remove_scaling_units() {
    return wrap(this._w.remove_scaling_units(), this.context);
  }
  add_unit(unit) {
    return wrap(this._w.add_unit(unit), this.context);
  }
  set_small_zero(tolerance) {
    return wrap(this._w.set_small_zero(tolerance === undefined ? 1e-14 : tolerance), this.context);
  }

  // ---- rounding ----
  round_numbers_to_precision(sigFigs) {
    return wrap(this._w.round_numbers_to_precision(sigFigs), this.context);
  }
  round_numbers_to_decimals(decimals) {
    return wrap(this._w.round_numbers_to_decimals(decimals), this.context);
  }
  round_numbers_to_precision_plus_decimals(digits, decimals) {
    return wrap(
      this._w.round_numbers_to_precision_plus_decimals(digits, decimals),
      this.context,
    );
  }

  // ---- evaluation ----
  evaluate_to_constant() {
    const v = this._w.evaluate_to_constant();
    return v === undefined ? null : v;
  }
  evaluate_to_complex() {
    const v = this._w.evaluate_to_complex();
    return v === undefined ? null : math.complex(v[0], v[1]);
  }
  evaluate(bindings) {
    const vars = Object.keys(bindings || {});
    const vals = Float64Array.from(vars.map((k) => Number(bindings[k])));
    const r = this._w.evaluate(vars, vals);
    return r === undefined ? NaN : r;
  }
  substitute(bindings) {
    let cur = this._w;
    for (const k of Object.keys(bindings || {})) {
      cur = cur.substitute_var(k, toExpr(bindings[k], this.context)._w);
    }
    return wrap(cur, this.context);
  }

  // ---- arithmetic ----
  add(other) {
    return wrap(this._w.add(toExpr(other, this.context)._w), this.context);
  }
  subtract(other) {
    return wrap(this._w.subtract(toExpr(other, this.context)._w), this.context);
  }
  multiply(other) {
    return wrap(this._w.multiply(toExpr(other, this.context)._w), this.context);
  }
  divide(other) {
    return wrap(this._w.divide(toExpr(other, this.context)._w), this.context);
  }
  pow(other) {
    return wrap(this._w.pow(toExpr(other, this.context)._w), this.context);
  }
  mod(other) {
    return wrap(this._w.mod(toExpr(other, this.context)._w), this.context);
  }

  // ---- matrices / vectors ----
  determinant() {
    return wrap(this._w.determinant(), this.context);
  }
  transpose() {
    return wrap(this._w.transpose(), this.context);
  }
  trace() {
    return wrap(this._w.trace(), this.context);
  }
  matrix_inverse() {
    return wrap(this._w.matrix_inverse(), this.context);
  }
  rref() {
    return wrap(this._w.rref(), this.context);
  }
  rank() {
    return this._w.rank();
  }
  matmul(other) {
    return wrap(this._w.matmul(toExpr(other, this.context)._w), this.context);
  }
  dot_prod(other) {
    return wrap(this._w.dot_prod(toExpr(other, this.context)._w), this.context);
  }
  cross_prod(other) {
    return wrap(this._w.cross_prod(toExpr(other, this.context)._w), this.context);
  }
  vector_add(other) {
    return wrap(this._w.vector_add(toExpr(other, this.context)._w), this.context);
  }
  vector_sub(other) {
    return wrap(this._w.vector_sub(toExpr(other, this.context)._w), this.context);
  }

  // ---- pattern matching (default mode only) ----
  match(pattern, _options) {
    const res = wasm.match_template(
      this._w.tree_json(),
      toExpr(pattern, this.context)._w.tree_json(),
    );
    return res === undefined ? false : JSON.parse(res);
  }
}

// Legacy methods with no Rust backing — defined so calls fail loudly, not as
// "undefined is not a function" surprises. Tests using them fail; suite runs.
for (const name of [
  "derivative_with_story",
  "derivative_story",
  "derivativeStory",
  "integrateNumerically",
  "toXML",
  "toGLSL",
  "toMathjs",
  "f",
  "solve_linear",
  "substitute_component",
  "get_component",
  "create_discrete_infinite_set",
  "expression_to_polynomial",
  "finite_field_evaluate",
]) {
  (Expression.prototype as Record<string, unknown>)[name] = notImplemented(name);
}

// Normalization / transformation passes with no standalone Rust entry point
// (folded into `canonicalize`). Compat no-ops that return the expression
// unchanged, so method chains still resolve and specs collect + run. Cases that
// depended on the pass mismatch and fail — as expected (JS_TEST_COVERAGE_AUDIT).
for (const name of [
  "default_order",
  "normalize_negative_numbers",
  "normalize_applied_functions",
  "expand_relations",
  "applyAllTransformations",
]) {
  (Expression.prototype as Record<string, unknown>)[name] = function (this: Expression) {
    return this;
  };
}

function parseText(string) {
  return new Expression(wasm.parse_text(string), Context);
}
function parseLatex(string) {
  return new Expression(wasm.parse_latex(string), Context);
}
function createFrom(expr) {
  if (typeof expr === "string") {
    try {
      return parseText(expr);
    } catch (e_text) {
      try {
        return parseLatex(expr);
      } catch (e_latex) {
        if (expr.indexOf("\\") !== -1) throw e_latex;
        throw e_text;
      }
    }
  }
  return Context.fromAst(expr); // number or AST
}

const Context = {
  from: createFrom,
  fromText: parseText,
  parse: parseText,
  fromLatex: parseLatex,
  fromLaTeX: parseLatex,
  fromTeX: parseLatex,
  fromTex: parseLatex,
  parse_tex: parseLatex,
  fromMml: notImplemented("fromMml"),
  fromAst(ast) {
    return new Expression(wasm.from_ast(JSON.stringify(ast)), Context);
  },
  reviver(key, value) {
    if (value && value.objectType === "math-expression" && value.tree !== undefined) {
      return Context.fromAst(value.tree);
    }
    return value;
  },
  isTree,
  math,
  converters,
  utils: { match, flatten, unflattenLeft, unflattenRight },
  class: Expression,

  // ---- assumptions (context-level) ----
  // Backed by a wasm `Assumptions` handle plus a parallel text list so
  // `simplify_with_assumptions` can be fed. `get_assumptions` is best-effort —
  // the original returned a richly-structured object this does not reproduce.
  _assumptionsHandle: new wasm.Assumptions(),
  _assumptionTexts: [],
  set_to_default() {
    this._assumptionsHandle = new wasm.Assumptions();
    this._assumptionTexts = [];
  },
  clear_assumptions() {
    this.set_to_default();
  },
  add_assumption(assumption) {
    const text = toExpr(assumption, this).toString();
    this._assumptionsHandle.add(text);
    this._assumptionTexts.push(text);
    return true;
  },
  add_generic_assumption(assumption) {
    return this.add_assumption(assumption);
  },
  remove_assumption(assumption) {
    const text = toExpr(assumption, this).toString();
    this._assumptionsHandle.remove(text);
    this._assumptionTexts = this._assumptionTexts.filter((t) => t !== text);
  },
  remove_generic_assumption(assumption) {
    return this.remove_assumption(assumption);
  },
  get_assumptions() {
    if (!this._assumptionTexts.length) return undefined;
    try {
      return Context.fromText(this._assumptionTexts.join(" and "));
    } catch {
      return undefined;
    }
  },
  get assumptions() {
    return this._assumptionsHandle;
  },
};

export { Expression };
export default Context;
