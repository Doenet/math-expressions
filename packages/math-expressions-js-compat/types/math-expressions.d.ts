// The public type contract of this package.
//
// These declarations are the hand-written ones that shipped with the legacy
// JavaScript library (`math-expressions@2.0.0-alpha95`, `build/index.d.ts`).
// They are reproduced here because they were never really *about* that library:
// this package is a drop-in for exactly that surface, so they describe it too.
// They come back at the same time the package becomes publishable, since
// `exports["."]` has to name a `types` entry or every TypeScript consumer sees
// `any`.
//
// WHY HAND-WRITTEN RATHER THAN GENERATED
//
// `lib/**` is loosely typed JS-in-TS — `tsc --emitDeclarationOnly` over it does
// produce declarations, but with `any` in most parameter positions, and they
// import types from `math-expressions-rs-wasm`, which is bundled into `dist/`
// and not published. A generated file would therefore be both weaker and
// unresolvable. The tradeoff taken instead is drift: this is a snapshot of the
// contract, checked by `npm run typecheck` for internal consistency and by the
// DoenetML files that import the library for usefulness, but not derived from
// the implementation. (The count of those files is stated in exactly one place,
// the table in `MATH_EXPRESSIONS_ENGINE_NOTES.md`, with the command that
// re-derives it; a number repeated here is a number that goes stale here.)
// Where the Rust engine genuinely diverges from this file, the divergence
// belongs in the release notes, not in a widened type that hides it — and where
// the engine simply does not have a member, the member does not belong here at
// all. See the note at the end of `Expression` for the 48 that went.
//
// The v3 additions over the legacy file are at the bottom: `setWasmModule`,
// `dopri`, and the default export.

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
export function isTree(value: unknown): value is Tree;

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
    /**
     * Accepted for source compatibility and **ignored**. Legacy used it to ask
     * for `null` instead of `NaN` on a non-numeric result; this engine always
     * answers `NaN` — legacy's default — so passing `false` changes nothing.
     * @deprecated has no effect
     */
    nan_for_non_numeric?: boolean;
}

/**
 * Options for analytic checks.
 */
export interface IsAnalyticOptions {
    /** Allow abs(x) in analytic checks */
    allow_abs?: boolean;
    /** Allow arg(x) in analytic checks */
    allow_arg?: boolean;
    /** Allow relation operators in analytic checks */
    allow_relation?: boolean;
}

/**
 * Options for simplify method
 */
export interface SimplifyOptions {
    /** Maximum number of digits for floating point operations */
    max_digits?: number;
}

/**
 * Parser parameters accepted by `fromText`/`fromLatex` — the same keys, in the
 * same JS spellings, that `converters.textToAstObj`/`latexToAstObj` take.
 * Passing them here parses straight to an `Expression`; going through a
 * converter yields a JSON AST, which floats every exact decimal on the way.
 */
