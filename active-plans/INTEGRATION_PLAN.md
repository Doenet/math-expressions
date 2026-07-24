# Symbolic Integration Plan

> **PROGRESS (audited 2026-07-20):** PARTIAL — I1–I2 shipped (indefinite
> integration, differentiation-gated; wasm `integrate`). I3–I5 open
> (`WHATS_LEFT.md` §B.2, items 27–29): by-parts/trig clusters/√-substitutions,
> hypergeometric terminal node, `integrate_with_story` + presentation polish.

Design for symbolic (indefinite) integration in `math-expressions-rs`, based
on an investigation of the Rubi rule-based integrator as foundation, with
generalized hypergeometric functions as the terminal representation for
non-elementary antiderivatives. Companion documents: `active-plans/DONE_MATRIX_PLAN.md`
(the `RootOf` construct is a direct dependency, §3), and
`active-plans/DONE_ARBITRARY_PERCISION_PLAN.md` (the pFq node is one more series kernel
there, §5; numeric self-verification uses its quadrature hooks).

**This is new capability, not porting.** The JS library has no symbolic
integration (only `integrateNumerically`, itself unported — PORTING_PLAN
§17). Everything here is beyond-JS, consistent with the divergence pattern
already established (assumptions-driven `abs`, matrices, precision).

---

## 1. Rubi: what it is and what the investigation found

