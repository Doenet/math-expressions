# math-expressions (workspace)

Parse expressions like `sin^2 (x^3)` and do computer algebra on them — symbolic
differentiation and integration, numeric and symbolic equality testing,
simplification, assumptions, matrices, ODEs, and more.

This repository is the **monorepo root** (private; package name
`math-expressions-workspace`). The library itself is implemented in Rust and
shipped to JavaScript through wasm. The former JavaScript implementation has been
ported to Rust; its source and test suite are preserved out-of-tree under
`tmp/js-legacy/` (git-ignored) for reference — see [History](#history).

## Packages

Everything lives under `packages/`:

### `math-expressions-rs/` — the core (Rust)
The pure-Rust library: parsing (`text ↔ ast`, `latex ↔ ast`), equality
(numeric + finite-field + exact + structural), normalization / simplify / expand,
differentiation, symbolic + certified integration, matrices / eigenvalues, ODE
solving, assumptions, factoring, and an arbitrary-precision engine. Tests and the
JS-derived differential fixtures live in `tests/`; `scripts/` regenerates the
fixtures from the legacy JS oracle. No JavaScript — a pure library crate.

### `math-expressions-rs-wasm/` — the wasm boundary
The single place the core is compiled to WebAssembly and adapted for JS.
Co-locates two language trees (the Doenet layout):
- `src-rust/` (+ `Cargo.toml`) — the `math-expressions-wasm` `wasm-bindgen`
  crate: a thin adapter over the core's public API.
- `src-js/` — TypeScript bindings, principally the **AST → math.js bridge** for
  fast numeric graphing (Doenet + jsxgraph), plus the shared wasm handle types.
- `build-wasm.sh` — the **single source of truth** for the wasm build; other
  packages call it with a target (`nodejs` for synchronous Node use, `web` for
  the browser). `tests/` is a Vitest end-to-end suite that loads the browser
  build (ESM + `initSync`) and exercises every subsystem.

### `math-expressions-js-compat/` — the drop-in (published as `math-expressions`)
The directory is `math-expressions-js-compat`, but its `package.json` `name` is
**`math-expressions`** — this is the npm package (v3, `3.0.0-alpha1`), a drop-in
replacement for the original math-expressions JS API (`me.fromText(...).equals(...)`),
implemented in TypeScript over the wasm core — no math of its own. `lib/` is the
compat layer (mirrors the old `lib/**` module paths); `spec/` is the legacy
Vitest suite converted to TypeScript and run against the drop-in. *(Not all
legacy behavior is ported yet — see
[JS_TEST_COVERAGE_AUDIT.md](active-plans/JS_TEST_COVERAGE_AUDIT.md).)*

### `playground/` — Rust-vs-JS comparison app
A Vite/React app that runs the Rust (wasm) engine side-by-side with the
**original published** JS library (pulled in via the `math-expressions-canonical`
npm alias → `math-expressions@2.0.0-alpha94`) so their outputs can be compared.

## Development

Suggested development happens inside the dev container: open the repository in
VS Code and choose **Reopen in Container** for the correct Node.js and Rust
toolchains (including the `wasm32` target and a pinned `wasm-bindgen-cli`).

## Tests

```bash
# Core Rust library
cargo test --release            # some corpus tests are slow in debug

# wasm bindings — browser build + end-to-end suite
cd packages/math-expressions-rs-wasm && npm run build:wasm && npm test

# math-expressions drop-in (dir: math-expressions-js-compat) — legacy JS suite (TS)
cd packages/math-expressions-js-compat && npm run build:wasm && npm test
```

CI (`.github/workflows/ci.yml`) runs the Rust build/test, the playground build,
the wasm end-to-end suite, and the drop-in suite. The Rust suite includes
differential corpora checked against the original JS/mathjs behavior
(`packages/math-expressions-rs/tests/fixtures/`).

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
