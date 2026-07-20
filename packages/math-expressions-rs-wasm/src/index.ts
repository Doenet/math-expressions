/*
 * math-expressions-rs-wasm — TypeScript bindings adapting the math-expressions
 * Rust/WASM port for JS consumers.
 *
 * The core export is the AST → math.js bridge (`compileRustExpr` /
 * `rustExprToMathNode`) used to numerically evaluate expressions — the path
 * Doenet drives jsxgraph through. See ./tree-to-mathjs.ts for the design notes.
 */

export {
  TreeToMathjs,
  factorialToGamma,
  treeToMathNode,
  compileTree,
  rustExprToMathNode,
  compileRustExpr,
} from "./tree-to-mathjs";
export type { Tree, TreeArray, Scope, RustExprLike } from "./tree-to-mathjs";

export { default } from "./tree-to-mathjs";

export type { RustExpression, MathExpressionsWasmModule } from "./wasm";
