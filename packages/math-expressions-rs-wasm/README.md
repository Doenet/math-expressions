# math-expressions-rs-wasm

TypeScript bindings that adapt the [`math-expressions`](../math-expressions-rs)
Rust/WASM port for JavaScript consumers — principally **Doenet**, which plots
via **jsxgraph**.

## What's here

The load-bearing piece is the **AST → math.js bridge**. The Rust port owns the
AST and its normalization; this package owns only the construction of math.js
nodes (which are JavaScript objects that must be built in JS). A consumer
compiles an expression **once** and then calls `.evaluate(scope)` per sample in
a tight loop that stays entirely in JS — no JS↔WASM boundary crossing per point,
which is what makes numeric graphing fast.

This is "option 1" of the porting analysis: keep AST → math.js in JS/TS, fed the
already-normalized Rust AST, rather than reimplementing math.js in Rust.

## Layout

- `src/tree-to-mathjs.ts` — `Tree` (the JSON AST from `expr.tree_json()`) →
  math.js `MathNode`, plus `factorial → gamma`, `compileTree`, and the
  `rustExprToMathNode` / `compileRustExpr` bridge (normalization done Rust-side
  via `normalize_function_names`).
- `src/wasm.ts` — structural types for the wasm module surface. The
  authoritative types are the generated `pkg/math_expressions.d.ts`; these are a
  minimal, build-independent subset.
- `src/index.ts` — public entry point.

## Usage

```ts
import { create, all } from "mathjs";
import { compileRustExpr } from "math-expressions-rs-wasm";
// `expr` is a parsed handle from the wasm module (parse_text / parse_latex)

const math = create(all, {});
const compiled = compileRustExpr(math, expr); // normalizes Rust-side, builds node
const y = compiled.evaluate({ x: 3.2 });      // call this per sample (jsxgraph)
```

math.js is a **peer dependency** — pass in your own configured instance so it
isn't double-bundled.
