# JS Test-Coverage Audit — the "nothing lost" ledger

One row per `spec/*.spec.js` file from the (now relocated) JavaScript suite
`tmp/js-legacy/spec/`, mapping each to its status in the Rust port. Companion to
[JS_RUST_DIFF.md](JS_RUST_DIFF.md) (API diff) and
[JS_RUST_TEST_DIVERGENCES.md](JS_RUST_TEST_DIVERGENCES.md) (case-by-case output
diff); this file is the single checklist proving every JS spec is accounted for
before the JS code is gone.

Rust tests live in `packages/math-expressions-rs/tests/` (+ inline `src/`).

## Legend

- ✅ **covered** — cases are asserted in the Rust suite (directly or via an
  extracted fixture / corpus).
- ⚠️ **portable gap** — the feature exists in Rust but the JS cases were only
  covered *indirectly*; closed by a new Rust test (§2b of the refactor plan).
- ⛔ **intentional non-port** — the feature is deliberately absent from Rust
  (see [WHATS_LEFT.md](WHATS_LEFT.md)); cannot have a Rust test without building
  the feature. Listed with a reason in the [Non-ports](#intentional-non-ports)
  section.
- 🔜 **future drop-in** — real coverage comes from the forthcoming
  `packages/math-expressions-js-compat` package (a TS drop-in over the Rust core) once the `spec/`
  suite is converted to TypeScript and run against it.

## Ledger

| JS spec | ~cases | Rust coverage | Status |
|---|---|---|---|
| `quick_text-to-ast` | ~550 + 9 err + 8 opt | `fixtures/text-to-ast{,-edge,-errors}.json` → `text_parse.rs`; option blocks → `parser_options.rs` | ✅ |
| `quick_latex-to-ast` | ~620 + 27 err + 8 opt | `fixtures/latex-to-ast{,-edge,-errors,-edge-errors}.json` → `latex_parse.rs`; options → `parser_options.rs` | ✅ |
| `quick_text-to-ast-to-text` | ~357 (×2 modes) | `roundtrip.rs` (every parser-fixture input round-trips) | ✅ / ⚠️ (`showBlanks`-off mode ⛔) |
| `quick_latex-to-ast-to-latex` | ~370 (×2 modes) | `roundtrip.rs` | ✅ / ⚠️ (`showBlanks`-off mode ⛔) |
| `quick_ast-to-latex` | ~258 + 8 opt | `fixtures/ast-to-latex.json` (265) — was probe-only | ⚠️ → **`output_golden.rs`**; emitter options ⛔ |
| `quick_ast-to-text` | ~244 | `fixtures/ast-to-text.json` (247) — was probe-only | ⚠️ → **`output_golden.rs`** |
| `quick_arithmetic` | 6 | `ops_corpus.rs`, `norm.rs`, `number_ops.rs` | ✅ |
| `quick_normalization` | 62 | `norm.rs` (holistic `canonicalize`), `special_values.rs`, `display.rs`, `functions_registry.rs` | ✅ (per-pass asserts folded into `canonicalize` — intentional) |
| `quick_pm` | 58 | `pm.rs` (15) + inline `src/pm.rs` (3) | ✅ |
| `quick_rounding` | 13 | `number_ops.rs` (single-number round; §4 of divergences = 0 diff) | ⚠️ (don't-round edge guards) → **`number_ops.rs` additions** |
| `quick_sets` | 7 | `sets.rs` (8) | ✅ |
| `quick_solve` | 3 | `grade.rs` (`solve_linear`) | ✅ |
| `quick_trees` | 39 | utils in `src/js_match.rs`; no dedicated test file | ⚠️ → **`tree_utils.rs`** |
| `quick_transformation` | 8 | `expand` → `expand.rs` / `expand_corpus.rs`; `expand_relations` op ⛔ | ✅ (expand) / ⛔ (`expand_relations`) |
| `quick_ast-to-mathjs` | 139 | `astToMathjs` shim in `packages/math-expressions-rs-wasm/src/tree-to-mathjs.ts` (TS, not the Rust crate) | ⛔ Rust / 🔜 drop-in |
| `quick_mathjs-to-ast` | ~28 | none | ⛔ |
| `quick_ast-to-guppy` | 4 | none | ⛔ |
| `quick_mml-to-latex` | 1 | none | ⛔ |
| `slow_math-expressions` | ~900 pairs + 14 | `equality_corpus.rs` (824-pair `equality-corpus.json`) + `equality.rs` (22 hand) | ✅ |
| `slow_simplify` | 74 (474 expects) | `simplify_corpus.rs` (342) + `norm.rs` / `display.rs` / `expand.rs` / `matrix.rs` | ✅ |
| `slow_assumptions` | 44 (420 expects) | `assumptions_corpus.rs` (546) + `assumptions.rs` + `doenet_utils.rs` | ✅ |
| `slow_matrix` | 12 (~30) | `matrix.rs` (31) | ✅ |
| `slow_polynomial` | 23 | no public Rust polynomial/Groebner API (`src/poly` internal only) | ⛔ |
| `slow_rational` | 2 | `reduce_rational.rs` (5) | ✅ |
| `slow_check-equality-numerical-errors` | 26 objs | `equality.rs` + **`tolerance.rs`** (fixture-driven, `equals`) | ✅ (16 sampling divergences snapshotted) |
| `slow_check-symbolic-equality-numerical-errors` | 26 objs | Rust `equals_syntactic` is exact — ignores `allowed_error_in_numbers` | ⛔ behavioral divergence / 🔜 drop-in |
| `build_esm` | 7 | tests the built JS bundle | 🔜 drop-in |
| `build_umd` | 7 | tests the built JS bundle | 🔜 drop-in |

## Portable gaps closed in this refactor (§2b)

- `tests/output_golden.rs` — asserts `ast-to-latex.json` (265) / `ast-to-text.json`
  (247) against the JS golden output, with the intentional divergences from
  [JS_RUST_TEST_DIVERGENCES.md](JS_RUST_TEST_DIVERGENCES.md) §2–§3 snapshotted in
  `fixtures/ast-output-known-divergences.json` (**114** entries: 36 latex + 78
  text). Converts the informational `zzz_divergence_probe` into a regression
  guard: any *new* or *changed* divergence, and any *stale* entry, fails the
  build. Re-bless with `BLESS=1 cargo test --test output_golden`.
- `tests/tree_utils.rs` — the `quick_trees` tree-utility surface via
  `src/js_match.rs` + `js_tree::to_js` + crate `substitute` (equal / flatten /
  unflatten / substitute / default-mode template match). Opt-in match modes
  (`variables`, regex/function conditions, permutations) are intentionally
  unported and out of scope (see `src/js_match.rs` docs).
- `tests/number_ops.rs` additions — the "don't round fractions / π / e" guards
  from `quick_rounding.spec.js`.
- `tests/tolerance.rs` — the **numeric** `allow_error_in_numbers` matrix (26
  objects) from `slow_check-equality-numerical-errors`, fixture-driven
  (`scripts/generate-tolerance-corpus.mjs` → `fixtures/tolerance-corpus.json`)
  against `equals`. `equals` grades by random sampling, so 16 of the hardest
  cases (exponent-tolerance on fractional powers, deeply nested exp) diverge from
  JS and are snapshotted in `fixtures/tolerance-known-failures.json`
  (no-regressions contract). The **symbolic** companion is a behavioral
  divergence, not a portable gap — see below.

## Intentional non-ports

Each is a JS spec case with **no Rust feature to test** — deliberately dropped,
not forgotten. Cross-referenced to [WHATS_LEFT.md](WHATS_LEFT.md) §A.

| Item | JS spec(s) | Reason |
|---|---|---|
| ast→mathjs | `quick_ast-to-mathjs` (139) | Done as a TS shim (`tree-to-mathjs.ts`) powering `evaluate`/`equals`, not in the Rust crate (WHATS_LEFT A.1 #6). Coverage belongs to that TS layer / drop-in. |
| mathjs→ast | `quick_mathjs-to-ast` (28) | Not needed for Doenet — unused internally (WHATS_LEFT A.1 #5). |
| ast→guppy | `quick_ast-to-guppy` (4) | Not needed for Doenet — legacy Guppy-editor XML (WHATS_LEFT A.1 #4). |
| MathML (mml→latex) | `quick_mml-to-latex` (1) | No MathML parser/emitter in Rust (WHATS_LEFT A.1 #1–2). |
| polynomial / Groebner | `slow_polynomial` (23) | No public Rust polynomial API; `src/poly` is internal (JS_RUST_DIFF §4.2). |
| `expand_relations` | `quick_transformation` (few) | Op absent in Rust (JS_RUST_DIFF §3.1). |
| emitter options | `quick_ast-to-latex` standalone (8) | `LatexOpts`/`TextOpts` fixed-behavior: matrix env, pad-to-digits/decimals, avoid-scientific-notation, `showBlanks` (JS_RUST_DIFF §2.2). |
| syntactic tolerance | `slow_check-symbolic-equality-numerical-errors` (26) | Rust `equals_syntactic` does exact structural comparison (`na == nb`) and does **not** apply `allowed_error_in_numbers`; JS `equalsViaSyntax` does. Number-tolerance lives only on the numeric `equals` path in Rust. |

If any ⛔ item is later required, it becomes a `drop-in` feature (with its
converted TS spec) or a new Rust capability — this table is where to look first.
