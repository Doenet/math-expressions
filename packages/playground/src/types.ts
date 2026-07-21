// Shared types for the playground.
//
// The playground runs a user-authored *method chain* (e.g.
// `parse("x^2-1").reduce_rational().toLatex()`) against two implementations of
// the math-expressions library — the canonical JS package and the Rust (WASM)
// port — and shows every step side by side. These types describe the parsed
// chain, the dual-engine operation registry that dispatches it, and the
// handle-free result records the UI renders.

import type { Expression as JsExpr } from "math-expressions-canonical";

export type { JsExpr };

/** The JS `Tree` JSON shape both engines emit, e.g. `["+", 1, "x", 3]`. */
export type Tree = string | number | boolean | TreeArray;
/** The compound (operator) case of {@link Tree}: `[head, ...operands]`. */
export interface TreeArray extends Array<Tree> {}

/** A complex value; `im === 0` for reals. */
export interface Complex {
  re: number;
  im: number;
}

/** Result of a guarded step: either a value or an error message. */
export type SafeResult<T> =
  | { ok: true; value: T }
  | { ok: false; error: string };

/** The syntax a *source* expression is written in. */
export type Syntax = "text" | "latex";

/* ---------- Rust wasm-bindgen surface (only what the registry uses) ---------- */

/**
 * A wasm-bindgen `Expression` handle from the Rust implementation. Handles are
 * wasm-owned and must be freed via `free()` — see `freeHandle` in engines.ts.
 * Only the methods the operation registry dispatches to are declared here.
 */
export interface RustExpr {
  __wbg_ptr: number;
  free(): void;
  // rendering / inspection
  to_text(): string;
  to_latex(): string;
  tree_json(): string;
  to_serialized(): string;
  variables(): string[];
  functions(): string[];
  // evaluation
  evaluate_to_complex(): Float64Array | undefined;
  evaluate_to_constant(): number | undefined;
  // transforms (return new handles)
  simplify(): RustExpr;
  simplify_with_assumptions(assumptions: string[]): RustExpr;
  expand(): RustExpr;
  evaluate_numbers(): RustExpr;
  collect_like_terms_factors(): RustExpr;
  reduce_rational(): RustExpr;
  together(): RustExpr;
  factor(): RustExpr;
  constants_to_floats(): RustExpr;
  normalize_function_names(): RustExpr;
  copy(): RustExpr;
  derivative(v: string): RustExpr;
  substitute_var(v: string, value: RustExpr): RustExpr;
  add(other: RustExpr): RustExpr;
  subtract(other: RustExpr): RustExpr;
  multiply(other: RustExpr): RustExpr;
  divide(other: RustExpr): RustExpr;
  pow(other: RustExpr): RustExpr;
  mod(other: RustExpr): RustExpr;
  // predicates / queries
  equals(other: RustExpr): boolean;
  is_zero(): boolean | undefined;
  is_analytic(allow_abs: boolean, allow_arg: boolean, allow_relation: boolean): boolean;
  /** Indefinite integral in `v`; `undefined` when no elementary form is found. */
  integrate(v: string): RustExpr | undefined;
}

/** The Rust wasm-bindgen module (glue), loaded at runtime. */
export interface RustModule {
  default(moduleOrPath?: unknown): Promise<unknown>;
  parse_text(s: string): RustExpr;
  parse_latex(s: string): RustExpr;
  from_ast(tree_json: string): RustExpr;
}

/* ---------- chain parser ---------- */

/** A character span `[start, end)` in the source text, for editor underlining. */
export interface Span {
  start: number;
  end: number;
}

/** A parsed JS literal argument (string / number / boolean / array / object). */
export type Literal =
  | { kind: "string"; value: string; span: Span }
  | { kind: "number"; value: number; span: Span }
  | { kind: "boolean"; value: boolean; span: Span }
  | { kind: "array"; items: Literal[]; span: Span }
  | { kind: "object"; entries: { key: string; value: Literal }[]; span: Span };

/** The factory that starts a chain. `parse` is an alias for `fromText`. */
export type SourceKind = "parse" | "fromText" | "fromLatex" | "fromAst";

/**
 * What a chain starts from: either a factory call (`parse("…")`, …) or a bare
 * variable reference — the equation entered in the first box, stored as `expr`.
 */
