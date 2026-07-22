/*
 * Convert the `Tree` AST (the JSON shape emitted by both the JS library's
 * `expr.tree` and the Rust/WASM port's `expr.tree_json()`) into a math.js
 * expression node that can be numerically evaluated.
 *
 * This is the TypeScript implementation of "option 1" for the Rust port: the
 * AST → math.js conversion stays in JS/TS (math.js nodes are JavaScript
 * objects that only JS can construct), fed the already-normalized AST produced
 * by Rust. Downstream consumers — notably Doenet, which plots via jsxgraph —
 * compile the resulting node once and then call `.evaluate(scope)` in a tight
 * per-sample loop entirely in JS, with no JS↔WASM boundary crossing per point.
 *
 * Ported from `lib/converters/ast-to-mathjs.js` (same author lineage) with
 * strict type annotations throughout.
 *
 * Copyright 2014-2017 by
 * Jim Fowler <kisonecat@gmail.com>
 * Duane Nykamp <nykamp@umn.edu>
 *
 * This file is part of a math-expressions library
 *
 * math-expressions is free software: you can redistribute
 * it and/or modify it under the terms of the GNU General Public
 * License as published by the Free Software Foundation, either
 * version 3 of the License, or at your option any later version.
 *
 * math-expressions is distributed in the hope that it
 * will be useful, but WITHOUT ANY WARRANTY; without even the implied
 * warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details.
 */

import type { EvalFunction, MathJsInstance, MathNode } from "mathjs";

// ---------------------------------------------------------------------------
// AST shape
// ---------------------------------------------------------------------------

/**
 * The math-expressions AST. Mirrors the public `Tree` type in `index.d.ts`;
 * re-declared here (as `packages/playground/src/types.ts` also does) so this
 * module is self-contained and can be vendored on its own.
 */
export type Tree = number | string | boolean | TreeArray;

/** An operator node: a tag string followed by zero or more operand subtrees. */
export type TreeArray = [string, ...Tree[]];

/** A scope of variable bindings passed to a compiled expression's `evaluate`. */
export type Scope = Record<string, unknown>;

// ---------------------------------------------------------------------------
// Small typed helpers
// ---------------------------------------------------------------------------

/** Narrow a `Tree` to a `TreeArray`, throwing on anything else. */
function asTreeArray(tree: Tree, message = "Badly formed ast"): TreeArray {
  if (!Array.isArray(tree)) throw new Error(message);
  return tree;
}

/** True when `node` is a math.js `ArrayNode` (avoids `instanceof` on the ctor). */
function isArrayNode(node: MathNode): boolean {
  return (node as { isArrayNode?: boolean }).isArrayNode === true;
}

/**
 * Construct a math.js `OperatorNode`. The mathjs type declares the ctor with
 * string-literal generics for `op`/`fn`; we build operator nodes dynamically
 * from AST tags, so the ctor is widened at this single, contained boundary.
 */
function operatorNode(
  math: MathJsInstance,
  op: string,
  fn: string,
  args: MathNode[],
): MathNode {
  const Ctor = math.OperatorNode as unknown as new (
    op: string,
    fn: string,
    args: MathNode[],
  ) => MathNode;
  return new Ctor(op, fn, args);
}

/** Fold a list of comparison nodes into a left-associated chain of `and`s. */
function conjoin(math: MathJsInstance, comparisons: MathNode[]): MathNode {
  if (comparisons.length === 0) throw new Error("Badly formed ast");
  let result =
    comparisons.length === 1
      ? comparisons[0]
      : operatorNode(math, "and", "and", comparisons.slice(0, 2));
  for (let i = 2; i < comparisons.length; i++) {
    result = operatorNode(math, "and", "and", [result, comparisons[i]]);
  }
  return result;
}

/** AST function names that map onto a different math.js function name. */
const functionConversions: Record<string, string> = {
  nCr: "combinations",
  nPr: "permutations",
  binom: "combinations",
};

// ---------------------------------------------------------------------------
// Converter
// ---------------------------------------------------------------------------

/**
 * Converts a `Tree` into a math.js `MathNode`. Construct once with a configured
 * math.js instance (dependency-injected so callers reuse their own instance
 * rather than double-bundling math.js), then call `convert` per expression.
 */
export class TreeToMathjs {
  private readonly math: MathJsInstance;

  constructor(math: MathJsInstance) {
    this.math = math;
  }

