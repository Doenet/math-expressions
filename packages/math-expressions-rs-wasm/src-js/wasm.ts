/*
 * Structural TypeScript surface for the math-expressions Rust/WASM module
 * (`./pkg`, emitted by wasm-bindgen from the sibling `crate/`).
 *
 * This is the single source of truth for the wasm handle/module types across
 * the JS packages: `math-expressions-js-compat` imports these rather than
 * hand-mirroring its own copy. The *authoritative, exhaustive* types are the
 * generated `pkg/math_expressions_wasm.d.ts`, but that is a git-ignored build
 * artifact whose shape varies by wasm-bindgen `--target`, so this module
 * declares the structural surface the bindings actually rely on. Nothing here
 * imports the generated glue, keeping the package type-checkable and
 * installable without first building the wasm.
 */

/** A parsed Rust/WASM `Expression` handle — the full method surface. */
export interface WasmExpression {
  tree_json(): string;
  to_text(): string;
  to_latex(): string;
  to_serialized(): string;
  variables(): string[];
  functions(): string[];

  equals(other: WasmExpression): boolean;
  equals_with_options(other: WasmExpression, optionsJson: string): boolean;
  equals_via_real(other: WasmExpression): boolean;
  /** Structural comparison against `key` under a criterion JSON (e.g.
   * `"sameStructure"`, which routes to Rust `equals_syntactic`). */
  structural_equality(key: WasmExpression, comparisonJson: string): boolean;
  /** Structure-only check (no key): is this expression in the form named by the
   * criterion JSON? Returns a JSON string `{"ok":boolean,"why":string|null}`; an
   * unknown criterion yields `{"ok":false,"why":"unknown structural comparison"}`. */
  check_structural_comparison(comparisonJson: string): string;
  is_zero(): boolean | undefined;
  is_analytic(allowAbs: boolean, allowArg: boolean, allowRelation: boolean): boolean;

  derivative(variable: string): WasmExpression;
  integrate(variable: string): WasmExpression | undefined;

  simplify(): WasmExpression;
  simplify_with_assumptions(assumptions: string[]): WasmExpression;
  simplify_logical(): WasmExpression;
  expand(): WasmExpression;
  factor(): WasmExpression;
  evaluate_numbers(): WasmExpression;
  collect_like_terms_factors(): WasmExpression;
  simplify_ratios(): WasmExpression;
  reduce_rational(): WasmExpression;
  together(): WasmExpression;
  normalize_function_names(): WasmExpression;
  constants_to_floats(): WasmExpression;

  tuples_to_vectors(): WasmExpression;
  altvectors_to_vectors(): WasmExpression;
  to_intervals(): WasmExpression;
  subscripts_to_strings(): WasmExpression;
  strings_to_subscripts(): WasmExpression;
  copy(): WasmExpression;

  remove_units(scaleBasedOnUnit: boolean): WasmExpression;
  remove_scaling_units(): WasmExpression;
  add_unit(unit: string): WasmExpression;
  set_small_zero(tolerance: number): WasmExpression;

  round_numbers_to_precision(sigFigs: number): WasmExpression;
  round_numbers_to_decimals(decimals: number): WasmExpression;
  round_numbers_to_precision_plus_decimals(digits: number, decimals: number): WasmExpression;

  evaluate_to_constant(): number | undefined;
  evaluate_to_complex(): Float64Array | undefined;
  evaluate(vars: string[], values: Float64Array): number | undefined;
  substitute_var(variable: string, value: WasmExpression): WasmExpression;

  add(other: WasmExpression): WasmExpression;
  subtract(other: WasmExpression): WasmExpression;
  multiply(other: WasmExpression): WasmExpression;
  divide(other: WasmExpression): WasmExpression;
  pow(other: WasmExpression): WasmExpression;
  mod(other: WasmExpression): WasmExpression;

  determinant(): WasmExpression;
  transpose(): WasmExpression;
  trace(): WasmExpression;
  matrix_inverse(): WasmExpression;
  rref(): WasmExpression;
  rank(): number | undefined;
  matmul(other: WasmExpression): WasmExpression;
  dot_prod(other: WasmExpression): WasmExpression;
  cross_prod(other: WasmExpression): WasmExpression;
  vector_add(other: WasmExpression): WasmExpression;
  vector_sub(other: WasmExpression): WasmExpression;

  free(): void;
}

/** A mutable assumptions set. */
export interface WasmAssumptions {
  add(relation: string): boolean;
  remove(relation: string): void;
  clear(): void;
  is_empty(): boolean;
  is_integer(expr: WasmExpression): boolean | undefined;
  is_real(expr: WasmExpression): boolean | undefined;
  is_complex(expr: WasmExpression): boolean | undefined;
  is_nonzero(expr: WasmExpression): boolean | undefined;
  is_nonnegative(expr: WasmExpression): boolean | undefined;
  is_nonpositive(expr: WasmExpression): boolean | undefined;
  is_positive(expr: WasmExpression): boolean | undefined;
  is_negative(expr: WasmExpression): boolean | undefined;
}

export interface WasmAssumptionsConstructor {
  new (): WasmAssumptions;
}

/** The free functions wasm-bindgen exports (the `parse_*` entry points, the
 * JS-tree AST boundary, and the `Assumptions` constructor). */
export interface WasmModule {
  parse_text(source: string): WasmExpression;
  parse_latex(source: string): WasmExpression;
  parse_text_with_options(source: string, optionsJson: string): WasmExpression;
  parse_latex_with_options(source: string, optionsJson: string): WasmExpression;
  from_ast(treeJson: string): WasmExpression;
  from_serialized(json: string): WasmExpression;
  match_template(treeJson: string, patternJson: string): string | undefined;
  flatten_ast(treeJson: string): string | undefined;
  unflatten_left(treeJson: string): string | undefined;
  unflatten_right(treeJson: string): string | undefined;
  Assumptions: WasmAssumptionsConstructor;
}

/**
 * Back-compat / graphing alias: the AST → math.js bridge only needs the minimal
 * {@link RustExprLike} shape, but consumers that hold a full handle can use this.
 */
export type RustExpression = WasmExpression;

/**
 * The module with the wasm-bindgen async initializer present (the `--target web`
 * shape): call (and await) `default()` once before parsing.
 */
export interface MathExpressionsWasmModule extends WasmModule {
  /** wasm-bindgen initializer; call (and await) once before parsing. */
  default(moduleOrPath?: unknown): Promise<unknown>;
}
