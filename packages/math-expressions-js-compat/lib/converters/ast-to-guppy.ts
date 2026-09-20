/*
 * math-expressions AST → Guppy XML.
 *
 * Ported from the legacy `lib/converters/ast-to-guppy.js`. Guppy is an editable
 * math widget whose document is XML: `<e>` nodes hold literal text and `<f>`
 * nodes are templates carrying their own LaTeX/plaintext renderings plus one
 * `<c>` child per editable slot. Emitting it is notation only, so it stays in
 * TypeScript rather than crossing into the Rust core.
 *
 * The grammar is the usual expression/term/factor cascade; each level decides
 * whether its operands need parentheses before handing them to an emitter.
 *
 * Copyright 2017 by Jim Fowler <kisonecat@gmail.com>
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

import type { Tree } from "math-expressions-rs-wasm";

/**
 * The AST as callers hand it over: a plain array literal such as
 * `["+", 1, "x"]` widens to `(string | number)[]`, which is not assignable to
 * `Tree`'s `[tag, ...operands]` tuple. Narrowed to `Tree` once, at `convert`.
 */
export type AstInput = Tree | AstInput[];

// ---------------------------------------------------------------------------
// Guppy `<f>` templates
// ---------------------------------------------------------------------------

function dfrac(a: string, b: string): string {
  return (
    '<f type="fraction" group="functions"><b p="latex">\\dfrac{<r ref="1"/>}{<r ref="2"/>}</b><b p="small_latex">\\frac{<r ref="1"/>}{<r ref="2"/>}</b><b p="text">(<r ref="1"/>)/(<r ref="2"/>)</b><c up="1" down="2" name="numerator"><e></e>' +
    a +
    '<e></e></c><c up="1" down="2" name="denominator"><e></e>' +
    b +
    "<e></e></c></f>"
  );
}

/** One-argument named function (`sin`, `log`, `exp`, …) — one editable slot. */
function trig(name: string, parameter: string): string {
  return (
    '<f type="' +
    name +
    '" group="functions"><b p="latex">\\' +
    name +
    '\\left(<r ref="1"/>\\right)</b><b p="text"> ' +
    name +
    '(<r ref="1"/>)</b><c delete="1"><e></e>' +
    parameter +
    "<e></e></c></f>"
  );
}

function sqrt(x: string): string {
  return (
    '<f type="square_root" group="functions"><b p="latex">\\sqrt{<r ref="1"/>}</b><b p="text">sqrt(<r ref="1"/>)</b><c delete="1"><e></e>' +
    x +
    "<e></e></c></f>"
  );
}

function power(x: string, y: string): string {
  return (
    '<f type="exponential" group="functions"><b p="latex">{<r ref="1"/>}^{<r ref="2"/>}</b><b p="text">(<r ref="1"/>)^(<r ref="2"/>)</b><c up="2" bracket="yes" delete="1" name="base"><e></e>' +
    x +
    '<e></e></c><c down="1" delete="1" name="exponent" small="yes"><e></e>' +
    y +
    "<e></e></c></f>"
  );
}

function abs(x: string): string {
  return (
    '<f type="absolute_value" group="functions"><b p="latex">\\left|<r ref="1"/>\\right|</b><b p="text">abs(<r ref="1"/>)</b><c delete="1"><e></e>' +
    x +
    "<e></e></c></f>"
  );
}

function paren(x: string): string {
  return (
    '<f type="bracket" group="functions"><b p="latex">\\left(<r ref="1"/>\\right)</b><b p="text">(<r ref="1"/>)</b><c delete="1" is_bracket="yes"><e></e>' +
    x +
    "<e></e></c></f>"
  );
}

/**
 * Emitters keyed by AST operator. Every operand has already been rendered to
 * Guppy XML by the caller, which is also where parenthesization is decided.
 */
const operators: Record<string, (operands: string[]) => string> = {
  "+": (operands) => operands.join("<e>+</e>"),
  // Unary minus: the sign lives inside the `<e>` so `factor` can spot it by
  // the leading `<e>-` and re-parenthesize when it appears as an operand.
  "-": (operands) => "<e>-" + operands.join("-") + "</e>",
  "*": (operands) =>
    operands.join(
      '<f type="*" group="operations" c="yes"><b p="latex">\\cdot</b><b p="text">*</b></f>',
    ),
  "/": (operands) => dfrac(operands[0], operands[1]),
  "^": (operands) => power(operands[0], operands[1]),
  sin: (operands) => trig("sin", operands[0]),
  cos: (operands) => trig("cos", operands[0]),
  tan: (operands) => trig("tan", operands[0]),
  arcsin: (operands) => trig("arcsin", operands[0]),
  arccos: (operands) => trig("arccos", operands[0]),
  arctan: (operands) => trig("arctan", operands[0]),
  arccsc: (operands) => trig("arccsc", operands[0]),
  arcsec: (operands) => trig("arcsec", operands[0]),
  arccot: (operands) => trig("arccot", operands[0]),
  csc: (operands) => trig("csc", operands[0]),
  sec: (operands) => trig("sec", operands[0]),
  cot: (operands) => trig("cot", operands[0]),
  log: (operands) => trig("log", operands[0]),
  exp: (operands) => trig("exp", operands[0]),
  ln: (operands) => trig("ln", operands[0]),
  sqrt: (operands) => sqrt(operands[0]),
  abs: (operands) => abs(operands[0]),
  //"factorial": function(operands) { return operands[0] + "!"; },
};