  convert(tree: Tree): MathNode {
    const math = this.math;

    if (typeof tree === "number") {
      if (Number.isNaN(tree)) return new math.SymbolNode("NaN");
      if (Number.isFinite(tree)) return new math.ConstantNode(tree);
      if (tree < 0)
        return operatorNode(math, "-", "unaryMinus", [
          new math.SymbolNode("Infinity"),
        ]);
      return new math.SymbolNode("Infinity");
    }

    if (typeof tree === "string") {
      return new math.SymbolNode(tree);
    }

    if (typeof tree === "boolean") throw new Error("no support for boolean");

    if (!Array.isArray(tree)) throw new Error("Invalid ast");

    const operator = tree[0];
    const operands: Tree[] = tree.slice(1);

    switch (operator) {
      case "apply":
        return this.convertApply(operands);
      case "lts":
      case "gts":
        return this.convertInequalityChain(operator, operands);
      case "=":
        return this.convertEquality(operands);
      case "in":
      case "notin":
      case "ni":
      case "notni":
        return this.convertMembership(operator, operands);
      case "subset":
      case "notsubset":
      case "superset":
      case "notsuperset":
        return this.convertContainment(operator, operands);
      case "matrix":
        return this.convertMatrix(operands);
      case "array":
        return new math.ArrayNode(operands.map((e) => this.convert(e)));
      default:
        return this.convertSimpleOperator(operator, operands);
    }
  }

  // -- apply --------------------------------------------------------------

  private convertApply(operands: Tree[]): MathNode {
    const math = this.math;
    const name = operands[0];
    if (typeof name !== "string")
      throw new Error(
        "Non string functions not implemented for conversion to mathjs",
      );

    if (name === "factorial")
      return operatorNode(math, "!", "factorial", [this.convert(operands[1])]);

    const functionWord = functionConversions[name] ?? name;
    const f = new math.SymbolNode(functionWord);

    const args = operands[1];
    let f_args: MathNode[];
    if (Array.isArray(args) && args[0] === "tuple") {
      f_args = args.slice(1).map((v) => this.convert(v));
    } else {
      f_args = [this.convert(args)];
    }

    if (name === "count") {
      if (f_args.length > 2 || !isArrayNode(f_args[0])) {
        // A `count` whose argument isn't a single array: wrap the args in one.
        f_args = [new math.ArrayNode(f_args)];
      }
    }

    return new math.FunctionNode(f, f_args);
  }

  // -- chained strict/non-strict inequalities (`lts` / `gts`) --------------

  private convertInequalityChain(operator: string, operands: Tree[]): MathNode {
    const math = this.math;
    const args = asTreeArray(operands[0]);
    const strict = asTreeArray(operands[1]);

    if (args[0] !== "tuple" || strict[0] !== "tuple")
      throw new Error("Badly formed ast");

    const argNodes = args.slice(1).map((v) => this.convert(v));

    const comparisons: MathNode[] = [];
    for (let i = 1; i < args.length - 1; i++) {
      const pair = argNodes.slice(i - 1, i + 1);
      const isStrict = Boolean(strict[i]);
      if (operator === "lts") {
        comparisons.push(
          isStrict
            ? operatorNode(math, "<", "smaller", pair)
            : operatorNode(math, "<=", "smallerEq", pair),
        );
      } else {
        comparisons.push(
          isStrict
            ? operatorNode(math, ">", "larger", pair)
            : operatorNode(math, ">=", "largerEq", pair),
        );
      }
    }
    return conjoin(math, comparisons);
  }

  // -- chained equality (`=`) ---------------------------------------------

  private convertEquality(operands: Tree[]): MathNode {
    const math = this.math;
    const argNodes = operands.map((v) => this.convert(v));

    const comparisons: MathNode[] = [];
    for (let i = 1; i < argNodes.length; i++) {
      comparisons.push(
        operatorNode(math, "==", "equal", argNodes.slice(i - 1, i + 1)),
      );
    }
    return conjoin(math, comparisons);
  }

  // -- interval membership (`in` / `notin` / `ni` / `notni`) --------------

  private convertMembership(operator: string, operands: Tree[]): MathNode {
    const math = this.math;
    const flipped = operator === "ni" || operator === "notni";
    const rawX = flipped ? operands[1] : operands[0];
    const rawInterval = flipped ? operands[0] : operands[1];

    if (typeof rawX !== "number" && typeof rawX !== "string")
      throw new Error(
        "Set membership non-string variables not implemented for conversion to mathjs",
      );
    const x = this.convert(rawX);

    const interval = asTreeArray(rawInterval);
    if (interval[0] !== "interval")
      throw new Error(
        "Set membership in non-intervals not implemented for conversion to mathjs",
      );

    const args = asTreeArray(interval[1]);
    const closed = asTreeArray(interval[2]);
    if (args[0] !== "tuple" || closed[0] !== "tuple")
      throw new Error("Badly formed ast");

    const a = this.convert(args[1]);
    const b = this.convert(args[2]);

    const comparisons: MathNode[] = [];
    comparisons.push(
      closed[1]
        ? operatorNode(math, ">=", "largerEq", [x, a])
        : operatorNode(math, ">", "larger", [x, a]),
    );
    comparisons.push(
      closed[2]
        ? operatorNode(math, "<=", "smallerEq", [x, b])
        : operatorNode(math, "<", "smaller", [x, b]),
    );

    let result = operatorNode(math, "and", "and", comparisons);
    if (operator === "notin" || operator === "notni")
      result = operatorNode(math, "not", "not", [result]);
    return result;
  }

