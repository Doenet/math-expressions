// Type definitions for math-expressions
// Project: https://github.com/Doenet/math-expressions
// Definitions by: GitHub Copilot

/**
 * Abstract syntax tree representation of mathematical expressions.
 * Can be a primitive (number or string), or a nested array structure
 * where the first element is an operator/function name and remaining
 * elements are operands.
 *
 * Examples:
 * - Number: `5`
 * - Variable: `"x"`
 * - Addition: `["+", 1, "x", 3]` represents `1 + x + 3`
 * - Function: `["sin", "x"]` represents `sin(x)`
 * - Power: `["^", "x", 2]` represents `x^2`
 */
export type Tree = number | string | boolean | [string, ...Tree[]];

/**
 * Type guard to check if a value is a valid Tree
 * @param value The value to check
 * @returns True if the value is a valid Tree structure
 */
export function isTree(value: unknown): value is Tree {
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

/**
 * Complex number representation (from mathjs)
 */
export interface Complex {
  re: number;
  im: number;
}

/**
 * Assumptions about variables in the expression.
 * Used to inform simplification and equality testing.
 */
export interface Assumptions {
  [variable: string]: {
    element_of?: string | string[];
    [key: string]: any;
  };
}

/**
 * Options for equality testing
 */
export interface EqualsOptions {
  /** Relative tolerance for numerical comparisons (default: 1e-12) */
  relative_tolerance?: number;
  /** Absolute tolerance for numerical comparisons (default: 0) */
  absolute_tolerance?: number;
  /** Tolerance for determining if value is zero (default: 1e-15) */
  tolerance_for_zero?: number;
  /** Allowed error in numerical coefficients (default: 0) */
  allowed_error_in_numbers?: number;
  /** Whether to include error in number exponents (default: false) */
  include_error_in_number_exponents?: boolean;
  /** Whether allowed_error_in_numbers is absolute rather than relative (default: false) */
  allowed_error_is_absolute?: boolean;
  /** Allow blank placeholders in comparison (default: false) */
  allow_blanks?: boolean;
  /** Coerce tuples and arrays to be equal (default: true) */
  coerce_tuples_arrays?: boolean;
  /** Coerce vectors to be equal (default: true) */
  coerce_vectors?: boolean;
}

/**
 * Options for evaluate_to_constant method
 */
export interface EvaluateToConstantOptions {
  /** Remove units before evaluating (default: true) */
  remove_units_first?: boolean;
  /** Scale result based on unit (default: true) */
  scale_based_on_unit?: boolean;
  /** Return NaN instead of null for non-numeric results (default: true) */
  nan_for_non_numeric?: boolean;
}

/**
 * Options for simplify method
 */
export interface SimplifyOptions {
  /** Maximum number of digits for floating point operations */
  max_digits?: number;
}

/**
 * Options for derivative method
 */
export interface DerivativeOptions {
  /** Array to store the story of the differentiation process */
  story?: string[];
}

/**
 * Options for formatting expressions as strings
 */
export interface FormatParams {
  /** Custom formatting parameters */
  [key: string]: any;
}

/**
 * Bindings for variable evaluation
 */
export interface Bindings {
  [variable: string]: number | Complex;
}

/**
 * Match result from pattern matching
 */
export interface MatchResult {
  [key: string]: Tree;
}

/**
 * Mathematical expression class with symbolic manipulation capabilities
 */
export interface Expression {
  /** Internal tree representation */
  tree: Tree;

  /** Context containing assumptions and other metadata */
  context: Context;

  // ========== Arithmetic methods (return Expression) ==========

  /**
   * Add another expression or tree
   */
  add(other: Expression | Tree): Expression;

  /**
   * Subtract another expression or tree
   */
  subtract(other: Expression | Tree): Expression;

  /**
   * Multiply by another expression or tree
   */
  multiply(other: Expression | Tree): Expression;

  /**
   * Divide by another expression or tree
   */
  divide(other: Expression | Tree): Expression;

  /**
   * Raise to a power
   */
  pow(exponent: Expression | Tree | number): Expression;

  /**
   * Modulo operation
   */
  mod(other: Expression | Tree): Expression;

  /**
   * Create a copy of the expression
   */
  copy(): Expression;

  // ========== Simplification methods ==========

  /**
   * Simplify the expression algebraically
   * @param assumptions Optional assumptions about variables
   * @param max_digits Maximum digits for numerical operations
   */
  simplify(assumptions?: Assumptions, max_digits?: number): Expression;

  /**
   * Simplify logical expressions
   * @param assumptions Optional assumptions about variables
   */
  simplify_logical(assumptions?: Assumptions): Expression;

  /**
   * Collect like terms and factors
   * @param assumptions Optional assumptions about variables
   * @param max_digits Maximum digits for numerical operations
   */
  collect_like_terms_factors(
    assumptions?: Assumptions,
    max_digits?: number,
  ): Expression;

  /**
   * Clean/flatten the expression tree
   */
  clean(): Expression;

  /**
   * Collapse unary minus operations
   */
  collapse_unary_minus(): Expression;

  /**
   * Perform vector and matrix additions and scalar multiplications
   */
  perform_vector_matrix_additions_scalar_multiplications(): Expression;

  /**
   * Remove units from expression
   */
  remove_units(): Expression;

  /**
   * Add units to expression
   */
  add_unit(unit: Expression | Tree): Expression;

  /**
   * Remove scaling units from expression
   */
  remove_scaling_units(): Expression;

  /**
   * Simplify integer square roots
   */
  simplify_integer_square_roots(): Expression;

  /**
   * Simplify ratios in expression
   * @param assumptions Optional assumptions about variables
   */
  simplify_ratios(assumptions?: Assumptions): Expression;

  // ========== Differentiation and Integration ==========

  /**
   * Compute symbolic derivative with respect to a variable
   * @param variable Variable to differentiate with respect to
   * @param story Optional array to capture differentiation steps
   */
  derivative(variable: string, story?: string[]): Expression;

  /**
   * Integrate with respect to a variable (symbolically if possible)
   * @param variable Variable to integrate with respect to
   */
  integrate(variable: string): Expression;

  /**
   * Numerical integration
   */
  integrateNumerically(): Expression;

  // ========== Expansion and Transformation ==========

  /**
   * Expand products and powers in the expression
   * @param no_division If true, don't expand divisions
   */
  expand(no_division?: boolean): Expression;

  /**
   * Factor the expression if possible
   */
  factor(): Expression;

  /**
   * Expand relations in expression
   */
  expand_relations(): Expression;

  /**
   * Substitute variables with expressions
   * @param substitutions Object mapping variable names to expressions
   */
  substitute(substitutions: {
    [variable: string]: Expression | Tree;
  }): Expression;

  /**
   * Substitute a component of a container (list, tuple, vector, array)
   * @param component Index or array of indices to substitute
   * @param value New value to substitute
   */
  substitute_component(
    component: number | number[],
    value: Expression | Tree,
  ): Expression;

  /**
   * Get a component of a container (list, tuple, vector, array)
   * @param component Index or array of indices to retrieve
   */
  get_component(component: number | number[]): Expression;

  /**
   * Perform vector scalar multiplications
   * @param include_tuples Whether to include tuples
   */
  perform_vector_scalar_multiplications(include_tuples?: boolean): Expression;

  /**
   * Perform matrix scalar multiplications
   */
  perform_matrix_scalar_multiplications(): Expression;

  /**
   * Perform matrix multiplications
   * @param include_vectors Whether to include vectors
   * @param include_tuples Whether to include tuples
   */
  perform_matrix_multiplications(
    include_vectors?: boolean,
    include_tuples?: boolean,
  ): Expression;

  // ========== Normalization methods ==========

  /**
   * Normalize function names to standard forms
   */
  normalize_function_names(): Expression;

  /**
   * Normalize applied functions (e.g., f(x) notation)
   */
  normalize_applied_functions(): Expression;

  /**
   * Normalize negative numbers representation
   */
  normalize_negative_numbers(): Expression;

  /**
   * Normalize the order of arguments for angles and line segments
   */
  normalize_angle_linesegment_arg_order(): Expression;

  /**
   * Apply default ordering to expression
   */
  default_order(): Expression;

  /**
   * Convert constants to floating point numbers
   */
  constants_to_floats(): Expression;

  /**
   * Convert log subscripts to two-argument log function
   * Converts log_a(b) to log(a, b)
   */
  log_subscript_to_two_arg_log(): Expression;

  /**
   * Convert subscripts to strings for single variable names
   * Converts variables like x_t to single string variable names
   * @param force - If true, convert all subscripts; if false (default), only convert when both parts are strings/numbers
   */
  subscripts_to_strings(force?: boolean): Expression;

  /**
   * Normalize subscripts to strings
   */
  subscripts_to_strings(): Expression;

  /**
   * Convert strings to subscripts
   */
  strings_to_subscripts(): Expression;

  /**
   * Convert tuples to vectors
   */
  tuples_to_vectors(): Expression;

  /**
   * Convert to intervals
   */
  to_intervals(): Expression;

  /**
   * Convert alternative vectors to vectors
   */
  altvectors_to_vectors(): Expression;

  /**
   * Convert log subscripts to two-argument log
   */
  log_subscript_to_two_arg_log(): Expression;

  /**
   * Substitute abs function
   */
  substitute_abs(): Expression;

  // ========== Rounding methods ==========

  /**
   * Round numbers to specified precision (significant figures)
   * @param precision Number of significant figures
   */
  round_numbers_to_precision(precision: number): Expression;

  /**
   * Round numbers to specified number of decimal places
   * @param decimals Number of decimal places
   */
  round_numbers_to_decimals(decimals: number): Expression;

  /**
   * Round numbers to precision plus decimals
   * @param precision Number of significant figures
   * @param decimals Number of decimal places
   */
  round_numbers_to_precision_plus_decimals(
    precision: number,
    decimals: number,
  ): Expression;

  // ========== Number evaluation ==========

  /**
   * Evaluate numbers in the expression
   * @param options Evaluation options
   */
  evaluate_numbers(options?: {
    max_digits?: number;
    skip_ordering?: boolean;
    evaluate_functions?: boolean;
    set_small_zero?: number | boolean;
    assumptions?: Assumptions;
  }): Expression;

  /**
   * Set small numbers to zero
   * @param tolerance Tolerance threshold
   */
  set_small_zero(tolerance?: number): Expression;

  // ========== Solve and Rational methods ==========

  /**
   * Solve linear equation for a variable
   * @param variable Variable to solve for
   */
  solve_linear(variable: string): Expression;

  /**
   * Reduce to rational form (numerator/denominator)
   */
  reduce_rational(): Expression;

  /**
   * Find common denominator for rational expressions
   */
  common_denominator(): Expression;

  /**
   * Get the numerator of a rational expression
   */
  get_numerator(): Expression;

  /**
   * Get the denominator of a rational expression
   */
  get_denominator(): Expression;

  // ========== Matrix operations ==========

  /**
   * Create a matrix from expression
   */
  matrix(): Expression;

  /**
   * Vector addition
   * @param other Vector to add
   */
  vector_add(other: Expression | Tree): Expression;

  /**
   * Vector subtraction
   * @param other Vector to subtract
   */
  vector_sub(other: Expression | Tree): Expression;

  /**
   * Scalar multiplication
   * @param scalar Scalar value
   */
  scalar_mul(scalar: number): Expression;

  /**
   * Dot product
   * @param other Vector for dot product
   */
  dot_prod(other: Expression | Tree): Expression;

  /**
   * Cross product
   * @param other Vector for cross product
   */
  cross_prod(other: Expression | Tree): Expression;

  // ========== Sets operations ==========

  /**
   * Create discrete infinite set from expression
   */
  create_discrete_infinite_set(): Expression;

  // ========== Inspection methods (return other types) ==========

  /**
   * Get list of variables in the expression
   * @param include_subscripts Whether to include subscripted variables
   */
  variables(include_subscripts?: boolean): string[];

  /**
   * Get list of operators used in the expression
   */
  operators(): string[];

  /**
   * Get list of functions used in the expression
   */
  functions(): string[];

  /**
   * Test equality with another expression
   * @param other Expression to compare with
   * @param options Comparison options
   */
  equals(other: Expression, options?: EqualsOptions): boolean;

  /**
   * Test equality using syntax comparison
   * @param other Expression to compare with
   * @param options Comparison options
   */
  equalsViaSyntax(other: Expression, options?: EqualsOptions): boolean;

  /**
   * Test equality using complex number evaluation
   * @param other Expression to compare with
   * @param options Comparison options
   */
  equalsViaComplex(other: Expression, options?: EqualsOptions): boolean;

  /**
   * Test equality using real number evaluation
   * @param other Expression to compare with
   * @param options Comparison options
   */
  equalsViaReal(other: Expression, options?: EqualsOptions): boolean;

  /**
   * Test equality using finite field arithmetic
   * @param other Expression to compare with
   * @param options Comparison options
   */
  equalsViaFiniteField(other: Expression, options?: EqualsOptions): boolean;

  /**
   * Get an evaluator function for this expression
   * Returns a bound function that evaluates the expression with variable bindings
   * @returns A function that takes variable bindings and returns the evaluated result
   */
  f(): (bindings: Bindings) => number | Complex;

  /**
   * Evaluate expression with variable bindings
   * @param bindings Object mapping variable names to numeric values
   */
  evaluate(bindings: Bindings): number | Complex;

  /**
   * Evaluate expression in a finite field
   * @param bindings Variable bindings
   * @param modulus Modulus for finite field
   */
  finite_field_evaluate(bindings: Bindings, modulus: number): number;

  /**
   * Evaluate to a constant number if possible
   * @param options Evaluation options
   * @returns Constant value or null if expression contains variables
   */
  evaluate_to_constant(options?: EvaluateToConstantOptions): number | null;

  /**
   * Check if expression is analytic (has no discontinuities)
   * @param variables Variables to check analyticity over
   */
  isAnalytic(variables?: string[]): boolean;

  /**
   * Match expression against a pattern
   * @param pattern Pattern tree to match against
   * @param allow_permutations Allow reordering of commutative operations
   */
  match(pattern: Tree, allow_permutations?: boolean): MatchResult | null;

  /**
   * Check if expression contains a sign error
   */
  sign_error(): boolean;

  // ========== Formatting methods ==========

  /**
   * Convert to text string representation
   * @param params Formatting parameters
   */
  toString(params?: FormatParams): string;

  /**
   * Convert to LaTeX representation
   * @param params Formatting parameters
   */
  toLatex(params?: FormatParams): string;

  /**
   * Alias for toLatex
   */
  tex(params?: FormatParams): string;

  /**
   * Convert to XML/MathML representation
   */
  toXML(): string;

  /**
   * Convert to GLSL shader code
   */
  toGLSL(): string;

  /**
   * Convert to Guppy representation
   */
  toGuppy(): string;

  /**
   * Convert to MathJS representation
   */
  toMathjs(): any;

  /**
   * Serialize to JSON
   */
  toJSON(): {
    objectType: "math-expression";
    tree: Tree;
    assumptions?: Assumptions;
  };

  // ========== Mathematical function methods ==========
  // These create function applications and return Expression

  abs(): Expression;
  exp(): Expression;
  log(): Expression;
  log10(): Expression;
  sign(): Expression;
  sqrt(): Expression;
  conj(): Expression;
  im(): Expression;
  re(): Expression;
  factorial(): Expression;
  gamma(): Expression;
  erf(): Expression;
  acos(): Expression;
  acosh(): Expression;
  acot(): Expression;
  acoth(): Expression;
  acsc(): Expression;
  acsch(): Expression;
  asec(): Expression;
  asech(): Expression;
  asin(): Expression;
  asinh(): Expression;
  atan(): Expression;
  atanh(): Expression;
  cos(): Expression;
  cosh(): Expression;
  cot(): Expression;
  coth(): Expression;
  csc(): Expression;
  csch(): Expression;
  sec(): Expression;
  sech(): Expression;
  sin(): Expression;
  sinh(): Expression;
  tan(): Expression;
  tanh(): Expression;
  atan2(other: Expression | Tree): Expression;
}

/**
 * Converter classes for transforming between different representations
 */
export interface Converters {
  astToLatexObj: any;
  astToTextObj: any;
  astToGuppyObj: any;
  astToMathjsObj: any;
  astToFiniteFieldObj: any;
  latexToAstObj: any;
  latexToGuppyObj: any;
  latexToMathjsObj: any;
  latexToTextObj: any;
  mathjsToAstObj: any;
  mathjsToGuppyObj: any;
  mathjsToLatexObj: any;
  mathjsToTextObj: any;
  mmlToAstObj: any;
  mmlToGuppyObj: any;
  mmlToLatexObj: any;
  mmlToMathjsObj: any;
  mmlToTextObj: any;
  textToAstObj: any;
  textToGuppyObj: any;
  textToLatexObj: any;
  textToMathjsObj: any;
}

/**
 * Utility functions
 */
export interface Utils {
  /**
   * Match a tree against a pattern
   */
  match(
    tree: Tree,
    pattern: Tree,
    allow_permutations?: boolean,
  ): MatchResult | null;

  /**
   * Flatten an AST tree
   */
  flatten(tree: Tree): Tree;

  /**
   * Unflatten left-associative operations
   */
  unflattenLeft(tree: Tree): Tree;

  /**
   * Unflatten right-associative operations
   */
  unflattenRight(tree: Tree): Tree;
}

/**
 * Main context interface with factory methods and global operations
 */
export interface Context {
  /**
   * ZmodN class for finite field arithmetic
   */
  ZmodN: any;

  /**
   * Current assumptions about variables
   */
  assumptions: Assumptions;

  /**
   * Parser parameters
   */
  parser_parameters: { [key: string]: any };

  /**
   * Parse expression from text format
   * @param text Text representation (e.g., "x^2 + 2x + 1")
   */
  fromText(text: string): Expression;

  /**
   * Parse expression from LaTeX format
   * @param latex LaTeX representation (e.g., "\\frac{x+1}{2}")
   */
  fromLatex(latex: string): Expression;

  /**
   * Alias for fromLatex (LaTeX with capital T)
   */
  fromLaTeX(latex: string): Expression;

  /**
   * Alias for fromLatex (TeX)
   */
  fromTeX(latex: string): Expression;

  /**
   * Alias for fromLatex (Tex)
   */
  fromTex(latex: string): Expression;

  /**
   * Alias for fromLatex (legacy)
   */
  parse_tex(latex: string): Expression;

  /**
   * Parse expression from MathML/XML format
   * @param mml MathML string
   */
  fromMml(mml: string): Expression;

  /**
   * Create expression from abstract syntax tree
   * @param tree AST representation
   */
  fromAst(tree: Tree): Expression;

  /**
   * Parse from any supported format (auto-detect)
   * @param input String or tree representation
   */
  from(input: string | number | Tree): Expression;

  /**
   * Alias for fromText
   */
  parse(text: string): Expression;

  /**
   * Add assumption about a variable
   * @param assumption Assumption object
   * @param exclude_generic Exclude from generic assumptions
   */
  add_assumption(assumption: {
    variable?: string;
    element_of?: string | string[];
    [key: string]: any;
  }, exclude_generic?: boolean): void;

  /**
   * Add generic assumption about a variable
   * @param assumption Assumption object
   */
  add_generic_assumption(assumption: {
    variable?: string;
    element_of?: string | string[];
    [key: string]: any;
  }): void;

  /**
   * Remove assumption about a variable
   * @param assumption Assumption object or variable name
   */
  remove_assumption(assumption: string | { variable: string } | any): void;

  /**
   * Remove generic assumption about a variable
   * @param assumption Assumption object
   */
  remove_generic_assumption(assumption: any): void;

  /**
   * Get assumptions for variables
   * @param variables Variable names or expression
   * @param params Optional parameters
   */
  get_assumptions(variables: string[] | Expression, params?: any): Assumptions;

  /**
   * Clear all assumptions
   */
  clear_assumptions(): void;

  /**
   * Set context to default settings
   */
  set_to_default(): void;

  /**
   * The Expression class constructor
   */
  class: new (ast: Tree, context: Context) => Expression;

  /**
   * Converter objects
   */
  converters: Converters;

  /**
   * Utility functions
   */
  utils: Utils;

  /**
   * MathJS instance
   */
  math: any;

  /**
   * JSON reviver for deserializing expressions
   */
  reviver(key: string, value: any): any;

  // ========== Factory methods for tree transformations (all methods from Expression) ==========
  // These are the same methods available on Expression instances, but work on Trees directly

  // ========== Arithmetic methods ==========
  add(expr: Tree, other: Expression | Tree): Expression;
  subtract(expr: Tree, other: Expression | Tree): Expression;
  multiply(expr: Tree, other: Expression | Tree): Expression;
  divide(expr: Tree, other: Expression | Tree): Expression;
  pow(expr: Tree, exponent: Expression | Tree | number): Expression;
  mod(expr: Tree, other: Expression | Tree): Expression;
  copy(expr: Tree): Expression;

  // ========== Simplification methods ==========
  simplify(
    expr: Tree,
    assumptions?: Assumptions,
    max_digits?: number,
  ): Expression;
  simplify_logical(expr: Tree, assumptions?: Assumptions): Expression;
  collect_like_terms_factors(
    expr: Tree,
    assumptions?: Assumptions,
    max_digits?: number,
  ): Expression;
  clean(expr: Tree): Expression;
  collapse_unary_minus(expr: Tree): Expression;
  perform_vector_matrix_additions_scalar_multiplications(
    expr: Tree,
  ): Expression;
  remove_units(expr: Tree): Expression;
  add_unit(expr: Tree, unit: Expression | Tree): Expression;
  remove_scaling_units(expr: Tree): Expression;
  simplify_integer_square_roots(expr: Tree): Expression;
  simplify_ratios(expr: Tree, assumptions?: Assumptions): Expression;

  // ========== Differentiation and Integration ==========
  derivative(expr: Tree, variable: string, story?: string[]): Expression;
  integrate(expr: Tree, variable: string): Expression;
  integrateNumerically(expr: Tree): Expression;

  // ========== Expansion and Transformation ==========
  expand(expr: Tree, no_division?: boolean): Expression;
  factor(expr: Tree): Expression;
  expand_relations(expr: Tree): Expression;
  substitute(
    expr: Tree,
    substitutions: { [variable: string]: Expression | Tree },
  ): Expression;
  substitute_component(
    expr: Tree,
    component: number | number[],
    value: Expression | Tree,
  ): Expression;
  get_component(expr: Tree, component: number | number[]): Expression;
  perform_vector_scalar_multiplications(
    expr: Tree,
    include_tuples?: boolean,
  ): Expression;
  perform_matrix_scalar_multiplications(expr: Tree): Expression;
  perform_matrix_multiplications(
    expr: Tree,
    include_vectors?: boolean,
    include_tuples?: boolean,
  ): Expression;

  // ========== Normalization methods ==========
  normalize_function_names(expr: Tree): Expression;
  normalize_applied_functions(expr: Tree): Expression;
  normalize_negative_numbers(expr: Tree): Expression;
  normalize_angle_linesegment_arg_order(expr: Tree): Expression;
  default_order(expr: Tree): Expression;
  constants_to_floats(expr: Tree): Expression;
  log_subscript_to_two_arg_log(expr: Tree): Expression;
  subscripts_to_strings(expr: Tree, force?: boolean): Expression;
  strings_to_subscripts(expr: Tree): Expression;
  tuples_to_vectors(expr: Tree): Expression;
  to_intervals(expr: Tree): Expression;
  altvectors_to_vectors(expr: Tree): Expression;
  substitute_abs(expr: Tree): Expression;

  // ========== Rounding methods ==========
  round_numbers_to_precision(expr: Tree, precision: number): Expression;
  round_numbers_to_decimals(expr: Tree, decimals: number): Expression;
  round_numbers_to_precision_plus_decimals(
    expr: Tree,
    precision: number,
    decimals: number,
  ): Expression;

  // ========== Number evaluation ==========
  evaluate_numbers(
    expr: Tree,
    options?: {
      max_digits?: number;
      skip_ordering?: boolean;
      evaluate_functions?: boolean;
      set_small_zero?: number | boolean;
      assumptions?: Assumptions;
    },
  ): Expression;
  set_small_zero(expr: Tree, tolerance?: number): Expression;

  // ========== Solve and Rational methods ==========
  solve_linear(expr: Tree, variable: string): Expression;
  reduce_rational(expr: Tree): Expression;
  common_denominator(expr: Tree): Expression;
  get_numerator(expr: Tree): Expression;
  get_denominator(expr: Tree): Expression;

  // ========== Matrix operations ==========
  matrix(expr: Tree): Expression;
  vector_add(expr: Tree, other: Expression | Tree): Expression;
  vector_sub(expr: Tree, other: Expression | Tree): Expression;
  scalar_mul(expr: Tree, scalar: number): Expression;
  dot_prod(expr: Tree, other: Expression | Tree): Expression;
  cross_prod(expr: Tree, other: Expression | Tree): Expression;

  // ========== Sets operations ==========
  create_discrete_infinite_set(expr: Tree): Expression;

  // ========== Inspection methods ==========
  variables(expr: Tree, include_subscripts?: boolean): string[];
  operators(expr: Tree): string[];
  functions(expr: Tree): string[];
  equals(
    expr: Tree,
    other: Expression | Tree,
    options?: EqualsOptions,
  ): boolean;
  equalsViaSyntax(
    expr: Tree,
    other: Expression | Tree,
    options?: EqualsOptions,
  ): boolean;
  equalsViaComplex(
    expr: Tree,
    other: Expression | Tree,
    options?: EqualsOptions,
  ): boolean;
  equalsViaReal(
    expr: Tree,
    other: Expression | Tree,
    options?: EqualsOptions,
  ): boolean;
  equalsViaFiniteField(
    expr: Tree,
    other: Expression | Tree,
    options?: EqualsOptions,
  ): boolean;
  f(expr: Tree): (bindings: Bindings) => number | Complex;
  evaluate(expr: Tree, bindings: Bindings): number | Complex;
  finite_field_evaluate(
    expr: Tree,
    bindings: Bindings,
    modulus: number,
  ): number;
  evaluate_to_constant(
    expr: Tree,
    options?: EvaluateToConstantOptions,
  ): number | null;
  isAnalytic(expr: Tree, variables?: string[]): boolean;
  match(
    expr: Tree,
    pattern: Tree,
    allow_permutations?: boolean,
  ): MatchResult | null;
  sign_error(expr: Tree): boolean;

  // ========== Formatting methods ==========
  toString(expr: Tree, params?: FormatParams): string;
  toLatex(expr: Tree, params?: FormatParams): string;
  tex(expr: Tree, params?: FormatParams): string;
  toXML(expr: Tree): string;
  toGLSL(expr: Tree): string;
  toGuppy(expr: Tree): string;
  toMathjs(expr: Tree): any;

  // ========== Mathematical function methods ==========
  abs(expr: Tree): Expression;
  exp(expr: Tree): Expression;
  log(expr: Tree): Expression;
  log10(expr: Tree): Expression;
  sign(expr: Tree): Expression;
  sqrt(expr: Tree): Expression;
  conj(expr: Tree): Expression;
  im(expr: Tree): Expression;
  re(expr: Tree): Expression;
  factorial(expr: Tree): Expression;
  gamma(expr: Tree): Expression;
  erf(expr: Tree): Expression;
  acos(expr: Tree): Expression;
  acosh(expr: Tree): Expression;
  acot(expr: Tree): Expression;
  acoth(expr: Tree): Expression;
  acsc(expr: Tree): Expression;
  acsch(expr: Tree): Expression;
  asec(expr: Tree): Expression;
  asech(expr: Tree): Expression;
  asin(expr: Tree): Expression;
  asinh(expr: Tree): Expression;
  atan(expr: Tree): Expression;
  atanh(expr: Tree): Expression;
  cos(expr: Tree): Expression;
  cosh(expr: Tree): Expression;
  cot(expr: Tree): Expression;
  coth(expr: Tree): Expression;
  csc(expr: Tree): Expression;
  csch(expr: Tree): Expression;
  sec(expr: Tree): Expression;
  sech(expr: Tree): Expression;
  sin(expr: Tree): Expression;
  sinh(expr: Tree): Expression;
  tan(expr: Tree): Expression;
  tanh(expr: Tree): Expression;
  atan2(expr: Tree, other: Expression | Tree): Expression;
}

/**
 * Main export: Context object with factory methods
 */
declare const MathExpression: Context;

export default MathExpression;
