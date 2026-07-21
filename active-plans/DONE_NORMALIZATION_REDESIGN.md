# Normalization redesign note (Phase 4)

Status: **implemented in full — historical record** (updated 2026-07-19; was
"draft for decision"). Every §3 design point shipped as proposed: §3.4 went
with option B (`norm/order.rs`); §5's decisions resolved as recommended
(exact-first, JS-as-baseline — which produced REPORT_BUGS.md); §6's "later"
items are also done (finite field, simplify + assumptions, and
discrete-infinite-sets, which did NOT stay stubbed). One extension beyond
this note: the presentation layer (`norm/present.rs`, PORTING_PLAN §7e) that
converts canonical → display-faithful form for user-facing operation
results. Current state lives in PORTING_PLAN.md §7; this note remains as
the rationale record. Companion to PORTING_PLAN.md §7. Written under the
clean-slate mandate (do not reproduce the JS's structural/algorithmic
decisions; a better product is acceptable).

## 1. What normalization is *for*

Normalization is not a user-facing feature; it serves three consumers, in
priority order:

1. **Equality** (`equals`) — the central operation of this library. Two
   expressions are "equal" if they normalize to the same canonical form
   (fast path), falling back to numerical testing at random points.
2. **The polynomial layer** (§8) — needs a canonical, ordered form to extract
   coefficients and leading terms.
3. **Display** — a lightly-cleaned tree reads better (`x + -3` → `x - 3`).

The design must be driven by (1). Everything else follows.

## 2. How the JS does it (grounded)

`equals(a, b)` (lib/expression/equality.js) runs **both** operands through a
fixed pipeline, then compares:

```
a.evaluate_numbers({max_digits: Infinity})   // ALL numbers → float
 .normalize_function_names()                  // arcsin→asin, ln→log, …
 .normalize_applied_functions()
 .normalize_negative_numbers()                // -(…) pushed onto coefficients
 .normalize_angle_linesegment_arg_order()
 .remove_scaling_units()
 .simplify()                                  // large rewrite bag
```

then `equalsViaSyntax` — which is *not* a plain structural compare: it
re-normalizes names, then runs `tree_equal` with numeric fuzz
(`allowed_error_in_numbers`, optionally in exponents, relative or absolute)
and tuple/array/vector **coercion** (`(1,2)` can equal `[1,2]` and vector
forms). If that fails: `equalsViaFiniteField` as a **rejection filter**
(exact evaluation mod p at random points; a mismatch is a definitive *false*;
skipped when `allowed_error_in_numbers != 0`), then `equalsViaComplex`
(sample random complex points, compare within `relative_tolerance` /
`absolute_tolerance` / `tolerance_for_zero`), then discrete-infinite-set.
Note the finite-field and complex stages run on the **original** inputs, not
the normalized ones.

Canonical ordering (lib/trees/default_order.ts) builds a recursive `sort_key`
tuple per node — `[class, typename, value, …children]` — and compares them
lexicographically. Classes are hand-assigned integers (number=0, symbol=1,
function=2, product/quotient=4, sum/root=5, minus/pm=6, tuple=7, …).

`simplify.js` is a ~thousand-line bag of rewrites operating directly on the
ad-hoc array tree (`["*", …]`, `operands[0][0] === "*"`), leaning on mathjs for
numeric evaluation and on the assumptions system for `is_positive`/`is_negative`.

### What's wrong with it (why we diverge)

- **Float-flattening destroys exactness.** `evaluate_numbers({max_digits:
  Infinity})` converts every number to f64 *before* comparison, so
  `1/3 + 1/6` and `1/2` are only equal via floating-point coincidence, not
  structurally. This directly wastes the §3a exact-rational work.
- **Untyped tree, re-parsed everywhere.** Every rewrite re-discovers structure
  by string-matching heads and indexing (`operands[0][0] === "*"`). Our typed
  `Expr` already encodes that structure; a `match` is exhaustive and cheap.
- **Canonical form and simplification are entangled.** `simplify()` both
  canonicalizes (confluent, cheap, mandatory) and applies heuristic rewrites
  (root pulling, trig, log rules — expensive, non-confluent, optional). They
  have different correctness and performance profiles and should be separate.
- **The sort key is ad-hoc and allocation-heavy** — a fresh nested array per
  node per comparison, with quirky class collisions (sqrt shares class 5 with
  sums). A typed comparator needs no allocation.

## 3. Proposed Rust design

### 3.1 Two representations, not one mutated tree

