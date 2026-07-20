// Adapters that expose the JS and Rust (WASM) implementations of
// math-expressions behind one common interface. Every method operates on an
// opaque per-engine expression handle; the UI never touches the underlying
// objects directly.
//
// Ownership: callers free handles deterministically via `engine.free(h)` (a
// no-op for JS, a real wasm free for Rust) — see freeHandle below. Relying on
// FinalizationRegistry GC corrupted the wasm heap under rapid handle churn.

import type {
  Complex,
  Engine,
  Engines,
  JsExpr,
  MathExprContext,
  RustExpr,
  RustModule,
  Tree,
} from "./types";

// JS implementation — the bundled ESM library from the repo's build output. The
// bundle ships no adjacent .d.ts for this relative path (the library's types
// live in build/index.d.ts), so type its default export as our minimal
// MathExprContext — only the members the adapter actually uses.
// @ts-expect-error -- no declaration file alongside the built JS bundle
import ContextUntyped from "../../../build/math-expressions.js";
const Context: MathExprContext = ContextUntyped;

// Rust implementation — the wasm-bindgen glue is static-copied to /wasm/ by
// vite-plugin-static-copy (see vite.config.js) and loaded at runtime by URL, so
// Vite never bundles the wasm. Loading the glue from its served location lets
// it resolve math_expressions_bg.wasm relative to itself.
const RUST_GLUE_URL = `${import.meta.env.BASE_URL}wasm/math_expressions.js`;

/**
 * Free a wasm-bindgen handle, tolerating already-freed / non-wasm values. The
 * `__wbg_ptr === 0` check guards against a double free (wasm-bindgen zeroes the
 * pointer on free), which would otherwise corrupt the wasm heap.
 */
function freeHandle(h: RustExpr): void {
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
function normalizeComplex(r: unknown): Complex | null {
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

const jsEngine: Engine<JsExpr> = {
  name: "JavaScript",
  parse(text, syntax) {
    return syntax === "latex"
      ? Context.fromLatex(text)
      : Context.fromText(text);
  },
  tree: (h) => h.tree,
  toText: (h) => h.toString(),
  toLatex: (h) => h.toLatex(),
  variables: (h) => h.variables(),
  // Substitute the bindings (which may be complex expressions), then evaluate to
  // a complex constant. Returns Complex, or null when it can't be reduced to one
  // (free variables, non-finite) — matching the Rust side's null. JS handles are
  // plain GC'd objects, so no freeing is needed here.
  evaluate(h, subs) {
    try {
      const keys = Object.keys(subs);
      let closed = h;
      if (keys.length > 0) {
        const map: Record<string, JsExpr["tree"]> = {};
        for (const k of keys) map[k] = Context.fromText(subs[k]).tree;
        closed = h.substitute(map);
      }
      return normalizeComplex(closed.evaluate({}));
    } catch {
      return null;
    }
  },
  derivative: (h, v) => h.derivative(v),
  // The JS library has no symbolic integration (only `integrateNumerically`).
  // Throw so the caller can distinguish "unsupported" from "no elementary form".
  integrate() {
    throw new Error("the JS library has no symbolic integration");
  },
  // JS handles are plain GC'd objects — nothing to free.
  free() {},
  // Simplify under `assumptions` (relation strings, e.g. "x > 0"). The JS
  // library reads assumptions from the shared Context, so we set them around
  // the call and clear afterwards to keep them scoped to simplification.
  simplifyWith(h, assumptions) {
    Context.clear_assumptions();
    for (const a of assumptions) {
      try {
        Context.add_assumption(Context.fromText(a).tree);
      } catch {
        /* ignore unparseable assumption */
      }
    }
    try {
      return h.simplify();
    } finally {
      Context.clear_assumptions();
    }
  },
  expand: (h) => h.expand(),
  equals: (a, b) => a.equals(b),
};

function makeRustEngine(rust: RustModule): Engine<RustExpr> {
  return {
    name: "Rust (WASM)",
    parse(text, syntax) {
      return syntax === "latex"
        ? rust.parse_latex(text)
        : rust.parse_text(text);
    },
    // JSON.parse returns `any`; assert the documented Tree shape at this one
    // boundary rather than letting `any` flow into the UI untyped.
    tree: (h) => JSON.parse(h.tree_json()) as Tree,
    toText: (h) => h.to_text(),
    toLatex: (h) => h.to_latex(),
    variables: (h) => Array.from(h.variables()),
    // Substitute the (possibly complex) bindings, then evaluate to a complex
    // constant [re, im]. Returns Complex, or null when it can't be reduced to
    // one (free variables, non-finite). Every intermediate wasm handle created
    // here is freed deterministically (never `h` itself).
    evaluate(h, subs) {
      const temps: RustExpr[] = [];
      try {
        let cur = h;
        for (const [k, v] of Object.entries(subs)) {
          const val = rust.parse_text(v);
          temps.push(val);
          cur = cur.substitute_var(k, val);
          if (cur !== h) temps.push(cur);
        }
        const pair = cur.evaluate_to_complex();
        return normalizeComplex(pair ? Array.from(pair) : null);
      } catch {
        return null;
      } finally {
        for (const t of temps) freeHandle(t);
      }
    },
    derivative: (h, v) => h.derivative(v),
    // Symbolic indefinite integral; the binding returns undefined when no
    // elementary antiderivative is found, which we normalise to null.
    integrate: (h, v) => h.integrate(v) ?? null,
    // Free a wasm-owned handle. Deterministic freeing (rather than relying on
    // FinalizationRegistry GC) keeps the wasm heap bounded under rapid churn.
    free: (h) => freeHandle(h),
    // Assumptions are relation strings; the wasm binding parses them and
    // ignores any that don't parse.
    simplifyWith(h, assumptions) {
      return h.simplify_with_assumptions(assumptions);
    },
    expand: (h) => h.expand(),
    equals: (a, b) => a.equals(b),
  };
}

/** Load both engines. Resolves once the wasm module is initialised. */
export async function loadEngines(): Promise<Engines> {
  const rust = (await import(/* @vite-ignore */ RUST_GLUE_URL)) as RustModule;
  await rust.default(); // wasm-bindgen init()
  return { js: jsEngine, rust: makeRustEngine(rust) };
}
