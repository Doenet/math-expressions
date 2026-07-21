# Matrix Capabilities Plan

Design for symbolic matrix algebra in `math-expressions-rs`, up to and
including **abstract eigenvalues/eigenvectors** — exact answers for matrices
whose eigenvalues are not expressible in standard functions (degree ≥ 5 char
polys have no radical closed form at all; degree 3–4 have closed forms that
are pedagogically useless). Companion document to
`tmp/ARBITRARY_PERCISION_PLAN.md` (root isolation is a planned consumer of
that evaluator) and `tmp/PORTING_PLAN.md` (§8 poly layer is the workhorse).
`tmp/INTEGRATION_PLAN.md` §3 is the second consumer of the `RootOf`
construct (Lazard–Rioboo–Trager log sums) and shares the squarefree
decomposition added in §2c — whichever plan executes first owns those pieces.

**This is new capability, not porting.** The JS library treats matrices
exactly as we currently do: a passive container. Today `Expr::Matrix { rows,
cols, entries }` parses (LaTeX environments, `parse/latex.rs:810-`), renders
(`output/latex.rs:273`), canonicalizes entry-wise (`norm/mod.rs:80`),
participates in `substitute`/`variables`/`free_symbols`, and `equals`
compares dimension-guarded componentwise (`eq/mod.rs:364`). There is no
arithmetic: `A + B` and `A·B` do not combine.

---

## 0. Scope decisions (made up front, documented as library conventions)

1. **Only literal `Matrix` nodes are matrices.** A bare symbol is always a
   scalar; we do not infer "x names a matrix." Scalars commute with
   matrices; matrix factors do not commute with each other.
2. **Dimension mismatches stay unevaluated** (opaque `Add`/`Mul` nodes), the
   same policy as every other non-numeric shape in the library — no errors
   thrown from canonicalization.
3. **`Matrix + scalar` stays unevaluated** (mathjs errors here; broadcasting
   is a numpy-ism that confuses linear-algebra pedagogy). `scalar · Matrix`
   distributes into entries.
