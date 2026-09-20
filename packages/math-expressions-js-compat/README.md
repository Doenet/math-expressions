# math-expressions-js-compat

This directory is `math-expressions-js-compat`, but it is **published to npm as
`math-expressions`** (v3 — see `package.json` `name`). It's a drop-in replacement
for the original math-expressions JavaScript API, implemented in TypeScript on
top of the Rust core (`math-expressions-rs`) compiled to wasm. It has no math of
its own — every method delegates to the wasm bindings, and the few converters
that stay in TypeScript (AST ↔ math.js nodes, AST → Guppy XML) only relabel
notation — and preserves the legacy synchronous surface. (The older published JS library is `2.0.0-alpha94`.)

```ts
import me from "math-expressions"; // the published name

const f = me.fromText("sin^2 x + cos^2 x");
f.toLatex();               // "\\sin^{2}\\left(x\\right) + \\cos^{2}\\left(x\\right)"
f.equals(me.fromText("1")); // true
me.fromText("x^2").derivative("x").toString(); // "2 x"
```

> **Using this from DoenetML?** See
> [`DOENET_INTEGRATION.md`](../../active-plans/DOENET_INTEGRATION.md) — the behavior changes,
> the known blockers, and the one request still open (wasm32 stack safety).

## wasm handle lifetimes

Every `Expression` this package returns wraps a Rust/wasm handle that owns memory
in the wasm heap. **The wrapper does not free the handles it hands back** — it
frees only its own short-lived internal temporaries (converters, tree-op routing,
`substitute` intermediates). Yours are yours.

They are not leaked outright: wasm-bindgen's generated glue registers each class
with a `FinalizationRegistry`, so a handle no longer reachable from JS is
released once the GC gets to it. But that is non-deterministic and can lag far
behind allocation, which is too late for a long-lived host that mints a handle
per evaluation.

So release them eagerly where it matters:

```ts
const e = me.fromText("x^2");
try {
    /* … */
} finally {
    e.free(); // alias: e.dispose(); also `using e = me.fromText(…)`
}
```

`free()` is idempotent, and a method call on a freed expression raises a
`TypeError` (naming the wasm method it tried to reach) rather than reading
through a dangling pointer.
For scripts and test runs none of this matters — the process exits and reclaims
all wasm memory.

## Layout

- `lib/` — the TypeScript compat layer. `lib/math-expressions.ts` is the entry
  (the `Context`/`me` factory + `Expression`); the other files mirror the old
  `lib/**` module paths (`trees/`, `converters/`, `assumptions/`, `expression/`)
  so unchanged specs that import `../lib/...` resolve here.
- `lib/_wasm.ts` — the swappable wasm provider: the Node fallback loader and the
  `setWasmModule` injection point a browser host calls. The typed surface of the
  wasm module itself lives in `math-expressions-rs-wasm`'s `src-js/wasm.ts`.
- `types/math-expressions.d.ts` — the published type contract (`exports["."]`'s
  `types`). Hand-written; `npm run typecheck` compiles `types/usage.ts`
  against it.
- `vendor/wasm/`, `vendor/wasm-web/` — the generated wasm bindings, one per
  target (git-ignored; build below).
- `scripts/` — `verify-package.mjs` and the two consumer programs it runs.
- `spec/` — the original suite, copied verbatim from `tmp/js-legacy/spec` and
  renamed to `.spec.ts`. These run against this package.

## Build the wasm (required before tests)

```
./build-wasm.sh            # both targets
./build-wasm.sh nodejs     # or just one
```

Two wasm-bindgen packages come out of one `cargo build`, and both are published:

- `vendor/wasm/` — **nodejs** target. Instantiates synchronously at `require()`
  time, so the legacy synchronous API works with no `await`. `lib/_wasm.ts`
  loads it by itself when nothing was injected, which is why Node consumers need
  no setup at all.
- `vendor/wasm-web/` — **web** target. ESM, with an async `default()` and a
  synchronous `initSync()`. A browser or Web Worker cannot use the nodejs build,
  so such a host instantiates this one from bytes and passes it to
  `setWasmModule`. Shipping it is what lets a browser host consume this package
  from npm without cargo — see "Publishing" below.

## Test

```
npm test               # vitest run
npm run typecheck      # tsc over lib/ and over the published declarations
```

