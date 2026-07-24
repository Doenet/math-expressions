# Architecture Review — refactoring footguns (Rust crates)

Date: 2026-07-22. Scope: `packages/math-expressions-rs` + `packages/math-expressions-rs-wasm/crate`.
Method: four parallel audits (rendering, Expr/traversal, math primitives, boundaries/errors),
top findings re-verified against source. Each finding is labeled **[NEW]** or
**[TRACKED: plan]** so this doc doesn't duplicate the existing plans.

The existing plans (IMPROVEMENT, STACK_SAFETY, FULL_SIMPLIFY §8) already cover a
lot; the two big *untracked* structural risks found are **the Const/Sym constant
duality (§1)** and **the text/latex twin-file duplication in the parsers (§2)**.

> **STATUS (2026-07-22):** most findings are fixed. Per-finding markers below:
> **✅ DONE** · **🔵 ASSESSED — deliberately deferred** (with reason) ·
> **⬜ OPEN** (not started). The blow-by-blow of the work is §9. Remaining
> **⬜ OPEN**: printer dedup (§3, tracked to IMPROVEMENT Phase 2); two LOW
> rendering items (§4); sampling-constant consolidation (§5); the RootOf
> invariant *doc note* (§6); `Tok::Comma` Display hardcode (§7); and in §8 —
> `from_decimal_str` pub-panic, `Sym` interner growth, a crate-wide error enum,
> and module `pub(crate)` demotion.

---

## 1. ✅ DONE — [NEW · HIGH] `MathConst::Pi/E/I` vs `Sym("pi"/"e"/"i")` — two spellings of one constant

> **✅ Fixed at the root:** `canonicalize` rewrites `Const(Pi/E/I)` → `Sym`,
> both internal producers (`exact::to_expr`, deg-units desugar) now mint `Sym`,
> and `fuzzy.rs::replace_numbers` parameterizes both spellings. The `==`
> contract holds again; regression `equals("90 deg", "pi/2")` in
> `tests/architecture_fixes.rs`. The Sym-only holes (`constants_to_floats`,
> `e^x→exp`, `variables()`) are now unreachable in practice (nothing produces
> `Const(Pi)` before canonicalize). *Deferred cleanup:* deleting the ~10 now-
> defensive both-spelling helpers (correct, just redundant).

The parsers only ever produce `Sym("pi")` etc. (`Const` is minted for Inf/NegInf/NaN
only), but two internal producers mint `Const(Pi/E)` **into the canonical layer**:
`exact::Exact::to_expr` (exact.rs:173,179 — i.e. inside simplify/equals folding) and
`desugar_units` deg-handling (norm/units.rs:64 — inside `equals`). `canonicalize`
does **not** unify the spellings, and they rank differently in `order::cmp`
(order.rs:19-21). Consequences:

- **Breaks the crate's core contract** ("after canonicalize, `==` is semantic
  equality", norm/mod.rs:5-6) — `Sym("pi") != Const(Pi)` yet both are π. Every
  `==` fast path, fixpoint loop, and `HashMap<Expr,_>` key inherits this.
- **Behavioral hole:** `equality/fuzzy.rs:147-148` `replace_numbers` parameterizes
  only the `Sym` spelling in the error-tolerance path; a `Const(Pi)` arriving via
  `90 deg` desugaring is not parameterized → tolerance semantics differ between
  `90 deg` and `pi/2`. (Verified against source.)
- Other single-spelling holes: `ops.rs:424-431` `constants_to_floats` (Sym-only),
  `norm/syntactic.rs:71` `e^x→exp(x)` (Sym-only), `ops.rs:558-563` `variables()`
  (Sym counted, Const not).
- **Duplication tax:** ≥10 sites carry hand-rolled both-spelling checks
  (`is_pi`/`is_e` triplicated in exact.rs, norm/special_values.rs, diff.rs;
  plus diverge.rs, eval, tape, finite_field, assumptions…).

**Fix direction (pick one):**
(a) canonicalize one spelling into the other (safest: `canonicalize` rewrites
`Const(Pi/E/I)` → `Sym`, since the whole codebase already handles Sym), or
(b) crate-level `Expr::is_pi()/is_e()/is_i()` helpers + delete the local copies.
Option (a) actually removes the class; (b) only shrinks it. Either way add a
regression test: `equals("90 deg", "pi/2 rad")`-style tolerance case.

## 2. ✅ DONE — [NEW · HIGH] Parser grammar-skeleton duplication (text.rs ↔ latex.rs)