export interface ParserOptions {
    /** Split multi-letter symbols into implicit products (`xy` → `x·y`) */
    splitSymbols?: boolean;
    /** Symbols exempt from splitting */
    unsplitSymbols?: string[];
    /** Names treated as functions when applied (`f(x)`) */
    appliedFunctionSymbols?: string[];
    /** Names always treated as functions */
    functionSymbols?: string[];
    /** Names treated as infix operators */
    operatorSymbols?: string[];
    /** LaTeX control sequences accepted as symbols (LaTeX only) */
    allowedLatexSymbols?: string[];
    /** Accept `1E5` as scientific notation rather than `1·E·5` */
    parseScientificNotation?: boolean;
    /** Allow `f x` to mean `f(x)` */
    allowSimplifiedFunctionApplication?: boolean;
    /** Read `dy/dx` as a derivative */
    parseLeibnizNotation?: boolean;
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
 * Bindings for the real-only evaluation path.
 *
 * {@link Expression.evaluate} marshals its bindings into a `Float64Array` to
 * cross the wasm boundary, so a `Complex` there becomes `NaN` rather than
 * being evaluated. {@link Expression.f} takes the wider {@link Bindings}.
 */
export interface NumericBindings {
    [variable: string]: number;
}

/**
 * Match result from pattern matching
 */
export interface MatchResult {
    [key: string]: Tree;
}

/**
 * Options for the pattern-matching methods.
 *
 * The declared shape used to be a bare `allow_permutations?: boolean` second
 * argument. That was never the API: the implementation takes an options
 * *object* and routes it through `normalizeMatchOptions`, and a bare `true`
 * takes the no-options path instead (`hasOptions` requires an object), so the
 * declaration described a call that silently does nothing.
 */
export interface MatchOptions {
    /**
     * The declared parameters, as `{name: kind}`. Present-and-empty declares
     * *no* parameters, so only an exact match succeeds; omitting it keeps the
     * legacy default where every string leaf in the pattern binds.
     *
     * Legacy's per-parameter predicate functions and regular expressions are
     * rejected with an error rather than ignored — a function cannot cross the
     * wasm boundary, and silently treating one as "any" is what made
     * `requireNumericMatches` a no-op.
     */
    variables?: { [name: string]: true | "any" | "number" | "variable" };
    /** Match `+`/`*` operands in any order. */
    allow_permutations?: boolean;
    /**
     * Parameter names that may take the operator's identity, so `a x + b`
     * matches `x` with `a = 1`, `b = 0`.
     */
    allow_implicit_identities?: string[];
    /**
     * Let a `+`/`*` pattern match a *subset* of a larger sum or product,
     * reporting the operands it did not consume as `_skipped`.
     *
     * Handled outside the core matcher — by enumerating operand subsets — so
     * it was reachable only through `me.utils.match` and was dropped by
     * `Expression.match`, which called the matcher directly. Both go through
     * one implementation now, and the option is declared because it works.
     */
    allow_extended_match?: boolean;
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
     *
     * **Both parameters are accepted and ignored.** This engine's `simplify`
     * takes no arguments; the declaration keeps them so a legacy call still
     * compiles, but passing them changes nothing. Assumptions come from the
     * context (`add_assumption`). Filed in
     * `MATH_EXPRESSIONS_UPSTREAM_REQUESTS.md`.
     *
     * @param assumptions @deprecated has no effect
     * @param max_digits @deprecated has no effect
     */
    simplify(assumptions?: Assumptions, max_digits?: number): Expression;

    /**
     * Simplify logical expressions
     *
     * @param assumptions @deprecated accepted and ignored — see `simplify`
     */
    simplify_logical(assumptions?: Assumptions): Expression;

    /**
     * Collect like terms and factors
     *
     * @param assumptions @deprecated accepted and ignored — see `simplify`
     * @param max_digits @deprecated accepted and ignored — see `simplify`
     */
    collect_like_terms_factors(
        assumptions?: Assumptions,
        max_digits?: number,
    ): Expression;

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
     * Simplify ratios in expression
     *
     * @param assumptions @deprecated accepted and ignored — see `simplify`
     */
    simplify_ratios(assumptions?: Assumptions): Expression;

    // ========== Differentiation and Integration ==========

    /**
     * Compute symbolic derivative with respect to a variable
     *
     * @param variable Variable to differentiate with respect to
     * @param story @deprecated accepted and ignored — this engine records no
     * differentiation steps, so the array is left empty
     */
    derivative(variable: string, story?: string[]): Expression;

    /**
     * Numerical integration
     */
    integrateNumerically(
        variable: string,
        lower: number,
        upper: number,
    ): number;

    // ========== Expansion and Transformation ==========

    /**
     * Expand products and powers in the expression
     *
     * @param no_division @deprecated accepted and ignored — this engine's
     * `expand` takes no arguments and always expands divisions
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
     * Substitute variables with expressions, **simultaneously** — no
     * replacement is open to the other bindings. Each binding is coerced
     * first, so a string value is parsed.
     * @param substitutions Object mapping variable names to expressions
     */
    substitute(substitutions: {
        [variable: string]: Expression | Tree;
    }): Expression;