4. **Zero-testing of symbolic pivots is tri-state.** Rank/rref/inverse over
   symbolic entries gate every pivot decision on the assumptions system's
   `is_nonzero`; `Some(true)` → pivot, `Some(false)` → skip, `None` → the
   operation returns its unevaluated form (same policy as the derivative
   catch-all and `solve_linear`'s nonzero gate). No silent case-guessing.
5. **Jordan form / generalized eigenvectors are out of scope.** We report
   algebraic multiplicity (from squarefree decomposition) and geometric
   multiplicity (nullspace rank); defective matrices are visible from the
   mismatch, not "repaired."
6. **No full factorization over ℚ** (Zassenhaus/LLL). Squarefree
   decomposition + rational-root extraction + quadratic-factor extraction
   only. A `RootOf` over a reducible (but squarefree, rational-root-free)
   polynomial is still *correct*, occasionally just less minimal than
   SymPy's.

## 1. Layer 1 — matrix algebra in the canonical layer

> **Status: M1 ✓ done 2026-07-19** (test-first; `tests/matrix.rs`, 20 tests).
> Implemented: segmented non-commutative `mul` with adjacent literal folding
> and scalar distribution into entries (`norm/mod.rs::mul` + `matmul_literal`
> + `is_matrix_valued`), entrywise `add` with per-dimension accumulation,
> `pow` matrix arms (k ≥ 2 binary powering under `max_expand_power`, `A⁰ → I`,
> negative/symbolic/non-square stay unevaluated), the
> `(a·b)^k`-distribution guard for matrix products, `transpose`/`trace`/
> `matmul` in `src/matrix.rs` (opaque `OtherOp` fallbacks), and the
> presentation guard keeping matrix bases off fraction bars. Properties
> verified: non-commutativity witness, `(A+B)C = AC+BC`, `(AB)C = A(BC)`,
> `(AB)ᵀ = BᵀAᵀ`, dimension-mismatch opacity, simplify idempotence.
> **§1b (M2) ✓ done 2026-07-19** (same TDD pass; `tests/matrix.rs` now 31
> tests): `det` tiered exactly as the table below — exact `Number`
> elimination / polynomial Bareiss with `reduce_rational` cancellation
> (verified past the symbolic cap on an 8×8) / cofactor ≤
> `max_symbolic_det_dim` — plus `matrix_inverse` (rational Gauss–Jordan;
> symbolic adjugate/det gated on `is_nonzero(det)`), assumption-gated
> `rref`/`rank`/`nullspace` (undecidable pivot ⇒ opaque/`None`, never a
> guess), nullspace vectors normalized to numeric leading 1, and the
> canonical `pow` folding `A^(−k)` through the exact inverse for invertible
> rational matrices (singular/symbolic stay unevaluated). New limits:
> `max_matrix_dim` 64, `max_symbolic_det_dim` 6.
>
> **M3 + M4 + M5 ✓ done 2026-07-19** — the plan is fully implemented
> (`tests/eigen.rs`, 27 tests, written first; full suite 35/35 binaries;
> clippy clean; wasm smoke 47/47). What landed, per section:
>
> - **§2a `Expr::RootOf`**: leaf variant exactly as specified (Box<[Number]>
>   coeffs + index); canonical rank 3 (between `Sym` and `Pow`, later ranks
>   shifted +1); invariant (primitive integer, positive lc, squarefree)
>   enforced by `rootof::make_rootof` and re-enforced in `canonicalize` for
>   deserialized trees. Text form `rootof(t^3 - t - 1, 2)` parses (new
>   `rootof` row in the default function table) and round-trips; LaTeX
>   renders `\operatorname{Root}_{k}(…)`; the JS-tree serializer emits the
>   application spelling, which re-canonicalizes to the leaf.
> - **§2b `char_poly`**: Faddeev–LeVerrier over exact `BigRational` for
>   rational entries at any n ≤ `max_matrix_dim`; cofactor expansion of
>   `λI − A` for symbolic entries under `max_symbolic_det_dim`.
> - **§2c pipeline**: new dense univariate module `src/upoly.rs` — Yun
>   squarefree decomposition, capped rational-root extraction (divisor
>   enumeration under `max_trial_divisor`; unfactorable ends leave roots in a
>   correct-but-less-minimal `RootOf`, decision 6), quadratic closed forms
>   via the formula, Sturm-chain isolation (exact, count-based bisection —
>   endpoint-root-proof) with `max_isolation_bits` as the bisection budget,
>   and Durand–Kerner + Newton polish for the complex ordering, certified
>   against the exact Sturm real count (any ambiguity ⇒ `None`, never a
>   guessed index). Canonical order: reals ascending, then conjugate pairs
>   (negative imaginary first), pairs by (re, |im|).
> - **§2d arithmetic**: power reduction in the canonical `pow` (`t^n mod p`
>   by square-and-multiply, negative powers through the inverse of `t`);
>   `p(RootOf(p,k)) → 0` falls out of reduction + like-term folding, verified
>   structurally. `eval_complex` evaluates `RootOf` from a thread-local
>   per-polynomial root cache, so every numeric `equals` stage works
>   unchanged. **Update 2026-07-19: the last §2d item is done** — the
>   `precise` tape gained an `Op::Root` leaf; real roots refine by dyadic
>   Newton with an exact sign-change certificate at ±ulp/4, complex roots by
>   CFix Newton accepted only under the rigorous `n·|p(z)/p′(z)|` bound
>   (`tests/precise_rootof.rs`, 7 tests: plastic number to 60 digits vs OEIS
>   A060006, `rootof(t²−2,1)` = √2 to 100 digits, conjugate-pair sums
>   resolving real, eigenvalue → 50 digits end-to-end).
> - **§3 eigenvectors**: nullspace of `A − tI` over ℚ[t]/(f) (dense reps,
>   ext-Euclid inverses); a zero-divisor pivot yields the discovered factor,
>   which splits `f` and restarts (bounded by deg p) — exercised by the
>   block-diagonal two-cubics test, where the char poly is a reducible
>   squarefree sextic. First nonzero component normalized to 1 in the ring;
>   rational eigenvalues ride the same path with deg f = 1. Defectiveness
>   visible as `basis.len() < alg_mult` ([[0,1],[0,0]] test).
> - **§5/§6**: `char_poly`/`eigenvalues`/`eigenvectors`/`EigenPair`
>   re-exported; wasm `determinant()`, `char_poly(var)`, `eigenvalues()`,
>   `eigenvectors()` (JSON, text-syntax values). No SymPy in the container,
>   so testing uses §6.2 self-verification as the primary oracle —
>   `A·v − λ·v` expands to the structural zero vector and `char_poly(λ) = 0`
>   through the library's own `equals` — plus hand-checked closed forms and
>   numeric index-order probes via `eval_complex`.
>
> Documented deviations: symbolic-entry eigenvalues answer 2×2 quadratic
> closed forms only (§8 Q1 resolved as planned); `max_isolation_bits` is a
> flat bisection budget rather than Mahler-derived (the budget is far above
> any Mahler bound at the capped degree/height); after a split-restart,
> `eigenvectors` may report a value as `RootOf` of a proper factor while
> `eigenvalues` reports the unsplit polynomial — different spellings of the
> same root, and `equals` agrees numerically.

### 1a. The non-commutative product segment (the one real invariant change)

`norm::mul` (`norm/mod.rs:271`) flattens, combines like powers, and **sorts**
factors — sorting `A·B → B·A` is wrong for matrices. Canonical form for a
product containing matrix factors:

```
Mul([ …scalar factors, canonical as today (sorted, coeff first)…,
      …matrix factors, ORIGINAL RELATIVE ORDER PRESERVED… ])
```

- Implementation: `mul()` partitions factors into scalar/matrix segments
  before its existing pipeline; the scalar segment goes through the current
  fold-sort-combine unchanged; the matrix segment keeps order. `cmp` already
  ranks `Matrix` last (rank 21, `norm/order.rs:40`), so the segmented layout
  is *almost* what sorting produces — the change is using a **stable
  partition instead of a full sort** so equal-rank matrix factors never
  reorder.
- Like-power combining within the matrix segment is only allowed for
  *adjacent equal bases* (`A·A → A²` fine; nothing ever commutes past a
  different matrix).
- **Folding**: adjacent literal `Matrix` factors with compatible dimensions
  multiply out symbolically — entries are `add(mul(...))` of entry
  expressions, so this is ordinary smart-constructor composition. Incompatible
  dimensions: the pair stays adjacent and unfolded (decision 2).
- `Add`: all-`Matrix` term groups with equal dimensions combine entry-wise
  (extends the existing like-term machinery: the "term key" for a matrix term
  is its dimension + non-scalar part); `scalar·Matrix` terms combine like
  coefficients (`2·A + 3·A → 5·A`).
- `Pow(Matrix, n)`: n ≥ 1 integer → binary powering via the product fold;
  `n = 0` on square matrices → identity matrix; `n = −1` → inverse (§1b)
  when the det is provably nonzero, else unevaluated; non-integer →
  unevaluated.
- `Neg`/scalar distribution into entries; `transpose`, `trace` as new ops in
  a new `src/matrix.rs` (public functions, not new Expr variants — they
  evaluate eagerly on literal matrices, return unevaluated `OtherOp` on
  non-matrices, mirroring `derivative`'s opaque fallback).

### 1b. Determinant, inverse, rref — tiered by entry type

| Entry type | Algorithm | Guard |
|---|---|---|
| All rational (`Num`) | Bareiss fraction-free elimination over `Number` | `limits.max_matrix_dim` (default 64) |
| Polynomial in ≤ few vars (§8 `expr_to_poly` succeeds) | Bareiss over the poly layer (`exact_div` exists, `poly/mod.rs:176` — Bareiss's exact divisions are its native operation) | poly `MAX_DEGREE`/size caps already in place |
| General symbolic entries | cofactor expansion + `simplify` per minor | n ≤ `limits.max_symbolic_det_dim` (default 6; n! terms) |

- `inverse` = adjugate/det for symbolic (small n), Gauss–Jordan with
  assumption-gated pivots (decision 4) for rational/poly entries.
- `rref`/`rank`/`nullspace` share the elimination core; every pivot
  decision flows through `is_nonzero(entry, assumptions)`; a `None` verdict
  aborts to the unevaluated form. `nullspace` is written once here and
  reused by eigenvectors (§3).

## 2. Layer 2 — abstract eigenvalues: the `RootOf` construct

### 2a. Representation

New **leaf** variant (no `Expr` children — coefficients are `Number`s, so
traversal, substitution, and canonical ordering treat it as an atom like
`Num`):

```rust
Expr::RootOf {
    /// Dense ℚ[t] coefficients, low→high, in canonical form:
    /// primitive integer coefficients, positive leading coeff, squarefree.
    poly: Box<[Number]>,
    /// Root index under the canonical ordering (§2c). Stable across sessions.
    index: u32,
}
```

- Canonical-form invariant makes structural equality of `RootOf` = semantic
  equality (same normalized poly, same index) — it slots into `order.rs`
  with a new rank between `Sym` and `Pow` and into `equals`' canonical fast
  path for free.
- Text form `rootof(t^3 - t - 1, 2)` (parseable back — one row in the
  function table); LaTeX `\operatorname{Root}_{2}\!\left(t^{3}-t-1\right)`.
- Why not `OtherOp`: `RootOf` needs arithmetic (power reduction §2d),
  numeric evaluation, and a canonical-order rank; a first-class variant makes
  the compiler enumerate every match site (the usual cost — ~20 match arms —
  paid once).

### 2b. Characteristic polynomial

`char_poly(A, var) -> Option<Expr>` in `src/matrix.rs` via
**Faddeev–LeVerrier**: `M₁ = A, c₁ = −tr A; Mₖ = A(Mₖ₋₁ + cₖ₋₁I), cₖ =
−tr(A·Mₖ₋₁ + cₖ₋₁A)/k`. Only ring operations plus division by integers —
works verbatim over exact rational entries, and over symbolic entries via
the Layer-1 product/sum (entries grow; bounded by `max_symbolic_det_dim`).
Rational-entry result lands in ℚ[λ] as a dense coefficient vector — exactly
`RootOf`'s food.

### 2c. Root pipeline: squarefree → extract → isolate → index

```
p ∈ ℚ[λ]
 ├─ squarefree decomposition: gcd(p, p′) chain            (new, ~40 lines on §8)
 │    → factors f₁·f₂²·f₃³… ⇒ algebraic multiplicities
 ├─ rational roots: divisors of a₀/aₙ, exact evaluation    (closed form, any degree)
 ├─ quadratic factors: quadratic formula                    (closed form via sqrt)
 └─ every remaining factor f, deg ≥ 3: eigenvalues are
      RootOf(f, 0), …, RootOf(f, deg f − 1)                 (abstract, exact)
```

**Canonical index ordering** (SymPy `CRootOf` convention): real roots
ascending, then complex roots by (re, im), conjugates adjacent. Assigning
indices requires **certified numeric isolation**:

- Real roots: Descartes/VCA bisection with Sturm-sequence counts (Sturm =
  signed pseudo-remainder chain — the §8 `pseudo_rem` already exists).
- Complex roots: f64 companion-matrix / Aberth iteration for initial guesses,
  certified by interval Newton at escalating precision (a direct consumer of
  the ARBITRARY_PERCISION_PLAN Tier-0 → Tier-2 ladder).
- **Termination is provable, so the limits story is clean**: for a squarefree
  integer polynomial, Mahler's bound gives a computable lower bound on root
  separation from the degree and coefficient height. Isolation at that
  precision *must* succeed — so `limits.max_isolation_bits` (default derived
  from the Mahler bound, hard-capped e.g. 64 k bits) is a genuine backstop,
  not a correctness gamble. Exhaustion (adversarial coefficient heights) →
  the eigenvalue API returns `None`/unevaluated, never a hang (§7f
  philosophy: operation counts, tri-state failure).

`eigenvalues(A, assumptions) -> Option<Vec<(Expr, u32 /*alg. multiplicity*/)>>`
returns closed forms where honest, `RootOf` elsewhere.

### 2d. `RootOf` arithmetic, simplification, equality, evaluation

- **Simplify rules** (new cluster in `norm/simplify.rs`): `p(RootOf(p,k)) →
  0`; power reduction — `RootOf(p,k)ⁿ` for `n ≥ deg p` rewrites by the
  precomputed remainder of `tⁿ mod p` (coefficients cached per poly). Sums
  and products of *the same* root reduce to a polynomial in that root of
  degree < deg p (i.e. arithmetic in ℚ[t]/(p)); *different* roots stay as
  opaque combinations (no resultant-based algebraic-number arithmetic —
  out of scope, same spirit as decision 6).
- **`equals`**: canonical fast path handles the exact case structurally
  (§2a). The numeric stages need `RootOf` to be evaluable: `eval_complex`
  gains a case that returns the k-th root numerically (isolation data cached
  on first use, thread-local like the Sym interner and diff's template
  cache). After that, every existing sampling/fuzzy stage works unchanged.
- **`evaluate_to_constant` / arbitrary precision**: same evaluation hook; at
  Tier 2 the root refines by interval Newton to the requested tolerance.

## 3. Layer 3 — abstract eigenvectors

For eigenvalue λ = `RootOf(f, k)` of A: eigenvectors = nullspace of
`A − λI` computed **over the quotient ring ℚ[t]/(f)**:

- Matrix entries become dense polys of degree < deg f (reduce after every
  multiply — the §2d power-reduction table again).
- Fraction-free (Bareiss) elimination avoids inverses almost everywhere;
  where a leading-coefficient inverse is needed, extended Euclid against f
  supplies it — and if the gcd is nontrivial (f reducible), that gcd is a
  *discovered factor*: split f, recompute indices, restart once (bounded).
- Nullspace basis normalized canonically: reduced echelon form, first
  nonzero component = 1. Components are polynomials in the abstract
  eigenvalue — e.g. companion-matrix eigenvectors come out `(1, λ, λ², …)`
  exactly.
- Rational eigenvalues use the same code path with deg f = 1 (plain ℚ).
- API: `eigenvectors(A, assumptions) ->
  Option<Vec<{ value: Expr, alg_mult: u32, basis: Vec<Vec<Expr>> }>>`
  (geometric multiplicity = `basis.len()`; defectiveness visible as
  `basis.len() < alg_mult`).

## 4. Limits additions (§7f style — all operation counts)

| Field | Default | Bounds |
|---|---|---|
| `max_matrix_dim` | 64 | any elimination/product loop |
| `max_symbolic_det_dim` | 6 | cofactor expansion (n! terms), Faddeev–LeVerrier on symbolic entries |
| `max_rootof_degree` | 64 | char-poly degree accepted into `RootOf` (matches poly `MAX_DEGREE`) |
| `max_isolation_bits` | Mahler-derived, hard cap 65 536 | root isolation/refinement precision |

## 5. Integration & API surface

- `src/matrix.rs`: `matmul`, `transpose`, `trace`, `determinant`,
  `matrix_inverse`, `rref`, `rank`, `nullspace`, `char_poly`, `eigenvalues`,
  `eigenvectors` — all taking `&Assumptions` where pivoting/nonzero gates
  apply, re-exported from `lib.rs`.
- Canonical-layer changes: `mul` segmentation (§1a), `add` matrix like-terms,
  `pow` matrix cases, `order.rs`/`eq` ranks for `RootOf`.
- Presentation: matrices render as today; `present` orders *entries*
  (each entry already flows through the §7e presentation pass); `RootOf`
  renders per §2a.
- wasm: `determinant()`, `eigenvalues()`, `eigenvectors()` on `Expression`,
  JSON-shaped returns (mirror the existing `variables()` pattern).

## 6. Testing

1. **SymPy oracle corpus** — `scripts/generate-matrix-corpus.py` (seeded):
   ~300 matrices (2×2…6×6; integer/rational/symbolic entries; companion
   matrices; defective cases; symmetric/rotation families) with SymPy's
   `charpoly`, `eigenvals` (incl. `CRootOf` prints), `eigenvects`, `det`,
   `rref` outputs. Same fixture + known-failures machinery as the existing
   corpora.
2. **The self-verifying property** (stronger than any oracle): for every
   corpus (λ, v): `simplify(A·v − λ·v)` must be the zero vector, and
   `simplify(char_poly(A)(λ)) == 0` — pure internal consistency using
   `equals`, valid even for `RootOf` answers (this exercises §2d reduction
   end-to-end).
3. **Property tests**: `p(RootOf(p,k)) → 0`; index stability (isolate at two
   precisions → same ordering); `det(A·B) == det(A)·det(B)` on random
   rational matrices; non-commutativity respected (`A·B ≠ B·A` canonical
   trees for a witness pair).
4. **Adversarial**: near-multiple roots (Mahler bound path), dimension-cap
   inputs, symbolic pivots with `None` verdicts → unevaluated returns, all
   op-count-bounded (no wall-clock, no in-container OOM repros — caps are
   validated arithmetically per the standing policy).

## 7. Phasing

| Phase | Deliverable | Exit criteria |
|---|---|---|
| **M1** (~2–3 d) | §1a canonical matrix arithmetic + `transpose`/`trace` + segmented `mul` | corpus arithmetic rows green; canonical-invariant property tests green |
| **M2** (~2 d) | §1b det/inverse/rref/rank/nullspace, tiered, assumption-gated | SymPy det/rref rows green; symbolic-pivot tri-state tests |
| **M3** (~3–4 d) | `RootOf` variant, squarefree + rational/quadratic extraction, isolation + indexing, `char_poly`, `eigenvalues` | eigenvalue corpus green; `CRootOf`-index agreement with SymPy |
| **M4** (~3 d) | quotient-ring elimination, `eigenvectors` | `A·v = λ·v` self-verification green across corpus |
| **M5** (~2 d) | `equals`/eval/`present`/wasm integration, docs, §17 disposition update | full suite green; wasm smoke rows |

M1–M2 are independently shippable (useful without eigen-anything). M3 has a
soft dependency on ARBITRARY_PERCISION_PLAN P1–P2 for certified isolation;
an interim f64-only isolator (companion matrix + interval check, `Unknown`
when uncertifiable at f64) lets M3 land first with a narrower certified
range.

## 8. Open questions

1. Should `eigenvalues` on symbolic-entry matrices (char poly in ℚ(a,b,…)[λ])
   attempt anything beyond deg ≤ 2 closed forms? (Plan: no — `RootOf` is
   ℚ-coefficients only; symbolic char polys return the polynomial itself.)
2. Vector/matrix bridging: should `Seq(Vector)` auto-coerce to n×1 `Matrix`
   in products (`A·(1,2)`)? (Plan: yes, coerce in `matmul` only, not
   globally.)
3. `RootOf` in student-facing *answers*: Doenet may want a display like
   `λ₂ ≈ 1.8393` alongside the exact form — presentation-layer question,
   deferred to the `present` pass once M3 exists.
