import {ParseError} from './error';
import lexer from './lexer';
import flatten from './flatten';

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


// UPDATETHIS: Is this grammar still correct?

/* Grammar:

   statement_list =
    statement_list ',' statement |
    statement

   statement =
    statement 'OR' statement2 |
    statement2

   statement2 =
    statement2 'AND' relation |
    relation

   relation =
    'NOT' relation |
    '!' relation |
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
    '(' statement ',' statement ']' |
    '[' statement ',' statement ')' |
    number |
    variable |
    modified_function '(' statement_list ')' |
    modified_applied_function '(' statement_list ')' |
    modified_function |
    modified_applied_function factor |
    baseFactor '_' baseFactor |
    *** modified_applied_function factor
        allowed only if allowSimplifiedFunctionApplication==true

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
    nonMinusFactor |
    '|' statement '|'

*/


const text_rules = [
  ['[0-9]+(\\.[0-9]+)?(E[+\\-]?[0-9]+)?', 'NUMBER'],
  ['\\.[0-9]+(E[+\\-]?[0-9]+)?', 'NUMBER'],
  ['\\*\\*', '^'],
  ['\\*', '*'], // there is some variety in multiplication symbols
  ['\\xB7', '*'], // '·'
  ['\u00B7', '*'], // '·'
  ['\u2022', '*'], // '•'
  ['\u22C5', '*'], // '⋅'
  ['\u00D7', '*'], // '×'
  ['\/', '/'],
  ['-', '-'], // there is quite some variety with unicode hyphens
  ['\u058A', '-'], // '֊'
  ['\u05BE', '-'], // '־'
  ['\u1806', '-'], // '᠆'
  ['\u2010', '-'], // '‐'
  ['\u2011', '-'], // '‑'
  ['\u2012', '-'], // '‒'
  ['\u2013', '-'], // '–'
  ['\u2014', '-'], // '—'
  ['\u2015', '-'], // '―'
  ['\u207B', '-'], // '⁻'
  ['\u208B', '-'], // '₋'
  ['\u2212', '-'], // '−'
  ['\u2E3A', '-'], // '⸺'
  ['\u2E3B', '-'], // '⸻'
  ['\uFE58', '-'], // '﹘'
  ['\uFE63', '-'], // '﹣'
  ['\uFF0D', '-'], // '－'
  ['\\+', '+'],
  ['\\^', '^'], // a few ways to denote exponentiation
  ['\u2038', '^'], // '‸'
  ['\u028C', '^'], // 'ʌ'
  ['\\|', '|'],
  ['\\(', '('],
  ['\\)', ')'],
  ['\\[', '['],
  ['\\]', ']'],
  ['\\{', '{'],
  ['\\}', '}'],
  [',', ','],

  ['\u03B1', 'VARMULTICHAR', 'alpha'], // 'α'
  ['\u03B2', 'VARMULTICHAR', 'beta'], // 'β'
  ['\u03D0', 'VARMULTICHAR', 'beta'], // 'ϐ'
  ['\u0393', 'VARMULTICHAR', 'Gamma'], // 'Γ'
  ['\u03B3', 'VARMULTICHAR', 'gamma'], // 'γ'
  ['\u0394', 'VARMULTICHAR', 'Delta'], // 'Δ'
  ['\u03B4', 'VARMULTICHAR', 'delta'], // 'δ'
  ['\u03B5', 'VARMULTICHAR', 'epsilon'], // 'ε' should this be varepsilon?
  ['\u03F5', 'VARMULTICHAR', 'epsilon'], // 'ϵ'
  ['\u03B6', 'VARMULTICHAR', 'zeta'], // 'ζ'
  ['\u03B7', 'VARMULTICHAR', 'eta'], // 'η'
  ['\u0398', 'VARMULTICHAR', 'Theta'], // 'Θ'
  ['\u03F4', 'VARMULTICHAR', 'Theta'], // 'ϴ'
  ['\u03B8', 'VARMULTICHAR', 'theta'], // 'θ'
  ['\u1DBF', 'VARMULTICHAR', 'theta'], // 'ᶿ'
  ['\u03D1', 'VARMULTICHAR', 'theta'], // 'ϑ'
  ['\u03B9', 'VARMULTICHAR', 'iota'], // 'ι'
  ['\u03BA', 'VARMULTICHAR', 'kappa'], // 'κ'
  ['\u039B', 'VARMULTICHAR', 'Lambda'], // 'Λ'
  ['\u03BB', 'VARMULTICHAR', 'lambda'], // 'λ'
  ['\u03BC', 'VARMULTICHAR', 'mu'], // 'μ'
  ['\u00B5', 'VARMULTICHAR', 'mu'], // 'µ' should this be micro?
  ['\u03BD', 'VARMULTICHAR', 'nu'], // 'ν'
  ['\u039E', 'VARMULTICHAR', 'Xi'], // 'Ξ'
  ['\u03BE', 'VARMULTICHAR', 'xi'], // 'ξ'
  ['\u03A0', 'VARMULTICHAR', 'Pi'], // 'Π'
  ['\u03C0', 'VARMULTICHAR', 'pi'], // 'π'
  ['\u03D6', 'VARMULTICHAR', 'pi'], // 'ϖ' should this be varpi?
  ['\u03C1', 'VARMULTICHAR', 'rho'], // 'ρ'
  ['\u03F1', 'VARMULTICHAR', 'rho'], // 'ϱ' should this be varrho?
  ['\u03A3', 'VARMULTICHAR', 'Sigma'], // 'Σ'
  ['\u03C3', 'VARMULTICHAR', 'sigma'], // 'σ'
  ['\u03C2', 'VARMULTICHAR', 'sigma'], // 'ς' should this be varsigma?
  ['\u03C4', 'VARMULTICHAR', 'tau'], // 'τ'
  ['\u03A5', 'VARMULTICHAR', 'Upsilon'], // 'Υ'
  ['\u03C5', 'VARMULTICHAR', 'upsilon'], // 'υ'
  ['\u03A6', 'VARMULTICHAR', 'Phi'], // 'Φ'
  ['\u03C6', 'VARMULTICHAR', 'phi'], // 'φ' should this be varphi?
  ['\u03D5', 'VARMULTICHAR', 'phi'], // 'ϕ'
  ['\u03A8', 'VARMULTICHAR', 'Psi'], // 'Ψ'
  ['\u03C8', 'VARMULTICHAR', 'psi'], // 'ψ'
  ['\u03A9', 'VARMULTICHAR', 'Omega'], // 'Ω'
  ['\u03C9', 'VARMULTICHAR', 'omega'], // 'ω'


  ['oo\\b', 'INFINITY'],
  ['OO\\b', 'INFINITY'],
  ['infty\\b', 'INFINITY'],
  ['infinity\\b', 'INFINITY'],
  ['Infinity\\b', 'INFINITY'],
  ['\u221E', 'INFINITY'], // '∞'

  ['\u212F', 'VAR', 'e'], // 'ℯ'

  ['\u2660', 'VARMULTICHAR', 'spade'], // '♠'
  ['\u2661', 'VARMULTICHAR', 'heart'], // '♡'
  ['\u2662', 'VARMULTICHAR', 'diamond'], // '♢'
  ['\u2663', 'VARMULTICHAR', 'club'], // '♣'
  ['\u2605', 'VARMULTICHAR', 'bigstar'], // '★'
  ['\u25EF', 'VARMULTICHAR', 'bigcirc'], // '◯'
  ['\u25CA', 'VARMULTICHAR', 'lozenge'], // '◊'
  ['\u25B3', 'VARMULTICHAR', 'bigtriangleup'], // '△'
  ['\u25BD', 'VARMULTICHAR', 'bigtriangledown'], // '▽'
  ['\u29EB', 'VARMULTICHAR', 'blacklozenge'], // '⧫'
  ['\u25A0', 'VARMULTICHAR', 'blacksquare'], // '■'
  ['\u25B2', 'VARMULTICHAR', 'blacktriangle'], // '▲'
  ['\u25BC', 'VARMULTICHAR', 'blacktriangledown'], //'▼'
  ['\u25C0', 'VARMULTICHAR', 'blacktriangleleft'], // '◀'
  ['\u25B6', 'VARMULTICHAR', 'blacktriangleright'], // '▶'
  ['\u25A1', 'VARMULTICHAR', 'Box'], // '□'
  ['\u2218', 'VARMULTICHAR', 'circ'], // '∘'
  ['\u22C6', 'VARMULTICHAR', 'star'], // '⋆'

  ['and\\b', 'AND'],
  ['\\&\\&?', 'AND'],
  ['\u2227', 'AND'], // '∧'

  ['or\\b', 'OR'],
  ['\u2228', 'OR'], // '∨'

  ['not\\b', 'NOT'],
  ['\u00ac', 'NOT'], // '¬'

  ['=', '='],
  ['\u1400', '='], // '᐀'
  ['\u30A0', '='], // '゠'
  ['!=', 'NE'],
  ['\u2260', 'NE'], // '≠'
  ['<=', 'LE'],
  ['\u2264', 'LE'], // '≤'
  ['>=', 'GE'],
  ['\u2265', 'GE'], // '≥'
  ['<', '<'],
  ['>', '>'],

  ['elementof\\b', 'IN'],
  ['\u2208', 'IN'], // '∈'

  ['notelementof\\b', 'NOTIN'],
  ['\u2209', 'NOTIN'], //'∉'

  ['containselement\\b', 'NI'],
  ['\u220B', 'NI'], // '∋'

  ['notcontainselement\\b', 'NOTNI'],
  ['\u220C', 'NOTNI'], // '∌'

  ['subset\\b', 'SUBSET'],
  ['\u2282', 'SUBSET'], // '⊂'

  ['notsubset\\b', 'NOTSUBSET'],
  ['\u2284', 'NOTSUBSET'], // '⊄'

  ['superset\\b', 'SUPERSET'],
  ['\u2283', 'SUPERSET'], // '⊃'

  ['notsuperset\\b', 'NOTSUPERSET'],
  ['\u2285', 'NOTSUPERSET'], //'⊅'

  ['union\\b', 'UNION'],
  ['\u222A', 'UNION'], // '∪'

  ['intersect\\b', 'INTERSECT'],
  ['\u2229', 'INTERSECT'], //'∩'

  ['!', '!'],
  ['\'', '\''],
  ['_', '_'],

  ['[a-zA-Z∂][a-zA-Z∂0-9]*', 'VAR'],  // include ∂ in VAR
];