// The legacy file was damaged by an over-eager `factorial` → `this.factorial`
// find/replace (the same one that produced its "math-this.expressions" header).
// Kept verbatim so behavior matches: with no `factorial` emitter above, the
// name below is the only thing keeping factorial out of the function branch.
const FACTORIAL = "this.factorial";

const functionSymbols = [
  "sin",
  "cos",
  "tan",
  "csc",
  "sec",
  "cot",
  "arcsin",
  "arccos",
  "arctan",
  "arccsc",
  "arcsec",
  "arccot",
  "log",
  "ln",
  "exp",
  "sqrt",
  "abs",
  FACTORIAL,
];

function isFunctionSymbol(symbol: string): boolean {
  return functionSymbols.includes(symbol);
}

const greekSymbols = [
  "pi",
  "theta",
  "Theta",
  "alpha",
  "nu",
  "beta",
  "xi",
  "Xi",
  "gamma",
  "Gamma",
  "delta",
  "Delta",
  "Pi",
  "epsilon",
  "rho",
  "zeta",
  "sigma",
  "Sigma",
  "eta",
  "tau",
  "upsilon",
  "Upsilon",
  "iota",
  "phi",
  "Phi",
  "kappa",
  "chi",
  "lambda",
  "Lambda",
  "psi",
  "Psi",
  "omega",
  "Omega",
];

function isGreekLetterSymbol(symbol: string): boolean {
  return greekSymbols.includes(symbol);
}

// ---------------------------------------------------------------------------
// Converter
// ---------------------------------------------------------------------------

/** `new astToGuppy().convert(["+", 1, "x"])` → `"<m><e>1+x</e></m>"`. */
export default class astToGuppy {
  /*
    factor =
    '(' expression ')' |
    number |
    variable |
    function factor |
    factor '^' factor
    '-' factor |
    nonMinusFactor
  */

  factor(tree: Tree): string {
    if (typeof tree === "string") {
      if (isGreekLetterSymbol(tree)) {
        return (
          '<f type="' +
          tree +
          '" group="greek" c="yes"><b p="latex">\\' +
          tree +
          '</b><b p="text"> $' +
          tree +
          "</b></f>"
        );
      }

      return "<e>" + tree + "</e>";
    }

    if (typeof tree === "number") {
      return "<e>" + tree + "</e>";
    }

    if (!Array.isArray(tree)) {
      return "<e></e>";
    }

    let operator = tree[0];
    let operands: Tree[] = tree.slice(1);

    // `["apply", f, arg]` renders as whatever `f` renders as.
    if (operator === "apply") {
      operator = tree[1] as string;
      operands = tree.slice(2);
    }

    // No emitter for this operator — an unported function symbol, or `~`,
    // which the legacy file dispatched on without ever adding a `"~"` entry.
    // Falling through to the parenthesized default beats a `TypeError` out of
    // `operators[operator](...)`.
    const emit = operators[operator as string];
    if (!emit) {
      return paren(this.expression(tree));
    }

    // Absolute value doesn't need any special parentheses handling, but its
    // operand is really an expression
    if (operator === "abs") {
      return emit(operands.map((v) => this.expression(v)));
    } else if (isFunctionSymbol(operator)) {
      // A short or purely numeric factorial argument needs no grouping.
      if (
        operator === FACTORIAL &&
        (String(operands[0]).length === 1 ||
          /^[0-9]*$/.test(String(operands[0])))
      )
        return emit(operands.map(String));

      return emit(operands.map((v) => this.factor(v)));
    }

    if (operator === "^" || operator === "~") {
      return emit(operands.map((v) => this.factor(v)));
    }

    return paren(this.expression(tree));
  }

  /** As {@link factor}, but bracketing anything that came back negated. */
  factorWithParenthesesIfNegated(tree: Tree): string {
    const result = this.factor(tree);

    if (/^<e>-/.test(result)) return paren(result);

    // else
    return result;
  }

  /*
    term =
    term '*' factor |
    term nonMinusFactor |
    term '/' factor |
    factor
  */

  term(tree: Tree): string {
    if (!Array.isArray(tree)) {
      return this.factor(tree);
    }

    const operator = tree[0];
    const operands: Tree[] = tree.slice(1);

    if (operator === "*") {
      return operators[operator](
        operands.map((v, i) => {
          const result = this.factorWithParenthesesIfNegated(v);

          // A following factor that starts with a digit would read as one
          // number juxtaposed with the previous one, so spell the `*` out.
          if (/^[0-9]/.test(result) && i > 0) return " * " + result;
          else return result;
        }),
      );
    }

    if (operator === "/") {
      return operators[operator](operands.map((v) => this.factor(v)));
    }

    return this.factor(tree);
  }

  /*
     expression =
      expression '+' term |
      expression '-' term |
      term
  */

  expression(tree: Tree): string {
    if (!Array.isArray(tree)) {
      return this.term(tree);
    }

    const operator = tree[0];
    const operands: Tree[] = tree.slice(1);

    if (operator === "+" || operator === "-") {
      return operators[operator](
        operands.map((v) => this.factorWithParenthesesIfNegated(v)),
      );
    }

    return this.term(tree);
  }

  convert(tree: AstInput): string {
    // The emitters pad slots with empty `<e></e>`; collapsing every adjacent
    // `</e><e>` merges those into the neighbouring literal text nodes.
    return (
      "<m><e></e>" +
      this.expression(tree as Tree) +
      "<e></e></m>"
    ).replace(/<\/e><e>/g, "");
  }
}
