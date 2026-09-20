# Tracking signed zero (±0) so `1/-0 → -∞`

Investigation of what it takes to distinguish `+0` from `-0` in the exact model
so that division reports a signed infinity:

- `1/0 → +∞`  (already works)
- `1/-0 → -∞`  (wanted)
- `1/((-1)*0) → -∞`  (wanted — sign must flow through a product into the zero)

These are already encoded as **known failures** in the corpus:
`tests/fixtures/simplify-known-failures.json` lists `6/-0`, `-6/-0`,
`1/((-1)(0))`; `tests/fixtures/simplify-corpus.json` records the desired
`6/-0 → -Inf`, `-6/-0 → Inf`, and (crucially) `-0 → 0`.

## Current model (why it's +∞ today)

- Zero is only ever `Number::Int(0)` (or `Rat` num 0 / `Float(0.0)`). There is
  **no exact signed zero**. Multiple comments assert this as a deliberate
  invariant: `simplify.rs:254`, `simplify.rs:285`, `constructors.rs:160`.
- `a/b` canonicalises to `Mul(a, Pow(b, -1))`; a literal `1/0` becomes
  `Pow(Num(0), Num(-1))`.
- `is_zero_pole` (`simplify.rs:286`) recognises `Pow(0, negative)` and
  `fold_infnan_pow` (`simplify.rs:305`) folds it to `+∞` **unconditionally**.
- The numerator's sign is already handled correctly downstream:
  `fold_infnan_mul` (`simplify.rs:344`) computes the product sign, so
  `6·(+∞) → +∞` and `(−6)·(+∞) → −∞`. **So the only missing sign is the
  pole's own sign** — get `Pow(-0, -1) → -∞` and the rest composes for free.
- `(-1)*0` collapses in `mul`: the coefficient folds to `Int(0)` and
  `annihilate` (`constructors.rs:150`) returns a bare `Num(0)`, discarding the
  sign before the reciprocal is ever taken. `Neg(0)` canonicalises to
  `Mul(-1, 0)` and meets the same fate.

## Core difficulty: signed zero is *contextual* and *crosses node boundaries*

