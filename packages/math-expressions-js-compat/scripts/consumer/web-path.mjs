/**
 * Consumer smoke test #2: the browser / Web Worker path, with no Rust toolchain.
 *
 * This is the one that decides whether publishing this package lets a host such
 * as DoenetML drop its `vendor/math-expressions` submodule *and* its cargo +
 * wasm-bindgen requirement, rather than only the submodule. It therefore uses
 * nothing but what the tarball ships:
 *
 *   1. resolve the `--target web` wasm through the package's `./wasm-web/*`
 *      export — the nodejs-target build under `vendor/wasm` cannot be used off
 *      Node, and building a web one needs cargo;
 *   2. instantiate it from *bytes*, never from a URL. `fetch` of a blob/data
 *      URL is blocked in the VS Code web-worker extension host, so hosts inline
 *      the binary; reading the file here is the same shape;
 *   3. hand the initialized module to `setWasmModule` before parsing anything.
 *
 * Node stands in for the worker: it runs the same ESM glue and the same
 * `initSync`. What it cannot check is that the browser main thread refuses a
 * synchronous compile this large — that is the host's problem, and DoenetML's
 * `wasm-loader.ts` handles it with an async init.
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import * as glue from "math-expressions/wasm-web/math_expressions_wasm.js";
import me, { setWasmModule } from "math-expressions";

const bytes = readFileSync(
  fileURLToPath(
    import.meta
      .resolve("math-expressions/wasm-web/math_expressions_wasm_bg.wasm"),
  ),
);
assert.ok(bytes.byteLength > 1_000_000, "the web wasm binary looks truncated");
// The wasm magic number, so a text file renamed `.wasm` fails here rather than
// inside the instantiate call.
assert.deepEqual([...bytes.subarray(0, 4)], [0x00, 0x61, 0x73, 0x6d]);

glue.initSync({ module: bytes });
setWasmModule(glue);

assert.equal(me.fromText("x^2 + 2x + 1").toString(), "x^2 + 2 x + 1");
assert.equal(me.fromText("sin^2 x + cos^2 x").equals(me.fromText("1")), true);
assert.equal(me.fromAst(["*", 0.5, 2, "x"]).simplify().toLatex(), "x");

console.log("web path: ok");
