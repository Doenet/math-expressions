import { ParseError } from "./error";
import lexer from "./lexer";
import flatten from "./flatten";
import { get_all_units } from "../expression/units";

// UPDATETHIS: Delete or change to new license & package name

/*
 * recursive descent parser for math expressions
 *
 * Copyright 2014-2017 by
 *  Jim Fowler <kisonecat@gmail.com>
 *  Duane Nykamp <nykamp@umn.edu>
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
 *
 */

// UPDATETHIS: This grammar is out-of-date!!!

/* Grammar:

   statement_list =
   statement_list ',' statement |
   statement

   statement =
   '\\dots' |
   statement_a '|' statement_a |
   statement_a 'MID' statement_a |
   statement_a ':' statement_a
   **** statement_a '|' statement_a
        used with turning off '|' statement '|' in baseFactor
        tried only after parse error encountered

   statement_a =
   statement_a 'OR' statement_b |
   statement_b

   statement_b =
   statement_b 'AND' relation |
   relation

   relation =
   'NOT' relation |
   relation '=' expression |
   relation 'NE' expression |
   relation '<' expression |
   relation '>' expression |
   relation 'LE' expression |
   relation 'GE' expression |
   relation 'IN' expression |
   relation 'NOTIN' expression |
   relation 'NI' expression |
   relation 'NOTNI' expression |
   relation 'SUBSET' expression |
   relation 'NOTSUBSET' expression |
   relation 'SUPERSET' expression |
   relation 'NOTSUPERSET' expression |
   expression

   expression =
   expression '+' term |
   expression '-' term |
   expression 'UNION' term |
   expression 'INTERSECT' term |
   '+' term |
   term

   term =
   term '*' factor |
   term nonMinusFactor |
   term '/' factor |
   factor

   baseFactor =
   '(' statement_list ')' |
   '[' statement_list ']' |
   '{' statement_list '}' |
   'LBRACE' statement_list 'RBRACE' |
   '(' statement ',' statement ']' |
   '[' statement ',' statement ')' |
   '|' statement '|' |
   \frac{statement}{statement} |
   number |
   variable |
   modified_function '(' statement_list ')' |
   modified_applied_function '(' statement_list ')' |
   modified_function '{' statement_list '}' |
   modified_applied_function '{' statement_list '}' |
   modified_function |
   modified_applied_function factor |
   sqrt '[' statement ']' '{' statement '}' |
   baseFactor '_' baseFactor |
   *** modified_applied_function factor
       allowed only if allowSimplifiedFunctionApplication==true
   *** '|' statement '|'
       allowed only at beginning of factor or if not currently in absolute value

   modified_function =
   function |
   function '_' baseFactor |
   function '_' baseFactor '^' factor |
   function '^' factor
   function "'"
   function '_' baseFactor "'"
   function '_' baseFactor "'" '^' factor
   function "'" '^' factor
   *** where the "'" after the functions can be repeated

   modified_applied_function =
   applied_function |
   applied_function '_' baseFactor |
   applied_function '_' baseFactor '^' factor |
   applied_function '^' factor
   applied_function "'"
   applied_function '_' baseFactor "'"
   applied_function '_' baseFactor "'" '^' factor
   applied_function "'" '^' factor
   *** where the "'" after the applied_functions can be repeated

   nonMinusFactor =
   baseFactor |
   baseFactor '^' factor |
   baseFactor '!' and/or "'" |
   baseFactor '!' and/or "'"  '^' factor|
   *** where '!' and/or "'"  indicates arbitrary sequence of "!" and/or "'"

   factor =
   '-' factor |
   nonMinusFactor

*/

// Some of the latex commands that lead to spacing
const whitespace_rule =
  "\\s|\\\\,|\\\\!|\\\\ |\\\\>|\\\\;|\\\\:|\\\\quad\\b|\\\\qquad\\b";

// in order to parse as scientific notation, e.g., 3.2E-12 or .7E+3,
// it must be at the end or followed a comma, &, |, \|, ), }, \}, ], \\, or \end
const sci_notat_exp_regex =
  "(E[+\\-]?[0-9]+\\s*($|(?=\\,|&|\\||\\\\\\||\\)|\\}|\\\\}|\\]|\\\\\\\\|\\\\end)))?";

const number_rules_sci = [
  ["[0-9]+(\\.[0-9]*)?" + sci_notat_exp_regex, "NUMBER"],
  ["\\.[0-9]+" + sci_notat_exp_regex, "NUMBER"],
];

const number_rules_non_sci = [
  ["[0-9]+(\\.[0-9]*)?", "NUMBER"],
  ["\\.[0-9]+", "NUMBER"],
];