> **✅ Fixed:** the 20 semantically-identical methods now live once in
> `parse/shared_grammar.rs`, stamped into both impls by
> `shared_grammar_methods!`. Verified behavior-preserving by diffing every
> shared method against the pre-dedup `latex.rs` from git (string-safe
> comment-insensitive compare — all 20 identical). text.rs 1247→731,
> latex.rs 1461→1009.

~23 identically-named methods with near-verbatim bodies (≈400+ lines):
`statement_list` (byte-identical), `statement_a/b/c`, the ~100-line
`relation_inner` RelOp-chain ladder, `enter/leave/advance/state/set_state/err`
scaffolding, the bar-fallback rewind, the number-destructure sites. A grammar or
depth-cap fix applied to one file silently misses the other. IMPROVEMENT_PLAN
Phase 2 covers *tables* only — this skeleton duplication is untracked.

**Fix direction:** extract a shared grammar core generic over a small
`ParserFlavor` trait (token-table + the ~6 genuinely latex-specific hooks:
matrix envs, `\sqrt`, opsym, `\circ`, braced Leibniz, LATEXCOMMAND validation).
Big but mechanical; best done *before* new grammar features (e.g. LIMITS P0
`lim` notation would currently have to be implemented twice).

## 3. ⬜ OPEN — Printer duplication (text ↔ latex Writers) — [PARTIALLY TRACKED: IMPROVEMENT Phase 2]

