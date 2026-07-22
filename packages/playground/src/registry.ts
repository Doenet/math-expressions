// The dual-engine operation registry. Each entry describes ONE operation for
// BOTH the canonical JS library and the Rust (WASM) port, capturing their API
// divergences (JS assumptions-via-Context vs Rust `simplify_with_assumptions`;
// JS `substitute(map)` vs Rust looped `substitute_var`; JS `toString`/`toLatex`
// vs Rust `to_text`/`to_latex`; methods present on only one engine). The registry
// drives the palette, autocomplete, and the chain evaluator's dispatch.

import { Context, normalizeComplex } from "./engines";
import type {
  Complex,
  EngineCtx,
  JsExpr,
  Literal,
  NativeResult,
  OpCategory,
  OpEntry,
  RustExpr,
  Tree,
} from "./types";

/* ----------------------------- arg readers ----------------------------- */

function needStr(lit: Literal | undefined, name: string): string {
  if (!lit) throw new Error(`missing argument "${name}"`);
  if (lit.kind !== "string")
    throw new Error(`argument "${name}" must be a string`);
  return lit.value;
}

function needNum(lit: Literal | undefined, name: string): number {
  if (!lit) throw new Error(`missing argument "${name}"`);
  if (lit.kind !== "number")
    throw new Error(`argument "${name}" must be a number`);
  return lit.value;
}

function optStringArray(lit: Literal | undefined): string[] {
  if (!lit) return [];
  if (lit.kind !== "array") throw new Error("expected an array of strings");
  return lit.items.map((it) => {
    if (it.kind !== "string") throw new Error("assumptions must be strings");
    return it.value;
  });
}

/** Read a `{x: "2"}`-style object literal into `[name, exprString]` pairs. */
function readMap(lit: Literal | undefined, name: string, optional: boolean): [string, string][] {
  if (!lit) {
    if (optional) return [];
    throw new Error(`missing argument "${name}"`);
  }
  if (lit.kind !== "object")
    throw new Error(`argument "${name}" must be an object like {x: "2"}`);
  return lit.entries.map(({ key, value }) => {
    if (value.kind !== "string")
      throw new Error(`value for "${key}" must be a string expression`);
    return [key, value.value] as [string, string];
  });
}

/* ----------------------- native-result constructors --------------------- */

const expr = <H>(handle: H): NativeResult<H> => ({ kind: "expression", handle });
const maybeExpr = <H>(handle: H | null): NativeResult<H> => ({
  kind: "maybeExpression",
  handle,
});
const str = (value: string, render?: "text" | "latex"): NativeResult<never> => ({
  kind: "string",
  value,
  render,
});
const strList = (value: string[]): NativeResult<never> => ({
  kind: "stringList",
  value,
});
const bool = (value: boolean): NativeResult<never> => ({ kind: "boolean", value });
const num = (value: number | null): NativeResult<never> => ({
  kind: "number",
  value,
});
const cplx = (value: Complex | null): NativeResult<never> => ({
  kind: "complex",
  value,
});
const treeRes = (value: Tree): NativeResult<never> => ({ kind: "tree", value });

/* ------------------------- shared JS assumptions ------------------------ */

// The JS library reads assumptions off the shared Context; its `simplify(...)`
// first arg is a structured Assumptions object, not the relation strings we have.
// So (matching the original playground) we set the Context assumptions from the
// relation strings around the call and clear them afterwards. The runtime accepts
// a relation *tree* even though the public type asks for an object.
const jsAddAssumption = Context.add_assumption.bind(Context) as (
  tree: Tree,
) => void;

/* ------------------------------ evaluation ------------------------------ */

function jsEvaluate(
  recv: JsExpr,
  bindings: [string, string][],
  ctx: EngineCtx<JsExpr>,
): Complex | null {
  try {
    let closed = recv;
    if (bindings.length > 0) {
      const map: Record<string, Tree> = {};
      // Parse binding values through ctx so the current notation applies.
      for (const [k, v] of bindings) map[k] = ctx.parseExpr(v).tree as Tree;
      closed = recv.substitute(map as Parameters<typeof recv.substitute>[0]);
    }
    return normalizeComplex(closed.evaluate({}));
  } catch {
    return null;
  }
}