const base_latex_rules = [
  ["\\*", "*"],
  ["\\/", "/"],
  ["-", "-"],
  ["\\+", "+"],
  ["\\^", "^"],
  ["\\(", "("],
  ["\\\\left\\s*\\(", "("],
  ["\\\\bigl\\s*\\(", "("],
  ["\\\\Bigl\\s*\\(", "("],
  ["\\\\biggl\\s*\\(", "("],
  ["\\\\Biggl\\s*\\(", "("],
  ["\\)", ")"],
  ["\\\\right\\s*\\)", ")"],
  ["\\\\bigr\\s*\\)", ")"],
  ["\\\\Bigr\\s*\\)", ")"],
  ["\\\\biggr\\s*\\)", ")"],
  ["\\\\Biggr\\s*\\)", ")"],
  ["\\[", "["],
  ["\\\\left\\s*\\[", "["],
  ["\\\\bigl\\s*\\[", "["],
  ["\\\\Bigl\\s*\\[", "["],
  ["\\\\biggl\\s*\\[", "["],
  ["\\\\Biggl\\s*\\[", "["],
  ["\\]", "]"],
  ["\\\\right\\s*\\]", "]"],
  ["\\\\bigr\\s*\\]", "]"],
  ["\\\\Bigr\\s*\\]", "]"],
  ["\\\\biggr\\s*\\]", "]"],
  ["\\\\Biggr\\s*\\]", "]"],
  ["\\|", "|"],
  ["\\\\left\\s*\\|", "|L"],
  ["\\\\bigl\\s*\\|", "|L"],
  ["\\\\Bigl\\s*\\|", "|L"],
  ["\\\\biggl\\s*\\|", "|L"],
  ["\\\\Biggl\\s*\\|", "|L"],
  ["\\\\right\\s*\\|", "|"],
  ["\\\\bigr\\s*\\|", "|"],
  ["\\\\Bigr\\s*\\|", "|"],
  ["\\\\biggr\\s*\\|", "|"],
  ["\\\\Biggr\\s*\\|", "|"],
  ["\\\\big\\s*\\|", "|"],
  ["\\\\Big\\s*\\|", "|"],
  ["\\\\bigg\\s*\\|", "|"],
  ["\\\\Bigg\\s*\\|", "|"],
  ["{", "{"],
  ["}", "}"],
  ["\\\\{", "LBRACE"],
  ["\\\\left\\s*\\\\{", "LBRACE"],
  ["\\\\bigl\\s*\\\\{", "LBRACE"],
  ["\\\\Bigl\\s*\\\\{", "LBRACE"],
  ["\\\\biggl\\s*\\\\{", "LBRACE"],
  ["\\\\Biggl\\s*\\\\{", "LBRACE"],
  ["\\\\}", "RBRACE"],
  ["\\\\right\\s*\\\\}", "RBRACE"],
  ["\\\\bigr\\s*\\\\}", "RBRACE"],
  ["\\\\Bigr\\s*\\\\}", "RBRACE"],
  ["\\\\biggr\\s*\\\\}", "RBRACE"],
  ["\\\\Biggr\\s*\\\\}", "RBRACE"],
  ["\\\\lfloor(?![a-zA-Z])", "LFLOOR"],
  ["\\\\left\\s*\\\\lfloor(?![a-zA-Z])", "LFLOOR"],
  ["\\\\bigl\\s*\\\\lfloor(?![a-zA-Z])", "LFLOOR"],
  ["\\\\Bigl\\s*\\\\lfloor(?![a-zA-Z])", "LFLOOR"],
  ["\\\\biggl\\s*\\\\lfloor(?![a-zA-Z])", "LFLOOR"],
  ["\\\\Biggl\\s*\\\\lfloor(?![a-zA-Z])", "LFLOOR"],
  ["\\\\rfloor(?![a-zA-Z])", "RFLOOR"],
  ["\\\\right\\s*\\\\rfloor(?![a-zA-Z])", "RFLOOR"],
  ["\\\\bigr\\s*\\\\rfloor(?![a-zA-Z])", "RFLOOR"],
  ["\\\\Bigr\\s*\\\\rfloor(?![a-zA-Z])", "RFLOOR"],
  ["\\\\biggr\\s*\\\\rfloor(?![a-zA-Z])", "RFLOOR"],
  ["\\\\Biggr\\s*\\\\rfloor(?![a-zA-Z])", "RFLOOR"],
  ["\\\\lceil(?![a-zA-Z])", "LCEIL"],
  ["\\\\left\\s*\\\\lceil(?![a-zA-Z])", "LCEIL"],
  ["\\\\bigl\\s*\\\\lceil(?![a-zA-Z])", "LCEIL"],
  ["\\\\Bigl\\s*\\\\lceil(?![a-zA-Z])", "LCEIL"],
  ["\\\\biggl\\s*\\\\lceil(?![a-zA-Z])", "LCEIL"],
  ["\\\\Biggl\\s*\\\\lceil(?![a-zA-Z])", "LCEIL"],
  ["\\\\rceil(?![a-zA-Z])", "RCEIL"],
  ["\\\\right\\s*\\\\rceil(?![a-zA-Z])", "RCEIL"],
  ["\\\\bigr\\s*\\\\rceil(?![a-zA-Z])", "RCEIL"],
  ["\\\\Bigr\\s*\\\\rceil(?![a-zA-Z])", "RCEIL"],
  ["\\\\biggr\\s*\\\\rceil(?![a-zA-Z])", "RCEIL"],
  ["\\\\Biggr\\s*\\\\rceil(?![a-zA-Z])", "RCEIL"],
  ["\\\\langle(?![a-zA-Z])", "LANGLE"],
  ["\\\\left\\s*\\\\langle(?![a-zA-Z])", "LANGLE"],
  ["\\\\bigl\\s*\\\\langle(?![a-zA-Z])", "LANGLE"],
  ["\\\\Bigl\\s*\\\\langle(?![a-zA-Z])", "LANGLE"],
  ["\\\\biggl\\s*\\\\langle(?![a-zA-Z])", "LANGLE"],
  ["\\\\Biggl\\s*\\\\langle(?![a-zA-Z])", "LANGLE"],
  ["\\\\rangle(?![a-zA-Z])", "RANGLE"],
  ["\\\\right\\s*\\\\rangle(?![a-zA-Z])", "RANGLE"],
  ["\\\\bigr\\s*\\\\rangle(?![a-zA-Z])", "RANGLE"],
  ["\\\\Bigr\\s*\\\\rangle(?![a-zA-Z])", "RANGLE"],
  ["\\\\biggr\\s*\\\\rangle(?![a-zA-Z])", "RANGLE"],
  ["\\\\Biggr\\s*\\\\rangle(?![a-zA-Z])", "RANGLE"],
  ["\\\\cdot(?![a-zA-Z])", "*"],
  ["\\\\div(?![a-zA-Z])", "/"],
  ["\\\\times(?![a-zA-Z])", "*"],
  [",", ","],
  [":", ":"],
  ["\\\\mid(?![a-zA-Z])", "MID"],

  ["\\\\varnothing(?![a-zA-Z])", "LATEXCOMMAND", "\\emptyset"],

  ["\\\\vartheta(?![a-zA-Z])", "LATEXCOMMAND", "\\theta"],
  ["\\\\varepsilon(?![a-zA-Z])", "LATEXCOMMAND", "\\epsilon"],
  ["\\\\varrho(?![a-zA-Z])", "LATEXCOMMAND", "\\rho"],
  ["\\\\varphi(?![a-zA-Z])", "LATEXCOMMAND", "\\phi"],

  ["\\\\infty(?![a-zA-Z])", "INFINITY"],

  ["\\\\asin(?![a-zA-Z])", "LATEXCOMMAND", "\\arcsin"],
  ["\\\\acos(?![a-zA-Z])", "LATEXCOMMAND", "\\arccos"],
  ["\\\\atan(?![a-zA-Z])", "LATEXCOMMAND", "\\arctan"],
  ["\\\\sqrt(?![a-zA-Z])", "SQRT"],

  ["\\\\land(?![a-zA-Z])", "AND"],
  ["\\\\wedge(?![a-zA-Z])", "AND"],

  ["\\\\lor(?![a-zA-Z])", "OR"],
  ["\\\\vee(?![a-zA-Z])", "OR"],

  ["\\\\lnot(?![a-zA-Z])", "NOT"],
  ["\\\\neg(?![a-zA-Z])", "NOT"],

  ["=", "="],
  ["\\\\neq(?![a-zA-Z])", "NE"],
  ["\\\\ne(?![a-zA-Z])", "NE"],
  ["\\\\not\\s*=", "NE"],
  ["\\\\leq(?![a-zA-Z])", "LE"],
  ["\\\\le(?![a-zA-Z])", "LE"],
  ["\\\\geq(?![a-zA-Z])", "GE"],
  ["\\\\ge(?![a-zA-Z])", "GE"],
  ["<", "<"],
  ["\\\\lt(?![a-zA-Z])", "<"],
  [">", ">"],
  ["\\\\gt(?![a-zA-Z])", ">"],

  ["\\\\forall(?![a-zA-Z])", "FORALL"],
  ["\\\\exists(?![a-zA-Z])", "EXISTS"],

  ["\\\\in(?![a-zA-Z])", "IN"],

  ["\\\\notin(?![a-zA-Z])", "NOTIN"],
  ["\\\\not\\s*\\\\in(?![a-zA-Z])", "NOTIN"],

  ["\\\\ni(?![a-zA-Z])", "NI"],

  ["\\\\not\\s*\\\\ni(?![a-zA-Z])", "NOTNI"],

  ["\\\\subset(?![a-zA-Z])", "SUBSET"],
  ["\\\\subseteq(?![a-zA-Z])", "SUBSETEQ"],

  ["\\\\not\\s*\\\\subset(?![a-zA-Z])", "NOTSUBSET"],
  ["\\\\not\\s*\\\\subseteq(?![a-zA-Z])", "NOTSUBSETEQ"],

  ["\\\\supset(?![a-zA-Z])", "SUPERSET"],
  ["\\\\supseteq(?![a-zA-Z])", "SUPERSETEQ"],

  ["\\\\not\\s*\\\\supset(?![a-zA-Z])", "NOTSUPERSET"],
  ["\\\\not\\s*\\\\supseteq(?![a-zA-Z])", "NOTSUPERSETEQ"],

  ["\\\\cup(?![a-zA-Z])", "UNION"],

  ["\\\\cap(?![a-zA-Z])", "INTERSECT"],

  ["\\\\to(?![a-zA-Z])", "RIGHTARROW"],
  ["\\\\rightarrow(?![a-zA-Z])", "RIGHTARROW"],
  ["\\\\longrightarrow(?![a-zA-Z])", "RIGHTARROW"],

  ["\\\\gets(?![a-zA-Z])", "LEFTARROW"],
  ["\\\\leftarrow(?![a-zA-Z])", "LEFTARROW"],
  ["\\\\longleftarrow(?![a-zA-Z])", "LEFTARROW"],

  ["\\\\leftrightarrow(?![a-zA-Z])", "LEFTRIGHTARROW"],
  ["\\\\longleftrightarrow(?![a-zA-Z])", "LEFTRIGHTARROW"],

  ["\\\\implies(?![a-zA-Z])", "IMPLIES"],
  ["\\\\Longrightarrow(?![a-zA-Z])", "IMPLIES"],
  ["\\\\Rightarrow(?![a-zA-Z])", "IMPLIES"],

  ["\\\\impliedby(?![a-zA-Z])", "IMPLIEDBY"],
  ["\\\\Longleftarrow(?![a-zA-Z])", "IMPLIEDBY"],
  ["\\\\Leftarrow(?![a-zA-Z])", "IMPLIEDBY"],

  ["\\\\iff(?![a-zA-Z])", "IFF"],
  ["\\\\Longleftrightarrow(?![a-zA-Z])", "IFF"],
  ["\\\\Leftrightarrow(?![a-zA-Z])", "IFF"],

  ["\\\\perp(?![a-zA-Z])", "PERP"],
  ["\\\\bot(?![a-zA-Z])", "PERP"],

  ["\\\\parallel(?![a-zA-Z])", "PARALLEL"],
  ["\\\\\\|", "PARALLEL"],

  ["\\\\angle(?![a-zA-Z])", "ANGLE"],

  ["\\\\int(?![a-zA-Z])", "INT"],

  ["!", "!"],
  ["'", "'"],
  ["_", "_"],
  ["&", "&"],
  ["\\\\ldots(?![a-zA-Z])", "LDOTS"],

  ["\\\\\\\\", "LINEBREAK"],

  ["\\\\begin\\s*{\\s*[a-zA-Z0-9]+\\s*}", "BEGINENVIRONMENT"],

  ["\\\\end\\s*{\\s*[a-zA-Z0-9]+\\s*}", "ENDENVIRONMENT"],

  ["\\\\operatorname\\s*{\\s*[a-zA-Z0-9\\+\\-]+\\s*}", "VARMULTICHAR"],

  ["\\\\[a-zA-Z]+(?![a-zA-Z])", "LATEXCOMMAND"],
  ["\\\\[$%]", "LATEXCOMMAND"],
  ["[a-zA-Z\uff3f$%]", "VAR"],
];

