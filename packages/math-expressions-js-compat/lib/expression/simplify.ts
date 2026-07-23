// Tree-level normalization ops (`simplify.simplify(tree)`, `.expand(tree)`, …),
// implemented by routing the raw tree through the wasm Expression and back.
// Ops with no Rust backing are omitted (calls throw a TypeError → test fails,
// suite runs).
import wasm from "../_wasm";

function op(method) {
  return (tree) => {
    const src = wasm.from_ast(JSON.stringify(tree));
    try {
      const out = src[method]();
      try {
        return JSON.parse(out.tree_json());
      } finally {
        out.free(); // throwaway: method result, never returned
      }
    } finally {
      src.free(); // throwaway: parse source, never returned
    }
  };
}

export const simplify = op("simplify");
export const expand = op("expand");
export const evaluate_numbers = op("evaluate_numbers");
export const collect_like_terms_and_factors = op("collect_like_terms_factors");
export const factor = op("factor");
export const together = op("together");

export default { simplify, expand, evaluate_numbers, collect_like_terms_and_factors };