  // -- interval containment (`subset` / `superset` / negations) -----------

  private convertContainment(operator: string, operands: Tree[]): MathNode {
    const math = this.math;
    const flipped = operator === "superset" || operator === "notsuperset";
    const small = asTreeArray(flipped ? operands[1] : operands[0]);
    const big = asTreeArray(flipped ? operands[0] : operands[1]);

    if (small[0] !== "interval" || big[0] !== "interval")
      throw new Error(
        "Set containment of non-intervals not implemented for conversion to mathjs",
      );

    const smallArgs = asTreeArray(small[1]);
    const smallClosed = asTreeArray(small[2]);
    const bigArgs = asTreeArray(big[1]);
    const bigClosed = asTreeArray(big[2]);
    if (
      smallArgs[0] !== "tuple" ||
      smallClosed[0] !== "tuple" ||
      bigArgs[0] !== "tuple" ||
      bigClosed[0] !== "tuple"
    )
      throw new Error("Badly formed ast");

    const smallA = this.convert(smallArgs[1]);
    const smallB = this.convert(smallArgs[2]);
    const bigA = this.convert(bigArgs[1]);
    const bigB = this.convert(bigArgs[2]);

    const comparisons: MathNode[] = [];
    comparisons.push(
      smallClosed[1] && !bigClosed[1]
        ? operatorNode(math, ">", "larger", [smallA, bigA])
        : operatorNode(math, ">=", "largerEq", [smallA, bigA]),
    );
    comparisons.push(
      smallClosed[2] && !bigClosed[2]
        ? operatorNode(math, "<", "smaller", [smallB, bigB])
        : operatorNode(math, "<=", "smallerEq", [smallB, bigB]),
    );

    let result = operatorNode(math, "and", "and", comparisons);
    if (operator === "notsubset" || operator === "notsuperset")
      result = operatorNode(math, "not", "not", [result]);
    return result;
  }

  // -- matrices -----------------------------------------------------------

  private convertMatrix(operands: Tree[]): MathNode {
    const math = this.math;
    // Nested ArrayNodes; math.js turns these into a matrix on eval.
    const size = asTreeArray(operands[0]);
    const nrows = size[1];
    const ncols = size[2];
    if (!Number.isInteger(nrows) || !Number.isInteger(ncols))
      throw new Error("Matrix must have integer dimensions");

    const entries = asTreeArray(operands[1]);
    const rows: MathNode[] = [];
    for (let i = 1; i <= (nrows as number); i++) {
      const rowTree = asTreeArray(entries[i]);
      const row: MathNode[] = [];
      for (let j = 1; j <= (ncols as number); j++) {
        row.push(this.convert(rowTree[j]));
      }
      rows.push(new math.ArrayNode(row));
    }
    return new math.ArrayNode(rows);
  }

  // -- plain operators / vectors / logical ops ----------------------------

  private convertSimpleOperator(operator: string, operands: Tree[]): MathNode {
    const math = this.math;
    const args = operands.map((v) => this.convert(v));

    switch (operator) {
      case "+":
        return args.length === 1
          ? args[0]
          : operatorNode(math, "+", "add", args);
      case "*":
        return operatorNode(math, "*", "multiply", args);
      case "/":
        return operatorNode(math, "/", "divide", args);
      case "-":
        return operatorNode(math, "-", "unaryMinus", [args[0]]);
      case "^":
        return operatorNode(math, "^", "pow", args);
      case "vector":
      case "altvector":
        return new math.ArrayNode(args);
      case "and":
        return operatorNode(math, "and", "and", args);
      case "or":
        return operatorNode(math, "or", "or", args);
      case "not":
        return operatorNode(math, "not", "not", [args[0]]);
      case "<":
        return operatorNode(math, "<", "smaller", args);
      case ">":
        return operatorNode(math, ">", "larger", args);
      case "le":
        return operatorNode(math, "<=", "smallerEq", args);
      case "ge":
        return operatorNode(math, ">=", "largerEq", args);
      case "ne":
        return operatorNode(math, "!=", "unequal", args);
      case "binom":
        return new math.FunctionNode(new math.SymbolNode("combinations"), args);
      default:
        throw new Error(
          `Operator ${operator} not implemented for conversion to mathjs`,
        );
    }
  }
}

