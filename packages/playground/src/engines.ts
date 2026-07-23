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
import { fromPeriodOutput, toPeriodInput } from "./util";
import type {
  Complex,
  JsExpr,
  Notation,
  RustExpr,
  RustModule,
  Tree,
} from "./types";

// Re-export the JS context so the registry's JS ops (assumptions, expression-typed
// args) can reach the factory methods directly.
export { Context };

// The Rust wasm-bindgen glue is static-copied to /wasm/ by vite-plugin-static-copy
// (see vite.config.ts) and loaded at runtime by URL, so Vite never bundles the
// wasm. Loading from the served location lets the glue resolve its .wasm sibling.
const RUST_GLUE_URL = `${import.meta.env.BASE_URL}wasm/math_expressions_wasm.js`;

// The wasm-bindgen type declarations, static-copied alongside the glue. Fetched
// (not imported) so the palette can reflect the live API surface at runtime —
// see wasmApi.ts. Regenerated on every `build:wasm`, so it never drifts.
const RUST_DTS_URL = `${import.meta.env.BASE_URL}wasm/math_expressions_wasm.d.ts`;

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
 * Guard LaTeX before parsing. BOTH upstream parsers (the JS library and the
 * Rust port) infinite-loop on an opened-but-unclosed environment — e.g.
 * `\begin{bmatrix}` with no matching `\end{bmatrix}` — which, on the main
 * thread, freezes the whole playground. Reject unbalanced `\begin`/`\end` up
 * front so the UI shows an error instead of hanging. Returns `s` for chaining.
 */
function guardLatex(s: string): string {
  const begins = (s.match(/\\begin\b/g) ?? []).length;
  const ends = (s.match(/\\end\b/g) ?? []).length;
  if (begins > ends)
    throw new Error(
      "unclosed \\begin{…} environment — add a matching \\end{…}",
    );
  return s;
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

/** The Rust `parse_*_with_options` JSON for a notation, or null for the default. */
function rustNotationJson(notation: Notation): string | null {
  return notation === "comma"
    ? JSON.stringify({
        notation: { decimalSeparator: ",", argumentSeparator: ";" },
      })
    : null; // period is the wasm default — use the plain parse_* for byte-identity
}

/**
 * The JS engine adapter for a notation. The JS library only understands period
 * notation, so under `comma` the adapter transliterates input on the way in and
 * output on the way out. Handles are plain GC'd objects, so `free` is a no-op.
 */
export function makeJsAdapter(notation: Notation): EngineAdapter<JsExpr> {
  const parse = (s: string) => Context.fromText(toPeriodInput(s, notation));
  return {
    tag: "js",
    fromText: parse,
    fromLatex: (s) => Context.fromLatex(toPeriodInput(guardLatex(s), notation)),
    fromAst: (tree) =>
      Context.fromAst(tree as Parameters<typeof Context.fromAst>[0]),
    parseExpr: parse,
    toText: (h) => fromPeriodOutput(h.toString(), notation),
    toLatex: (h) => fromPeriodOutput(h.toLatex(), notation, true),
    treeOf: (h) => h.tree as Tree,
    free: () => {},
  };
}

/** Back-compat default (period) JS adapter. */
export const jsAdapter = makeJsAdapter("period");

/**
 * Build the Rust engine adapter for a notation. Rust parses natively via
 * `parse_*_with_options`; the notation is carried on the handle, so output
 * (`to_text`/`to_latex`) and derived handles honor it automatically.
 */
export function makeRustAdapter(
  rust: RustModule,
  notation: Notation,
): EngineAdapter<RustExpr> {
  const opts = rustNotationJson(notation);
  const parse = (s: string) =>
    opts ? rust.parse_text_with_options(s, opts) : rust.parse_text(s);
  return {
    tag: "rust",
    fromText: parse,
    fromLatex: (s) =>
      opts
        ? rust.parse_latex_with_options(guardLatex(s), opts)
        : rust.parse_latex(guardLatex(s)),
    fromAst: (tree) => rust.from_ast(JSON.stringify(tree)),
    parseExpr: parse,
    toText: (h) => h.to_text(),
    toLatex: (h) => h.to_latex(),
    // JSON.parse returns `any`; assert the documented Tree shape at this boundary.
    treeOf: (h) => JSON.parse(h.tree_json()) as Tree,
    free: (h) => freeHandle(h),
  };
}

/** The loaded wasm module; adapters are built per-notation by the caller. */
export interface LoadedEngines {
  rustModule: RustModule;
  /** Raw `math_expressions_wasm.d.ts` text, or "" if it could not be fetched. */
  wasmDts: string;
}

/** Load the wasm module and its type declarations. Resolves once initialised. */
export async function loadEngines(): Promise<LoadedEngines> {
  const [rust, wasmDts] = await Promise.all([
    (async () => {
      const r = (await import(/* @vite-ignore */ RUST_GLUE_URL)) as RustModule;
      await r.default(); // wasm-bindgen init()
      return r;
    })(),
    // A missing/older .d.ts just means an empty "Other" section — never fatal.
    fetch(RUST_DTS_URL)
      .then((r) => (r.ok ? r.text() : ""))
      .catch(() => ""),
  ]);
  return { rustModule: rust, wasmDts };
}
