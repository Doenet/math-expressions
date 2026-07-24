// AST → math.js converter, backed by the shared bridge in
// `math-expressions-rs-wasm` (the same `TreeToMathjs` the graphing SDK uses).
// The legacy `me.converters.astToMathjsObj` shape is a class with a no-arg
// constructor and a `convert(ast)` method; we adapt to it here, feeding this
// package's configured math.js instance to the shared converter.
import { TreeToMathjs, type Tree } from "math-expressions-rs-wasm";
import math from "../mathjs";

export default class {
  private inner = new TreeToMathjs(math as never);

  convert(ast: Tree) {
    return this.inner.convert(ast);
  }
}
