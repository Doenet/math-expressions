// Adapters that expose the JS and Rust (WASM) implementations of
// math-expressions behind one common interface. Every method operates on an
// opaque per-engine expression handle; the UI never touches the underlying
// objects directly.
//
// Both handles are cheap to create and are garbage-collected (the wasm handles
// via FinalizationRegistry), so we don't manually free them in this playground.

// JS implementation — the bundled ESM library from the repo's build output.
import Context from "../../build/math-expressions.js";

// Rust implementation — the wasm-bindgen glue is static-copied to /wasm/ by
// vite-plugin-static-copy (see vite.config.js) and loaded at runtime by URL, so
// Vite never bundles the wasm. Loading the glue from its served location lets
// it resolve math_expressions_bg.wasm relative to itself.
const RUST_GLUE_URL = `${import.meta.env.BASE_URL}wasm/math_expressions.js`;

/**
 * Normalise an evaluation result to `{ re, im } | null`. Accepts a real number,
 * a mathjs `Complex` (`{ re, im }`), or a `[re, im]` pair (the Rust binding).
 */
function normalizeComplex(r) {
  if (r === undefined || r === null) return null;
  if (typeof r === "number")
    return Number.isFinite(r) ? { re: r, im: 0 } : null;
  if (Array.isArray(r) && r.length === 2) {
    return Number.isFinite(r[0]) && Number.isFinite(r[1])
      ? { re: r[0], im: r[1] }
      : null;
  }
  if (typeof r === "object" && "re" in r && "im" in r) {
    return Number.isFinite(r.re) && Number.isFinite(r.im)
      ? { re: r.re, im: r.im }
      : null;
  }
  return null;
}

const jsEngine = {
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
  substitute(h, subs) {
    // subs: { varName: expressionString } — values may be complex, e.g. "2+3i".
    const keys = Object.keys(subs);
    if (keys.length === 0) return h;
    const map = {};
    for (const k of keys) map[k] = Context.fromText(String(subs[k])).tree;
    return h.substitute(map);
  },
  // Substitute the bindings (which may be complex expressions), then evaluate to
  // a complex constant. Returns { re, im }, or null when it can't be reduced to
  // one (free variables, non-finite) — matching the Rust side's null.
  evaluate(h, subs) {
    try {
      return normalizeComplex(this.substitute(h, subs).evaluate({}));
    } catch {
      return null;
    }
  },
  derivative: (h, v) => h.derivative(v),
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

function makeRustEngine(rust) {
  return {
    name: "Rust (WASM)",
    parse(text, syntax) {
      return syntax === "latex"
        ? rust.parse_latex(text)
        : rust.parse_text(text);
    },
    tree: (h) => JSON.parse(h.tree_json()),
    toText: (h) => h.to_text(),
    toLatex: (h) => h.to_latex(),
    variables: (h) => Array.from(h.variables()),
    substitute(h, subs) {
      let cur = h;
      for (const [k, v] of Object.entries(subs)) {
        cur = cur.substitute_var(k, rust.parse_text(String(v)));
      }
      return cur;
    },
    // Substitute the (possibly complex) bindings, then evaluate to a complex
    // constant [re, im]. Returns { re, im }, or null when it can't be reduced
    // to one (free variables, non-finite).
    evaluate(h, subs) {
      try {
        const pair = this.substitute(h, subs).evaluate_to_complex();
        return normalizeComplex(pair ? Array.from(pair) : null);
      } catch {
        return null;
      }
    },
    derivative: (h, v) => h.derivative(v),
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
export async function loadEngines() {
  const rust = await import(/* @vite-ignore */ RUST_GLUE_URL);
  await rust.default(); // wasm-bindgen init()
  return { js: jsEngine, rust: makeRustEngine(rust) };
}