The sign is **created** in one node (`(-1)*0`, an inner `Mul`) and **consumed**
in another (the outer `Pow(_, -1)`), which are canonicalised in separate steps.
So the sign cannot be computed locally at the division site from the collapsed
denominator (it's already `0` by then) — it must be represented as a **value
that survives in the tree** between the two steps.

But the corpus also demands `-0 → 0`: a signed zero must be **invisible
everywhere except as the base of a negative power**. It must print as `0`,
serialize as `0`, and compare equal to `0`.

## Recommended approach: `Number::NegZero`, value-equal to `0`

Add an exact negative-zero that **is** zero for every purpose except a single
sign-reading predicate. This makes it safe to appear anywhere in a canonical
tree (no "leak" cleanup needed) because it is indistinguishable from `0` to all
consumers that don't explicitly ask for its sign.

### `num/number.rs`
- New variant `Number::NegZero` + `pub fn is_neg_zero(&self) -> bool`.
- `PartialEq` / `Hash`: **`NegZero == Int(0)`**, identical hash. (So
  `Expr::Num(NegZero) == Expr::Num(Int(0))`, dedup/like-term folding/HashMap
  keys all treat it as plain zero.)
- `is_zero() → true`, `is_negative() → false`, `is_positive() → false`
  (it is zero, not a negative number), `abs() → +0`, `to_f64() → -0.0`,
  `to_bigrational() → Some(0)`, `spelling() → Fraction`.
- Sign logic for a zero *result*: `mul` must compute the zero's sign as the
  XOR of operand signs — `(-3)·(+0) → -0`, `(-0)·(-0) → +0`, `2·0 → +0`.
  `neg`: `neg(0) → -0`, `neg(-0) → +0`. `add`/`sub`: minimal IEEE rules
  (`-0 + -0 → -0`, `-0 + +0 → +0`); low classroom impact, define for
  consistency. `checked_div`: `0/(-5) → -0`.
- **Mechanical fallout:** every exhaustive `match self { Number::… }` in
  `number.rs` (add/sub/mul via `binop`, `neg`, `magnitude_log10`,
  `round_to_decimals`, `rational_parts`, `checked_pow_int`, `to_f64`,
  `is_positive`, `is_negative`, `is_zero`, …) needs a `NegZero` arm. The
  compiler enumerates these; ~10 files hold `match` arms over `Number`, ~36
  files reference `Number::Int` (most are `matches!`/`if let`, unaffected).

### `normalize/constructors.rs`
- `annihilate` must **preserve the coefficient's sign**: return `Num(coeff)`
  (which may be `NegZero`) instead of hard-coded `Num(Number::zero())`, so
  `(-1)*0` yields `Num(NegZero)`. The indeterminate (`0·∞ → NaN`) branch is
  unchanged. Requires threading `coeff` into `annihilate` (both call sites at
  `constructors.rs:310,332`).
- `pow`: no change strictly required — `Pow(NegZero, -1)` stays an unevaluated
  node (like `Pow(0,-1)` today) and is folded in simplify. Optionally fold it
  directly to `Const(NegInf)` here.

### `normalize/simplify.rs`
- `fold_infnan_pow`: neg-zero base + negative exp → `NegInf`; plain zero →
  `Inf` (today's behaviour).
- `is_zero_pole` → add a sign-returning form (e.g. `zero_pole_sign() ->
  Option<Sign>`) so `fold_infnan_mul` / `fold_infnan_add` pick up the pole's
  sign instead of assuming `+∞`.
- Update the cluster doc comment (`simplify.rs:243-266`) — the "no signed zero"
  divergence is being removed.

### `ops/preserve_order.rs`
- The direct division fold (`preserve_order.rs:158-165`) computes `NegInf/Inf`
  from the numerator's sign only; incorporate `d.is_neg_zero()`.

### serde / print
- `expr/serde.rs:334` (`serialize_number`) needs a `NegZero => json!(0)` arm;
  confirm every printer path renders `NegZero` as `0`. (Because it's
  value-equal to zero, most paths that branch on `is_zero()` already do the
  right thing.)

### tests
- Remove `6/-0`, `-6/-0`, `1/((-1)(0))` from `simplify-known-failures.json`;
  keep/verify `-0 → 0`. Add: `1/-0 → -Inf`, `1/((-1)*0) → -Inf`,
  `1/(2*(-3)*0) → -Inf`, `0/(-5) → 0` (prints 0), `-0 == 0` (equality),
  `(-0)^2 → 0`, `1/(0-0) → +Inf`.

## Risks / caveats
- **"Equal-but-distinguishable" footgun.** A value that `== 0` yet carries
  hidden state is a classic trap. Contained here because the sign is read in
  exactly one cluster (pole folding) via `is_neg_zero()`; everywhere else it is
  literally zero. Note the F64 wrapper already takes the *opposite* policy
  (`+0.0 != -0.0`), so document the asymmetry.
- **Partial IEEE semantics.** Adopting signed zero for division but nowhere
  else can surprise: `1/(0*(-1)) → -∞` while `1/(0-0) → +∞`. Both match IEEE,
  but it's a philosophical shift away from the current "exact model has no
  signed zero" stance — worth a deliberate sign-off.
- **Verify no Number interning/caching** silently swaps a `NegZero` for a
  cached `Int(0)` before pole-folding runs (Sym is interned; Number appears not
  to be — confirm).

## Alternative (rejected): infinitesimals (signed ε)

Model an annihilated zero as a signed infinitesimal ε (ε > 0, −ε < 0), so
`1/ε → +∞`, `1/(−ε) → −∞`, and `(−1)·0 → −ε → −∞` all fall out of ordinary
arithmetic instead of special-cased pole folding. Elegant, but rejected:

- **Conflicts with the core requirement.** The corpus demands `-0 → 0`,
  `-0 == 0`, and the normalizer leans on `0·x → 0` / sum-absorption (~132
  `is_zero()` sites). An ε is *by definition non-zero*, so a faithful ε breaks
  all of these (`-ε ≠ 0`, `ε·x` doesn't drop `x`, `1 + ε ≠ 1`). The only fix —
  take the standard part everywhere except under a reciprocal — *is* the
  `NegZero` design above, but with a value that's harder to contain (every
  `is_zero()` consumer must standard-part it first).
- **Dual numbers can't do it.** With ε² = 0, ε is a zero-divisor and `1/ε` is
  undefined — no infinity. Needs a Levi-Civita / hyperreal-style *invertible*
  ε, i.e. a full ordered-field numeric tower (higher orders, truncation,
  ordering): far more than one `NegZero` variant.
- **Its extra power isn't wanted here.** Infinitesimals would enable limit
  evaluation (`sin(x)/x → 1`) and signed `0/0` with orders (`2ε/3ε → 2/3`),
  but the latter contradicts the deliberate `0/0 → 0`/NaN behaviour DoenetML
  relies on (`constructors.rs:143`). That's a separate limit-engine project
  (belongs with the calculus module), not a route to `1/-0 → -∞`.

## Alternative (rejected): local sign detection at the division site
Detect the denominator's sign structurally when building the reciprocal, without
a signed-zero value. Rejected: the sign genuinely crosses node boundaries, so
this is fragile for nested products (`1/(2*((-1)*0))`) and duplicates the sign
logic that `mul` already performs.

## Effort estimate
Small-to-medium. The design is well-scoped (single consumer: pole folding;
single new value: `NegZero`), and the corpus already pins the expected results.
The bulk of the work is the mechanical `match`-arm fallout in `number.rs` plus
careful handling of the `mul`/`annihilate` sign-preservation and the three
folding sites. ~1 focused day including corpus/tests.