> **⬜ Not started.** Still rides IMPROVEMENT Phase 2; the i18n helpers
> `arg_sep`/`decimal` remain duplicated in both Writers (post-date the plan —
> add to Phase 2's scope).

~24 structurally parallel Writer methods; `render_add` byte-identical,
`render_mul` identical modulo the join glyph, `is_shorthand_angle` copy-pasted,
and the new i18n helpers `arg_sep`/`decimal` duplicated in both (post-date the
plan). Phase 2 covers RelOp/SeqKind/OtherOp render tables but not the shared
sum/product/leibniz skeletons or the i18n helpers. Extend Phase 2's scope note.

## 4. Rendering paths that bypass the central printers

- **✅ DONE — [MED] LaTeX RootOf doesn't round-trip:** `\operatorname{rootof}`
  is now a registered applied name and the printer emits `rootof(p, k)`
  (matching the text path); round-trip verified `parse(to_latex(r)) == r`.
- **✅ DONE — [MED] Debug output in user-facing renders:** `deriv_var` now
  renders a malformed Leibniz entry through the text printer, never `Debug`.
- **✅ DONE — [MED] `precise` number strings:** documented as **display-only**
  (not re-parseable, not notation-aware) on both `MpFix::to_decimal_string` and
  `Precise::to_decimal_string`; anything user-round-trippable must go through
  `output::to_text`/`to_latex` instead.
- **⬜ OPEN — [LOW] `subscripts_to_strings`** bakes `.`-decimal digits into Sym
  names — a later notation-aware print can't retarget them. Not started (LOW).
- **⬜ OPEN** [TRACKED: IMPROVEMENT Phase 3] `opaque_key` via `Debug`
  (eval/mod.rs).

## 5. Certified-zero / numeric-gate sprawl — [was TRACKED: FULL_SIMPLIFY §8]

- **✅ DONE** `diverge.rs` private `PiLin`/`exact_eval`/`rational_sqrt_exact`
  (~160 lines) deleted; `exactly_zero_at`/`certified_nonzero_at` now call
  `exact::exact_eval`.
- **✅ DONE** `matrix/kernels::is_zero` now escalates non-literals to
  `exact::certified_zero` (a symbolic-but-zero entry can no longer be a pivot),
  plus `entry_nonzero` adds the certified-*nonzero* direction for variable-free
  entries. Regression in `tests/architecture_fixes.rs`.
- **✅ DONE** the ±1-ulp contract is one function: `MpFix::excludes_zero()`
  (both former copies call it).
- **✅ DONE** `number_to_rational` clone delegates to `Number::to_bigrational`.
- **✅ DONE** ln/log: all registry builders emit canonical `log`;
  `assumptions` matches `log|ln`. (Corpora green — no JS-parity break.)
- **🔵 ASSESSED — deferred:** eigen root ladder → `factor()` — today's `factor`
  is strictly weaker (no quadratic closed forms / ordered RootOf tail); deferred
  to FULL_SIMPLIFY S4, doc comment at the eigen site.
- **🔵 ASSESSED — not merged:** `integrate::expr_to_ratfun` vs `ratform` share
  no representation (univariate `UPoly` pairs vs multivariate kernelized `Rep`);
  cross-reference notes added at both sites.
- **⬜ OPEN** 4 independent sampling mechanisms (equality/numeric.rs,
  finite_field.rs, exact.rs `SAMPLE_POINTS`, diverge.rs `GRID`) — a shared
  sample-point policy module is deferred.

## 6. Traversal architecture — [MOSTLY TRACKED: IMPROVEMENT Phase 3/4, STACK_SAFETY]

- N ≈ 12 compiler-enforced match sites + ~8 silent-fallback traversals per new
  `Expr` variant (LIMITS P0 will pay this: budget for it).
- **✅ DONE — `flatten`'s catch-all** is now an explicit leaf list — a new
  compound variant is a compile error, not silent corruption.
- **✅ DONE — cheap wins:** `desugar_units` and `coerce_seqs` reduced to
  `map_children` + one specific arm.
- **✅ DONE — Doc contradiction:** expr.rs header now says `OtherOp` lives in
  both layers (canonicalize preserves it; pm/diff/matrix ops mint it).
- **⬜ OPEN — RootOf round-trip asymmetry (doc-only):** the invariant "RootOf
  leaf exists only post-canonicalize" is not yet noted at the variant
  declaration. (The LaTeX/round-trip *behavior* is fixed in §4; this is the
  remaining documentation item.)
- **✅ DONE** [STACK_SAFETY] stale checkboxes corrected — item 22 (parser depth
  caps) marked done; iterative Drop / fold driver remain genuinely open in that
  plan.

## 7. Notation/options architecture — [NEW, from the i18n work]

**✅ Root cause fixed:** the wasm `Expression` now *carries* its parse notation
(`Expression(Expr, NumberNotation)`); every render inherits it, killing the
whole "re-supply at every call" class.
- **✅ DONE** matrix/calculus JSON payloads and `to_text`/`to_latex` inherit the
  handle's notation (with `*_with_options` to override per call).
- **✅ DONE** `simplify_with_assumptions` parses assumption strings in the
  expression's notation (`"x > 1,5"` reads correctly under comma notation). The
  residual silent-drop of a genuinely-unparseable assumption is intentional JS
  parity (`me.simplify` ignores bad assumptions).
- **✅ DONE** `validate()` folded into `read_notation` (returns `Result`) — a
  5th entry point can no longer forget it.
- **✅ DONE** `validate()` now rejects operator-glyph collisions (argument
  `':'`/`'|'`/brackets/…, decimal `'-'`, argument `'.'`).
- **✅ DONE** Phase-2 stub keys (`groupSeparator`, `grouping`, `digits`) now
  error explicitly at the boundary ("not yet supported") instead of silently
  no-op'ing.
- **⬜ OPEN** `Tok::Comma` Display hardcodes `","` in parser error strings —
  deferred (cosmetic; JS-parity error text).

## 8. Panic/abort & boundary hygiene — [NEW]

Workspace sets `panic = "abort"`: every panic is a full wasm-worker crash.
- **✅ DONE — HIGH:** `OdeSolution::at(NaN)` now propagates NaN (guard added);
  regression in `tests/architecture_fixes.rs`.
- **✅ DONE** `exact.rs`: `Exact::surd` no longer re-factors or `expect`s; the
  trial-division cap moved into `ResourceLimits::max_squarefree_trial_divisor`.
- **✅ DONE** `js_tree::from_js` (panicking) removed; the few test callers use
  `try_from_js(...).expect(...)`.
- **✅ DONE** `equals_with_options` now errors on malformed JSON (returns
  `Result<bool, JsError>`) instead of silently grading with defaults. *(Breaking
  JS API change — throws instead of returning a bool on bad JSON.)*
- **✅ DONE** `rootof` ROOT_CACHE/ISO_CACHE capped (1024 entries,
  clear-on-overflow).
- **✅ DONE** `ResourceLimits` exposed to JS
  (`set_resource_limits`/`get_resource_limits`).
- **⬜ OPEN** `Number::from_decimal_str` is still `pub` and still panics on
  non-numeric input (`expect("NUMBER token is all digits")`). Safe today via the
  lexer↔notation invariant, but Phase-2 digit sets will strain it — make it
  return `Option`/`Result` or demote to `pub(crate)`.
- **⬜ OPEN** `Sym` interner is append-only (slow leak; plus the native-only
  cross-thread-interner hazard). No cap/eviction yet.
- **⬜ OPEN** Error-type story: reason-less `Option` refusals vs `ParseError` vs
  `Precise::Unknown(&'static str)`; a crate-wide error enum is deferred.
- **⚠️ PARTIAL** pub-surface drift: the wasm crate now consumes root re-exports
  (missing ones added), but demoting the unconsumed `pub` modules to
  `pub(crate)` is deferred (it churns the test suite, which reaches into
  internals).

## 9. Suggested sequencing

1. **✅ DONE 2026-07-22 — point fixes** (regressions in
   `tests/architecture_fixes.rs` + extended `tests/notation.rs`; 488 lib tests
   green, clippy clean): ode NaN guard; `flatten` exhaustive leaf arms;
   fuzzy.rs Const(Pi/E) parameterization; expr.rs OtherOp doc fix;
   `number_to_rational` → `Number::to_bigrational`; `equals_with_options` now
   errors on bad JSON (signature: `Result<bool, JsError>`); `read_notation`
   validates internally (returns `Result`); validate() rejects operator-glyph
   separator collisions (incl. decimal `'-'`, argument `'.'`/`':'`/`'|'`);
   assumptions matches `log|ln`; exact.rs `surd` no longer re-factors or
   `expect`s and the squarefree trial-division cap moved into
   `ResourceLimits::max_squarefree_trial_divisor`; stale STACK_SAFETY /
   WHATS_LEFT item-22 notes corrected.
2. **✅ DONE 2026-07-22 — Const/Sym unification (§1), root-cause form:** both
   producers (`exact::to_expr`, deg-units desugar) now mint `Sym`, and
   `canonicalize` unifies `Const(Pi/E/I)` → `Sym` for any remaining producer.
   The ~10 local both-spelling helpers are now defensive-only and can be
   deleted opportunistically (not urgent — they're correct, just redundant).
3. **✅ DONE 2026-07-22 — §8 refactors (the feasible ones):**
   - `matrix/kernels::is_zero` → syntactic fast path + `exact::certified_zero`
     (a symbolic-zero entry can no longer be chosen as a pivot; regression in
     `tests/architecture_fixes.rs`), plus `entry_nonzero`: variable-free
     entries get the exact service's certified-*nonzero* direction, so
     `rank([[√8−√2]]) = 1` now decides instead of refusing.
   - diverge.rs `PiLin`/private `exact_eval`/`rational_sqrt_exact` (~160
     lines) deleted; `exactly_zero_at`/`certified_nonzero_at` call
     `exact::exact_eval`. The ±1-ulp contract is one function:
     `MpFix::excludes_zero()`.
   - **eigen ladder → factor: assessed, deliberately NOT done** — today's
     `factor()` is strictly weaker than the ladder (no quadratic closed
     forms, no ordered RootOf tail); doc comment at `matrix/eigen.rs::
     eigen_items` defers it to FULL_SIMPLIFY S4.
   - **expr_to_ratfun → ratform: assessed, deliberately NOT merged** — they
     share no representation (univariate `UPoly` pairs vs multivariate
     kernelized `Rep`); cross-reference layering notes added at both sites.
4. **✅ DONE 2026-07-22 — parser grammar-core extraction (§2):** the 20
   semantically-identical methods (statement/relation/expression/term/factor
   ladder + scaffolding, ~450 lines of duplication) now live once in
   `parse/shared_grammar.rs`, stamped into both impls by
   `shared_grammar_methods!`; flavor-specific productions (`Tok::Mid`, pipe
   fallback, sub/superscript digits, unit tables, `advance`) stay per-file.
   text.rs 1247→731 lines, latex.rs 1461→1009. LIMITS P0 now adds `lim` to
   ONE grammar skeleton.
5. **✅ DONE 2026-07-22 — notation-carrying `Expression` (§7):** the wasm
   handle carries its parse notation; `to_text`/`to_latex`, matrix/calculus
   JSON payloads, `simplify_with_assumptions`'s assumption parsing, and all
   derived expressions inherit it (`*_with_options` overrides per call).
   Also: Phase-2 stub notation keys now error explicitly; `ResourceLimits`
   exposed to JS (`set_resource_limits`/`get_resource_limits`).
6. Printer-table consolidation rides IMPROVEMENT Phase 2 as planned (scope
   note: add the i18n helpers + sum/product skeletons). Remaining smaller
   items also landed 2026-07-22: LaTeX RootOf round-trip
   (`\operatorname{rootof}` registered + emitted), `deriv_var` Debug fallback
   → text render, builders emit canonical `log`, rootof caches capped
   (1024, clear-on-overflow), `js_tree::from_js` removed, `precise`
   stringifiers documented display-only, wasm consumes root re-exports
   (+`together`/`cancel` re-exported), facade tiers documented in lib.rs.
   **Still open (documented, deliberately deferred):** module demotion to
   `pub(crate)` (churns the test suite), a crate-wide error enum, Cargo
   feature flags (IMPROVEMENT Phase 5), sampling-constant consolidation,
   `Tok::Comma` Display hardcode (JS-parity error strings).