The suite is the legacy JS test corpus and it passes: 6,383 tests, 6,372
passing, 11 skipped, nothing failing. One of those skips is a divergence rather
than an unported feature — `slow_assumptions.spec.ts` → `logical combinations`,
on which **legacy commits to answers this engine declines to give**; the engine
is incomplete there, never unsound. It is skipped rather than left red because
the CI job gates, and a permanently red test would make that job unable to
report anything else. Vitest aborts
an `it` at its first failure, so the single failing test name hides **six**
failing assertions (spec lines 7357, 7415, 7417, 7418, 7419, 7420 — count them
by converting that `it`'s `expect` to `expect.soft`), from **two** unrelated
root causes: `Facts::and_meet` declining under contradictory premises where
legacy takes `left || right`, and non-realness never propagating through
`+`/`*`/`^` in `assumptions/infer/combine/mod.rs`. Both are written up in
`../../active-plans/COMPAT_TEST_FAILURE_SUMMARY.md`. Some legacy areas remain unported
(richly-structured `get_assumptions`; the MathML converters are ported, but
`Context.fromMml` is still `notImplemented` and `me.from` does not try MathML as
its third fallback the way legacy's `create_from_multiple` did). See
`../../active-plans/JS_TEST_COVERAGE_AUDIT.md` for the coverage ledger.

`typecheck` covers `lib/**` and `types/**`, not `spec/**`. The specs are the
legacy JS suite renamed to `.spec.ts` and carry thousands of type errors;
typing them is separate work. Keeping `lib/**` at zero is what makes
`math-expressions-rs-wasm`'s `src-js/wasm.ts` an enforced contract rather than
a claimed one.

Nothing is excluded: `vite.config.ts` runs every `spec/**/*.spec.ts`. (This
paragraph used to record `slow_check-symbolic-equality-numerical-errors.spec.ts`
as excluded for hanging on a perturbed exp/log input. It has not been excluded
for some time, and it passes.)

`spec/build_esm.spec.ts` and `spec/build_umd.spec.ts` are the exception to
"the suite tests `lib/`": they load `dist/`, the artifact a consumer installs.
They skip themselves when `dist/` is absent, so run `npm run build` first — or
run them the way CI does, after `npm run build:package`.

## Build the library

```
npm run build          # vite build (ES + UMD)
npm run build:package  # both wasm targets + the rs-wasm bindings + the above
```

`build:package` is what `prepack` runs, so `npm pack` and `npm publish` produce a
complete tarball on their own.

math.js is the one runtime dependency left as a bare import rather than inlined:
it is 95% of the bundle otherwise (1,002 kB inlined against 48 kB external), it
is in `dependencies` so npm resolves it, and a private copy would be a second
`math.create(math.all)` instance as well as dead weight. The UMD build resolves
it through the global `math`, as UMD externals must.

`math-expressions-rs-wasm` is the opposite case: it is a workspace-internal
package that is *not* published, so the build inlines it and it sits in
`devDependencies`. Leaving it in `dependencies` made `npm install` of this
package fail outright with a registry 404.

## Publishing

```
npm run verify:package  # what CI's "package publishability" job runs
```

That packs the tarball, installs it into a throwaway project outside the
workspace, and drives it through both supported loading paths. Inside the
monorepo everything resolves through workspace symlinks and every build output
is already present, so it is the only check that sees what a consumer sees.

What the tarball has to contain, and why:

| | |
| --- | --- |
| `dist/` | the built entry point `main`/`exports` name. Git-ignored, so `prepack` builds it. |
| `types/math-expressions.d.ts` | `exports["."]`'s `types`. Without it every TypeScript consumer sees `any`. |
| `vendor/wasm/` | the Node fallback, loaded by `lib/_wasm.ts` with no host cooperation. |
| `vendor/wasm-web/` | the `--target web` build, exported as `math-expressions/wasm-web/*`. A browser host injects it through `setWasmModule`; without it, consuming this package in a browser still needs cargo. |
| `lib/` | the TypeScript sources, under `exports["./lib/*"]`. Read-only in practice: they import `math-expressions-rs-wasm` by bare specifier, and that package is bundled into `dist/` rather than published, so a consumer compiling `lib/` itself has to alias it. Take `dist/`. |
