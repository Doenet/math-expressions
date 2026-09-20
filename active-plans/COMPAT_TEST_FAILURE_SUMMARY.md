# js-compat test failure summary

Snapshot of `packages/math-expressions-js-compat` (`npx vitest run`) on branch
`doenet`, after rebuilding `vendor/wasm` (`bash build-wasm.sh`) so results reflect
current Rust source.

**Current totals (2026-08-14, at `41b9cb4`, re-measured):
0 failed / 6341 passed / 6352 total** (11 skipped, 0 todo). The suite is green.
The eleventh skip is `slow_assumptions` → `logical combinations`, where **legacy
commits to answers that are mathematically false and this engine declines to**
(see below); it was carried as the one red test until the ninth review pass,
which skipped it at its site with the reason and dropped the `|| true` from the
CI job, so a *new* failure can now be reported. Nothing about the divergence
changed — only whether the check is capable of checking.
Previous snapshots: 380, 162, 97, 83, 82, 61, 55, 54, 46, 43, 38, 20, 16, 12, 11, 7.
(The dated 2026-08-11 line this replaces read `1 failed / 6316 passed / 6329
total`, 12 skipped and 2 todo; that was a historical snapshot at `7082f8a`, and
the suite has grown tests since.)

Both `*-numerical-errors` files are at **zero**, and so are `slow_simplify` and
`slow_rational`.

Ten of the eleven skips are **wontfix, not pending**: nine are the deprecated
`match` conditions under `quick_trees` below — read the 54 → 46 step as a
reclassification, not nine defects fixed — and the tenth is `logical
combinations`. The eleventh, `define constants`, is skipped for a reason of its
own recorded at the test.

## How to measure

This snapshot is reproducible from a clean tree: run `bash build-wasm.sh` then
`npx vitest run`, both from inside `packages/math-expressions-js-compat`.
Rebuild the wasm first or the run measures the previous engine; that alone
accounted for a 4-test discrepancy while an earlier snapshot was being taken,
and `build-wasm.sh` lives in the package, not at the repo root.

Judge changes by name-level diff against a baseline worktree, never by aggregate
counts (see memory `js-compat-suite-baseline-diff`). When diffing, note that
several spec files contain **duplicate test names**, so keying a comparison by
name alone silently drops results — key by (file, name, occurrence). Some specs
also derive the test *name* from the expected string, so updating an expectation
renames its test; those show up as removed-plus-added rather than failed→passed,
and the honest check is that no name present in *both* runs went passing→failing.

**A matched pair means the two runs differ by the change set and nothing else.**
On a tree someone else is editing, a baseline goes stale in minutes; capture both
halves back to back or the diff is measuring the other person. Two past
attributions were confounded exactly this way — one over-claimed another change
set's fixes, and one made a resource guard look like a correctness fix.

A further caution: `slow_simplify`-style tests bundle 5–20 assertions each, so a
per-test bucket count is an upper bound on what any one fix buys. Attributing a
test to the first assertion that fails hides everything behind it.

## Remaining divergences by spec file

| count | spec file                                       | root cause / category                                                                    |
| ----: | ----------------------------------------------- | ---------------------------------------------------------------------------------------- |
|     1 | `slow_assumptions`                              | one skipped test, `logical combinations` — legacy answers unsoundly where we decline, see below |

The six other failures were feature gaps, now closed (see "Feature gaps
closed" below): `quick_trees` (`allow_extended_match`, graceful invalid match
conditions) and `slow_math-expressions` (container/union coercion flags, an
integer-assumption equality, the `nthroot` derivative). Nine `quick_trees`
predicate/`RegExp` match tests stay **wontfix**-skipped.

`quick_solve` is at zero, and its last failure was fixed at the root rather than
adopted as a divergence — see below.

### `simplify` now has one fixpoint per value, not one per spelling

`-3y-v <= 2xz+r` solved to `(-2xz-r-v)/3` where alpha94 gives `-((2xz+r+v)/3)`.
Same value, but *both* were fixpoints of `simplify`, so which one came out
depended only on how the input was spelled. Idempotence was never the issue
(`simplify∘simplify == simplify` held throughout, asserted in
`simplify_corpus.rs`); **confluence** was.

The cause was two rules that disagreed, keyed on something mathematically
irrelevant — the *magnitude* of the coefficient:

- `rule_distribute_neg_over_sum` fired only for a coefficient of exactly `−1`
  over a lone sum, and distributed **unconditionally**.
- `rule_distribute_sign` fired for any negative coefficient, but only when it
  reduced the sign count.

So `-(x+y)` distributed and `-2(x+y)` did not, and `-(a+b)/3` — whose canonical
coefficient is `−1/3` — took the second branch. Meanwhile `(-a-b)/3` has a
*positive* coefficient, so neither rule looked at it. Both stood still. More
generally: the engine only ever pushed signs **into** a sum and never pulled one
out, so any input already spelled the disfavoured way was a fixpoint by default.
A preference expressed by only one of the two rewrites is not a normal form.