`Expr` already distinguishes a **faithful layer** (parser output: `Div`, `Neg`,
`OtherOp`, unsorted) from a **canonical layer** (§5). Make that explicit:

- `canonicalize(&Expr) -> Expr` is a **pure faithful → canonical** function.
  Canonical form eliminates `Div`/`Neg` (→ `Mul(-1, …)`, `Pow(_, -1)`), flattens
  and sorts associative-commutative ops, and folds constants exactly.
- **Display keeps using the faithful tree** (the round-trip formatters already
  do). We never "un-normalize" for output; the two representations coexist.
- **Equality and the polynomial layer consume canonical form.**

This is the key structural break from the JS, which mutates one tree in place
and re-derives display forms repeatedly.

### 3.2 Exact-first arithmetic (the §3a payoff)

Constant folding uses exact `Number` arithmetic (`Int`/`Rat`/`Big`), never
float. `1/3 + 1/6 → 1/2` becomes a *structural* identity, so `equalsViaSyntax`
succeeds without the numerical fallback. Float only appears if the user's input
already contained an evaluation-produced `Float` (never from decimals). This
requires completing `Number` arithmetic (add/sub/mul/div/gcd, overflow
promotion) — the immediate prerequisite, JS-independent, proptest-backed.

### 3.3 Smart constructors maintain the canonical invariant

```
Number  add(&Expr…) mul(&Expr…) pow(base, exp) neg(&Expr)   // canonical builders
```

Each enforces: flatten same-op children, fold the numeric part exactly, drop
identities (`x+0`, `x*1`, `x^1`), annihilate (`x*0`), combine like terms
(`3x + 2x → 5x`) and like powers (`x²·x³ → x⁵`), then sort. Because each builder
returns canonical output, canonicalization is just a bottom-up rebuild.

### 3.4 A total order — and a real decision

Structural equality after canonicalization needs *a* total order; its
*meaning* only matters for display readability and the polynomial layer's
leading-term choice. Two options:

- **(A) Derive `Ord` on `Expr`.** Cheapest, zero-alloc, total. But the order is
  variant-declaration order + field order, and `Sym(u32)` sorts by *interner
  insertion order*, not alphabetically — fine within one equality check (both
  sides share the session interner), non-deterministic across runs, so not
  suitable for golden-output tests or readable display.
- **(B) Hand-written SymEngine-style comparator** — numbers < consts < symbols
  (alphabetical) < powers < products < sums < …, recursing on a typed key with
  no allocation. More code, but deterministic, readable, and what the poly
  layer wants.

Recommendation: **(B)**, but implemented as a typed `fn cmp(&Expr, &Expr) ->
Ordering` (no `sort_key` allocation), with symbols compared by resolved name so
ordering is stable across sessions. (A) is a valid MVP if we want equality
working before display/poly care about order.

### 3.5 Layering: canonicalize vs simplify vs equals

- `canonicalize` — confluent, cheap, mandatory, no assumptions. Always safe.
- `simplify` — heuristic, non-confluent, opt-in: root pulling (`√12 → 2√3`),
  `(xᵃ)ᵇ` flattening (only when exponent signs are known-safe), log/trig rules.
  Deferred; needs the assumptions system (§11) for the sign-conditional rules.
- `equals` — the staged algorithm of plan §10: canonicalize both sides and
  compare (stage 1); finite-field rejection (stage 2, ported — only
  discrete-infinite-set is stubbed per §17); complex sampling (stage 3) with
  the JS tolerance knobs.

The stage-1 comparator must reproduce two `equalsViaSyntax` behaviours beyond
plain `==` on canonical trees:

- **numeric fuzz**: `allowed_error_in_numbers` compares numeric leaves within
  a relative/absolute error (used for "answer within 1%" grading) — so the
  comparator takes options and only degenerates to `PartialEq` when all knobs
  are zero;
- **coercion**: `coerce_tuples_arrays` / `coerce_vectors` let `(1,2)`, `[1,2]`,
  and vector forms match. Cleanly expressible as a `SeqKind` equivalence-class
  function rather than the JS's scattered special cases.

### 3.6 Folding guards (partial functions)

Exact folding must not panic where the JS's float path produced `Infinity`
or `NaN`. `Number::rat` panics on a zero denominator by design, so the
folding rules guard:

- `x / 0`, `Pow(0, negative)` — leave **unfolded** (canonical form keeps the
  residual node; the numerical stages then behave exactly like the JS, which
  evaluates to `Infinity` and fails comparisons naturally).
