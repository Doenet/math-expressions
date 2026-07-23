# Comprehensive simplify plan (Mathematica/SymPy-class `full_simplify`)

> **PROGRESS (updated 2026-07-20):** IN PROGRESS — chunks **S1–S4 landed**.
> Full native suite 422 passing; clippy clean throughout. All paths below are
> relative to `packages/math-expressions-rs/` (crate moved this session).
>
> **S1 — exact eval + `is_zero`** (`src/exact.rs`, `tests/exact_is_zero.rs`):
> exact-constant evaluator over ℚ(π, e, surds) + certified
> `is_zero(e, &Assumptions) -> Tri`. `max_exact_eval_ops` (10 000) added to
> `ResourceLimits`. Consequence wired: the `integrate` I2 gate accepts iff
> sampled `equals` OR the certified exact stages confirm `F'−f ≡ 0`
> (`exact::certified_zero`, the accept-only pipeline without the sampling
> refuter). Post-review note: `equals` runs first — the naive is_zero-first
> gate cost ~35× on the integrate suite because the refuter burns its full
> arbitrary-precision budget precisely on true zeros; the disjunction is
> order-independent, so equals-first is behavior-identical at baseline cost
> (measured 0.090s vs 0.089s baseline vs 3.09s naive, release, warm).
> wasm: `Expression.is_zero()`.
>
> **S2 — rational normal form** (`src/ratform.rs`, `tests/ratform.rs`):
> `together`/`cancel` over the multivariate poly GCD (`src/poly`) with opaque
> **kernels** (`sin x`, `√x`, `π` held fixed via fresh `$k` indeterminates).
> `max_ratform_terms` (512) + a 6-indeterminate cap guard blow-up. `is_zero`
> **stage (d)** now decides rational identities exactly
> (`1/(x+1)+1/(x-1)-2x/(x²-1) → 0`). wasm: `Expression.together()`.
>
> **S3 — trig/exp/log special values + parity** (`src/norm/special_values.rs`,
> `tests/special_values.rs`): `fold_special_values` — sin/cos/tan/cot/sec/csc
> on the π/12 lattice (reusing S1's tables + new `Exact::to_expr`), parity,
> integer-π-shift, `e^{ln u}→u`, gated `ln(e^u)→u`, `ln e`, `ln 1`, `e^0`.
> Answers the `sin(2π) ↛ 0` gap. No wasm surface (per §10; consumed by S7).
>
> **S4 — factorization over ℚ** (`src/factor.rs`, `tests/factor_s4.rs`):
> Kronecker splitting of the no-rational-root remainder into irreducibles
> (`x⁶−1 → (x−1)(x+1)(x²+x+1)(x²−x+1)`; `x⁴+1` correctly kept irreducible),
> product-exact and `equals`-gated. Added `factor_terms` (numeric-content +
> common-monomial pull-out, kernel-aware: `6x²+9x → 3x(2x+3)`,
> `sin(x)a+sin(x)b → sin(x)(a+b)`). wasm: `factor` (pre-existing).
> Post-review fixes: `factor_terms` output was HashMap-order nondeterministic
> (violates the S7 wasm/native-agreement requirement) — factor lists now
> sorted by `norm::cmp`; `factor_int` re-normalizes to primitive-int at every
> recursion level (an interpolated factor can be a non-integer rational
> multiple of the true factor, which broke the divisor enumeration's
> completeness — correctness was never at risk, every split is
> division-verified). Known limits: `factor_terms` skips negative-power
> commons (gate rejects, input returned unchanged); Kronecker budget (200k
> combos) is a hardcoded const, not a `ResourceLimits` field.
>
> **Deferred consequences** (behavior-preserving refactors, not blocking):
> collapse `diverge.rs` PiLin onto `exact::is_zero`; matrix eigen residual
> check → `is_zero`; matrix eigenvalue root ladder → `factor`; integrate I1 →
> `ratform`. True multivariate irreducible factorization (beyond `factor_terms`
> content) is not yet implemented.
>
> **Next: S5** (assumption/sign propagation), **S6** (radical denesting),
> **S7** (trig restructuring + the cost-directed `full_simplify` driver — the
> chunk after which the `full_simplify` wasm entry point lands). See
> `WHATS_LEFT.md` §B.5.

Status: DRAFT (not started). Chunked so every phase lands independently,
TDD-first, with the differential oracle intact throughout.

## 0. Design stance

Two decisions up front that everything else hangs on:

1. **New entry point, not a change to `simplify`.** The existing
   `simplify`/`simplify_core` is differential-tested against the JS
   library (simplify corpus); making it smarter breaks parity. Add
   `full_simplify(e, &Assumptions) -> Expr` (Mathematica's `Simplify` vs
   plain evaluation). `simplify` stays the oracle-compatible normalizer
   and is used as `full_simplify`'s inner canonicalizer. Corpus fixtures
   never change; `full_simplify` gets its own test suites plus
   spot-checks against SymPy-derived expected values (SymPy used offline
   as a second oracle for the *new* behavior; we never invent expecteds).
2. **Zero-equivalence as a service is the keystone.** Most of what makes
   a CAS simplifier trustworthy is a certified `is_zero(e) -> Tri`
   (Yes/No/Unknown). Several modules have already grown private
   approximations of it (see §8). Build it first (S1); every later chunk
   both consumes it and strengthens it.

Driver architecture (S7): Mathematica-style cost-directed search.
`full_simplify` applies a set of *transformations* (expand, together,
factor, trig-contract, trig-expand, exp-form, radical rules, …), scores
each candidate with a complexity measure (leaf count + per-node weights,
Big-integer digit penalty), keeps the frontier small (beam width ~4),
and stops at a fixpoint or budget. All transformations are
canonicalize-composable and individually sound; the driver only ever
*chooses*, so a bug in scoring can cost quality, never correctness.

## 1. Chunk S1 — exact constant evaluation + zero-equivalence service

**What.** New module `src/exact.rs`:
- Promote and extend the `PiLin` evaluator currently private to
  `precise/diverge.rs` (`exact_eval`, ℚ + ℚ·π with trig at the kπ/2
  lattice, exact sqrt of rational squares, exp(0)/ln(1)) into a general
  exact-constant evaluator over the tower ℚ → ℚ(π) → ℚ(e) → quadratic
  radicals → `RootOf` algebraics (via `rootof::upoly_in_root` /
  `power_reduced` arithmetic that matrix.rs already uses).
- `pub fn is_zero(e: &Expr, a: &Assumptions) -> Tri` pipeline:
  (a) canonicalize + expand-to-fixpoint; (b) exact constant evaluation
  when variable-free; (c) `evaluate_to_precision` at random rational
  points as a certified *refuter* (No answers are certified, unlike the
  current f64 sampling); (d) rational-function normalization once S2
  lands; (e) Unknown otherwise. §7f-limited.

**Tests.** Constant zoo (sin(2π), cos(π/3)−1/2, √8−2√2,
rootof-arithmetic identities, e^{ln 3}−3), non-zeros certified No,
adversarial almost-zeros (exp(π√163) style) → Unknown not No.

**Consequences.**
- `diverge.rs` sheds ~150 lines: `exact_eval`/`PiLin`,
  `exactly_zero_at`, `certified_nonzero_at` become calls into
  `exact::is_zero` (behavior-preserving refactor, divergence suite is
  the guard).
- `matrix.rs` eigen self-verification drops the expand-fixpoint helper
  and the `p(λ)+1 = 1` radical zero-compare trick.
- `integrate/mod.rs` I2 gate upgrades from sampled
  `equals(derivative(F), f)` to certified-when-possible `is_zero`
  (falls back to sampling on Unknown — strictly stronger).
- Risk: none to existing behavior (pure refactors gated by suites).

## 2. Chunk S2 — rational normalization (`together`/`cancel`/`ratsimp`)

**What.** Promote `integrate::expr_to_ratfun` (already builds ℚ(x)
rational functions) into `src/ratform.rs`, add multivariate support by
recursive univariate treatment (main variable by canonical order,
coefficients as expressions), gcd-cancel numerator/denominator
(`upoly::gcd` at the univariate base). Transformations exposed:
`together(e)`, `cancel(e)`. Non-rational subtrees (sin(x), √x, …) are
treated as opaque kernels (substituted by fresh symbols, restored
after) — the SymPy trick that makes ratsimp apply everywhere.

**Tests.** `1/(x+1) + 1/(x−1) → 2x/(x²−1)`, `(x²−1)/(x−1) → x+1`,
kernel opacity (`1/sin(x) + 1/sin(x) → 2/sin(x)` without touching sin),
no-blowup guards on 20-term sums.

**Consequences.**
- `is_zero` stage (d) activates: rational identities decided exactly.
- Equality engine gets a cheap pre-stage (normalize both sides, compare
  canonically) that resolves many cases before finite-field/sampling.
- `integrate` I1 can consume `ratform` instead of its private
  conversion (dedup ~100 lines).
- Risk: expression swell (gcd of large multivariate) → degree/term
  §7f limits, fall back to unnormalized on breach.

## 3. Chunk S3 — trig/exp/log special values and parity

**What.** Fold rule set (sound unconditionally, lives in
`full_simplify`'s inner pass):
- sin/cos/tan/sec/csc/cot at the kπ/6 and kπ/4 lattices (table +
  periodicity reduction of the rational multiple of π).
- Parity/phase normalization: `sin(−u) → −sin u`, `cos(−u) → cos u`,
  `sin(u+kπ)` reduction — argument normalized by leading-sign of the
  canonical form.
- Inverse-trig exact points (`asin(1/2) → π/6`, …), `atan` quadrant care.
- `e^{ln u} → u`; `ln(e^u) → u` only when `u` certified real (S5 else
  gated); `ln(1) → 0`, `e^0 → 1` (already in smart constructors — audit).

**Tests.** The lattice × all six functions; `sin(2π) → 0` (the question
that prompted this); `sin(101π/6)` periodicity; parity idempotence
(ping-pong guard extended).

**Consequences.**
- Answers the standing user-visible gap (`sin(2pi)` ↛ 0 today).
- `exact::is_zero` stage (b) gets these for free once folding runs
  before evaluation.
- Divergence classifier's `closed_form_candidates` π-lattice probing
  gets cheaper (folded divisors expose zeros structurally).
- Risk: none for `simplify` (untouched); `full_simplify`-only.

## 4. Chunk S4 — factorization over ℚ

**What.** `factor(e)` transformation: squarefree (Yun — exists in
`upoly`), rational-root extraction, quadratics exactly, and
degree-bounded Zassenhaus (small cases only; §7f `max_factor_degree`,
default 12) at the univariate base; multivariate by the S2 recursive
representation + content/primitive splitting. Also `factor_terms`
(common-factor pull-out, cheap, always tried by the driver).

**Tests.** `x⁴−1`, `x⁶−1` full splits; content extraction
`6x²+9x → 3x(2x+3)`; irreducibles stay put; degree-13 input refuses
politely (falls back to squarefree).

**Consequences.**
- `cancel` (S2) upgrades from gcd-only to factored cancellation.
- `matrix::eigenvalues` char-poly handling simplifies: its bespoke
  squarefree → rational-root → quadratic ladder becomes a `factor` call.
- Driver gets its most powerful complexity-reducer for polynomials.
- Risk: Zassenhaus blowup → hard degree/coefficient limits, tested.

## 5. Chunk S5 — assumption engine (sign/realness propagation)

**What.** Extend `src/assumptions/mod.rs` from leaf-lookup to a
recursive interval/sign analyzer: `sign(e, a) -> Tri`-style propagation
through Add/Mul/Pow/exp/sqrt/abs (even powers ⇒ ≥0, exp ⇒ >0, sums of
same-sign, products by sign algebra), plus `is_integer`/`is_real`
propagation. Then unlock the gated rules: `√(u²) → |u|`, and → `u` when
`u ≥ 0`; `|u| → u` when `u ≥ 0`; `ln(uv) → ln u + ln v` when both > 0;
`(u^a)^b → u^{ab}` when `u > 0`; `√(t²·R) → t√R` for `t ≥ 0`.

**Tests.** Propagation table; the JS `simplify(assumptions)` corpus
cases that currently sit in known-failures for assumption reasons;
branch-cut refusals (`√(x²) ↛ x` without the assumption) as *negative*
tests.

**Consequences.**
- **Subsumes T1b (`pull_nonneg_var`) in
  active-plans/SINGULARITY_TRANSFORM_PLAN.md**: the transform pipeline calls
  `full_simplify` with `t ≥ 0` in scope instead of a bespoke rewriter.
  (If S5 lands first, T1b shrinks to a one-line assumption insertion.)
- Existing `rule_assumptions` cluster in simplify.rs is re-expressed on
  the new engine (behavior-preserving for the corpus).
- Risk: branch-cut soundness is THE hazard of this chunk — every rule
  documented with its precondition and a negative test; nothing fires
  on Unknown.

## 6. Chunk S6 — radical denesting and rationalized forms

**What.** `radsimp`: rationalize denominators (`1/(1+√2) → √2−1`,
conjugate multiplication incl. RootOf denominators via
`rootof::power_reduced` inverse — `qinv` in matrix.rs already does the
quotient-ring inverse); sqrt denesting `√(a+b√c)` (Borodin–Fagin
special case: works iff a²−b²c is a perfect square); collected radical
arithmetic `√8 + √2 → 3√2` (partially in `rule_radical` — audit and
extend to the general integer-radicand case).

**Tests.** Denesting hits and certified misses; golden-ratio identities;
`1/rootof(t³−2, 0) → rootof(...)²/2`-style rationalization.

**Consequences.**
- Eigen output beautification: M3's quadratic closed forms and M4
  eigenvector components currently ship unrationalized.
- `exact.rs` gains stronger normal forms → more Yes answers from
  `is_zero`.
- Risk: low; rules are equational identities checked by `is_zero` in
  debug builds (self-verifying rule harness — cheap and catches rule
  typos at test time).

## 7. Chunk S7 — trig restructuring + the cost-directed driver

**What.** The driver described in §0, plus the transformation pair it
needs most: `trig_expand` (angle addition/multiple angles outward) and
`trig_contract` (product-to-sum, power reduction — the direction
Pythagorean alone can't reach). Complexity measure in
`src/norm/cost.rs` (leaf count, weight 2 per Apply, digit-length of
integers, +penalty for Float). `full_simplify` = beam search over
{expand, together, cancel, factor, factor_terms, trig_expand,
trig_contract, radsimp, special-value folds} with §7f budget
(`max_full_simplify_candidates`, default 64) — mirrors the existing
operation-count limits idiom in src/resource_limits.rs.

**Tests.** `sin²x·cos x + cos³x − cos x → 0`-class identities;
`(1+tan²x) cos²x → 1`; driver-budget exhaustion returns best-so-far
(never Unknown); idempotence `full_simplify(full_simplify(e)) ==
full_simplify(e)` fuzzed over the existing corpus inputs.

**Consequences.**
- This is the chunk after which "like Mathematica/SymPy" is fair to
  claim; S1–S6 are its arsenal.
- Equality engine can offer `equals_symbolic` (is_zero of the
  difference) as a certified pre-stage; the JS-parity `equals` behavior
  is unchanged by default.
- Risk: search-cost blowups → beam width + candidate budget; determinism
  required (no randomness in scoring — stable tie-breaks by canonical
  order) so wasm and native agree.

## 8. Existing code simplified (the payoff map)

| Existing code | Today | After |
|---|---|---|
| `diverge.rs` PiLin + `exactly_zero_at`/`certified_nonzero_at` (~150 lines) | private ℚ+ℚπ evaluator | `exact::is_zero` calls (S1) |
| `matrix.rs` eigen residual check (expand-fixpoint + `p(λ)+1=1` trick) | bespoke | `exact::is_zero` (S1) |
| `matrix.rs` eigenvalue root ladder | bespoke squarefree/rational/quadratic | `factor` (S4) |
| `matrix.rs` `qinv` quotient-ring inverse | private | shared with radsimp rationalization (S6) |
| `integrate/mod.rs` `expr_to_ratfun` | private, I1-only | `ratform` module, shared (S2) |
| `integrate` I2 gate `equals(F′, f)` | numeric sampling | certified `is_zero` first (S1) |
| SINGULARITY_TRANSFORM T1b `pull_nonneg_var` | planned bespoke rewriter | S5 assumption-gated radical rule |
| `eq/mod.rs` stage ladder | syntactic → finite-field → sampling | + cheap normalize-and-compare pre-stage (S2), optional certified mode (S7) |
| `simplify.rs` `rule_assumptions` | leaf-lookup gating | S5 propagation engine underneath (same corpus behavior) |

## 9. Limits (§7f additions, src/resource_limits.rs)

| Field | Default | Guards |
|---|---|---|
| `max_exact_eval_ops` | 10_000 | S1 tower evaluation |
| `max_ratform_terms` | 512 | S2 together/cancel swell |
| `max_factor_degree` | 12 | S4 Zassenhaus |
| `max_denest_depth` | 4 | S6 |
| `max_full_simplify_candidates` | 64 | S7 driver budget |
| `max_trig_lattice_denominator` | 48 | S3 periodicity reduction |

## 10. Ordering and dependencies

S1 → S2 → S3 land in order (each unlocks the next's tests); S4, S5, S6
are then independent of each other (any order, S5 first if the
singularity-transform work is scheduled soon); S7 last. Every chunk:
clippy clean, full native suite + wasm smoke green, no simplify-corpus
fixture changes ever. wasm surface grows one method per chunk milestone
(`full_simplify`, `is_zero`, `factor`, `together`) — additive only.
