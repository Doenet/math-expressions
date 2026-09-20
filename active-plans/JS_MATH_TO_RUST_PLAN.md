# Moving the mathematics out of JS

**Status: done.** Phases A (`a5d1867`), B (`133ce6f`), C (`087c3ab`).
`lib/` went from **5,918 to 2,578 lines** (−3,340, −56%); the Rust core gained
~3,370. The js-compat suite moved 379 → **377 failing with zero newly-failing
names** at every phase boundary, and `cargo test` stayed green throughout.
`slow_polynomial` is 205/205 with no spec expectation touched, and runs in 0.16 s
instead of dominating the suite.

Outcome per phase, and what was learned, is recorded at the end of this file.

Goal: `packages/math-expressions-js-compat` becomes a binding shim. Every
algorithm lives in the Rust core. The only substantial JS allowed is glue that
exists because we *deliberately* chose a Rust API shape that needs reassembling
on the JS side — and each such case is named here with its reason.

## What counts as math

Not everything in `lib/` is math. Deep structural equality on arrays, unwrapping
an Expression to its AST, and JSON marshalling are plumbing and stay. The test
is whether the code decides a *mathematical* result: an ordering, a normal form,
a derived fact, a polynomial.

## Inventory

| module | lines | verdict | destination |
|--------|------:|---------|-------------|
| `lib/polynomial/polynomial.ts` | 1784 | math | Rust `polynomials/` — Phase C |
| `lib/polynomial/single-var-poly.ts` | 313 | math | subsumed by Rust `univariate.rs` — Phase C |
| `lib/assumptions/derive/*` | 471 | math | Rust `assumptions/derive/` — Phase B |
| `lib/assumptions/expand_relations.ts` | 173 | math | Rust `assumptions/expand.rs` — Phase B |
| `lib/assumptions/store.ts` | 183 | mixed | logic → Rust; handle bookkeeping stays — Phase B |
| `lib/assumptions/mutate.ts` | 156 | math | Rust `assumptions/store.rs` — Phase B |
| `lib/trees/default_order.ts` | 193 | math (150 of it) | already in Rust, just private — Phase A |
| `lib/expression/variables.ts` | 137 | math | already in Rust `ops/query.rs` — Phase A |
| `lib/assumptions/clean.ts` | 127 | math | Rust `assumptions/clean.rs` — Phase B |
| `lib/assumptions/linear.ts` | 113 | math | already in Rust `grade::solve_linear` — Phase A |
| `lib/assumptions/logical.ts` | 89 | math | already in Rust `simplify::push_not` — Phase A |
| `lib/expression/evaluation.ts` | 20 | shim | stays (already delegates) |
| `lib/trees/basic.ts` | 24 | plumbing | stays (deep-equal + substitute wrapper) |
| `lib/trees/util.ts` | 35 | plumbing | `subsets` dies with the polynomial code |

Roughly **3,400 lines of JS**, of which ~3,200 is math that should not be there.

## The pleasant surprise

Much of this is already written in Rust and merely unreachable:

- `ops::query::{variables, operators, functions}` — complete; only `operators`
  lacks a wasm binding.
- `normalize/default_order.rs` already carries a **faithful port of the legacy JS
  `sort_key`/`arrayCompare`**, quirks included. It is a private module fn with no
  binding. This is exactly `compare_function`.
- `grade::solve_linear` — stronger than the JS version (which recovers
  coefficients by evaluating `f(0)` and `f(e_v) − f(0)`); not exposed.
- `normalize::simplify::push_not` — De Morgan + relation negation; reachable only
  through `simplify_logical`, which *also* canonicalizes each relation and so
  reorients `x > a` into `a < x`. That reorientation is the whole reason
  `logical.ts` reimplemented it.
- `polynomials/ratform.rs` has **kernel interning** (`Kernels`/`kernelize`,
  private): maximal non-rational subtrees — `sin(x)`, `x^(1/2)`, `π` — replaced by
  fresh `$k0`, `$k1` symbols, deduped by structural equality. This is precisely the
  "arbitrary tree as an opaque polynomial variable" facility the JS engine builds
  with `stringify_vars`, already written.

