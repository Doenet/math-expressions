# JS ↔ Rust: API & Test Differences

Exhaustive inventory of where the JavaScript library (`lib/`, tests in `spec/`)
and the Rust port (`packages/math-expressions-rs/`, tests in
`packages/math-expressions-rs/tests/`) differ — public API surface and test
coverage. Intended as the checklist to work through **before deleting the JS
code**. No implementation details, just what is different.

Companion to [WHATS_LEFT.md](WHATS_LEFT.md) (feature-gap tracker) and to
[JS_RUST_TEST_DIVERGENCES.md](JS_RUST_TEST_DIVERGENCES.md) — the empirical,
case-by-case enumeration of every JS test case whose Rust output differs. This
document is the API-level diff; that one is the measured output diff.

> **Layout note.** The Rust crate lives at `packages/math-expressions-rs/`. Any
> `math-expressions-rs/...` (no `packages/` prefix) path — including in the
> session's initial git status — is a stale/relative artifact; there is no such
> real directory (`git ls-files math-expressions-rs` is empty).

> **Where the JS "methods" come from.** `lib/math-expressions.js` only defines
> the `Context` factory (`:148-214`) and the `Expression` object (`:36-57`).
> Every math method (`equals`, `derivative`, `simplify`, …) is mixed onto
> `Expression.prototype` (`:218-224`) from the module arrays in
> `lib/expression/index.js` and `lib/functions/index.js`. The Rust surface is the
> crate re-exports in `src/lib.rs:43-81` and the `#[wasm_bindgen]` items in
> `src/wasm.rs`.

---

## 0. TL;DR — the big-ticket differences

**JS has, Rust does not (real capability gaps):**
- Converters: **ast↔mathjs, mml→\*, ast→guppy, ast→GLSL, ast→finite-field**, and
  all the mathjs/guppy/mml cross-converters (§2).
- ~~The entire **± ("pm") subsystem**~~ — ported: `src/pm.rs` primitives,
  `eq::pm_equals`, and the pm-aware `simplify`/`expand` rules in `norm` (§3).
- **Symbolic polynomial algebra + Groebner bases** (`lib/polynomial/`, 1847 lines)
  — no public Rust polynomial API at all (§4).
- **Derivative "story"** step-by-step narration (§4).
- **No elementary-function builder methods** (`sin`, `sqrt`, `exp`, …) on the
  Rust public surface (§1).
- Configurable **emitter options** (number padding, `showBlanks`, matrix env,
  scientific-notation avoidance) — Rust emitters are fixed-behavior (§2).
- Auto-detecting `from()`, `fromMml`, JSON `reviver`, and several parser options
  (`units`, latex `operatorSymbols`) (§1, §2).

**Rust has, JS does not:**
- Symbolic **integration**, arbitrary-precision/verified **integrate & evaluate**
  (`precise`), **ODE** solving, broad **matrix algebra** (det / inverse / rref /
  rank / nullspace / char-poly / eigenvalues / eigenvectors), **factor**,
  **rootof**/algebraic numbers, **exact** rational arithmetic (§4).
- The whole **differential-corpus test machinery** — pass-rate assertions +
  snapshotted `*-known-failures.json` / `*-known-divergences.json` using
  JS/mathjs as the oracle. JS has no corpus/snapshot tests at all (§5).

**Naming rule:** JS mixes camelCase + snake_case with many aliases; the WASM
layer normalizes to a single **snake_case** name each. Only two names are
preserved via `js_name`: `mod` (`wasm.rs:205`) and the `Assumptions` class
(`wasm.rs:454`).

---

## 1. Top-level public API

### 1.1 JS public items with NO Rust/WASM equivalent

**Factory / Context (`lib/math-expressions.js`):**
- `from(expr, pars)` — auto-detect text→latex→mml. No Rust auto-detect entry.
- `fromMml` / `parseMml` — **no MathML parser in Rust at all**.
- `reviver(key, value)` — JSON.parse reviver callback. (`from_serialized` exists
  but is not a reviver.)
- `isTree(value)` type guard (`:18`) — no equivalent.
- Context properties: `parser_parameters`, `math` (the bundled mathjs instance),
  `converters`, `utils`, `ZmodN`, `class` — none exposed by WASM.
