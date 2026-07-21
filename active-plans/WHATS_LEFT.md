# What's Left

Tracking doc for remaining work on the Rust port (`math-expressions-rs`). Two
parts: (A) the JS→Rust feature gap, (B) unfinished items in the `tmp/*_PLAN.md`
documents. Check items off as they land.

Last audited: 2026-07-21.

---

## A. JS features the Rust port lacks

### A.1 Converters / formatters (largest real gap)

JS (`lib/converters/`) has a full matrix between AST ↔ {latex, text, guppy,
mathjs, mml, glsl}. Rust does only **latex ↔ ast** and **text ↔ ast**.

- [ ] 1. MathML parsing (`mmlToAst`) — no Rust equivalent
- [ ] 2. MathML output (AST → MathML) — **not needed for Doenet** (no AST→MathML consumer)
- [ ] 3. GLSL output (`toGLSL`) — **not needed for Doenet** (shader grapher; Doenet uses jsxgraph, which evaluates numerically via mathjs)
- [ ] 4. Guppy output (`toGuppy`) — **not needed for Doenet** (legacy Guppy-editor XML; unused internally)
- [ ] 5. MathJS input converter (`mathjsToAst`) — **not needed for Doenet** (unused internally; only commented-out in `meTest.js`)
- [x] 6. MathJS output converter (`astToMathjs`) — powers `f()`/`evaluate()`/`equals()` numeric eval; jsxgraph plots via this path. Done as **option 1** (JS-level shim over the Rust AST) in its own npm-workspace package `packages/math-expressions-rs-wasm/` (`src/tree-to-mathjs.ts`) — typed `Tree`→math.js node + factorial→gamma + `compileTree`/`compileRustExpr` bridge (normalization done Rust-side via `normalize_function_names`)

### A.2 Genuinely missing capabilities

- [x] 7. `isAnalytic()` — `ops::is_analytic` + `AnalyticOpts`; wasm `is_analytic(allow_abs, allow_arg, allow_relation)`
- [x] 8. `factor()` — real univariate factoring over ℚ (`src/factor.rs`): content + Yun squarefree + rational-root deflation, `equals`-gated. Multivariate/non-poly returned unchanged
- [x] 9. `simplify_logical()` — `norm::simplify_logical`: numeric fold + De Morgan / not-pushdown / relation negation
- [x] 10. `equals_via_real()` — `real_only` sampling flag on `EqOptions`, isAnalytic-gated
- [x] 11. Fuzzy number matching — already in `EqOptions`; now exposed via wasm `equals_with_options(other, json)`

### A.3 Implemented in Rust but not exposed at the wasm boundary (wiring, not algorithms)

- [x] 12. Matrix/vector ops wired to wasm: `transpose`, `trace`, `matmul`, `matrix_inverse`, `rref`, `rank`, `nullspace` + newly implemented `dot_prod`, `cross_prod`, `vector_add`, `vector_sub` (in `matrix.rs`)
- [x] 13. Granular normalization passes implemented + wasm-exposed: `normalize_function_names`, `tuples_to_vectors`, `altvectors_to_vectors`, `subscripts_to_strings`, `strings_to_subscripts`, `to_intervals`. (The rest — `normalize_negative_numbers`, `default_order`, `normalize_angle_linesegment_arg_order`, `substitute_abs` — remain subsumed by `canonicalize()`; expose individually only if a caller needs them)
- [x] 14. Units: `remove_units`, `add_unit`, `remove_scaling_units` (`ops.rs` + wasm)
- [x] 15. Assumptions management surface: wasm `Assumptions` handle (`add`/`remove`/`clear`/`is_empty`/`simplify` + the 8 predicates)
- [x] 16. `finite_field_evaluate()` — `eq::finite_field_evaluate` + wasm free function
- [x] 17. `set_small_zero()` — `ops::set_small_zero` + wasm
- [x] 18. `mod()` and `copy()` expression methods (wasm)
- [x] 19. `collect_like_terms_factors()` → `simplify`, `simplify_ratios()` → `reduce_rational` (wasm aliases, per scoping decision)
- [ ] 20. Derivative step recording (`derivative(var, story)`) — **deferred** (largest single piece; JS `derivative_with_story` emits LaTeX narration)

