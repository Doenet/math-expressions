# Divergence Classification Plan

Design for distinguishing **divergent** integrals from convergent-improper
and merely-expensive ones in the certified quadrature
(`precise/quad.rs::integrate_to_precision`). Companion to
`tmp/ARBITRARY_PERCISION_PLAN.md` (the interval tape evaluator and the
certified f64 node evaluator are the workhorses) and
`tmp/INTEGRATION_PLAN.md` (the rational machinery of I1 — `expr_to_ratfun`,
Sturm isolation — makes one whole tier *exactly decidable*).

**Motivating gap (probed 2026-07-19, `tests/quadrature_poles.rs`)**: today
every integral the quadrature cannot certify returns the same shape of
`Unknown` — `∫₋₁¹ dx/x²` (divergent), `∫₀¹ dx/√x` (convergent improper,
value 2), and a merely sharp-but-smooth spike all look identical to the
caller until the spike converges. Budget exhaustion is deliberately not a
divergence claim (§7f: never guess), but the caller deserves three honest
answers where three exist: **a certified value, a certified "diverges," or
Unknown**.

---

> **Status: ✓ fully implemented 2026-07-20** (`src/precise/diverge.rs`;
> `tests/divergence.rs`, 17 tests across all six §7 suites; full suite
> 360/360 over 44 binaries; wasm smoke 59/59; clippy clean). All four
> phases landed:
>
> - **V1 (D1)**: exact rational decision via `expr_to_ratfun` + Sturm
>   isolation; pole locations reported with exact forms (rational snap or
>   `RootOf` of the denominator's radical at its real index). Two real bugs
>   caught by the suite: a units error in the rational-root snap loop
>   (comparing n/q against b·q — quadratic blowup), and a sign-orientation
>   bug in the straddle bisection when the isolating interval's *left
>   endpoint is itself another root* (f(a) = 0 broke the comparison and
>   lost the pole at 9 in x(x−9)); bisection now orients by −sign(f(b)).
> - **V2 (D2)**: structural divisor discovery (negative powers incl.
>   `sqrt(D)⁻¹`, tan/cot/sec/csc rewrites), grid + certified-sign
>   bisection, the MVT comparison certificate, and exact-point probing.
>   The probing required a new **exact evaluator over ℚ + ℚ·π** (`PiLin`):
>   neither `canonicalize` nor `simplify` folds sin(0)/cos(0), and the
>   numeric tiers can never certify a true zero — trig/exp/ln fold only at
>   their exactly-known points, so probe verdicts stay rigorous.
> - **V3 (D3)**: convergence certificates (lower MVT/Taylor bounds per
>   vanishing divisor, one ln factor per cell via the (eγ)⁻¹t^{−γ}
>   inequality) and cell-excision with certified tails; complement pieces
>   run one digit tighter than the total target through the refactored
>   `adaptive_quadrature` (now exposing an absolute-tolerance floor).
>   ∫₀¹ x^{−1/2} = 2, ∫₀¹ x^{−1/3} = 3/2, ∫₀¹ ln x = −1 to 8 digits;
>   ∫₀¹ (1−x²)^{−1/2} = π/2 to 6.
> - **V4**: wasm `integrate_analyzed` (JSON verdicts) + smoke rows;
>   `integrate_to_precision` screens divergent inputs into the
>   "integral diverges on the interval" reason (suite-6 consistency).
>
> Documented deviations/limitations discovered in implementation:
> - **The f64 cliff at non-zero singular points**: interval cancellation
>   makes cells within ~16ε·|c| of a singularity at c ≠ 0 unresolvable, so
>   tail-excision accuracy is capped (β = ½ at x = 1 ⇒ ~6–7 digits; the
>   same singularity at x = 0 reaches full depth thanks to subnormal
>   spacing). Deeper requests refuse with a dedicated reason. A variable
>   transform (x = c − t²) would lift this; planned in
>   tmp/SINGULARITY_TRANSFORM_PLAN.md.
> - At most one vanishing ln factor per singular cell (the multi-log
>   product bound was not worth its complexity yet).
> - Empirical (non-structural) candidate discovery is folded into the
>   grid's ambiguous-sign cells rather than a separate interval-failure
>   bisection pass; unclassified candidates route to the plain quadrature
>   as the honest arbiter.
> - The suite-1 property demanded full decidability for rational inputs
>   with poles *outside* the interval too — near-boundary poles make the
>   proper integral expensive but D1 itself never fails; the property held
>   as planned.

## 0. Scope decisions (up front, as library conventions)

1. **Finite intervals only.** `[a, b]` with finite endpoints, as today.
   Infinite intervals (∫₀^∞) are a different feature (decay analysis,
   oscillatory convergence) and stay out of scope.
2. **Improper-Riemann semantics.** "Divergent" means: some one-sided limit
   ∫→c of |the integral| is infinite. Cauchy principal values (`PV ∫₋₁¹
   dx/x = 0`) are NOT computed; a PV flag is noted as possible future work
   (§8). An interior pole of order 1 is therefore *divergent* here.
3. **Every verdict is a certificate, never a heuristic.** A `Divergent`
   answer must be backed by a rigorous comparison bound (below); numeric
   slope-fitting of the singularity exponent is explicitly rejected as a
   deciding mechanism (it may *guide* which certificate to attempt).
   Anything uncertifiable stays `Unknown` — same honesty bar as the root
   isolation's index ordering.
4. **The classifier is a front end, not a rewrite.** The existing certified
   Simpson machinery is untouched; classification runs before (and its
   singular-point knowledge later *enables* certified improper values, §5).

## 1. Verdict type and API

```rust
pub enum IntegralVerdict {
    /// Certified digits (proper, or — §5 — certified improper).
    Value(Precise),
    /// Certified divergent, with the singular points that prove it.
    Divergent { at: Vec<SingularPoint> },   // SingularPoint { location: Expr (exact where known) + f64, side, order info }
    Unknown(&'static str),
}

pub fn integrate_analyzed(f: &Expr, var: &str, a: &Expr, b: &Expr,
                          digits: usize) -> IntegralVerdict
```

- `integrate_to_precision` keeps its exact signature and Value/Unknown
  contract (Doenet compatibility); it delegates to the classifier so that
  divergent inputs return `Unknown("integral diverges at x = …")` — a
  better reason string, same type. New callers use `integrate_analyzed`.
- wasm: `integrate_analyzed(...)` returning JSON
  `{"status": "value"|"divergent"|"unknown", "value"?, "singularities"?,
  "reason"?}` (mirrors the `eigenvalues()` JSON pattern).

## 2. Tier D1 — rational integrands: an exact decision procedure

For `f ∈ ℚ(x)` (recognized by the I1 engine's `expr_to_ratfun`, which
already cancels the gcd):

1. Reduce `p/q`, `gcd(p, q) = 1`.
2. Isolate the real roots of `q` in `[a, b]` (Sturm — exists, exact, and
   handles irrational pole locations like `√2` that no float scan can pin).
   Endpoints count: a root at `a` or `b` is a one-sided singularity.
3. **Any root ⇒ Divergent.** After cancellation, every real pole has order
   ≥ 1 with nonvanishing numerator, and ∫ dx/|x−c|^m diverges for all
   m ≥ 1 (order 1 logarithmically — divergent under decision 2's
   semantics). The verdict carries the isolated pole locations (exact
   `RootOf`/rational where the factor structure gives them, else the
   certified interval midpoint).
4. No roots ⇒ proper; hand off to the existing certified quadrature.

This tier is **complete** — a genuine decision procedure, zero heuristics,
built entirely from shipped parts. It alone converts the two most common
student inputs (`1/x²` across 0, `1/(x−c)` across c) from budget-exhausted
`Unknown` to instant `Divergent` with the pole named.

## 3. Tier D2 — divergence certificates for kernel-built integrands

Beyond ℚ(x), decidability is gone in general; what remains is a
*certificate search* with three rigorous certificate forms. First,
candidate singular cells:

### 3a. Candidate discovery

- **Structural**: walk the canonical integrand for divisor structure —
  `Pow(D, e)` with `e < 0` (any spelling, including `sqrt(D)⁻¹` = e = −½),
  `tan`/`cot`-style kernels (pole lattice `π/2 + kπ` mapped through linear
  inner arguments — exact locations), `ln`/`log` arguments (for §5, not
  divergence). Each divisor `D` gets its real zeros bracketed: rational
  sub-`D` via Sturm (exact); general `D` via certified-sign bisection using
  the Tier-0 evaluator (a sign is certified when `|value| > err`).
- **Empirical fallback**: bisect the interval-evaluator failure set (as the
  adaptive splitter already implicitly does) down to
  `max_singularity_candidates` isolated cells; each becomes a candidate.
  Empirical candidates can only *locate* trouble — verdicts still require a
  certificate below.

### 3b. The mean-value comparison certificate (the main tool)

For integrand locally of the form `N(x)/D(x)^s`, `s ≥ 1` (s = |e| from the
`Pow(D, e)` structure; products of divisors handled factor-by-factor with
the rest folded into N):

> If on a cell `C = [l, r]` we certify
> (i) a sign change of `D` between `l` and `r` (certified f64 evaluations,
>     `|value| > err` at both ends) — so `D(ρ) = 0` for some `ρ ∈ C`;
> (ii) `sup_C |D′| ≤ M` (interval evaluation of the derivative tape — the
>      same tapes the Simpson remainder already builds);
> (iii) `inf_C |N| ≥ n > 0` and `N` sign-constant (interval evaluation);
>
> then by the mean value theorem `|D(x)| ≤ M·|x − ρ|` on C, hence
> `|f| ≥ (n/Mˢ)·|x − ρ|^{−s}`, and with `s ≥ 1`:
> **∫_C |f| = ∞ — divergence certified.**

Properties worth noting:

- The MVT bound needs **no knowledge of the zero's order** — higher-order
  zeros only make `|D|` smaller, so the certificate covers `D = (x)`,
  `D = sin x`, `D = x − sin x` (order 3; sign change still present)
  uniformly, as long as the zero has odd order (sign change exists).
- `s < 1` fails (correctly): `1/√x` gets no divergence certificate and
  flows to §5.
- Literal fractional exponents make the convergent/divergent boundary
  *exactly* decidable at the structural level: `x^{−1001/1000}` certifies
  divergent, `x^{−999/1000}` certifies convergent (§5) — a sharp test pair.

### 3c. Exact-point probing for even-order zeros

Even-order zeros (no sign change: `1/(cos x − 1)` at 0) are
indistinguishable from near-misses (`1/((x−c)² + 10⁻³⁰⁰)`) by any finite
f64 evidence — the honest f64 answer is Unknown. But the *symbolic* layer
can often do better: collect closed-form candidate points in the cell
(rational points from `D`'s rational sub-structure, `π`-lattice points from
trig kernels, `RootOf` atoms) and test `D`, `D′`, … at them with
`evaluate_to_precision` / `simplify` — which answer **exactly** (Tier R) for
these inputs. If `D(r) = 0` exactly with `D⁽ʲ⁾(r) = 0` for j < m and
`D⁽ᵐ⁾(r) ≠ 0` certified, and `m·s ≥ 1` with N nonzero at r: divergent,
certified via the Taylor bound `|D| ≤ sup|D⁽ᵐ⁾|/m!·|x−r|^m` (sup from the
interval evaluator, legitimate now because the lower derivatives vanish
*exactly* at r). This catches `1/(cos x − 1)`, `1/(x − sin x)` would
already fall to 3b, and `1/(1 − cos x)^{1/2}` (m·s = 1).

## 4. What stays Unknown (documented, tested)

- Even-order zeros at points with no closed form (`1/(g(x))²` with `g`'s
  zero only numerically known): no finite certificate can exclude
  `g² + 10⁻³⁰⁰`. Unknown is correct, not a limitation to fix.
- Divisors whose derivative bound `M` cannot be certified (interval
  evaluator fails on `D′` over the cell) — rare, since `D` is smooth
  wherever `f`'s only trouble is `D`'s zero; but honest fallback exists.
- Essential singularities (`sin(1/x)/x` oscillation): candidate discovery
  finds the cell, no certificate form applies — Unknown.

## 5. Tier D3 — certified values for convergent improper integrals

The classifier's flip side, so "not divergent" doesn't collapse into
Unknown. For an endpoint singularity at `a` (interior ones split first):

1. **Convergence certificate**: exhibit `β < 1` and certified `M` with
   `|f| ≤ M·(x−a)^{−β}` near `a`. Mechanism: form `h = f·(x−a)^β`
   *symbolically*, `simplify` (structural cancellation is what makes this
   work: `1/√x · x^{1/2} → 1` exactly), and interval-evaluate `h` on
   `[a, a+δ]`. β comes from the structural exponent (3a): `s` for
   `Pow(D, −s)` divisors with simple zeros, ½ for `sqrt` divisors, any
   `β ∈ (0,1)` for `ln` (with a dedicated `ln`-bound rule, since interval
   arithmetic can't see `x^β·ln x → 0` unaided).
2. **Tail-bounded evaluation**: certified tail
   `∫_a^{a+ε} |f| ≤ M·ε^{1−β}/(1−β)`; run the existing certified Simpson on
   `[a+ε, b]`; shrink ε (geometric ladder, `max_improper_refinements`)
   until tail + quadrature error meet the digit target. The result is a
   true certified `Value` — `∫₀¹ dx/√x = 2` and `∫₀¹ ln x dx = −1` become
   answers instead of refusals, cross-checkable against
   `evaluate_to_precision` of their closed forms.

(A variable transform `x = a + t²` is a cheaper special case for β = ½;
the tail-bound route is kept as the general mechanism. The transform is
now planned separately in tmp/SINGULARITY_TRANSFORM_PLAN.md — it is also
what lifts the f64 cliff at non-zero singular points.)

## 6. Limits (§7f — operation counts)

| Field | Default | Bounds |
|---|---|---|
| `max_singularity_candidates` | 32 | structural + empirical candidate cells per call |
| `max_certificate_bisections` | 256 | sign-certified bisections across all candidate brackets |
| `max_improper_refinements` | 64 | ε-ladder steps in §5 (each step reuses the quadrature budget) |
| (reused) `max_quadrature_segments` | 16 384 | unchanged |

Failure of any budget → `Unknown`, never a hang; validated analytically per
the standing no-OOM policy.

## 7. Testing

1. **D1 exactness suite**: poles at rational, irrational (`x²−2` across
   `√2`), and endpoint locations; high-order poles; no-pole controls on
   adjacent intervals; a seeded property test — plant poles by
   construction (`random poly / ∏(x−cᵢ)^{mᵢ}`) and require `Divergent`
   exactly when some planted `cᵢ ∈ [a,b]` survives cancellation.
2. **Certificate suite (D2)**: `tan` on [0,2], `1/sin` on [−1,1],
   `1/(x−sin x)`, `1/(1−cos x)` (even order, via 3c), the sharp pair
   `x^{−1001/1000}` vs `x^{−999/1000}` on (0,1].
3. **D3 values vs closed forms**: `1/√x → 2`, `x^{−1/3} → 3/2`,
   `ln x → −1`, `1/√(1−x²) → π/2` (upper-endpoint), all compared
   digit-for-digit against `evaluate_to_precision` of the closed forms.
4. **Adversarial near-divergence**: `1/(x² + 10⁻¹²)` and kin must return
   *values* (already proven convergent by the spike fix), never
   `Divergent`; `1/((x−c)² + 10⁻³⁰⁰)` must be `Value` or `Unknown`, never
   `Divergent` — the certificate-only discipline makes this a hard
   invariant, asserted as a property.
5. **Honest-Unknown suite**: the §4 cases, asserted `Unknown` with the
   documented reasons, all fast (budget-bounded).
6. **Consistency**: every `Divergent` input, fed to plain
   `integrate_to_precision`, must produce `Unknown` (never a value) — the
   two front ends may not contradict each other.

## 8. Phasing

| Phase | Deliverable | Estimate |
|---|---|---|
| **V1** | `IntegralVerdict` + D1 rational decision + reason-string wiring into `integrate_to_precision` + suite 1 | ~1 d |
| **V2** | candidate discovery (3a) + MVT certificates (3b) + exact-point probing (3c) + suites 2, 4, 5, 6 | ~2 d |
| **V3** | convergence certificates + tail-bounded improper values (§5) + suite 3 | ~2 d |
| **V4** | `integrate_analyzed` wasm + smoke rows, plan/doc updates | ~½ d |

V1 is independently shippable and covers the dominant classroom cases; V2
and V3 are independent of each other (either can land second).

## 9. Deferred / rejected

- **Cauchy principal values**: a `principal_value: bool` option on
  `integrate_analyzed` could serve `PV ∫ dx/x`-style exercises; deferred
  until Doenet asks. The D1 pole report already contains everything needed.
- **Infinite intervals**: separate feature (decay/oscillation analysis at
  ∞); rejected here to keep the certificate forms compact.
- **Numeric exponent estimation as a verdict source**: rejected on
  principle (decision 3) — it may only select which certificate to try.
- **Symbolic divergence via antiderivatives** (integrate symbolically with
  I1/I2, inspect the antiderivative's limits): tempting for rational
  integrands but subsumed by D1, and limit analysis of general
  antiderivatives is a larger feature than the certificates themselves.
