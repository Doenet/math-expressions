// Tree-level normalization ops (`simplify.simplify(tree)`, `.expand(tree)`, …),
// implemented by routing the raw tree through the wasm Expression and back.
// Ops with no Rust backing are omitted (calls throw a TypeError → test fails,
// suite runs).
import wasm from "../_wasm";
import { get_tree } from "../trees/util";
import { astToJson, jsonToAst } from "../converters/ast-json";

function op(method) {
  return (tree) => {
    // Legacy ops accepted an expression-or-tree; unwrap an Expression to its
    // AST. Tag non-finite numbers so `from_ast` accepts NaN/±Infinity.
    tree = get_tree(tree);
    const src = wasm.from_ast(astToJson(tree));
    try {
      const out = src[method]();
      try {
        return jsonToAst(out.tree_json());
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

export default {
  simplify,
  expand,
  evaluate_numbers,
  collect_like_terms_and_factors,
};