> Rust is *ahead* of JS on: symbolic `integrate()`, arbitrary-precision eval &
> quadrature, `rootof`, symbolic eigenvalues/eigenvectors, ODE solver.
>
> NOTE: calculus limits (`lim_{x→a}`) are **not** implemented — only designed
> (tmp/LIMITS_PLAN.md, draft). The `src/resource_limits.rs` module (formerly
> `limits.rs`, added by commit "Implement limit") is the operation-count
> resource governor, unrelated to calculus limits. Do not list `limits` as a
> shipped feature.

---

## B. Unfinished plan items

Complete plans (no action): ARBITRARY_PERCISION, DIVERGENCE,
NORMALIZATION_REDESIGN, MATRIX, ODE.

Partially done (§B.1–B.3): STACK_SAFETY, INTEGRATION, IMPROVEMENT.

Draft — designed but **not started** (§B.4–B.7): LIMITS, FULL_SIMPLIFY,
SINGULARITY_TRANSFORM, FORM_GRADING. These are greenfield design docs with
zero implementation; none is a partial port.

### B.1 STACK_SAFETY_PLAN — not started (highest risk)

Recursive traversals can overflow the ~1 MB wasm32 shadow stack on deep
expressions (including on `Drop` — freeing a deep tree crashes). Sequenced:

- [ ] 21. Iterative `Drop` for `Expr` (kills the "freeing the tree crashes" class)
- [ ] 22. Parser depth cap at the ~4 self-nesting entry points + `from_js` depth check (default 256)
- [ ] 23. `children(&Expr)` helper + iterative post-order `fold` driver in `expr.rs`
- [ ] 24. Port the ~8 passes to the driver, in dependency order: `flatten` → `canonicalize` → `cmp` → `eval_complex`/`free_symbols`/`contains_blank`/`coerce_seqs` → `to_js`/`from_js` → formatters → `convert_units_in_term`
- [ ] 25. Replace `opaque_key`; decide whether to replace derived `PartialEq` with an iterative version (per frame-size measurement)
- [ ] 26. Verification: small-stack CI test (128 KiB threads), 10⁵-deep-paren inputs, document `-zstack-size`

> Note: shares a `children()`/`for_each_child` primitive with IMPROVEMENT
> Phase 3/4 (items 30, 32) — build it once.

### B.2 INTEGRATION_PLAN — I1–I2 done, I3–I5 left

- [ ] 27. I3: integration-by-parts, trig clusters (sinᵐ·cosⁿ, tan/sec, product-to-sum), algebraic √-substitutions; fuel adversarial tests
- [ ] 28. I4: hypergeometric terminal node — terminal rules, contraction simplify cluster, precision-plan pFq kernel row, numeric cross-check
- [ ] 29. I5: `integrate_with_story`, presentation polish (RootOf / pFq display), wasm binding, PORTING_PLAN §17 update

### B.3 IMPROVEMENT_PLAN — Phases 0–1 done, 2–5 left

- [ ] 30. Phase 2: consolidate non-function notation tables — Greek letters (lexer + output), RelOp/SeqKind render forms, OtherOp escape-hatch metadata
- [ ] 31. Phase 3: memory quick-wins — `opaque_key`→hash, `Sym::name()`→`&'static str`, `Expr::for_each_child` primitive, static parser tables with `OnceLock`
- [ ] 32. Phase 4: by-value normalization passes + unchanged-propagation (the real peak-memory fix)
- [ ] 33. Phase 5: bundle cleanups — Cargo features for heavy subsystems (eigen, integrate, precise, numeric-compat), dedup precise/interpreter, matrix clone reduction, series-loop mutation, polynomial docs
- [ ] 34. Deferred (revisit if bundle size becomes pressing): drop `serde_json` at the JS boundary (~100–200 KB); hash-consing/arena `Expr`

### B.4 LIMITS_PLAN — draft, not started (greenfield, no JS reference)

Calculus limits `lim_{x→a} f(x)`. No upstream JS equivalent, so no differential
corpus. New `Expr::Limit` binder variant + `src/limit/` engine. Phased P0–P5:

- [ ] 35. P0: `Expr::Limit`/`LimitDir` variant, ~10 match arms, text + LaTeX parse/print, `js_tree` round-trip (no evaluation yet)
- [ ] 36. P1: binder audit (variables/substitute/diff/eval-opaque) + Stage 0–1 (preprocess + direct-substitution continuity) + numeric verification-gate skeleton
- [ ] 37. P2: algebraic (factor/cancel/rationalize) + one-sided reconciliation + DNE + rational end-behavior at ±∞
- [ ] 38. P3: L'Hôpital (indeterminate-form detection, recursion cap) + known-limits table (`FnDef::asymptotics` facet)
- [ ] 39. P4 (stretch): symbolic Taylor/Laurent series stage
- [ ] 40. P5: wasm surface (`Expression::limit`, `limit_from_text`)