export type ChainSource =
  | { kind: SourceKind; arg: Literal; span: Span }
  | { kind: "var"; name: string; span: Span };

/** One `.method(args)` call in the chain. */
export interface ChainStep {
  method: string;
  args: Literal[];
  nameSpan: Span;
  span: Span;
}

/** A fully parsed chain: a source followed by zero or more steps. */
export interface ParsedChain {
  source: ChainSource;
  steps: ChainStep[];
}

/** Result of parsing a chain: either the chain or a located error. */
export type ParseOutcome =
  | { ok: true; chain: ParsedChain }
  | { ok: false; error: { message: string; start: number; end: number } };

/* ---------- operation registry ---------- */

export type ArgKind =
  | "variable"
  | "number"
  | "string"
  | "boolean"
  | "stringArray"
  | "expression"
  | "substitutionMap";

export interface ArgSpec {
  name: string;
  kind: ArgKind;
  optional?: boolean;
}

/**
 * What an operation returns. `expression` continues the chain; `maybeExpression`
 * is a handle-or-null (null ends the chain, e.g. no elementary integral); the
 * rest are terminal values.
 */
export type ReturnKind =
  | "expression"
  | "maybeExpression"
  | "string"
  | "stringList"
  | "boolean"
  | "number"
  | "complex"
  | "tree";

export type OpCategory =
  | "Core"
  | "Arithmetic"
  | "Algebra"
  | "Calculus"
  | "Query"
  | "Render";

/**
 * The native (handle-holding) result of running one operation on one engine.
 * The evaluator turns this into a handle-free {@link Displayable}, freeing any
 * produced handle afterwards.
 */
export type NativeResult<H> =
  | { kind: "expression"; handle: H }
  | { kind: "maybeExpression"; handle: H | null }
  | { kind: "string"; value: string; render?: "text" | "latex" }
  | { kind: "stringList"; value: string[] }
  | { kind: "boolean"; value: boolean }
  | { kind: "number"; value: number | null }
  | { kind: "complex"; value: Complex | null }
  | { kind: "tree"; value: Tree };

/** Primitives an operation needs to materialize handle-typed args. */
export interface EngineCtx<H> {
  /** Parse an expression string (always text syntax) into a handle. */
  parseExpr(text: string): H;
  /** Register a temporary handle to be freed after the step; returns it. */
  track(h: H): H;
}

/** One engine's implementation of an operation. */
export interface EngineOp<H> {
  /** The exact underlying invocation, shown in tooltips. */
  call: string;
  run(recv: H, args: Literal[], ctx: EngineCtx<H>): NativeResult<H>;
}

/** A single operation, applicable to both engines, driving the palette + autocomplete. */
export interface OpEntry {
  id: string;
  display: string;
  category: OpCategory;
  args: ArgSpec[];
  returns: ReturnKind;
  /** How a terminal `string` result is rendered. */
  stringRender?: "text" | "latex";
  /** Snippet the palette / autocomplete inserts, e.g. `derivative("x")`. */
  insertText: string;
  js: EngineOp<JsExpr> | null;
  rust: EngineOp<RustExpr> | null;
  unsupportedReason?: { js?: string; rust?: string };
}

/* ---------- handle-free result records for the UI ---------- */

/** A handle-free rendering of one operation's result on one engine. */
export type Displayable =
  | { kind: "expression"; text: string; latex: string; tree: Tree }
  | { kind: "string"; value: string; render: "text" | "latex" }
  | { kind: "stringList"; value: string[] }
  | { kind: "boolean"; value: boolean }
  | { kind: "number"; value: number | null }
  | { kind: "complex"; value: Complex | null }
  | { kind: "tree"; value: Tree }
  | { kind: "none" } // maybeExpression → null (no result)
  | { kind: "unsupported"; reason?: string }
  | { kind: "ended" }; // the chain already terminated at a prior step

/** One chain step's paired result across both engines. */
export interface StepResult {
  /** `"source"` for the factory, else the operation's display name. */
  label: string;
  call: { js?: string; rust?: string };
  jsResult: SafeResult<Displayable>;
  rustResult: SafeResult<Displayable>;
  /** Set only when both sides produced comparable displayables. */
  agree?: boolean;
}
