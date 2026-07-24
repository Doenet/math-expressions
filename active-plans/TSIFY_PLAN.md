# tsify Type-Interface Plan

Status: **DRAFT — awaiting decisions (see §1). Nothing implemented.**
Created: 2026-07-20.

Goal: replace the hand-serialized **JSON-string** values that cross the
wasm boundary in [`src/wasm.rs`](../math-expressions-rs/src/wasm.rs) with
strongly-typed structs, so the generated `pkg/math_expressions.d.ts` describes
the real shapes instead of `string`. Backed by [`tsify`](https://github.com/madonoharu/tsify)
(or the maintained fork `tsify-next`), which derives a TypeScript type from a
serde type and wires the wasm ABI.

---

## 0. Why (current state)

`wasm-bindgen` already emits a typed `.d.ts`. The gap is **stringly-typed
structure**: everything richer than a primitive crosses as a JSON string built
by `serde_json::json!(...).to_string()` on the Rust side and re-parsed as
`JSON.parse(...) as X` on the JS side (e.g.
[`engines.ts:136`](../playground/src/engines.ts#L136)). Those `string`s are
where the type interface is dishonest.

**Stringly-typed boundary surface (the whole scope of this plan):**

Returns `string` that is really structured:
`tree_json`, `to_serialized`, `nullspace`, `eigenvalues`, `eigenvectors`,
`integrate_analyzed`, `eigs`, `match_template`, `flatten_ast`,
`unflatten_left`, `unflatten_right`.

Takes `string` that is really structured:
`equals_with_options`, `parse_text_with_options`, `parse_latex_with_options`,
`from_ast`, `from_serialized`, `match_template`.

≈ 11 return sites + 6 input sites. All are leaf boundary shapes — none touch
the core `Expr` enum (see the Level-2 decision in §1).

**Constraints that shape the plan:**
- `wasm-bindgen` is pinned exactly: `wasm-bindgen = "=0.2.126"`
  ([`Cargo.toml`](../math-expressions-rs/Cargo.toml)). tsify tracks
  wasm-bindgen closely; the pin is the single largest compatibility risk.
- The crate is **size-tuned** (`-Oz`, size release profile,
  [`scripts/build-wasm.sh`](../math-expressions-rs/scripts/build-wasm.sh)).
  IMPROVEMENT_PLAN item 34 explicitly wants to *drop* `serde_json` at the
  boundary to save ~100–200 KB — so the serialization-backend choice (§1.C)
  interacts with a separate, already-planned goal.
- `--target nodejs`.
- The core `Expr` enum ([`src/expr.rs`](../math-expressions-rs/src/expr.rs)) has
  **no serde derives**, and the JS `Tree` shape (`number | string | boolean |
  [string, ...Tree[]]` with `{"$":"Inf"}` specials) is a custom encoding in
  [`src/js_tree.rs`](../math-expressions-rs/src/js_tree.rs) that does **not**
  match a serde-default derive of `Expr`.

---

## 1. Decisions for you

These change what gets built. My recommendation is first in each list.

**A. Scope — how far into the boundary do we go?**
- **A1 (recommended): Level 1 only.** Type the option and result *leaf*
  structs. Leave `Expression` opaque and leave the `Tree` encoding in
  `js_tree.rs` untouched. Removes every dishonest `string` except the raw
  `Tree` payloads, which stay `string` or get a hand-written `Tree` TS type
  (see D). Low risk, local to `wasm.rs`.
- **A2: Level 1 + typed `Tree`.** Additionally give the tree-carrying sites
  (`tree_json`, `from_ast`, `to_serialized`, `match_template`, `flatten_ast`,
  …) a real `Tree` TS type via a `#[tsify(type = "Tree")]` newtype over
  `serde_json::Value`. More sites, still keeps `js_tree.rs`.
- **A3: rejected here — type `Expr` itself.** Deriving `Tsify` on `Expr` would
  emit a *tagged discriminated union* that is **incompatible** with the JS
  array-`Tree` the rest of the ecosystem uses, so it would fork the tree
  format. Not recommended; documented so it is a decision, not an oversight.

**B. Which crate?**
- **B1 (recommended): `tsify-next`** — maintained fork, current wasm-bindgen
  support.
- **B2: `tsify`** — original; less active, higher chance of friction against
  the `=0.2.126` pin.

**C. Serialization backend / bundle size.**
- **C1 (recommended): `serde-wasm-bindgen`** (tsify default) — structured
  values, no JSON round-trip. But it is *additional* code alongside the
  `serde_json` still used elsewhere, so peak bundle may not shrink until a
  later pass removes `serde_json`. Coordinate with IMPROVEMENT_PLAN item 34.
- **C2: JSON backend** (`#[tsify(into_wasm_abi)]` with the json feature) —
  reuses `serde_json`, smaller marginal code, but keeps a JSON round-trip at
  the boundary (types honest, perf unchanged). Lowest-risk on size.

**D. `Tree`-carrying sites (only if A2, or as the A1 fallback).**
- **D1 (recommended if A1): keep them `string`.** The `Tree` values stay JSON
  strings exactly as today; only the *non-tree* structured sites get typed.
  Smallest diff.
- **D2: hand-typed `Tree`.** Introduce a `JsTree(serde_json::Value)` newtype
  with `#[tsify(type = "Tree")]` and a passthrough `Serialize`/`Deserialize`,
  reusing the existing `Tree` TS definition from
  [`index.d.ts`](../index.d.ts). Honest `Tree` types, `js_tree.rs` unchanged.

**E. Hand-maintained `index.d.ts` coordination.** The repo ships a curated
[`index.d.ts`](../index.d.ts) separate from the generated `pkg/*.d.ts`. Decide
whether this plan also updates `index.d.ts`/`REMAINING_TYPING_ISSUES.md`, or
only the generated types. (Recommended: update both in the final phase so the
public surface and the generated surface agree.)

---

## 2. Gate 0 — de-risking spike (do this before committing to phases)

One throwaway commit, ~half a day, answers the two unknowns (pin
compatibility, size delta) before any real work:

1. Add `tsify-next` + `serde-wasm-bindgen` to
   `[target.'cfg(target_arch = "wasm32")'.dependencies]`.
2. Convert exactly **one output** (`eigenvalues` → `Vec<EigenValue>`) and
   **one input** (`equals_with_options` → `EqOptionsJs`).
3. `cargo build --target wasm32-unknown-unknown --release` must succeed
   against `wasm-bindgen =0.2.126`.
4. Run `scripts/build-wasm.sh`; confirm the smoke test passes and record
   `pkg/math_expressions_bg.wasm` size before/after.

**Exit criteria:** builds clean on the pin, `.d.ts` shows the new types, size
delta acceptable. If the pin fights tsify-next, fall back to decision C2 (JSON
backend) or park the plan. **Do not proceed to §3 until Gate 0 is green.**

---

## 3. Implementation phases (assuming A1 + B1 + C1 + D1)

Each phase is independently shippable and keeps the smoke test green.

### Phase 1 — scaffolding
- [ ] New module `src/wasm_types.rs` (wasm32-only) holding the boundary
      structs, each `#[derive(Serialize, Deserialize, Tsify)]` with
      `#[tsify(into_wasm_abi)]` / `#[tsify(from_wasm_abi)]` as appropriate and
      `#[serde(rename_all = "camelCase")]` to match the JS spellings.
- [ ] Confirm `wasm-smoke.cjs` still passes.

### Phase 2 — input options (retire the hand-rolled JSON option parsers)
Replaces `read_opt_bool` / `read_opt_f64` / `read_opt_strings` plumbing.
- [ ] `EqOptionsJs` → `equals_with_options(other, opts: EqOptionsJs)`.
- [ ] `TextParseOptionsJs` → `parse_text_with_options(s, opts)`.
- [ ] `LatexParseOptionsJs` → `parse_latex_with_options(s, opts)`.
- [ ] Delete the now-unused `read_opt_*` helpers.

### Phase 3 — structured results (non-tree)
- [ ] `EigenValue { value: String, multiplicity: u32 }` →
      `eigenvalues() -> Option<Vec<EigenValue>>`.
- [ ] `EigenVector { value, multiplicity, basis: Vec<Vec<String>> }` →
      `eigenvectors() -> Option<Vec<EigenVector>>`.
- [ ] `nullspace() -> Option<Vec<Vec<String>>>` (drop the JSON string).
- [ ] `EigsResult` + tagged `ComplexNum` → `eigs(...) -> Option<EigsResult>`.
- [ ] Tagged `IntegralAnalysis` (`Value` / `Divergent` / `Unknown`,
      `#[serde(tag = "status", rename_all = "lowercase")]`) →
      `integrate_analyzed(...) -> IntegralAnalysis`.

### Phase 4 — tree-carrying sites
- **If D1:** leave `tree_json`, `to_serialized`, `from_ast`, `from_serialized`,
      `match_template`, `flatten_ast`, `unflatten_left/right` as `string`.
      Document them as `Tree`-JSON in doc comments. *(No code change.)*
- **If D2/A2:** introduce `JsTree`, retype those sites, keep `js_tree.rs` as
      the conversion core.

### Phase 5 — JS-side + public types
- [ ] Update [`playground/src/engines.ts`](../playground/src/engines.ts) and
      `playground/src/types.ts` to consume the typed returns (drop
      `JSON.parse(...) as X` where the value is now structured).
- [ ] Per decision E: reconcile [`index.d.ts`](../index.d.ts) and note the
      resolution in [`REMAINING_TYPING_ISSUES.md`](../REMAINING_TYPING_ISSUES.md).
- [ ] Final `scripts/build-wasm.sh`; record final size; update this plan and
      WHATS_LEFT.

---

## 4. Out of scope
- Deriving `Tsify`/serde on `Expr`, `Number`, `Sym`, `SeqKind`, `MathConst`,
  `RelOp` (decision A3).
- Changing the `Tree` wire format.
- Making `Expression` a by-value serializable type (it stays an opaque handle).

## 5. Risks
- **Pin compatibility** (`=0.2.126`) — retired by Gate 0.
- **Bundle size** vs IMPROVEMENT item 34 — measured in Gate 0; decision C is
  the lever.
- **`Number` fidelity** — any serde form must keep the deliberate
  `Rat`/`Big` → f64 projection that `js_tree.rs` already does
  ([`number_to_js`](../math-expressions-rs/src/js_tree.rs)); do not silently
  change numeric precision at the boundary.