    /**
     * Substitute variables with expressions, simultaneously. Identical to
     * `substitute` except that it does not coerce its values, so every one
     * must already be an `Expression` or a tree.
     * @param substitutions Object mapping variable names to expressions
     */
    substitute_all(substitutions: {
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
     * Apply default ordering to expression
     */
    default_order(): Expression;

    /**
     * Convert constants to floating point numbers
     */
    constants_to_floats(): Expression;

    /**
     * Convert subscripts to strings for single variable names
     * Converts variables like x_t to single string variable names
     * @param force - If true, convert all subscripts; if false (default), only convert when both parts are strings/numbers
     */
    subscripts_to_strings(force?: boolean): Expression;

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
     *
     * Always an `Expression`, never `undefined` — but when there is no answer
     * (not linear in `variable`, or a coefficient whose sign or nonzero-ness
     * the assumptions cannot settle) it is a frozen stand-in whose `.tree` is
     * `undefined`. That is legacy's own contract and is why it is not declared
     * `Expression | undefined`: callers read `.tree` off the result
     * unconditionally, and legacy handed them one to read. Test the `.tree`,
     * not the result.
     *
     * @param variable Variable to solve for
     */
    solve_linear(variable: string): Expression;

    /**
     * Reduce to rational form (numerator/denominator)
     */
    reduce_rational(): Expression;

    // ========== Matrix operations ==========

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
     * Dot product
     * @param other Vector for dot product
     */
    dot_prod(other: Expression | Tree): Expression;

    /**
     * Cross product
     * @param other Vector for cross product
     */
    cross_prod(other: Expression | Tree): Expression;

    // ========== Inspection methods (return other types) ==========

    /**
     * Get list of variables in the expression
     * @param include_subscripts Whether to include subscripted variables
     */
    variables(include_subscripts?: boolean): string[];

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
     *
     * @param other Expression to compare with
     * @param options @deprecated accepted and ignored — the tolerances come
     * from the engine's defaults. Use `equals`, whose options *are* honored.
     */
    equalsViaComplex(other: Expression, options?: EqualsOptions): boolean;

    /**
     * Test equality using real number evaluation
     *
     * @param other Expression to compare with
     * @param options @deprecated accepted and ignored — see `equalsViaComplex`
     */
    equalsViaReal(other: Expression, options?: EqualsOptions): boolean;

    /**
     * Get an evaluator function for this expression
     * Returns a bound function that evaluates the expression with variable bindings
     * @returns A function that takes variable bindings and returns the evaluated result
     */
    f(): (bindings: Bindings) => number | Complex;

    /**
     * Evaluate expression with variable bindings.
     *
     * Real only, in both directions: the bindings cross the wasm boundary as a
     * `Float64Array`, so a `Complex` binding becomes `NaN`, and the result is
     * always a `number` (`NaN` where there is none) — never a `Complex`. Use
     * {@link Expression.f} when either end can be complex; it goes through
     * math.js and handles both.
     *
     * @param bindings Object mapping variable names to numeric values
     */
    evaluate(bindings: NumericBindings): number;

    /**
     * Evaluate expression in a finite field
     * @param bindings Variable bindings
     * @param modulus Modulus for finite field
     */
    finite_field_evaluate(bindings: Bindings, modulus: number): number;

    /**
     * Evaluate to a constant if possible.
     *
     * **There are exactly two things this can return, and neither is `null`.**
     *
     * `number` — including `Infinity` and, when the expression has no numeric
     * value at all, `NaN`. `NaN` is the "no value" marker, as it was in the
     * legacy library: a free variable (`x+1`), a blank `＿`, a matrix, a
     * leftover unit all answer `NaN`. It is deliberately the same `NaN` an
     * expression can genuinely *evaluate* to (`0/0`), because a marker that
     * does not poison arithmetic is worse than no marker — `null` coerces to
     * `0` and satisfies `<`/`<=`, and this method returned `null` for a while,
     * which is how blank answers came to score full credit.
     *
     * `Complex` — a math.js complex value, for a non-real result.
     * `fromText("i").evaluate_to_constant()` is a `Complex`, as legacy's was,
     * so a caller that passes the result into math.js gets it intact.
     *
     * That means `typeof x === "number"` is the narrowing every real-valued
     * caller wants, and it is not enough to write `x !== null`: `Complex` is
     * still there and `NaN` is still a number. Prefer
     * `typeof x === "number" && !Number.isNaN(x)`.
     *
     * @param options Evaluation options
     * @returns The constant value; `NaN` when the expression has none
     */
    evaluate_to_constant(
        options?: EvaluateToConstantOptions,
    ): number | Complex;

    /**
     * Check if expression is analytic (has no discontinuities)
     *
     * The options must be an object. A `string[]` — which this was declared to
     * accept — is read as an options object, so every flag comes out `false`
     * and the call silently takes the strictest path: the same shape of bug
     * `match(true)` had.
     *
     * @param options Analytic check options
     */
    isAnalytic(options?: IsAnalyticOptions): boolean;

    /**
     * Match expression against a pattern.
     *
     * Returns `false`, not `null`, when the pattern does not match — as legacy
     * did. (This was declared `MatchResult | null` for a long time, so a
     * `!== null` test written against the declaration was always true.)
     *
     * @param pattern Pattern tree to match against
     * @param options Matching options
     */
    match(
        pattern: Expression | Tree,
        options?: MatchOptions,
    ): MatchResult | false;

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
     * Serialize to JSON
     */
    toJSON(): {
        objectType: "math-expression";
        tree: Tree;
        assumptions?: Assumptions;
    };

    // ========== Not declared: 48 legacy members this engine does not have ====
    //
    // The legacy file declared 114 members here; 48 of them are `undefined` at
    // runtime on this engine, so calling one is a `TypeError` and not the
    // documented result. They are no longer declared. A `.d.ts` whose job is to
    // describe a drop-in earns nothing by promising members that are not there:
    // keeping them made `expr.sin()` a *compile-time success* and a runtime
    // failure, which is the worst of the two places to find out. Removing them
    // moves the report to `tsc`, names the member, and costs a caller who was
    // going to fail anyway nothing.
    //
    // Reinstate a name here the moment `lib/` implements it — this list is the
    // gap, not a decision to leave it open, and `MATH_EXPRESSIONS_UPSTREAM_
    // REQUESTS.md` in DoenetML tracks it as one of the two open asks:
    //
    //   the elementwise numeric applications — abs, exp, log, log10, sqrt,
    //     sign, re, im, conj, factorial, gamma, erf, and the whole
    //     trig/hyperbolic/inverse family including atan2 (37 members);
    //   the matrix/vector helpers — perform_matrix_multiplications,
    //     perform_matrix_scalar_multiplications,
    //     perform_vector_scalar_multiplications;
    //   and the one-offs — clean, common_denominator, substitute_abs,
    //     log_subscript_to_two_arg_log, normalize_angle_linesegment_arg_order,
    //     equalsViaFiniteField, operators, toGuppy.
    //
    // Each is mirrored on `Context` (`me.abs(expr)`, …) and is absent there for
    // the same reason, so 96 declarations went, not 48 — plus `Context`'s own
    // two, `ZmodN` and `parser_parameters`, which are Context-only properties
    // and so were outside the `Expression` audit that found the 48. Nothing in
    // DoenetML calls any of them; re-derive with the audit in the
    // upstream-requests file before trusting that sentence again.
    //
    // What is left is true: every member either interface declares is present
    // at runtime — 66 on `Expression`, 87 on `Context`.
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
        options?: MatchOptions,
    ): MatchResult | false;

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
     * The assumption store, as an object of methods.
     *
     * Not a variable-keyed map, which is what this was declared to be: legacy
     * carried the same add/get/remove methods on `me.assumptions` as on `me`
     * itself, and this port keeps that, forwarding the wasm predicates through
     * it as well (`lib/assumptions/element_of_sets` reads it as its default
     * source). The per-variable facts are under `byvar`.
     */
    assumptions: {
        /** The recorded fact per variable; `undefined` where there is none. */
        byvar: { [variable: string]: Tree | undefined };
        /** Facts implied by combining the recorded ones. */
        derived: { [variable: string]: Tree | undefined };
        /** The generic assumption, applying to every variable. */
        generic: Tree | undefined;
        get_assumptions(
            variables: string | [string[]] | Expression | Tree,
            params?: {
                exclude_variables?: string | string[];
                omit_derived?: boolean;
            },
        ): Tree | undefined;
        add_assumption(
            assumption: Expression | Tree,
            exclude_generic?: boolean,
        ): void;
        add_generic_assumption(assumption: Expression | Tree): void;
        remove_assumption(assumption: Expression | Tree): void;
        remove_generic_assumption(assumption: Expression | Tree): void;
        clear_assumptions(): void;
        set_to_default(): void;
        /** Three-valued predicates, forwarded to the wasm store. */
        [predicate: string]: any;
    };

    /**
     * Parse expression from text format
     * @param text Text representation (e.g., "x^2 + 2x + 1")
     */
    fromText(text: string, options?: ParserOptions): Expression;

    /**
     * Parse expression from LaTeX format
     * @param latex LaTeX representation (e.g., "\\frac{x+1}{2}")
     */
    fromLatex(latex: string, options?: ParserOptions): Expression;

    /**
     * Alias for fromLatex (LaTeX with capital T)
     */
    fromLaTeX(latex: string, options?: ParserOptions): Expression;

    /**
     * Alias for fromLatex (TeX)
     */
    fromTeX(latex: string, options?: ParserOptions): Expression;

    /**
     * Alias for fromLatex (Tex)
     */
    fromTex(latex: string, options?: ParserOptions): Expression;

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
     *
     * `undefined` for `undefined`/`null` input, which is legacy's answer to
     * "nothing converts to nothing": callers write `me.from(value)` over a
     * table whose empty rows mean "no expression". Declared, because a caller
     * who does not check gets a `TypeError` one line later.
     *
     * @param input String or tree representation
     */
    from(input: string | number | Tree): Expression | undefined;

    /**
     * Alias for fromText
     */
    parse(text: string, options?: ParserOptions): Expression;

    /**
     * Add assumption about a variable
     * @param assumption Assumption object
     * @param exclude_generic Exclude from generic assumptions
     */
    add_assumption(
        assumption: {
            variable?: string;
            element_of?: string | string[];
            [key: string]: any;
        },
        exclude_generic?: boolean,
    ): void;

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
     * Everything known about a variable, a list of variables or an expression,
     * as a tree stating it — `undefined` when nothing is known.
     *
     * Three query shapes, and the list one is a *nested* array: `"x"` for one
     * variable, `[["x", "y"]]` for several, or an expression. A bare
     * `["x", "y"]` — which this was declared to take — is read as a tree, not
     * as a list of names, and answers `undefined`. The nesting is legacy's own
     * spelling; `slow_assumptions.spec.ts`, which mirrors legacy's suite,
     * queries `[["x"]]`.
     *
     * The answer is a `Tree` (`["<", 0, "x"]`), not an `Assumptions` map.
     *
     * @param variables A name, a `[names]` list, or an expression
     * @param params `exclude_variables`, `omit_derived`
     */
    get_assumptions(
        variables: string | [string[]] | Expression | Tree,
        params?: {
            exclude_variables?: string | string[];
            omit_derived?: boolean;
        },
    ): Tree | undefined;

    /**
     * Clear all assumptions
     */
    clear_assumptions(): void;

    /**
     * Set context to default settings
     */
    set_to_default(): void;

    /**
     * The Expression class itself — for `instanceof`, which is what DoenetML
     * and the spec suite use it for.
     *
     * *Not* an AST constructor, though legacy's was and this was declared as
     * one: this engine's `Expression` wraps a wasm handle, so the first
     * argument is that handle and `new me.class(["+", 1, "x"])` builds an
     * object whose every method fails. Use `me.fromAst`. The constructor is
     * declared as taking `never` so the mistake is a compile error rather than
     * a runtime one; filed as a legacy-parity gap in
     * `MATH_EXPRESSIONS_UPSTREAM_REQUESTS.md`.
     */
    class: new (handle: never, context?: Context) => Expression;

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
    add(expr: Expression | Tree, other: Expression | Tree): Expression;
    subtract(expr: Expression | Tree, other: Expression | Tree): Expression;
    multiply(expr: Expression | Tree, other: Expression | Tree): Expression;
    divide(expr: Expression | Tree, other: Expression | Tree): Expression;
    pow(
        expr: Expression | Tree,
        exponent: Expression | Tree | number,
    ): Expression;
    mod(expr: Expression | Tree, other: Expression | Tree): Expression;
    copy(expr: Expression | Tree): Expression;

    // ========== Simplification methods ==========
    simplify(
        expr: Expression | Tree,
        assumptions?: Assumptions,
        max_digits?: number,
    ): Expression;
    simplify_logical(
        expr: Expression | Tree,
        assumptions?: Assumptions,
    ): Expression;
    collect_like_terms_factors(
        expr: Expression | Tree,
        assumptions?: Assumptions,
        max_digits?: number,
    ): Expression;
    collapse_unary_minus(expr: Expression | Tree): Expression;
    perform_vector_matrix_additions_scalar_multiplications(
        expr: Expression | Tree,
    ): Expression;
    remove_units(expr: Expression | Tree): Expression;
    add_unit(expr: Expression | Tree, unit: Expression | Tree): Expression;
    remove_scaling_units(expr: Expression | Tree): Expression;
    simplify_ratios(
        expr: Expression | Tree,
        assumptions?: Assumptions,
    ): Expression;

    // ========== Differentiation and Integration ==========
    derivative(
        expr: Expression | Tree,
        variable: string,
        story?: string[],
    ): Expression;
    integrateNumerically(
        expr: Expression,
        variable: string,
        lower: number,
        upper: number,
    ): number;

    // ========== Expansion and Transformation ==========
    expand(expr: Expression | Tree, no_division?: boolean): Expression;
    factor(expr: Tree): Expression;
    expand_relations(expr: Expression | Tree): Expression;
    substitute(
        expr: Expression | Tree,
        substitutions: { [variable: string]: Expression | Tree },
    ): Expression;
    substitute_component(
        expr: Expression | Tree,
        component: number | number[],
        value: Expression | Tree,
    ): Expression;
    get_component(
        expr: Expression | Tree,
        component: number | number[],
    ): Expression;
    // ========== Normalization methods ==========
    normalize_function_names(expr: Expression | Tree): Expression;
    normalize_applied_functions(expr: Expression | Tree): Expression;
    normalize_negative_numbers(expr: Expression | Tree): Expression;
    default_order(expr: Expression | Tree): Expression;
    constants_to_floats(expr: Expression | Tree): Expression;
    subscripts_to_strings(expr: Expression | Tree, force?: boolean): Expression;
    strings_to_subscripts(expr: Expression | Tree): Expression;
    tuples_to_vectors(expr: Expression | Tree): Expression;
    to_intervals(expr: Expression | Tree): Expression;
    altvectors_to_vectors(expr: Expression | Tree): Expression;
    // ========== Rounding methods ==========
    round_numbers_to_precision(
        expr: Expression | Tree,
        precision: number,
    ): Expression;
    round_numbers_to_decimals(
        expr: Expression | Tree,
        decimals: number,
    ): Expression;
    round_numbers_to_precision_plus_decimals(
        expr: Expression | Tree,
        precision: number,
        decimals: number,
    ): Expression;

    // ========== Number evaluation ==========
    evaluate_numbers(
        expr: Expression | Tree,
        options?: {
            max_digits?: number;
            skip_ordering?: boolean;
            evaluate_functions?: boolean;
            set_small_zero?: number | boolean;
            assumptions?: Assumptions;
        },
    ): Expression;
    set_small_zero(expr: Expression | Tree, tolerance?: number): Expression;

    // ========== Solve and Rational methods ==========
    solve_linear(expr: Expression | Tree, variable: string): Expression;
    reduce_rational(expr: Expression | Tree): Expression;
    // ========== Matrix operations ==========
    matrix(entries: Expression[][]): Expression;
    vector_add(expr: Expression | Tree, other: Expression | Tree): Expression;
    vector_sub(expr: Expression | Tree, other: Expression | Tree): Expression;
    scalar_mul(
        scalar: number | Expression | Tree,
        vector: Expression | Tree,
    ): Expression;
    dot_prod(expr: Expression | Tree, other: Expression | Tree): Expression;
    cross_prod(expr: Expression | Tree, other: Expression | Tree): Expression;

    // ========== Sets operations ==========
    /**
     * A periodic solution set such as `π/4 + nπ`, as the union of one
     * arithmetic progression per offset.
     *
     * `undefined` — legacy's failure value, so declared — when an operand is
     * missing or the offset and period lists disagree in length.
     */
    create_discrete_infinite_set(params?: {
        offsets?: Expression | Tree;
        periods?: Expression | Tree;
        min_index?: Expression | Tree;
        max_index?: Expression | Tree;
    }): Expression | undefined;

    // ========== Inspection methods ==========
    variables(expr: Expression | Tree, include_subscripts?: boolean): string[];
    functions(expr: Expression | Tree): string[];
    equals(
        expr: Expression,
        other: Expression,
        options?: EqualsOptions,
    ): boolean;
    equalsViaSyntax(
        expr: Expression,
        other: Expression,
        options?: EqualsOptions,
    ): boolean;
    equalsViaComplex(
        expr: Expression,
        other: Expression,
        options?: EqualsOptions,
    ): boolean;
    equalsViaReal(
        expr: Expression,
        other: Expression,
        options?: EqualsOptions,
    ): boolean;
    f(expr: Expression | Tree): (bindings: Bindings) => number | Complex;
    evaluate(expr: Expression | Tree, bindings: NumericBindings): number;
    finite_field_evaluate(
        expr: Expression,
        bindings: Bindings,
        modulus: number,
    ): number;
    /** See {@link Expression.evaluate_to_constant} — `NaN` when there is no
     * numeric value, never `null`. */
    evaluate_to_constant(
        expr: Expression | Tree,
        options?: EvaluateToConstantOptions,
    ): number | Complex;
    isAnalytic(
        expr: Expression | Tree,
        options?: IsAnalyticOptions | string[],
    ): boolean;
    /** Returns `false`, not `null`, when the pattern does not match. */
    match(
        expr: Expression | Tree,
        pattern: Expression | Tree,
        options?: MatchOptions,
    ): MatchResult | false;
    // ========== Formatting methods ==========
    // No expression-first `toString`. `Context` inherits `Object.prototype`'s,
    // and the expression-first mirror deliberately does not shadow it — doing
    // so would break `String(me)` and every template literal. `me.toString(e)`
    // therefore answers `"[object Object]"`, so it is not declared; write
    // `e.toString()`.
    toLatex(expr: Expression | Tree, params?: FormatParams): string;
    tex(expr: Expression | Tree, params?: FormatParams): string;
    toXML(expr: Expression | Tree): string;
    toGLSL(expr: Expression | Tree): string;
    // The expression-first mirror of the 48 members this engine does not
    // implement is gone with them; see the note at the end of `Expression`.
}

// ---------------------------------------------------------------------------
// v3 additions
//
// Everything above is the legacy contract. Below is what this package adds on
// top of it: the wasm injection point, the ODE integrator that used to come
// from the bundled math.js, and the default export.
// ---------------------------------------------------------------------------

/**
 * A scalar state, or a vector one. `dopri` mirrors whichever shape `y0` had:
 * a `number` initial condition gives `number` states back, an array gives
 * arrays.
 */
export type OdeState = number | number[];

/** A computed trajectory with dense output — what {@link dopri} returns. */
export interface OdeSolution {
    /** Interpolated state at `x`, or one state per element of an `x` array. */
    at(x: number): OdeState;
    at(x: number[]): OdeState[];
    /** Accepted step abscissas. */
    readonly x: number[];
    /** States at each step abscissa. */
    readonly y: OdeState[];
    /** True when integration stopped before `x1` (blow-up or step budget). */
    readonly terminatedEarly: boolean;
    /**
     * Release the underlying wasm handle. Idempotent. numeric.js had nothing
     * to release, so this is additive — a caller that never frees behaves as
     * it did before, but a long-lived host integrating in a loop leaks one
     * handle per call.
     */
    free(): void;
    /** Alias of {@link free}, and the `using`-statement protocol. */
    dispose(): void;
}

/**
 * The Dormand-Prince integrator, a `numeric.dopri` drop-in. DoenetML reached
 * numeric.js's copy through the old bundled math.js (`me.math.dopri`); this
 * package supplies its own `solve_ode`-backed equivalent, exported both as a
 * named export and as `me.dopri`. Neither supports numeric.js's `event`
 * argument, so it is absent here.
 */
export function dopri(
    x0: number,
    x1: number,
    y0: OdeState,
    f: (x: number, y: OdeState) => OdeState,
    tol?: number,
    maxit?: number,
): OdeSolution;

/**
 * Supply the wasm module this package runs on.
 *
 * Node needs no call: the vendored *nodejs-target* build under `vendor/wasm` is
 * loaded on first use. A browser or Web Worker host cannot use that build and
 * must inject instead — instantiate the `--target web` build this package also
 * ships (`math-expressions/wasm-web/math_expressions_wasm.js`, alongside its
 * `math_expressions_wasm_bg.wasm`), then pass the initialized module here.
 *
 * Call it **before anything parses**. The module is read lazily, so an
 * injection made at import time always wins; one made after the first parse
 * swaps the module and invalidates every wasm handle minted by the old one.
 *
 * The parameter is typed structurally rather than as the generated
 * wasm-bindgen module, because that module is produced by the build rather than
 * shipped as a type.
 */
export function setWasmModule(mod: Record<string, unknown>): void;

/**
 * The default context — the `me` of `import me from "math-expressions"`.
 */
declare const MathExpression: Context;
export default MathExpression;