- Assumption mutators on Context: `set_to_default`, `get_assumptions`,
  `add_assumption(…, exclude_generic)`, `add_generic_assumption`,
  `remove_assumption`, `remove_generic_assumption`, `clear_assumptions`. The WASM
  `Assumptions` class covers add/remove/clear but has **no generic-assumption
  concept**, no `exclude_generic`, no `get_assumptions`, no `set_to_default`.

**`Expression.prototype` methods with no WASM export:**
- Elementary/transcendental **function builders** — `abs`, `exp`, `log`, `log10`,
  `sign`, `sqrt`, `conj`, `im`, `re`, `factorial`, `gamma`, `erf`, all
  trig/inverse/hyperbolic, `atan2` (`lib/functions/standard.js:9-125`). Entirely
  absent from the Rust public surface as callable builders.
- Output: `toXML` (Guppy/MathML), `toGLSL` — no analog. `tex` alias.
- `derivative_with_story`, `derivative_story`, `derivativeStory` — WASM has plain
  `derivative` only.
- `expand_relations`.
- `clean`, `collapse_unary_minus`, `default_order`,
  `perform_vector_matrix_additions_scalar_multiplications`,
  `perform_vector_scalar_multiplications`,
  `perform_matrix_scalar_multiplications`, `perform_matrix_multiplications`.
- Normalization passes not individually callable:
  `normalize_applied_functions`, `log_subscript_to_two_arg_log`,
  `substitute_abs`, `normalize_angle_linesegment_arg_order`,
  `normalize_negative_numbers`.
- `matrix` (constructor), `scalar_mul` (matrix.js).
- `integrateNumerically` — Rust integration is symbolic/precise, different
  contract and name.
- `f()` (returns a JS-callable evaluator) — no equivalent.
- equality variants: `equalsViaComplex`, `equalsViaSyntax`,
  `equalsViaFiniteField` (boolean), `equalsDiscreteInfinite` (functionality
  folded into `equals`, but not standalone entry points).

**In the Rust crate but NOT exposed via WASM** (available to Rust callers only):
`operators`, `solve_linear`, `equals_syntactic`, `equal_with_sign_errors`,
`equal_specified_sign_errors`, `substitute` (general/multi-var form).

### 1.2 Rust/WASM public items with NO JS equivalent

- `is_zero()` → three-valued `Option<bool>`.
- `evaluate_to_complex()` (JS `evaluate_to_constant` returns a single value).
- Whole **precise layer**: `evaluate_to_precision`, `integrate_to_precision`,
  `integrate_analyzed`, types `Precise` / `IntegralVerdict` / `SingularPoint`.
- `factor()`, crate `factor` / `factor_terms`.
- `together()` (closest JS concept is the renamed `common_denominator`).
- **Matrix algebra**: `determinant`/`det`, `char_poly`, `eigenvalues`,
  `eigenvectors`, `matmul`, `matrix_inverse`, `rref`, `rank`, `nullspace`,
  `trace`, `transpose`, `EigenPair`.
- **ODE**: free fns `solve_ode`, `solve_ode_expressions`; crate `solve_ode_exprs`,
  `solve_ode_with`; `OdeSolution` class (`at` / `at_many` / `dim` / `last_t` /
  `last_y` / `terminated_early` / `times`).
- Stats/linalg free functions replacing bundled mathjs: `gcd`, `lcm`, `mean`,
  `median`, `variance`, `std`, `quantile_seq`, `lusolve`, `eigs`, `math_mod`.
- AST utility free functions: `from_ast`, `from_serialized`, `to_serialized`,
  `tree_json`, `match_template`, `flatten_ast`, `unflatten_left`,
  `unflatten_right` (JS exposes these under `Context.utils` / `Context.fromAst` /
  `toJSON`, not standalone).
- `parse_text_with_options` / `parse_latex_with_options` + typed
  `TextToAstOptions` / `LatexToAstOptions`.
- `simplify_with_assumptions(Vec<String>)` per-call assumptions form.
- Crate-only, no JS analog: `canonicalize`, `desugar_units`, `simplify_with`,
  `evaluate_membership`, `grade` module, `AnalyticOpts`, `Sym`, `Number`,
  `MathConst`, `RelOp`, `contains_blank` (public), `match_discrete_infinite`.

### 1.3 Present in both, but different name / signature / shape

