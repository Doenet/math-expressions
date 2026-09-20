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
 *
 * **Scope of the claim.** It covers what `lib/**` calls, and that much is
 * enforced: `math-expressions-js-compat`'s `npm run typecheck` type-checks
 * `lib/**` against these declarations and CI gates on it. It is not a mirror of
 * the whole generated surface — plenty of wasm-bindgen exports are absent
 * because nothing calls them — and it says nothing about `spec/**`, which is
 * not type-checked. Before this was enforced the file had drifted ten members
 * and one arity behind its own consumers, silently.
 */

/** A parsed Rust/WASM `Expression` handle — the full method surface. */
export interface WasmExpression {
  tree_json(): string;
  to_text(): string;
  to_latex(): string;
  /** Render to text under a JSON options object (unicode, notation, padToDigits, padToDecimals, showBlanks, explicitMultiplicationSymbols). */
  to_text_with_options(options_json: string): string;
  /** Render to LaTeX under a JSON options object (notation, padToDigits, padToDecimals, showBlanks, explicitMultiplicationSymbols). */
  to_latex_with_options(options_json: string): string;
  to_serialized(): string;
  variables(): string[];
  functions(): string[];
  operators(): string[];

  equals(other: WasmExpression): boolean;
  equals_with_options(other: WasmExpression, optionsJson: string): boolean;
  equals_via_real(other: WasmExpression): boolean;
  /** Structural comparison against `key` under a criterion JSON (e.g.
   * `"sameStructure"`, which routes to Rust `equals_syntactic`). */
  structural_equality(key: WasmExpression, comparisonJson: string): boolean;
  /** {@link structural_equality} under the `.equals` option object (tolerances,
   * `nSignErrorsMatched`, …) rather than the criterion alone. */
  structural_equality_with_options(
    key: WasmExpression,
    comparisonJson: string,
    optionsJson: string,
  ): boolean;
  /** Structure-only check (no key): is this expression in the form named by the
   * criterion JSON? Returns a JSON string `{"ok":boolean,"why":string|null}`; an
   * unknown criterion yields `{"ok":false,"why":"unknown structural comparison"}`. */
  check_structural_comparison(comparisonJson: string): string;
  is_zero(): boolean | undefined;
  is_analytic(
    allowAbs: boolean,
    allowArg: boolean,
    allowRelation: boolean,
  ): boolean;

  derivative(variable: string): WasmExpression;
  /** Exact real solutions of `d/dvariable = 0`, ascending; `undefined` if undecided. */
  critical_points(variable: string): WasmExpression[] | undefined;
  integrate(variable: string): WasmExpression | undefined;
  /** Numeric definite integral over `[lower, upper]`, backed by certified
   * quadrature; `undefined` when it cannot be certified (never a wrong value). */
  integrate_numerically(
    variable: string,
    lower: number,
    upper: number,
  ): number | undefined;

  simplify(): WasmExpression;
  simplify_with_assumptions(assumptions: string[]): WasmExpression;
  simplify_logical(): WasmExpression;
  /** Sort into the JS default order without evaluating (`simplify="normalizeOrder"`). */
  default_order(): WasmExpression;
  expand(): WasmExpression;
  factor(): WasmExpression;
  evaluate_numbers(): WasmExpression;
  /** The `skip_ordering` fold: `1+x+2` stays `1+x+2`. */
  evaluate_numbers_preserve_order(): WasmExpression;
  /** Fold numbers *and* applied functions (`floor(55.33)` → `55`). */
  evaluate_numbers_evaluate_functions(): WasmExpression;
  /** Fold to floats rather than exact rationals, capped at `max_digits`. */
  evaluate_numbers_to_floats(
    skipOrdering: boolean,
    evaluateFunctions: boolean,
    maxDigits: number,
  ): WasmExpression;
  collect_like_terms_factors(): WasmExpression;
  simplify_ratios(): WasmExpression;
  reduce_rational(): WasmExpression;
  together(): WasmExpression;
  normalize_function_names(): WasmExpression;
  /** Canonicalize applied-function spellings (`sin^2 x` → `(sin x)^2`). */
  normalize_applied_functions(): WasmExpression;
  /** Rewrite `a + (-b)` as `a - b` and friends. */
  normalize_negative_numbers(): WasmExpression;
  /** Split a chained relation (`a < b < c`) into its conjunction. */
  expand_relations(): WasmExpression;
  constants_to_floats(): WasmExpression;

  tuples_to_vectors(): WasmExpression;
  altvectors_to_vectors(): WasmExpression;
  to_intervals(): WasmExpression;
  /** Move `+`/scalar-`*` inside vector & matrix containers (the grading shape pass). */
  perform_vector_matrix_additions_scalar_multiplications(): WasmExpression;
  /** `force` flattens even subscripts that would round-trip, which is what
   * `variables(include_subscripts)` needs. */
  subscripts_to_strings(force: boolean): WasmExpression;
  strings_to_subscripts(): WasmExpression;
  copy(): WasmExpression;

