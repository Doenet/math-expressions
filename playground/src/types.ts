// Shared types for the playground.

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

export type Syntax = "text" | "latex";

/* ---------- external library surfaces (only what we use) ---------- */

/** An expression handle from the JS math-expressions library. */
export interface JsExpr {
  tree: Tree;
  toString(): string;
  toLatex(): string;
  variables(): string[];
  substitute(map: Record<string, Tree>): JsExpr;
  evaluate(bindings: Record<string, unknown>): number | Complex;
  derivative(v: string): JsExpr;
  simplify(): JsExpr;
  expand(): JsExpr;
  equals(other: JsExpr): boolean;
}

/** The JS math-expressions default-export context (factory + assumptions). */
export interface MathExprContext {
  fromText(s: string): JsExpr;
  fromLatex(s: string): JsExpr;
  add_assumption(tree: Tree): void;
  clear_assumptions(): void;
}

/** A wasm-bindgen `Expression` handle from the Rust implementation. */
export interface RustExpr {
  __wbg_ptr: number;
  free(): void;
  tree_json(): string;
  to_text(): string;
  to_latex(): string;
  variables(): string[];
  substitute_var(v: string, value: RustExpr): RustExpr;
  evaluate_to_complex(): Float64Array | undefined;
  derivative(v: string): RustExpr;
  /** Indefinite integral in `v`; `undefined` when no elementary form is found. */
  integrate(v: string): RustExpr | undefined;
  simplify_with_assumptions(assumptions: string[]): RustExpr;
  expand(): RustExpr;
  equals(other: RustExpr): boolean;
}

/** The Rust wasm-bindgen module (glue), loaded at runtime. */
export interface RustModule {
  default(moduleOrPath?: unknown): Promise<unknown>;
  parse_text(s: string): RustExpr;
  parse_latex(s: string): RustExpr;
}

/* ---------- unified engine adapter ---------- */

/** Common interface over both implementations, generic in the handle type. */
export interface Engine<H> {
  name: string;
  parse(text: string, syntax: Syntax): H;
  tree(h: H): Tree;
  toText(h: H): string;
  toLatex(h: H): string;
  variables(h: H): string[];
  evaluate(h: H, subs: Record<string, string>): Complex | null;
  derivative(h: H, v: string): H;
  /**
   * Indefinite integral in `v`. Returns `null` when the engine found no
   * elementary form; throws when the engine has no symbolic integration at all
   * (the JS library) so the two cases stay distinguishable to the caller.
   */
  integrate(h: H, v: string): H | null;
  free(h: H): void;
  simplifyWith(h: H, assumptions: string[]): H;
  expand(h: H): H;
  equals(a: H, b: H): boolean;
}

export interface Engines {
  js: Engine<JsExpr>;
  rust: Engine<RustExpr>;
}

/** Inputs to one analysis pass. */
export interface EngineParams {
  input: string;
  syntax: Syntax;
  diffVar: string;
  bindings: Record<string, string>;
  assumptions: string[];
  simplifyDeriv: boolean;
}

/** All extracted, handle-free results for one engine. */
export interface Analysis {
  parseError?: string;
  tree?: SafeResult<Tree>;
  text?: SafeResult<string>;
  latex?: SafeResult<string>;
  variables?: SafeResult<string[]>;
  evalF?: SafeResult<Complex | null>;
  simpTree?: SafeResult<Tree>;
  simpText?: SafeResult<string>;
  simpLatex?: SafeResult<string>;
  derText?: SafeResult<string>;
  derLatex?: SafeResult<string>;
  evalDer?: SafeResult<Complex | null>;
  /**
   * The simplified indefinite integral (text + latex), or `null` when no
   * elementary form was found. A failed result carries the reason (e.g. the JS
   * library not supporting symbolic integration).
   */
  integral?: SafeResult<{ text: string; latex: string } | null>;
}
