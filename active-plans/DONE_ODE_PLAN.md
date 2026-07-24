# ODE Numerics Plan

Plan for replacing the ODE-solving capability Doenet currently obtains
through math-expressions *by accident*: the JS library's `me.math` is a
customized mathjs instance that does `math.import(numeric, { wrap: true })`
(`lib/mathjs.js`), which smuggles the abandoned `numeric` package's `dopri`
(Dormand–Prince RK45) onto the exported object. Doenet's `ODESystem`
component destructures `const { dopri } = me.math` and uses it to integrate
user-authored dynamical systems
(`packages/doenetml-worker-javascript/src/components/dynamicalSystems/ODESystem.js`).

Companion documents: `active-plans/DONE_ARBITRARY_PERCISION_PLAN.md` (the compiled
evaluation tape is the natural integrand/vector-field evaluator),
`active-plans/INTEGRATION_PLAN.md` (quadrature = the one-dimensional special case).
Kept separate per the maintainer's instruction: ODE solving is a numerics
capability with its own error-control design, not a CAS feature.

**Status: ✓ fully implemented 2026-07-19** (`src/ode.rs`; `tests/ode.rs`,
11 tests; `tests/ode_corpus.rs` differential vs the vendored JS
`numeric.dopri` on 10 systems incl. pendulum, van der Pol, Lotka–Volterra —
agreement ≤ 1e-4·scale at 18 abscissae each; wasm smoke 55/55). All three
phases landed together:

- **O1**: DP5(4) with FSAL, Hairer's PI controller (β = 0.04) and the
  dopri5.f dense-output rcont interpolant; guards per §2 — `max_ode_steps`
  (new limit, 10 000), vanishing-step and non-finite/eval-failure detection
  all truncate to the last accepted step with `terminated_early` set (the
  y′ = y² blow-up test stops just short of t = 1 with every dense sample
  finite). Backward integration (t1 < t0) supported.
- **O2 + O3** (merged): `solve_ode_exprs` per §5c — canonicalizes the RHS,
  rejects unknown free variables, and compiles each RHS to the
  ARBITRARY_PERCISION_PLAN evaluation tape with per-tape variable-slot maps
  (no per-step allocation); expressions the tape can't take (unknown
  functions) fall back to `eval_complex` per stage. Domain failures mid-
  trajectory (y′ = ln y crossing y = 0) terminate cleanly.
- **wasm**: `OdeSolution` class with `at(t)` (always length-n, §5a),
  `at_many` (flattened batch for plotting), `last_t`/`last_y` (§5b chunk
  chaining — verified by the chaining test and smoke row),
  `terminated_early()`, `times()`; constructors `solve_ode(jsCallback, …)`
  (drop-in for `numeric.dopri`) and `solve_ode_expressions(rhsTuple,
  indVar, stateVars, …)` (in-wasm evaluation, the migration target).

Original status note (kept for history): planned, deliberately not yet
implemented. The f64 numeric
module (`src/numeric.rs`, 2026-07-19) covers the rest of Doenet's `me.math`
usage (`mod`/stats/`lusolve`/`eigs`) but intentionally excludes ODE code.

## 1. What Doenet actually needs (the `dopri` contract)

`numeric.dopri(t0, t1, y0, f, tol, maxit, event?)` returns a solution object
whose `at(t)` interpolates the trajectory (dense output). ODESystem calls it
per render window and samples `at` for plotting/animation. Requirements
extracted from the component:

- first-order systems `y′ = f(t, y)`, `y ∈ ℝⁿ` (n small — typically 1–4;
  the right-hand sides are user-authored math-expressions, evaluated
  numerically per step);