// defaults for parsers if not overridden by context

// if true, allowed applied functions to omit parentheses around argument
// if false, omitting parentheses will lead to a Parse Error
const allowSimplifiedFunctionApplicationDefault = true;

// allowed multicharacter latex symbols
// in addition to the below applied function symbols
const allowedLatexSymbolsDefault = [
  "alpha",
  "beta",
  "gamma",
  "Gamma",
  "delta",
  "Delta",
  "epsilon",
  "zeta",
  "eta",
  "theta",
  "Theta",
  "iota",
  "kappa",
  "lambda",
  "Lambda",
  "mu",
  "nu",
  "xi",
  "Xi",
  "pi",
  "Pi",
  "rho",
  "sigma",
  "Sigma",
  "tau",
  "Tau",
  "upsilon",
  "Upsilon",
  "phi",
  "Phi",
  "chi",
  "psi",
  "Psi",
  "omega",
  "Omega",
  "partial",
  "angle",
  "circ",
  "%",
  "$",
  "emptyset",
];

// Applied functions must be given an argument so that
// they are applied to the argument
const appliedFunctionSymbolsDefault = [
  "abs",
  "exp",
  "log",
  "ln",
  "log10",
  "sign",
  "sqrt",
  "erf",
  "cos",
  "cosh",
  "acos",
  "acosh",
  "arccos",
  "arccosh",
  "cot",
  "coth",
  "acot",
  "acoth",
  "arccot",
  "arccoth",
  "csc",
  "csch",
  "acsc",
  "acsch",
  "arccsc",
  "arccsch",
  "sec",
  "sech",
  "asec",
  "asech",
  "arcsec",
  "arcsech",
  "sin",
  "sinh",
  "asin",
  "asinh",
  "arcsin",
  "arcsinh",
  "tan",
  "tanh",
  "atan",
  "atan2",
  "atanh",
  "arctan",
  "arctanh",
  "arg",
  "conj",
  "Re",
  "Im",
  "det",
  "trace",
  "nPr",
  "nCr",
  "floor",
  "ceil",
  "round",
];

// Functions could have an argument, in which case they are applied
// or, if they don't have an argument in parentheses, then they are treated
// like a variable, except that trailing ^ and ' have higher precedence
const functionSymbolsDefault = ["f", "g"];

// operators must be given an argument
const operatorSymbolsDefault = {
  frac: { nargs: 2, substitute: "/" },
  binom: { nargs: 2 },
  vec: { nargs: 1 },
  overline: { nargs: 1, substitute: "linesegment", remove_products: true },
};

const unitsDefault = get_all_units();
unitsDefault["circ"] = unitsDefault["deg"];
unitsDefault["circ"].substitute = "deg";
unitsDefault["circ"].isExponent = true;
delete unitsDefault["deg"];

// Parse Leibniz notation
const parseLeibnizNotationDefault = true;

class latexToAst {
  constructor({
    allowSimplifiedFunctionApplication = allowSimplifiedFunctionApplicationDefault,
    allowedLatexSymbols = allowedLatexSymbolsDefault,
    appliedFunctionSymbols = appliedFunctionSymbolsDefault,
    functionSymbols = functionSymbolsDefault,
    operatorSymbols = operatorSymbolsDefault,
    units = unitsDefault,
    parseLeibnizNotation = parseLeibnizNotationDefault,
    parseScientificNotation = true,
  } = {}) {
    this.allowSimplifiedFunctionApplication =
      allowSimplifiedFunctionApplication;
    this.allowedLatexSymbols = allowedLatexSymbols;
    this.appliedFunctionSymbols = appliedFunctionSymbols;
    this.functionSymbols = functionSymbols;
    this.operatorSymbols = operatorSymbols;
    this.units = units;
    this.parseLeibnizNotation = parseLeibnizNotation;

    let latex_rules = base_latex_rules;

    if (parseScientificNotation) {
      latex_rules = [...number_rules_sci, ...latex_rules];
    } else {
      latex_rules = [...number_rules_non_sci, ...latex_rules];
    }
    this.lexer = new lexer(latex_rules, whitespace_rule);
  }

