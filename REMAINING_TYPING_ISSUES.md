# Remaining Typing Issues

This document lists typing issues that were intentionally left after the type-only pass, because fixing them completely would require runtime/API decisions.

## 1) Declaration/runtime mismatch kept for compatibility

These are currently typed to satisfy existing type tests and historical usage, but do not perfectly match the current runtime implementation.

- `Expression.factor()` and `Context.factor(expr)`
  - Declared in `index.d.ts`.
  - No corresponding exported implementation was found in `lib/expression/*`.
  - Current state: kept in types for compatibility.
  - Proposed fix: either implement `factor` in runtime or remove/deprecate from declarations.

- Context printing methods accepting `Tree`
  - `Context.toString`, `toLatex`, `tex`, `toXML`, `toGLSL`, `toGuppy` are typed as `Expression | Tree`.
  - Runtime printing helpers in `lib/expression/printing.js` access `expr.tree`, so they naturally expect `Expression`.
  - Current state: typed permissively for compatibility.
  - Proposed fix: add wrappers that convert `Tree` to `Expression` before calling printing helpers, or narrow declarations to `Expression` in a major-version change.

- `Expression.isAnalytic` and `Context.isAnalytic`
  - Runtime implementation takes options object (`allow_abs`, `allow_arg`, `allow_relation`).
  - Declarations currently also allow `string[]` to preserve prior typing expectations.
  - Current state: union type maintained for compatibility.
  - Proposed fix: deprecate `string[]` and move to options-only typing.

## 2) APIs where runtime shape should be confirmed and documented

These were updated in declarations to match observed runtime, but should still be reviewed and documented as public API intent.

- `integrateNumerically`
  - Runtime: `integrateNumerically(expr, x, a, b) => number`.
  - Declaration now reflects this signature.
  - Follow-up: confirm if this should remain context-only and whether an expression instance method should exist.

- `Context.matrix`
  - Runtime: `matrix(entries)` where entries are arrays of expressions.
  - Declaration now: `matrix(entries: Expression[][]): Expression`.
  - Follow-up: confirm whether `Tree` entries should also be supported via conversion.

- `Context.scalar_mul`
  - Runtime helper in `lib/expression/matrix.js` is `scalar_mul(k, v)` (scalar first).
  - Declaration updated to scalar-first parameter order.
  - Follow-up: confirm naming/ordering consistency with expression instance method.

- `Context.create_discrete_infinite_set`
  - Runtime: options object with `offsets`, `periods`, `min_index`, `max_index`.
  - Declaration now reflects object form.
  - Follow-up: confirm if this should be documented as a factory-only API.

## 3) Potential future cleanup items

- Review whether `Context.equals*` methods should remain public direct methods or be treated as expression-centric only.
- Add explicit API docs for context methods that require `Expression` at runtime.
- If a major version is planned, tighten permissive compatibility unions to exact runtime contracts.

## Suggested order of future work

1. Decide fate of `factor` (implement vs remove/deprecate).
2. Decide and standardize printing method input type policy (`Expression` only vs wrappers for `Tree`).
3. Decide whether to keep `isAnalytic(...string[])` compatibility.
4. Document the finalized contracts in README/types docs.