Rubi (Albert Rich, [rulebasedintegration.org](https://rulebasedintegration.org/))
is ~6,700–7,800 integration rules organized as a decision tree over integrand
forms, each with applicability conditions and an "optimal antiderivative,"
plus a **72,000-problem test suite** with graded expected answers. MIT
licensed, Mathematica sources (human- and machine-readable), test suite
distributed translated into several CAS syntaxes. It outperforms Maple and
Mathematica on its own suite.

Facts that shape this plan (verified 2026-07-19):

1. **Rubi 4 selects rules by Mathematica-grade conditional pattern
   matching.** This is the porting killer: SymPy's MatchPy-based port
   ([sympy#12233](https://github.com/sympy/sympy/issues/12233)) stalled on
   matcher performance/completeness and was later deleted from SymPy;
   Symja's Java port works but consumed years of transpilation effort.
2. **Rubi 5** ([repo](https://github.com/RuleBasedIntegration/Rubi-5)) is
   the announced fix — the same rules compiled into a pure if-then-else
   decision tree, "relatively easy to port to virtually any CAS," ~2 orders
   of magnitude faster. But compilation of the ~7,800 leaves was being done
   *manually* and the repo has been effectively stalled since 2021. **Do not
   put Rubi 5 on the critical path**; monitor it (§9).
3. **Rubi's value system is this library's value system**: answers are
   graded A/B/C by whether they are the minimal, elementary,
   student-recognizable antiderivative (a result "unnecessarily involving
   higher-level functions" is downgraded) — the same bar the §7e
   presentation layer answers for `simplify`. And a rule-firing trace is a
   step-by-step derivation, the integration analog of the JS library's
   `derivative_with_story`.
4. Rubi's generic-parameter rules terminate in `Hypergeometric2F1` /
   `AppellF1` forms when no elementary antiderivative exists — so a
   hypergeometric terminal node (§5) is not a rival approach bolted on; it
   is what lets a Rubi-derived rule set be *total* on its input class.

**Central decisions:** (a) do not port Rubi's engine; use Rubi's rule
*content* and its test suite as the oracle, on a small matcher over our
canonical layer; (b) ship the complete rational-function algorithm first —
it is algorithmic, not rule-based, and everything it needs already exists;
(c) every result is gated by verify-by-differentiation, which this library
is unusually well equipped for (`derivative` + `equals` exist and are
corpus-hardened).

## 2. Architecture

```
integrate(f, x, assumptions) -> Option<Expr>       (src/integrate/)
  │  canonical f
  ├─ 1. table/linearity pass: Σ cᵢ·fᵢ integrates termwise; constants slide out
  ├─ 2. RATIONAL ENGINE (complete): f ∈ ℚ(vars)(x) → Hermite + LRT   (§3)
  ├─ 3. RULE ENGINE: curated Rubi-subset table + u-substitution +
  │     integration by parts, iterated to fixpoint under fuel        (§4)
  │        └─ terminal rules may emit Hypergeometric nodes           (§5)
  └─ 4. no engine succeeded → None (JS-style honest failure; the caller
        can still integrate numerically via the quadrature plan)

  Every success passes the gate:  equals(derivative(F, x), f)  — a result
  that fails the gate is discarded (and, in tests, is a hard error).
```

- The result flows through `present` like every user-facing operation, plus
  an integration-specific cleanup (`+ C` is the *caller's* concern; we
  return one antiderivative, like Rubi and mathjs).
- `integrate_with_story(f, x)` returns the rule trace (rule name, before,
  after) — deferred to the last phase but designed in from the start: the
  rule engine's step record is the story.

## 3. The rational engine — complete, first, and not rule-based

For f = p/q with p, q polynomial in x (coefficients rational, or symbolic
and x-free): **Hermite reduction + Lazard–Rioboo–Trager** is a complete
decision procedure needing only squarefree decomposition and subresultant
GCD chains — *no factorization*:

1. **Hermite reduction** peels the rational part: repeated use of
   `gcd(q, q′)` (the §8 poly layer's primitive-PRS gcd, plus the squarefree
   machinery MATRIX_PLAN §2c adds) rewrites ∫p/q as
   `rational_part + ∫a/b` with b squarefree.
2. **Lazard–Rioboo–Trager** on the squarefree remainder: the subresultant
   PRS of `b` and `a − t·b′` w.r.t. x yields, per degree d, a resultant
   factor `Rd(t)` and remainder `Sd(t, x)` with
   `∫a/b = Σd Σ_{t : Rd(t)=0} t · ln(Sd(t, x))`.
   The inner sums are **sums over the roots of Rd** — represented exactly as
   `RootOf(Rd, k)` (MATRIX_PLAN §2a): the second consumer of that construct,
   arriving with its own display form (`Σ` over roots, or expanded when
   roots are rational/quadratic — the same closed-form-extraction ladder as
   eigenvalues).
3. **Real-form cleanup** (presentation, not correctness): conjugate log
   pairs with quadratic `Rd` rewrite to `atan`/`ln` of real arguments —
   the form a student expects for `∫1/(x²+1)`. Rational-root and quadratic
   `Rd` cover every integral a calculus course assigns; higher-degree `Rd`
   stays in exact `RootOf`-sum form.

This one engine makes the library *provably complete* on rational functions
— the entire partial-fractions unit — in ~a week, before any rule exists.
Poly-layer additions needed: squarefree decomposition (shared with
MATRIX_PLAN), subresultant PRS with the `t`-parameter (a modest
generalization of the existing `pseudo_rem` chain), all under the existing
`MAX_DEGREE`/`MAX_PRS_STEPS` caps.

## 4. The rule engine — a curated Rubi subset on a deliberately small matcher

### 4a. Why the matcher can be small

Rubi 4's matcher must handle raw Mathematica patterns with
associative-commutative matching and arbitrary conditions. Our integrands
arrive **canonical**: flat sorted `Add`/`Mul`, numeric coefficient split
off, `Div`/`Neg` eliminated, like powers combined. Matching a rule like
`∫u·cos(a+b·x) dx` against canonical form reduces to: partition `Mul`
factors into (x-free coefficient, matched core, residual), where the
x-free/x-dependent split is one `contains_var` scan. The matcher is a
structural walk with typed holes (`const`, `linear-in-x`, `power-of-x`,
`any`), ~300 lines, not a term-rewriting engine. This is precisely the trick
that made the §7e simplify rules cheap.

### 4b. The rule inventory (~150–300 rules ≈ Calc I/II coverage)

Sourced from Rubi's published rule files (content, conditions, and optimal
answers — with attribution; MIT license is compatible), organized as Rubi
does, by integrand class:

| Cluster | Examples | Rubi section |
|---|---|---|
| Power/table | `xⁿ`, `1/x`, `eˣ`, `aˣ`, `ln x`, `sin`, `sec²`, `1/(1+x²)`, `1/√(1−x²)` … | 1.1, elementary tables |
| Linear substitution | `f(a+bx)` for every table `f` | pervasive `a+b·x` rules |
| **Derivative-divides (u-sub)** | `∫f(u)·u′ dx` — try each composite subterm `u`, test `reduce_rational(integrand / (f′∘u rule form))` x-free | Rubi's substitution meta-rules; SymPy `manualintegrate`'s workhorse |
| Integration by parts | `∫u·dv` with LIATE-ordered candidate split, fuel-bounded recursion, cyclic-parts detection (`eˣsin x`) | 1.3-adjacent |
| Trig powers/products | `sinᵐx·cosⁿx` reduction formulas, `tan/sec`, `sin(ax)cos(bx)` product-to-sum | ch. 2 |
| Standard algebraic substitutions | `√(a²−x²)`, `√(a²+x²)`, `√(x²−a²)` trig subs; `√(a+bx)` | ch. 1.2 |
| Partial-fraction dispatch | rational integrands route to §3 (the rule engine never does partial fractions itself) | — |
| **Terminal pFq rules** | `∫xᵐ(a+bxⁿ)ᵖ` generic exponents → `₂F₁`; kin | Rubi's generic-parameter leaves |

Rules fire innermost-first to a fixpoint under a fuel budget
(`limits.max_integration_steps`); by-parts and u-sub recursions consume the
same fuel, so mutual recursion cannot loop (§6).

### 4c. What makes this safe to build incrementally

- **The gate**: `equals(derivative(F,x), f)` after every candidate result.
  A mistranslated rule produces a discarded answer (and a corpus failure),
  never a wrong answer to a student.
- **The corpus**: filter Rubi's 72k suite to the clusters above
  (`scripts/generate-integration-corpus.*` — the suite ships in multiple CAS
  syntaxes; translate mechanically, seeded/deterministic like every other
  generator), run under the standard known-failures snapshot machinery.
  Coverage % per cluster becomes the objective progress metric, and decides
  empirically whether more rules are worth porting (§9).

## 5. Hypergeometric functions: terminal representation, not rival engine

The Marichev–Adamchik Meijer-G approach (SymPy `meijerint`) and Rubi are
philosophical opposites: uniform algorithmic coverage with special-function
answers vs. thousands of special cases with pedagogically-minimal answers.
For an educational CAS Rubi's philosophy wins as the *engine* — but the
hypergeometric family is the right **answer representation** where
elementary forms don't exist, exactly as `RootOf` is for eigenvalues:

- New node `Hypergeometric { p, q, params: Box<[Expr]>, arg: Box<Expr> }`
  (`₂F₁` etc. as p=2,q=1) — like `RootOf`, an exact object that is:
  - **emitted** by the terminal rules of §4b (Rubi's own design: its
    generic-parameter leaves return `Hypergeometric2F1`/`AppellF1`);
  - **numerically evaluable to arbitrary precision**: the pFq Taylor series
    has term ratio rational in n — one more ratio-recurrence `FnKernel` row
    in ARBITRARY_PERCISION_PLAN §6 (with the standard |arg|<1 domain guard;
    analytic continuation out of scope);
  - **simplifiable**: contraction rules for special parameters
    (`₂F₁(1,1;2;−x) → ln(1+x)/x`, arcsin/arctanh families, `₁F₁` → exp
    kin) live in a simplify cluster, so abstract answers degrade to
    elementary ones whenever parameters allow — Rubi's A-grade bar enforced
    post hoc;
  - **self-verifying numerically**: quadrature of `f` vs. pFq evaluation of
    `F` at sample points (the second verification loop, complementing the
    symbolic gate — valuable exactly where `equals`' symbolic derivative
    check meets an unfamiliar special function).
- **Rejected**: the full Meijer-G convolution machinery (huge lookup tables,
  contraction database; its payoff is *definite* integrals, which the
  quadrature plan serves honestly and far more cheaply). AppellF1 (two-
  variable) likewise deferred — represent as `OtherOp` if a ported rule
  needs it, no arithmetic.

## 6. Limits (§7f style)

| Field | Default | Bounds |
|---|---|---|
| `max_integration_steps` | 256 | total rule firings incl. by-parts/u-sub recursion (fuel) |
| `max_integration_candidates` | 64 | u-sub/by-parts split candidates per node |
| `max_lrt_degree` | 64 | Hermite/LRT input degrees (aligns with poly `MAX_DEGREE`) |
| `max_pfq_terms` | (shared) | pFq numeric kernel uses `max_series_terms` from the precision plan |

Failure is `None` (or an unevaluated `OtherOp("int", …)` at the API's
option), never a hang; all counts, no wall-clock; validated analytically per
the standing no-OOM-repro policy.

## 7. Testing

1. **Rubi-suite corpus** (§4c) with per-cluster coverage tracking; graded
   not just pass/fail but Rubi-style: does our answer `equals` the optimal
   one, or merely differentiate back correctly (a B/C grade — logged, not
   failed).
2. **The gate as a property**: for every corpus success,
   `equals(derivative(F,x), f)` — doubles as a stress corpus for
   `derivative`/`equals` themselves.
3. **Round-trip property** (proptest): generate F from a grammar of
   elementary forms, set f = derivative(F); `integrate(f)` must succeed and
   differentiate back (it need not reproduce F).
4. **Rational-engine completeness**: random p/q under the degree cap —
   `integrate` must never return `None`; LRT `RootOf` sums verified by the
   gate plus numeric spot checks.
5. **Adversarial**: fuel exhaustion on integration-by-parts cycles,
   `max_lrt_degree` inputs, nested pathological composites — `None` within
   budget, op-count asserted.

## 8. Phasing

> **Status: I1 + I2 ✓ done 2026-07-19** (`src/integrate/`,
> `tests/integrate.rs`, 14 tests; full suite 310 tests / 38 binaries; wasm
> smoke 51/51 incl. `integrate()` and the certified
> `integrate_to_precision()`).
>
> **I1 — the rational engine** (`integrate/rational.rs`), complete over
> ℚ(x) under `max_lrt_degree`:
> - canonical-tree → p/q extraction with fold-time gcd cancellation;
> - polynomial part by division; the rational part via
>   **Ostrogradsky–Hermite** — one linear solve over ℚ instead of iterated
>   Hermite reductions;
> - the log part via **Rothstein–Trager** rather than LRT's subresultant
>   bookkeeping (deliberate deviation: the resultant
>   `res_x(q, A − t·q′)` is computed by evaluation + Lagrange interpolation
>   with degree-drop point skipping, and each residue class takes a gcd in
>   its own domain — ℚ, ℚ(√m) as pair arithmetic, or ℚ[t]/(F) with
>   ext-Euclid inverses. Same theorem, same output; LRT's efficiency win is
>   irrelevant at our caps. A zero divisor in the quotient-ring gcd is a
>   discovered factor of F: split and re-dispatch, same pattern as the
>   eigenvector engine);
> - the §3.3 real cleanup: complex-pair residues h ± i·s emit
>   `h·ln(U² + s²V²) − 2s·atan(…)` with the atan argument oriented so the
>   higher-degree polynomial sits in the numerator (`atan(x)`, not
>   `−atan(1/x)`), and √ of perfect-square rationals folds exactly; real
>   quadratic pairs stay as two ln's with `√m` coefficients; deg ≥ 3
>   residues emit per-index `RootOf` log terms (exercised and gate-verified
>   on `1/(x³−x−1)`).
> - Exit criteria met: the seeded completeness property (25 random proper
>   rational functions, never `None`, every result gate-verified) is green.
>
> **I2 — matcher + rules** (`integrate/mod.rs`): linearity + x-free
> coefficient slide; the elementary table (power rule incl. rational
> exponents, 1/u → ln u, aᵘ/eᵘ, ln/log10, sin/cos/tan/cot, sec²/csc² in
> their canonical `cos(u)⁻²` clothing, sinh/cosh/tanh, sqrt,
> asin/acos/atan by-parts closed forms, and 1/√(c−bu²) → asin) with linear
> inner arguments throughout (`f(a+bx) → F(a+bx)/b`, symbolic
> coefficients included); and derivative-divides **u-substitution** that
> re-enters the whole pipeline on the rewritten integrand (with divisor-
> power candidates and the `b^(kj) → u^j` rewrite so `u = x²` works inside
> `x⁴`). All under one `max_integration_steps` fuel budget; every
> top-level result passes the gate `equals(derivative(F, x), f)` or is
> discarded. New limits: `max_integration_steps` 256,
> `max_integration_candidates` 64, `max_lrt_degree` 64.
>
> Documented deviations/scope notes: the rational engine is ℚ-coefficient
> (symbolic-coefficient integrands route through the table/u-sub rules —
> the §3 "symbolic and x-free" generality is deferred to I3+); the Rubi
> 72k corpus is not yet vendored (no fetch tooling for it in-container),
> so I2's exit is the gate-verified rule/u-sub suite rather than the
> cluster-coverage metric — wiring the corpus stays on the I3 docket.
> `+ C` remains the caller's concern. Definite integrals are served
> numerically by the certified quadrature
> (`precise/quad.rs::integrate_to_precision`, see
> ARBITRARY_PERCISION_PLAN post-plan additions) pending the §10 Q2
> branch-cut design note.

| Phase | Deliverable | Exit criteria |
|---|---|---|
| **I1** (~1 wk) | rational engine: squarefree (shared w/ MATRIX_PLAN), t-parameterized subresultant PRS, Hermite, LRT, `RootOf` sums + atan/ln real cleanup | rational corpus: 100% gate-verified; completeness property green |
| **I2** (~1 wk) | matcher + linearity + table/linear-sub clusters + u-substitution | Rubi-suite clusters 1–3 ≥ 95% gate-verified |
| **I3** (~1 wk) | by-parts, trig clusters, algebraic substitutions | clusters 4–6 coverage tracked; fuel adversarial tests green |
| **I4** (~3 d) | `Hypergeometric` node: terminal rules, contraction simplify cluster, precision-plan kernel row, numeric cross-check | generic-exponent corpus rows green; contraction tests |
| **I5** (~3 d) | `integrate_with_story`, `present` polish (`RootOf`/pFq display), wasm `integrate()`, PORTING_PLAN §17 update | full suite green; wasm smoke |

Dependencies: I1 needs MATRIX_PLAN's squarefree + `RootOf` (M3) *or* ships
first with `RootOf` introduced here (whichever plan executes first owns it).
I4 needs ARBITRARY_PERCISION_PLAN P2 for numeric pFq (the symbolic node and
contraction rules don't).

## 9. Deferred / rejected / monitored

- **Full Rubi 4 port** (pattern-matching engine): rejected — SymPy's
  documented failure mode; Symja's cost. The corpus coverage numbers from
  §4c are the evidence basis for ever revisiting.
- **Rubi 5 transpilation**: monitored — if the if-then-else tree ships, a
  Mathematica→Rust codegen of it becomes the cheapest route to ~full
  elementary coverage, and our rule engine's answers remain the verified
  baseline underneath.
- **Meijer-G definite-integral machinery**: rejected (§5).
- **Risch** (complete transcendental algorithm): out of scope — months of
  work, answers often pedagogically alien; the honest `None` + numeric
  quadrature is the right fallback for an educational tool.

## 10. Open questions

1. API failure form: `None` vs. unevaluated `OtherOp("int", [f, x])` (which
   renders as `∫ f dx` and lets Doenet display "no elementary form found").
   Plan leans `Option` at the Rust API with the opaque node available
   behind a flag.
2. Definite integrals: `integrate(f, x, a, b)` = F(b) − F(a) with a
   continuity check on [a, b] (branch-cut hazards!) — worth a small design
   note before I5; numeric fallback via the quadrature plan regardless.
3. Should `integrate_with_story` reuse the JS `derivative_with_story` text
   conventions for Doenet consistency? (Needs a look at how Doenet renders
   those stories.)

---

*Sources: [rulebasedintegration.org](https://rulebasedintegration.org/)
(rules, vision, test problems/results pages), the
[Rubi](https://github.com/RuleBasedIntegration/Rubi) and
[Rubi-5](https://github.com/RuleBasedIntegration/Rubi-5) repositories, and
[sympy/sympy#12233](https://github.com/sympy/sympy/issues/12233) (SymPy Rubi
port post-mortem). Rational-engine references: Hermite reduction and
Lazard–Rioboo–Trager as presented in Bronstein, "Symbolic Integration I"
(Springer), ch. 2.*
