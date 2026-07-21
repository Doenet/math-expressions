// Engine glue: the canonical JS library and the Rust (WASM) port, exposed to the
// chain evaluator behind a minimal `EngineAdapter` (source factories + display
// extraction + freeing) and to the operation registry as the raw `Context` /
// `RustModule` surfaces.
//
// Ownership: the Rust engine's handles are wasm-owned and freed deterministically
// via `freeHandle` — relying on FinalizationRegistry GC corrupted the wasm heap
// under rapid handle churn. The JS engine's handles are plain GC'd objects, so
// its `free` is a no-op.

import Context from "math-expressions-canonical";
import type { Complex, JsExpr, RustExpr, RustModule, Tree } from "./types";

// Re-export the JS context so the registry's JS ops (assumptions, expression-typed
// args) can reach the factory methods directly.
export { Context };

// The Rust wasm-bindgen glue is static-copied to /wasm/ by vite-plugin-static-copy
// (see vite.config.ts) and loaded at runtime by URL, so Vite never bundles the
// wasm. Loading from the served location lets the glue resolve its .wasm sibling.
const RUST_GLUE_URL = `${import.meta.env.BASE_URL}wasm/math_expressions_wasm.js`;

/** A minimal per-engine adapter used by the evaluator for sourcing + display. */
export interface EngineAdapter<H> {
  tag: "js" | "rust";
  fromText(s: string): H;
  fromLatex(s: string): H;
  fromAst(tree: Tree): H;
  /** Parse an expression string as text syntax (for expression-typed args). */
  parseExpr(s: string): H;
  toText(h: H): string;
  toLatex(h: H): string;
  treeOf(h: H): Tree;
  free(h: H): void;
}

/**
 * Free a wasm-bindgen handle, tolerating already-freed / non-wasm values. The
 * `__wbg_ptr === 0` check guards against a double free (wasm-bindgen zeroes the
 * pointer on free), which would otherwise corrupt the wasm heap.
 */
export function freeHandle(h: RustExpr): void {
  try {
    if (h && typeof h.free === "function" && h.__wbg_ptr !== 0) h.free();
  } catch {
    /* not a wasm handle, or already freed */
  }
}

/**
 * Normalise an evaluation result to `Complex | null`. Accepts a real number, a
 * mathjs `Complex` (`{ re, im }`), or a `[re, im]` pair (the Rust binding).
 */
export function normalizeComplex(r: unknown): Complex | null {
  if (r === undefined || r === null) return null;
  if (typeof r === "number")
    return Number.isFinite(r) ? { re: r, im: 0 } : null;
  if (Array.isArray(r) && r.length === 2) {
    const [re, im] = r as [number, number];
    return Number.isFinite(re) && Number.isFinite(im) ? { re, im } : null;
  }
  if (typeof r === "object" && "re" in r && "im" in r) {
    const { re, im } = r as Complex;
    return Number.isFinite(re) && Number.isFinite(im) ? { re, im } : null;
  }
  return null;
}

/** The JS engine adapter — plain GC'd handles, so `free` is a no-op. */
export const jsAdapter: EngineAdapter<JsExpr> = {
  tag: "js",
  fromText: (s) => Context.fromText(s),
  fromLatex: (s) => Context.fromLatex(s),
  fromAst: (tree) => Context.fromAst(tree as Parameters<typeof Context.fromAst>[0]),
  parseExpr: (s) => Context.fromText(s),
  toText: (h) => h.toString(),
  toLatex: (h) => h.toLatex(),
  treeOf: (h) => h.tree as Tree,
  free: () => {},
};

/** Build the Rust engine adapter over a loaded wasm module. */
export function makeRustAdapter(rust: RustModule): EngineAdapter<RustExpr> {
  return {
    tag: "rust",
    fromText: (s) => rust.parse_text(s),
    fromLatex: (s) => rust.parse_latex(s),
    fromAst: (tree) => rust.from_ast(JSON.stringify(tree)),
    parseExpr: (s) => rust.parse_text(s),
    toText: (h) => h.to_text(),
    toLatex: (h) => h.to_latex(),
    // JSON.parse returns `any`; assert the documented Tree shape at this boundary.
    treeOf: (h) => JSON.parse(h.tree_json()) as Tree,
    free: (h) => freeHandle(h),
  };
}

export interface Engines {
  js: EngineAdapter<JsExpr>;
  rust: EngineAdapter<RustExpr>;
  rustModule: RustModule;
}

/** Load both engines. Resolves once the wasm module is initialised. */
export async function loadEngines(): Promise<Engines> {
  const rust = (await import(/* @vite-ignore */ RUST_GLUE_URL)) as RustModule;
  await rust.default(); // wasm-bindgen init()
  return { js: jsAdapter, rust: makeRustAdapter(rust), rustModule: rust };
}
