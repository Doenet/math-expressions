// Swappable provider for the Rust core's wasm bindings.
//
// Two hosts, one seam:
//
//   • Node / Vitest — the default. The vendored *nodejs-target* build
//     (`build-wasm.sh` → ../vendor/wasm) instantiates the wasm synchronously at
//     require() time, so the synchronous math-expressions API
//     (`me.fromText(x).equals(y)`, no await) works with no init step. Loaded
//     lazily on first use through `createRequire`, so the raw CJS module (and
//     its `require('fs')` wasm read) bypasses the bundler transform.
//
//   • Browser / Web Worker — call {@link setWasmModule} with an already-
//     initialized `--target web` module (after its `initSync(bytes)`) *before*
//     the first parse. The `--target web` build must be instantiated from bytes
//     (no `fetch`: the VS Code web-worker host blocks blob/data-URL fetch); the
//     inlining + `initSync` glue is the host's, per DOENET_COMPAT_PLAN R1. Once
//     injected, the node fallback below is never reached.
//
// This file deliberately has **no import of `node:module`**. A static import
// would be evaluated by a browser bundle even when `setWasmModule` is called
// first — marking the specifier external does not help, since the browser then
// has to resolve `node:module` at run time and cannot. Reaching the builtin
// through `process.getBuiltinModule` instead leaves nothing for a bundler to
// resolve: browser builds see only a `globalThis.process` probe, and no host-
// specific aliasing or stubbing is required to bundle this package.
//
// Everything downstream imports the default export and calls `wasm.parse_text(…)`
// etc.; the Proxy forwards each access to whichever module is current, so an
// injection that happens after this module is imported is still honored.
import type { WasmModule } from "math-expressions-rs-wasm";

/** `process.getBuiltinModule` — Node ≥ 20.16 / ≥ 22.3, absent in browsers. */
type BuiltinModuleHost = {
  getBuiltinModule?: (id: string) => {
    createRequire(path: string): (id: string) => unknown;
  };
};

let injected: WasmModule | undefined;
let nodeFallback: WasmModule | undefined;
const swapListeners: Array<() => void> = [];

/**
 * Register a callback to run when {@link setWasmModule} swaps in a different
 * module.
 *
 * Anything that caches a wasm *handle* has to drop it here: a handle belongs to
 * the module that minted it, and handing one to a different module's function
 * fails with "expected instance of Expression". The listener seam (rather than
 * this file reaching into the caches) keeps the dependency one-way —
 * `math-expressions.ts` imports `_wasm`, never the reverse.
 */
export function onWasmModuleChange(fn: () => void): void {
  swapListeners.push(fn);
}

/**
 * Inject the wasm module to use — an initialized `--target web` wasm-bindgen
 * module (post-`initSync`) for browser/worker hosts where the synchronous node
 * loader is unavailable. Call once, before any parsing. Overrides the node
 * fallback for every subsequent call.
 */
export function setWasmModule(mod: WasmModule): void {
  const changed = injected !== mod;
  injected = mod;
  // Re-injecting the *same* module is a no-op, so caches keep their entries.
  if (changed) {
    for (const fn of swapListeners) fn();
  }
}

/**
 * Load the vendored nodejs-target build. Deferred so a browser host that
 * injects first never reaches it, and so the `process` probe never runs where
 * there is no `process`.
 */
function loadNodeFallback(): WasmModule {
  const proc = (globalThis as { process?: BuiltinModuleHost }).process;
  const mod = proc?.getBuiltinModule?.("node:module");
  if (!mod) {
    throw new Error(
      "math-expressions: no wasm module available. Outside Node, call " +
        "`setWasmModule(mod)` with an initialized `--target web` wasm-bindgen " +
        "module (after its `initSync(bytes)`) before parsing anything.",
    );
  }
  const req = mod.createRequire(import.meta.url);
  return req("../vendor/wasm/math_expressions_wasm.js") as WasmModule;
}

/** The module currently in effect: an injected one, else the node vendored build. */
function current(): WasmModule {
  if (injected) return injected;
  if (!nodeFallback) {
    nodeFallback = loadNodeFallback();
  }
  return nodeFallback;
}

// A Proxy so `import wasm from "./_wasm"` stays a stable reference that always
// reflects the current module (injected or node), resolved on first access.
const wasm = new Proxy({} as WasmModule, {
  get: (_target, prop) => {
    const mod = current();
    return Reflect.get(mod as object, prop, mod);
  },
  has: (_target, prop) => prop in (current() as object),
});

export default wasm;