// defaults for parsers if not overridden by context

// if true, allowed applied functions to omit parentheses around argument
// if false, omitting parentheses will lead to a Parse Error
const allowSimplifiedFunctionApplicationDefault = true;

// if true, split multicharacter symbols into a product of letters
const splitSymbolsDefault = true;

// symbols that won't be split into a product of letters if splitSymbols==true
const unsplitSymbolsDefault = ['alpha', 'beta', 'gamma', 'Gamma', 'delta', 'Delta', 'epsilon', 'zeta', 'eta', 'theta', 'Theta', 'iota', 'kappa', 'lambda', 'Lambda', 'mu', 'nu', 'xi', 'Xi', 'pi', 'Pi', 'rho', 'sigma', 'Sigma', 'tau', 'Tau', 'upsilon', 'Upsilon', 'phi', 'Phi', 'chi', 'psi', 'Psi', 'omega', 'Omega' ];


// Applied functions must be given an argument so that
// they are applied to the argument
const appliedFunctionSymbolsDefault = ["abs", "exp", "log", "ln", "log10", "sign", "sqrt", "erf", "acos", "acosh", "acot", "acoth", "acsc", "acsch", "asec", "asech", "asin", "asinh", "atan", "atanh", "cos", "cosh", "cot", "coth", "csc", "csch", "sec", "sech", "sin", "sinh", "tan", "tanh", 'arcsin', 'arccos', 'arctan', 'arccsc', 'arcsec', 'arccot', 'cosec', 'arg'];

