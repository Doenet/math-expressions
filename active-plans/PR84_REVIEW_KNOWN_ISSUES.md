# PR #84 review — known issues and durable findings

The durable ledger from the twenty-two review passes over
[Doenet/math-expressions#84](https://github.com/Doenet/math-expressions/pull/84). The pass-by-pass
history lives in the git log (`Review cycle N:` commits) and the PR's edit history; this file keeps
only what still describes the code. Every entry below was re-verified against the pin it names or
carries a symbol anchor checked to exist at the head this file is committed at (`41b9cb4` when the
anchors were first swept at the eleventh pass, re-spot-checked at the thirteenth, fourteenth and
fifteenth); the fifth pass re-reproduced each then-open entry through the built compat package.

Conventions: "legacy" is `math-expressions@2.x` from npm. File paths are relative to
`packages/math-expressions-rs/src/` for `.rs` and `packages/math-expressions-js-compat/lib/` for
`.ts` unless said otherwise.

**The `null` sentinel is gone (twentieth pass).** `evaluate_to_constant` used to answer `null` for a
free variable or a placeholder blank, and `NaN` only for an indeterminate form. Legacy answered
`NaN` for all of them, and legacy was right: `null` is *anti*-poisoning in JavaScript
(`Number(null)` is `0`, `null + 5` is `5`, `null <= 1` is `true`, `Number.isNaN(null)` is `false`),
so a value that did not exist behaved like zero in any consumer that had not been individually
taught to test for it. Roughly fourteen DoenetML grading defects were traced to that one inversion,
found one at a time over the preceding passes. The compat layer answers `NaN` now, and the
declarations say `number | Complex` rather than `number | Complex | null`. Entries below that
turned on the old sentinel are struck through or amended in place rather than deleted, so the
history stays readable. The native Rust API keeps `Option<f64>`, which is right where there is no
coercion hazard.

## Known issues, open

None of these block DoenetML (Doenet/DoenetML#1622); they are recorded for follow-up work.

### Rust crate

- **DP5(4) evaluates stage 7 twice per accepted step** (`mathjs_compat/ode.rs`, `solve_ode`): the
  FSAL stage loop already produced `f(t+h, ynew)` into `k[6]`. 7 RHS calls per step instead of 6,
  and through `solve_ode` each one is a JS boundary crossing. The `terminated_early` branch hanging
  off it is dead.
- **`max_steps` is off by one** (`mathjs_compat/ode.rs`): an integration converging in exactly
  `max_steps` steps reports `terminatedEarly`. The vanishing-step guard beside it also uses a
  different scale from the completion test, so a large-`t` run can reject its final sliver.
- **`digits = +Infinity` disables the decimals mode** in
  `round_numbers_to_precision_plus_decimals` (`ops/numbers.rs`), asymmetrically with `-Infinity`.
- **`evaluate_many`'s scalar fallback skips canonicalization** while the tape path canonicalizes,
  giving intra-batch 1-ulp inconsistency.
- **`sort_key`'s `ignore_negatives` parameter is permanently `false`** at all three call sites,
  making several branches of `normalize/default_order.rs` unreachable. (Its doc no longer
  contradicts the code about whether nested keys propagate it — the `Pow`, `Apply` and unit
  branches do, the rest do not.)
- **Two-argument `log` does not fold** (`log(8,2)` stays an application), while the `log_10` /
  `log10` spellings do.
- **`polynomials::rootof::expr_to_upoly` reads a monomial of any degree**, so
  `rootof(x^1000000000 - 1, 0)` allocates a billion `BigRational` zeros before `make_rootof` can
  refuse the degree. Pre-existing, and untouched by the seventeenth pass's product reading, which
  deliberately left that arm alone so the change could be a strict widening; the cap belongs on the
  dense allocation, not on the product.
- **`mono_less_than` answers `true` in both directions** (`polynomials/compat/mono.rs`) for two
  distinct variables that `cmp_default_order` ranks `Equal` — the tie case `mono_gcd` already
  acknowledges.
- **`evaluate_to_constant` returns `Some(NaN)`** while its rustdoc mentions only `±∞`, and the
  `evaluate_to_complex` beside it rejects `NaN`. Undocumented asymmetry.
- **Non-realness does not propagate through `+`, `*` or `^`** — incompleteness, deliberately
  declined, with no consumer left that can be wrong about it. `real: Some(false)` is produced in
  only three places (an explicit `x ∉ R` assumption, the `i` literal, a constant with a nonzero
  imaginary part) and `combine::add`/`combine::mul` never carry it through an operator, so
  `is_real(x+1)` with `x ∉ R` answers _unknown_ where legacy answers `false`. Adding the rules
  would turn `None` into `Some(false)`, and `simplify`'s rewrites are gated on exactly those
  facts; the one rewrite that treated `None` as permission (the odd-root sign extraction in
  `normalize/simplify.rs`, `simplify_root`) now declines when any _part_ of the residual is
  provably non-real, which over-declines and moves no other rewrite.
- **`MAX_UNFLATTEN_OPERANDS = 1000`** (`math-expressions-rs-wasm/src-rust/js_match.rs`) **exceeds
  serde_json's 128-deep default**, so `unflatten_left` on a wide sum returns JSON that `from_ast`
  then refuses.
- **An exact integer past f64 range crosses to JS as `Infinity`.** `simplify` folds `2^2000`
  exactly and `to_text` prints all 613 digits, but `to_js` puts each part through an f64, so
  `.tree` answers `Infinity` and `2^2000` and `2^2001` have the same `.tree` while `equals` still
  tells them apart. Parity with legacy (which held everything in a JS number) — a limit of the AST
  wire format, not a regression — recorded because `max_pow_bits` deliberately permits results a
  thousand times past the f64 ceiling.
- **`1/(0^0)` stays written out** as `["/", 1, NaN]` rather than folding to `NaN`. Every other
  arithmetic combination with a `NaN` operand folds. (The `{"$":"NaN"}` envelope no longer reaches
  `.tree`; that half is fixed — see `engine-rust.ts` in DoenetML and the corresponding
  `MATH_EXPRESSIONS_UPSTREAM_REQUESTS.md` entry.)

### Compat layer

- **Handle leaks, systemic.** `evaluate_to_constant` creates intermediates via `remove_units` and
  `simplify` and frees neither; `Context.matrix` has the same shape; `equalSpecifiedSignErrors`
  mints a `fromAst` per sign variant per recursion level (the `numSignErrorsMatched` grading
  path); `trees/basic.ts`'s `evaluateNumbers` leaks two per rewrite per pattern per round inside
  `applyAllTransformations`; `evaluate_numbers`'s `set_small_zero` branch and
  `create_discrete_infinite_set` each discard intermediates. Systemically, every `toExpr(other, …)`
  in `equals`/`add`/`match`/… leaks whenever the argument is a tree or a string — `substitute` was
  the only method freeing carefully.
- **Nine declared parameters the implementation has no arity for.** Found by a member-by-member
  audit of `types/math-expressions.d.ts` against `lib/` at the twenty-first pass, prompted by the
  two the twentieth pass had found by accident (`match`'s `allow_permutations?: boolean`, and
  `evaluate_to_constant`'s `| null`). `simplify`, `simplify_logical`, `collect_like_terms_factors`,
  `simplify_ratios` and `expand` are all declared with options and are arity 0; `derivative`'s
  `story` array is never written; `equalsViaReal`/`equalsViaComplex` ignore their `EqualsOptions`,
  so their tolerances have no effect; and `isAnalytic`'s declared `string[]` arm is read as an
  options object, so every flag comes out `false` — `match(true)` a second time. All nine are now
  marked `@deprecated` and "accepted and ignored" in the declarations, and the `string[]` arm is
  gone, which is honesty rather than a fix: the engine should either honor them or they should be
  dropped. Verified against the built package, not read off the source — and re-verified the same
  way at the twenty-second pass, where all nine still behave exactly as described. The verdict
  stands: **accepted and ignored**, because each is a parameter legacy honored and this engine has
  no arity for, so the only alternatives are engine work (the ask upstream) or breaking a legacy
  call that compiles today; the declaration saying so costs neither.
- ~~**The parsers read a non-string as a pointer into linear memory.**~~ Fixed at the
  twenty-second pass, and the reason it is recorded rather than quietly patched is that the
  twenty-first pass's `add_unit` fix reported having swept "the rest of the string-taking entry
  points; all were already guarded" — and `parse_text`/`parse_latex`, the two most-used entry
  points in the package, were not. `me.fromText(5)`, `me.fromText(anExpression)` and
  `me.fromText({})` were all `RuntimeError: memory access out of bounds`; an array tree was
  `arg.charCodeAt is not a function`. (The module recovers — the allocation fails at the boundary
  rather than corrupting the heap — but the message is engine-internal, and `<mathInput
  showPreview>` renders whatever the parser complains about, so a student could reach it.) They now
  throw a `TypeError` naming the argument type and pointing at `fromAst`/`from`. A *throw*, where
  `add_unit` took a coercion, because `add_unit`'s declaration invites an `Expression | Tree` and a
  unit is a symbol, while `fromText` is declared to take a string and no other value has a faithful
  reading — so nothing that used to succeed changed. `String` objects still parse, as wasm-bindgen
  always read them. Pinned in `quick_doenet_open_items.spec.ts`, revert-fail-restore verified.
  The rest of the sweep the twenty-first pass claimed *does* hold: every other string-taking wasm
  entry reachable from the published surface was re-probed at runtime with an `Expression`, an
  array tree and a number, and each is guarded (`varName` on
  `derivative`/`integrate`/`critical_points`/`evaluate_many`/`solve_linear`/`add_unit`,
  `JSON.stringify` or `tree_json()` on every options/AST parameter, `.toString()` on the assumption
  texts).
- ~~**48 of the 114 declared `Expression` members are `undefined` at runtime.**~~ Decided at the
  twenty-second pass, which is what the entry had been waiting for: **narrowed**, not documented.
  The members are gone from both `Expression` and `Context` in
  `types/math-expressions.d.ts` — 96 declarations, plus `Context`'s own `ZmodN` and
  `parser_parameters`, which are Context-only properties and so fell outside the `Expression` audit
  that found the 48. The argument for narrowing is that a `.d.ts` whose job is to describe a
  drop-in earns nothing by promising members that are not there: keeping them made `expr.sin()` a
  compile-time success and a runtime `TypeError`, which is the worse of the two places to find out,
  and removing them moves the report to `tsc`, names the member, and costs a caller who was going
  to fail anyway nothing. What is left is checkable and was checked: every member either interface
  declares — 66 on `Expression`, 87 on `Context` — is present at runtime on the built package. The
  gap itself is unchanged and is still the open ask upstream; it is enumerated in a comment at the
  end of `Expression` and in `MATH_EXPRESSIONS_UPSTREAM_REQUESTS.md`, and a name goes back the
  moment `lib/` implements it. DoenetML's vendored copy was narrowed in the same shape, and its
  `npm run typecheck` is unchanged by it (22 packages clean, the same 5 not gated, the same 59
  pre-existing errors), which is the measurement that nothing called them.
- **`Context.toString(expr)` answers `"[object Object]"`.** The expression-first mirror skips
  anything already `in Context`, and `toString` is inherited from `Object.prototype` — deliberately,
  since shadowing it would break `String(me)`. The declaration promised it anyway; it no longer
  does. `expr.toString()` is unaffected and is the only spelling that works.
- ~~**`Context.assumptions`, `get_assumptions`, `solve_linear`, `Context.from`,
  `create_discrete_infinite_set` and `Context.class` all return or accept something the
  declaration does not admit.**~~ Decided at the twenty-second pass, one verdict each, all six
  measured against the built package first. Five were the **declaration** being wrong about a
  deliberate implementation, and the declaration now says what the code does: `assumptions` is
  typed as the object of methods it is (the per-variable facts are under `byvar`);
  `get_assumptions` takes the three query shapes that work — a name, a *nested* `[["x","y"]]` list,
  or an expression — and returns `Tree | undefined`, the bare `["x","y"]` it used to declare being
  the one shape that answers `undefined` (legacy's own suite queries `[["x"]]`, so the nesting is
  parity, not a quirk); `from` and `create_discrete_infinite_set` are declared `| undefined`, which
  is legacy's failure value and what callers must check; and `class` takes a wasm handle, now
  declared `never` so `new me.class(tree)` is a compile error rather than an object whose every
  method fails. The sixth, `solve_linear`'s frozen `ABSENT_EXPRESSION`, is **accepted**: legacy
  handed back an `Expression` whose `.tree` was `undefined` and callers read `.tree`
  unconditionally, so declaring `| undefined` would break the callers the stand-in exists to serve.
  Documented in place instead — test the `.tree`, not the result.
- ~~**`Expression.match` drops `allow_extended_match`; the free `utils.match` honors it.**~~ Fixed
  at the twenty-second pass. The option is handled *outside* the Rust matcher — `trees/flatten.ts`
  enumerates operand subsets — and `Expression.match` called the matcher directly, sharing only
  `normalizeMatchOptions` while its comment claimed the two entry points could not drift. It now
  delegates to that shared `match`, which is what makes the claim true, and `MatchOptions` declares
  `allow_extended_match` because it now works from both. `x+y+z` against `a+b` bound `b` to `y+z`
  here and to `y` with `_skipped: ["z"]` there; both answer the second now. The no-options path is
  still gated on `hasOptions`, so an absent or empty options object keeps the legacy default where
  every string leaf in the pattern binds. Pinned in `quick_doenet_open_items.spec.ts`,
  revert-fail-restore verified.
- **`astToJson` and `astReplacer` are not interchangeable** despite their shared file's claim:
  `astToJson` tags non-finites but does not unwrap an `Expression`, so the tree utils reject one
  where `fromAst` accepts it.
- **`extendedMatch` produces `_skipped` but never `_skipped_before`**, leaving the `addLeft` path
  in `trees/basic.ts` dead. (`_skipped` itself is live, set by `trees/flatten.ts`.)
- **`applyAllTransformations` folds numbers only after the extended-match splice**, where legacy
  folds before it as well. The pre-fold's only observable effect is on which branch the
  `result[0] === pattern[0]` test takes; noted in the code, to keep one `fromAst` round-trip per
  rewrite. Separately, the `applyAllTransformations` _method_ on `Context`
  (`math-expressions.ts`) is documented as a normalization pass folded into `canonicalize` and
  returns `this`, silently discarding the caller's transformation list — it is neither; the real
  pattern-rewriting driver lives in `trees/basic.ts`, which nothing re-exports (`Context.utils`
  carries only `{match, flatten, unflattenLeft, unflattenRight}`).
- **`substitute_component` validates nothing**, where legacy validated the container head at each
  level and the index range. `me.fromText("x*y").substitute_component(0, 5)` answers `5·y` instead
  of throwing, and an out-of-range index returns `undefined` rather than an `Expression`, so the
  caller fails a line later on `.tree`. `get_component` has the same shape one level down: its
  container check runs on the receiver only, and the rest of the path indexes the operands of any
  operator — `("(x*y, 3)").get_component([0,0])` answers `x`. The comment describing a matrix
  entry as `[1, row, col]` describes a call the code rejects (`"matrix"` is not in
  `COMPONENT_CONTAINERS`). DoenetML's `@doenet/math` `getComponent` wrapper restores the legacy
  throw for the one call site that used it as a type test.
- ~~**`Expression#match` silently ignores `allow_extended_match`**~~ — the same finding as the
  entry above, filed twice; fixed once, at the twenty-second pass, by delegating to the shared
  implementation the way legacy's `Expression.prototype.match` did.
- **`ABSENT_EXPRESSION` snapshots the prototype before it is finished.** The `notImplemented`
  methods and `applyAllTransformations` are attached after the IIFE builds it, so
  `solve_linear(...).applyAllTransformations()` is a `TypeError` rather than the documented
  "returns the stand-in itself"; `toText()`/`tex()` hand back the stand-in _object_ rather than
  `""`, and `Symbol.dispose` is absent.
- **`me.from` never tries MathML** although `converters.MmlToAst` exists and works; legacy's
  `create_from_multiple` had that third fallback, and `Context.fromMml` is still `notImplemented`.
- **`Context.reviver` drops the `assumptions` field** legacy restored onto a revived expression,
  and `toJSON` no longer emits it — silent on both sides of a persist/revive round trip.
- **`evaluate_to_constant` does not read `nan_for_non_numeric`.** It now always behaves as legacy's
  `true` default — `NaN` for anything with no numeric value — so the only remaining divergence is
  that passing `false` is accepted and ignored rather than producing `null`. Marked `@deprecated`
  in the published declarations.
  _(Was: "always behaves as `false`, and DoenetML depends on it." Both halves were wrong to rely
  on. The `null` sentinel is gone; see the note at the top of this file.)_
- ~~**`evaluate_to_constant`'s blank-handling comments describe the wrong trees.**~~ Resolved by
  deletion: `treeHasBareBlank`/`treeHasBlank` existed only to split blanks between the `NaN` and
  `null` answers, and there is one answer now.
- **`equalSpecifiedSignErrors` does not require _exactly_ `n_sign_errors`,** as its docstring
  says. `singleNegations` enumerates sign-invariant positions too, so negating `x` inside `x^2`
  folds back and a perfectly correct answer scores as "1 sign error" on DoenetML's
  `numSignErrorsMatched` path. Possibly legacy-faithful; the doc should not claim otherwise
  either way.
- **The render-option key list is enumerated in four places and each is different.**
  `converters/render-options.ts`'s `FORWARDED` is the authority; two lists in
  `math-expressions.ts` omit `avoidScientificNotation` and `matrixEnvironment`, `ast-to-text.ts`
  omits `notation` and `matrixEnvironment`, and
  `packages/math-expressions-rs-wasm/src-js/wasm.ts` (outside this file's `lib/` path convention)
  omits both and drops `unicode` from the LaTeX variant only.

## Standing invariants worth knowing

- **Nothing anywhere under `lib/` may dereference `wasm` at module scope.** `setWasmModule` is
  re-exported from the package root, so importing it evaluates the whole barrel; a module-scope
  `wasm` touch triggers the node fallback — throwing in a browser, and under node quietly pinning
  the node build so a later injection can never win. The invariant is written at its site in
  `lib/math-expressions.ts` (the `Context._assumptionsHandle` lazy accessors) and pinned by a spec
  that injects a counting proxy and asserts zero touches during import.
- **The 11 skipped compat tests** are 9 in `quick_trees.spec.ts` and 2 in
  `slow_assumptions.spec.ts`; all but one carry a `[wontfix: …]` tag in the test name saying why.
  None is an engine unsoundness: legacy's expected answers there are partly false, so the tests
  cannot be passed soundly. See `active-plans/ASSUMPTIONS_ENGINE_PLAN.md` ("Accepted divergence").
  The exception is `slow_assumptions.spec.ts`'s "define constants" (`:7292`), which carries a plain
  comment rather than a tag — worth tagging so the count stays self-explaining.
- **Aggregates have no default parser spelling**: `fromText("sum(3,17,5-4)")` parses as
  `s·u·m·(…)` unless `appliedFunctionSymbols` is passed. Deliberate, matches legacy.

## Fixed during review, kept for its contract

**`add_unit` corrupted the wasm heap when handed the argument its declaration invites**
(twenty-first pass). The wasm entry point is `add_unit(unit: &str)`, and wasm-bindgen reads a
non-string argument as a pointer/length pair into linear memory. The published declaration says
`Expression | Tree`, as legacy's did, so `add_unit(me.fromText("%"))` — the documented call — gave
`RuntimeError: memory access out of bounds` and an array tree gave
`arg.charCodeAt is not a function`. The fix is the `varName` coercion `critical_points` already
used against the identical hazard, and it is worth recording that the hazard had been *named* in a
comment one method away for several passes without anyone checking which other methods had it. A
unit is a symbol, so its name is all the Rust side wants. Pinned in
`quick_doenet_open_items.spec.ts`, revert-fail-restore verified.

**`f((a, b))` and `f(a, b)` are one tree, because `to_js` cannot tell them apart** (eighteenth
pass). `f((x, y))` parsed to `Apply(f, [Seq(Tuple, [x, y])])` and `f(x, y)` to `Apply(f, [x, y])`,
but `to_js` writes `["apply","f",["tuple","x","y"]]` for *both* — byte-identical JSON — and
`try_from_js` maps that back to the second. The serialization was not injective, and the JS AST is
the contract with every consumer, so an expression was not equal to itself after a round trip
through its own `.tree`: `me.fromText("f((1,2))").tree` equalled `me.fromText("f(1,2)").tree` while
`x.equals(me.fromAst(x.tree))` was **`false`**.

The fix is in the **parsers**, not in canonicalization, because the printers read the raw tree: a
canonical-form fix would have repaired `equals` and left the display wrong. `parse::common::apply`
flattens a lone `Tuple` argument exactly as `expr::serde::try_from_js` always has, and every
`Expr::Apply` the two parsers build now goes through it — the call form, the simplified
application, `|…|`, `⌊…⌋`, `⌈…⌉`, `√`, `∛`, `…!` and the integral. Only a *lone* tuple flattens: in
`f((x, y), z)` the inner tuple is one of two arguments, survives the round trip intact, and is left
alone. Legacy had one tree for both spellings, so this is parity, not a new rule.

It was never a grading defect — DoenetML's `checkEquality` rebuilds both operands with `me.fromAst`
one line before `.equals()`, so the distinction was erased on the way in — but it was a **display**
regression: the same saved JSON rendered `\sin\left(\left( x, y \right)\right)` before a
save/restore and `\sin\left( x, y \right)` after. Pinned in `tests/js_ast_image.rs`, whose
corpus sweep states the property directly (rendering is a function of the saved JSON) over every
parser fixture plus six hand-written spellings, and which fails both against the unfixed parsers
and against the plausible over-flattening variant that spreads *every* tuple argument.

Two things followed from it. The sixteenth pass's `normalize::spread_list_argument` keeps its
place, but for the *other* list kinds — `mod([7,3])` and `["apply","mod",["list",7,3]]` are still
one sequence argument, and `Tuple` no longer reaches it from any parser — so its tests now exercise
the bracketed spelling, which is the one that can still fail if the branch is narrowed. And the
LaTeX printer's bracket notations turned out to be guarded on `args.len() == 1`, falling through to
`head\left(…\right)` otherwise and spelling the head as a command that does not exist: `abs(x, y)`
rendered as `\abs\left( x, y \right)` and `sqrt(x, y)` as `\sqrt\left( x, y \right)`, neither
of which MathJax can render. They now wrap the tuple, which is both what the JS AST says the
argument is and what legacy rendered (`print/latex.rs::sole_argument`, pinned in
`tests/formatter_fixes.rs`).

**A `rootof` whose polynomial is written as a product reaches the same leaf** (seventeenth pass).
`canon_apply` rewrites `rootof(p, k)` into the `Expr::RootOf` leaf only when
`polynomials::rootof::from_apply_args` accepts, and `expr_to_upoly` read only a *sum of monomials*.
Canonicalization does not expand products, so a factored spelling was declined and stayed an
application of a head with no evaluation at all — an opaque atom. Two spellings of the same number
therefore compared unequal: `rootof((x-1)(x-2), 0)` was neither `1` nor `rootof(x^2-3x+2, 0)`.
`expr_to_upoly` now multiplies and adds polynomials.

Three things the sixteenth pass wrote about this were imprecise, and measuring them is what set the
fix's shape. It is not "dense canonical" input that was required — sparse (`x^2-2`) reads fine, and
so does a *scaled* one: `make_rootof` normalizes to primitive integer coefficients with a positive
leading coefficient, so `rootof(2x^2-6x+4, 0)` and `rootof(x^2/2-3x/2+1, 0)` already equalled `1`.
The one shape that failed was an **unexpanded product** — `(x-1)(x-2)`, `2(x^2-2)`, `x(x-1)`. And
the degree guard cannot simply be `max_rootof_degree` applied everywhere: the first draft put it on
every arm and thereby *narrowed* `rootof(x^70 - x^69, 0)`, which the old reading accepted because
`make_rootof` takes the squarefree radical (degree 70 → `t^2 - t`). The cap is on products only,
where multiplying many-term polynomials grows the coefficients as well as the degree —
`(x^2+x+1)^200` alone spent ten seconds under a more generous cap — while a monomial sum costs
nothing to read at any degree and keeps its old, uncapped arm. Pinned in
`tests/rootof_adversarial.rs`, both the widening and the two refusals, verified to fail against the
unfixed reading.

The residue this closes was never on a DoenetML path: `rootof` is in neither of DoenetML's
`appliedFunctionSymbols` lists (`utils/math.ts` has no occurrence of the name), and legacy
`math-expressions@2.x` has no `rootof` at all, so it is a defect in this engine's own new surface
rather than a regression. It is fixed rather than filed because that surface ships as
`math-expressions@3.x` to npm, where a library caller reaches it through the default text parser —
which applies any name followed by a parenthesized list — and through `\operatorname{rootof}` in
LaTeX.

**A sequence argument is read as an argument list by the sampler too** (sixteenth pass). The same
split as the `det` entry below, on the same path, found by asking what else the two layers could
disagree about. Legacy's text parser wrote one tree for `mod(7,3)` and `mod((7,3))` — a head
applied to a tuple — so the extra parentheses cost nothing and both answered `1`. This parser kept
them apart, and only `normalize/fold_apply.rs` put them back together, via an `effective_args`
helper that spread a single list argument. The sampler in `eval_numeric/complex.rs` did not, and
`known_function("mod", 1)` is false, so it called the application an opaque atom: `simplify` gave
`1`, `equals(mod((7,3)), 1)` gave `false`, and `evaluate_to_constant` gave `None` — `<number>` read
nothing and `<answer>` graded it wrong. `nPr` and `nCr` are the other two heads whose folder takes
the arity the spread produces. (The *parenthesized* spelling is no longer this helper's business —
the eighteenth pass's parser fix, above, makes `mod((7,3))` the same tree as `mod(7,3)` — but the
bracketed `mod([7,3])` and the JS `["apply","mod",["list",7,3]]` still are, and that is what the
tests now exercise.)

An earlier pass had this in the open list, as "`fold_apply::is_variadic` tests 'has an exact
folder' rather than 'is an aggregate', so a tuple argument spreads into fixed-arity heads". Two
things in that were wrong, and both had to be established by measurement before the fix could be
the right one. The spreading is not the bug — it is legacy parity, and narrowing `is_variadic` to
the aggregates makes `mod((7,3))` stop being `1`, which is a *regression*. And the example given,
`["apply","mod",["tuple",7,3]]`, never took the branch: the JS deserializer flattens a tuple
argument into an argument list before any of this runs, so a DoenetML tree could not reach it and
only the text parser could — which is the observation the eighteenth pass followed back to the
parsers.

The spread is now `normalize::spread_list_argument`, `pub(crate)` and consulted by both layers, the
way `det`/`trace` go through `matrix::scalar_reduction`. `head_evaluable` asks it for the effective
arity and `eval_apply` evaluates the spread list; `free_symbols` needed no change, for the same
reason it did not for `det`. The arity check still happens downstream on the spread list, so
`abs([-3,5])` stays symbolic on *both* layers rather than being forced into a two-argument `abs`.
Pinned in `tests/equality.rs`, `normalize/fold_apply.rs` and
`spec/quick_doenet_compat_pr84.spec.ts`, verified to fail against the unfixed engine.

**`det`/`trace` of a literal matrix are evaluated, not sampled as unknowns** (fifteenth pass).
`equals` said a determinant differed from its own value: `\det\begin{pmatrix}1&2\\3&4\end{pmatrix}`
simplified to `-2` and compared `false` against `-2`, and `evaluate_to_constant` answered `None` on
it. The cause was the third numeric path — `equals` samples through `eval_numeric/complex.rs`, whose
`head_evaluable` asked `special_functions::eval1` alone. `DET` had no kernel and `TRACE`'s is a
scalar identity that cannot see into a `Matrix`, so `is_opaque_atom` classified the whole
application as an opaque atom and sampled it as a fresh variable, which agrees with `-2` nowhere.
The legacy JavaScript library answered `-2`/`5`/`true` to all of it, so this was a regression, on a
grading path.

`matrix::scalar_reduction` is now the single place that decides whether an application of `det`/
`trace` has a scalar value; `normalize/fold_apply.rs` (which keeps its `Num`-only gate, so
`simplify` is byte-unchanged) and `eval_numeric/complex.rs` both call it. `head_evaluable` takes the
argument list rather than its length so it can ask; `free_symbols` needed no change, since an
application that is no longer opaque already descends into its arguments and `Expr::Matrix` already
walks its entries — so `det([[x,2],[3,4]])` reports `x` and compares equal to `4x-6`. A matrix the
reducers decline (non-square, over `resource_limits`) still comes back as the `OtherOp` residual and
is still sampled as an unknown. `DET` also gained the scalar identity `TRACE` had, matching mathjs's
`det(2) = 2` and legacy's `det(x) == x`.

Pinned in `tests/matrix.rs` and `spec/quick_doenet_compat_pr84.spec.ts`, both verified to fail
against the unfixed engine. `tests/functions_registry.rs`'s deny list — which is where the omission
was codified, as `erf`'s had been — now says in the file that it pins a decision rather than an
outside fact, and names the compat suite as the check that has outside authority.

**Odd roots of negative reals read on the real branch on every numeric path** (eleventh pass).
The branch for `(negative)^(p/q)`, odd `q`, used to depend on whether the radicand was a perfect
power — `(-8)^(1/3)` folded to `-2` while `(-2)^(1/3)` evaluated to the principal
`0.6300 + 1.0911i` — so `equals` told the same number apart from itself and four DoenetML
`<answer>` cases regressed against legacy. Fixed at three sites: `rule_radical`'s `Pow` arm
(`normalize/simplify.rs`) pulls the sign out at simplify time, which is load-bearing because
`evaluate_to_constant` runs `simplify_core` and the certified-digits tape before any evaluator;
`eval_complex`'s `Pow` arm (`eval_numeric/complex.rs`, `odd_root_exponent`) takes the same branch
for sampling — matching the raw quotient-node exponent shape too, because that walk is
`evaluate_many`'s per-point fallback and gating on `Num(Rat)` alone diverges the batch and
single-point paths at 835 corpus points; and `CBRT::eval1`/`NTHROOT::eval2`
(`special_functions/powers.rs`) follow. Even roots, decimal exponents with even reduced
denominators (`(-8)^0.3333` = `3333/10000`), and complex bases stay principal. This is a
deliberate divergence from mathjs on the engine's *own* numeric paths (`x^(1/3)` at `x = -8` is
`-2` there, not `1 + i√3`), stated in `evaluate_fast_f64`'s rustdoc. **It does not extend to
`f()`**, which compiles the tree to math.js and so keeps mathjs's principal branch for a `Pow`
node: `f()` of `x^(1/3)` at `x = -8` is `1 + i√3`, while `cbrt` and `nthroot` — which map onto
math.js functions that take the real branch themselves — are `-2`. So `evaluate_many` and `f()`
disagree about the power spelling and agree about the root spellings, and a DoenetML
`<function>x^(1/3)</function>` still has a gap at negative inputs that `<answer>` grading does not.
That gap is unchanged from legacy (which also evaluated the power spelling principal through
`numericalf`), so it is a standing difference rather than a regression, and closing it would mean
mapping the odd-root `Pow` shape onto `nthRoot` in `tree-to-mathjs.ts`. Pinned in
`tests/odd_root_real_branch.rs` and `spec/quick_doenet_grading_gaps.spec.ts`, both verified to fail
against the unfixed engine.

**`f()` could not compile `nthroot`** (twelfth pass). `functionConversions` in
`packages/math-expressions-rs-wasm/src-js/tree-to-mathjs.ts` maps AST heads onto math.js names,
and math.js spells this one `nthRoot`. An unknown head is not a compile error — it becomes a
`FunctionNode` over an undefined symbol and throws `Undefined function nthroot` on the first
`evaluate` — so `nthroot(x, n)` was unevaluable through `f()` at _every_ input, not only at
negative ones. `f()` is the plotting and root-finding entry point, so a DoenetML
`<function>nthroot(x,3)</function>` drew nothing at all; legacy plotted it. Now mapped, which also
puts an odd root of a negative on the real branch (`nthRoot(-8, 3) === -2`), consistent with the
odd-root entry above and with `cbrt`. Pinned in `spec/quick_doenet_compat_pr84.spec.ts`, verified
to fail with the mapping removed.

**The sibling sweep that entry asked for, done** (thirteenth pass). Every spelling the Rust
registry can produce was diffed against `Object.keys(mathjs)` and against `functionConversions`,
and every author-typable spelling — the union of DoenetML's `appliedFunctionSymbolsDefault` and
`…Latex`, 69 of them — was then evaluated through `f()` at two in-domain points. `nthroot` was the
only head broken that way. One head, `rootof`, is deliberately unmapped: it is in neither of
DoenetML's applied lists, so it cannot be typed, and the `critical_points()` output that produces
it goes through `evaluate_to_constant`, never `f()`.

**And the sweep's own blind spot** (fourteenth pass). It covered `f()` and `evaluate_to_constant`;
there is a third numeric path, the sampler `eval_complex` that `equals` runs on, and it fails
*differently* — a head it cannot evaluate becomes an opaque variable rather than a `NaN`, so the
divergence is an equality that answers `false` instead of a value that reads `NaN`. `det` and
`trace` are in that state; see the entry at the top of "Known issues, open". The registry test
(`tests/functions_registry.rs`) does not catch this class either: it asserts that the evaluable
list evaluates and the not-evaluable list does not, so it pins whatever is true rather than
testing the list against an outside authority. A head that *ought* to be evaluable, is not, and is
written into the deny list passes — which is exactly how `erf` was codified, and how `det` still
is.

**`erf` had no evaluation kernel at all** (thirteenth pass) — the mirror image of `nthroot`, and
just as silent. `ERF` in `special_functions/misc.rs` carried parser spellings and LaTeX rendering
but no `eval1`, so `evaluate_to_constant("erf(0.5)")` was `None` and `evaluate_many` sampled `NaN`
at every point, while `f()` was right throughout because math.js *has* `erf`. A DoenetML
`<function>erf(x)</function>` therefore plotted a correct curve whose
`<number>$$f(0.5)</number>` read `NaN` and whose extrema search found nothing — and legacy
evaluated `erf` from all of those paths, so this was a regression. `eval1` is now a port of the
same W. J. Cody rational-Chebyshev approximation math.js uses, so the two paths agree to the last
bit rather than to a tolerance. Re-measured independently at the fourteenth pass over 38,385
sample points (both tails, both interval boundaries, denormals, ±0, ±∞, NaN): **0 mismatches on
the shipped wasm build**, because wasm32 Rust uses the `libm` crate's fdlibm `exp` and V8 uses the
same one. A *native* `cargo test` build differs at ≤2 ulp (max relative 3.6e-16, all inside the
`erfc2` branch) because it links glibc's `exp` instead — a property of the two `exp`s, not of the
port, and ~2,800× under the 1e-12 `relative_tolerance` grading uses. Worth knowing because
`spec/quick_doenet_compat_pr84.spec.ts`'s exact `toBe` on an `erf` value is a stricter contract
than that. Pinned in `tests/erf.rs` (which also asserts the three numeric
entry points agree) and `spec/quick_doenet_compat_pr84.spec.ts`, verified to fail with `eval1`
removed. The general lesson is the one the sweep confirms: a head can be missing from *either*
path, and neither absence produces a warning.

Everything else fixed during the review passes is described by its `Review cycle N:` commit and
its tests; the suite state at this head is `cargo test --workspace` 876 passed / 0 failed and the
compat suite 6,383 tests — 6,372 passing, 11 skipped, 0 failing — with `cargo fmt` and
`clippy -D warnings` clean. (These two numbers were written at the thirteenth pass and left to rot
through seven more; they were re-measured at the twenty-first and again at the twenty-second, each
from a redirected run whose own exit status was checked. If you are editing this line, re-run them — a count that says "at this
head" and is not is worse than no count.)