So Phase A is mostly *exposure*, not implementation.

## Phase A — expose what exists

New bindings (pattern: free fn, JSON in / JSON out, `expr::serde::try_from_js`
and `to_js`, per `interop.rs:229`):

| binding | backs | deletes |
|---------|-------|---------|
| `operators(tree_json)` | `ops::query::operators` | `variables.ts` (137) |
| `cmp_default_order(a_json, b_json) -> i32` | make `default_order::{sort_key, cmp_key}` `pub`; new `pub fn cmp_default_order` | `default_order.ts` sort_key/compare_function (150) |
| `solve_linear(tree_json, var)` | `grade::solve_linear` | `linear.ts` (113) |
| `linear_decomposition(tree_json, vars_json)` | new thin wrapper over the same coefficient extraction | — |
| `push_not(tree_json)`, `flatten_logical(tree_json)` | `normalize::simplify::push_not`, `default_order::flatten` (make `pub`) | `logical.ts` (89) |

`cmp_default_order` must expose the **legacy** key (`default_order.rs:565`), not
`normalize::order::cmp` — the latter is documented as deliberately
non-JS-compatible, and the polynomial variable order depends on the legacy one.

Risk: `push_not` without canonicalization is a *new* entry point. Verify it does
not reorient relations; if it does, the not-pushdown needs its own path.

## Phase B — port the missing assumptions logic

Nothing in Rust does relation expansion or transitive closure today.

- `src/assumptions/expand.rs` — chained inequalities, interval membership, and
  interval containment into `and`/`or` of two-sided comparisons. The endpoint rule
  (closed small end inside an open big end is the sole strict case) and the De
  Morgan flip on negation carry over verbatim. `ops::to_intervals` already does
  the tuple/array → `Expr::Interval` half.
- `src/assumptions/derive/` — the `COMBINED_OPERATOR` composition table
  (`<`∘`≤` ⇒ `<`; `in`∘`subset` ⇒ `in`; the pairs that compose to nothing) and the
  closure that chains stored facts.
- `src/assumptions/clean.rs` — canonical form for a stored fact.
- Widen the wasm `Assumptions` class: `add` currently takes **text syntax**;
  `get`, `add_generic`, `remove_generic` are unexposed. Add AST-JSON variants.

`store.ts` keeps only the handle/JS-object bookkeeping — that is genuine glue
(the JS API hands back trees; the Rust store answers predicates) and is named
here as an accepted exception.

## Phase C — polynomials

Decision: back the compat API with the **existing** Rust engine rather than
re-porting the legacy one. `multivariate.rs` already has recursive-dense ℚ with
content/PRS GCD; the JS engine computes the same GCDs the heavyweight way, via
Buchberger plus ideal intersection (`lcm(f,g)` from a Gröbner basis of
`⟨t·f, (1−t)·g⟩`, then `gcd = f·g/lcm`).

1. `src/polynomials/kernels.rs` — lift `Kernels`/`kernelize` out of `ratform.rs`,
   make it `pub(crate)`, switch the linear `Vec` dedupe to `HashMap<Expr, String>`
   (`Expr` is `Eq + Hash`).
2. `src/polynomials/groebner.rs` — **genuinely new**: monomial order, S-polynomials,
   Buchberger with a pair queue, reduced basis. This is the one piece with no Rust
   counterpart.
3. Compat bindings: `poly_gcd`, `poly_lcm`, `poly_reduce_rational`,
   `poly_groebner`, `expression_to_polynomial`, `polynomial_to_expression`.