| Concept | JS | Rust / WASM | Difference |
|---|---|---|---|
| LaTeX output | `tex(params)` / `toLatex(params)` | `to_latex()` | rename; **JS takes a params object, WASM takes none** |
| Text output | `toString(params)` | `to_text()` | rename; JS takes params, WASM none |
| Serialize | `toJSON()` → object | `to_serialized()` / `tree_json()` → **string** | rename + JSON string not object |
| equals | `equals(other, {…9 opts})` | `equals(other)` + `equals_with_options(other, json)` | opts move to a separate method taking a JSON string; wasm keys are camelCase |
| derivative | `derivative(x)` (symbol/expr) | `derivative(var: &str)` | WASM takes a **string var name**, not an expression |
| evaluate | `evaluate(bindingsObj)` | `evaluate(vars[], values[])` → `Option<f64>` | object → **two parallel arrays**; real-f64 only (+ separate `evaluate_to_complex`) |
| substitute | `substitute(bindingsObj)` | `substitute_var(var, value)` | multi-var object → single var/value pair (crate `substitute` is general) |
| finite_field_evaluate | method `(bindings, modulus)` | **free function** (`wasm.rs:433`) | method → free function, different arg shape |
| variables | `variables(include_subscripts=false)` | `variables()` | WASM drops the `include_subscripts` flag |
| isAnalytic | `isAnalytic({allow_abs,allow_arg,allow_relation})` | `is_analytic(bool, bool, bool)` | opts object → three explicit bools |
| set_small_zero | `set_small_zero(paramsObj)` | `set_small_zero(tolerance: f64)` | explicit single f64 |
| remove_units | `remove_units(opts)` | `remove_units(scale_based_on_unit: bool)` | explicit bool |
| round precision | `round_numbers_to_precision(digits=14)` | `round_numbers_to_precision(sig_figs: i32)` | JS has a default; WASM requires the arg |
| reduce_rational pair | `reduce_rational` + `common_denominator` | `reduce_rational` + `together` | `common_denominator` → `together` rename |
| mod | `expr.mod(other)` | `modulo` exported as js_name `mod` | same JS name, different Rust identity |
| discrete infinite set | method `create_discrete_infinite_set(...)` | free fn `discrete_infinite_set(offsets, periods)` | method → free function |
| assumptions add | `add_assumption(a, exclude_generic)` on Context | `Assumptions.add(relation)->bool` | stateful class instead of Context; drops `exclude_generic`; returns bool |
| assumption predicates | bundled `get_assumptions(...)` | 8 individual `is_*(expr)->Option<bool>` | split into 8 three-valued predicates |
| integrate | `integrateNumerically()` (float) | `integrate(var)` / `integrate_to_precision` / `integrate_analyzed` | numeric-only → symbolic + precision variants |
| parse entry | `fromText`/`parse` + 4 latex aliases + `fromMml` + auto `from` | `parse_text`, `parse_latex` (+ `_with_options`) | alias fan-out collapsed to two; MML + auto-detect dropped |

---

## 2. Converters / parsers

JS exports a near-full matrix between AST ↔ {latex, text, mathjs, mml, guppy,
finite-field, GLSL} from `lib/converters/index.js`. Rust implements **only the
core four**.

| Direction | JS | Rust |
|---|---|---|
| text → ast | ✅ | ✅ (`parse::text`, wasm `parse_text`) |
| latex → ast | ✅ | ✅ (`parse::latex`, wasm `parse_latex`) |
| ast → text | ✅ | ✅ (`output::to_text`, wasm `to_text`) |
| ast → latex | ✅ | ✅ (`output::to_latex`, wasm `to_latex`) |
| ast → mathjs | ✅ | ❌ |
| mathjs → ast | ✅ | ❌ |
| mml → ast / latex / text / mathjs / guppy | ✅ | ❌ (no MathML anywhere) |
| ast → guppy (+ latex/text/mathjs → guppy) | ✅ | ❌ |
| ast → finite-field | ✅ | ❌ (an `eq/finite_field.rs` exists for *equality*, not a converter) |
| ast → GLSL | ✅ | ❌ |
| text ↔ latex, mathjs → latex/text | ✅ | ❌ as named converters (text↔latex trivially composable from the two Rust primitives) |

Grep for `guppy|mathjs|mml|sage|glsl|finite.field` in Rust `src/` hits **only
comments**. (There is no `sage` converter on either side.)

### 2.1 Parser option differences

