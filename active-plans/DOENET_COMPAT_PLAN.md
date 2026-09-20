# DoenetML compatibility — fix plan for issue #83

**Source:** [Doenet/math-expressions#83](https://github.com/Doenet/math-expressions/issues/83)
("Compatibility issues for use with Doenet"), re-verified against merged `main`.
**Goal:** make `math-expressions-js-compat` a genuine drop-in for DoenetML, and
unblock DoenetML "Stage 2" (depending on `math-expressions-rs` as a crate).
**Status:** Phase 1 **landed** (see §8); Phases 2–4 planning. Every claim below is
grounded in current source; file:line references are to `packages/` unless noted.

> **Decision taken (2026-07-31): DoenetML owns the round-trip serialization fix.**
> That closes open decision 1 — **R13b is not ours**. We shipped R13a (below); the
> exact-value degradation across state save/load is handled on the DoenetML side.
>
> **Decision taken (2026-07-31): `Expr::Bool(bool)`.** That closes open decision 2;
> **R2-bool has landed** (see §8). The `null` half of R2 stays in Phase 2.

This plan groups the 13 requests (R1–R13) by **where the fix lives** and
**effort**, sequences them into shippable phases, and calls out the handful that
are better solved (or co-owned) on the DoenetML side.

---

## 0. Orientation — where each item actually lives

Three surfaces are in play:

- **Core crate** `math-expressions-rs/src/` — the Rust engine (`Expr`, printers,
  serde bridge).
- **WASM crate** `math-expressions-rs-wasm/src-rust/` (Rust bindings) +
  `src-js/` (the TS glue, incl. `compileRustExpr`).
- **Compat layer** `math-expressions-js-compat/lib/` — the legacy-API drop-in.

Findings from the code audit that change the shape of the work:

| Assumption in the issue | Reality in the code | Effect on the fix |
| --- | --- | --- |
| R3 needs new option plumbing | `to_latex_with_options` / `to_text_with_options` **already exist** ([wasm `core_ops.rs:41,56`](math-expressions-rs-wasm/src-rust/core_ops.rs#L41)) and read `notation`/`unicode` | R3 = add option *fields* to `LatexOpts`/`TextOpts` + forward from compat, not build plumbing |
| R4/R10 need new core logic | `get_component`/`substitute_component` **already exist** ([`ops/query.rs:74,83`](math-expressions-rs/src/ops/query.rs#L74)) but are `Seq`-only and unbound | R4/R10 = add a WASM binding + extend to `Expr::Matrix` |
| R2 = "add a match arm for Bool" | `Expr` has **no boolean-literal representation at all** ([`expr/tree.rs:20`](math-expressions-rs/src/expr/tree.rs#L20); `MathConst` = Pi/E/I/Inf/-Inf/NaN) | R2 needs a real boolean leaf in `Expr` + printer + serde — a medium core change, not a one-liner |
| R6 needs new machinery | `compileRustExpr` exists and is exported from the wasm package ([`src-js/index.ts:16`](math-expressions-rs-wasm/src-js/index.ts#L16)) | R6 = ~4 lines of compat wiring |
| R13 is a ~5-line `number_to_js` change (issue's framing) | **False, measured.** `1/2` and `0.5` are the *same* `Expr` (`Rat(1,2)`) — decimals parse to exact rationals. The issue's patch verbatim breaks **27** `parse_matches_js` fuzz cases | R13 re-scoped into R13a (safe, ship now) / R13b (needs a provenance decision) — see §2 |
| R13 is confined to `serde.rs` | The **printers** decimalize independently (`terminating_decimal` rule, [`print/text.rs:205`](math-expressions-rs/src/print/text.rs#L205)) — that is the `toLatex → "0.5"` complaint | R13b spans serde *and* both printers |

---

## 1. Priority & sequencing overview

| # | Item | Fix lives in | Effort | Phase |
| --- | --- | --- | --- | --- |
| **R13a** ✅ | Non-terminating rationals lose precision (`1/3` → `0.333…`) | Core (`serde.rs`) | **S** | 1 — **done** |
| **R13b** | Terminating rationals decimalize (`1/2` → `0.5`) | — | — | **DoenetML-side** |
| **R5** ✅ | Context-level op family (`me.simplify(expr)`) | Compat | **S** | 1 — **done** |
| **R6** ✅ | `Expression#f()` numeric evaluator | Compat | **S** | 1 — **done** |
| **R4** ✅ | `get_component()` unbound (+ matrices) | Core + WASM + compat | **S–M** | 1 — **done** |
| **R10** ✅ | `substitute_component()` unbound | Core + WASM + compat | **S** | 1 — **done** |
| **R2-bool** ✅ | `from_ast` rejects `boolean` leaves | Core (`Expr` + serde) | **M** | 1 — **done** |
| **R2-null** | `from_ast` rejects `null` leaves | Core / DoenetML (source unknown) | **S** | 2 |
| **R3** | Render options silently dropped | Core (printers) + WASM + compat | **M** | 2 |
| **R7** | Passes that silently no-op | Core + WASM + compat | **M** | 2 |
| **R1** | Browser/worker WASM loading seam | WASM(js) + compat (+ Doenet) | **M** | 2 |
| **R8** | Handle lifetime / `Sym` interner unbounded | Compat + Core | **M–L** | 3 |
| **R9** | `panic = "abort"` + WASM32 stack safety | Core | **L** | 3 |
| **R11** | MathML input (`mmlToAst`) | — (confirm not needed) | — | — |
| **R12** | Publish `3.0.0-alpha` to npm | Process | **S** | 4 |

**Phase 1 — quick blocking wins** (R13, R5, R6, R4, R10, R2-bool): small, mostly
compat + contained core edits; clears most of the 32 reported failures. **Landed
in full — see §8.**
**Phase 2 — rendering & normalization fidelity** (R3, R7, R2-null, R1).
**Phase 3 — robustness & lifecycle** (R8, R9).
**Phase 4 — release** (R12).

---

## 2. Phase 1 — quick blocking wins

### R13 — exact rationals crossing to JS  *(highest priority — and NOT a 5-line change)*

> **Corrected after a code spike.** The issue proposes "emit `["/", num, den]` for
> `Number::Rat`". **That fix as stated is wrong** and breaks 27 fuzz cases. R13 is
> really two items with very different difficulty. Evidence below is measured, not
> inferred.

**The blocker: `Number::Rat` has no provenance.** User-typed decimals parse to
*exact rationals*, by deliberate design
([`num/number.rs:3`](math-expressions-rs/src/num/number.rs#L3): "User-typed decimals
parse to exact rationals … never `Float`"). So `1/2` and `0.5` are **the identical
`Expr`**:

| input | internal `Expr` | `to_text` | `to_latex` | `tree_json` (today) |
| --- | --- | --- | --- | --- |
| `1/2` | `Num(Rat(1,2))` | `0.5` | `0.5` | `0.5` |
| `0.5` | `Num(Rat(1,2))` | `0.5` | `0.5` | `0.5` |
| `cos(pi/3)` | `Num(Rat(1,2))` | `0.5` | `0.5` | `0.5` |
| `1/3` | `Num(Rat(1,3))` | `1/3` | `\frac{1}{3}` | **`0.3333333333333333`** |
| `5/6` | `Num(Rat(5,6))` | `5/6` | `\frac{5}{6}` | **`0.8333333333333334`** |
| `19.9` | `Num(Rat(199,10))` | `19.9` | `19.9` | `19.9` |

Nothing at the JS boundary can separate `1/2` from `0.5` — the distinction is
destroyed at parse/canonicalize time. **Measured:** applying the issue's proposed
patch verbatim makes `cargo test --workspace` fail `parse_matches_js` with **27
divergences**, all decimal literals turning into fractions (`19.9` →
`["/",199,10]`, `tan(14.2)` → `["apply","tan",["/",71,5]]`).

Note also the printers use a **different** rule than serde:
`render_number` ([`print/text.rs:205`](math-expressions-rs/src/print/text.rs#L205))
emits a positional decimal for any *terminating* rational (denominator 2^a·5^b) and
`a/b` otherwise — which is why `1/3` prints correctly but `1/2` does not. So the
issue's `(2/4).toLatex() → "0.5"` complaint is a **printer** defect, not a serde
one. R13 spans both sites.

#### R13a — non-terminating rationals lose information *(safe, small, do now)*

`1/3` → `0.3333333333333333` in `tree_json` is **irreversible loss** and has no
provenance ambiguity: a non-terminating rational can never have come from a decimal
literal. The printers already get this right; only serde is wrong.

```rust
Number::Rat(num, den) if !is_terminating(den) => json!(["/", num, den]),
```

- Fixes the issue's `1/3`, `1/2+1/3` cases.
- **Sign convention — RESOLVED empirically:** the JS fixtures spell negative
  rationals sign-on-numerator (`["/",-2,3]` in `ast-to-latex.json`,
  `["/",-5,8]` in `expand-corpus.json`, `["/",-1,2]` in `simplify-corpus.json`),
  which matches the Rust `Rat` normal form (`den > 0`, sign on `num`,
  [`num/number.rs:60`](math-expressions-rs/src/num/number.rs#L60)). Emit directly.
- **Risk: very low.** `simplify_corpus` compares by `equals`, not tree-match
  ([`simplify_corpus.rs:167`](math-expressions-rs/tests/simplify_corpus.rs#L167)), and
  JS-tree agreement is explicitly advisory — so `0.5`-vs-`["/",1,2]` never gated
  anything. The corpus fixtures *already* expect `["/",1,3]`, `["/",5,6]`,
  `["/",-1,2]`, so this moves Rust **toward** the fixtures.

#### R13b — terminating rationals need a provenance decision *(the real work)*

`1/2`, `3/4`, `cos(pi/3)` are indistinguishable from `0.5`, `0.75`. Getting
`cos(pi/3) → ["/",1,2]` **and** keeping `19.9 → 19.9` requires distinguishing
decimal-origin from fraction-origin values. Options, for a maintainer call:

1. **Track provenance in `Number`** (a decimal-origin marker, or a distinct
   `Number::Dec` tier). Fully correct; touches the parser, arithmetic
   (propagation rules), both printers, and serde. **Effort: M–L.**
2. **Make it an explicit render/serialize option** — e.g. a `preferFractions`
   notation flag, which dovetails with **R3**'s options work. Cheap and gives
   DoenetML exactly the behavior it wants globally, but a document mixing decimals
   and fractions cannot get both right, and `0.5` typed by a student would echo
   back as `1/2` (bad in a *decimals* lesson).
3. **Do nothing for R13b** — ship R13a, and tell DoenetML that terminating
   rationals render as decimals. Given they call fractions "a first-class teaching
   subject", this likely fails their requirement.

*Recommendation:* ship **R13a now** (it is unambiguous and fixes real data loss),
and treat **R13b** as a scoped design task — option 1 if fraction fidelity must be
per-value, option 2 if a document-level policy is acceptable. **Confirm with
DoenetML which they need**, since option 2 is far cheaper.

**Bonus severity the issue did not note:** because `tree_json` → `fromAst`
round-trips `Rat` → `Float`, DoenetML's `serializedComponentsReviver` **degrades
exact values on every state save/load**, not merely on display. R13a stops that for
non-terminating rationals.

### R5 — regenerate the context-level operation family  *(compat only)*

**Root cause.** `Context` ([`math-expressions.ts:411`](math-expressions-js-compat/lib/math-expressions.ts#L411))
stops at the factories; the legacy `me.simplify(expr)` free-function form is
absent. This one gap caused **all 79** `math.test.ts` cases to fail in the
issue's first run.

**Fix (compat).** After `Context` is defined, mirror `Expression.prototype` onto
it, expression-first, without shadowing existing members (factories win):

```ts
for (const name of Object.getOwnPropertyNames(Expression.prototype)) {
  if (name === "constructor" || name in Context) continue;
  const d = Object.getOwnPropertyDescriptor(Expression.prototype, name);
  if (typeof d?.value !== "function") continue;   // skip accessors (`tree`)
  (Context as any)[name] = (expr: ExpressionLike, ...args: unknown[]) =>
    (Context.from(expr) as any)[name](...args);
}
```

DoenetML offered this as a PR — accept it. **Effort: S.**

### R6 — wire `Expression#f()`  *(compat only)*

**Root cause.** `f` is in the `notImplemented` list
([`math-expressions.ts:362`](math-expressions-js-compat/lib/math-expressions.ts#L362))
even though `compileRustExpr` exists and is exported
([`src-js/index.ts:16`](math-expressions-rs-wasm/src-js/index.ts#L16)).

**Fix (compat).** Remove `"f"` from `notImplemented` and add:

```ts
import { compileRustExpr } from "math-expressions-rs-wasm";
import math from "./mathjs";           // the math.js instance, NOT the wasm module
Expression.prototype.f = function (this: Expression) {
  const compiled = compileRustExpr(math, this._w);
  return (bindings: Record<string, number>) => compiled.evaluate(bindings);
};
```

**Signature verified:** `compileRustExpr(math: MathJsInstance, expr: RustExprLike,
options?: { normalize?: boolean })`
([`tree-to-mathjs.ts:527`](math-expressions-rs-wasm/src-js/tree-to-mathjs.ts#L527)) —
the first argument is the **math.js instance**, and `math` is already imported in
the compat module. It normalizes via a temporary handle and frees it internally;
pass `{normalize:false}` only if the expression is already normalized. **Effort: S.**

### R4 — bind `get_component()` and extend it to matrices

**Root cause.** [`ops/query.rs::get_component`](math-expressions-rs/src/ops/query.rs#L74)
exists but (a) has no WASM binding and (b) only handles `Expr::Seq`, returning
`None` for `Expr::Matrix`.

**Fix.**
1. **Core:** extend `get_component`/`substitute_component` to index `Expr::Matrix`
   (row-major). **Indexing is 0-based** and already documented as a port of
   `me.get_component` ([`ops/query.rs:71`](math-expressions-rs/src/ops/query.rs#L71)),
   so the existing `Seq` behavior needs no change — only the `Matrix` arm is new
   (confirm row/col order against the 63 call sites).
2. **WASM:** add `Expression::get_component(&self, i: usize) -> Option<Expression>`
   in `core_ops.rs` (mirrors `copy`/`substitute_var`).
3. **Compat:** remove `"get_component"` from `notImplemented`, add
   `get_component(i) { return wrap(this._w.get_component(i), this.context); }`.

**Effort: S–M** (the matrix indexing + call-site semantics are the only real
work). Clears 3 of the 32 failures and is load-bearing for editing
(`EssentialValueWriter.ts`).

### R10 — bind `substitute_component()`  *(minor)*

Same shape as R4 for [`ops/query.rs:83`](math-expressions-rs/src/ops/query.rs#L83):
add the matrix case, a WASM binding, and compat wiring. One call site; do it
alongside R4 since they share code. **Effort: S.**

### R2 (part 1) — boolean leaves

**Root cause.** The published `Tree` type allows `boolean`
([`math-expressions.ts:14`](math-expressions-js-compat/lib/math-expressions.ts#L14))
but `Expr` has **no boolean literal** — `try_from_js` falls through to
`Err("unexpected value …")` ([`serde.rs:44`](math-expressions-rs/src/expr/serde.rs#L44)),
so `["and", true, false]` is unconstructible.

**Design decision (needs a call — see §6).** Give booleans a home in `Expr`.
*Recommended: `Expr::Bool(bool)`.* Folding them into `MathConst` looks smaller but
is a poor fit: every other `MathConst` serializes to a JSON **string** (`"pi"`,
`"e"`) or a special object (`{"$":"Inf"}`), whereas a boolean must serialize to a
JSON **boolean** — and `MathConst` means *mathematical constant* (π, e, i, ∞), not
a truth value. A distinct variant keeps both the serde shape and the semantics
honest. Then:
- `try_from_js`: `Value::Bool(b) => Ok(<bool leaf>)`.
- `to_js`: emit JSON `true`/`false` for the leaf (closes the type-faithful
  round-trip — symbol-mapping to `"true"`/`"false"` does **not**, since it comes
  back as a string).
- Printers (`text.rs`, `latex.rs`) and any exhaustive `match` on `Expr`/`MathConst`
  gain arms — the compiler enumerates them.

**Effort: M** (mechanical once the representation is chosen). The `null` half of
R2 moves to Phase 2 (§3).

---

## 3. Phase 2 — rendering & normalization fidelity

### R3 — honor render options (or throw)

**Root cause.** Compat's `toLatex()/toString()/tex()` call the no-arg bindings
and ignore any options object
([`math-expressions.ts:112`](math-expressions-js-compat/lib/math-expressions.ts#L112)).
The WASM `*_with_options` bindings exist but only read `notation`/`unicode`; the
core `LatexOpts` has a single field (`notation`) and `TextOpts` two
([`print/latex.rs:12`](math-expressions-rs/src/print/latex.rs#L12),
[`print/text.rs:19`](math-expressions-rs/src/print/text.rs#L19)).

**Fix (three layers, options exist — fields do not).**
1. **Core printers:** add fields to `LatexOpts`/`TextOpts` and implement them in
   the number-rendering path: `pad_to_decimals`, `pad_to_digits`, `show_blanks`,
   `explicit_multiplication_symbols`. (Padding hooks into the existing
   `f64_positional_string`/number emitter; `show_blanks` controls how `Expr::Blank`
   renders.)
2. **WASM readers:** extend `to_latex_with_options`/`to_text_with_options`
   ([`core_ops.rs:41`](math-expressions-rs-wasm/src-rust/core_ops.rs#L41)) with
   `read_opt_*` calls for the new keys, mirroring `read_opt_bool`.
3. **Compat:** forward the options object — `toLatex(opts)` →
   `to_latex_with_options(JSON.stringify(opts))` when `opts` is non-empty.

**Minimum bar** (if the printer work is deferred): make an unrecognized/non-empty
options object **throw** rather than silently no-op, so callers discover the gap.
Full implementation is required for DoenetML Stage 2 (`eval-math.ts` passes these
from the Rust core). Named counts: `padToDecimals` 9, `padToDigits` 9,
`showBlanks` 11, `explicitMultiplicationSymbols` 1. **Effort: M.**

### R7 — real passes, or loud failures

**Root cause.** Five passes are compat no-ops returning `this`
([`math-expressions.ts:377`](math-expressions-js-compat/lib/math-expressions.ts#L377)),
and `evaluate_numbers(_opts)` ignores its argument
([`math-expressions.ts:191`](math-expressions-js-compat/lib/math-expressions.ts#L191)).
Three are user-facing DoenetML features (`simplify="normalizeorder"`,
`"numberspreserveorder"`, answer grading).

**Fix — split by need (issue's own guidance: implement the two we need, throw the rest).**
- **`default_order()`** — add a WASM binding over the core `normalize::order`
  path (order without full aggressive simplify), compat forwards. *Implement.*
- **`evaluate_numbers({skip_ordering})`** — thread a `skip_ordering` option
  through the WASM `evaluate_numbers` binding
  ([`core_ops.rs:193`](math-expressions-rs-wasm/src-rust/core_ops.rs#L193)) into the
  core pass; compat passes the flag through instead of dropping it. *Implement.*
  (Same silent-ignored-option bug class as R3.)
- **`normalize_applied_functions`, `normalize_negative_numbers`,
  `expand_relations`, `applyAllTransformations`** — change the compat stubs from
  silent no-ops to `notImplemented(name)` so answer-grading paths surface the gap
  at the call site instead of silently changing behavior. *Throw* until a
  consumer proves it needs the real pass.

**Effort: M.**

### R2 (part 2) — the `null` leaves

`Tree = number | string | boolean | Tree[]` has no `null`, so `me.fromAst(null)`
erroring is arguably *correct* — the defect is the unclear error and the six
downstream failures. **Action:** improve the `try_from_js` error to name the path,
and **investigate the source** of the `null`s DoenetML feeds in (the issue notes
they cluster around display-rounding / units / blanks — likely a blank `＿`
serialized as `null` somewhere). This may resolve to a **DoenetML-side fix** (emit
the blank symbol, not `null`) or a decision to accept `null` → `Expr::Blank`.
Coordinate before choosing. **Effort: S** on our side once the source is known.

### R1 — a supported browser/worker loading seam

**Root cause.** [`lib/_wasm.ts`](math-expressions-js-compat/lib/_wasm.ts) is
node-only (`createRequire` over the `nodejs`-target build). DoenetML runs in a Web
Worker and had to alias the internal module path.

**Fix — provide the seam; let DoenetML own byte-loading.** The WASM `web` build
already exists; what's missing is an injection point:
- Add a `setWasmModule(glue)` / `math-expressions/web` entry that accepts an
  already-initialized `--target web` module, so the compat layer uses an injected
  module instead of hard-wiring the node loader.
- Preserve a **synchronous** path (`initSync` from inlined bytes) — the legacy
  API is sync and ~150 DoenetML files call it without `await` — and expose an
  async `init(bytesOrUrl)` for the browser main thread.
- **No `fetch`:** instantiate from an `ArrayBuffer` only (VS Code web-worker host
  blocks blob/data-URL fetch).

**Ownership split (see §5):** *math-expressions* ships the injection API and a
`web` entry; *DoenetML* keeps its base64-inlining + `initSync` glue (bundling is a
consumer concern). This deletes DoenetML's fragile internal-path alias.
**Effort: M.**

---

## 4. Phase 3 — robustness & lifecycle

### R8 — bound memory: handles and the `Sym` interner

**Root cause.** Compat `Expression` handles are never freed except in a few
throwaway spots ([`math-expressions.ts:282`](math-expressions-js-compat/lib/math-expressions.ts#L282));
the `Sym` interner is append-only. DoenetML's long-lived worker mints an
expression per state-variable eval and per state-JSON round-trip → unbounded
growth.

**Fix — phased, in preference order.**
1. **`free()`/`dispose()` on compat `Expression`** (+ the `__wbg_ptr !== 0` guard
   the playground already uses, [`engines.ts:72`](playground/src/engines.ts#L72)).
   Cheap, immediate, lets hosts own lifetimes. **S.**
2. **Cap / evict the `Sym` interner** (core) — no consumer-side discipline can fix
   this one. Needs an eviction policy or a per-context arena. **M.**
3. **Value-first `Expression`** (hold the plain `Tree` as canonical state,
   materialize a handle only for an operation's duration). Also removes the
   uncached `get tree()` `JSON.parse` cost on DoenetML's two hottest calls
   (`fromAst` ~675, `.tree` ~600 sites). Largest change; **L**; can be a
   follow-up once (1)+(2) stop the bleeding.

**Effort: M–L overall.** Do (1) immediately; schedule (2); treat (3) as a
separate design.

### R9 — panic firewall + WASM32 stack safety

**Root cause.** `panic = "abort"` ([`Cargo.toml:22`](Cargo.toml#L22)) turns any
reachable panic into a dead worker (JS raised catchable exceptions);
`ARCHITECTURE_REVIEW.md` still lists reachable panics (`Number::from_decimal_str`)
and `STACK_SAFETY_PLAN` items 21/23–26 are open (deep `Expr` overflows the ~1 MB
shadow stack, **including on `Drop`**). Student input is adversarial.

**Fix.**
- **Keep `panic = "abort"`** (the size win, −35% wasm) and instead **eliminate
  reachable panics at the boundary**: audit `unwrap`/`expect`/`assert`/`panic!` on
  any path reachable from a `#[wasm_bindgen]` entry and return `Result`
  (`from_decimal_str` first).
- **`STACK_SAFETY_PLAN` item 21 — iterative `Drop` for `Expr`** (the plan's
  sequencing step 1, "smallest change, kills a whole crash class"). Gets *more*
  important if R8 frees aggressively.
- Consider the bounded-boundary depth guard from `STACK_SAFETY_PLAN §3` for the
  parse/serde entry points.

**Effort: L.** Reference and advance `STACK_SAFETY_PLAN`; this is the one item
that is genuinely deep work.

---

## 5. What is better changed on the DoenetML side

Recording the ownership split so it is explicit:

- **R1 byte-loading glue.** math-expressions provides the injection seam
  (`setWasmModule` / `web` entry, sync + async init); **DoenetML keeps** the
  base64-inlining and `initSync`-from-bytes — that is bundling policy specific to
  the VS Code web-worker host, not something the library should encode.
- **R2 `null`.** If the `null`s trace back to DoenetML serializing a blank as
  `null`, the fix is **DoenetML emitting the `＿` blank symbol** (or an agreed
  `null → Blank` mapping); math-expressions only owns the clearer error.
- **§5 of the issue — divergences DoenetML absorbs (no change requested):** the
  more aggressive `simplify` (feature, not bug), exact-constant equality, and
  pure-presentation formatter differences. DoenetML **updates its own tests** to
  compare parsed trees / `equals` rather than exact strings. The 15 "assertion
  divergence" failures of the 32 are DoenetML-side test updates.
- **R8 value caching (partial).** Even before a value-first `Expression`,
  DoenetML can cache `.tree` at its call sites; the interner cap, however, must
  be library-side.

Everything else (R13, R3, R4, R5, R6, R7-implement, R9, R10) is
math-expressions-side, most of it in the compat layer or a contained core edit.

## 5b. No action

- **R11 MathML input.** No `fromMml`/`mmlToAst` call sites in DoenetML; the
  `WHATS_LEFT.md §A.1` gap is confirmed not needed. Leave `fromMml` as
  `notImplemented`; record the confirmation. GLSL/Guppy/MathML-output/`mathjsToAst`
  likewise have no consumers.

---

## 6. Open decisions (need a maintainer call)

1. ~~**R13b provenance**~~ — **RESOLVED 2026-07-31: DoenetML owns the round-trip
   serialization fix.** math-expressions ships R13a and does not track
   decimal-origin provenance. If DoenetML later needs per-value fraction fidelity
   in the *printers* (`(2/4).toLatex() → "0.5"`), reopen as a separate item — that
   is a display question, distinct from the round-trip data question now settled.
2. ~~**R2 boolean representation**~~ — **RESOLVED 2026-07-31: `Expr::Bool(bool)`**,
   shipped. `MathConst::True/False` and symbol-mapping both lose the JSON type on
   the way out; see §2. The follow-on question this raised — whether interval
   closures and `lts`/`gts` strictness flags should become `Expr::Bool` children —
   was answered **no**, deliberately; see §8.
3. **R7 scope** — confirm implement-`default_order`+`skip_ordering`, throw the
   other three (per the issue's own preference).
   *(R13 negative-rational spelling: **resolved** — sign-on-numerator, matches
   fixtures and the `Rat` normal form. No longer an open question.)*
4. **R3 depth** — implement the four printer options now (needed for Stage 2) vs
   ship throw-on-unsupported first. *Recommendation: implement.*
5. **R9 policy** — keep `panic="abort"` + Result-ify reachable panics (recommended)
   vs switch to `panic="unwind"` for wasm and `catch_unwind` at the boundary
   (size cost, simpler firewall).
6. **R8 depth** — how far this cycle: (1) `free()`/`dispose()` now, (2) interner
   cap next, (3) value-first `Expression` as a tracked follow-up. *Recommendation:
   (1)+(2) now, (3) later.*

---

## 7. Suggested first PR (smallest thing that unblocks the most)

**PR 1 — no design decisions required, all verified low-risk:**
**R5 + R6 + R4 + R10 + R13a.** R5/R6 are pure compat (and R5 is offered to us as a
PR); R4/R10 are a core `Matrix` arm plus two bindings; R13a is a contained serde
change whose risk I measured (corpus compares by `equals`, fixtures already expect
the exact spelling). Nothing here is blocked on a maintainer call.

**Deliberately *not* in PR 1:** R2-bool (blocked on the representation decision) and
R13b (blocked on the provenance decision). Bundling a decision-blocked M-item with
five unblocked S-items would stall the whole PR — the mistake in this plan's first
draft.

*(Both decisions landed the same day; R2-bool went in immediately after PR 1 and is
part of the same change set.)*

**PR 2:** R3 + R7 once decisions land. **PR 3+:** R1, then R8/R9 as their own
tracks.

### Verification per item (this repo has strong differential infrastructure — use it)

| Item | How we know it worked |
| --- | --- |
| R13a | `cargo test --workspace`; confirm `parse_matches_js` stays green (the canary that caught the naive fix); add corpus rows for `1/3`, `5/6` |
| R4/R10 | New `tests/` cases for `Seq` **and** `Matrix` indexing incl. out-of-range → `None`; wasm e2e for the binding |
| R5/R6 | js-compat spec; re-run DoenetML's `math.test.ts` (their 79-case suite is the real oracle) |
| R2-bool ✅ | Round-trip property: `from_ast(t).tree === t` for `true`/`false` leaves — done, plus a test that a boolean is not the *symbol* `"true"`, and one that the flag tuples did **not** become boolean children |
| R3/R7 | Assert options actually change output — a test that would pass against a silent no-op is worthless |
| all | Track the js-compat differential baseline — it must not regress |

---

## 8. What Phase 1 actually landed (2026-07-31)

**R5 + R6 + R4 + R10 + R13a**, as scoped in §7. Measured, both sides rebuilt from
the same source: js-compat differential **1461 → 1441 failing** (+20 passing,
zero newly failing); `cargo test --workspace` fully green including
`parse_matches_js`; the wasm e2e suite 51/51; 14 pre-existing `tsc` errors
removed, none added.

Two things the plan had wrong, found while implementing:

### Component paths are indexed over the **flattened** tree

`get_component` had to index the same tree the caller sees, and `expr.tree` is
`expr::serde::to_js`, which **flattens first**
([`serde.rs:224`](../packages/math-expressions-rs/src/expr/serde.rs#L224)). `Expr`
keeps associative operators as the parser nested them, so `x+y+z` is
`Add[Add[x, y], z]` — two components — while the JS tree is
`["+", "x", "y", "z"]` — three. Indexing the unflattened tree would have silently
disagreed with every call site. The agreement is now pinned by a test that walks
`components()` against `to_js`'s operand array.

### The API takes a **path**, not an index — and it is not sequence-only

The plan described `get_component(i)` over `Expr::Seq`. The ported JS spec
([`quick_transformation.spec.ts:241`](../packages/math-expressions-js-compat/spec/quick_transformation.spec.ts#L241))
shows the real surface: `get_component([2, 1, 2])` walking nested tuples, and
indexing works on *any* operator node (`["+", "x", 1]` has components `x` and
`1`), not just sequences. So the core signature is `&[usize]`, and the component
list is read off the JS spelling generically.

That also settles the matrix question the plan flagged as "confirm row/col order":
**there is no choice to make** — a matrix is
`["matrix", ["tuple", rows, cols], ["tuple", <row-tuples>]]`, so its component `0`
is the dimension pair and an entry is the path `[1, row, col]`. Following the JS
spelling is the only option that cannot disagree with the 63 call sites.

Shapes whose JS spelling carries boolean flags — `interval` and the `lts`/`gts`
mixed relation chains — still have no component list. See the R2 note below: the
boolean leaf alone does **not** close that, and closing it was deliberately
deferred.

### Where the code lives

- [`ops/components.rs`](../packages/math-expressions-rs/src/ops/components.rs) — new module (the old
  `Seq`-only pair is gone from `ops/query.rs`)
- [`expr/serde.rs`](../packages/math-expressions-rs/src/expr/serde.rs) — `number_to_js` split on
  `terminating_decimal()`, plus an out-of-JS-integer-range guard
- [`core_ops.rs`](../packages/math-expressions-rs-wasm/src-rust/core_ops.rs) — the two bindings
- [`math-expressions.ts`](../packages/math-expressions-js-compat/lib/math-expressions.ts) — `f()`,
  the component methods, and the `Context` mirror

One correction to §2's R5 snippet, which is also what the issue proposed: it
coerced with `Context.from(expr)`, but the argument is normally an `Expression`
already and `from` would read that as an AST. The landed version uses `toExpr`.

### R2-bool, landed the same day (2026-07-31)

`Expr::Bool(bool)` ([`expr/tree.rs`](../packages/math-expressions-rs/src/expr/tree.rs)),
both serde directions, both printers, and arms in the ten exhaustive matches the
compiler enumerated. `["and", true, false]` is constructible; a boolean crosses
back as a JSON boolean, not the string `"true"`.

**No local number moved, and that is expected.** The js-compat differential stayed
at 1441 failing / 4744 passing, `cargo test --workspace` and clippy stayed green,
the wasm e2e suite stayed 51/51, `tsc` stayed clean. Nothing in this repo's suites
feeds a boolean *operand* — the parsers cannot produce one, and every `true` in the
ported specs is an interval/`lts` flag tuple, which already worked. The oracle for
this item is DoenetML's own suite. Verified here end to end by the round-trip tests
in `expr/serde.rs` and by a throwaway compat spec exercising
`me.fromAst(["or", ["and", true, "x"], false]).tree`.

**Two things chosen deliberately, worth knowing before extending this:**

*The printers are display-only for booleans.* Text prints `true`/`false` and LaTeX
prints `\operatorname{true}`; neither re-parses to a boolean, because no parser can
produce `Expr::Bool` — there is no text or LaTeX spelling to add. This breaks the
printers' usual round-trip contract for exactly this leaf. The AST round-trip is
the one DoenetML depends on, and it is exact.

*Interval closure and `lts`/`gts` strictness stay metadata.* They are **not**
`Expr::Bool` children, and a test now asserts they never become any. Two reasons:
the flags carry more than a bool (`("lts", false)` is `Le`, `("gts", false)` is
`Ge` — a bare boolean loses which head it was under), and the metadata is what
makes `operands.len() == ops.len() + 1` structural instead of a runtime check.

The consequence is that the boolean leaf did **not** complete component access for
those shapes, as §8 above first assumed. Doing so needs `components()` to
*synthesize* a `Bool` from the metadata and `rebuild()` to *destructure* it back —
and that introduces the first case where `substitute_component` fails on a **value**
rather than a path (`substitute_component(interval, [1,0], parse("x"))` has nowhere
to put an `x`). That is a real design commitment, so it waits for a call site that
needs it. None exists today: every DoenetML failure in this area is a `fromAst`
rejection, not component access into a closure tuple.

---

## 9. PR #84 measurement follow-up (2026-07-31)

DoenetML built `@doenet/math` against #84 and ran its **worker** suite: 3469 tests,
969 failing (72.1%), all genuine engine-swap divergences (JS control: 0 failures).
Their ranked asks and what landed here (numbers are their `#`, not the R-items):

| # | Ask | Failures | Landed |
| --- | --- | ---: | --- |
| **1** | `perform_vector_matrix_additions_scalar_multiplications` missing | 431 (44.5%) | ✅ core [`ops/vector_matrix.rs`], wasm binding, compat method |
| **2** | `{"$":"None"}` emitted then rejected by `from_ast` | 90 (9.3%) | ✅ `MathConst::None`, serde both directions |
| **3a** | `fromAst(NaN/±Inf)` → `null` (JSON) → rejected | part of 76 | ✅ `astReplacer` on the compat `fromAst` stringify |
| **3b** | undefined quantities eval to `0` not `NaN` | part of 76 | ⏳ needs DoenetML's offered bisect (`evaluate_to_constant`?) |
| **4** | `me.math` ≠ legacy injected instance (`dopri`) | 5 | ⏳ decision: is `me.math` a compat instance or plain mathjs? |
| **5** | render options silently dropped | — (blocks Stage 2) | ◑ **min-bar**: compat now *throws* on a requested unsupported option (`padToDecimals`/`padToDigits`/`showBlanks`/`explicitMultiplicationSymbols`); full printer impl still owed |
| **6** | passes silently no-op | — (3 features) | ◑ `evaluate_numbers({skip_ordering:true})` now throws (was silently reordering); the five passes **stay no-ops** — see below |
| 7/8/9 | browser loader / handle freeing / panic-abort | — (ship blockers) | ⏳ = R1/R8/R9, unchanged |

### Why the five passes stayed no-ops (not throws)

The issue said "implement or throw — either is fine." **Measured: throwing is not
fine here.** Converting `default_order`/`normalize_negative_numbers`/
`normalize_applied_functions`/`expand_relations`/`applyAllTransformations` to
`notImplemented` **regressed ~170 currently-passing js-compat specs** (4753 → 4584)
and aborted whole spec files at collection (total 6200 → 5866) — those inputs are
idempotent, so the no-op *is* the right answer and the specs legitimately pass on
the unchanged tree. So they remain no-ops until **properly implemented**.
`default_order` specifically needs the **JS ordering key** ported (legacy
`3+x → ["+","x",3]`, symbol-first), not Rust's canonical `cmp` (number-first) — a
`cmp`-based version would silently disagree with `simplify="normalizeorder"`, i.e.
the same silent-wrong class. That port is the real R7 work.

`evaluate_numbers({skip_ordering})` **is** now a throw: it is a *distinct* option
that was being silently ignored (reordering `1+x+2` to `x+3`), it has no core
order-preserving mode, and rejecting it regressed nothing (specs don't pass it in a
file-aborting way). Item 5's guard likewise regressed nothing (total held at 6200).

### Verification

- Rust: `cargo test` green (59 suites), clippy clean; new unit tests in
  [`ops/vector_matrix.rs`], serde `the_none_special_round_trips_…`, and an integration
  test in [`tests/doenet_utils.rs`].
- js-compat differential: **4753 → 4756 passing, 1444 → 1441 failing, zero
  regression, total held at 6200**; end-to-end coverage in
  `spec/quick_doenet_compat_pr84.spec.ts` (12/12). The real oracle for items 1–3 is
  DoenetML's own worker suite, not reproducible here (no DoenetML checkout).

### Still owed / needs a call

- **3b, 4** — need DoenetML's bisect (3b) and a `me.math` decision (4).
- **5 full** — implement the four printer options (core `LatexOpts`/`TextOpts`
  fields + wasm readers + compat forwarding); required for Stage 2.
- **6 real** — port the JS ordering key for `default_order`; give the other four
  real passes or confirm they can stay no-ops.
- **Rebuild note:** the compat package's **vendored** wasm (`vendor/wasm/`, via
  `npm run build:wasm`) must be regenerated for the core changes (items 1/2) to take
  effect — it is separate from `math-expressions-rs-wasm/pkg`.

---

## 10. PR #84 follow-up, round 2 (2026-08-01)

Maintainer answers to the round-1 deferred items landed the rest of the list.

| # | Ask | Landed |
| --- | --- | --- |
| **3b** | undefined quantity evaluates to `0`/`1` not `NaN` | ✅ **root-caused & fixed.** `evaluate_to_constant` ran `simplify_core` *first*, which absorbed the hole (`0·＿ → 0`, `＿/＿ → 1`) before eval saw it. Now guards on a blank/`None` leaf *before* simplifying, like it already did for free variables. [`ops/evaluate.rs`] |
| **4** | `dopri` (Doenet dropping mathjs) | ✅ implemented as a **peer** compat export (`me.dopri` + named `dopri`), not under `me.math` — numeric.js `dopri(x0,x1,y0,f,tol,maxit)` contract with `.at()`/`.x`/`.y`, backed by the existing Rust `solve_ode`. [`math-expressions.ts`] |
| **5** | render options (Stage-2 blocker) | ✅ **all four implemented** end to end: `padToDigits`/`padToDecimals`/`showBlanks`/`explicitMultiplicationSymbols`, in `TextOpts`/`LatexOpts` + the number/blank/mul printers ([`print/*`], padding is a faithful port of `pad-numbers.js`), read by the wasm `*_with_options` entry points, forwarded by compat `toString/toText/toLatex/tex`. |
| **7** | browser/worker loader seam | ✅ `_wasm.ts` is now a **swappable provider**: `setWasmModule(mod)` injects an initialized `--target web` module; node/Vitest keep the lazy vendored fallback. `setWasmModule` is exported from the entry. Bundling (base64 inline + `initSync`, marking `node:module` external) stays host-side per R1. |
| **8** | handle lifetime / interner | ◑ `free()`/`dispose()`/`[Symbol.dispose]` on compat `Expression` (idempotent, nulls the handle) — the immediate win. `interner_size()` exposed (core `interner_len` + wasm binding + `me.interner_size()`) so growth can be **measured**. True eviction still owed — see below. |
| **9** | panic-abort + stack safety | ◑ **item 21 done**: iterative [`expr::tear_down`] dismantles a deep tree with a heap worklist (verified freeing a 200 k-deep tower in a 128 KiB thread), called from the wasm `Expression` `Drop`. `from_decimal_str` no longer `panic`s on a non-digit token (falls back to the JS `parseFloat` value). Boundary audit 23–26 still open — see below. |

### Design notes / still owed

- **8 — true interner eviction needs a redesign, not a cap.** A `Sym` is a raw
  `u32` index into the append-only `names` table, so evicting or compacting would
  dangle every live `Sym`. A safe cap/clear is impossible without generational or
  ref-counted symbols (or the flat-arena `Expr` from STACK_SAFETY_PLAN §4). Shipped
  the **gauge** (`interner_size()`) so DoenetML can send the growth numbers they
  offered; size the redesign against those.
- **9 — `impl Drop for Expr` is blocked by E0509.** `Expr` derives `Clone` and is
  destructured by value crate-wide (`match e { Expr::Add(xs) => … }`); a `Drop`
  impl makes every such move a borrow error. So teardown is a free function called
  from the ownership sink (the wasm handle) instead. The remaining boundary panics
  (STACK_SAFETY_PLAN 23–26: iterative `Clone`/`PartialEq`/`Hash`, the rest of the
  `unwrap`/`expect` audit) are the deeper, still-open part of item 9.
- **6 — `default_order`** still needs the JS ordering key ported (round-1 note).

### Verification (round 2)

- Rust: `cargo test` green (59 suites) + clippy clean, both crates; new tests in
  `ops/evaluate.rs` (3b), `print/mod.rs` (padding), `expr/teardown.rs` (deep-tree
  free on a 128 KiB stack).
- js-compat differential: **4753 → 4765 passing, zero regression, total steady**
  (~6206); `spec/quick_doenet_compat_pr84.spec.ts` covers all of items 1–9 (19/19),
  including the dopri scalar+system solve and the render options.
- **Rebuild note still applies:** regenerate the vendored wasm
  (`npm run build:wasm` in js-compat) after any core/wasm change.

---

## 11. Review of the round-2 work (2026-08-04)

A review of `c110a56..02293bf` found eleven defects, all now fixed. Three
returned **silently wrong numbers** — the failure shape DOENET_INTEGRATION.md
§1 argues is worst on a grading path, because nothing logs and the student sees
a confident answer.

| # | Defect | Fix |
| --- | --- | --- |
| **1** | The vector/matrix shape pass never flattened, so it silently no-op'd on any sum of 3+ addends built by `parse_text` — `fromText("x+(1,2)+(3,4)")` and `fromAst` of the same tree disagreed, and grading's componentwise branch never engaged. Its unit-test helper pre-flattened, which is what hid it. | `flatten` once at the entry point; test helper no longer pre-flattens [`ops/vector_matrix.rs`] |
| **2** | The prototype-mirroring loop installed `Context.toJSON`, so `JSON.stringify` passed the **property key** as the expression: `{me}` emitted a `math-expression` envelope `Context.reviver` would revive the library context from, and `{"(": me}` *threw* out of a plain stringify. `toJSON` is not on `Object.prototype`, so the `in Context` guard missed it. | `NOT_EXPRESSION_FIRST` skip set [`math-expressions.ts`] |
| **3** | `0 · {"$":"None"}` → `0`, dropping DoenetML's "no value here" | `is_infinite_factor` poisons on `None` → `NaN`, per item 3b [`normalize/constructors.rs`] |
| **4** | `dopri` swallowed an exception thrown by the derivative (the Rust side correctly refuses to unwind under `panic="abort"`, but the wrapper never surfaced it) and returned the initial condition; a wrong-width derivative integrated silently; the solution leaked its wasm handle. | capture-and-rethrow, width check, `free`/`dispose`/`Symbol.dispose` [`math-expressions.ts`] |
| **5** | The addition pass reordered addends, and an empty container swallowed its scalar (`3·()` → `()`) | ordered slot list; empty containers absorb nothing [`ops/vector_matrix.rs`] |
| **6** | `fromAst({})` reported `unknown special None` — that `None` was the `Option` from the `$` lookup, while `{"$":"None"}` is a *legal* tree. The §2 message the DoenetML team lost a cycle to. | the two cases report separately [`expr/serde.rs`] |
| **7** | `max`/`min`/`median` returned **wrong numbers** under NaN: `partial_cmp(…).unwrap_or(Equal)` is not a total order, so `max(4,NaN,3,2,1)` → 3 while `min` of the same list → 4. Rust's sort may also *panic* on detecting it, which `panic="abort"` turns into a module crash. | NaN short-circuits to NaN (IEEE/mathjs); `sort_by(f64::total_cmp)` keeps the comparator total by construction [`special_functions/aggregate.rs`] |
| **8** | `0/∞` → `NaN`, should be `0` — the guard read the base and ignored the exponent, and `∞^(-1)` *is* `0`. Collateral from the §4 `0/0` fix itself. | exponent-aware; `NaN`/`None` still poison at any exponent [`normalize/constructors.rs`] |
| **9** | `nCr`/`nPr` on a large float: `n.re.round() as i64` saturated at `i64::MAX`, so `nCr(1e20,3)` came back ~1275× low. Pre-existing in `combinatorial`, newly reachable from `simplify` via `fold_approximately`. | the float path stays in f64; only `r` needs an integer type [`special_functions/misc.rs`] |
| **10** | Hang vectors from a few characters of student input: `nCr(10^500,500)` 4.0 s (running product is quadratic in the result size), `log₂(2^200000)` 2.2 s (one factor stripped per iteration). | `balanced_product` + result-size bound charged against `max_pow_bits`; `integer_log` binary-searches the exponent. **4055 ms → 21 ms** and **2190 ms → 0.5 ms**, same accepted inputs [`misc.rs`, `exp_log.rs`] |
| **11** | `fold_numeric_applications` was not canonical-out, contradicting its own doc — harmless via `simplify()` (which re-simplifies after), but the exported pass handed out `["+",55,3]` | re-canonicalize when anything folded [`normalize/fold_apply.rs`] |

### Verification (round 3)

- Rust: **616 passing, 0 failing** (was 602), clippy clean. New suite
  `tests/doenet_review_fixes.rs` (7) plus inline regressions in
  `ops/vector_matrix.rs` (3), `special_functions/aggregate.rs` (3),
  `normalize/fold_apply.rs` (1).
- js-compat differential against a rebuilt pre-fix tree: **zero newly failing,
  zero newly passing** across the 6230 pre-existing tests — these are edge cases
  (NaN, ∞, huge/float arguments) the legacy corpus does not exercise, so the
  fixes are behaviour-preserving on everything it *does*.
  `spec/quick_doenet_review_fixes.spec.ts` adds 17, all passing.
- **Rebuild note still applies:** regenerate the vendored wasm
  (`./build-wasm.sh` in js-compat) after any core change, or the JS suite tests
  the old engine.

---

## 12. DOENET_INTEGRATION §2–§5 (2026-08-04)

DoenetML's integration report filed four engine-level items after the permanent
switch to the Rust engine. All four are fixed. §1 of that report is not a fifth
item: its two WASM traps are §2 and §5 reached through an `assert_eq!` in
DoenetML's own core, which `panic = "abort"` reduces to a bare `unreachable`.

| § | Defect | Fix |
| --- | --- | --- |
| **2** | `evaluate_numbers` collected like terms (`x²+3x²` → `4x²`), so `simplify="numbers"` — specified as "fold numeric constants, leave the symbolic structure alone" — was indistinguishable from `simplify="full"`. A wrong answer, a lost public attribute, and the `simplify_math` trap, from one behaviour. | `add` runs with like-term collection off under `without_like_term_collection`. *Cancellation still collapses* (`3x−3x` → `0`); like **powers** in `mul` still combine, because the legacy oracle needs `i·i → −1`, which is that same merge [`normalize/constructors.rs`, `ops/numbers.rs`] |
| **3** | Inverse trig never folded: `asin(1)` stayed symbolic, so `simplifyOnCompare` could not grade it. Conspicuous next to the log and combinatoric identities, which do fold. | `inverse_trig_special_value` **inverts the forward table** rather than tabulating the values again — one table, so the directions cannot drift, and correctness reduces to the index range being the principal branch. Values are compared **in the `Exact` ring**, not as trees, so the fold recognizes a number rather than a spelling [`eval_exact/eval.rs`, `normalize/special_values.rs`] |
| **4** | `log_b(a)` was inert — it combined with nothing, and `log_b(a) − log(a)/log(b)` never reached zero. | change-of-base rewrite in `fold_special_values`, declining where the numeric pass would answer exactly (`log_2(8)` is `3`, not `log 8 / log 2`) [`normalize/special_values.rs`, `normalize/fold_apply.rs`] |
| **5** | `.tree` decimalized every terminating rational, so `3/6` was `0.5` and the structural criteria (`ReducedFraction`, `ExactValue`) could not see a fraction that was no longer there. This is **R13b**, deferred in §2 above pending a provenance decision — now taken. | a `Spelling` (`Fraction` \| `Decimal`) carried on `Number::Rat`/`BigNumber::Rat` [`num/number.rs`] |

### R13b — the provenance decision, and why it is not a serde patch

The issue's framing ("a ~5-line `number_to_js` change") does not work, for the
reason §2 measured: user-typed decimals parse to *exact rationals*, so `0.5` and
`1/2` are the same `Rat(1, 2)` and the distinction is gone before anything
reaches the boundary. The maintainer call was the **broad** rule:

> A non-integer exact rational spells as a fraction unless it descends from a
> decimal literal. `Decimal` is contagious through arithmetic, exactly as
> `Float` is, which makes `Fraction` the join identity and hence the right
> default for integers, floats, and every value the engine computes.

So `3/6` → `["/",1,2]`, `cos(pi/3)` → `["/",1,2]`, while `0.5` → `0.5`,
`0.1+0.2` → `0.3`, `19.9` → `19.9`, `1/2 + 0.25` → `0.75`.

The spelling is **not part of a number's identity**: `Number`/`BigNumber` have
hand-written `PartialEq`/`Hash` that ignore it, so `0.5 == 1/2` structurally and
canonical trees stay comparable and hashable. `tests/doenet_integration_fixes.rs
:: spelling_does_not_affect_equality` pins that, because getting it wrong would
silently split canonical trees in two.

Spelling originates in exactly two places — `from_decimal_str` (the literal) and
`round_to_decimals` (the operation whose purpose *is* the decimal spelling) —
and propagates through `binop`, `neg`, and `checked_pow_int`. Three passes work
in spelling-free `BigRational` and so restore one explicitly: `fold_apply`
(joined over the arguments, so `abs(-3.5)` is `3.5` and `mean(1,2,3,4)` is
`5/2`), and `ops::numbers::reduce_node` via `spelling_of`/`respell`.

**The subtle half.** `present`'s `split_number` moved *every* rational
coefficient under a fraction bar, so `0.5·x` presented as `Div(x, 2)` — and
re-canonicalizing that left two plain integers with the decimal origin destroyed.
That is why `0.5^2` came back as `1/4` several passes downstream of anything
that looked responsible. `split_number` now applies the spelling gate, which
also lets `present_exponent` stop being a special case: `x^(3/2)` and `x^1.5`
now differ because the *values* differ.

### Verification (round 4)

- Rust: **631 passing, 0 failing** (was 617), clippy **0 diagnostics**. New
  suite `tests/doenet_integration_fixes.rs` (14).
- js-compat differential against a rebuilt HEAD (`640d32c`): **zero newly
  failing, zero newly passing** across the 6247 pre-existing tests.
  `spec/quick_doenet_integration_fixes.spec.ts` adds 14, all passing.
- `tsc --noEmit`: 3356 errors, against 3357 at HEAD (pre-existing noise; no new
  ones).
- Four legacy-corpus expectations that pinned the *old* behaviour were updated
  rather than worked around, each with the reason in place:
  `serde.rs` (terminating rationals now split by spelling — the old test became
  two, one per spelling), `fold_apply.rs` (`median(1,2,3,4)` → `5/2`),
  `preserve_order.rs` (`2/4+x` → `["/",1,2]`, which now *matches* legacy), and
  `quick_doenet_compat_pr84.spec.ts` (`asin(1)` folds, so the exactness-gate
  example moved to `asin(2)`/`atan(1/3)`).
- **Rebuild note still applies:** regenerate the vendored wasm
  (`./build-wasm.sh` in js-compat) after any core change, or the JS suite tests
  the old engine.

### Not addressed, and why

- **§6** is DoenetML's own list; nothing requested.
- **§7** (`panic = "abort"` / wasm32 stack safety) is `STACK_SAFETY_PLAN` items
  21 and 23–26, unchanged by this round.
- **§9** (scientific-notation threshold) is an open question to the maintainer,
  not a defect — it needs an answer, not a patch.
- §2's operand *ordering* is untouched: DoenetML explicitly says the ordering
  axis is fine, and matching legacy's `default_order` exactly is a separate job.
  Several `slow_simplify.spec.ts` blocks still fail on ordering (and on
  pre-existing `{"$":"Inf"}` / negative-zero divergences) *after* their
  like-term content became correct.

### §3 — what the inverse fold does *not* do

It is a table lookup, not an identity engine. Three kinds of "not in the table",
only the first of which is closed:

1. **Same value, differently written** — closed. The first cut compared
   canonicalized *expressions*, which made the fold depend on whether the
   radical rules had happened to rationalize the argument first: `asin(√2/2)`
   folded, `asin(1/√2)` did not. Comparison now happens in `Exact`, whose normal
   form is zero exactly when the value is, so every spelling of a lattice value
   folds. Pinned by `inverse_trig_ignores_how_the_argument_is_written`.
2. **Off-lattice exact values** — `acos((1+√5)/4)` is `π/5` and does not fold.
   Inherent to the π/12 lattice, and symmetric with the forward direction, which
   does not fold `cos(π/5)` either. Extending means a larger table or a real
   algebraic-number inversion, not a tweak.
3. **Symbolic identities** — none implemented: parity (`asin(−x) → −asin x`,
   `acos(−x) → π − acos x`), complementary (`asin x + acos x → π/2`,
   `atan x + atan(1/x) → π/2`), and composition (`sin(asin x) → x`,
   `cos(asin x) → √(1−x²)`). Note the asymmetry — the *forward* fold does carry a
   parity + π-shift layer (`normalize_trig_arg`); the inverse direction has no
   counterpart.

`equals` decides all of (2) and (3) by numerical sampling, so **grading is not
affected** — it is `simplify` / `.tree` that leaves them alone. Parity is the
cheapest of the three to add if a display-level normal form is ever wanted.

*(2) and (3) were both asked for and are now closed — see §13.*

## 13. Closing §3's limits (2)/(3) (2026-08-04)

Everything in the list above except item 1 is now implemented. Three pieces,
each independently useful:

**A. A second lattice (π/10) in `eval_exact::eval`.** The pentagonal angles are
constructible, so half of them are in the surd ring: `cos 36° = (1+√5)/4`,
`sin 18° = (√5−1)/4` and their reflections. The other half are not —
`sin 36° = √(10−2√5)/4` needs a radical nested one level deeper than the
`π^i·e^j·√r` basis holds — and they decline rather than approximate.

The asymmetry propagates cleanly: `sin_at` tries twelfths then tenths, while
tangent stays on the twelfths alone, because a tangent needs the sine *and* the
cosine of the same angle and on this lattice exactly one of the two nests. The
inverse walk moved to units of π/60 (the first unit both lattices fit in) and
skips indices that are on neither, so it evaluates ~22 candidates where the
twelfths-only version evaluated 13 — not the 61 a naive π/60 sweep would.

**B. General inversion in the `Exact` ring** (`value.rs`). Inversion used to
handle a rational or a single surd term; anything with a two-term denominator
declined. That made `sec(π/12) = 4/(√6+√2)` unfoldable even though its value,
`√6−√2`, is in the ring — and correspondingly `asec(√6−√2)` did not invert.

Now: pick a prime `p` dividing some radicand, split `x = A + √p·B` over the
subfield generated by the remaining primes, and use
`1/x = (A − √p·B)/(A² − p·B²)`. The denominator involves strictly fewer primes,
so the recursion bottoms out at a rational. `A² − p·B² = 0` would put
`√p = A/B` in the smaller subfield, which it is not, so the denominator is
nonzero whenever `x` is. `inverse` now takes the shared op budget (term count
can double per prime), and radicands carrying π or e still decline — `1/(1+π)`
is not a polynomial in π, so it is genuinely outside this ring.

This is what makes the reciprocal branches work on *both* lattices:
`sec(π/5) → √5−1`, `csc(π/10) → 1+√5`, `acot(2+√3) → π/12`.

`eval_apply` also routes sec/csc/cot through `trig_exact` now, so
`is_zero("sec(π/5) − (√5−1)")` is certified rather than undecided; before, only
sin/cos/tan reached the tables from the `exact_eval` entry point.

**C. The identity layer** (`normalize/special_values.rs`). Four rules, all
unconditional — no domain hypothesis, so no assumptions machinery:

| rule | form | why it is sound |
| --- | --- | --- |
| parity | `asin(−u) → −asin u`, `acos(−u) → π − acos u` | asin/atan/acsc/acot are odd, acos/asec reflect through π/2 |
| composition | `f(f⁻¹(u)) → u` | each inverse *is* a right inverse of its branch |
| mixed composition | `cos(asin u) → √(1−u²)`, `sec(atan u) → √(1+u²)`, … | the branch ranges make the principal root the right one |
| complementary | `asin u + acos u → π/2`, `acsc u + asec u → π/2` | `acos = π/2 − asin` by definition of the branch |

All 36 outer/inner pairs go through one path: `g⁻¹(u)` is turned into the pair
`(sin θ, cos θ)` for `θ = g⁻¹(u)`, and the outer function is read off that pair.
The direct pairs are not special-cased — `sec(asec u)` is `1/cos(acos(1/u))` is
`1/(1/u)` is `u`, and the smart constructors finish the job.

Three things deliberately left out, each for a stated reason:

- **`asin(sin x) → x`** and its family. True only on the principal branch
  (`asin(sin 3) = π − 3`), so it needs a range assumption on `x`, not a rewrite.
- **`atan u + acot u → π/2`.** This library defines `acot z` as `atan(1/z)`
  (`special_functions::trig_inverse::ACOT`), which makes the sum `π/2` for
  positive `u` and `−π/2` for negative `u`. Worth knowing: `equals` answers
  `true` for the symbolic `atan(x)+acot(x) == pi/2` while answering `false` at
  `x = −1` — a sampling gap. Folding would have written that gap into the
  simplified tree, where it is far harder to undo. Pinned as a negative test.
- **Parity through a sum.** `acos(−x−1)` is left alone. The trigger is
  `strip_negation`, which is stricter than the forward direction's
  `neg_leading`: its result is guaranteed not to be negated in turn, so a
  rewrite keyed on it cannot ping-pong inside the fixpoint. Negating `−x−1`
  just parks a `−1` in front of the sum, which would.

### Verification (round 5)

- Rust: **616 passing, 0 failing**, clippy **0 diagnostics**. 10 new tests —
  9 in `tests/special_values.rs` (the pass's own suite), 1 in
  `tests/exact_is_zero.rs` for the general reciprocal.
- js-compat differential against the same rebuilt pre-§2 baseline: **zero newly
  failing, zero newly passing** across the 6247 pre-existing tests.
  `spec/quick_trig_identities.spec.ts` adds 11, all passing.
- `tsc --noEmit`: no errors in the new spec.
- One existing expectation moved: `inverse_trig_declines_everything_off_the_lattice`
  listed `asin(-3)` among the "stays an `Apply`" cases. Parity now pulls the
  sign out, so it asserts the exact shape `−asin(3)` instead — the point of the
  test (no angle is invented off the lattice) is unchanged.

---

## 14. Three printer / display-rounding open items (2026-08-05)

DoenetML filed three more against the printers and display-rounding (items 8,
9, 10 in the thread). Items 8 and 10 are fully resolved; item 9 is split — its
unambiguous half landed, the rest waits on the corpus DoenetML offered.

| # | Ask | Landed |
| --- | --- | --- |
| **8** | Display rounding turned an exact rational into a decimal even when rounding changed nothing (`round_numbers_to_precision(5/2, 3)` was `2.5`, legacy kept `\frac{5}{2}`) | ✅ `round_to_decimals` now returns the value **unchanged** — keeping its spelling — when the rounded value equals the input; it still decimalizes when rounding genuinely changes the value (`1/3 → 0.333`). [`num/number.rs`] |
| **10a** | A negative leading coefficient printed `a + (-3) b` instead of `a - 3 b`; a negative fraction `(-2)/3` instead of `-2/3` | ✅ a display-only `normalize_display_negative_fractions` pass (port of the legacy one) pulls the minus out of a `/` numerator, `split_sign` now splits a `Mul` with a negative leading factor, and `render_mul` renders a leading negative inline (no parens) at `NEG` precedence. [`print/mod.rs`, `print/text.rs`, `print/latex.rs`] |
| **10b** | The integral head lost its `∫` glyph and wrapped the integrand in parens (`int_a^b(f(x) dx)`) | ✅ `int → ∫` under unicode, and an integral-head case in `render_apply` renders `∫_a^b <integrand>` with no parentheses. [`print/mod.rs`, both printers] |
| **9 (i)** | `i^2` stayed `["^","i",2]` instead of folding to `-1` | ✅ `i^n → {1, i, −1, −i}` (`n mod 4`) in the unconditionally-sound `fold_special_values` pass. `equals` already certified `i²=−1` numerically, so this is the display/`simplify` half only. [`normalize/special_values/`] |
| **9 (roots)** | `cbrt(x^3)`, `nthroot(x^3,3)`, `sqrt(16x²y⁴)` keep their radicals; `sqrt(-4)` stayed symbolic | ✅ **resolved** after the maintainer settled the convention (see `ROOT_SIMPLIFICATION_SPEC.md`): a *number* under a root folds, preferring the real root else the principal complex root; a *variable* radicand never folds. The concrete gap was even roots of negative numbers — `sqrt(-4) → 2i`, `sqrt(-2) → i·sqrt(2)` — now folded (q = 2 is always exact). Odd roots already preferred the real value (`cbrt(-8) → -2`) and variable radicands already stayed put. Higher even roots (`(-16)^(1/4)`) need the surd-lattice form and stay symbolic for now. [`normalize/simplify.rs`] |

### Why 10a is a display pass, not a tree change, and the round-trip cost

`a - 3 b` re-parses to `Neg(Mul(3,b))`, an **equal but distinct** tree from the
`Mul(-3,b)` that produced it — which is exactly why the old `split_sign`
refused to touch a `Mul` ("would not round-trip", its comment). Two facts make
the change safe anyway: the parsers **never emit** a `Mul` with a negative
leading factor (they use `Neg` and bare negative numbers — verified across all
four parser tree-fixtures, zero hits), so `tests/roundtrip.rs`'s corpus never
exercised the form; and the whole transformation runs at the printer entry,
never on a stored tree. The absorbed style difference DoenetML already accepted
(`(x²)/2 → x²/2`, no numerator parens) means a few fraction cases still diverge
from JS on paren *placement* only (`-2 x/3` vs JS `-(2 x)/3`) — re-blessed in
`ast-output-known-divergences.json`, with the sign now correctly out front.

### Item 8 supersedes §12's "round_to_decimals imposes the decimal spelling"

§12 established `round_to_decimals` as one of the two origins of a `Decimal`
spelling. Item 8 refines that: it imposes the decimal spelling **only when it
rounds**. A no-op round (`5/2` to 3 s.f., `3/6` to 3 d.p.) now preserves the
existing spelling, so a fraction a student sees stays a fraction. The
`doenet_integration_fixes::rounding_produces_decimals` expectation and its JS
mirror were updated to the refined rule (the `1/3 → 0.33` half is unchanged).

### Verification (round 6)

- Rust: **green, 0 failing** (658 tests), clippy **0 diagnostics**. New
  `tests/doenet_open_items.rs` (12, incl. the item-9 numeric roots added after
  the maintainer settled the convention). `output_established` re-blessed: the
  negative-coefficient/fraction cases now **match JS** (stale divergences
  removed); the remaining fraction cases changed to the sign-out-front,
  no-numerator-paren form.
- js-compat differential (vendored wasm rebuilt both sides): **+40 passing,
  zero regressions** — verified at the file *and* the individual-assertion
  level (`4902 → 4942` passing, 24+ tests fixed, 0 passed→fail). The gains land
  exactly in the printer specs: `quick_ast-to-latex` +15, `quick_ast-to-text`
  +9, `quick_latex-to-ast-to-latex` +5, `quick_text-to-ast-to-text` +5.
  `spec/quick_doenet_printer_and_rounding.spec.ts` adds 6, all passing;
  `quick_doenet_integration_fixes.spec.ts`'s rounding case updated to the item-8
  rule.
- **Rebuild note still applies:** regenerate the vendored wasm (`./build-wasm.sh`
  in js-compat) after any core change, or the JS suite tests the old engine.

### For DoenetML — the item 8 subtlety worth knowing

`me.fromAst(["/",5,2])` is a **`Div` of two integers**, and display rounding
maps over each integer separately, so it was never the case that reached the
bug — it stayed `5/2` all along. The bug shows on a **bare rational** (`5/2`
after `.simplify()`, or any computed value like `cos(pi/3) → 1/2`), which is
what the display path actually rounds. Both now stay fractions.
