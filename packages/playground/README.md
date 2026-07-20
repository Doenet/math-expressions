# math-expressions playground

A Vite + React web playground for **comparing the Rust (WASM) and JavaScript
implementations** of math-expressions side by side. Type a math expression and
see, for each engine:

- the **parse tree** (rendered as an indented AST, plus `toText` / `toLatex`),
- the **simplified form**, optionally under **assumptions** (e.g. `x > 0`,
  `x elementof R`) — `sqrt(x^2)` simplifies to `x`, `|x|`, or stays put
  depending on what's assumed,
- the **evaluated value** — substitutions are entered as expressions, so
  bindings and results may be **complex** (e.g. `x = i`, or `sqrt(-4)` → `2i`),
- the **symbolic derivative** with respect to a chosen variable, and its value
  at the current substitutions.

Agreement badges flag where the two implementations produce identical parse
trees or matching numeric results — handy for spotting porting gaps.

## Running

```bash
cd playground
npm install
npm run dev      # http://localhost:5173
```

`npm run build` produces a static bundle in `dist/`.

## Build graph (wireit)

The scripts are managed by [wireit](https://github.com/google/wireit), so the
Rust → wasm step is a tracked dependency of the app build rather than a manual
step you have to remember:

| Script               | Runs                   | Depends on                |
| -------------------- | ---------------------- | ------------------------- |
| `npm run dev`        | `vite` (dev server)    | `build:wasm`              |
| `npm run build`      | `vite build` → `dist/` | `build:wasm`, `typecheck` |
| `npm run build:wasm` | `rebuild-wasm.sh`      | —                         |
| `npm run typecheck`  | `tsc --noEmit`         | —                         |
| `npm run preview`    | `vite preview`         | `build`                   |

`build:wasm` is fingerprinted on the Rust sources (`../math-expressions-rs/src/**`,
`Cargo.toml`), so it only recompiles when they actually change. `dev`/`build`
run it first automatically, so **you no longer rebuild the wasm by hand** after
editing the Rust crate; just re-run `npm run dev`.

This is a TypeScript project. Vite/esbuild strips types for dev and build (it
does not type-check), so `npm run build` also runs `typecheck` (`tsc --noEmit`)
as a gate — a type error fails the build. Shared types live in
[`src/types.ts`](src/types.ts); the external JS library and the wasm glue are
typed by the minimal interfaces there (only the members the adapter uses).

`build:wasm` requires `rustup target add wasm32-unknown-unknown` and a
`wasm-bindgen` CLI matching the `wasm-bindgen` crate version in
`math-expressions-rs/Cargo.toml`.

## How the two engines are wired

- **JavaScript**: imported from the repo's built bundle
  `../build/math-expressions.js` (run `npm run build` at the repo root to
  regenerate it — it is also a tracked input of the app build). The dev server
  is configured with `server.fs.allow: ['..']` so it can serve that sibling
  file.
- **Rust**: `build:wasm` compiles the crate to a browser-targeted wasm-bindgen
  package in **`../math-expressions-rs/pkg/`** (not vendored into this project).
  `vite.config.ts` resolves that location and uses
  [`vite-plugin-static-copy`](https://github.com/sapphi-red/vite-plugin-static-copy)
  to serve `math_expressions.js` + `math_expressions_bg.wasm` under `/wasm/`.
  `src/engines.ts` then loads the glue at runtime via a dynamic `import()` of
  that URL, so Vite never bundles the wasm — the static copy is its sole
  delivery, and the glue resolves the `.wasm` relative to its own served URL.

Both engines are hidden behind one adapter interface in
[`src/engines.ts`](src/engines.ts).

## Notes

- Two bindings were added to `math-expressions-rs/src/wasm.rs` for the
  playground: `Expression.tree_json()` (serialises to the same JS `Tree` JSON
  shape the JS library uses, so the two trees are directly comparable) and
  `Expression.evaluate_to_complex()` (returns `[re, im]`, so complex-valued
  results are kept instead of discarded), and
  `Expression.simplify_with_assumptions([...])` (assumption-aware simplify).
- `../math-expressions-rs/pkg/` is generated build output (wasm-bindgen) and is
  git-ignored — `build:wasm` produces it. Don't edit or commit it by hand. If
  you delete it without also clearing `.wireit`, run `npm run build:wasm` to
  regenerate it.
