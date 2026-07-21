# Improvement Plan: maintainability, memory, and bundle size

> **PROGRESS (audited 2026-07-20):** PARTIAL — Phases 0–1 done. Phases 2–5 open
> (`WHATS_LEFT.md` §B.3, items 30–34): notation-table consolidation, memory
> quick-wins (`opaque_key`→hash, `for_each_child` primitive), by-value
> normalization passes, Cargo-feature bundle cleanups, and the deferred
> serde_json-at-boundary drop (item 34 — coordinate with TSIFY_PLAN §1.C).

Analysis date: 2026-07-19. Target: `math-expressions-rs` compiled to
wasm32-unknown-unknown. Priorities, in order:

1. **Maintainability** — the library will keep growing; adding a math function
   must be a one-place change.
2. Memory efficiency — wasm linear memory grows and is never returned to the
   OS, so *peak* allocation is *permanent* footprint.
3. Bundle size.

Standing constraints:
- Output stays compatible with the JS math-expressions library in most cases
  (the architecture may deviate; the rendered trees/strings should not).
- **Decision (2026-07-19): keep `serde_json`** at the JS boundary for now.
  Dropping it is a real bundle lever (~100–200 KB) but the refactor is
  deferred; revisit if bundle size becomes pressing.

---

## Phase 0 — Build configuration (measured, do first, one commit)

> **STATUS (2026-07-20): DONE.** Profile added to the **workspace root**
> Cargo.toml (cargo ignores profiles in member manifests — the crate-level
> attempt warned and did nothing). wasm-opt step added to build-wasm.sh
> (skips when not installed). Verified: 1.76 MB → 1.14 MB (−35 %), gzip
> 539 KB → 375 KB, 55/55 smoke tests pass.

`Cargo.toml` has no `[profile.release]` section and `scripts/build-wasm.sh`
never runs `wasm-opt`. Measured result with the settings below:
**1,702,675 → 1,099,471 bytes (−35 %)**; gzipped 511 KB → 354 KB. All 51
smoke tests pass with the optimized build.

```toml
[profile.release]
opt-level = "z"      # try "s" if runtime benchmarks regress
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "debuginfo"
```

Notes:
- `src/js_tree.rs:31` already *assumes* wasm builds abort on panic; the
  profile just wasn't set, so the unwinding + panic-formatting machinery
  ships today for no reason.
- Add an optional `wasm-opt -Oz` step to `build-wasm.sh` (skip gracefully if
  not installed); typically another 10–15 %.
- `cargo install twiggy` when hard per-subsystem numbers are wanted.

---

## Phase 1 — Function registry: one place per function  ★ centerpiece

### The problem, measured

A single function — `sinh` — is currently known in **9 files / 12 sites**:

| # | Facet | Location |
|---|-------|----------|
| 1 | text-parser default `applied_function_symbols` | `parse/text.rs:59` |
| 2 | latex-parser default `applied_function_symbols` | `parse/latex.rs:50` |
| 3 | latex printer allowed/backslash-symbol list | `output/latex.rs:496` |
| 4 | evaluator sampling/closedness filter | `eval/mod.rs:109` |
| 5 | evaluator complex dispatch | `eval/mod.rs:188` |
| 6 | precise-eval kernel registry (`f`, `df`, `cf`, `cdfm`, domain) | `precise/kernels.rs:152` |
| 7 | syntactic-simplify class list A | `norm/syntactic.rs:50` |
| 8 | syntactic-simplify class list B | `norm/syntactic.rs:57` |
| 9 | derivative template table | `diff.rs:168` |
| 10 | inverse-function pairing | `norm/mod.rs:805` |
| 11 | antiderivative table | `integrate/mod.rs:256` |
| 12 | (spelling normalization `arc…` → `a…`, latex `\operatorname` choice) | scattered |

Adding `asinh`-style functions today means editing ~5–9 files and hoping no
list was missed — the lists are stringly and nothing cross-checks them.

