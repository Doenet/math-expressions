/*
 * Structural TypeScript surface for the math-expressions Rust/WASM module
 * (`packages/math-expressions-rs/pkg`, emitted by wasm-bindgen).
 *
 * The authoritative, exhaustive types are the *generated* `math_expressions.d.ts`
 * inside that `pkg/` directory. Those are a build artifact (git-ignored, and the
 * shape differs by wasm-bindgen `--target`), so this module declares the small
 * structural subset the bindings actually rely on. Consumers pass a live wasm
 * module/handle in; nothing here imports the generated glue, keeping this
 * package type-checkable and installable without first building the wasm.
 */

import type { RustExprLike } from "./tree-to-mathjs";

/**
 * A parsed Rust/WASM `Expression` handle. Extends the minimal {@link RustExprLike}
 * (what the math.js bridge needs) with the handful of other methods a graphing
 * consumer commonly touches. This is intentionally a *subset* of the generated
 * `Expression` class; widen it against `pkg/math_expressions.d.ts` as needed.
 */
export interface RustExpression extends RustExprLike {
  /** Render back to text / LaTeX. */
  to_text(): string;
  to_latex(): string;
  /** Free variables, in order. */
  variables(): string[];
  /** Substitute a variable with another expression, returning a fresh handle. */
  substitute_var(variable: string, value: RustExpression): RustExpression;
  /** Reduce to a real constant, or `undefined` when it can't be. */
  evaluate_to_constant(): number | undefined;
  /** Reduce to a complex constant `[re, im]`, or `undefined`. */
  evaluate_to_complex(): Float64Array | undefined;
  /** Symbolic derivative with respect to `variable`. */
  derivative(variable: string): RustExpression;
  free(): void;
}

/**
 * The math-expressions wasm module: the free functions wasm-bindgen exports
 * plus its default `init`. `parse_text` / `parse_latex` are the entry points;
 * every other operation is a method on the returned {@link RustExpression}.
 */
export interface MathExpressionsWasmModule {
  /** wasm-bindgen initializer; call (and await) once before parsing. */
  default(moduleOrPath?: unknown): Promise<unknown>;
  parse_text(source: string): RustExpression;
  parse_latex(source: string): RustExpression;
}
