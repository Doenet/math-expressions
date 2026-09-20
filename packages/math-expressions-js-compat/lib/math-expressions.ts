// Drop-in replacement for the original `lib/math-expressions.js` default export
// (the `Context` factory + `Expression`), backed by the Rust/wasm core.
//
// Not every legacy method exists on the Rust side; those that don't are either
// approximated, or throw a clear "not implemented in js-compat" so the calling
// test fails cleanly (the suite still runs). See JS_TEST_COVERAGE_AUDIT.md.
import wasm, { onWasmModuleChange, setWasmModule } from "./_wasm";
import math from "./mathjs";
import { match, flatten, unflattenLeft, unflattenRight } from "./trees/flatten";
import * as converters from "./converters/index";
import { jsonToAst, tagNonFinite } from "./converters/ast-json";
import { renderOptions } from "./converters/render-options";
import * as assumptionStore from "./assumptions/store";
import { expression_to_polynomial } from "./polynomial/polynomial";
import { get_tree } from "./trees/util";
import { compileRustExpr } from "math-expressions-rs-wasm";
import type { WasmExpression } from "math-expressions-rs-wasm";
import type { MathJsInstance } from "mathjs";

// `me.math.pow_strict` — the legacy library carried this on its bundled mathjs
// instance to switch `0^0` (and the other indeterminate `x^0` forms) between
// `NaN` (strict) and `1`. The Rust core keeps it as ambient policy rather than
// baking it into an instance, so intercept the property on the shared `math`
// object and route it there. Assignment is the shape the spec uses
// (`me.math.pow_strict = false`), so a getter/setter is required — a method
// would not answer it.
Object.defineProperty(math, "pow_strict", {
  configurable: true,
  get(): boolean {
    return JSON.parse(wasm.get_constant_policy()).pow_strict;
  },
  set(value: boolean) {
    wasm.set_constant_policy(JSON.stringify({ pow_strict: Boolean(value) }));
  },
});

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
  if (
    Array.isArray(value) &&
    value.length > 0 &&
    typeof value[0] === "string"
  ) {
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
function wrap(
  handle: WasmExpression | undefined,
  context: Ctx,
): Expression | undefined {
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

/**
 * A component index is either a bare index or a path of them — `get_component(2)`
 * and `get_component([2, 1, 2])` are both legal, the first being the one-element
 * path. Indices count operands of the tree spelling, 0-based.
 */
function componentPath(component: number | number[]): Uint32Array {
  const path = Array.isArray(component) ? component : [component];
  return Uint32Array.from(path, (i) => Number(i));
}

/**
 * `JSON.stringify` replacer that preserves the non-finite numbers JSON cannot
 * hold. `JSON.stringify(NaN) === "null"` and likewise for `±Infinity`, so a
 * `NaN` slope or an infinite bound would reach the Rust boundary as `null` and
 * be rejected — the tree is serialized here on the way in, and this maps those
 * three values to the `{"$":…}` specials the Rust `from_ast` already reads back.
 * An already-special `{"$":"NaN"}` object passes through untouched.
 *
 * The *wire* format is tagged in both directions, because JSON cannot hold
 * these three values in either one. The *values a caller sees* are not: `.tree`
 * untags them back to JS scalars (see `untagNonFinite`), because `Infinity` is
 * what legacy handed back and what `typeof x === "number"` and `x === -Infinity`
 * consumers test against. `fromAst(x).tree` is still a fixpoint — this replacer
 * re-tags on the way in — it just holds at the value level rather than the wire
 * level. `{"$":"None"}` is the exception in both directions: it has no JS scalar
 * to untag to, and DoenetML emits and reads it in that form already.
 */
function astReplacer(this: unknown, key: string, value: unknown): unknown {
  // An `Expression` standing where a tree is expected — `fromAst(expr)`, or an
  // `expr` nested inside one (`["+", someExpr, 2]`). A math-valued DoenetML
  // state variable *holds* an Expression, so code that re-wraps one hands it
  // straight back here; this makes that a no-op instead of a throw.
  //
  // Note this reads the *holder* rather than `value`: `JSON.stringify` calls
  // `toJSON()` before consulting the replacer, so by the time `value` arrives an
  // Expression has already become its `{objectType:"math-expression",tree:…}`
  // envelope and `value instanceof Expression` is always false. That envelope is
  // precisely the "object with no `$` key" the Rust side used to reject.
  //
  // Both unwrapped trees go back through `tagNonFinite`: `.tree` hands out the
  // *untagged* scalars, so an `Expression` holding `NaN` or `±Infinity` would
  // otherwise be returned as a bare JS non-finite and `JSON.stringify` would
  // write `null` for it — the "unexpected value null" the Rust side rejects.
  // (Nested ones are covered by the fall-through below, which `stringify`
  // reaches when it walks into the value returned here.)
  const held = (this as Record<string, unknown> | undefined)?.[key];
  if (held instanceof Expression) return tagNonFinite(held.tree);
  // The same envelope arriving as plain data — a `JSON.parse` of a persisted
  // expression that never got run through `Context.reviver`. Keyed on the shape
  // `reviver` itself recognizes.
  if (isSerializedExpression(value)) return tagNonFinite(value.tree);
  // Shared with the standalone converters, so the two cannot tag `Infinity`
  // differently (see `converters/ast-json.ts`).
  return tagNonFinite(value);
}

/** The `toJSON()` envelope shape, as `Context.reviver` recognizes it. */
function isSerializedExpression(v: unknown): v is { tree: unknown } {
  return (
    !!v &&
    typeof v === "object" &&
    (v as { objectType?: unknown }).objectType === "math-expression" &&
    (v as { tree?: unknown }).tree !== undefined
  );
}

/**
 * Whether a call carries options worth forwarding to the wasm
 * `*_with_options` entry points — render options (padToDigits, padToDecimals,
 * showBlanks, explicitMultiplicationSymbols, notation, unicode) or parser
 * options (splitSymbols, appliedFunctionSymbols, …). An empty/absent object
 * takes the cheaper no-options path.
 */
function hasOptions(opts: unknown): opts is Record<string, unknown> {
  return !!opts && typeof opts === "object" && Object.keys(opts).length > 0;
}

/** A variable argument may be a string name or an Expression of a symbol. */
function varName(v: string | Expression): string {
  if (typeof v === "string") return v;
  if (v instanceof Expression) return v.toString();
  return String(v);
}

/** Whether a tree involves the imaginary unit `i` as a leaf — used to tell a
 * complex NaN (`Infinity*i` → `{re:NaN, im:NaN}`) apart from a real NaN
 * (`0/0` → scalar `NaN`), since both fold to a single `NaN`. */
function treeHasImaginary(tree: Tree): boolean {
  if (tree === "i") return true;
  return Array.isArray(tree) && tree.some((t) => treeHasImaginary(t));
}

/** Whether a tree contains a `det`/`trace` application — the matrix reductions
 * that only fold under `simplify`, so `evaluate_to_constant` retries them there
 * (but nowhere else, to avoid simplifying an undefined leaf into a number). */
function treeHasMatrixReduction(tree: Tree): boolean {
  if (Array.isArray(tree)) {
    if (tree[0] === "apply" && (tree[1] === "det" || tree[1] === "trace")) {
      return true;
    }
    return tree.some((t) => treeHasMatrixReduction(t));
  }
  return false;
}

/** The tree heads `get_component` will index — the JS library's set. */
const COMPONENT_CONTAINERS = new Set([
  "list",
  "tuple",
  "vector",
  "altvector",
  "array",
]);

/** The Context (`me`) shape, used for the back-reference on each Expression. */
type Ctx = typeof Context;

// Legacy `.equals` options are snake_case; the wasm entry points that read them
// — `Assumptions#equals_expressions` and `Expression#structural_equality_with_options`
// — take camelCase JSON keys. Map the ones the Rust side understands; drop the
// rest.
const EQ_OPTION_KEYS: Record<string, string> = {
  relative_tolerance: "relativeTolerance",
  absolute_tolerance: "absoluteTolerance",
  tolerance_for_zero: "toleranceForZero",
  allowed_error_in_numbers: "allowedErrorInNumbers",
  include_error_in_number_exponents: "includeErrorInNumberExponents",
  allowed_error_is_absolute: "allowedErrorIsAbsolute",
  allow_blanks: "allowBlanks",
  coerce_tuples_arrays: "coerceTuplesArrays",
  coerce_vectors: "coerceVectors",
};
function mapEqOptions(opts: EqualityOptions): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(opts)) {
    if (EQ_OPTION_KEYS[k]) out[EQ_OPTION_KEYS[k]] = v;
    else if (Object.values(EQ_OPTION_KEYS).includes(k)) out[k] = v; // already camelCase
  }
  return out;
}

/**
 * Handles for *recurring* atomic trees, and their (primitive) `.tree` readback.
 *
 * Evaluating a function over a domain drives `fromAst` in a tight loop, and
 * overwhelmingly on an atom: in one DoenetML `<evaluate>` test, 2.13M of 2.14M
 * calls were a bare number or the blank `"＿"` a domain miss returns, and
 * re-parsing those through `JSON.stringify` + `from_ast` was 40% of the run.
 * The handles are immutable, so a repeated atom can be built once and shared.
 *
 * The catch is which atoms actually repeat. Symbols, the blank, and small
 * integers do — 1.39M of those 2.13M calls were the single string `"＿"`. A
 * *sampled coordinate* does not: an interpolated function is evaluated at
 * millions of distinct floats, and caching those turns every call into a miss
 * plus table churn and holds a wasm handle per sample alive until the next
 * sweep. That is not merely a wash, it is a large loss: caching every atom
 * took the interpolated-function test from 77s to 153s while taking the
 * blank-driven one from 173s to 105s. Restricting the cache to strings and
 * small integers gives 78s and 4s — both faster than either. So an arbitrary
 * float goes straight to `from_ast`.
 *
 * `MAX_ATOMS` still bounds the table, since symbol names are unbounded over a
 * long session; on overflow it is dropped wholesale rather than evicted one at
 * a time, the working set being small and an atom cheap to re-parse.
 */