### Design: `FnDef` + explicit registry

One struct holds every facet of one function. One `const` list registers
them all. Everything else *derives* from the registry.

```rust
// src/functions/mod.rs
pub struct FnDef {
    /// Canonical spelling ("asin"). Normalization target for aliases.
    pub name: &'static str,
    /// Parse-time alternate spellings ("arcsin"). Normalized to `name`.
    pub aliases: &'static [&'static str],
    /// Which default symbol lists this belongs to (text parser, latex
    /// parser, latex allowed-symbols) — bitflags or bools.
    pub parse: ParseDefaults,
    /// LaTeX rendering: Backslash ("\\sin") vs OperatorName vs Custom.
    pub latex: LatexForm,
    /// Derivative as a text template in `x` (e.g. "cosh(x)"), parsed once
    /// on first use. `None` = no elementary derivative rule.
    pub derivative: Option<&'static str>,
    /// Antiderivative template, same convention.
    pub antiderivative: Option<&'static str>,
    /// Canonical name of the inverse function, if notated ("sinh" → "asinh").
    pub inverse: Option<&'static str>,
    /// f64/Complex64 evaluation + precise-eval kernel bundle. Reuses the
    /// existing `FnKernel` shape from precise/kernels.rs (f, df, domain,
    /// cf, cdfm) — that struct moves here or is referenced here.
    pub kernel: Option<&'static FnKernel>,
    /// Class for syntactic-simplify membership tests (Trig, InverseTrig,
    /// Hyperbolic, …). Replaces the ad-hoc lists in norm/syntactic.rs.
    pub class: FnClass,
}

/// The single source of truth. Explicit, compiler-checked, no linker magic
/// (inventory/linkme are unreliable on wasm — do not use them).
pub const ALL: &[&FnDef] = &[
    &trig::SIN, &trig::COS, /* … */
    &hyperbolic::SINH, /* … */
];

/// Name/alias → FnDef, built once (OnceLock<HashMap<&str, &FnDef>>).
pub fn lookup(name: &str) -> Option<&'static FnDef> { … }
/// Iterators the old lists are rebuilt from:
pub fn all_applied_function_names_text() -> impl Iterator<Item = &'static str> { … }
pub fn in_class(c: FnClass) -> impl Iterator<Item = &'static FnDef> { … }
```