  advance(params) {
    this.token = this.lexer.advance(params);
    if (this.token.token_type === "INVALID") {
      throw new ParseError(
        "Invalid symbol '" + this.token.original_text + "'",
        this.lexer.location,
      );
    }
  }

  return_state() {
    return {
      lexer_state: this.lexer.return_state(),
      token: Object.assign({}, this.token),
    };
  }

  set_state(state) {
    this.lexer.set_state(state.lexer_state);
    this.token = Object.assign({}, state.token);
  }

  convert(input) {
    this.lexer.set_input(input);
    this.advance();

    var result = this.statement_list();

    if (this.token.token_type !== "EOF") {
      throw new ParseError(
        "Invalid location of '" + this.token.original_text + "'",
        this.lexer.location,
      );
    }

    return flatten(result);
  }

  statement_list() {
    var list = [this.statement()];

    while (this.token.token_type === ",") {
      this.advance();
      list.push(this.statement());
    }

    if (list.length > 1) list = ["list"].concat(list);
    else list = list[0];

    return list;
  }

  statement({ inside_absolute_value = 0 } = {}) {
    // \ldots can be a statement by itself
    if (this.token.token_type === "LDOTS") {
      this.advance();
      return ["ldots"];
    }

    var original_state;

    try {
      original_state = this.return_state();

      let lhs = this.statement_a({
        inside_absolute_value: inside_absolute_value,
      });

      if (this.token.token_type !== ":" && this.token.token_type !== "MID")
        return lhs;

      let operator = this.token.token_type === ":" ? ":" : "|";

      this.advance();

      let rhs = this.statement_a();

      return [operator, lhs, rhs];
    } catch (e) {
      try {
        // if ran into problem parsing statement
        // then try again with ignoring absolute value
        // and then interpreting bar as a binary operator

        // return state to what it was before attempting to parse statement
        this.set_state(original_state);

        let lhs = this.statement_a({ parse_absolute_value: false });

        if (this.token.token_type[0] !== "|") {
          throw e;
        }

        this.advance();

        let rhs = this.statement_a({ parse_absolute_value: false });

        return ["|", lhs, rhs];
      } catch (e2) {
        throw e; // throw original error
      }
    }
  }

  statement_a({ inside_absolute_value = 0, parse_absolute_value = true } = {}) {
    var lhs = this.statement_b({
      inside_absolute_value: inside_absolute_value,
      parse_absolute_value: parse_absolute_value,
    });

    while (
      [
        "IMPLIES",
        "IMPLIEDBY",
        "IFF",
        "LEFTARROW",
        "RIGHTARROW",
        "LEFTRIGHTARROW",
      ].includes(this.token.token_type)
    ) {
      let operation = this.token.token_type.toLowerCase();

      this.advance();

      let rhs = this.statement_b({
        inside_absolute_value: inside_absolute_value,
        parse_absolute_value: parse_absolute_value,
      });

      lhs = [operation, lhs, rhs];
    }

    return lhs;
  }

  statement_b({ inside_absolute_value = 0, parse_absolute_value = true } = {}) {
    var lhs = this.statement_c({
      inside_absolute_value: inside_absolute_value,
      parse_absolute_value: parse_absolute_value,
    });

    while (this.token.token_type === "OR") {
      let operation = this.token.token_type.toLowerCase();

      this.advance();

      let rhs = this.statement_c({
        inside_absolute_value: inside_absolute_value,
        parse_absolute_value: parse_absolute_value,
      });

      lhs = [operation, lhs, rhs];
    }

    return lhs;
  }

  statement_c(params) {
    // split AND into second statement to give higher precedence than OR

    var lhs = this.relation(params);

    while (this.token.token_type === "AND") {
      let operation = this.token.token_type.toLowerCase();

      this.advance();

      let rhs = this.relation(params);

      lhs = [operation, lhs, rhs];
    }

    return lhs;
  }

  relation(params) {
    if (this.token.token_type === "NOT" || this.token.token_type === "!") {
      this.advance();
      return ["not", this.relation(params)];
    }

    if (
      this.token.token_type === "FORALL" ||
      this.token.token_type === "EXISTS"
    ) {
      let operator = this.token.token_type.toLowerCase();
      this.advance();
      return [operator, this.relation(params)];
    }

    var lhs = this.expression(params);

    while (
      this.token.token_type === "=" ||
      this.token.token_type === "NE" ||
      this.token.token_type === "<" ||
      this.token.token_type === ">" ||
      this.token.token_type === "LE" ||
      this.token.token_type === "GE" ||
      this.token.token_type === "IN" ||
      this.token.token_type === "NOTIN" ||
      this.token.token_type === "NI" ||
      this.token.token_type === "NOTNI" ||
      this.token.token_type === "SUBSET" ||
      this.token.token_type === "NOTSUBSET" ||
      this.token.token_type === "SUBSETEQ" ||
      this.token.token_type === "NOTSUBSETEQ" ||
      this.token.token_type === "SUPERSET" ||
      this.token.token_type === "NOTSUPERSET" ||
      this.token.token_type === "SUPERSETEQ" ||
      this.token.token_type === "NOTSUPERSETEQ"
    ) {
      let operation = this.token.token_type.toLowerCase();

      let inequality_sequence = 0;

      if (this.token.token_type === "<" || this.token.token_type === "LE") {
        inequality_sequence = -1;
      } else if (
        this.token.token_type === ">" ||
        this.token.token_type === "GE"
      ) {
        inequality_sequence = 1;
      }

      this.advance();
      let rhs = this.expression(params);

      if (inequality_sequence === -1) {
        if (this.token.token_type === "<" || this.token.token_type === "LE") {
          // sequence of multiple < or <=
          let strict = ["tuple"];
          if (operation === "<") strict.push(true);
          else strict.push(false);

          let args = ["tuple", lhs, rhs];
          while (
            this.token.token_type === "<" ||
            this.token.token_type === "LE"
          ) {
            if (this.token.token_type === "<") strict.push(true);
            else strict.push(false);

            this.advance();
            args.push(this.expression(params));
          }
          lhs = ["lts", args, strict];
        } else {
          lhs = [operation, lhs, rhs];
        }
      } else if (inequality_sequence === 1) {
        if (this.token.token_type === ">" || this.token.token_type === "GE") {
          // sequence of multiple > or >=
          let strict = ["tuple"];
          if (operation === ">") strict.push(true);
          else strict.push(false);

          let args = ["tuple", lhs, rhs];
          while (
            this.token.token_type === ">" ||
            this.token.token_type === "GE"
          ) {
            if (this.token.token_type === ">") strict.push(true);
            else strict.push(false);

            this.advance();
            args.push(this.expression(params));
          }
          lhs = ["gts", args, strict];
        } else {
          lhs = [operation, lhs, rhs];
        }
      } else if (operation === "=") {
        lhs = ["=", lhs, rhs];

        // check for sequence of multiple =
        while (this.token.token_type === "=") {
          this.advance();
          lhs.push(this.expression(params));
        }
      } else {
        lhs = [operation, lhs, rhs];
      }
    }

    return lhs;
  }