const ATOM_HANDLES = new Map<string, WasmExpression>();
const ATOM_TREES = new WeakMap<WasmExpression, unknown>();
/**
 * Handles the atom cache owns. A shared handle outlives any one wrapper, so
 * `free()` on a wrapper around one must not release it — see `free`.
 */
const ATOM_SHARED = new WeakSet<WasmExpression>();
/**
 * Live wrappers per shared handle, and the key each was cached under.
 *
 * Together these let `free()` release a handle the cache has since dropped
 * (`MAX_ATOMS` overflow, or a wasm-module swap) instead of leaving it to the
 * GC. Counting happens in the `Expression` constructor rather than in
 * `fromAst`, so a wrapper minted by any other route — `wrap`, the reviver, a
 * wasm call that hands back the same handle — is counted too; miss one and
 * `free()` would release a handle another live wrapper still points at.
 *
 * A wrapper that is garbage-collected without `free()` never decrements, which
 * only ever *inhibits* the release. The FinalizationRegistry is still the
 * backstop, so the failure direction is "freed late", never "freed early".
 */
const ATOM_REFS = new WeakMap<WasmExpression, number>();
const ATOM_KEYS = new WeakMap<WasmExpression, string>();
const MAX_ATOMS = 4096;

// Handles belong to the module that minted them, so a swap invalidates every
// cached one — passing a stale handle to the new module's `from_ast` fails with
// "expected instance of Expression". Dropping the table is enough: wrappers
// already handed out keep working against their own module, and the orphaned
// handles are released by `free()` or the GC as usual.
onWasmModuleChange(() => ATOM_HANDLES.clear());
/** Integers up to this magnitude are treated as recurring; see `atomKey`. */
const MAX_CACHED_INT = 1024;

/** Cache key for an atomic tree, or `undefined` if it is not worth caching. */
function atomKey(ast: unknown): string | undefined {
  if (typeof ast === "string") return "s" + ast;
  if (
    typeof ast === "number" &&
    Number.isInteger(ast) &&
    Math.abs(ast) <= MAX_CACHED_INT &&
    // `-0` and `0` are distinct expressions (see `Number::NegZero`); rather
    // than spell the sign into the key, leave `-0` uncached — it is rare, and
    // an uncached atom is correct, just not free.
    !Object.is(ast, -0)
  ) {
    return "n" + ast;
  }
  return undefined;
}

class Expression {
  _w: WasmExpression;
  context: Ctx;

  constructor(handle: WasmExpression, context?: Ctx) {
    this._w = handle;
    this.context = context || Context;
    if (ATOM_SHARED.has(handle)) {
      ATOM_REFS.set(handle, (ATOM_REFS.get(handle) ?? 0) + 1);
    }
  }

  // ---- inspection / rendering ----
  /**
   * The AST as plain JS data. `±Infinity` and `NaN` read back as the JS
   * scalars legacy handed out, not as their `{"$":…}` wire tags — see
   * `untagNonFinite`. `{"$":"None"}` stays tagged, having no scalar to become.
   */
  get tree() {
    // Memoized only when the tree is a primitive (a number, or a symbol/blank
    // string). A composite tree is handed out as a fresh array every read and
    // callers are free to mutate what they get back, so those must not be
    // shared; a primitive has nothing to mutate. Keyed on the wasm handle
    // rather than the wrapper because handles are immutable and are shared by
    // the atom cache below — `fromAst("＿")` is the single hottest call in
    // a function-evaluation loop, and this makes its `.tree` free after the
    // first read.
    const cached = ATOM_TREES.get(this._w);
    if (cached !== undefined) return cached;
    const tree = jsonToAst(this._w.tree_json());
    if (tree === null || typeof tree !== "object")
      ATOM_TREES.set(this._w, tree);
    return tree;
  }
  // Rendering honors the legacy render options (padToDigits, padToDecimals,
  // showBlanks, explicitMultiplicationSymbols, notation/unicode) by forwarding
  // a non-empty options object to the `*_with_options` wasm entry points.
  // It goes through `renderOptions` rather than a bare `JSON.stringify` so the
  // legacy spellings are translated, not silently dropped: callers pass
  // `output_unicode`, which the Rust side reads as `unicode`. The no-arg path
  // stays on the cheap no-options render — `toString()` is what JS coercion
  // (`String(expr)`) calls.
  toString(opts?) {
    return hasOptions(opts)
      ? this._w.to_text_with_options(renderOptions(opts))
      : this._w.to_text();
  }
  toText(opts?) {
    return hasOptions(opts)
      ? this._w.to_text_with_options(renderOptions(opts))
      : this._w.to_text();
  }
  toLatex(opts?) {
    return hasOptions(opts)
      ? this._w.to_latex_with_options(renderOptions(opts))
      : this._w.to_latex();
  }
  tex(opts?) {
    return hasOptions(opts)
      ? this._w.to_latex_with_options(renderOptions(opts))
      : this._w.to_latex();
  }
  toJSON() {
    return JSON.parse(this._w.to_serialized());
  }
  /**
   * The free variable names, in first-appearance order.
   *
   * `include_subscripts` reports a subscripted variable under its full name:
   * `x_1 + y` gives `["x_1", "y"]` rather than `["x", "y"]`. The argument was
   * dropped on the floor here, so a caller testing membership against a
   * subscripted name — `Line.js` deciding whether a coefficient mentions the
   * line's own variables — never found one.
   *
   * Implemented by flattening the subscript nodes into plain symbols first,
   * which is the same spelling `subscripts_to_strings` produces and the one
   * legacy's own `include_subscripts` pass builds.
   */
  variables(include_subscripts?: boolean) {
    const source = include_subscripts
      ? this._w.subscripts_to_strings(false)
      : this._w;
    try {
      return source.variables();
    } finally {
      if (source !== this._w) source.free();
    }
  }
  functions() {
    return this._w.functions();
  }
  /**
   * This expression read as a polynomial — `["polynomial", v, [[deg, coeff], …]]`
   * — or `false` when it is not one. See `lib/polynomial/polynomial`.
   */
  expression_to_polynomial() {
    return expression_to_polynomial(this.tree);
  }