// Functions could have an argument, in which case they are applied
// or, if they don't have an argument in parentheses, then they are treated
// like a variable, except that trailing ^ and ' have higher precedence
const functionSymbolsDefault = ['f', 'g'];

// Parse Leibniz notation
const parseLeibnizNotationDefault = true;


class textToAst {
  constructor({
    allowSimplifiedFunctionApplication = allowSimplifiedFunctionApplicationDefault,
    splitSymbols = splitSymbolsDefault,
    unsplitSymbols = unsplitSymbolsDefault,
    appliedFunctionSymbols = appliedFunctionSymbolsDefault,
    functionSymbols = functionSymbolsDefault,
    parseLeibnizNotation = parseLeibnizNotationDefault,
  } = {}) {
    this.allowSimplifiedFunctionApplication = allowSimplifiedFunctionApplication;
    this.splitSymbols = splitSymbols;
    this.unsplitSymbols = unsplitSymbols;
    this.appliedFunctionSymbols = appliedFunctionSymbols;
    this.functionSymbols = functionSymbols;
    this.parseLeibnizNotation = parseLeibnizNotation;

    this.lexer = new lexer(text_rules);

  }

  advance(params) {
    this.token = this.lexer.advance(params);
    if (this.token.token_type == 'INVALID') {
      throw new ParseError("Invalid symbol '" + this.token.original_text + "'",
        this.lexer.location);
    }
  }