  expression(params) {
    let plus_begin = false;
    if (this.token.token_type === "+") {
      plus_begin = true;
      this.advance();
    }

    let negative_begin = false;
    if (this.token.token_type === "-") {
      negative_begin = true;
      this.advance();
    }

    var lhs = this.term(params);

    if (negative_begin || plus_begin) {
      if (lhs === false) {
        return (plus_begin ? "+" : "") + (negative_begin ? "-" : "");
      } else if (
        typeof lhs === "string" &&
        [...lhs].every((x) => ["+", "-"].includes(x))
      ) {
        return (plus_begin ? "+" : "") + (negative_begin ? "-" : "") + lhs;
      }
    }

    if (lhs === false) {
      lhs = "\uff3f";
    }

    if (negative_begin) {
      if (lhs > 0) {
        lhs = -lhs;
      } else {
        lhs = ["-", lhs];
      }
    }

    if (plus_begin) {
      lhs = ["+", lhs];
    }

    while (
      ["+", "-", "UNION", "INTERSECT", "PERP", "PARALLEL"].includes(
        this.token.token_type,
      )
    ) {
      let operation = this.token.token_type.toLowerCase();
      let negative = false;
      let positive_then_negative = false;

      if (this.token.token_type === "-") {
        operation = "+";
        negative = true;
        this.advance();
      } else {
        this.advance();
        if (operation === "+" && this.token.token_type === "-") {
          negative = true;
          positive_then_negative = true;
          this.advance();
        }
      }
      let rhs = this.term(params);

      if (operation === "+") {
        if (
          rhs === false &&
          (typeof lhs === "number" || typeof lhs === "string")
        ) {
          if (positive_then_negative) {
            return lhs + "+-";
          } else if (negative) {
            return lhs + "-";
          } else {
            return lhs + "+";
          }
        } else if (
          typeof rhs === "string" &&
          [...rhs].every((x) => ["+", "-"].includes(x)) &&
          (typeof lhs === "number" || typeof lhs === "string")
        ) {
          if (positive_then_negative) {
            return lhs + "+-" + rhs;
          } else if (negative) {
            return lhs + "-" + rhs;
          } else {
            return lhs + "+" + rhs;
          }
        }
      }

      if (rhs === false) {
        rhs = "\uff3f";
      }

      if (negative) {
        if (rhs > 0) {
          rhs = -rhs;
        } else {
          rhs = ["-", rhs];
        }
      }

      lhs = [operation, lhs, rhs];
    }

    return lhs;
  }

  term(params) {
    var lhs = this.factor(params);

    var keepGoing = false;

    do {
      keepGoing = false;

      if (this.token.token_type === "*") {
        this.advance();
        if (lhs === false) {
          lhs = "\uff3f";
        }
        let rhs = this.factor(params);
        if (rhs === false) {
          rhs = "\uff3f";
        }
        lhs = ["*", lhs, rhs];
        keepGoing = true;
      } else if (this.token.token_type === "/") {
        this.advance();
        if (lhs === false) {
          lhs = "\uff3f";
        }
        let rhs = this.factor(params);
        if (rhs === false) {
          rhs = "\uff3f";
        }
        lhs = ["/", lhs, rhs];
        keepGoing = true;
      } else {
        // this is the one case where a | could indicate a closing absolute value
        let params2 = Object.assign({}, params);
        params2.allow_absolute_value_closing = true;
        let rhs = this.nonMinusFactor(params2);
        if (rhs !== false) {
          if (lhs === false) {
            lhs = "\uff3f";
          }
          lhs = ["*", lhs, rhs];
          keepGoing = true;
        }
      }
    } while (keepGoing);

    return this.convert_units_in_term(flatten(lhs));
  }

  convert_units_in_term(tree) {
    if (!Array.isArray(tree)) {
      return tree;
    }

    let operator = tree[0];
    let operands = tree.slice(1);

    if (operator === "*") {
      let n_ops = operands.length;
      for (let [ind, op] of operands.entries()) {
        let unit_object = this.units[op];
        if (unit_object) {
          if (unit_object.prefix && ind < n_ops - 1) {
            let post_unit_converted;
            if (ind === n_ops - 2) {
              post_unit_converted = operands[n_ops - 1];
            } else {
              post_unit_converted = this.convert_units_in_term([
                "*",
                ...operands.slice(ind + 1),
              ]);
            }
            let unit = op;
            if (unit_object.substitute) {
              unit = unit_object.substitute;
            }
            let unit_tree = ["unit", unit, post_unit_converted];

            if (ind === 0) {
              return unit_tree;
            } else {
              return [
                "*",
                ...operands
                  .slice(0, ind)
                  .map(this.convert_units_in_term.bind(this)),
                unit_tree,
              ];
            }
          } else if (!unit_object.prefix && ind > 0) {
            let unit = op;
            if (unit_object.substitute) {
              unit = unit_object.substitute;
            }
            let unit_tree;
            if (ind === 1) {
              unit_tree = ["unit", operands[0], unit];
            } else {
              unit_tree = [
                "unit",
                [
                  "*",
                  ...operands
                    .slice(0, ind)
                    .map(this.convert_units_in_term.bind(this)),
                ],
                unit,
              ];
            }
            if (ind === n_ops - 1) {
              return unit_tree;
            } else {
              return this.convert_units_in_term([
                "*",
                unit_tree,
                ...operands.slice(ind + 1),
              ]);
            }
          }
        }
      }
      return [operator, ...operands.map(this.convert_units_in_term.bind(this))];
    } else if (operator === "/") {
      return [
        "/",
        this.convert_units_in_term(operands[0]),
        this.convert_units_in_term(operands[1]),
      ];
    } else if (operator === "^") {
      let unit_object = this.units[operands[1]];
      if (unit_object && unit_object.isExponent) {
        let unit = operands[1];
        if (unit_object.substitute) {
          unit = unit_object.substitute;
        }
        let unit_tree = ["unit", operands[0], unit];
        return unit_tree;
      }
      return [operator, ...operands.map(this.convert_units_in_term.bind(this))];
    }

    return [operator, ...operands.map(this.convert_units_in_term.bind(this))];
  }

  factor(params) {
    if (this.token.token_text === "+") {
      this.advance();

      let factor = this.factor(params);
      if (factor === false) {
        return "+";
      } else if (
        typeof factor === "string" &&
        [...factor].every((x) => ["+", "-"].includes(x))
      ) {
        return "+" + factor;
      } else {
        return ["+", factor];
      }
    }

    if (this.token.token_type === "-") {
      this.advance();

      let factor = this.factor(params);
      if (factor > 0) {
        return -factor;
      } else if (factor === false) {
        return "-";
      } else if (
        typeof factor === "string" &&
        [...factor].every((x) => ["+", "-"].includes(x))
      ) {
        return "-" + factor;
      } else {
        return ["-", factor];
      }
    }

    let result = this.nonMinusFactor(params);

    if (result === false) {
      if (this.token.token_type === "PERP") {
        result = "perp";
        this.advance();
      }
    }

    return result;
  }