**The fix** is the converse rewrite, `rule_factor_sign_out_of_sum`
(`normalize/simplify.rs`): a sum of `k` terms with `n` negated factors its `−1`
out iff `2n ≥ k + 2`. That complements `rule_distribute_sign`'s push-in
threshold exactly — with the tie there moved from "decline" to "push in", which
is load-bearing rather than cosmetic. Writing the factored spelling's negated
count as `m = k − n`:

- distributed is stable iff `2n ≤ k + 1`;
- factored is stable iff `2m ≤ k − 2`, i.e. `2n ≥ k + 2`.

Exact complements, so precisely one of the two spellings is stable for every
sum: never both (two fixpoints — the bug) and never neither (a ping-pong).
Moving either threshold by one re-opens one of those. The tie cases (`k` odd,
`2n = k+1`, e.g. `-x-y+z` vs `-(x+y-z)`) are what a paper argument gets wrong
first; they are pinned in `tests/simplify_sign_fixpoint.rs`.

The old unconditional distribution survives, narrowed, as
`rule_flatten_negated_sum_term`: a negated sum is spliced into its parent only
when it is a *term of a larger sum*, which is the only position where terms can
meet and cancel. That preserves `(q + 12 - (q+2))/2 → 5`, the `<lineSegment>`
midpoint shape that motivated the original rule.

**Cost: one row.** `-(x+y)` now stays factored where alpha94 distributes it to
`-x-y`. That row is unavoidable — `-(x+y)` costs one sign and `-x-y` costs two,
so every sign-counting rule prefers the factored form, and alpha94 prefers the
other only because it never consults a count. Keeping alpha94's answer would
require the distributed spelling to stay a fixpoint, which is precisely the bug.
Recorded as `DIVERGENCE (adopted):` in `tests/doenet_sign_distribution.rs`.

**Measured: 0 net js-compat change** (11 failures, identical by name), 77 cargo
suites green, clippy clean. The `quick_solve` expectation reverted to alpha94's
spelling and now passes with no divergence note, and `-(x+y-z)` — a tie —
still matches alpha94 because ties push in.

Two things fell out. `(-a-b)/(-c-d)` now reduces to `(a+b)/(c+d)`; it did not
before, in this engine **or** alpha94, because with no numeric coefficient
anywhere neither sign rule could see it. And the cost table at
`simplify.rs`'s sign cluster was wrong about `-(x+y)` — it claimed "unchanged"
while the unconditional rule preempted it and distributed. Both are now correct
and covered.

### `slow_assumptions` — 4 of 5 fixed, 1 where legacy answers unsoundly

The five test *names* here were hiding **52** failing assertions (each `it`
aborts at its first failure; re-run with `expect` → `expect.soft` to see them
all). Four of the five tests are now green. There is no separate plan document
for this — the writeup is the list below, and the tests are
`packages/math-expressions-rs/tests/assumptions_sound_reasoning.rs`. In brief:

- `is integer / via assumptions` (**fixed**) — the text printer lost negation
  scope on the assumption round trip: `paren_if_spaced` tested
  `starts_with('(') && ends_with(')')`, which cannot tell `(a or b)` from
  `(a) or (b)`, so `not(p or q)` re-parsed wrong. Replaced with a paren-balance
  scan (`src/print/text.rs`, `latex.rs`).
- `strict pow` (**fixed**) — `pow_strict` is now a `ConstantPolicy` field
  (default strict), reached from JS as `me.math.pow_strict` via a
  `defineProperty` that routes to `set_constant_policy`. Off, `x^0 → 1`
  unconditionally.
- `combined assumptions`, `combined assumptions, negated` (**fixed**) — the
  predicate engine now reads the *derived* store (equality following, bound
  chaining, disjunctions) instead of the flat one, and walks its boolean
  structure with sound semantics (`and` = meet, `or` = join). See
  `src/assumptions/infer/vars.rs` and `assumptions_sound_reasoning.rs`.