### B.5 FULL_SIMPLIFY_PLAN — draft, not started

Mathematica/SymPy-class `full_simplify` as a **new** entry point (leaves the
oracle-compatible `simplify` untouched). Chunks S1–S7, TDD-first:

- [~] 41. S1: `src/exact.rs` — exact constant evaluation + certified `is_zero(e) -> Tri` service (the keystone). **Core landed** (`exact::is_zero`, `exact::exact_eval`, `Exact` value type over ℚ+surds+π+e, trig on the π/12 lattice, exp/ln inversion, single-`RootOf` reduction mod its defining poly; `max_exact_eval_ops` §7f cap; `tests/exact_is_zero.rs`, 10 tests). **Remaining:** the behavior-preserving refactors onto it — diverge.rs `PiLin`/`exactly_zero_at`/`certified_nonzero_at`, matrix.rs eigen residual check, integrate I2 gate.
- [ ] 42. S2: `src/ratform.rs` — rational normalization (`together`/`cancel`/`ratsimp`) with opaque-kernel trick
- [ ] 43. S3: trig/exp/log special values + parity (answers the `sin(2π) ↛ 0` gap)
- [ ] 44. S4: factorization over ℚ (squarefree + rational-root + bounded Zassenhaus; multivariate)
- [ ] 45. S5: assumption/sign-propagation engine (unlocks `√(u²)→|u|`, `ln(uv)→ln u+ln v`, …; subsumes SINGULARITY T1b)
- [ ] 46. S6: radical denesting + rationalized forms (`radsimp`)
- [ ] 47. S7: trig restructuring (`trig_expand`/`trig_contract`) + cost-directed beam-search driver (`src/norm/cost.rs`)

### B.6 SINGULARITY_TRANSFORM_PLAN — draft, not started

Lift the improper-integral f64 "cliff" (~6–7 digits near a singular point
c ≠ 0) via the change of variables x = c ∓ t². Follow-up to DIVERGENCE.

- [ ] 48. T1: endpoint singularity at an exact rational endpoint — transform + `pull_nonneg_var` (T1b) normalizer + acceptance gate with fallback + (value, err) refactor in `improper_value`/`plain_value`
- [ ] 49. T2: interior singularity / multiple cells — split at each exact rational singular point, transform each endpoint half, sum error budgets
- [ ] 50. T3: irrational singular locations — **out of scope**, documented (stays on the cliff-capped path)

### B.7 FORM_GRADING_PLAN — draft, not started (greenfield, teaching)

Grade the *form a student wrote* (factored / reduced / decimal-vs-exact /
standard form / `+C`) as opt-in, composable predicates over the faithful
(pre-`flatten`) `Expr` — distinct from `equals` (value only). Modeled on
STACK/WeBWorK answer tests; standards diverge on what to enforce (CCSSM/TEKS
mandate many forms, Ontario almost none), so checks are per-problem opt-in.
Phased F0–F4:

- [ ] 51. F0: `preserve_grouping` parse option (skip the terminal `flatten` in `convert`, `src/parse/{text,latex}.rs`); analysis-only faithful tree; existing suites stay green
- [ ] 52. F1: structure-only `FormCheck`s in `src/form.rs` (`ReducedFraction`, `CombinedLikeTerms`, `Expanded`, `FactoredCompletely`, `SingleFraction`, `NoNegativeExponents`, `RadicalSimplified`, `MatchesForm`, `CompletedSquare`, `HasIntegrationConstant`) — reuse `js_match`/`factor`/`ratform`/`upoly` as oracles, never replacing the student tree
- [ ] 53. F2: `Prov` tag on `Num`/`Mul` (constant `Eq`/`Hash`/`Ord`; struct-form variants + `..`); wire `Decimal{places}`/`ExactValue`/`MulStyleIs` (the two facts the lexer/parser discard)
- [ ] 54. F3: wasm surface — `Expression.check_form(json)` / `grade(student, key, checks)` returning JSON `FormReport`s (verdict + `why`) for DoenetML
- [ ] 55. F4 (deferred): byte-span source maps — *only* if a UI wants caret-on-error highlighting; no sourced directive requires it