| Option | JS text | JS latex | Rust |
|---|---|---|---|
| `allowSimplifiedFunctionApplication` | ✅ | ✅ | ✅ |
| `splitSymbols` / `unsplitSymbols` | ✅ | — | ✅ (text) |
| `appliedFunctionSymbols` / `functionSymbols` | ✅ | ✅ | ✅ |
| `operatorSymbols` | ✅ | ✅ | ✅ text; **❌ latex** (hardcoded) |
| `allowedLatexSymbols` | — | ✅ | ✅ |
| `units` | ✅ | ✅ | **❌ both** (hardcoded) |
| `parseLeibnizNotation` / `parseScientificNotation` | ✅ | ✅ | ✅ |

Rust adds a `MAX_PARSE_DEPTH = 64` recursion guard (no JS counterpart).

### 2.2 Emitter option differences (Rust emitters are largely fixed-behavior)

| JS `astToLatex` option | Rust `LatexOpts` |
|---|---|
| `padToDigits`, `padToDecimals` | ❌ |
| `avoidScientificNotation` | ❌ |
| `showBlanks` (default true) | ❌ |
| `matrixEnvironment` | ❌ |
| `convertLatexSymbols` / `allowedLatexSymbols` | ❌ |

`LatexOpts` is an **empty struct**.

| JS `astToText` option | Rust `TextOpts` |
|---|---|
| `output_unicode` | ✅ `unicode` |
| `padToDigits`, `padToDecimals` | ❌ |
| `avoidScientificNotation` | ❌ |
| `showBlanks` | ❌ |
| `explicitMultiplicationSymbols` | ❌ |

Also: JS `ast→mathjs` deliberately **throws** on unsupported input (booleans,
malformed AST, non-integer matrix dims) and asserts it — no Rust analog.

---

## 3. Expression operations (equality / normalization / simplify / assumptions / pm)

### 3.1 Missing in Rust (genuine capability gaps)

- **pm/± subsystem — ported.** `contains_pm`, `count_pm`, `expand_pm_signs`
  (`lib/expression/pm.js`) → `src/pm.rs` (crate + wasm, with the same
  `MAX_PM_COUNT = 10` cap, returned as an error rather than thrown);
  `pm_equals_numerical` (`equality/pm-numerical.js`) → `eq::pm_equals`, wired into
  `equals` (equations compared proportionally via branch-products, inequalities/
  expressions via value-multiset bipartite matching with per-variant tolerance);
  the pm-aware `simplify` canonicalization rules (reorder, scaling `2·±x → ±(2x)`,
  `−(±x) → ±x`, and the guard against combining independent ± like-terms) and the
  `expand` guard against duplicating a ± are in `norm`.
- `expand_relations` — no named Rust op.
- `equalsViaFiniteField` as a standalone **boolean** equality (Rust only has the
  internal `definitely_unequal` + the `finite_field_evaluate` value helper).

### 3.2 Present in Rust but folded (behavior reachable, not individually callable)

Folded into `canonicalize` / `normalize_syntactic` / `simplify` and not exposed
as separate calls: `clean`, `collapse_unary_minus`, `default_order`,
`normalize_applied_functions`, `normalize_negative_numbers`,
`normalize_angle_linesegment_arg_order`, `log_subscript_to_two_arg_log`,
`substitute_abs`, `perform_vector_matrix_additions_scalar_multiplications`,
`perform_vector_scalar_multiplications`,
`perform_matrix_scalar_multiplications`. And the equality variants
`equalsViaComplex` / `equalsViaSyntax` / `equalsDiscreteInfinite` are folded into
`equals` (only `equals_via_real` and `equals_syntactic` are separate).

### 3.3 Fully ported