  convert(input) {

    this.lexer.set_input(input);
    this.advance();

    var result = this.statement_list();

    if (this.token.token_type != 'EOF') {
      throw new ParseError("Invalid location of '" + this.token.original_text + "'",
        this.lexer.location);
    }

    return flatten(result);

  }


  statement_list() {

    var list = [this.statement()];

    while (this.token.token_type == ",") {
      this.advance();
      list.push(this.statement());
    }

    if (list.length > 1)
      list = ['list'].concat(list);
    else
      list = list[0];

    return list;
  }

  statement() {

    var lhs = this.statement2();

    while (this.token.token_type == 'OR') {

      var operation = this.token.token_type.toLowerCase();

      this.advance();

      var rhs = this.statement2();

      lhs = [operation, lhs, rhs];
    }

    return lhs;
  }

  statement2() {
    // split AND into second statement to give higher precedence than OR

    var lhs = this.relation();

    while (this.token.token_type == 'AND') {

      var operation = this.token.token_type.toLowerCase();

      this.advance();

      var rhs = this.relation();

      lhs = [operation, lhs, rhs];
    }

    return lhs;
  }


  relation() {

    if (this.token.token_type == 'NOT' || this.token.token_type == '!') {
      this.advance();
      return ['not', this.relation()];
    }

    var lhs = this.expression();

    while ((this.token.token_type == '=') || (this.token.token_type == 'NE') ||
      (this.token.token_type == '<') || (this.token.token_type == '>') ||
      (this.token.token_type == 'LE') || (this.token.token_type == 'GE') ||
      (this.token.token_type == 'IN') || (this.token.token_type == 'NOTIN') ||
      (this.token.token_type == 'NI') || (this.token.token_type == 'NOTNI') ||
      (this.token.token_type == 'SUBSET') || (this.token.token_type == 'NOTSUBSET') ||
      (this.token.token_type == 'SUPERSET') || (this.token.token_type == 'NOTSUPERSET')) {

      var operation = this.token.token_type.toLowerCase();

      var inequality_sequence = 0;

      if ((this.token.token_type == '<') || (this.token.token_type == 'LE')) {
        inequality_sequence = -1;
      } else if ((this.token.token_type == '>') || (this.token.token_type == 'GE')) {
        inequality_sequence = 1;
      }

      this.advance();
      var rhs = this.expression();

      if (inequality_sequence == -1) {
        if ((this.token.token_type == '<') || this.token.token_type == 'LE') {
          // sequence of multiple < or <=
          var strict = ['tuple'];
          if (operation == '<')
            strict.push(true)
          else
            strict.push(false)

          var args = ['tuple', lhs, rhs];
          while ((this.token.token_type == '<') || this.token.token_type == 'LE') {
            if (this.token.token_type == '<')
              strict.push(true)
            else
              strict.push(false)

            this.advance();
            args.push(this.expression());
          }
          lhs = ['lts', args, strict];
        } else {
          lhs = [operation, lhs, rhs];
        }

      } else if (inequality_sequence == 1) {
        if ((this.token.token_type == '>') || this.token.token_type == 'GE') {
          // sequence of multiple > or >=
          var strict = ['tuple'];
          if (operation == '>')
            strict.push(true)
          else
            strict.push(false)

          var args = ['tuple', lhs, rhs];
          while ((this.token.token_type == '>') || this.token.token_type == 'GE') {
            if (this.token.token_type == '>')
              strict.push(true)
            else
              strict.push(false)

            this.advance();
            args.push(this.expression());
          }
          lhs = ['gts', args, strict];
        } else {
          lhs = [operation, lhs, rhs];
        }

      } else if (operation === '=') {
        lhs = ['=', lhs, rhs];

        // check for sequence of multiple =
        while (this.token.token_type === '=') {
          this.advance();
          lhs.push(this.expression());
        }
      } else {

        lhs = [operation, lhs, rhs];
      }

    }

    return lhs;
  }