  nonMinusFactor(params) {
    var result = this.baseFactor(params);

    // allow arbitrary sequence of exponents, factorials or primes
    while (
      this.token.token_type === "^" ||
      this.token.token_type === "!" ||
      this.token.token_type === "'"
    ) {
      if (result === false) {
        result = "\uff3f";
      }
      if (this.token.token_type === "^") {
        this.advance();
        let superscript = this.get_subsuperscript(params);
        result = ["^", result, superscript];
      } else if (this.token.token_type === "!") {
        result = ["apply", "factorial", result];
        this.advance();
      } else {
        result = ["prime", result];
        this.advance();
      }
    }

    return result;
  }

  get_single_digit_as_number() {
    if (
      this.token.token_type === "NUMBER" &&
      this.token.token_text[0] !== "."
    ) {
      let num = Number(this.token.token_text[0]);
      if (this.token.token_text.length > 1) {
        this.lexer.unput(this.token.token_text.slice(1));
      }
      this.advance();

      return num;
    }

    return null;
  }

  get_subsuperscript({ parse_absolute_value }) {
    let num = this.get_single_digit_as_number();
    if (num !== null) {
      return num;
    } else if (["+", "-", "PERP"].includes(this.token.token_type)) {
      let subresult = this.token.token_type.toLowerCase();
      this.advance();
      return subresult;
    } else {
      let subresult = this.baseFactor({
        parse_absolute_value,
        in_subsuperscript_with_no_delimiters: true,
      });
      if (subresult === false) {
        subresult = "\uff3f";
      }
      return subresult;
    }
  }

