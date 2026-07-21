# Singularity-transform plan: lifting the f64 cliff via x = c ∓ t²

> **PROGRESS (audited 2026-07-20):** NOT STARTED — draft, zero implementation
> (`WHATS_LEFT.md` §B.6, items 48–50, T1–T3). Follow-up to DIVERGENCE (done);
> T3 (irrational singular locations) is explicitly out of scope.

Status: DRAFT (not started). Follow-up to tmp/DIVERGENCE_PLAN.md, which ships
with the documented limitation that improper values near a singular point
c ≠ 0 cap out at ~6–7 digits (the "f64 cliff"). This plan removes that cap
for the common cases by a change of variables that relocates the singularity
to t = 0, where the existing machinery already reaches full depth.

## 0. The problem being solved

`improper_value` (src/precise/diverge.rs) excises a cell of width w around
each certified-convergent singular point and bounds its tail ∝ w^(1−β).
Reaching more digits requires shrinking w. Near a singular point at c ≠ 0
(e.g. x = 1 for ∫₀¹ dx/√(1−x²)) the interval evaluator computes the divisor
D = 1 − x² by subtracting two magnitude-~1 quantities; the mandatory outward
widening (≈16ε·magnitude per op) contributes an *absolute* uncertainty of
~10⁻¹⁴ independent of cell width. Once |D| falls below that, every interval
contains 0, sqrt/division refuse, and both the tail certificate and the
complement quadrature die. The cliff guard in `improper_value` detects this
and refuses with a dedicated reason. At c = 0 there is no cliff: D = x is
computed without cancellation and f64's graded (subnormal) spacing gives
absolute resolution all the way down.

## 1. The transform

For a singular point at the **upper** endpoint c = hi, substitute
x = c − t², dx = −2t dt:

    ∫_a^c f(x) dx  =  ∫_0^{√(c−a)} 2t · f(c − t²) dt

(lower endpoint c = lo symmetrically with x = c + t², upper limit √(b−c)).
Effects:

1. The singular point moves to t = 0 — exactly where f64 and the interval
   evaluator have no cliff.
2. A |x−c|^(−β) singularity becomes t^(−2β)·2t = 2·t^(1−2β):
   - β = ½ (sqrt divisors, the classroom case): exponent 0 — singularity
     **gone**; plain certified quadrature applies at full depth.
   - β < ½: also regular.
   - ½ < β < 1: still improper (exponent in (−1,0)) but now at t = 0,
     where excision reaches full depth. Weakened either way (2β−1 < β).

The far endpoint √(c−a) is generally irrational; that is fine — the
integrand is regular there and quadrature endpoints are f64 already (same
half-ulp endpoint rounding the current pipeline accepts). Construct it as
the exact Expr `sqrt(c − a)` and let `endpoint()` round it.

## 2. The structural-cancellation gate (the load-bearing step)

Substitution alone is not enough: `sqrt(1−x²)` becomes `sqrt(t²·(2−t²))`,
and the tapes/interval evaluator face the same 0/0 unless t² is pulled out
*structurally*. `simplify_core` will not do this in general (it cannot
assume t ≥ 0). Since the transformed domain is t ∈ [0, √(c−a)] with t ≥ 0
guaranteed by construction, add a dedicated post-substitution normalizer:

- **T1b `pull_nonneg_var`**: given the transformed integrand and the fresh
  variable t known ≥ 0, rewrite `(t^(2k) · R)^s → t^(2ks) · R^s` for
  Pow/sqrt nodes whose base factors as an even t-power times a remainder
  (reuse `split_factors` / the divisor factoring in `factor_divisor`).
  Valid only under t ≥ 0; lives inside the transform pipeline, NOT in
  general simplification. Then `simplify_core` cancels `2t · t^(−2β·…)`
  into the weakened power.