  expression() {
    if (this.token.token_type == '+')
      this.advance();

    var lhs = this.term();
    while ((this.token.token_type == '+') || (this.token.token_type == '-')
	   || (this.token.token_type == 'UNION') ||
      (this.token.token_type == 'INTERSECT')) {

      var operation = this.token.token_type.toLowerCase();
      var negative = false;

      if (this.token.token_type == '-') {
        operation = '+';
        negative = true;
        this.advance();
      } else {
        this.advance();
      }
      var rhs = this.term();
      if (negative) {
        rhs = ['-', rhs];
      }

      lhs = [operation, lhs, rhs];
    }

    return lhs;
  }


  term() {
    var lhs = this.factor();

    var keepGoing = false;

    do {
      keepGoing = false;

      if (this.token.token_type == '*') {
        this.advance();
        lhs = ['*', lhs, this.factor()];
        keepGoing = true;
      } else if (this.token.token_type == '/') {
        this.advance();
        lhs = ['/', lhs, this.factor()];
        keepGoing = true;
      } else {
        var rhs = this.nonMinusFactor();
        if (rhs !== false) {
          lhs = ['*', lhs, rhs];
          keepGoing = true;
        }
      }
    } while (keepGoing);

    return lhs;
  }


  factor() {

    if (this.token.token_type == '-') {
      this.advance();
      return ['-', this.factor()];
    }

    if (this.token.token_type == '|') {
      this.advance();

      var result = this.statement();
      result = ['apply', 'abs', result];

      if (this.token.token_type != '|') {
        throw new ParseError('Expected |', this.lexer.location);
      }
      this.advance();
      return result;
    }

    var result = this.nonMinusFactor();

    if (result === false) {
      if (this.token.token_type == "EOF") {
        throw new ParseError("Unexpected end of input", this.lexer.location);
      } else {
        throw new ParseError("Invalid location of '" + this.token.original_text + "'",
          this.lexer.location);
      }
    } else {
      return result;
    }

  }

