/*
 * math.js expression tree → math-expressions AST.
 *
 * Ported from the legacy `lib/converters/mathjs-to-ast.js`. This is pure
 * notation shuffling — it walks the node tree math.js' parser produced and
 * relabels it into the AST shape — so it stays in TypeScript rather than
 * crossing into the Rust core, which never sees a math.js node.
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

import type { Tree, TreeArray } from "math-expressions-rs-wasm";

/**
 * The structural subset of a math.js node this converter reads. Declared here
 * rather than reusing mathjs' `MathNode` because the conversion dispatches on
 * the `isXNode` marker flags — which the mathjs typings put on the concrete
 * node classes, not on the base node the parser is declared to return.
 */
export interface MathJsNode {
  type: string;
  isConstantNode?: boolean;
  isSymbolNode?: boolean;
  isOperatorNode?: boolean;
  isFunctionNode?: boolean;
  isArrayNode?: boolean;
  isParenthesisNode?: boolean;
  /** ConstantNode */
  value?: Tree;
  /** SymbolNode / FunctionNode */
  name?: string;
  /** OperatorNode: the source token (`-`) and the function it denotes */
  op?: string;
  fn?: string;
  /** OperatorNode / FunctionNode operands */
  args?: MathJsNode[];
  /** ArrayNode entries */
  items?: MathJsNode[];
  /** ParenthesisNode */
  content?: MathJsNode;
}

/**
 * Operator emitters keyed by `"<op>,<fn>"`, because neither half identifies an
 * operator on its own: math.js reuses `-` for both `subtract` and `unaryMinus`.
 * Unlisted combinations are rejected rather than guessed at.
 */
const operators: Record<string, (operands: Tree[]) => TreeArray> = {
  "+,add": (operands) => ["+", ...operands],
  "*,multiply": (operands) => ["*", ...operands],
  "/,divide": (operands) => ["/", operands[0], operands[1]],
  "-,unaryMinus": (operands) => ["-", operands[0]],
  // The AST has no binary subtraction: `a - b` is `a + (-b)`.
  "-,subtract": (operands) => ["+", operands[0], ["-", operands[1]]],
  "^,pow": (operands) => ["^", operands[0], operands[1]],
  "and,and": (operands) => ["and", ...operands],
  "or,or": (operands) => ["or", ...operands],
  "not,not": (operands) => ["not", operands[0]],
  "==,equal": (operands) => ["=", ...operands],
  "<,smaller": (operands) => ["<", operands[0], operands[1]],
  ">,larger": (operands) => [">", operands[0], operands[1]],
  "<=,smallerEq": (operands) => ["le", operands[0], operands[1]],
  ">=,largerEq": (operands) => ["ge", operands[0], operands[1]],
  "!=,unequal": (operands) => ["ne", operands[0], operands[1]],
  "!,factorial": (operands) => ["apply", "factorial", operands[0]],
};

/** `new mathjsToAst().convert(math.parse("1+x"))` → `["+", 1, "x"]`. */
export default class mathjsToAst {
  convert(mathnode: MathJsNode): Tree {
    if (mathnode.isConstantNode) return mathnode.value as Tree;
    if (mathnode.isSymbolNode) return mathnode.name as string;

    if (mathnode.isOperatorNode) {
      const key = [mathnode.op, mathnode.fn].join(",");
      const emit = operators[key];
      if (!emit)
        throw Error(`Unsupported operator: ${mathnode.op}, ${mathnode.fn}`);
      return emit((mathnode.args ?? []).map((v) => this.convert(v)));
    }

    if (mathnode.isFunctionNode) {
      const converted = (mathnode.args ?? []).map((v) => this.convert(v));
      // A multi-argument call becomes a single `tuple` argument, since `apply`
      // in the AST is always `["apply", name, oneArgument]`.
      const args: Tree =
        converted.length > 1 ? ["tuple", ...converted] : converted[0];
      return ["apply", mathnode.name as string, args];
    }

    if (mathnode.isArrayNode) {
      // The legacy port read `.args` here, which current math.js calls `.items`.
      const entries = mathnode.items ?? mathnode.args ?? [];
      return ["vector", ...entries.map((v) => this.convert(v))];
    }

    if (mathnode.isParenthesisNode)
      return this.convert(mathnode.content as MathJsNode);

    throw Error(`Unsupported node type: ${mathnode.type}`);
  }
}