All **8 assumption predicates** (`is_integer/real/complex/nonzero/nonnegative/
positive/negative/nonpositive`) and the add/remove/**generic**/get assumption API
map cleanly to the Rust `Assumptions` class. (JS's `_ast` predicate variants are
an internal tree-vs-Expression split Rust doesn't need.)

### 3.4 Behavioral differences

- **`EqOptions`** has the same nine JS options with identical defaults, **plus
  two Rust-only fields**: `num_samples` (JS hardcodes internally) and `real_only`
  (JS exposes this as the separate `equalsViaReal`, not an option). WASM option
  keys are camelCase.
- **Assumptions are an explicit parameter in Rust** (`simplify_with` /
  `simplify_with_assumptions`), whereas JS reads them from the expression's
  `context`. Notably, **Rust `equals` has no assumptions parameter** while JS
  `.equals` can see context assumptions (tested via "integer assumption").
- pm equality's per-variant tolerance and order-independent (bipartite) variant
  matching are ported in `eq::pm_equals`.

---

## 4. Mathematical functions

### 4.1 Rust-only (no JS equivalent)

- **Symbolic integration** — `integrate`, rational integration (`src/integrate/`,
  wasm `integrate`). JS has only the crude 100-interval midpoint
  `integrateNumerically`.
- **Precise / verified** integrate & evaluate — `integrate_to_precision`,
  `integrate_analyzed`, `evaluate_to_precision`, `Precise`, `IntegralVerdict`,
  `SingularPoint`.
- **ODE** solving — `src/ode.rs`, `solve_ode*`, `OdeSolution`.
- **Full matrix algebra** — det, inverse, rref, rank, nullspace, matmul,
  transpose, trace, char_poly, **eigenvalues/eigenvectors** (`src/matrix.rs`).
  JS `matrix.js` has only a constructor + vector ops.
- **Factoring** — `factor`, `factor_terms` (`src/factor.rs`). JS has no `factor`.
- **Rational canonical form** (`ratform`), **rootof / algebraic numbers**
  (`rootof`), **exact rational arithmetic** (`exact.rs`, `Number`).
- Numeric stats exposed from wasm — `gcd`, `lcm`, `mean`, `median`, `variance`,
  `std`, `quantile_seq`, `lusolve`, `eigs`, `math_mod` (JS uses mathjs directly).

### 4.2 JS-only (missing from Rust)

- **Full symbolic polynomial algebra + Groebner bases** — the largest JS-only
  area. `lib/polynomial/polynomial.js` (1847 lines): `expression_to_polynomial`,
  `polynomial_add/neg/sub/mul/pow`, `poly_div`, `poly_gcd`, `poly_lcm`, `reduce`,
  `reduced_grobner`, monomial ops, plus a parallel `pt_`-prefixed term API; and
  `single-var-poly.js` (`sv_*`). Rust has **no Groebner and no public polynomial
  API** — `src/poly/` is internal-only, backing factor/ratform/reduce_rational.
- **Derivative "story"** — `derivative_story` / `derivativeStory` /
  `derivative_with_story` (step-by-step LaTeX narration). Rust `derivative`
  returns only the result.
- **`integrateNumerically`** — the fixed-100-interval midpoint routine (Rust's
  integration is a different, symbolic/precise contract).
- `common_denominator` (Rust's analog is the renamed `together`).
- Unit introspection accessors `get_all_units` / `get_unit_of_tree` /
  `get_unit_value_of_tree` (Rust has desugar/remove/add but no readers).

### 4.3 Behavioral differences

- **Numeric integration contract** is fundamentally different (JS float midpoint
  → returns a number; Rust symbolic → returns an expression, or precision-
  parameterized verified value).
- **Rounding**: same three variants; JS defaults digits/decimals to 14, Rust wasm
  takes explicit ints. JS asserts edge behaviors (don't round fractions / π / e /
  fallback) that have **no dedicated Rust test**.
- `scalar_mul` is a standalone JS op; Rust folds scalar-multiply into vector ops.
- Matrix construction validation: JS `matrix()` throws on ragged rows; Rust
  builds via the parser (different error path).
- JS exposes polynomial **tree encodings** publicly (`polynomial_terms`,
  `sv_poly`, `pt_*`); Rust exposes none.

---

## 5. Test coverage

### 5.1 JS spec areas with NO Rust equivalent

| JS spec | ~cases | Area | Status in Rust |
|---|---|---|---|
| `quick_ast-to-mathjs.spec.js` | ~89 | ast → mathjs (incl. throw-cases) | none |
| `quick_mathjs-to-ast.spec.js` | ~24 | mathjs → ast | none |
| `quick_ast-to-guppy.spec.js` | ~4 | ast → guppy | none |
| `quick_mml-to-latex.spec.js` | 1 | mml → latex | none |
| `quick_pm.spec.js` | 58 | ± helpers, pm equality, pm × simplify/expand/tuples | ported: ± helpers, `.equals`, and pm × `simplify`/`expand` (`tests/pm.rs`) |
| `slow_polynomial.spec.js` | ~23 / 1807 lines | Groebner, poly div/gcd/lcm, monomial ordering, text→poly | **none** (no poly API); only reduce-rational overlaps |
| `quick_transformation.spec.js` — `expand_relations` cases | few | expand of relations | none (op missing) |
| `quick_rounding.spec.js` edge behaviors | 13 | don't-round fractions/π/e, fallback | no dedicated Rust rounding test |
| `slow_check-equality-numerical-errors` / `-symbolic-` | 26 blocks each | tolerance-option matrices on `.equals` | collapsed to one ~12-assert `tests/equality.rs:317` |
| `quick_ast-to-latex` / `quick_ast-to-text` direct-output asserts | 265 / 247 | golden ast→latex/text outputs | **fixtures generated but not wired to any test** — see 5.3 |
| `quick_latex-to-ast-to-latex` / `quick_text-to-ast-to-text` `showBlanks`-off variants | — | round-trip without blanks | none (`LatexOpts` empty) |
| per-pass normalization asserts (`quick_normalization`) | 62 | individual passes (`default_order`, subscripts, negative-number, applied-fn) | Rust tests `canonicalize` holistically (`tests/norm.rs`, 12), not per-pass |
| `quick_trees.spec.js` | 39 | flatten/unflatten/match tree utils | utils exist (`src/js_match.rs`) but **no dedicated test file** |

### 5.2 Rust tests with NO JS equivalent

- **Differential corpora** — Rust output checked mathematically equal to the
  JS/mathjs reference, with **pass-rate assertions** and snapshotted accepted
  divergences (`*-known-failures.json` / `*-known-divergences.json`). JS has no
  corpus/snapshot machinery whatsoever:
  - `derivative_corpus` (300), `evaluate_corpus` (250), `expand_corpus` (247),
    `ops_corpus` (200), `simplify_corpus` (342), `equality_corpus` (824),
    `assumptions_corpus` (546), `numeric_corpus`, `ode_corpus` (10).
- **Whole subsystems with no JS side**: `integrate` (14), `quadrature` (10),
  `quadrature_poles` (3), `ode` (11), `eigen` (27), `factor_s4` (8),
  `ratform` (10), `rootof_adversarial` (6), `precise_rootof` (7), `precise` (17),
  `precise_digits` (16), `divergence` (17, integral poles).
- **Parser/emitter robustness**: `parser_options.rs` (14 option tests, hand-
  ported since the JS option tests live inline in the specs), `display.rs` (9,
  canonical presentation ordering), and the `*-edge` oracle fixtures generated by
  running the JS parser as an oracle.
- `sets.rs` partial-match **scoring** (`partial(a,b) -> f64`) — Rust-visible
  behavior with no in-scope JS analog.

### 5.3 Test-wiring gaps to close before deleting JS

- **Orphaned fixtures**: `scripts/extract-fixtures.mjs` generates
  `fixtures/ast-to-latex.json` (265) and `fixtures/ast-to-text.json` (247) from
  the JS specs, but **no Rust test `include_str!`s them**. ast→latex / ast→text
  are validated only *indirectly* (via `tests/roundtrip.rs` and `tests/display.rs`),
  never against the JS golden outputs. Wire these up before trusting the emitters.
- No `showBlanks`-off round-trip coverage (Rust emitters lack the flag).
- No dedicated rounding-edge-case test file.

---

## 6. Deletion-readiness summary

**Ported — safe to delete once test wiring is closed (§5.3):**
text↔ast, latex↔ast, ast→text, ast→latex; equality (core + syntactic + via-real);
normalization/simplify/expand (behavior, even if sub-ops are folded); assumptions
(full, incl. generic); arithmetic; evaluate/evaluate_to_constant;
finite_field_evaluate; rounding; reduce_rational; solve_linear; discrete-infinite
sets; isAnalytic; sign-error grading; elementary-function *registry* (parsing/
eval, though not exposed as builder methods).

**NOT yet portable — no Rust code exists; deleting JS loses the capability:**
- Converters: ast↔mathjs, all mml→\*, ast→guppy (+ latex/text/mathjs→guppy),
  ast→GLSL, ast→finite-field, mathjs→latex/text.
- **Symbolic polynomial algebra + Groebner bases**.
- Derivative **story** narration.
- Configurable **emitter options** (padding, `showBlanks`, matrix env,
  scientific-notation avoidance) and parser `units` / latex `operatorSymbols`.
- `fromMml`, auto-detecting `from()`, `reviver`, `isTree`, `f()` callable,
  elementary-function **builder methods**, `toXML`/`toGLSL`.

Cross-check each "not yet portable" item against actual Doenet usage —
[WHATS_LEFT.md](WHATS_LEFT.md) already marks several (guppy, GLSL, MathML output,
mathjs input) as **not needed for Doenet**, which may make them deletable anyway.
