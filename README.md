# math-expressions

Parse expressions like `sin^2 (x^3)` and do computer algebra on them —
symbolic differentiation and integration, numeric and symbolic equality testing,
simplification, assumptions, matrices, ODEs, and more.

This repository is a Rust workspace. The former JavaScript implementation has
been ported to Rust; the old JS source and test suite are preserved out-of-tree
under `tmp/js-legacy/` for reference (see [History](#history) below).

## Layout

Everything lives under `packages/`:

- **`packages/math-expressions-rs/`** — the core Rust crate. Parsing
  (`text ↔ ast`, `latex ↔ ast`), equality, normalization/simplify/expand,
  differentiation, symbolic + verified integration, matrices/eigenvalues, ODE
  solving, assumptions, factoring, and an arbitrary-precision engine. Tests and
  JS-derived fixtures live in `tests/`.
- **`packages/math-expressions-rs-wasm/`** — TypeScript wrapper around the
  wasm-bindgen build, plus the `astToMathjs` shim (`src/tree-to-mathjs.ts`).
- **`packages/playground/`** — a Vite/React app for exercising the wasm build.

A `packages/js-compat/` package — a drop-in TypeScript replacement for the
original `math-expressions` JS API, built on the Rust core — is planned. The
`tmp/js-legacy/spec` suite will be converted to TypeScript and run against it.

## Development

Suggested development happens inside the dev container. Open the repository in
VS Code and choose **Reopen in Container** for a pre-configured environment with
the correct Node.js and Rust toolchains (including the `wasm32` target and a
pinned `wasm-bindgen-cli`).

## Tests

```
cargo test            # core Rust suite
cargo test --release  # same, optimized (some corpus tests are slow in debug)
```

The Rust suite includes differential corpora that check output against the
original JS/mathjs behavior (`packages/math-expressions-rs/tests/fixtures/`),
regenerable via the scripts in `packages/math-expressions-rs/scripts/`.

## History

The pre-port JavaScript library (`lib/`) and its Vitest suite (`spec/`) now live
under `tmp/js-legacy/` (git-ignored, kept on disk). The JS→Rust mapping is
documented in `active-plans/`:
[JS_RUST_DIFF.md](active-plans/JS_RUST_DIFF.md),
[JS_RUST_TEST_DIVERGENCES.md](active-plans/JS_RUST_TEST_DIVERGENCES.md),
[WHATS_LEFT.md](active-plans/WHATS_LEFT.md), and
[JS_TEST_COVERAGE_AUDIT.md](active-plans/JS_TEST_COVERAGE_AUDIT.md).

## License

Math-expressions is dual-licensed under GPLv3 and under Apache Version 2.0.