- **Gate**: after normalizing, re-run `classify` on the transformed
  integrand over [0, b′]. Accept iff it has no divergent cells AND its
  singular cells (if any) all sit at t = 0 with strictly smaller Σ s·m
  than the original cell. Otherwise **fall back** to the current
  `improper_value` path (today's behavior, including the honest refusal).

## 3. Phases

### T1 — endpoint singularity at an exact rational endpoint
The common case (∫₀¹ dx/√(1−x²), ∫₀¹ dx/√(1−x), …).

- Hook: in `improper_value`'s caller path (`integrate_analyzed`, after
  `classify` returns convergent `singular_cells`), when a cell's
  `exact: Some(pt)` is a rational Number AND coincides with an interval
  endpoint, attempt the transform before falling into the generic
  excision loop.
- Build g(t) = 2t·f(c ∓ t²) via `ops::substitute`, apply T1b, gate per §2,
  then evaluate the transformed integral with the existing pipeline
  (`plain_value` when the gate reports no singular cells; recursive
  `improper_value` when the residual singularity is at t = 0).
- Recursion guard: a `depth` parameter threaded through; transform at most
  once per cell (depth ≤ 1). No new §7f limit needed.
- Internal refactor required: `improper_value`/`plain_value` should return
  `Result<(f64 value, f64 err), Unknown-reason>` internally and `package`
  only at the top of `integrate_analyzed`, so transformed pieces and
  ordinary pieces can be summed with their error budgets before packaging.

### T2 — interior singularity at an exact rational point; multiple cells
- Split the interval at each exact rational singular location c:
  [lo, c] and [c, hi]. Each half now has an endpoint singularity → T1.
- Budget: run each half at `digits + 1` (cap 13) so the summed error meets
  the total target; sum (value, err) pairs via the §T1 refactor.
- Also covers several distinct singular cells (e.g. ∫₀² dx/√(x(2−x))):
  split between cells, transform each endpoint half.

### T3 — irrational singular locations (OUT OF SCOPE, recorded)
Cells whose exact location is a RootOf or π-lattice point would need exact
(non-f64) sub-interval endpoints and an exact substitution x = c − t² with
symbolic c, after which f(c − t²) rarely simplifies structurally. Leave
these on the current path (cliff-capped, honest refusal beyond ~6–7
digits). Documented here so the deviation is deliberate.

## 4. What must NOT change

- `IntegralVerdict` surface and wasm `integrate_analyzed` JSON: unchanged
  (results transparently improve).
- Divergence detection: the transform runs only on certified-convergent
  singular cells, after the Divergent short-circuit — ∫₀¹ dx/(1−x) etc.
  must still return Divergent with the same locations.
- The gate's fallback preserves every currently-passing refusal and value.

## 5. Tests (TDD: write first, watch the cliff refusals, then lift)

1. `∫₀¹ dx/√(1−x²)` at 12 digits → π/2 (today: Unknown beyond ~6).
2. `∫₀¹ dx/√(1−x)` at 12 digits → 2.
3. `∫₀¹ x/√(1−x²)` at 12 digits → 1 (numerator also vanishes; gate must
   still accept — transformed integrand is fully regular).
4. T2: `∫₀² dx/√(x(2−x))` at 12 digits → π (two endpoint cells).
5. `∫₀¹ dx/(1−x)^(3/4)` at 12 digits → 4 (β = ¾: residual t^(−1/2)
   singularity at t = 0; exercises the recursive improper path).
6. Divergence preserved: `∫₀¹ dx/(1−x)`, `∫₀¹ dx/(1−x)²` → Divergent at 1.
7. Gate fallback: an integrand where cancellation fails structurally
   (e.g. divisor `sqrt(sin(1−x))` if T1b cannot factor it) → same verdict
   as today (value at low digits / honest Unknown), never a wrong Value.
8. Full suite + wasm smoke green; clippy clean.

## 6. Estimated shape

~150–250 lines in diverge.rs (transform + T1b normalizer + gate + split
logic + the (value, err) refactor), no new modules, no new limits, ~8 new
tests in tests/divergence.rs.