Dispatch stays fast: the evaluators resolve `Sym → Option<&'static FnDef>`
once per node (the interner id can cache the lookup if profiling ever says
the HashMap probe matters — don't pre-optimize this).

### File layout

Start with **family files**; graduate any function to **its own file** when
its definition (FnDef + its precise MpFix kernel + colocated unit tests)
exceeds ~150 lines. The per-function MpFix series kernels currently in
`precise/kernels.rs` (858 lines) are what make single-function files real —
`exp`, `ln`, `sin`/`cos`, `atan` each carry a substantial series
implementation and would each justify a file.

```
src/functions/
  mod.rs                 // FnDef, FnClass, registry, lookup, derived lists
  trig.rs                // sin cos tan sec csc cot  (~40–60 lines each)
  trig_inverse.rs        // asin acos atan asec acsc acot (+ arc… aliases)
  hyperbolic.rs          // sinh cosh tanh sech csch coth
  hyperbolic_inverse.rs
  exp.rs                 // exp + its MpFix series kernel   (>150 lines → own file)
  log.rs                 // ln log log10 + ln series kernel (>150 lines → own file)
  atan.rs                // if the atan series kernel moves here, it earns its file
  powers.rs              // sqrt cbrt nthroot abs sign
  misc.rs                // floor ceil round factorial re im conj arg …
```

What stays where it is:
- Shared precise-eval machinery (MpFix/CFix types, `Budget`, argument
  reduction, constant caches, the tape compiler/interpreter) stays in
  `precise/`. Function files own only *their* kernel entry points.
- Generic `Apply` handling (sin²-notation, prime, single-arg tuple encoding)
  stays in the parsers/printers — it is per-*notation*, not per-function.
- The `OtherOp` escape hatch and non-function operators are Phase 2.

> **STATUS (2026-07-20): steps 1–7 DONE — Phase 1 complete.** `src/functions/`
> (mod.rs + trig / trig_inverse / hyperbolic / hyperbolic_inverse / exp_log /
> powers / misc) now carries ALL per-function facets: spellings, parser
> defaults, aliases, inverses, move-exponent spellings, derivative
> templates, antiderivative builders, complex `eval1`/`eval2` (incl.
> factorial's Γ), LaTeX control words + apply-head overrides, and the
> precise-eval `FnKernel` rows. `"sinh"` appears in exactly ONE source file
> (was 9). Old tables deleted from parse/text.rs, parse/latex.rs,
> norm/syntactic.rs, norm/mod.rs, diff.rs, integrate/mod.rs, eval/mod.rs,
> output/latex.rs, precise/kernels.rs; `precise::kernels::registry()`/
> `lookup()` derive the `Op::Call` id space from `functions::ALL` (ids are
> per-run, so order changes are safe). tests/functions_registry.rs pins
> every derived view to the historical tables. Full suite: 343 tests green;
> wasm smoke 55/55; wasm 1.16 MB (registry indirection cost ~10 KB vs the
> Phase 0 floor — accepted for the one-place property).
>
> Design notes: (a) `move_exponent_spellings` and `latex_commands` are
> explicit per-spelling lists (not alias-derived) because those rewrites
> run before name normalization — `ln` qualifies, `cosec` never did;
> (b) `eval1`/`eval2`/`inverse` match canonical spellings only, like the
> historical lists; (c) `ALL` is a `static` (single identity — kernel ids
> are positions in it); (d) step-6 verdict: NO per-function file splits —
> no definition exceeds ~150 lines, and the MpFix series core in
> precise/kernels.rs is a tightly-coupled unit (const_pi feeds sin/cos,
> const_ln2 feeds exp and ln, tan composes sin/cos) referenced via
> `FnDef::kernel` rather than scattered.

### Migration steps (tests green after every step)

Each step ports one facet: the old table becomes a thin delegate to the
registry, tests run, then the old table is deleted. No big-bang.

1. **Scaffold**: `functions/mod.rs` with `FnDef { name, aliases, class,
   parse }` only. Port the four pure string lists — both parser defaults,
   the latex allowed-symbols list, both `norm/syntactic.rs` class lists —
   plus `inverse` and the `arc…`→`a…` normalization. Add a snapshot test
   asserting the derived default lists are *identical* to today's lists
   (this is the output-compatibility guarantee).
2. **Derivative + antiderivative**: fold `diff.rs::template_for` and the
   `integrate/mod.rs` table into `FnDef.derivative` / `.antiderivative`.
3. **Evaluation**: fold the `eval/mod.rs` complex dispatch and the sampling
   filter (domain moves into the kernel/def).
4. **Rendering**: fold the latex `\operatorname`-vs-backslash choice and
   `convert_latex_symbol` (asin→arcsin) into `FnDef.latex` + `aliases`.
5. **Precise kernels**: point `FnDef.kernel` at the existing `REGISTRY`
   entries; then dissolve `REGISTRY` into the FnDefs. The `FixId` enum in
   the tape can become an index into the registry.
6. **Split files**: move families into their files; graduate >150-line
   functions (exp, ln, sin/cos series, atan) to single files, bringing
   their MpFix kernels along.
7. **Document**: a short "Adding a function" section in `functions/mod.rs` —
   the checklist should be: *write one FnDef in the right family file, add
   it to `ALL`, add corpus/round-trip tests.* If the checklist has a third
   code location, the migration isn't done.

### Guardrails

- Compile-time: `ALL` is explicit, so a forgotten registration is visible in
  review; a unit test asserts no duplicate names/aliases.
- Compatibility: the step-1 snapshot test pins the derived parser defaults
  and printer tables to their current contents; JS-fixture corpus tests
  (already in `tests/`) pin behavior.

---

## Phase 2 — Non-function notation metadata (same idea, smaller)

After Phase 1 proves the pattern, consolidate the remaining scattered
notation tables:

- **Greek letters / symbol spellings** exist twice: lexer replacement rules
  (`parse/lexer.rs:518–560`, ~80 rules) and the 146-entry match in
  `output/mod.rs:92–150`. One `const GREEK: &[(&str, &str, …)]` consumed by
  both. (~2–3 KB of duplicated strings, and one place to add a symbol.)
- **RelOp / SeqKind** already centralize their JS names on the enum — good;
  extend the same enums with their text/latex render forms so
  `output/text.rs` and `output/latex.rs` stop re-matching them.
- The `OtherOp` tail (binom, vec, unit, pm, …) gets a small
  `OpMeta { name, arity, text_form, latex_form }` table shared by the latex
  parser's `operator_symbol()`/`unit_of()` and both printers' match arms.

---

## Phase 3 — Memory quick wins (small, independent, high steady-state value)

1. **`opaque_key` → hash** (`eval/mod.rs:153`): currently
   `format!("{e:?}")` — allocates a full Debug rendering of the subtree per
   opaque-env lookup *and* keeps the recursive `Debug` impls for the whole
   `Expr` family alive in the wasm binary. `Expr` derives `Hash`; return a
   `u64` from `std::hash` instead. (Collision risk is theoretical for an
   env-lookup key; if that bothers, use a 128-bit hash.)
2. **`Sym::name()` → `&'static str`** (`sym.rs`): interned names already
   live forever in the thread-local interner, so `Box::leak` each name once
   and return `&'static str`. Kills a `String` allocation at ~98 call sites
   (printers pay it per symbol node).
3. **`Expr::for_each_child`** (`expr.rs:189`): `children()` returns
   `Vec<&Expr>`, allocating at every node of every predicate walk. Add
   `for_each_child(&self, f: &mut dyn FnMut(&Expr))` as the primitive and
   implement `children()` on top. `&dyn` (not generic) so the traversal is
   compiled once, not per closure.
4. **Static parser tables** (`parse/text.rs:96–112`, mirrored in latex):
   every `TextToAst::new` clones the ~50-entry default `Vec<String>` lists
   into fresh `HashSet<String>`s. After Phase 1 the defaults are
   `&'static [&'static str]` from the registry; build the default sets once
   in a `OnceLock` and only materialize owned sets for user-supplied
   overrides. Also: lexer tokens for fixed operators should be
   `Cow<'static, str>` instead of per-token `to_string()`.

---

## Phase 4 — Normalization pass architecture (the real peak-memory fix)

Today every pass rebuilds the entire tree:

- `map_children` (`norm/syntactic.rs:273`) reallocates every node even when
  the callback returns its input unchanged.
- `simplify_rounds` (`norm/simplify.rs:66–83`) re-canonicalizes after every
  rewrite round, so one `equals()` reconstructs the full tree several times.
- `coerce_seqs` (`eq/mod.rs:180`) is another full rebuild before
  canonicalize.

Fix in two escalating steps, keeping the deep-owned `Expr` (no Rc/arena —
see below):

1. **By-value passes**: change pass signatures to `Expr → Expr` so unchanged
   `Vec`s/`Box`es move instead of clone. Mechanical, local, big.
2. **Unchanged propagation**: give `map_children` a sibling that returns
   `Option<Expr>` (`None` = untouched), so fixpoint loops skip
   re-canonicalization when nothing fired, and untouched subtrees are never
   reallocated. `try_distribute` in `norm/expand.rs:103` (cartesian product
   clones both sides of every term pair) is the single worst spike site and
   should be first to adopt it.

Also consolidate on exactly **two blessed traversals** in `expr.rs` —
borrowed `for_each_child` + owned `map_children` — and build every generic
pass on them. Today a new `Expr` variant touches ~8–10 match sites
(children, map_children, canonicalize, ordering, structural-eq in
`eq/mod.rs:302` vs `norm/order.rs:80`, both printers, js_tree). Target:
enum + the two traversals + only the passes with variant-specific logic.

Deliberately **not** planned: hash-consing / `Rc<Expr>` / arenas. It's the
known long-term destination if rewriting grows CAS-scale, but it touches
everything and conflicts with priority #1 right now. The by-value +
unchanged-propagation design does not foreclose it.

---

## Phase 5 — Bundle structure and subsystem cleanups (as-needed)

1. **Cargo features for heavy subsystems** (`eigen`, `integrate`, `precise`,
   `numeric-compat`), default **on**. A grading-only consumer
   (parse/simplify/equals) currently pays for exact eigenvectors, the
   rational-integration engine, the arbitrary-precision evaluator, and the
   f64 QR `eigs` in `numeric.rs` (which duplicates `matrix.rs`
   eigen-machinery for mathjs compat). Establishing the convention now is
   cheap; retrofitting later is not. Estimated slim build: ~1.2–1.3 MB
   pre-Phase-0-stacking (unverified — run twiggy for real numbers).
2. **`precise/` interpreter dedup**: three near-identical tape walks
   (real forward `precise/mod.rs:368`, complex planning ~651, complex
   forward ~708) and two structurally identical backward planners differing
   only in magnitude source. Factor the planner over a `mag: impl Fn(usize)
   -> f64` and the `Call` dispatch into a kernel table. Falls out naturally
   during Phase 1 step 5.
3. **Matrix clone reduction**: `det_bareiss`/`rref_core`/Gauss–Jordan in
   `matrix.rs` clone `Expr`/`BigRational` entries in O(n³) inner loops
   (71 `clone()` calls in the file); Faddeev–LeVerrier allocates a fresh
   n² matrix per step. Reuse buffers, take entries by reference, split
   `mem::take` where entries are consumed. Bounded by `limits.rs`, but each
   spike is permanent wasm memory.
4. **`precise/kernels.rs` series loops** clone `BigInt` accumulators per
   iteration; convert to in-place mutation. Batch the per-operand
   `at_scale().mant` rescales in Add/Mul.
5. **Polynomial docs**: three representations exist for good reasons
   (`Expr`-as-polynomial; `upoly.rs` dense univariate ℚ[t] for
   RootOf/Sturm/charpoly; `poly/` multivariate for `reduce_rational` GCD)
   — write the module-level doc explaining the split. Their ~150
   overlapping lines of dense add/mul/divrem are *not* worth unifying
   behind a trait; clarity beats deduplication here.

---

## Explicitly deferred / rejected

- **Dropping `serde_json`** — deferred by decision (2026-07-19). Cheap
  partial step if ever wanted: the `derive` feature on `serde` appears
  unused (nothing in `src/` derives Serialize/Deserialize).
- **Hash-consing / arena `Expr`** — rejected for now (see Phase 4).
- **`phf` or linker-based registration** — rejected; explicit `const`
  tables + `OnceLock` sets are simpler, wasm-safe, and fast enough.

## Suggested sequencing

| Order | Item | Size | Risk |
|-------|------|------|------|
| 1 | Phase 0 build profile (+ optional wasm-opt) | XS | none (measured, smoke-tested) |
| 2 | Phase 1 steps 1–2 (registry scaffold, string facets, deriv/antideriv) | M | low — snapshot-pinned |
| 3 | Phase 3 quick wins (can interleave with Phase 1) | S | low |
| 4 | Phase 1 steps 3–7 (eval, rendering, kernels, file split, docs) | M–L | low–medium |
| 5 | Phase 2 notation metadata | S | low |
| 6 | Phase 4 pass architecture | L | medium — corpus tests are the net |
| 7 | Phase 5 items, as pressure demands | S–M each | low |