function rustEvaluate(
  recv: RustExpr,
  bindings: [string, string][],
  ctx: EngineCtx<RustExpr>,
): Complex | null {
  try {
    let cur = recv;
    for (const [k, v] of bindings) {
      // Track every handle the moment it exists, so a later throw (e.g. an
      // unparseable binding value) can never orphan an earlier intermediate.
      // The final handle is terminal here (its complex value is read, not the
      // handle itself), so it is tracked and freed too. `recv` is never tracked.
      const val = ctx.track(ctx.parseExpr(v));
      cur = ctx.track(cur.substitute_var(k, val));
    }
    const pair = cur.evaluate_to_complex();
    return normalizeComplex(pair ? Array.from(pair) : null);
  } catch {
    return null;
  }
}

/* ------------------------------- registry ------------------------------- */

// Concise builder for the common case: an operation present on both engines
// with the same *chain* name but its own native method on each side.
function op(
  id: string,
  category: OpCategory,
  returns: OpEntry["returns"],
  args: OpEntry["args"],
  insertText: string,
  js: OpEntry["js"],
  rust: OpEntry["rust"],
  extra?: Partial<OpEntry>,
): OpEntry {
  return { id, display: id, category, returns, args, insertText, js, rust, ...extra };
}

export const REGISTRY: OpEntry[] = [
  /* ---- Core ---- */
  op(
    "simplify",
    "Core",
    "expression",
    [{ name: "assumptions", kind: "stringArray", optional: true }],
    "simplify()",
    {
      call: "simplify() with Context assumptions",
      run: (h, a, c) => {
        const rels = optStringArray(a[0]);
        Context.clear_assumptions();
        for (const r of rels) {
          try {
            // Parse the relation through ctx so the current notation applies.
            jsAddAssumption(c.parseExpr(r).tree as Tree);
          } catch {
            /* ignore unparseable assumption */
          }
        }
        try {
          return expr(h.simplify());
        } finally {
          Context.clear_assumptions();
        }
      },
    },
    {
      call: "simplify() / simplify_with_assumptions([...])",
      run: (h, a) => {
        const rels = optStringArray(a[0]);
        return expr(rels.length ? h.simplify_with_assumptions(rels) : h.simplify());
      },
    },
  ),
  op(
    "expand",
    "Core",
    "expression",
    [],
    "expand()",
    { call: "expand()", run: (h) => expr(h.expand()) },
    { call: "expand()", run: (h) => expr(h.expand()) },
  ),
  op(
    "evaluate_numbers",
    "Core",
    "expression",
    [],
    "evaluate_numbers()",
    { call: "evaluate_numbers()", run: (h) => expr(h.evaluate_numbers()) },
    { call: "evaluate_numbers()", run: (h) => expr(h.evaluate_numbers()) },
  ),
  op(
    "collect_like_terms_factors",
    "Core",
    "expression",
    [],
    "collect_like_terms_factors()",
    {
      call: "collect_like_terms_factors()",
      run: (h) => expr(h.collect_like_terms_factors()),
    },
    {
      call: "collect_like_terms_factors()",
      run: (h) => expr(h.collect_like_terms_factors()),
    },
  ),
  op(
    "copy",
    "Core",
    "expression",
    [],
    "copy()",
    { call: "copy()", run: (h) => expr(h.copy()) },
    { call: "copy()", run: (h) => expr(h.copy()) },
  ),

  /* ---- Arithmetic (expression-typed args) ---- */
  ...(["add", "subtract", "multiply", "divide", "pow", "mod"] as const).map(
    (name) =>
      op(
        name,
        "Arithmetic",
        "expression",
        [{ name: "other", kind: "expression" }],
        `${name}("x")`,
        {
          call: `${name}(expr)`,
          run: (h, a, c) => expr(h[name](c.parseExpr(needStr(a[0], "other")))),
        },
        {
          call: `${name}(expr)`,
          run: (h, a, c) =>
            expr(h[name](c.track(c.parseExpr(needStr(a[0], "other"))))),
        },
      ),
  ),

  /* ---- Algebra ---- */
  op(
    "substitute",
    "Algebra",
    "expression",
    [{ name: "subs", kind: "substitutionMap" }],
    'substitute({x: "2"})',
    {
      call: "substitute(map)",
      run: (h, a, c) => {
        const map: Record<string, Tree> = {};
        for (const [k, v] of readMap(a[0], "subs", false))
          map[k] = c.parseExpr(v).tree as Tree;
        return expr(h.substitute(map as Parameters<typeof h.substitute>[0]));
      },
    },
    {
      call: "substitute_var(...) looped",
      run: (h, a, c) => {
        let cur = h;
        for (const [k, v] of readMap(a[0], "subs", false)) {
          // Track the prior intermediate BEFORE parsing the next value, so a
          // throw there frees it. The final `cur` is left untracked — it is
          // returned as the step's result and becomes the new receiver.
          if (cur !== h) c.track(cur);
          const val = c.track(c.parseExpr(v));
          cur = cur.substitute_var(k, val);
        }
        return expr(cur);
      },
    },
  ),
  op(
    "reduce_rational",
    "Algebra",
    "expression",
    [],
    "reduce_rational()",
    { call: "reduce_rational()", run: (h) => expr(h.reduce_rational()) },
    { call: "reduce_rational()", run: (h) => expr(h.reduce_rational()) },
  ),
  op(
    "together",
    "Algebra",
    "expression",
    [],
    "together()",
    // Closest JS analog of Rust `together` is `common_denominator`.
    { call: "common_denominator()", run: (h) => expr(h.common_denominator()) },
    { call: "together()", run: (h) => expr(h.together()) },
  ),
  op(
    "factor",
    "Algebra",
    "expression",
    [],
    "factor()",
    null,
    { call: "factor()", run: (h) => expr(h.factor()) },
    { unsupportedReason: { js: "factor() is not implemented in the JS build" } },
  ),
  op(
    "constants_to_floats",
    "Algebra",
    "expression",
    [],
    "constants_to_floats()",
    { call: "constants_to_floats()", run: (h) => expr(h.constants_to_floats()) },
    { call: "constants_to_floats()", run: (h) => expr(h.constants_to_floats()) },
  ),
  op(
    "normalize_function_names",
    "Algebra",
    "expression",
    [],
    "normalize_function_names()",
    {
      call: "normalize_function_names()",
      run: (h) => expr(h.normalize_function_names()),
    },
    {
      call: "normalize_function_names()",
      run: (h) => expr(h.normalize_function_names()),
    },
  ),

  /* ---- Calculus ---- */
  op(
    "derivative",
    "Calculus",
    "expression",
    [{ name: "variable", kind: "variable" }],
    'derivative("x")',
    {
      call: "derivative(v)",
      run: (h, a) => expr(h.derivative(needStr(a[0], "variable"))),
    },
    {
      call: "derivative(v)",
      run: (h, a) => expr(h.derivative(needStr(a[0], "variable"))),
    },
  ),
  op(
    "integrate",
    "Calculus",
    "maybeExpression",
    [{ name: "variable", kind: "variable" }],
    'integrate("x")',
    null,
    {
      call: "integrate(v)",
      run: (h, a) => maybeExpr(h.integrate(needStr(a[0], "variable")) ?? null),
    },
    {
      unsupportedReason: {
        js: "the JS library has only integrateNumerically (no symbolic integral)",
      },
    },
  ),
  op(
    "integrateNumerically",
    "Calculus",
    "number",
    [
      { name: "variable", kind: "variable" },
      { name: "lower", kind: "number" },
      { name: "upper", kind: "number" },
    ],
    'integrateNumerically("x", 0, 1)',
    {
      call: "integrateNumerically(v, a, b)",
      run: (h, a) =>
        num(
          h.integrateNumerically(
            needStr(a[0], "variable"),
            needNum(a[1], "lower"),
            needNum(a[2], "upper"),
          ),
        ),
    },
    null,
    {
      unsupportedReason: {
        rust: "the Rust port exposes integrate_to_precision, not a bare numeric integral",
      },
    },
  ),

  /* ---- Query (terminal results) ---- */
  op(
    "variables",
    "Query",
    "stringList",
    [],
    "variables()",
    { call: "variables()", run: (h) => strList(h.variables()) },
    { call: "variables()", run: (h) => strList(Array.from(h.variables())) },
  ),
  op(
    "functions",
    "Query",
    "stringList",
    [],
    "functions()",
    { call: "functions()", run: (h) => strList(h.functions()) },
    { call: "functions()", run: (h) => strList(Array.from(h.functions())) },
  ),
  op(
    "operators",
    "Query",
    "stringList",
    [],
    "operators()",
    { call: "operators()", run: (h) => strList(h.operators()) },
    null,
    { unsupportedReason: { rust: "operators() has no Rust binding" } },
  ),
  op(
    "equals",
    "Query",
    "boolean",
    [{ name: "other", kind: "expression" }],
    'equals("...")',
    {
      call: "equals(expr)",
      run: (h, a, c) => bool(h.equals(c.parseExpr(needStr(a[0], "other")))),
    },
    {
      call: "equals(expr)",
      run: (h, a, c) =>
        bool(h.equals(c.track(c.parseExpr(needStr(a[0], "other"))))),
    },
  ),
  op(
    "evaluate",
    "Query",
    "complex",
    [{ name: "bindings", kind: "substitutionMap", optional: true }],
    'evaluate({x: "2"})',
    {
      call: "substitute → evaluate({})",
      run: (h, a, c) => cplx(jsEvaluate(h, readMap(a[0], "bindings", true), c)),
    },
    {
      call: "substitute_var → evaluate_to_complex",
      run: (h, a, c) => cplx(rustEvaluate(h, readMap(a[0], "bindings", true), c)),
    },
  ),
  op(
    "evaluate_to_constant",
    "Query",
    "number",
    [],
    "evaluate_to_constant()",
    {
      call: "evaluate_to_constant()",
      run: (h) => num(h.evaluate_to_constant()),
    },
    {
      call: "evaluate_to_constant()",
      run: (h) => num(h.evaluate_to_constant() ?? null),
    },
  ),
  op(
    "evaluate_to_precision",
    "Query",
    "string",
    [{ name: "digits", kind: "number" }],
    "evaluate_to_precision(20)",
    {
      // JS has NO arbitrary precision — it evaluates to a double (~15–16 real
      // sig figs). Honor the requested digits only within that range; beyond
      // it, show the honest double rather than fabricating float noise.
      call: "evaluate_to_constant() — double precision (~15–16 digits)",
      run: (h, a) => {
        const d = Math.round(needNum(a[0], "digits"));
        const v = h.evaluate_to_constant();
        if (v == null) return str("undefined", "text");
        return str(d >= 1 && d <= 15 ? v.toPrecision(d) : String(v), "text");
      },
    },
    {
      call: "evaluate_to_precision(n) — arbitrary precision",
      run: (h, a) =>
        str(
          h.evaluate_to_precision(Math.round(needNum(a[0], "digits"))) ??
            "undecided",
          "text",
        ),
    },
    { stringRender: "text" },
  ),
  op(
    "is_zero",
    "Query",
    "string",
    [],
    "is_zero()",
    null,
    {
      call: "is_zero()",
      run: (h) => {
        const z = h.is_zero();
        return str(z === undefined ? "undecided" : String(z));
      },
    },
    {
      stringRender: "text",
      unsupportedReason: {
        js: "the certified is_zero test is Rust-only",
      },
    },
  ),
  op(
    "isAnalytic",
    "Query",
    "boolean",
    [],
    "isAnalytic()",
    { call: "isAnalytic()", run: (h) => bool(h.isAnalytic()) },
    {
      call: "is_analytic(false, false, false)",
      run: (h) => bool(h.is_analytic(false, false, false)),
    },
  ),

  /* ---- Render (terminal results) ---- */
  op(
    "toString",
    "Render",
    "string",
    [],
    "toString()",
    { call: "toString()", run: (h, _a, c) => str(c.toText(h), "text") },
    { call: "to_text()", run: (h, _a, c) => str(c.toText(h), "text") },
    { stringRender: "text" },
  ),
  op(
    "toLatex",
    "Render",
    "string",
    [],
    "toLatex()",
    { call: "toLatex()", run: (h, _a, c) => str(c.toLatex(h), "latex") },
    { call: "to_latex()", run: (h, _a, c) => str(c.toLatex(h), "latex") },
    { stringRender: "latex" },
  ),
  op(
    "toJSON",
    "Render",
    "string",
    [],
    "toJSON()",
    { call: "toJSON()", run: (h) => str(JSON.stringify(h.toJSON()), "text") },
    { call: "to_serialized()", run: (h) => str(h.to_serialized(), "text") },
    { stringRender: "text" },
  ),
  op(
    "tree",
    "Render",
    "tree",
    [],
    "tree()",
    { call: ".tree", run: (h) => treeRes(h.tree as Tree) },
    { call: "tree_json()", run: (h) => treeRes(JSON.parse(h.tree_json()) as Tree) },
  ),
];

/** Registry indexed by chain method name. */
export const REGISTRY_BY_ID = new Map<string, OpEntry>(
  REGISTRY.map((e) => [e.id, e]),
);

/** Palette grouping: category → its ops, in registry order. */
export const CATEGORIES: OpCategory[] = [
  "Core",
  "Arithmetic",
  "Algebra",
  "Calculus",
  "Query",
  "Render",
];
