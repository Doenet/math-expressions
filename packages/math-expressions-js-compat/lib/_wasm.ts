// Synchronous loader for the Rust core's nodejs-target wasm bindings.
//
// `build-wasm.sh` emits a CommonJS wasm-bindgen package into ../vendor/wasm that
// instantiates the wasm at require() time — no async init — so the original
// synchronous math-expressions API (`me.fromText(x).equals(y)`, no await) works
// as-is under Node / Vitest. We reach it through createRequire so the raw CJS
// module (and its `require('fs')` wasm read) bypasses the bundler transform.
//
// Browser builds would instead use a --target web wasm + async init; that path
// is future work (see README).
import { createRequire } from "node:module";
import type { WasmModule } from "math-expressions-rs-wasm";

const require = createRequire(import.meta.url);
const wasm = require("../vendor/wasm/math_expressions_wasm.js") as WasmModule;

export default wasm;