  baseFactor({
    inside_absolute_value = 0,
    parse_absolute_value = true,
    allow_absolute_value_closing = false,
    in_subsuperscript_with_no_delimiters = false,
  } = {}) {
    var result = false;

    if (this.token.token_type === "BEGINENVIRONMENT") {
      let environment = /\\begin\s*{\s*([a-zA-Z0-9]+)\s*}/.exec(
        this.token.token_text,
      )[1];

      if (["matrix", "pmatrix", "bmatrix"].includes(environment)) {
        let n_rows = 0;
        let n_cols = 0;

        let all_rows = [];
        let row = [];
        let n_this_row = 0;
        let last_token = this.token.token_type;

        this.advance();

        while (this.token.token_type !== "ENDENVIRONMENT") {
          if (this.token.token_type === "&") {
            if (last_token === "&" || last_token === "LINEBREAK") {
              // blank entry, let entry be zero
              row.push(0);
              n_this_row += 1;
            }
            last_token = this.token.token_type;
            this.advance();
          } else if (this.token.token_type === "LINEBREAK") {
            if (last_token === "&" || last_token === "LINEBREAK") {
              // blank entry, let entry be zero
              row.push(0);
              n_this_row += 1;
            }
            all_rows.push(row);
            if (n_this_row > n_cols) n_cols = n_this_row;

            n_rows += 1;
            n_this_row = 0;
            row = [];
            last_token = this.token.token_type;
            this.advance();
          } else {
            if (
              last_token === "&" ||
              last_token === "LINEBREAK" ||
              "BEGINENVIRONMENT"
            ) {
              row.push(
                this.statement({ parse_absolute_value: parse_absolute_value }),
              );
              n_this_row += 1;
              last_token = " ";
            } else {
              throw new ParseError(
                "Invalid location of " + this.token.original_text,
                this.lexer.location,
              );
            }
          }
        }

        // token is ENDENVIRONMENT
        let environment2 = /\\end\s*{\s*([a-zA-Z0-9]+)\s*}/.exec(
          this.token.token_text,
        )[1];
        if (environment2 !== environment) {
          throw new ParseError(
            "Expecting \\end{" + environment + "}",
            this.lexer.location,
          );
        }

        // add last row
        if (last_token === "&") {
          // blank entry, let entry be zero
          row.push(0);
          n_this_row += 1;
        }
        all_rows.push(row);
        if (n_this_row > n_cols) n_cols = n_this_row;
        n_rows += 1;

        this.advance();

        // create matrix
        result = ["matrix", ["tuple", n_rows, n_cols]];
        let body = ["tuple"];
        for (let r of all_rows) {
          let new_row = ["tuple"].concat(r);
          for (let i = r.length; i < n_cols; i += 1) new_row.push(0);

          body.push(new_row);
        }
        result.push(body);

        return result;
      } else {
        throw new ParseError(
          "Unrecognized environment " + environment,
          this.lexer.location,
        );
      }
    }

    if (this.token.token_type === "NUMBER") {
      result = parseFloat(this.token.token_text);
      this.advance();
    } else if (this.token.token_type === "INFINITY") {
      result = Infinity;
      this.advance();
    } else if (this.token.token_type === "SQRT") {
      this.advance();

      let root = 2;
      if (this.token.token_type === "[") {
        this.advance();
        let parameter = this.statement({
          parse_absolute_value: parse_absolute_value,
        });
        if (this.token.token_type !== "]") {
          throw new ParseError("Expecting ]", this.lexer.location);
        }
        this.advance();

        root = parameter;
      }

      if (this.token.token_type !== "{") {
        throw new ParseError("Expecting {", this.lexer.location);
      }

      this.advance();
      let parameter = this.statement({
        parse_absolute_value: parse_absolute_value,
      });
      if (this.token.token_type !== "}") {
        throw new ParseError("Expecting }", this.lexer.location);
      }
      this.advance();

      if (root === 2) result = ["apply", "sqrt", parameter];
      else result = ["^", parameter, ["/", 1, root]];
    } else if (
      this.token.token_type === "VAR" ||
      this.token.token_type === "LATEXCOMMAND" ||
      this.token.token_type === "VARMULTICHAR"
    ) {
      result = this.token.token_text;

      if (this.token.token_type === "LATEXCOMMAND") {
        result = result.slice(1);
        if (
          !(
            this.appliedFunctionSymbols.includes(result) ||
            this.functionSymbols.includes(result) ||
            this.allowedLatexSymbols.includes(result) ||
            result in this.operatorSymbols
          )
        ) {
          throw new ParseError(
            "Unrecognized latex command " + this.token.original_text,
            this.lexer.location,
          );
        }
      } else if (this.token.token_type === "VARMULTICHAR") {
        // strip out name of variable from \operatorname command
        result = /\\operatorname\s*\{\s*([a-zA-Z0-9\+\-]+)\s*\}/.exec(
          result,
        )[1];
      }

      if (
        this.appliedFunctionSymbols.includes(result) ||
        this.functionSymbols.includes(result)
      ) {
        let must_apply = false;
        if (this.appliedFunctionSymbols.includes(result)) must_apply = true;

        if (["Re", "Im"].includes(result)) {
          result = result.toLowerCase();
        }

        this.advance();

        if (this.token.token_type === "_") {
          this.advance();

          let subscript = this.get_subsuperscript({ parse_absolute_value });

          if (result === "log" && subscript === 10) {
            result = "log10";
          } else {
            result = ["_", result, subscript];
          }
        }

        if (in_subsuperscript_with_no_delimiters) {
          if (must_apply) {
            result = ["apply", result, "\uff3f"];
          }
        } else {
          while (this.token.token_type === "'") {
            result = ["prime", result];
            this.advance();
          }

          while (this.token.token_type === "^") {
            this.advance();

            let superscript = this.get_subsuperscript({ parse_absolute_value });

            result = ["^", result, superscript];
          }

          if (this.token.token_type === "{" || this.token.token_type === "(") {
            let expected_right;
            if (this.token.token_type === "{") expected_right = "}";
            else expected_right = ")";

            this.advance();
            let parameters = this.statement_list();

            if (this.token.token_type !== expected_right) {
              throw new ParseError(
                "Expecting " + expected_right,
                this.lexer.location,
              );
            }
            this.advance();

            if (parameters[0] === "list") {
              // rename from list to tuple
              parameters[0] = "tuple";
            }

            result = ["apply", result, parameters];
          } else {
            // if was an applied function symbol,
            // cannot omit argument
            if (must_apply) {
              if (!this.allowSimplifiedFunctionApplication)
                throw new ParseError(
                  "Expecting ( after function",
                  this.lexer.location,
                );

              // if allow simplied function application
              // let the argument be the next factor
              let subresult = this.factor({
                parse_absolute_value: parse_absolute_value,
              });
              if (subresult === false) {
                subresult = "\uff3f";
              }
              result = ["apply", result, subresult];
            }
          }
        }

        return result; // have function so took care of subscript already
      } else if (result in this.operatorSymbols) {
        this.advance();

        if (result === "frac" && this.parseLeibnizNotation) {
          // determine if may be a derivative in Leibniz notation

          let original_state = this.return_state();

          let r = this.leibniz_notation();

          if (r) {
            // successfully parsed derivative in Leibniz notation, so return
            return r;
          } else {
            // didn't find a properly format Leibniz notation
            // so reset state and continue
            this.set_state(original_state);
          }
        }

        let operator = result;

        if (this.operatorSymbols[result].substitute) {
          operator = this.operatorSymbols[result].substitute;
        }

        let nargs = this.operatorSymbols[result].nargs;

        let remove_products = this.operatorSymbols[result].remove_products;

        let args = [];

        for (let i = 0; i < nargs; i++) {
          if (this.token.token_type === "{") {
            this.advance();

            let new_arg = this.statement({
              parse_absolute_value: parse_absolute_value,
            });

            if (
              remove_products &&
              Array.isArray(new_arg) &&
              new_arg[0] === "*"
            ) {
              args.push(...new_arg.slice(1));
            } else {
              args.push(new_arg);
            }

            if (this.token.token_type !== "}") {
              throw new ParseError("Expecting }", this.lexer.location);
            }

            this.advance();
          } else {
            // if don't have a "{", support only case with digits and characters

            let new_arg = this.get_single_digit_as_number();
            if (new_arg === null) {
              if (this.token.token_type === "VAR") {
                new_arg = this.token.token_text;
                this.advance();
              } else {
                throw new ParseError("Expecting {", this.lexer.location);
              }
            }

            args.push(new_arg);
          }
        }

        result = [operator, ...args];
      } else {
        this.advance();
      }
    } else if (
      this.token.token_type === "(" ||
      this.token.token_type === "[" ||
      this.token.token_type === "{" ||
      this.token.token_type === "LBRACE" ||
      this.token.token_type === "LANGLE"
    ) {
      let token_left = this.token.token_type;
      let expected_right, other_right;
      if (this.token.token_type === "(") {
        expected_right = ")";
        other_right = "]";
      } else if (this.token.token_type === "[") {
        expected_right = "]";
        other_right = ")";
      } else if (this.token.token_type === "{") {
        expected_right = "}";
        other_right = null;
      } else if (this.token.token_type === "LBRACE") {
        expected_right = "RBRACE";
        other_right = null;
      } else {
        expected_right = "RANGLE";
        other_right = null;
      }

      this.advance();
      result = this.statement_list();

      let n_elements = 1;
      if (result[0] === "list") {
        n_elements = result.length - 1;
      }

      if (this.token.token_type !== expected_right) {
        if (n_elements !== 2 || other_right === null) {
          throw new ParseError(
            "Expecting " + expected_right,
            this.lexer.location,
          );
        } else if (this.token.token_type !== other_right) {
          throw new ParseError("Expecting ) or ]", this.lexer.location);
        }

        // half-open interval
        result[0] = "tuple";
        result = ["interval", result];
        let closed;
        if (token_left === "(") closed = ["tuple", false, true];
        else closed = ["tuple", true, false];
        result.push(closed);
      } else if (n_elements >= 2) {
        if (token_left === "(" || token_left === "{") {
          result[0] = "tuple";
        } else if (token_left === "[") {
          result[0] = "array";
        } else if (token_left === "LBRACE") {
          result[0] = "set";
        } else {
          result[0] = "altvector";
        }
      } else if (token_left === "LBRACE") {
        if (result[0] === "|" || result[0] === ":") {
          result = ["set", result]; // set builder notation
        } else {
          result = ["set", result]; // singleton set
        }
      }

      this.advance();
    } else if (
      this.token.token_type[0] === "|" &&
      parse_absolute_value &&
      (inside_absolute_value === 0 ||
        !allow_absolute_value_closing ||
        this.token.token_type[1] === "L")
    ) {
      // allow the opening of an absolute value here if either
      // - we aren't already inside an absolute value (inside_absolute_value==0),
      // - we don't allows an absolute value closing, or
      // - the | was marked as a left
      // otherwise, skip this token so that will drop out the factor (and entire statement)
      // to where the absolute value will close

      inside_absolute_value += 1;

      this.advance();

      result = this.statement({ inside_absolute_value: inside_absolute_value });
      result = ["apply", "abs", result];

      if (this.token.token_type !== "|") {
        throw new ParseError("Expecting |", this.lexer.location);
      }

      this.advance();
    } else if (
      this.token.token_type === "LFLOOR" ||
      this.token.token_type === "LCEIL"
    ) {
      let expected_right, function_name;
      if (this.token.token_type === "LFLOOR") {
        expected_right = "RFLOOR";
        function_name = "floor";
      } else if (this.token.token_type === "LCEIL") {
        expected_right = "RCEIL";
        function_name = "ceil";
      }

      this.advance();

      result = this.statement();
      result = ["apply", function_name, result];

      if (this.token.token_type !== expected_right) {
        throw new ParseError(
          "Expecting " + expected_right,
          this.lexer.location,
        );
      }

      this.advance();
    } else if (this.token.token_type === "ANGLE") {
      this.advance();

      if (this.token.token_type === "{" || this.token.token_type === "(") {
        let expected_right;
        if (this.token.token_type === "{") expected_right = "}";
        else expected_right = ")";

        this.advance();
        let parameters = this.statement_list();

        if (this.token.token_type !== expected_right) {
          throw new ParseError(
            "Expecting " + expected_right,
            this.lexer.location,
          );
        }
        this.advance();

        if (parameters[0] === "list") {
          // remove list
          result = ["angle", ...parameters.slice(1)];
        } else if (parameters[0] === "*") {
          // not sure how to interpret this result, but leave it as an angle with the one argument
          result = ["angle", parameters];
        }
      } else {
        // have an angle not followed by { or (
        // look for non-minus factors and include them as arguments
        let args = [];

        let subresult = this.nonMinusFactor({ parse_absolute_value });

        while (subresult !== false) {
          args.push(subresult);
          subresult = this.nonMinusFactor({ parse_absolute_value });
        }

        if (args.length === 0) {
          result = "angle";
        } else {
          result = ["angle", ...args];
        }
      }
    } else if (this.token.token_type === "INT") {
      this.advance();

      let result = "int";

      if (this.token.token_type === "_") {
        this.advance();
        let subscript = this.get_subsuperscript({ parse_absolute_value });
        result = ["_", result, subscript];
      }

      if (this.token.token_type === "^") {
        this.advance();
        let superscript = this.get_subsuperscript({ parse_absolute_value });
        result = ["^", result, superscript];
      }

      let integrand = flatten(this.term({ parse_absolute_value }));

      if (Array.isArray(integrand) && integrand[0] === "*") {
        // look for consecutive factors of "d" followed by a string

        let ds = [];
        for (let i = 0; i < integrand.length - 1; i++) {
          let factor1 = integrand[i];
          if (factor1 === "d") {
            let factor2 = integrand[i + 1];
            integrand.splice(i, 2);
            ds.push(["d", factor2]);
            i--;
          }
        }
        integrand.push(...ds);
      }

      return ["apply", result, integrand];
    }

    if (this.token.token_type === "_") {
      if (result === false) {
        result = "\uff3f";
      }

      this.advance();

      let subscript = this.get_subsuperscript({ parse_absolute_value });

      result = ["_", result, subscript];
    }

    return result;
  }