- `logical combinations` (**still failing — accepted**). 18 of its 24 failing
  assertions were fixed by the change above. **6 remain**, measured by
  converting that one `it`'s `expect` to `expect.soft` (revert the scaffolding
  afterwards): spec lines **7357, 7415, 7417, 7418, 7419, 7420**. Note the
  direction — on every one of them **legacy commits to an answer that is
  mathematically false, or that we simply cannot yet prove, and this engine
  declines instead**. It is *incomplete* here, never unsound. Two unrelated
  root causes:
  - **(a) contradictory premises** (1 assertion, spec:7357).
    `x ∈ R and x ∉ R ⟹ is_real(x)` — legacy returns `true`, because its `and`
    is `left || right` and the first conjunct wins. No `x` satisfies both
    premises, so `true` is not entailed by anything; `Facts::and_meet`
    (`src/assumptions/facts.rs`) meets the two definite answers to `None` and
    we return `undefined`.
  - **(b) non-realness does not propagate through an operator** (5 assertions,
    spec:7415 and 7417–7420). Under `x ∈ C, x ∉ R, y ∈ R`:
    - `is_real/nonpositive/nonnegative(xy)` — legacy returns `false`, which is
      **wrong**: `y = 0` is a model of the premises, and there `xy = 0`, which
      *is* real, nonpositive and nonnegative. We answer `undefined`.
    - `is_positive/negative(xy)` — here `false` *is* sound (a value that is
      non-real-or-zero is never positive or negative), and we still answer
      `undefined`, because `combine::mul` in
      `src/assumptions/infer/combine/mod.rs` never carries a `real: Some(false)`
      operand through the product and so cannot reason by cases. Pure
      incompleteness on our side.

    **Deliberately not fixed.** The rule that would close the sound half —
    propagating non-realness through `+`/`*`/`^` — turns facts that are
    `None` today into `Some(false)`, and `simplify`'s rewrites are gated on
    exactly those facts. More definite answers means different rewrites, which
    means different grading on DoenetML's answer path. That is not a trade
    worth two assertions in a test that stays red either way (the three
    genuinely-unsound legacy answers above are unreachable without adopting
    legacy's unsoundness). Recorded as a known gap in
    `ASSUMPTIONS_ENGINE_PLAN.md` § "Accepted divergence"; left unfudged rather
    than asserted as intended behavior.

### Feature gaps closed

Six failures that were genuine feature gaps (not divergences) are now fixed —
each with a permanent test (`tests/missing_features.rs` for the core ones, the
spec files for the wasm/JS ones):

- **`nthroot` derivative** (`slow_math-expressions`). `nthroot(x,k)` denotes
  `x^(1/k)` but had no derivative-table entry, so on the faithful layer it left
  a formal `nthroot'`. `calculus/diff.rs` now rewrites it to the power form and
  differentiates that (as `sqrt`/`cbrt` already did).
- **Integer-assumption equality** (`slow_math-expressions`). The numeric
  equality sampler was assumption-blind, so `(-1)^n·(-1)^n = 1` failed under
  `n ∈ Z`. `EqOptions` gained an `assumptions` field; the `Assumptions` wasm
  handle now passes its store to `equals`, and the sampler draws an
  integer-proved variable over the integers (JS `integer_variables`).
- **Container coercion flags** (`slow_math-expressions`, two tests). The flags
  existed but were mis-wired: `coerce_seqs` gated tuple↔vector on the wrong flag
  (fixed to a two-step map matching legacy's non-transitive graph), and union
  equality did no member matching (added an accept-only pairwise-matching stage
  on raw members, so each pair coerces in isolation — a tuple can pair with a
  vector and a closed-interval-spelled array in the same union).
- **Graceful invalid match conditions** (`quick_trees`). `trees.match` threw on
  an unusable `variables` condition (`false`, an unknown kind string, a
  `RegExp`); it now maps them to `VarKind::Nothing` (admits nothing), so the
  match fails gracefully to no-match.
- **`allow_extended_match`** (`quick_trees`, `trig transformation`). A sum/
  product pattern can now match a subset of a larger sum/product. Implemented in
  the JS `match` bridge (`lib/trees/flatten.ts`): the tree operands are put in
  canonical order, operand subsets are matched by the existing Rust matcher, and
  the untouched operands are returned as `_skipped` for the (already-present)
  splice in `applyAllTransformations`.

There is also one **skipped** test here, `define constants`, which the legacy
suite skipped with the note "although this passes, skip test as setting
`define_i`, etc., no longer changes mathjs". The Rust port *does* thread that
declaration all the way through (`src/constant_policy.rs`, surfaced as
`me.setConstantPolicy`), so this test is nearly enableable: 13 of its 14
assertions pass as written. The one that does not is `is_real(i·x)` given
`x ∈ ℝ`, which answers `undefined` where the spec wants `false` — a missing
inference (a product of a nonzero real and the imaginary unit is not real), not
a declaration problem. Fix that and the test can be un-skipped.

### `quick_trees` — 0 left, 9 wontfix

#### WONTFIX — arbitrary per-parameter `match` conditions (9 tests, now skipped)

Legacy let `variables` map a parameter to a **predicate function** (8 tests) or
a **`RegExp`** (1). [`VarKind`] is the closed replacement and the open forms are
deprecated; the specs are `it.skip`ped with a `[wontfix: …]` name prefix and a
block comment, so they stay as the record of what legacy accepted rather than
sitting in the failing bucket forever.

Two reasons, and the second is the real one:

1. A predicate would be called back into JS once per **candidate** binding —
   the matcher backtracks, so the call count is a function of the search, not of
   the input. It would stop being a pure Rust search.
2. **Nobody needs it.** DoenetML's `<matchesPattern>`
   ([`MatchesPattern.js:259-274`](../tmp/DoenetML/packages/doenetml-worker-javascript/src/components/MatchesPattern.js#L259-L274))
   is the only real consumer and passes exactly two closures —
   `(m) => !Number.isNaN(me.fromAst(m).evaluate_to_constant())` under
   `requireNumericMatches`, `(m) => typeof m === "string"` under
   `requireVariableMatches`. Those *are* `VarKind::Number` and
   `VarKind::Variable`. No `RegExp` condition appears anywhere in DoenetML.

The declarative form is also sharper: `Number` means "evaluates to a real
numeric constant", where the legacy specs' hand-written `typeof s === "number"`
quietly rejected `π`.

Cost of the skip, and how it was covered: every legacy test exercising
`allow_permutations` and `allow_implicit_identities` *also* declared its
parameters with predicates, and those two options are supported and are what
Doenet passes. Skipping blind would have left both with **zero** coverage in the
suite. Two replacement tests ("… with parameters declared by kind") re-express
the same scenarios with declared kinds, including a negative control confirming
the kind is what fails the match (`e^(0.3s^2+3s+q)` matches under `true`, not
under `"number"`).

#### Both formerly-open tests now fixed

- **Graceful invalid match conditions** — "invalid matching conditions fail
  gracefully". `interop.rs` now maps an unusable condition (`false`, an unknown
  kind string, a `RegExp`) to `VarKind::Nothing` (admits nothing), so the match
  fails to no-match rather than throwing. See "Feature gaps closed" above.
- **`allow_extended_match`** — "trig transformation". Implemented in the JS
  `match` bridge (`lib/trees/flatten.ts`), reusing the Rust matcher over operand
  subsets and feeding the existing `applyAllTransformations` splice via
  `_skipped`. See "Feature gaps closed" above.

## Highest-leverage remaining item

**`slow_assumptions` → `logical combinations`** is the only divergence left, and
it is **accepted, not a bucket of work** — skipped at its site rather than left
red, so the suite's exit status is available to report regressions. Of its six failing assertions, three want
answers that are mathematically false (`y = 0` is a model, so `xy` really can be
real/nonpositive/nonnegative) and a fourth is legacy's first-conjunct-wins `and`
answering under a contradiction — reproducing any of them means encoding unsound
reasoning. The remaining two are a completeness gap we decline on purpose,
because closing it would move `simplify` and therefore grading (see that
section). This engine is *incomplete* on that test, never unsound. Every other
spec passes.

Two things worth doing that no failing test covers:

- **`equality/fuzzy.rs` is ~330 lines** and has an obvious seam (structural
  equality vs. the sensitivity tolerance) that the file's own module doc already
  names. Over the ~200-line split guideline.
- **`ops/numbers.rs` is ~640 lines** and holds two unrelated passes: numeric
  folding (`evaluate_numbers` and the rounding family) and the polynomial-GCD
  fraction cancellation (`reduce_rational`/`reduce_node`). The second reaches
  into `polynomials::kernel` and owns its own resource cap, which makes the seam
  wide. Splitting `reduce_rational` into its own module under `ops/` would leave
  both halves under the guideline.

## Rejected: folding roots to powers in `canonicalize`

Recorded so it is not relitigated. `ops/transforms.rs:122` records keeping
`sqrt(x)` and `x^(1/2)` as distinct canonical trees as a deliberate decision, and
the oracle backs it — legacy also keeps them distinct in `.tree` and in printed
output, folding to a power only inside the explicit `normalize_function_names`
pass:

```
legacy  fromText("sqrt(q)").tree  ->  ["apply","sqrt","q"]    prints sqrt(q)
legacy  fromText("q^(1/2)").tree  ->  ["^","q",["/",1,2]]     prints q^(1/2)
```

Making `canonicalize` fold roots would break `q^(1/2)` round-tripping (the two
become one tree and must therefore print alike), move roots from the `Apply` to
the `Pow` rank in both comparators, cost `sqrt(8) → 2√2` unless
`rule_radical`'s numeric-base-only `Pow` arm is generalized, drop `sqrt` off its
dedicated `z.sqrt()` / `FixId::Sqrt` kernels onto generic `powc` (branch-cut
risk), and require regenerating ~130 fixture entries.

A note for whoever picks these up: several past fixes turned out to be *bindings*
for engine code that already existed and was already exercised elsewhere. Before
implementing anything that looks like a missing feature here, grep the Rust core
for it first.
