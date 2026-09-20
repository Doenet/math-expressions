# Stack-safety plan: iterative traversals for a small-stack WASM target

> **PROGRESS (re-audited 2026-08-04):**
>
> - **21 (iterative `Drop`) — DONE.** `expr/teardown.rs`, a `Vec<Expr>`
>   worklist, wired into `impl Drop for Expression` at the wasm boundary
>   (`math-expressions-rs-wasm/src-rust/lib.rs`). Deviates from the plan
>   deliberately: a free function `tear_down`, not `impl Drop for Expr`, since a
>   `Drop` impl makes every by-value `match` destructure an E0509 error.
> - **22 (parser depth caps) — DONE.** `MAX_PARSE_DEPTH = 64` in
>   `parse/common.rs`, enforced by `enter`/`leave` in both parsers, exercised by
>   `tests/stack_safety.rs`; `from_js` documents its reliance on serde_json's
>   128-depth limit.
> - **26 (verification) — PARTIAL.** `tests/stack_safety.rs` has both the
>   10⁵-deep-paren test and a small-stack test (at 256 KB, not the planned
>   128 KB). What remains is the `-zstack-size` flag and its documentation;
>   there is no `.cargo/config.toml` in the repo, so `build-wasm.sh` is where it
>   would go.
> - **23 — HALF DONE.** `Expr::children()` and `map_children` already exist in
>   `expr/visit.rs` and are used in 27 files. What is missing is only the
>   iterative driver itself: no `fold`, no `Step`/`Prune`, no explicit-stack
>   traversal anywhere except `teardown::tear_down`. Note the two existing
>   helpers are by-reference; the by-value gap is why `flatten` still hand-rolls
>   a full variant match *in the same file*, and a `drain_children`/
>   `map_children_mut` is the missing third member of the family.
> - **24, 25 — OPEN, and the real remaining exposure.** No pass is iterative;
>   `opaque_key` is unchanged.
>
> Two corrections to §2 below, which is optimistic:
>
> - The recursion inventory is not ~10 functions. Counted against source it is
>   **~90 in the core crate plus 4 in the wasm crate** — every `ops/` transform,
>   the `equality_structural` predicates, `calculus::diff`, `eval_exact::eval`,
>   the polynomial walkers, and so on.
> - §3(c) says canonicalize "adds ≤1 level". That is per *node*: `Div(a,b) →
>   Mul[a, Pow(b,−1)]` is ×2 down a `Div` spine, so a chain of nested divisions
>   doubles in depth.
>
> Measured depth at which each pass overflows a **1 MB stack** (the wasm32
> default), release profile, on a single-child `Neg` tower / a two-child `Add`
> tower:
>
> | pass | traps at (Neg / Add) | bytes per level |
> |---|---|---|
> | `flatten`, `serde::to_js` | 2,976 / 1,824 | ~360 / ~585 |
> | `print::to_text` | 2,048 / 1,824 | ~520 / ~585 |
> | `canonicalize` | 2,624 / 1,984 | ~405 / ~537 |
> | derived `Clone` | 3,104 / 3,264 | ~340 |
> | `eval_complex` | 5,440 / 5,440 | ~195 |
> | derived `Drop` (no `tear_down`) | 32,768 / 13,056 | ~33 / ~82 |
> | derived `PartialEq` / `Hash` | loop-optimized / 21,760, 32,768 | ~49, ~33 |
>
> In a **debug** build the same passes are 5–20× worse: `to_js` overflows at
> **126 levels**, which is *below* the 128 that `from_js` admits. Release wasm
> has ~15× margin against `from_js`; a debug host has none.
>
> Unrelated but previously conflated with this plan: a wasm panic reaching the
> browser as a bare `unreachable` was blamed on `panic = "abort"`. That was
> wrong — std runs the panic hook before aborting, and there simply was no hook.
> One is installed now (`panic_report` in the wasm crate's `lib.rs`), so a stack
> overflow or assertion failure says what it was. That improves *diagnosis*, not
> safety; 23–25 are still the fix.

Companion to PORTING_PLAN.md §7f (resource limits). Scope: everything already
implemented (parsers, normalization, ordering, evaluation, equality, formatters,
expr::serde).

## 1. Problem

Rust on `wasm32-unknown-unknown` gets a **1 MB shadow stack** by default
(linker flag `-C link-arg=-zstack-size=N` changes it; there is no growth and
no graceful failure — overflow is a trap that kills the instance). Debug
frames for our tree-walking functions run hundreds of bytes to ~1–2 KB, so
recursion depth of a few hundred to a few thousand traps. All input is
untrusted; `((((…))))` at 50 000 deep costs an attacker 100 KB of text.

`stacker`/`psm` (segmented-stack growth) does **not** support wasm32 — on
that target it silently runs the closure without growing. Not an option.

## 2. Inventory of recursion (verified against source)

**Producers of `Expr`** (depth of everything downstream is bounded by what
these emit):
- `parse/text.rs`, `parse/latex.rs` — recursive descent, ~13 mutually
  recursive functions; **multiple frames per nesting level** (statement →
  … → base_factor → bracketed → statement_list → statement). Deepest frames
  in the codebase.
- `expr::serde::from_js` — the WASM `from_json` boundary.
- `parse/*::convert_units_in_term` — post-parse pass, recursive.

**Transforms / folds** (one frame per tree level):
- `expr::flatten`
- `normalize::canonicalize` (+ smart constructors calling each other)
- `normalize::order::cmp` (paired traversal)
- `eval_numeric::complex::eval_complex`, `eval_numeric::complex::free_symbols`
- `equality::contains_blank`, `equality::api::coerce_seqs`
- `print/text.rs`, `print/latex.rs` (`emit`/`render` mutual recursion)
- `expr::serde::to_js`

**Compiler-generated (easy to forget, equally fatal):**
- `#[derive(Clone, PartialEq, Eq, Hash, Debug)]` on `Expr` — all recursive.
  Stage-1 equality is derived `PartialEq`; `opaque_key` uses derived `Debug`
  *during sampling*; fixtures compare via derived `PartialEq`.
- **`Drop`**: dropping a deep `Expr` recurses through the `Box`/`Vec` chain.
  An iterative algorithm layer alone does not fix this — the tree still has
  to be freed.

## 3. Strategy: bounded boundary + depth-oblivious core

Two complementary layers, not one mechanism:

**(a) Depth cap at the producers.** The parsers get a nesting-depth counter
(increment in the ~4 self-nesting entry points: `statement`, `bracketed`,
`get_subsuperscript`, function-argument parsing; error "expression too
deeply nested" past the limit). `from_js` gets the same check. This is the
serde_json approach (default `recursion_limit = 128`) and is a legitimate
*language* restriction, not a workaround: real educational math is wide, not
deep (fixture corpus max depth is ~10–15). Proposed default **256**,
compile-time constant first, options-field later if ever needed.
Rationale for capping rather than rewriting: converting recursive descent to
an explicit pushdown automaton destroys the grammar-shaped readability that
makes the parsers verifiable against the JS, for zero user-visible benefit
once the cap exists.

**(b) Explicit-stack core.** Every post-parse pass becomes iterative, so the
library never trusts tree depth — trees can also arrive from `from_js` (its
cap can be configured away) or be built programmatically. One shared driver,
written once, replaces per-function hand-rolled loops:

```rust
/// Iterative post-order fold over an Expr. `enter` runs pre-order and may
/// prune (e.g. eval's opaque atoms, free_symbols' cutoff); `exit` combines
/// the node with its children's results.
enum Step<T> { Descend, Prune(T) }
fn fold<T>(
    root: &Expr,
    enter: impl FnMut(&Expr) -> Step<T>,
    exit: impl FnMut(&Expr, Vec<T>) -> T,
) -> T
```

Internally: `Vec<Frame>` with `Enter(&Expr)` / `Exit(&Expr, n_children)`
plus a value stack — the standard two-phase pattern, heap-allocated, O(depth)
memory. Needs a child-iteration helper (`fn children(&Expr) -> impl
Iterator<Item = &Expr>`) which several passes duplicate ad hoc today anyway.

Passes map onto the driver directly (all are genuine post-order folds):

| pass | fold result `T` | notes |
|---|---|---|
| `canonicalize` | `Expr` | exit = existing smart constructors, unchanged — they only look at already-folded children (O(1) deep) |
| `flatten` | `Expr` | same shape |
| `eval_complex` | `Option<Complex64>` | `enter` prunes opaque atoms |
| `free_symbols` | `()` + side-effect set | `enter` prunes opaque atoms |
| `contains_blank` | `bool` | could early-exit via `Prune` |
| `coerce_seqs` | `Expr` | trivial exit |
| `to_js` | `serde_json::Value` | trivial exit |
| formatters `render` | `(String, u8)` | exit = existing per-node render logic; `render_*` helpers stop calling `emit` recursively and instead receive child strings |
| `convert_units_in_term` | `Expr` | small; or absorb into canonicalize-era cleanup |

Not folds, need their own small loops:
- `order::cmp` — iterative paired traversal: `Vec<(&Expr, &Expr)>`, push
  child pairs, return on first non-equal. ~30 lines.
- `from_js` — same driver shape over `serde_json::Value` instead of `Expr`.

**(c) The compiler-generated impls.** With (a) capping producers *and* (b)
never deepening trees more than a constant factor (canonicalize adds ≤1
level for `Div → Mul[…, Pow(b,−1)]`), derived `Clone/PartialEq/Hash/Debug`
recursion is bounded by `cap × small-constant` — safe if the cap is sized
against the *largest* frame among these. Two exceptions to fix explicitly:
1. **`Drop`**: implement iteratively (the classic pattern — `impl Drop for
   Expr` that drains children into a `Vec` worklist, replacing them with
   cheap leaves). Do this regardless of the cap: it is ~25 lines and removes
   the entire "freeing the tree crashes" class, including for trees built
   before a cap existed.
2. **`opaque_key` via derived `Debug`**: replace with a key produced by the
   `fold` driver (also fixes its interner-index brittleness noted earlier).
Replacing derived `PartialEq` with an iterative one is optional under the
cap; decide by measuring frame sizes (see §5). If we ever expose programmatic
tree construction in the WASM API, revisit — cheap to do with the pair-stack
used by `cmp`.

## 4. What was considered and rejected

- **Pushdown-automaton parsers** — disproportionate (see 3a).
- **`stacker` stack growth** — no wasm32 support.
- **Flat arena representation** (`Vec<Node>` + u32 child indices): solves
  stack-safety *structurally* (no pointers to chase, `Drop` is one `Vec`
  free) and would speed everything up — but it invalidates every
  `match`-based pass and the faithful-port parsers mid-project. Recorded as
  the natural post-Phase-7 optimization, pairing with the hash-consing note
  in PORTING_PLAN §16. The `fold` driver introduced here survives that
  migration (only `children()` changes).

## 5. Verification

- **Small-stack CI test** (works natively, no wasm harness needed): run each
  pass in `std::thread::Builder::new().stack_size(128 * 1024)` against (i) a
  tree at exactly the parser depth cap, (ii) a 10⁵-node *wide* tree. Trapping
  depth ≫ cap proves the margin; wide trees prove the drivers don't regress.
- **Deep-input tests**: 10⁵-deep `((((…))))` → clean `ParseError`, on both
  parsers and `from_js` — not a crash.
- **Regression safety**: these are pure refactors under a strong oracle —
  the full parser fixture corpus, round-trip suite, and 824-pair equality
  corpus gate every conversion. Convert one pass per commit.
- **Frame-size measurement**: before finalizing the cap, measure worst-case
  frames (debug build) for the parser chain and derived `PartialEq` on the
  target: `cap × frames-per-level × max-frame` must fit comfortably in the
  configured `-zstack-size` (also add that link flag to the wasm build docs).

## 6. Sequencing (each step independently shippable, gated by the full suite)

1. **Iterative `Drop`** for `Expr` — smallest change, kills a whole crash
   class on its own.
2. **Parser depth cap** (text + latex + `from_js`) + deep-input tests +
   small-stack CI test. At this point the crash vector is closed end-to-end;
   everything after hardens internals.
3. **`children()` helper + `fold` driver** in `expr` (with `Prune`).
4. Port passes in dependency order: `flatten` → `canonicalize` → `cmp` →
   `eval_complex`/`free_symbols`/`contains_blank`/`coerce_seqs` →
   `to_js`/`from_js` → formatters → `convert_units_in_term`. One per commit.
5. Replace `opaque_key`; decide derived-`PartialEq` question from the §5
   frame measurements.
6. Document `-zstack-size` in the WASM build notes (§13) when bindings land.

Rough effort: steps 1–2 are small; step 3 plus the first two ports is the
bulk; remaining ports are mechanical repetitions of the pattern.