4. `lib/polynomial/polynomial.ts` → ~40 lines of wrappers; delete
   `single-var-poly.ts` (Rust's `univariate.rs` covers the fast path).

Spec consequence: ~115 assertions test mathematical results and survive
unchanged. ~90 test internal intermediates — `mono_less_than`, `hij` tables,
`poly_div` quotient lists, the `["polynomial", var, [[deg, coeff]]]` AST shape —
and get rewritten to test through the real API. Per the standing instruction,
legacy behaviour is not preserved where the result stays mathematically correct.

Note: coefficients are **not** rationals. `expression_to_polynomial` treats `pi`,
`e`, `i` as numbers, so `9x^(2/3) - pi*x` yields the coefficient `["-", "pi"]`.
The Rust side must carry `Expr` coefficients, not `BigRational`.

## Sequencing & verification

Phases are **sequential**: each rebuilds `vendor/wasm`, which would move under a
concurrently-measuring sibling.

- Baseline: **380 failing**, captured by name in `scratchpad/baseline-names.txt`.
  Judge by name-level diff, never aggregate counts (memory
  `js-compat-suite-baseline-diff`).
- `cargo test` green after every Rust phase. `tests/assumptions.rs` and
  `assumptions_corpus.rs` validate against the JS oracle; a "smarter" rule that
  contradicts them is a regression.
- Success is measured in **JS lines deleted** with the failure set no worse.
- Split any Rust file past ~200 lines into a subfolder (memory
  `rust-file-organization`).

---

## Outcome

### Where the plan was wrong

- **`ops::query::operators` was not a faithful port**, though it looked complete.
  It reported only a fixed operator whitelist rather than every array head. Since
  `expression_to_polynomial` whitelists operators, an unreported `tuple` made
  `(3,4)` parse as a polynomial. Fixed and pinned with assertions.
- **`ops::to_intervals` could not be reused** for `expand_relations`: it recurses,
  while the JS converts only the top level — a nested 2-tuple endpoint is an
  endpoint, not an interval.
- **`ratform::cancel` could not back `reduce_rational_expression`.** On
  `(t^100 − t)/t` it returns `t^(-1)(t^100 − t)` rather than cancelling; it hands
  back a product with a reciprocal instead of a numerator/denominator *pair*, so
  the "denominator leading coefficient is 1" rule has nothing to apply to; and
  being dense over ℚ it cannot represent the `t^1000000` rows at all.
- **Kernel interning was not worth lifting.** It only pays off for an engine whose
  variables are `String` names. The compat engine's variables are `Expr` trees
  compared structurally, so `sin(x)` is already one variable — which is also why
  `stringify_vars`/`destringify_vars` vanished rather than moved.
- **The `push_not` reorientation risk did not materialise.** `simplify.rs` rebuilds
  `Expr::Relation` from the same operand vector with only `ops[0].negate()`
  changed, so a standalone binding was safe. Pinned by a test.

### Semantics that had to be preserved rather than tidied

- `grade::linear_decomposition` must **reject exact fractions**, matching the JS
  `typeof tree === "number"`. The rejection is load-bearing: when decomposition
  fails, `get_assumptions_for_expr` falls back to per-variable facts, and that
  fallback is where transitive consequences come from.
- `byvar` is **insertion-ordered, not a map** — the closure iterates it to build a
  conjunction and the key order reaches that tree before `default_order` sorts it.
- A variable **met with no facts differs from one never seen**: only the latter
  picks up the generic assumption (`Facts::{Absent, Empty, Tree}`).
- The closure reads the *previous* `derived` map while recomputing it, and
  `clean_assumptions` can return a childless `["and"]`. Both are observable
  through `me.assumptions.derived`.

### Deliberate divergences

- Three JS warts were not reproduced, none reachable from the spec: multiplying
  zero by a polynomial read `p[1]` off a number; negating a zero polynomial
  returned `false`; `mono_gcd` on two variables the default order cannot separate
  advanced neither index and looped forever.
- A generic assumption that cleans to nothing is now a no-op; the JS left
  `store.generic` undefined so the *next* `add_assumption` threw a `TypeError`.

### Open items

- `lib/assumptions/linear.ts`, `lib/assumptions/logical.ts` and
  `lib/expression/variables.ts` are now shims with **zero importers**. They were
  left in place because `package.json` maps `"./lib/*": "./lib/*"`, so every file
  under `lib/` is a public deep-import path and removing one is an API change.
- `Expression.expand_relations()` is still a no-op stub. `assumptions::expand_relations`
  is now `pub` and would back it directly, likely flipping the two failing
  `quick_transformation.spec.ts :: expand relations …` names.
- `lib/polynomial/polynomial.ts` remains reachable only from its own spec;
  `expression_to_polynomial` is now wired up on `Expression` but nothing calls it.