- `0^0 → 1` (plan §7d, with warning), matching JS `Math.pow(0,0) === 1`.
- Folding a `Float` operand (evaluation-produced) uses f64 arithmetic; exact
  arithmetic applies only among `Int`/`Rat`/`Big`.

### 3.7 Name/form normalization (§7b)

Port the *tables*, not the code: `arcsin→asin`, `ln`↔`log` handling, applied-
function canonical shapes, negative-number pushing. These are data
(match arms), and belong in `canonicalize` since they are confluent and
assumption-free.

## 4. Divergences from JS to ratify

1. **Exact rational folding** instead of float-flattening in the equality fast
   path. *This is the substantive one.* Verdict impact is asymmetric:
   - Because the fallback stages (complex sampling) stay f64 with the JS
     tolerance knobs, exact folding mostly **adds fast-path accepts** — pairs
     the JS only accepted via sampling (`1/3 + 1/6` vs `1/2`) now match
     structurally. Same verdict, earlier and deterministic.
   - True verdict *flips* arise where exactness meets the **finite-field
     rejection**: e.g. `10^20 + 1` vs `10^20 + 2`. In JS both literals are the
     *same f64* before any comparison, so every stage sees identical inputs →
     `true`. With exact literals, stage 2 evaluates mod p exactly, detects the
     difference, and definitively rejects → `false`. Our answer is
     mathematically correct; the JS answer is representation slop. These are
     the cases the oracle decision (§5) is about.
2. **Canonical form is a separate pure derivation**; display stays on the
   faithful tree.
3. **Simplification split out** of canonicalization and deferred behind the
   assumptions system.
4. Numerical fallback keeps f64/complex sampling with the JS tolerance knobs,
   so behaviour on transcendental comparisons stays oracle-compatible for the
   differential harness (§15 Phase 7).
5. `evaluate_numbers` is also *public* JS API (used by `solve`,
   `transformation`, and downstream consumers). It is **not** replaced by
   `canonicalize`; the WASM wrapper will need a float-producing
   `evaluate_numbers` eventually — out of scope here, noted so the API surface
   isn't forgotten.

## 5. Open decisions for you

- **Ordering: (A) derive vs (B) hand-written comparator?** (Recommend B; A is
  an acceptable MVP.)
- **Equality exactness vs JS bug-compatibility.** Per §4.1, exact literals can
  flip specific JS verdicts from (wrongly) `true` to (correctly) `false` —
  precisely the cases where two distinct typed numbers collapse to one f64.
  Do we treat the JS `equals` as the oracle (match it, slop and all) or as a
  baseline we may legitimately improve on? Recommendation: **baseline** — the
  differential harness classifies these as *expected divergences* (it can
  detect them mechanically: project both inputs to f64 and re-check).
- **Scope of the first cut.** Minimal viable equality = complete `Number`
  arithmetic + `canonicalize` (flatten, fold, identities, order) + the
  options-aware stage-1 comparator + **basic complex sampling** (stage 3).
  Stage 3 is not optional garnish: `sin²x + cos²x = 1` and most
  transcendental identities pass *only* via sampling, so structural-only
  equality would regress badly vs the JS. That pulls a slice of the
  evaluation module (§9: `eval_complex` over the canonical tree) forward into
  this phase. Finite-field (stage 2) can land after; it only changes *speed*
  of rejection, not verdicts (except the exactness flips above).
  Trig/log/root `simplify` and the assumptions system come later. Confirm
  this slice.

## 6. Sequencing

1. Complete `Number` arithmetic (+ − × ÷, gcd, promotion) with proptest laws —
   JS-independent, unblocks everything. *(Start here regardless of decisions.)*
2. `cmp`/ordering (per decision A/B).
3. Smart constructors + `canonicalize` (flatten → names → fold with guards →
   order). Oracle: `tests/norm.rs` + the existing round-trip corpus
   (canonicalize must be idempotent: `canonicalize(canonicalize(e)) ==
   canonicalize(e)` — a cheap property test over the whole fixture corpus).
4. Stage-1 comparator (options-aware: numeric fuzz + Seq coercion).
5. `eval_complex` over canonical trees (minimal §9 slice) + stage-3 sampling
   with seeded RNG → first real `equals`.
6. `tests/equality.rs` (port the `equals` cases from the JS spec as the
   corpus; differential harness per the equality-oracle decision).
7. Later: finite-field rejection (stage 2), `simplify` heuristics +
   assumptions (§11), discrete-infinite-set (stays stubbed per §17).