  remove_units(scaleBasedOnUnit: boolean): WasmExpression;
  remove_scaling_units(): WasmExpression;
  add_unit(unit: string): WasmExpression;
  set_small_zero(tolerance: number): WasmExpression;

  round_numbers_to_precision(sigFigs: number): WasmExpression;
  round_numbers_to_decimals(decimals: number): WasmExpression;
  round_numbers_to_precision_plus_decimals(
    digits: number,
    decimals: number,
  ): WasmExpression;

  evaluate_to_constant(): number | undefined;
  evaluate_to_complex(): Float64Array | undefined;
  evaluate(vars: string[], values: Float64Array): number | undefined;
  /** One call, many points of a single variable; `NaN` where there is no finite real value. */
  evaluate_many(variable: string, values: Float64Array): Float64Array;
  substitute_var(variable: string, value: WasmExpression): WasmExpression;
  /** Simultaneous substitution from a JSON `{name: tree}` map — not a sequence
   * of `substitute_var` calls, so bindings cannot capture each other. */
  substitute_map(mapJson: string): WasmExpression;

  /** Operand path into the tree spelling, 0-based; `undefined` if out of range. */
  get_component(path: Uint32Array): WasmExpression | undefined;
  substitute_component(
    path: Uint32Array,
    value: WasmExpression,
  ): WasmExpression | undefined;

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
  scalar_mul(other: WasmExpression): WasmExpression;

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
  simplify(expr: WasmExpression): WasmExpression;
  /** `.equals` evaluated under these assumptions. */
  equals_expressions(
    a: WasmExpression,
    b: WasmExpression,
    optionsJson?: string | null,
  ): boolean;
  /** `undefined` when `expr` is not linear in `variable`, or when the sign an
   * inequality's direction depends on cannot be settled from these facts. */
  solve_linear(
    expr: WasmExpression,
    variable: string,
  ): WasmExpression | undefined;
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
  /** A numeric literal, built without a JSON round trip. Non-finite input
   * builds the same `Const` spelling the JSON path does. */
  from_number(value: number): WasmExpression;
  /** `{offsets + k·periods : k}` over an index range; `undefined` if the
   * arguments do not describe one. */
  discrete_infinite_set(
    offsets: WasmExpression,
    periods: WasmExpression,
    minIndexJson?: string | null,
    maxIndexJson?: string | null,
  ): WasmExpression | undefined;
  match_template(treeJson: string, patternJson: string): string | undefined;
  match_template_with_options(
    treeJson: string,
    patternJson: string,
    optionsJson: string,
  ): string | undefined;
  flatten_ast(treeJson: string): string | undefined;
  unflatten_left(treeJson: string): string | undefined;
  unflatten_right(treeJson: string): string | undefined;
  /** Order two trees by the legacy `default_order` sort key (-1/0/1). */
  cmp_default_order(aJson: string, bJson: string): number | undefined;
  /** Push `not` inward only — no simplification, no relation reorientation. */
  push_not_ast(treeJson: string): string | undefined;
  flatten_logical_ast(treeJson: string): string | undefined;
  /** Restate a relation with `variable` alone on the left. */
  solve_linear_ast(treeJson: string, variable: string): string | undefined;
  /** `{"b": tree, "coefficients": [tree, …]}`, positional in `variables`. */
  linear_decomposition_ast(
    treeJson: string,
    variables: string[],
  ): string | undefined;
  /**
   * Declare which of `pi`/`e`/`i` are mathematical constants here rather than
   * variable names, from a JSON object of `ConstantPolicy` fields. Absent keys
   * keep their values; unknown keys throw.
   */
  set_constant_policy(optionsJson: string): void;
  /** The constant policy in effect, as JSON (same keys as the setter takes). */
  get_constant_policy(): string;
  Assumptions: WasmAssumptionsConstructor;
  /** Distinct symbol names interned this session — an append-only memory gauge. */
  interner_size(): number;
  /** Dormand-Prince ODE integrator with dense output — the `numeric.dopri` core. */
  solve_ode(
    f: (t: number, y: Float64Array) => number[],
    t0: number,
    t1: number,
    y0: Float64Array,
    tol: number,
    maxSteps: number,
  ): WasmOdeSolution;
}

/** A computed ODE trajectory with dense output (the `solve_ode` result). */
export interface WasmOdeSolution {
  /** Interpolated state at `t` (length = {@link dim}). */
  at(t: number): Float64Array;
  /** States at each `ts[i]`, flattened row-major. */
  at_many(ts: Float64Array): Float64Array;
  dim(): number;
  last_t(): number;
  last_y(): Float64Array;
  /** True when integration stopped before `t1` (blow-up / step budget). */
  terminated_early(): boolean;
  /** Accepted step abscissas. */
  times(): Float64Array;
  /** Release the wasm handle. */
  free(): void;
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