- adaptive step with tolerance (defaults comparable to `numeric`'s 1e-6);
- **dense output** (`at(t)`) — not just endpoint values;
- graceful failure on blow-up (Doenet displays a warning, not a crash).

## 2. Design

New module `src/ode.rs` (f64-only, like `src/numeric.rs`), wasm-exposed.

- **Method**: Dormand–Prince RK45 (the same tableau as `numeric.dopri` and
  scipy's `RK45`) with the standard PI step-size controller and the free
  4th-order dense-output interpolant (the DP "b*" polynomial — gives `at(t)`
  without extra function evaluations).
- **Vector field evaluation**: two constructors —
  1. `solve_ode(f: js_callback, …)` taking a JS closure across the wasm
     boundary (drop-in for Doenet's current usage; one boundary call per
     RK stage);
  2. `solve_ode_exprs(rhs: Vec<Expression>, vars, …)` taking the right-hand
     sides as parsed expressions and evaluating them **inside** wasm via the
     existing `eval` machinery (later: the ARBITRARY_PERCISION_PLAN compiled
     tape, which removes per-step allocation) — faster (no boundary
     crossings) and the API Doenet should migrate to, since ODESystem
     already holds the RHS as math-expressions.
- **Result object**: `OdeSolution` (wasm class) storing accepted step points
  + dense coefficients; `at(t)` evaluates the interpolant, `Float64Array`
  batch variant for plotting.
- **Guards** (§7f philosophy, operation counts): `max_ode_steps` (default
  10 000), `min_step_fraction` (reject vanishing steps), non-finite state →
  truncate the solution at the last good step and mark
  `solution.terminated_early` (Doenet's warning path). No wall-clock.

## 3. Testing

1. Analytic cases: `y′ = y` (exp), `y′ = −y`, harmonic oscillator, logistic
   — endpoint + dense-output error ≤ 10·tol across the interval.
2. JS-oracle corpus: `scripts/generate-ode-corpus.mjs` sampling
   `numeric.dopri` solutions (via the vendored JS library) at fixed
   abscissae; agreement to combined tolerance (both are approximations —
   compare against the analytic solution where available, else mutual
   agreement at ~1e-4).
3. Stiff-ish blow-up guard: `y′ = y²`, `y(0)=1` on [0, 2] — must terminate
   early cleanly at the singularity (t = 1), not hang or emit NaN points.

## 4. Effort & phasing

| Phase | Deliverable | Estimate |
|---|---|---|
| O1 | RK45 core + PI controller + dense output + guards, callback constructor, wasm `OdeSolution` | ~2 days |
| O2 | expression-RHS constructor (in-wasm evaluation), ODE corpus, smoke rows | ~1 day |
| O3 (with precision plan P1) | tape-compiled RHS evaluation | ~½ day |

## 5. API details from ODESystem analysis (2026-07-19)

Reviewing `ODESystem.js` revealed several specifics not captured above.

### 5a. `at(t)` return type — uniform array for all dimensions

`numeric.dopri`'s `result.at(t)` returns a flat array `[y₁(t), y₂(t), …]`
for all n (including n=1). ODESystem stores `result.at.bind(result)` and then
indexes into the result: `calculatedNumericSolutions[chunk](t)[ind]`. The Rust
`OdeSolution.at(t)` must follow the same convention: always return a
`Float64Array` of length n, even for scalar (n=1) systems. A separate
scalar-unwrapping helper is **not** needed and would break the indexing pattern.

### 5b. Last-state accessor for chunk chaining

ODESystem integrates lazily in `chunkSize` windows. To continue from one chunk
to the next it reads `result.y[result.y.length - 1]` (the last accepted state
vector), passing it as `y0` to the next `dopri` call. `OdeSolution` must expose
this as a method, e.g. `last_y() -> Float64Array`. Without it the caller cannot
chain chunks without re-integrating from `t0` each time.

`result.x[result.x.length - 1]` (the last accepted time, used to detect early
termination — see §2 `solution.terminated_early`) is already implied by the
plan; make `last_t() -> f64` explicit alongside `last_y()`.

### 5c. Variable naming for `solve_ode_exprs`

ODESystem calls `.subscripts_to_strings()` on each RHS expression and on the
variable names before passing them anywhere, so all free variable names in the
expressions are plain strings (e.g. `"t"`, `"x"`, `"y_1"` → `"y1"`). The
`solve_ode_exprs` constructor must accept:

```rust
solve_ode_exprs(
    rhs:        &[Expression],   // one per state variable, in order
    ind_var:    &str,            // name of the independent variable ("t")
    state_vars: &[&str],         // names of the state variables, same order as rhs
    t0: f64, t1: f64, y0: &[f64],
    tol: f64, max_steps: usize,
) -> OdeSolution
```

At each RK stage the evaluator binds `ind_var → t_current` and
`state_vars[i] → y_current[i]` before calling `eval_complex` on each RHS. The
caller is responsible for ensuring the expressions have been pre-normalized
(subscripts converted to strings) so the free variable names match.

### 5d. Prefigure Python is NOT a consumer

The Python PreFigure library (loaded via Pyodide) uses `scipy.integrate`
(`solve_ivp` / `odeint`) for all ODE-related diagram elements (slope fields,
phase portraits, solution curves). It calls back to JS only for MathJax
rendering and text measurement. No DoenetML bridge code routes ODE evaluation
through math-expressions for prefigure's purposes. This module is purely a
replacement for `numeric.dopri` in `ODESystem.js`.

---

## 6. Out of scope

- Stiff solvers (BDF/Rosenbrock) — `numeric.dopri` doesn't have them either,
  so Doenet content has never depended on them; revisit only on demand.
- Symbolic ODE solving (closed-form solutions) — a different, much larger
  feature; would build on INTEGRATION_PLAN and is not planned.
- Events/root-finding during integration (`dopri`'s `event` parameter) —
  ODESystem does not use it; add only if Doenet asks.
- Slope fields / phase-portrait visualization — handled by prefigure Python
  (scipy) on the Python side; not an ODESystem or math-expressions concern.
