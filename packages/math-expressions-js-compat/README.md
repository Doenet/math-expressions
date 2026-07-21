# math-expressions-js-compat

A drop-in replacement for the original **math-expressions** JavaScript API,
implemented in TypeScript on top of the Rust core (`math-expressions-rs`)
compiled to wasm. It has no math of its own — every method delegates to the wasm
bindings — and preserves the legacy synchronous surface:

```ts
import me from "math-expressions-js-compat";

const f = me.fromText("sin^2 x + cos^2 x");
f.toLatex();               // "\\sin^{2}\\left(x\\right) + \\cos^{2}\\left(x\\right)"
f.equals(me.fromText("1")); // true
me.fromText("x^2").derivative("x").toString(); // "2 x"
```

## Layout

- `lib/` — the TypeScript compat layer. `lib/math-expressions.ts` is the entry
  (the `Context`/`me` factory + `Expression`); the other files mirror the old
  `lib/**` module paths (`trees/`, `converters/`, `assumptions/`, `expression/`)
  so unchanged specs that import `../lib/...` resolve here.
- `lib/wasm-types.ts` / `lib/_wasm.ts` — typed structural surface for the wasm
  module and its synchronous loader.
- `vendor/wasm/` — the generated wasm bindings (git-ignored; build below).
- `spec/` — the original suite, copied verbatim from `tmp/js-legacy/spec` and
  renamed to `.spec.ts`. These run against this package.

## Build the wasm (required before tests)

```
./build-wasm.sh        # cargo build --target wasm32 + wasm-bindgen --target nodejs
```

This emits a **nodejs-target** (synchronous) wasm-bindgen package into
`vendor/wasm/`, so the legacy synchronous API works with no `await`, and it loads
directly under Node / Vitest. A browser (`--target web`, async-init) build is
future work.

## Test

```
npm test               # vitest run
```

The suite is the legacy JS test corpus. It is **not expected to fully pass** yet:
the Rust core is intentionally not byte-for-byte identical (clean-slate
formatter, folded normalization passes) and some legacy areas are unported
(polynomial/Groebner, mathjs/guppy/MathML converters, richly-structured
`get_assumptions`). Those specs still *run* and fail per-assertion. See
`../../active-plans/JS_TEST_COVERAGE_AUDIT.md` for the coverage ledger.

**Known exclusion:** `spec/slow_check-symbolic-equality-numerical-errors.spec.ts`
is present but excluded from the run (see `vite.config.ts`). It exercises
`equalsViaSyntax` with number tolerance (unimplemented — Rust's `equals_syntactic`
is exact) and, on a perturbed exp/log input, drives a *synchronous* wasm call
into a long/hung computation the Vitest timeout cannot interrupt. Re-enable once
the core guards that input.

## Build the library

```
npm run build          # vite build (ES + UMD)
```

> The Vite browser build is scaffolded but not yet wired to a browser-target
> wasm; the node/vitest path (via `createRequire`) is the supported one today.