  leibniz_notation() {
    // attempt to find and return a derivative in Leibniz notation
    // if unsuccessful, return false

    if (this.token.token_type !== "{") {
      return false;
    }
    this.advance();

    var result = this.token.token_text;

    let deriv_symbol = "";

    let n_deriv = 1;

    let var1 = "";
    let var2s = [];
    let var2_exponents = [];

    if (
      this.token.token_type === "LATEXCOMMAND" &&
      result.slice(1) === "partial"
    )
      deriv_symbol = "∂";
    else if (this.token.token_type === "VAR" && result === "d")
      deriv_symbol = "d";
    else return false;

    // since have just a d or ∂
    // one option is that have a ^ followed by an integer next possibly in {}

    this.advance();

    if (this.token.token_type === "^") {
      // so far have d or ∂ followed by ^
      // must be followed by an integer
      this.advance();

      let in_braces = false;
      if (this.token.token_type === "{") {
        in_braces = true;

        this.advance();
      }

      if (this.token.token_type !== "NUMBER") {
        return false;
      }

      n_deriv = parseFloat(this.token.token_text);
      if (!Number.isInteger(n_deriv)) {
        return false;
      }

      // found integer,

      // if in braces, require }
      if (in_braces) {
        this.advance();

        if (this.token.token_type !== "}") {
          return false;
        }
      }

      this.advance();
    }

    // since have a d or ∂, optionally followed by ^ and integer
    // next we must have:
    // a VAR, a VARMULTICHAR, or a LATEXCOMMAND that is in allowedLatexSymbols

    if (this.token.token_type === "VAR") var1 = this.token.token_text;
    else if (this.token.token_type === "VARMULTICHAR") {
      // strip out name of variable from \operatorname command
      var1 = /\\operatorname\s*\{\s*([a-zA-Z0-9\+\-]+)\s*\}/.exec(
        this.token.token_text,
      )[1];
    } else if (this.token.token_type === "LATEXCOMMAND") {
      result = this.token.token_text.slice(1);
      if (this.allowedLatexSymbols.includes(result)) var1 = result;
      else return false;
    }

    // Finished numerator.
    // Next need a } and {

    this.advance();

    if (this.token.token_type !== "}") {
      return false;
    }

    this.advance();

    if (this.token.token_type !== "{") {
      return false;
    } else {
      this.advance();
    }

    // In denominator now
    // find sequence of
    // derivative symbol followed by
    // - a VAR, a VARMULTICHAR, or a LATEXCOMMAND that is in allowedLatexSymbols
    // optionally followed by a ^ and an integer
    // End when sum of exponents meets or exceeds n_deriv

    let exponent_sum = 0;

    while (true) {
      // next must be
      // - a VAR equal to deriv_symbol="d" or \partial when deriv_symbol = "∂"

      if (
        !(
          (deriv_symbol === "d" &&
            this.token.token_type === "VAR" &&
            this.token.token_text === "d") ||
          (deriv_symbol === "∂" &&
            this.token.token_type === "LATEXCOMMAND" &&
            this.token.token_text.slice(1) === "partial")
        )
      ) {
        return false;
      }

      // followed by
      // - a VAR, a VARMULTICHAR, or a LATEXCOMMAND that is in allowedLatexSymbols

      this.advance();

      if (this.token.token_type === "VAR") var2s.push(this.token.token_text);
      else if (this.token.token_type === "VARMULTICHAR") {
        // strip out name of variable from \operatorname command
        var2s.push(
          /\\operatorname\s*\{\s*([a-zA-Z0-9\+\-]+)\s*\}/.exec(
            this.token.token_text,
          )[1],
        );
      } else if (this.token.token_type === "LATEXCOMMAND") {
        let r = this.token.token_text.slice(1);
        if (this.allowedLatexSymbols.includes(r)) var2s.push(r);
        else {
          return false;
        }
      } else {
        return false;
      }
      // have derivative and variable, now check for optional ^ followed by number

      let this_exponent = 1;

      this.advance();

      if (this.token.token_type === "^") {
        this.advance();

        let in_braces = false;
        if (this.token.token_type === "{") {
          in_braces = true;

          this.advance();
        }

        if (this.token.token_type !== "NUMBER") {
          return false;
        }

        this_exponent = parseFloat(this.token.token_text);
        if (!Number.isInteger(this_exponent)) {
          return false;
        }

        // if in braces, require }
        if (in_braces) {
          this.advance();

          if (this.token.token_type !== "}") {
            return false;
          }
        }

        this.advance();
      }

      var2_exponents.push(this_exponent);
      exponent_sum += this_exponent;

      if (exponent_sum > n_deriv) {
        return false;
      }

      // possibly found derivative
      if (exponent_sum === n_deriv) {
        // next token must be a }
        if (this.token.token_type !== "}") {
          return false;
        }

        // found derivative!

        this.advance();

        let result_name = "derivative_leibniz";
        if (deriv_symbol === "∂") result_name = "partial_" + result_name;

        result = [result_name];

        if (n_deriv === 1) result.push(var1);
        else result.push(["tuple", var1, n_deriv]);

        let r2 = [];
        for (let i = 0; i < var2s.length; i += 1) {
          if (var2_exponents[i] === 1) r2.push(var2s[i]);
          else r2.push(["tuple", var2s[i], var2_exponents[i]]);
        }
        r2 = ["tuple"].concat(r2);

        result.push(r2);

        return result;
      }
    }
  }
}

export default latexToAst;