  // ---- equality ----
  equals(other, options?) {
    const o = toExpr(other, this.context);
    // Routed through the context's assumption store, not `this._w.equals`.
    // Discrete infinite sets are the one stage of the chain that needs it — the
    // comparison divides by the period, so a symbolic period means nothing
    // until it is known nonzero — and the legacy `equals` read the context's
    // assumptions for exactly that stage. Every other stage is assumption-free
    // and answers identically, so this costs a dispatch, not a second pass:
    // the store's method falls straight through to the plain chain when neither
    // side is a set.
    return this.context._assumptionsHandle.equals_expressions(
      this._w,
      o._w,
      options && Object.keys(options).length > 0
        ? JSON.stringify(mapEqOptions(options))
        : undefined,
    );
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
  equalsViaSyntax(other, options?) {
    const o = toExpr(other, this.context)._w;
    if (hasOptions(options)) {
      return this._w.structural_equality_with_options(
        o,
        '"sameStructure"',
        JSON.stringify(mapEqOptions(options)),
      );
    }
    return this._w.structural_equality(o, '"sameStructure"');
  }
  is_zero() {
    return this._w.is_zero();
  }
  isAnalytic(opts) {
    const o = opts || {};
    return this._w.is_analytic(
      !!o.allow_abs,
      !!o.allow_arg,
      !!o.allow_relation,
    );
  }

  // ---- calculus ----
  derivative(v) {
    return wrap(this._w.derivative(varName(v)), this.context);
  }
  integrate(v) {
    return wrap(this._w.integrate(varName(v)), this.context);
  }
  // Best-effort numeric definite integral. Unlike the original JS (which always
  // returns an uncertified estimate), this is backed by the CERTIFIED
  // quadrature and returns `NaN` when the value cannot be certified — never a
  // silently-wrong number.
  integrateNumerically(v, lower, upper) {
    const r = this._w.integrate_numerically(
      varName(v),
      Number(lower),
      Number(upper),
    );
    return r === undefined ? NaN : r;
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
  /**
   * Sort into the default order without evaluating — DoenetML's
   * `simplify="normalizeOrder"`. Unlike `simplify`, every term survives:
   * `0x^2` stays, `7+4` stays two terms, `1x^2` keeps its coefficient. The
   * ordering key is the JS library's, quirks included, because the term
   * sequence it produces is what gets displayed.
   */
  default_order() {
    return wrap(this._w.default_order(), this.context);
  }
  factor() {
    return wrap(this._w.factor(), this.context);
  }
  /**
   * Push every unary minus into the numeric literal it negates, bottom-up:
   * `-(3x)` → `(-3)x`, `-(3/y)` → `(-3)/y`, `-3` → the number `-3`.
   *
   * A normalization for pattern matching, not a simplification — `3x + 4y - 2x`
   * has to become `3x + 4y + (-2)x` before a `n·x + m·x` rule can see `-2` as a
   * coefficient. Nothing else changes, and a minus with no literal to fold into
   * (`-x`) stays where it is.
   *
   * Implemented over the raw AST rather than through the core because that is
   * what it is for: the trees it feeds are matched structurally, and
   * canonicalizing would reorder and re-fold them out from under the pattern.
   */
  collapse_unary_minus() {
    const collapse = (tree) => {
      if (!Array.isArray(tree)) return tree;
      const [operator, ...operands] = tree.map((t, i) =>
        i === 0 ? t : collapse(t),
      );
      if (operator === "-") {
        const operand = operands[0];
        if (typeof operand === "number") return -operand;
        if (Array.isArray(operand)) {
          // A product whose leading factor is a literal: negate that factor.
          if (operand[0] === "*" && typeof operand[1] === "number")
            return ["*", -operand[1], ...operand.slice(2)];
          // A quotient: the numerator is either a literal itself or a product
          // led by one. Only the numerator moves; negating a denominator would
          // change the value's spelling for no gain.
          if (operand[0] === "/") {
            const [, numerator, denominator] = operand;
            if (typeof numerator === "number")
              return ["/", -numerator, denominator];
            if (
              Array.isArray(numerator) &&
              numerator[0] === "*" &&
              typeof numerator[1] === "number"
            )
              return [
                "/",
                ["*", -numerator[1], ...numerator.slice(2)],
                denominator,
              ];
          }
        }
      }
      return [operator, ...operands];
    };
    return this.context.fromAst(collapse(this.tree));
  }
  evaluate_numbers(opts) {
    // `skip_ordering` (DoenetML's `simplify="numberspreserveorder"`) selects a
    // genuinely different core pass: numbers fold only with *adjacent* numbers,
    // so `1+x+2` stays `1+x+2` where the ordering form gives `x+3`. It used to
    // throw here, which was worse than a missing feature — the Rust core calls
    // this mode and is built `panic = "abort"`, so the exception unwound into
    // it as a WASM trap and took the whole worker down.
    // `max_digits` is how many significant digits the caller is willing to
    // spend turning an exact value into a decimal. `Infinity` — spend as many
    // as it takes — folds `π` and `1/3` too, which is what makes
    // `2π + π + 6` comparable against a response typed as `15.42478`; it is
    // what DoenetML's grading path passes. A *finite* cap converts only the
    // rationals whose decimal fits that many significant figures: `1/2 → 0.5`
    // at any budget ≥ 1, but `1/3` stays exact (its decimal never terminates)
    // and `π` stays symbolic (an irrational is never captured by a finite
    // count). Both go through the same digit-budget core; only `undefined`
    // (omit) keeps every exact value.
    const maxDigits = opts?.max_digits;
    if (
      maxDigits !== undefined &&
      maxDigits !== Infinity &&
      !(Number.isInteger(maxDigits) && maxDigits >= 0)
    ) {
      throw new Error(
        `evaluate_numbers: 'max_digits' must be a non-negative integer or Infinity (got ${maxDigits}).`,
      );
    }
    const skipOrdering = Boolean(opts?.skip_ordering);
    // `evaluate_functions` additionally folds a function applied to a numeric
    // argument (`sin(0)+2` → `2`), which is what `simplify="full"` needs.
    const evaluateFunctions = Boolean(opts?.evaluate_functions);
    let result;
    if (maxDigits !== undefined) {
      result = wrap(
        this._w.evaluate_numbers_to_floats(
          skipOrdering,
          evaluateFunctions,
          maxDigits,
        ),
        this.context,
      );
    } else if (skipOrdering) {
      result = wrap(this._w.evaluate_numbers_preserve_order(), this.context);
    } else if (evaluateFunctions) {
      result = wrap(
        this._w.evaluate_numbers_evaluate_functions(),
        this.context,
      );
    } else {
      result = wrap(this._w.evaluate_numbers(), this.context);
    }
    // `set_small_zero` drops residual round-off (`10x + 5e-15` → `10x`) after the
    // numeric fold. `true` uses the default tolerance; a number sets it. Mirrors
    // the standalone `set_small_zero()` method the legacy option delegated to.
    const ssz = opts?.set_small_zero;
    if (ssz) {
      // `set_small_zero` leaves the zeroed term in place (`10x + 0`); re-fold to
      // drop it (`10x`) and collapse `0·x → 0`. Same options minus `set_small_zero`
      // so this does not recurse.
      result = result
        .set_small_zero(ssz === true ? undefined : ssz)
        .evaluate_numbers({
          skip_ordering: skipOrdering,
          evaluate_functions: evaluateFunctions,
          max_digits: maxDigits,
        });
    }
    return result;
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
  normalize_applied_functions() {
    return wrap(this._w.normalize_applied_functions(), this.context);
  }
  normalize_negative_numbers() {
    return wrap(this._w.normalize_negative_numbers(), this.context);
  }
  expand_relations() {
    return wrap(this._w.expand_relations(), this.context);
  }
  constants_to_floats() {
    return wrap(this._w.constants_to_floats(), this.context);
  }

  // ---- solving ----
  /**
   * Restate a relation with `variable` alone on the left: `3x+4 = 2` in `x`
   * becomes `x = -2/3`, and an inequality flips when the coefficient is
   * negative (`2x-4 < 6+4x` → `x > -5`).
   *
   * Routed through the context's assumption store rather than the free
   * `solve_linear_ast`, which is deliberately assumption-blind. The facts on
   * file are what decide two of the three outcomes here: `2uv-v = 3u+q` has no
   * answer in `u` until something makes `2v-3` nonzero, and an inequality's
   * direction is unknowable until the coefficient's sign is.
   *
   * When there is no answer this returns the {@link ABSENT_EXPRESSION}
   * stand-in, not `undefined` — legacy handed back an `Expression` whose `.tree`
   * was `undefined`, and callers read `.tree` off the result unconditionally.
   */
  solve_linear(variable) {
    const solved = this.context._assumptionsHandle.solve_linear(
      this._w,
      varName(variable),
    );
    return solved === undefined
      ? ABSENT_EXPRESSION
      : wrap(solved, this.context);
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
  // Move `+`/scalar-`*` inside vector & matrix containers so grading can slice
  // the result into components. Not arithmetic — it deliberately leaves `1+3`
  // rather than folding to `4` (`checkEquality` compares components under
  // tolerance). Mirrored onto `Context`, so `me.perform_…(expr)` works too.
  perform_vector_matrix_additions_scalar_multiplications() {
    return wrap(
      this._w.perform_vector_matrix_additions_scalar_multiplications(),
      this.context,
    );
  }
  // `force` also collapses a compound subscript, by its text spelling —
  // `(x^3)_2` becomes that seven-character symbol name.
  subscripts_to_strings(force = false) {
    return wrap(this._w.subscripts_to_strings(force), this.context);
  }
  strings_to_subscripts() {
    return wrap(this._w.strings_to_subscripts(), this.context);
  }
  copy() {
    return wrap(this._w.copy(), this.context);
  }

  // ---- lifetime ----
  // Every Expression owns a Rust/wasm handle that is otherwise only reclaimed by
  // the JS GC's FinalizationRegistry — too late for DoenetML's long-lived worker,
  // which mints a handle per state-variable eval and per state-JSON revive. `free`
  // releases it eagerly. Idempotent: the handle is nulled, so freeing twice is a
  // no-op rather than the wasm-memory corruption a double free would cause, and a
  // later method call fails on the null handle (a TypeError naming the method)
  // instead of reading through a dangling pointer.
  free() {
    const w = this._w as WasmExpression | undefined;
    if (!w) return;
    this._w = undefined as unknown as WasmExpression;
    if (!ATOM_SHARED.has(w)) {
      w.free();
      return;
    }
    // A handle from the atom cache is shared by every wrapper `fromAst` has
    // handed out for that atom, so releasing it on the first `free()` would
    // dangle the others. Release it only once this is the last live wrapper
    // *and* the cache itself has let go — after a `MAX_ATOMS` sweep or a wasm
    // swap the handle is an orphan nothing will hand out again, and leaving it
    // to the GC is what made `free()` a silent no-op for atoms. While the
    // handle is still cached it stays alive by design.
    const refs = (ATOM_REFS.get(w) ?? 0) - 1;
    ATOM_REFS.set(w, refs);
    const key = ATOM_KEYS.get(w);
    if (refs <= 0 && (key === undefined || ATOM_HANDLES.get(key) !== w)) {
      ATOM_SHARED.delete(w);
      w.free();
    }
  }
  // Aliases: `dispose()` and the `using`-statement protocol.
  dispose() {
    this.free();
  }

  // ---- component access ----
  // `component` is an operand index into the tree spelling, or a path of them
  // for nested components. A matrix is `["matrix", ["tuple", rows, cols],
  // ["tuple", <row-tuples>]]`, so an entry of one is `[1, row, col]`.
  /**
   * The `component`-th operand of a **container** — a list, tuple, vector,
   * altvector or array.
   *
   * **Throws** for anything else, which is the legacy contract and what
   * callers are written against: DoenetML wraps this in `try/catch` and reads
   * the throw as "not a container, use the value whole". Two things went wrong
   * without it. The wasm entry point indexes the operands of *any* operator
   * (its paths are over the flattened JS tree, which is right for what it is
   * used for internally), so `xyz` — a product — reported its first factor as
   * `.x`, and a scalar reported `undefined`, which read as a container holding
   * nothing.
   */
  get_component(component) {
    const t = this.tree;
    if (!Array.isArray(t) || !COMPONENT_CONTAINERS.has(t[0])) {
      throw Error(
        "Invalid get_component: expected list, tuple, vector, or array",
      );
    }
    const got = this._w.get_component(componentPath(component));
    if (got === undefined) {
      throw Error(
        "Invalid get_component: expected list, tuple, vector, or array",
      );
    }
    return wrap(got, this.context);
  }
  substitute_component(component, value) {
    return wrap(
      this._w.substitute_component(
        componentPath(component),
        toExpr(value, this.context)._w,
      ),
      this.context,
    );
  }

  // ---- numeric evaluator ----
  // The plotting / root-finding entry point: compile once through math.js, then
  // evaluate per sample. `compileRustExpr` normalizes function names Rust-side
  // and frees its own temporary handle; `this._w` is untouched.
  f() {
    // `./mathjs` re-exports either a created instance or the namespace itself,
    // so its static type is a union; the runtime value is always an instance.
    const compiled = compileRustExpr(math as MathJsInstance, this._w);
    return (bindings = {}) => compiled.evaluate(bindings);
  }

  /**
   * The critical points with respect to `variable` — the real solutions of
   * `d/dvariable = 0` — exactly, in increasing order.
   *
   * Three outcomes, and a caller that keeps a numerical fallback needs to tell
   * them apart: an array of points; an **empty** array, meaning there are
   * provably none; and `null`, meaning undecided — sample instead. Undecided
   * is a derivative that is not a rational function of `variable` (`cos(x)`,
   * which has infinitely many roots anyway), one carrying a free parameter
   * (`d/dx a·x²`, whose roots depend on `a`), or a constant-zero derivative,
   * where every point is critical and no finite list says so.
   *
   * Exact means exact: a rational root comes back as a number, an algebraic one
   * as the `rootof` form carrying its defining polynomial, and a repeated root
   * is listed once. Points where the derivative does not *exist* — the corner
   * of `|x|` — are not reported; they are critical in the textbook sense, but
   * finding them is not rational root-finding.
   */
  critical_points(variable) {
    // Through `varName`, like `derivative`/`integrate`/`solve_linear`: the
    // wasm binding takes a `&str` and computes a length on whatever it is
    // handed, so an `Expression` argument reads out of bounds rather than
    // failing.
    const pts = this._w.critical_points(varName(variable));
    return pts === undefined
      ? null
      : pts.map((p) => new Expression(p, this.context));
  }

  // ---- units ----
  remove_units(scaleBasedOnUnit) {
    // Legacy default scales (`50%` → `0.5`, `180deg` → `π`); pass `false` to
    // keep the bare value (`50%` → `50`).
    const scale = scaleBasedOnUnit === undefined ? true : !!scaleBasedOnUnit;
    return wrap(this._w.remove_units(scale), this.context);
  }
  remove_scaling_units() {
    return wrap(this._w.remove_scaling_units(), this.context);
  }
  add_unit(unit) {
    // `varName`, for the same reason `critical_points` uses it: the wasm entry
    // point is `add_unit(unit: &str)`, and wasm-bindgen reads a non-string
    // argument as a pointer/length pair. The published declaration invites an
    // `Expression | Tree` here — legacy took one — and handing it either read
    // out of bounds (`RuntimeError: memory access out of bounds`) or threw
    // `arg.charCodeAt is not a function`. A unit is a symbol, so its name is
    // all the Rust side wants.
    return wrap(this._w.add_unit(varName(unit)), this.context);
  }
  set_small_zero(tolerance) {
    return wrap(
      this._w.set_small_zero(tolerance === undefined ? 1e-14 : tolerance),
      this.context,
    );
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
  // Two return shapes, and neither is `null`: a `number` — where `NaN` is the
  // "no numeric value" marker, as legacy's was — or a math.js `Complex` for a
  // non-real value. The wasm entry point reports only the real case; the
  // complex one comes back through `evaluate_to_complex`.
  //
  // Legacy returned a plain number for a real value and a complex value for a
  // non-real one, so `fromText("i").evaluate_to_constant()` is `{re:0, im:1}`,
  // not NaN.
  //
  // The complex value is a math.js `Complex`, as legacy's was: callers pass it
  // straight into math.js functions (`divide(evaluate_to_constant(a), …)`),
  // which reject a plain object. A consumer that puts one into a *state
  // variable* should flatten it there — it is structured-cloned to the main
  // thread and arrives prototype-stripped either way.
  evaluate_to_constant(opts) {
    // Units are scaled away first by default (`50%` → `0.5`, `180deg` → `π`):
    // `remove_units_first` (default true) strips them, `scale_based_on_unit`
    // (default true) applies the unit's factor. With `remove_units_first:false`
    // a unit-bearing value has no numeric constant, so it falls through to NaN.
    let e = this as unknown as Expression;
    if (opts?.remove_units_first ?? true) {
      e = e.remove_units(opts?.scale_based_on_unit ?? true);
    }
    const v = e._w.evaluate_to_constant();
    if (v !== undefined) {
      // A non-finite value of a *complex* expression has no defined direction —
      // `Infinity*i` and `Infinity*i + Infinity` are complex NaN
      // (`{re:NaN, im:NaN}`), matching mathjs. A real non-finite value stays as
      // it is (`Infinity`, or scalar `NaN` for `0/0` / `Infinity - Infinity`).
      if (!Number.isFinite(v) && treeHasImaginary(e.tree as Tree)) {
        return math.complex(NaN, NaN);
      }
      return v;
    }
    const c = e._w.evaluate_to_complex();
    if (c !== undefined) return math.complex(c[0], c[1]);
    // `det`/`trace` of a literal matrix only reduce to a number under
    // simplification (`\det[[1,2],[3,4]]` → −2), so retry once via the simplified
    // form — but *only* for those, since simplification would also absorb an
    // undefined leaf (`0·＿` → `0`) and wrongly turn a `null` into a number.
    if (treeHasMatrixReduction(e.tree as Tree)) {
      const s = e.simplify();
      const sv = s._w.evaluate_to_constant();
      if (sv !== undefined) return sv;
      const sc = s._w.evaluate_to_complex();
      if (sc !== undefined) return math.complex(sc[0], sc[1]);
    }
    // Not a constant: `NaN`, as legacy answered. Everything that reaches here —
    // a free variable (`x+1`), a blank `＿`, a placeholder hole (`0·_`, `_/_`),
    // a matrix, a leftover unit — is "no numeric value", and legacy spelled all
    // of them `NaN`.
    //
    // This used to be `null` for the free-variable and placeholder cases, on
    // the grounds that "cannot be evaluated" is worth telling apart from
    // "evaluates to NaN". The distinction is real, but `null` is the wrong way
    // to carry it across into JavaScript, and it was carrying it into every
    // consumer whether or not the consumer had asked. `null` is *anti*-
    // poisoning: `Number(null)` is `0`, `null + 5` is `5`, `null <= 1` is
    // `true`, `Number.isNaN(null)` is `false`. So an expression with no value
    // silently behaved like zero — a rectangle 0 wide, a line with slope 1, a
    // blank answer scoring full credit. `NaN` does the opposite: it propagates
    // through arithmetic and falsifies every comparison, which is what a
    // "no value" marker has to do to be safe by default.
    //
    // A caller that genuinely needs "unevaluable" apart from "evaluates to NaN"
    // can still get it — `variables()` reports the free variables, and the tree
    // is right there — but it has to ask, and the default is the safe one.
    //
    // Legacy's `nan_for_non_numeric` option is still *not* honored: this path
    // always behaves as its `true` default, which is now also the only
    // behavior. Passing `{nan_for_non_numeric: false}` does not produce `null`.
    return NaN;
  }
  /**
   * The complex half of `evaluate_to_constant`, on its own.
   *
   * This one *does* answer `null`, and deliberately, unlike
   * `evaluate_to_constant`. Two reasons it is not the same hazard. It has no
   * legacy counterpart, so there is no drop-in contract saying otherwise; and
   * its range already contains `Complex(NaN, NaN)` as a genuine value
   * (`Infinity*i`), so `NaN` cannot double as the "no value" marker here the
   * way it can for a real result. A `Complex` never coerces silently either —
   * math.js rejects `null` loudly rather than reading it as `0`.
   *
   * Not part of the published `types/math-expressions.d.ts` surface.
   */
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
  /**
   * Evaluate at many values of one variable in a single crossing.
   *
   * `evaluate` marshals the variable names on every call, which costs far more
   * than the arithmetic — measured at ~1.2µs a point against ~6ns of actual
   * work on `x²−3x+1`. Sampling a curve, scanning for extremum brackets or
   * hunting a root asks the same question thousands of times, and this pays
   * that overhead once.
   *
   * Any other variable is left unbound; `substitute` it first. The result is a
   * `Float64Array` the same length as `values`, with `NaN` wherever there is no
   * finite real value — a pole, a complex branch, an unbound variable — so it
   * lines up index-for-index with what was asked and the gaps carry the marker
   * consumers already test for.
   */
  evaluate_many(variable, values) {
    // `varName` for the same reason as `critical_points` above.
    return this._w.evaluate_many(
      varName(variable),
      values instanceof Float64Array ? values : Float64Array.from(values),
    );
  }
  /**
   * Replace variables by their bindings, all at once.
   *
   * Simultaneous, as the JS library was: no binding sees another's
   * replacement, so `a·x + b·y` with `{a: "b", b: "a"}` swaps the two
   * coefficients rather than collapsing both to `a`.
   *
   * This *was* a left-to-right pass here, on the stated grounds that legacy
   * was one too and that DoenetML relied on a substituted `<math>` code
   * expanding into further codes. Neither holds — legacy walks the tree once
   * (`trees/basic.js`), and `{c1: "c2", c2: 5}` leaves `c2` standing there as
   * well. A sequential pass silently captures instead: `sin(x+y)` with
   * `{x: "10y", y: "-π"}` answered `sin(-10π − π)`, and DoenetML substitutes
   * variable names into `a·x + b·y + c` in `Line.js`, where a document
   * declaring `variables="y x"` put both coefficients on one variable.
   *
   * Differs from {@link substitute_all} only in coercing each binding the way
   * the rest of this API does — a string is *parsed* (`{x: "2y"}` binds the
   * product `2y`, not a symbol spelled `"2y"`), matching legacy.
   */
  substitute(bindings) {
    const keys = Object.keys(bindings || {});
    if (keys.length === 0) return this;
    const map = {};
    for (const k of keys) map[k] = toExpr(bindings[k], this.context);
    return wrap(
      this._w.substitute_map(JSON.stringify(map, astReplacer)),
      this.context,
    );
  }

  /**
   * Replace variables by their bindings **simultaneously**, taking each
   * binding as the tree it already is.
   *
   * Same substitution as {@link substitute}; the difference is coercion. This
   * one serializes the binding as given, so a string binds a *symbol* of that
   * name (`{x: "2y"}` binds the single symbol `2y`), where `substitute` parses
   * it into the product `2·y`. Reach for this when the bindings are trees or
   * `Expression`s and there is nothing to parse.
   */
  substitute_all(bindings) {
    const keys = Object.keys(bindings || {});
    if (keys.length === 0) return this;
    const map = {};
    for (const k of keys) map[k] = bindings[k];
    return wrap(
      this._w.substitute_map(JSON.stringify(map, astReplacer)),
      this.context,
    );
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
    return wrap(
      this._w.cross_prod(toExpr(other, this.context)._w),
      this.context,
    );
  }
  vector_add(other) {
    return wrap(
      this._w.vector_add(toExpr(other, this.context)._w),
      this.context,
    );
  }
  vector_sub(other) {
    return wrap(
      this._w.vector_sub(toExpr(other, this.context)._w),
      this.context,
    );
  }
  // `me.scalar_mul(scalar, vector)` mirrors to `toExpr(scalar).scalar_mul(vector)`,
  // so `this` is the scalar and `other` the vector.
  scalar_mul(other) {
    return wrap(
      this._w.scalar_mul(toExpr(other, this.context)._w),
      this.context,
    );
  }

  // ---- pattern matching (default mode only) ----
  /**
   * Template match against `pattern`. Options:
   *
   * - `variables` — the declared parameters, as `{name: kind}` where kind is
   *   `true`/`"any"`, `"number"` or `"variable"`. Present-and-empty declares
   *   *no* parameters, so only an exact match succeeds; omitting the option
   *   keeps the legacy default where every string leaf in the pattern binds.
   * - `allow_permutations` — match `+`/`*` operands in any order.
   * - `allow_implicit_identities` — array of parameter names that may take the
   *   operator's identity, so `a x + b` matches `x` with `a = 1`, `b = 0`.
   * - `allow_extended_match` — let a `+`/`*` pattern match a *subset* of a
   *   larger sum or product, reporting the untouched operands as `_skipped`.
   *
   * The kinds replace the JS predicates the legacy API took: a function cannot
   * cross the wasm boundary, and these three are what the predicates expressed.
   * A predicate is therefore rejected rather than ignored — silently treating
   * one as "any" is what made `requireNumericMatches` a no-op.
   */
  match(pattern, options?) {
    // Delegated to the shared implementation, which is what makes the claim
    // that this and `me.utils.match` cannot drift true. It used to share only
    // `normalizeMatchOptions` and call the wasm matcher itself, and
    // `allow_extended_match` is handled *outside* that matcher — so the two
    // entry points answered differently for the same call:
    // `("x+y+z").match("a+b", {variables: {a: true, b: true},
    // allow_extended_match: true})` bound `b` to `y+z` here and to `y`, with
    // `_skipped: ["z"]`, through `me.utils.match`. Legacy's
    // `Expression.prototype.match` delegated for the same reason.
    //
    // `.tree`, not `_w.tree_json()`, because the shared entry takes trees; and
    // the pattern still goes through `toExpr` first, since a string pattern is
    // a *parse* here and would be a bare leaf to `astToJson`.
    //
    // `hasOptions`, not the shared `hasParams`, so an empty options object
    // keeps taking the cheaper no-options path — which is also the path whose
    // legacy default lets every string leaf in the pattern bind. Bindings come
    // back through `jsonToAst` in there, not bare `JSON.parse`: they are
    // subtrees, and `.tree` hands subtrees out untagged, so returning
    // `{a: {$: "Inf"}}` would contradict the convention the rest of the
    // surface follows — and break the `typeof m.a === "number"` consumers
    // legacy supported.
    return match(
      this.tree,
      toExpr(pattern, this.context).tree,
      hasOptions(options) ? options : undefined,
    );
  }
}

// The `using` protocol, attached only where the runtime actually has the symbol
// (Node ≥ 18.18, Chrome ≥ 125, Safari ≥ 18.4). Written as a class member,
// `[Symbol.dispose]() {}` on an engine without it would define a method keyed by
// the *string* "undefined" — silently useless rather than absent, and `free()`
// would never run. Feature-detecting keeps `using expr = me.fromText(…)` working
// where it is supported and simply unavailable where it is not.
if (typeof Symbol.dispose === "symbol") {
  (Expression.prototype as unknown as Record<symbol, unknown>)[Symbol.dispose] = function (
    this: Expression,
  ) {
    this.free();
  };
}

/**
 * The "no answer" result from a method that can fail to produce an expression
 * at all — currently only {@link Expression.solve_linear}.
 *
 * Legacy funnelled every tree-returning helper through `context.fromAst(...)`
 * (`extend_prototype` in the old `math-expressions.js`), so a helper that
 * returned `undefined` still handed back a real `Expression` — one whose `.tree`
 * was `undefined`. Callers, the specs included, read `.tree` off the result
 * without checking, so returning a bare `undefined` here would turn "unsolvable"
 * into a `TypeError`.
 *
 * A js-compat `Expression` is always backed by a wasm handle and no handle
 * spells "absent", so this is a separate object rather than an `Expression`.
 * Its shape is what the live legacy oracle actually hands out for an unsolvable
 * relation, checked case by case: `.tree` is `undefined`, `toString()` and
 * `toLatex()` are `""`, `equals(…)` is `false`, `variables()` is `[]`. Every
 * other `Expression` method returns the stand-in itself, so chaining off an
 * unsolvable relation stays absent instead of throwing — legacy's own chained
 * results were an artifact of wrapping `undefined` and not worth reproducing.
 */
const ABSENT_EXPRESSION = (() => {
  const absent: Record<string, unknown> = {
    tree: undefined,
    // A getter because `Context` is initialized further down this module and
    // this runs during its evaluation.
    get context() {
      return Context;
    },
    toString: () => "",
    toLatex: () => "",
    variables: () => [],
    equals: () => false,
    // Deliberately not self-returning: a `toJSON` handing back the stand-in
    // makes `JSON.stringify` recurse until the stack goes, and there is no
    // handle to free.
    toJSON: () => undefined,
    free: () => {},
  };
  for (const name of Object.getOwnPropertyNames(Expression.prototype)) {
    const d = Object.getOwnPropertyDescriptor(Expression.prototype, name);
    if (name === "constructor" || name in absent) continue;
    if (typeof d?.value !== "function") continue; // a getter has no `value`
    absent[name] = () => absent;
  }
  return Object.freeze(absent);
})();

// Legacy methods with no Rust backing — defined so calls fail loudly, not as
// "undefined is not a function" surprises. Tests using them fail; suite runs.
for (const name of [
  "derivative_with_story",
  "derivative_story",
  "derivativeStory",
  "toXML",
  "toGLSL",
  "toMathjs",
  "finite_field_evaluate",
]) {
  (Expression.prototype as unknown as Record<string, unknown>)[name] =
    notImplemented(name);
}

// Normalization passes with no faithful Rust entry point (folded into
// `canonicalize`). Kept as no-ops returning `this` rather than throwing: a
// blanket throw here regressed ~170 idempotent-input specs that legitimately
// pass on the unchanged tree, and aborted whole spec files at collection. The
// real fix is implementing them; see DOENET_COMPAT_PLAN R7 and the follow-up note.
// `default_order` graduated out of this list — it has a real implementation
// now (`normalize::default_order`), carrying the JS ordering key rather than
// the Rust canonical `cmp`, because the order it produces is displayed. So did
// `normalize_negative_numbers` and `normalize_applied_functions`: the passes
// they name were already in the Rust core as `normalize_syntactic`'s second and
// third steps, and are now exported individually. And so did `expand_relations`,
// which the assumptions store had been using all along
// (`assumptions::expand::expand_relations`) — only the public binding was
// missing.
for (const name of ["applyAllTransformations"]) {
  (Expression.prototype as unknown as Record<string, unknown>)[name] = function (
    this: Expression,
  ) {
    return this;
  };
}

// The parser options object is the legacy second argument (`splitSymbols`,
// `appliedFunctionSymbols`, `functionSymbols`, `operatorSymbols`, …). It was
// being dropped on the floor here, which mattered most for
// `appliedFunctionSymbols`: without it there is no way to get `sum(1,2,3)` to
// parse as an application rather than as `s·u·m·(1,2,3)`, since neither this
// library nor the legacy one lists the aggregates by default.
/**
 * The legacy library threw a `ParseError` — an `Error` subclass whose `name`
 * said so — and callers narrow on that name to tell "you typed something I
 * cannot read", which is worth showing a student, from any other failure, which
 * is not. `wasm-bindgen` throws a plain `Error`, so that name was lost and the
 * narrowing silently stopped matching: DoenetML's `<mathInput showPreview>` has
 * a slot for the parser's complaint and had been rendering nothing in it.
 *
 * The message is the engine's own and is already the useful part
 * (`Expecting } (at 7)`, `Invalid symbol '@' (at 0)`); only the label was
 * missing. `cause` keeps the original for anyone who wants the stack.
 */
function asParseError(e: unknown) {
  if (e instanceof Error && e.name === "Error") {
    e.name = "ParseError";
    return e;
  }
  if (e instanceof Error) {
    return e;
  }
  // wasm-bindgen can reject with a bare string.
  const wrapped = new Error(String(e), { cause: e });
  wrapped.name = "ParseError";
  return wrapped;
}

/**
 * Reject a non-string at the parser boundary, with a message that says so.
 *
 * `parse_text`/`parse_latex` are declared `(s: &str)` on the Rust side, and
 * wasm-bindgen reads a non-string argument as a pointer/length pair into linear
 * memory: `me.fromText(5)` and `me.fromText(anExpression)` both came out as
 * `RuntimeError: memory access out of bounds`, and an array tree as
 * `arg.charCodeAt is not a function`. These are the package's two most-used
 * entry points, and the second message reaches a student — `<mathInput
 * showPreview>` renders whatever the parser complains about.
 *
 * A *throw* is right here where `add_unit` takes a coercion: `add_unit`'s
 * declaration invites an `Expression | Tree` and a unit is a symbol, so its
 * name is a faithful reading; `fromText` is declared to take a `string` and
 * there is no faithful reading of anything else. So this changes no call that
 * used to succeed — it only replaces an engine-internal failure with a
 * diagnosable one.
 *
 * `String` *objects* are accepted: they carry `.length` and `.charCodeAt`, so
 * wasm-bindgen has always read them correctly and rejecting them here would be
 * a new restriction rather than a clearer message.
 */
function parseInput(s: unknown, what: "fromText" | "fromLatex"): string {
  if (typeof s === "string") return s;
  if (s instanceof String) return String(s);
  throw new TypeError(
    `${what}: expected a string, got ${s === null ? "null" : typeof s}. ` +
      "Use `me.fromAst` for an AST tree and `me.from` for an Expression.",
  );
}

function parseText(string, opts?) {
  const text = parseInput(string, "fromText");
  try {
    return new Expression(
      hasOptions(opts)
        ? wasm.parse_text_with_options(text, JSON.stringify(opts))
        : wasm.parse_text(text),
      Context,
    );
  } catch (e) {
    throw asParseError(e);
  }
}
function parseLatex(string, opts?) {
  const latex = parseInput(string, "fromLatex");
  try {
    return new Expression(
      hasOptions(opts)
        ? wasm.parse_latex_with_options(latex, JSON.stringify(opts))
        : wasm.parse_latex(latex),
      Context,
    );
  } catch (e) {
    throw asParseError(e);
  }
}
function createFrom(expr) {
  // "Nothing" converts to nothing. `fromAst(undefined)` reaches the core as a
  // literal `undefined` string and dies inside the parser with a
  // `Cannot read properties of undefined` — but callers do write
  // `me.from(value)` over a table whose empty rows mean "no expression", and
  // the legacy library handed those back an expression with an undefined tree
  // that every consumer treated as absent.
  if (expr === undefined || expr === null) return undefined;
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

/**
 * `numeric.dopri` drop-in — the Dormand-Prince ODE integrator DoenetML reached
 * through the old bundled math.js (`me.math.dopri`). Since DoenetML is dropping
 * mathjs, this is exported as a peer compat function (`me.dopri` / a named
 * export) rather than under `me.math`; the call contract is unchanged:
 *
 *   dopri(x0, x1, y0, f, tol?, maxit?)
 *
 * `f(x, y)` returns the derivative; `y0`, the states, and `f`'s return are
 * arrays for a system or plain numbers for a scalar ODE. The result exposes
 * `.at(x)` dense interpolation (a scalar/array x), and the `.x`/`.y` step
 * arrays. Backed by the Rust `solve_ode` integrator (one boundary crossing per
 * RK stage). numeric.js's `event` argument is not supported.
 */
function dopri(
  x0: number,
  x1: number,
  y0: number | ArrayLike<number>,
  f: (x: number, y: number | number[]) => number | number[],
  tol = 1e-6,
  maxit = 1000,
) {
  const scalar = typeof y0 === "number";
  const y0arr = scalar ? [y0 as number] : Array.from(y0 as ArrayLike<number>);
  const dim = y0arr.length;
  // `f` is called from inside the integrator, across the wasm boundary, where
  // an exception must not unwind — `panic = "abort"` makes that a module crash,
  // so the Rust side treats a throwing stage as a failed step and stops early.
  // Correct, but on its own it hands the caller a short, entirely
  // plausible-looking trajectory with only `terminatedEarly` to hint at why:
  // `dopri(0,1,1,()=>{throw …}).at(1)` returned the initial condition. Capture
  // the first failure and rethrow it on this side once the integrator is done.
  // A wrong-length derivative is caught here for the same reason — silently
  // integrating one component of a two-component system is a wrong answer.
  let failure: { error: unknown } | undefined;
  const zeros = () => new Array<number>(dim).fill(0);
  const rhs = (x: number, y: Float64Array): number[] => {
    if (failure) return zeros(); // already doomed; just let the solver wind down
    let out: number | number[];
    try {
      out = f(x, scalar ? y[0] : Array.from(y));
    } catch (error) {
      failure = { error };
      return zeros();
    }
    const arr =
      typeof out === "number"
        ? [out]
        : Array.from(out as ArrayLike<number>, Number);
    if (arr.length !== dim) {
      failure = {
        error: new TypeError(
          `dopri: the derivative returned ${arr.length} component(s) for a ${dim}-component state`,
        ),
      };
      return zeros();
    }
    return arr;
  };
  const sol = wasm.solve_ode(rhs, x0, x1, Float64Array.from(y0arr), tol, maxit);
  if (failure) {
    sol.free(); // nothing will read this solution; do not leak its handle
    throw failure.error;
  }
  const n = sol.dim();
  const state = (flat: Float64Array, i: number) => {
    const s = Array.from(flat.subarray(i * n, (i + 1) * n));
    return scalar ? s[0] : s;
  };
  // Guarded so a second `free()` is a no-op rather than the "null pointer
  // passed to rust" that a wasm-bindgen double free raises — same reasoning as
  // `Expression.free`.
  let freed = false;
  const freeSolution = () => {
    if (freed) return;
    freed = true;
    sol.free();
  };
  return {
    /** Dense output: interpolated state at `x` (or one per element of an `x` array). */
    at(x: number | number[]): number | number[] | (number | number[])[] {
      if (Array.isArray(x)) {
        const flat = sol.at_many(Float64Array.from(x));
        return x.map((_, i) => state(flat, i));
      }
      const s = Array.from(sol.at(x));
      return scalar ? s[0] : s;
    },
    /** Accepted step abscissas. */
    get x(): number[] {
      return Array.from(sol.times());
    },
    /** States at each step abscissa. */
    get y(): (number | number[])[] {
      const ts = sol.times();
      const flat = sol.at_many(ts);
      return Array.from(ts, (_v, i) => state(flat, i));
    },
    /** True when integration stopped before `x1` (blow-up / step budget). */
    get terminatedEarly(): boolean {
      return sol.terminated_early();
    },
    // Same contract as `Expression.free`/`dispose`: the solution owns a wasm
    // handle, and a worker that integrates in a loop leaks one per call
    // otherwise. numeric.js had nothing to release, so this is additive —
    // callers that never free behave exactly as before.
    /** Release the underlying wasm handle. Idempotent. */
    free() {
      freeSolution();
    },
    /** Alias of `free()`, and the `using`-statement protocol where supported. */
    dispose() {
      freeSolution();
    },
    ...(typeof Symbol.dispose === "symbol"
      ? { [Symbol.dispose]: freeSolution }
      : {}),
  };
}

/**
 * Every tree obtainable from `tree` by negating exactly one of its nodes,
 * itself included. Each node is negated once across the whole enumeration, so
 * an n-node tree yields n variants.
 */
function* singleNegations(tree: Tree): Generator<Tree> {
  yield ["-", tree] as Tree;
  if (Array.isArray(tree)) {
    for (let i = 1; i < tree.length; i++) {
      for (const variant of singleNegations(tree[i])) {
        const copy = tree.slice() as Tree[];
        copy[i] = variant;
        yield copy as Tree;
      }
    }
  }
}

/**
 * Does `expr` equal `other` once **exactly** `n_sign_errors` of its parts have
 * their sign flipped? Grading for "you had the right idea but dropped a minus
 * sign" — DoenetML's `numSignErrorsMatched`.
 *
 * Port of the JS `equalSpecifiedSignErrors`. That version negated nodes in
 * place, in the caller's tree, and relied on restoring them afterwards; this
 * one enumerates variants instead, since a wasm-backed `Expression` has no
 * mutable tree. Callers no longer need the defensive deep copy the old
 * contract forced on them, though making one is harmless.
 *
 * `equalityFunction` receives the *negated* expression first, matching the JS
 * argument order — DoenetML's normalizes that side before comparing.
 */
function equalSpecifiedSignErrors(
  expr: ExpressionLike,
  other: ExpressionLike,
  {
    equalityFunction,
    n_sign_errors = 1,
  }: {
    equalityFunction?: (a: Expression, b: Expression) => boolean;
    n_sign_errors?: number;
  } = {},
): boolean {
  const e = toExpr(expr, Context);
  const o = toExpr(other, Context);
  const baseEquality =
    equalityFunction ?? ((a: Expression, b: Expression) => a.equals(b));

  if (n_sign_errors === 0) {
    return baseEquality(e, o);
  }
  if (!(Number.isInteger(n_sign_errors) && n_sign_errors > 0)) {
    throw Error(
      `Have not implemented equality check with ${n_sign_errors} sign errors.`,
    );
  }

  // More than one error: each variant is then checked for the remaining ones,
  // so the negations compose without this function needing to enumerate
  // combinations itself.
  const compare =
    n_sign_errors === 1
      ? baseEquality
      : (a: Expression, b: Expression) =>
          equalSpecifiedSignErrors(a, b, {
            equalityFunction: baseEquality,
            n_sign_errors: n_sign_errors - 1,
          });

  const ctx = (e.context || Context) as Ctx;
  for (const variant of singleNegations(e.tree as Tree)) {
    if (compare(ctx.fromAst(variant) as Expression, o)) return true;
  }
  return false;
}

/**
 * Equal outright, or after up to `max_sign_errors` sign flips — reporting how
 * many it took. Port of the JS `equalWithSignErrors`.
 */
function equalWithSignErrors(
  expr: ExpressionLike,
  other: ExpressionLike,
  {
    equalityFunction,
    max_sign_errors = 1,
  }: {
    equalityFunction?: (a: Expression, b: Expression) => boolean;
    max_sign_errors?: number;
  } = {},
): { matched: boolean; n_sign_errors?: number } {
  const e = toExpr(expr, Context);
  const o = toExpr(other, Context);
  const compare =
    equalityFunction ?? ((a: Expression, b: Expression) => a.equals(b));

  if (compare(e, o)) return { matched: true, n_sign_errors: 0 };

  for (let i = 1; i <= max_sign_errors; i++) {
    if (
      equalSpecifiedSignErrors(e, o, {
        equalityFunction: compare,
        n_sign_errors: i,
      })
    ) {
      return { matched: true, n_sign_errors: i };
    }
  }
  return { matched: false };
}

const Context = {
  dopri,
  from: createFrom,
  fromText: parseText,
  parse: parseText,
  fromLatex: parseLatex,
  fromLaTeX: parseLatex,
  fromTeX: parseLatex,
  fromTex: parseLatex,
  parse_tex: parseLatex,
  fromMml: notImplemented("fromMml"),
  /**
   * `me.setConstantPolicy({define_e: false})` — declare which of `pi`, `e` and
   * `i` denote mathematical constants here rather than ordinary variable names.
   *
   * The original library took this at construction time
   * (`createInstance({define_e, define_pi, define_i})` in `lib/mathjs.js`); the
   * Rust core keeps it as ambient state instead, so it is set rather than
   * baked into an instance. Absent keys keep their current values.
   *
   * Turn `define_e` off for a document whose points are `(e, f)`: `e` then
   * behaves as a variable everywhere — `e^x` stops folding to `exp(x)`, `e` is
   * sampled as a free variable by `equals`, and it is an indeterminate rather
   * than a coefficient to the polynomial code. Likewise `define_i` off stops
   * `i·i` folding to `−1` in a document whose coordinates run `g, h, i`.
   *
   * `sort_constants_first` is the one display option here: off (the default),
   * everything sorts alphabetically, which is what the original library does
   * under every setting of `define_*`. On, declared constants lead a term, so
   * `2 π i` reads that way rather than as the alphabetical `2 i π`.
   *
   * Ordering aside, this never changes a comparator: canonical trees stay
   * comparable across policies, so a stored expression does not stop matching
   * because a document later redeclared a name.
   */
  setConstantPolicy(policy: Record<string, boolean>) {
    wasm.set_constant_policy(JSON.stringify(policy));
  },
  /** The constant policy currently in effect. */
  getConstantPolicy(): Record<string, boolean> {
    return JSON.parse(wasm.get_constant_policy());
  },
  // `me.matrix([[a,b],[c,d]])` — build a matrix literal from a 2-D array of
  // Expressions (or ASTs). Not expression-first (the argument is an array, not
  // an expression), so it lives on the Context directly rather than being
  // mirrored from the `Expression` prototype.
  matrix(rows: ExpressionLike[][]) {
    const nr = rows.length;
    const nc = nr > 0 ? rows[0].length : 0;
    const body = [
      "tuple",
      ...rows.map((row) => [
        "tuple",
        ...row.map((e) => toExpr(e, Context).tree),
      ]),
    ];
    return Context.fromAst(["matrix", ["tuple", nr, nc], body]);
  },
  /**
   * `me.create_discrete_infinite_set({offsets, periods})` — a periodic solution
   * set such as `π/4 + nπ`, written as the union of one arithmetic progression
   * per offset. `offsets` may be a comma list; `periods` is then either a
   * single shared period or a list of matching length. `min_index`/`max_index`
   * bound the index `n` (default: all of ℤ).
   *
   * Like `matrix`, it takes a config object rather than an expression, so it
   * lives on the Context directly. It used to be mirrored from the `Expression`
   * prototype instead, which made the mirror wrapper run `toExpr` over the
   * config object and reject it as a non-tree — the config is the argument, not
   * the receiver.
   *
   * `undefined` (the legacy failure value) when an operand is missing or the
   * offset/period list lengths disagree.
   */
  create_discrete_infinite_set(config?: {
    offsets?: ExpressionLike;
    periods?: ExpressionLike;
    min_index?: ExpressionLike;
    max_index?: ExpressionLike;
  }) {
    // Read the fields one at a time rather than destructuring `config`.
    // API Extractor — which `vite-plugin-dts`'s `rollupTypes` runs, and which
    // therefore walks every declaration in this file for any consumer whose
    // types reach it — cannot resolve an object binding pattern in this
    // position, and aborts that consumer's build outright with "Unable to
    // determine semantic information for declaration". A destructuring here is
    // not worth costing downstream builds their d.ts rollup, so this file keeps
    // to plain property reads.
    const opts = config ?? {};
    const offsets = opts.offsets;
    const periods = opts.periods;
    const min_index = opts.min_index;
    const max_index = opts.max_index;
    if (offsets === undefined || periods === undefined) return undefined;
    // The bounds cross as tree JSON because they are optional and wasm-bindgen
    // has no by-reference `Option<&Expression>`; `tree_json()` is the same wire
    // form `from_ast` reads, so no re-tagging is needed here.
    const bound = (b: ExpressionLike | undefined) =>
      b === undefined ? undefined : toExpr(b, Context)._w.tree_json();
    return wrap(
      wasm.discrete_infinite_set(
        toExpr(offsets, Context)._w,
        toExpr(periods, Context)._w,
        bound(min_index),
        bound(max_index),
      ),
      Context,
    );
  },
  fromAst(ast) {
    const key = atomKey(ast);
    if (key === undefined) {
      // A bare number skips JSON entirely. This is the sampled-coordinate
      // case, which `atomKey` deliberately declines to cache (see
      // `ATOM_HANDLES`: caching arbitrary floats is a measured loss), so it
      // arrives here on every call — once per sample point when a function is
      // evaluated over a domain. Going through `from_ast` meant a
      // `JSON.stringify` here and a full JSON parse in wasm to move one f64.
      //
      // Finite only. The JSON path routes a non-finite through `astReplacer`'s
      // `{"$":"Inf"}` / `{"$":"NaN"}` tags, which `from_ast` revives as the
      // infinity *constant* — a different expression from a float that happens
      // to be infinite, and one that `equals`, `simplify` and the interval
      // endpoints all treat differently. Taking the shortcut there made
      // `fromAst(Infinity)` stop comparing equal to `fromText("infinity")`.
      if (typeof ast === "number" && Number.isFinite(ast)) {
        return new Expression(wasm.from_number(ast), Context);
      }
      return new Expression(
        wasm.from_ast(JSON.stringify(ast, astReplacer)),
        Context,
      );
    }
    let handle = ATOM_HANDLES.get(key);
    if (handle === undefined) {
      handle = wasm.from_ast(JSON.stringify(ast, astReplacer));
      if (ATOM_HANDLES.size >= MAX_ATOMS) ATOM_HANDLES.clear();
      ATOM_HANDLES.set(key, handle);
      ATOM_SHARED.add(handle);
      ATOM_KEYS.set(handle, key);
    }
    // A fresh wrapper per call: the handle is immutable and safe to share, but
    // the `Expression` around it carries a `context` and is what callers hold.
    return new Expression(handle, Context);
  },
  reviver(key, value) {
    if (
      value &&
      value.objectType === "math-expression" &&
      value.tree !== undefined
    ) {
      return Context.fromAst(value.tree);
    }
    return value;
  },
  /**
   * Distinct symbol names interned this session — a memory gauge for the
   * long-lived worker. Append-only (see item 8); use it to measure symbol
   * growth over a session.
   */
  interner_size(): number {
    return wasm.interner_size();
  },
  isTree,
  math,
  converters,
  utils: { match, flatten, unflattenLeft, unflattenRight },
  class: Expression,

  // ---- sign-error grading (`lib/expression/sign_error.js`) ----
  equalSpecifiedSignErrors,
  equalWithSignErrors,

  // ---- assumptions (context-level) ----
  // One handle, fed twice. The wasm `Assumptions` handle answers the predicates
  // (`is_real`, `is_positive`, …) from the text spelling of every assumption,
  // and holds the same facts as trees — filed per variable, so that
  // `get_assumptions` can hand a fact *back*. The parallel text list is what
  // `simplify_with_assumptions` takes.
  //
  // The handle is constructed lazily, and that is load-bearing. As a plain `new
  // wasm.Assumptions()` in this literal it ran while *this module's body* was
  // still evaluating, so any consumer importing `setWasmModule` from the package
  // root forced the wasm load before it had a chance to inject — the injection
  // could never win, and silently fell through to the node loader. Nothing here
  // may touch `wasm` until someone actually calls a method.
  _assumptionsHandleCache: undefined,
  get _assumptionsHandle() {
    return (this._assumptionsHandleCache ??= new wasm.Assumptions());
  },
  set _assumptionsHandle(h) {
    this._assumptionsHandleCache = h;
  },
  _assumptionTexts: [],
  set_to_default() {
    // A fresh handle is the reset: it carries the per-variable facts too. The
    // one being replaced is wasm-side memory that nothing else holds, and
    // `clear_assumptions` delegates here, so a worker that clears between
    // problems would otherwise leak one `Assumptions` per clear. `free?.()`
    // because a test may have injected a stand-in module.
    this._assumptionsHandleCache?.free?.();
    this._assumptionsHandle = new wasm.Assumptions();
    this._assumptionTexts = [];
  },
  clear_assumptions() {
    this.set_to_default();
  },
  add_assumption(assumption, exclude_generic?) {
    const tree = syncAssumptionText(this, assumption, "add");
    if (tree === undefined) return 0;
    return assumptionStore.add_assumption(
      this._assumptionsHandle,
      tree,
      exclude_generic,
    );
  },
  add_generic_assumption(assumption) {
    // A generic assumption is stated in terms of `x` and stands for every
    // variable, which the wasm store cannot express; it gets the `x` spelling,
    // which is at least right for `x` itself.
    const tree = syncAssumptionText(this, assumption, "add");
    if (tree === undefined) return 0;
    return assumptionStore.add_generic_assumption(
      this._assumptionsHandle,
      tree,
    );
  },
  remove_assumption(assumption) {
    const tree = syncAssumptionText(this, assumption, "remove");
    if (tree === undefined) return 0;
    return assumptionStore.remove_assumption(this._assumptionsHandle, tree);
  },
  remove_generic_assumption(assumption) {
    const tree = syncAssumptionText(this, assumption, "remove");
    if (tree === undefined) return 0;
    return assumptionStore.remove_generic_assumption(
      this._assumptionsHandle,
      tree,
    );
  },
  get_assumptions(variables_or_expr, params?) {
    return assumptionStore.get_assumptions(
      this._assumptionsHandle,
      variables_or_expr,
      params,
    );
  },
  // `me.assumptions` was the assumptions object itself, carrying the same
  // add/get methods as the context. This port also has to keep answering the
  // wasm predicates through it, since `lib/assumptions/element_of_sets` reads
  // `Context.assumptions` as its default source — so the facade forwards those
  // to the handle rather than replacing it.
  _assumptionsFacadeCache: undefined,
  get assumptions() {
    return (this._assumptionsFacadeCache ??= makeAssumptionsFacade());
  },
};

// The lazy handle above is a cached wasm object, so it belongs to whichever
// module minted it — see `onWasmModuleChange`. Without this, a host that calls
// `setWasmModule` after anything has touched an assumption keeps handing the
// old module's `Assumptions` the new module's `Expression`s, and every
// `equals`/`solve_linear` fails with "expected instance of Expression". Drop
// the texts too: they are the same facts in the other representation, and a
// handle rebuilt from an empty store must not claim to hold them.
onWasmModuleChange(() => {
  Context._assumptionsHandleCache = undefined;
  Context._assumptionTexts = [];
});

/**
 * Mirror an assumption into the wasm handle and the `simplify_with_assumptions`
 * text list, returning its tree for the JS store to file — or undefined when
 * there is no assumption at all.
 *
 * An empty assumption is a no-op rather than an error: the spec tables drive
 * `me.add_assumption(me.from(input))` over rows whose input is undefined,
 * meaning "no assumptions for this row".
 */
function syncAssumptionText(
  context: Ctx,
  assumption: ExpressionLike,
  action: "add" | "remove",
): Tree | undefined {
  const tree = get_tree(assumption);
  if (!Array.isArray(tree)) return undefined;

  const text = toExpr(assumption, context).toString();
  if (action === "add") {
    context._assumptionsHandle.add(text);
    context._assumptionTexts.push(text);
  } else {
    context._assumptionsHandle.remove(text);
    context._assumptionTexts = context._assumptionTexts.filter(
      (t) => t !== text,
    );
  }
  return tree;
}

/**
 * `me.assumptions`: the JS assumption API plus the wasm predicates, both
 * pointing at the live context state (never a snapshot — the spec clears and
 * re-adds assumptions between calls while holding the same object).
 */
function makeAssumptionsFacade() {
  const facade: Record<string, unknown> = {
    get _assumptionsHandle() {
      return Context._assumptionsHandle;
    },
    get byvar() {
      return assumptionStore.byvar(Context._assumptionsHandle);
    },
    get derived() {
      return assumptionStore.derived(Context._assumptionsHandle);
    },
    get generic() {
      return assumptionStore.generic(Context._assumptionsHandle);
    },
  };
  for (const name of [
    "get_assumptions",
    "add_assumption",
    "add_generic_assumption",
    "remove_assumption",
    "remove_generic_assumption",
    "clear_assumptions",
    "set_to_default",
  ]) {
    facade[name] = (...args: unknown[]) => Context[name](...args);
  }
  // The three-valued predicates and the raw relation add/remove live on the
  // wasm handle; keep them reachable so a caller holding `me.assumptions` can
  // still use it as one.
  for (const name of [
    "is_integer",
    "is_real",
    "is_complex",
    "is_nonzero",
    "is_nonnegative",
    "is_nonpositive",
    "is_positive",
    "is_negative",
    "add",
    "remove",
  ]) {
    facade[name] = (...args: unknown[]) =>
      Context._assumptionsHandle[name](...args);
  }
  return facade;
}

// The legacy library exposed every `Expression` method a second time as a free
// function on the context, expression-first: `me.simplify(expr)` alongside
// `expr.simplify()`. Mirror the prototype onto `Context` once both exist.
//
// Anything already reachable on `Context` wins, so the factories (`from`,
// `fromAst`, `fromText`, …) are never shadowed — and neither are the inherited
// `Object.prototype` members, which is why `toString`/`valueOf` stay put rather
// than becoming expression-first functions that would break `String(me)`.
//
// `NOT_EXPRESSION_FIRST` covers what that `in Context` test misses. A protocol
// method the *runtime* calls is not a candidate for the expression-first
// treatment, because the runtime supplies its own argument: `JSON.stringify`
// invokes `toJSON(key)`, so mirroring it made the property key the "expression",
// and `JSON.stringify({me})` emitted a `{objectType:"math-expression"}` envelope
// that `Context.reviver` would then revive the whole library context from —
// while `JSON.stringify({"(": me})` *threw* a parse error out of a plain
// stringify. `toJSON` is not on `Object.prototype`, so only naming it works.
// `free`/`dispose` are excluded for a milder reason: they manage this port's
// wasm handles, which legacy had no concept of, so there is no expression-first
// spelling of them to be compatible with — and `me.dispose()` reads like "tear
// down the context", which it would not do.
//
// Coercion goes through `toExpr`, not `Context.from`: the argument is usually
// an `Expression` already, and `from` would try to read that as an AST.
const NOT_EXPRESSION_FIRST = new Set([
  "constructor",
  "toJSON",
  "free",
  "dispose",
]);
for (const name of Object.getOwnPropertyNames(Expression.prototype)) {
  if (NOT_EXPRESSION_FIRST.has(name) || name in Context) continue;
  const desc = Object.getOwnPropertyDescriptor(Expression.prototype, name);
  if (typeof desc?.value !== "function") continue; // skip accessors such as `tree`
  (Context as Record<string, unknown>)[name] = (
    expr: ExpressionLike,
    ...args: unknown[]
  ) =>
    (toExpr(expr) as unknown as Record<string, (...a: unknown[]) => unknown>)[
      name
    ](...args);
}

export { Expression, dopri, setWasmModule };
export default Context;