// ---------------------------------------------------------------------------
// Numeric-evaluation transforms & helpers
// ---------------------------------------------------------------------------

/**
 * Rewrite `x!` (a math.js `factorial` operator node) to `gamma(x + 1)` so the
 * expression evaluates at non-integer and complex arguments. Applied after
 * conversion, mirroring the JS library's `f()`.
 */
export function factorialToGamma(
  math: MathJsInstance,
  node: MathNode,
): MathNode {
  return node.transform((n: MathNode): MathNode => {
    const asOp = n as {
      isOperatorNode?: boolean;
      op?: string;
      fn?: string;
      args?: MathNode[];
    };
    if (asOp.isOperatorNode && asOp.op === "!" && asOp.fn === "factorial") {
      const arg = (asOp.args as MathNode[])[0];
      const incremented = operatorNode(math, "+", "add", [
        arg,
        new math.ConstantNode(1),
      ]);
      return new math.FunctionNode(new math.SymbolNode("gamma"), [incremented]);
    }
    return n;
  });
}

/**
 * Convert a `Tree` to a math.js node ready for numeric evaluation
 * (factorial → gamma applied).
 */
export function treeToMathNode(math: MathJsInstance, tree: Tree): MathNode {
  return factorialToGamma(math, new TreeToMathjs(math).convert(tree));
}

/**
 * Compile a `Tree` to a math.js {@link EvalFunction}. Compile once, then call
 * `compiled.evaluate(scope)` per sample — this is the entry point for graphing
 * loops (e.g. jsxgraph via Doenet), which stay entirely in JS.
 */
export function compileTree(math: MathJsInstance, tree: Tree): EvalFunction {
  return treeToMathNode(math, tree).compile();
}

// ---------------------------------------------------------------------------
// Rust/WASM bridge
// ---------------------------------------------------------------------------

/**
 * The subset of the Rust/WASM `Expression` handle this bridge uses. `expr.tree`
 * is not exposed by the WASM port; the AST comes across as JSON via
 * `tree_json()`. Normalization passes return fresh handles that must be freed.
 */
export interface RustExprLike {
  /** The parse tree serialized to the JS `Tree` JSON shape. */
  tree_json(): string;
  /** Canonicalize function-name variants to math.js-recognized names. */
  normalize_function_names(): RustExprLike;
  /** wasm-bindgen free; present on real handles, absent on plain stand-ins. */
  free?(): void;
  /** wasm-bindgen zeroes this on free; used to guard against a double free. */
  readonly __wbg_ptr?: number;
}

/** Free a wasm-bindgen handle, tolerating already-freed / non-wasm values. */
function freeHandle(h: RustExprLike): void {
  try {
    if (h && typeof h.free === "function" && h.__wbg_ptr !== 0) h.free();
  } catch {
    /* not a wasm handle, or already freed */
  }
}

/**
 * Build a numeric-evaluation math.js node from a Rust/WASM `Expression` handle.
 *
 * Normalization is done Rust-side (`normalize_function_names`) — the "option 1"
 * split where Rust owns the AST and its normalization, and JS owns only the
 * math.js node construction. The temporary normalized handle is freed before
 * returning; the caller's `expr` is never freed here.
 *
 * @param normalize  set `false` if `expr` is already normalized, to skip the
 *                   extra WASM round-trip and handle allocation.
 */
export function rustExprToMathNode(
  math: MathJsInstance,
  expr: RustExprLike,
  { normalize = true }: { normalize?: boolean } = {},
): MathNode {
  const source = normalize ? expr.normalize_function_names() : expr;
  try {
    const tree = JSON.parse(source.tree_json()) as Tree;
    return treeToMathNode(math, tree);
  } finally {
    if (source !== expr) freeHandle(source);
  }
}

/**
 * Compile a Rust/WASM `Expression` handle to a math.js {@link EvalFunction}.
 * Convenience wrapper over {@link rustExprToMathNode} for the graphing path:
 * compile once, then call `.evaluate(scope)` per sample.
 */
export function compileRustExpr(
  math: MathJsInstance,
  expr: RustExprLike,
  options?: { normalize?: boolean },
): EvalFunction {
  return rustExprToMathNode(math, expr, options).compile();
}

export default TreeToMathjs;