  nonMinusFactor() {

    var result = this.baseFactor();

    // allow arbitrary sequence of factorials
    if (this.token.token_type == '!' || this.token.token_type == "'") {
      if (result === false)
        throw new ParseError("Invalid location of " + this.token.token_type,
          this.lexer.location);
      while (this.token.token_type == '!' || this.token.token_type == "'") {
        if (this.token.token_type == '!')
          result = ['apply', 'factorial', result]
        else
          result = ['prime', result];
        this.advance();
      }
    }

    if (this.token.token_type == '^') {
      if (result === false) {
        throw new ParseError("Invalid location of ^", this.lexer.location);
      }
      this.advance();
      return ['^', result, this.factor()];
    }

    return result;
  }


  baseFactor() {
    var result = false;

    if (this.token.token_type == 'NUMBER') {
      result = parseFloat(this.token.token_text);
      this.advance();
    } else if (this.token.token_type == 'INFINITY') {
      result = 'infinity';
      this.advance();
    } else if (this.token.token_type == 'VAR' || this.token.token_type == 'VARMULTICHAR') {
      result = this.token.token_text;

      if (this.appliedFunctionSymbols.includes(result) ||
        this.functionSymbols.includes(result)) {
        var must_apply = false
        if (this.appliedFunctionSymbols.includes(result))
          must_apply = true;

        result = result.toLowerCase();
        this.advance();

        if (this.token.token_type == '_') {
          this.advance();
          var subresult = this.baseFactor();

          // since baseFactor could return false, must check
          if (subresult === false) {
            if (this.token.token_type == "EOF") {
              throw new ParseError("Unexpected end of input",
                this.lexer.location);
            } else {
              throw new ParseError("Invalid location of '" + this.token.original_text +
                "'", this.lexer.location);
            }
          }
          result = ['_', result, subresult];
        }

        var n_primes = 0;
        while (this.token.token_type == "'") {
          n_primes += 1;
          result = ['prime', result];
          this.advance();
        }

        if (this.token.token_type == '^') {
          this.advance();
          result = ['^', result, this.factor()];
        }

        if (this.token.token_type == '(') {
          this.advance();
          var parameters = this.statement_list();

          if (this.token.token_type != ')') {
            throw new ParseError('Expected )', this.lexer.location);
          }
          this.advance();

          if (parameters[0] == 'list') {
            // rename from list to tuple
            parameters[0] = 'tuple';
          }

          result = ['apply', result, parameters];
        } else {
          // if was an applied function symbol,
          // cannot omit argument
          if (must_apply) {
            if (!this.allowSimplifiedFunctionApplication)
              throw new ParseError("Expected ( after function",
                this.lexer.location);

            // if allow simplied function application
            // let the argument be the next factor
            result = ['apply', result, this.factor()];
          }
        }
      } else {

	// determine if may be a derivative in Leibniz notation
	if(this.parseLeibnizNotation
	   && this.token.token_type == 'VAR' && (result[0]=="d" || result[0] == "∂")
	   && (result.length==1 || (result.length==2 && /[a-zA-Z]/.exec(result[1])))) {

	  // found one of these two possibilities for start of derivative are
	  // - dx or ∂x (no space, x is a single letter)
	  // - d or ∂

	  let deriv_symbol = result[0];

	  let derivative_possible = true;
	  let token_list = [this.token.original_text];

	  let n_deriv = 1;

	  let var1 = "";
	  let var2s = [];
	  let var2_exponents = [];

	  if(result.length == 2)
	    var1 = result[1];
	  else { // result is length 1

	    // since have just a d or ∂
	    // must be followed by a ^ or a VARMULTICHAR
	    this.advance({remove_initial_space: false});
	    token_list.push(this.token.original_text);

	    if(this.token.token_type == 'VARMULTICHAR') {
	      var1 = this.token.token_text;
	    }

	    else {
	      // since not VARMULTICHAR, must be a ^ next
	      if(this.token.token_type != '^') {
		derivative_possible = false;
	      }
	      else {
		// so far have d or ∂ followed by ^
		// must be followed by an integer
		this.advance({remove_initial_space: false});
		token_list.push(this.token.original_text);

		if(this.token.token_type != 'NUMBER') {
		  derivative_possible = false;
		}
		else {
		  n_deriv = parseFloat(this.token.token_text);
		  if(!Number.isInteger(n_deriv)) {
		    derivative_possible = false;
		  }
		  else {
		    // see if next character is single character
		    this.advance({remove_initial_space: false});
		    token_list.push(this.token.original_text);

		    // either a single letter from VAR
		    // or a VARMULTICHAR
		    if((this.token.token_type=='VAR' && (/^[a-zA-Z]$/.exec(this.token.token_text)))
		       || this.token.token_type == 'VARMULTICHAR') {
		      var1 = this.token.token_text;
		    }
		    else {
		      derivative_possible=false;
		    }
		  }
		}
	      }
	    }
	  }

	  // next character must be a /
	  if(derivative_possible) {
	    // allow a space this time, but store in token_list
	    this.advance({remove_initial_space: false});
	    token_list.push(this.token.original_text);
	    if(this.token.token_type == "SPACE") {
	      this.advance({remove_initial_space: false});
	      token_list.push(this.token.original_text);
	    }

	    if(this.token.token_type != '/')
	      derivative_possible = false;
	    else {

	      // find sequence of
	      // derivative symbol followed by a single character or VARMULTICHAR (with no space)
	      // optionally followed by a ^ and an integer (with no spaces)
	      // (with spaces allowed between elements of sequence)
	      // End when sum of exponents meets or exceeds n_deriv

	      let exponent_sum = 0;

	      this.advance({remove_initial_space: false});
	      token_list.push(this.token.original_text);

	      // allow space just after the /
	      if(this.token.token_type == "SPACE") {
		this.advance({remove_initial_space: false});
		token_list.push(this.token.original_text);
	      }

	      while(true) {

		// next must either be
		// - a VAR whose first character matches derivative symbol
		//   and whose second character is a letter, or
		// - a single character VAR that matches derivative symbol
		//   which must be followed by a VARMULTICHAR (with no space)


		if(this.token.token_type != 'VAR'|| this.token.token_text[0] !== deriv_symbol) {
		  derivative_possible = false
		  break;
		}


		if(this.token.token_text.length > 2) {
		  // Put extra characters back on lexer
		  this.lexer.unput(this.token.token_text.slice(2));

		  // keep just two character token
		  this.token.token_text = this.token.token_text.slice(0,2);

		  // show that only two characters were taken
		  token_list[token_list.length-1] = this.token.token_text;
		}


		let token_text = this.token.token_text;

		// derivative symbol and variable together
		if(token_text.length == 2) {
		  if(/[a-zA-Z]/.exec(token_text[1]))
		    var2s.push(token_text[1])
		  else {
		    derivative_possible = false;
		    break;
		  }
		}
		else { // token text was just the derivative symbol
		  this.advance({remove_initial_space: false});
		  token_list.push(this.token.original_text);

		  if(this.token.token_type !== 'VARMULTICHAR') {
		    derivative_possible = false;
		    break;
		  }
		  else
		    var2s.push(this.token.token_text);
		}


		// have derivative and variable, now check for optional ^ followed by number

		let this_exponent = 1;

		this.advance({remove_initial_space: false});
		token_list.push(this.token.original_text);

		if(this.token.token_type === '^') {

		  this.advance({remove_initial_space: false});
		  token_list.push(this.token.original_text);

		  if(this.token.token_type != 'NUMBER') {
		    derivative_possible = false;
		    break;
		  }

		  this_exponent = parseFloat(this.token.token_text);
		  if(!Number.isInteger(this_exponent)) {
		    derivative_possible = false;
		    break;
		  }

		  this.advance({remove_initial_space: false});
		  token_list.push(this.token.original_text);

		}

		var2_exponents.push(this_exponent);
		exponent_sum += this_exponent;

		if(exponent_sum > n_deriv) {
		  derivative_possible= false;
		  break;
		}

		// possibly found derivative
		if(exponent_sum == n_deriv) {

		  // check to make sure next token isn't another VAR or VARMULTICHAR
		  // in this case, the derivative isn't separated from what follows
		  if(this.token.token_type == "VAR" || this.token.token_type == "VARMULTICHAR") {
		    derivative_possible = false;
		    break;
		  }

		  // found derivative!

		  // if last token was a space advance to next non-space token
		  if(this.token.token_type=="SPACE")
		    this.advance();

		  let result_name = "derivative_leibniz"
		  if(deriv_symbol == "∂")
		    result_name = "partial_" + result_name;

		  result = [result_name];

		  if(n_deriv == 1)
		    result.push(var1);
		  else
		    result.push(["tuple", var1, n_deriv]);

		  let r2 = []
		  for(let i=0; i<var2s.length; i+=1) {
		    if(var2_exponents[i] == 1)
		      r2.push(var2s[i])
		    else
		      r2.push(["tuple", var2s[i], var2_exponents[i]]);
		  }
		  r2 = ["tuple"].concat(r2);

		  result.push(r2);

		  return result;

		}
	      }
	    }
	  }

	  // failed to get derivative, push back extra tokens on lexer
	  for(let token of token_list.reverse()) {
	    this.lexer.unput(token);
	  }
	  this.advance();

	} // end of checking if Leibniz derivative

        // determine if should split text into single letter factors
        var split = this.splitSymbols;

        if (split) {
          if (this.token.token_type == 'VARMULTICHAR' ||
            this.unsplitSymbols.includes(result) ||
            result.length == 1) {
            split = false;
          } else if (result.match(/[\d]/g)) {
            // don't split if has a number in it
            split = false;
          }
        }

        if (split) {
          // so that each character gets processed separately
          // put all characters back on the input
          // but with spaces
          // then process again

          for (var i = result.length - 1; i >= 0; i--) {
            this.lexer.unput(" ");
            this.lexer.unput(result[i]);
          }
          this.advance();

          return this.baseFactor();
        } else {
          this.advance();
        }
      }
    } else if (this.token.token_type == '(' || this.token.token_type == '[' ||
      this.token.token_type == '{') {
      var token_left = this.token.token_type;
      var expected_right, other_right;
      if (this.token.token_type == '(') {
        expected_right = ')';
        other_right = ']';
      } else if (this.token.token_type == '[') {
        expected_right = ']';
        other_right = ')';
      } else {
        expected_right = '}';
        other_right = null;
      }

      this.advance();
      result = this.statement_list();

      var n_elements = 1;
      if (result[0] == "list") {
        n_elements = result.length - 1;
      }

      if (this.token.token_type != expected_right) {
        if (n_elements != 2 || other_right === null) {
          throw new ParseError('Expected ' + expected_right,
            this.lexer.location);
        } else if (this.token.token_type != other_right) {
          throw new ParseError('Expected ) or ]', this.lexer.location);
        }

        // half-open interval
        result[0] = 'tuple';
        result = ['interval', result];
        var closed;
        if (token_left == '(')
          closed = ['tuple', false, true];
        else
          closed = ['tuple', true, false];
        result.push(closed);

      } else if (n_elements >= 2) {
        if (token_left == '(') {
          result[0] = 'tuple';
        } else if (token_left == '[') {
          result[0] = 'array';
        } else {
          result[0] = 'set';
        }
      } else if (token_left === '{') {
        // singleton set
        result = ['set'].concat(result);
      }

      this.advance();
    }

    if (this.token.token_type == '_') {
      if (result === false) {
        throw new ParseError("Invalid location of _", this.lexer.location);
      }
      this.advance();
      var subresult = this.baseFactor();

      if (subresult === false) {
        if (this.token.token_type == "EOF") {
          throw new ParseError("Unexpected end of input", this.lexer.location);
        } else {
          throw new ParseError("Invalid location of '" + this.token.original_text + "'",
            this.lexer.location);
        }
      }
      return ['_', result, subresult];
    }

    return result;
  }

}

export default textToAst;
