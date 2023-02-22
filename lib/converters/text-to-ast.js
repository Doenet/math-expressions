import { ParseError } from './error';
import lexer from './lexer';
import flatten from './flatten';
import { get_all_units } from '../expression/units';

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
    '...' |
    statement_a '|' statement_a |
    statement_a ':' statement_a |
    statement_a
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
    '|' statement '|' |
    number |
    variable |
    modified_function '(' statement_list ')' |
    modified_applied_function '(' statement_list ')' |
    modified_function |
    modified_applied_function factor |
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

// in order to parse as scientific notation, e.g., 3.2E-12 or .7E+3,
// it must be at the end or followed a comma, |, ), }, or ]
const sci_notat_exp_regex = '(E[+\\-]?[0-9]+\\s*($|(?=\\,|\\||\\)|\\}|\\])))?';


const number_rules_sci = [
  ['[0-9]+(\\.[0-9]*)?' + sci_notat_exp_regex, 'NUMBER'],
  ['\\.[0-9]+' + sci_notat_exp_regex, 'NUMBER'],
]

const number_rules_non_sci = [
  ['[0-9]+(\\.[0-9]*)?', 'NUMBER'],
  ['\\.[0-9]+', 'NUMBER'],
]

const base_text_rules = [
  ['\\*\\*', '^'],
  ['\\*', '*'], // there is some variety in multiplication symbols
  ['\\xB7', '*'], // '·'
  ['\u00B7', '*'], // '·'
  ['\u2022', '*'], // '•'
  ['\u22C5', '*'], // '⋅'
  ['\u00D7', '*'], // '×'
  ['/', '/'],
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
  ['\u27E8', 'LANGLE'],
  ['\u27E9', 'RANGLE'],
  ['\u3008', 'LANGLE'],
  ['\u3009', 'RANGLE'],
  [',', ','],
  [':', ':'],

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

  ['perp\\b', 'PERP'],
  ['\u27c2', 'PERP'], // '⟂'

  ['parallel\\b', 'PARALLEL'],
  ['\u2225', 'PARALLEL'], // '∥'

  ['angle\\b', 'ANGLE'],
  ['\u2220', 'ANGLE'],  // '∠'

  ['!', '!'],
  ['\'', '\''],
  ['_', '_'],
  ['\\.\\.\\.', 'LDOTS'],
  ['[a-zA-Z∂][a-zA-Z∂0-9]*', 'VAR'],  // include ∂ in VAR
  ['[\uff3f$%]', 'VAR']
];


// defaults for parsers if not overridden by context

// if true, allowed applied functions to omit parentheses around argument
// if false, omitting parentheses will lead to a Parse Error
const allowSimplifiedFunctionApplicationDefault = true;

// if true, split multicharacter symbols into a product of letters
const splitSymbolsDefault = true;

// symbols that won't be split into a product of letters if splitSymbols==true
const unsplitSymbolsDefault = ['alpha', 'beta', 'gamma', 'Gamma', 'delta', 'Delta', 'epsilon', 'zeta', 'eta', 'theta', 'Theta', 'iota', 'kappa', 'lambda', 'Lambda', 'mu', 'nu', 'xi', 'Xi', 'pi', 'Pi', 'rho', 'sigma', 'Sigma', 'tau', 'Tau', 'upsilon', 'Upsilon', 'phi', 'Phi', 'chi', 'psi', 'Psi', 'omega', 'Omega', 'angle', 'deg'];

// Applied functions must be given an argument so that
// they are applied to the argument
const appliedFunctionSymbolsDefault = [
  "abs", "exp", "log", "ln", "log10", "sign", "sqrt", "erf",
  "cos", "cosh", "acos", "acosh", 'arccos', 'arccosh',
  "cot", "coth", "acot", "acoth", 'arccot', 'arccoth',
  "csc", "csch", "acsc", "acsch", 'arccsc', 'arccsch',
  "sec", "sech", "asec", "asech", 'arcsec', 'arcsech',
  "sin", "sinh", "asin", "asinh", 'arcsin', 'arcsinh',
  "tan", "tanh", "atan", "atan2", "atanh", 'arctan', 'arctanh',
  'arg', 'conj', 're', 'im', 'det', 'trace', 'nPr', 'nCr',
  'floor', 'ceil', 'round',
];

// Functions could have an argument, in which case they are applied
// or, if they don't have an argument in parentheses, then they are treated
// like a variable, except that trailing ^ and ' have higher precedence
const functionSymbolsDefault = ['f', 'g'];

// operators must be given an argument
const operatorSymbolsDefault = ['binom', 'vec'];

const unitsDefault = get_all_units();

// Parse Leibniz notation
const parseLeibnizNotationDefault = true;


class textToAst {
  constructor({
    allowSimplifiedFunctionApplication = allowSimplifiedFunctionApplicationDefault,
    splitSymbols = splitSymbolsDefault,
    unsplitSymbols = unsplitSymbolsDefault,
    appliedFunctionSymbols = appliedFunctionSymbolsDefault,
    functionSymbols = functionSymbolsDefault,
    operatorSymbols = operatorSymbolsDefault,
    units = unitsDefault,
    parseLeibnizNotation = parseLeibnizNotationDefault,
    parseScientificNotation = true,
  } = {}) {
    this.allowSimplifiedFunctionApplication = allowSimplifiedFunctionApplication;
    this.splitSymbols = splitSymbols;
    this.unsplitSymbols = unsplitSymbols;
    this.appliedFunctionSymbols = appliedFunctionSymbols;
    this.functionSymbols = functionSymbols;
    this.operatorSymbols = operatorSymbols;
    this.units = units;
    this.parseLeibnizNotation = parseLeibnizNotation;

    let text_rules = base_text_rules;

    if (parseScientificNotation) {
      text_rules = [...number_rules_sci, ...text_rules]
    } else {
      text_rules = [...number_rules_non_sci, ...text_rules]
    }

    this.lexer = new lexer(text_rules);

  }

  advance(params) {
    this.token = this.lexer.advance(params);
    if (this.token.token_type === 'INVALID') {
      throw new ParseError("Invalid symbol '" + this.token.original_text + "'",
        this.lexer.location);
    }
  }

  return_state() {
    return ({
      lexer_state: this.lexer.return_state(),
      token: Object.assign({}, this.token)
    });
  }

  set_state(state) {
    this.lexer.set_state(state.lexer_state);
    this.token = Object.assign({}, state.token);
  }


  convert(input) {

    this.lexer.set_input(input);
    this.advance();

    var result = this.statement_list();

    if (this.token.token_type !== 'EOF') {
      throw new ParseError("Invalid location of '" + this.token.original_text + "'",
        this.lexer.location);
    }

    return flatten(result);

  }


  statement_list() {

    var list = [this.statement()];

    while (this.token.token_type === ",") {
      this.advance();
      list.push(this.statement());
    }

    if (list.length > 1)
      list = ['list'].concat(list);
    else
      list = list[0];

    return list;
  }

  statement({ inside_absolute_value = 0 } = {}) {

    // three periods ... can be a statement by itself
    if (this.token.token_type === 'LDOTS') {
      this.advance();
      return ['ldots'];
    }

    var original_state;

    try {

      original_state = this.return_state();

      let lhs = this.statement_a({ inside_absolute_value: inside_absolute_value });

      if (this.token.token_type !== ':')
        return lhs;

      this.advance();

      let rhs = this.statement_a();

      return [':', lhs, rhs];

    }
    catch (e) {
      try {

        // if ran into problem parsing statement
        // then try again with ignoring absolute value
        // and then interpreting bar as a binary operator

        // return state to what it was before attempting to parse statement
        this.set_state(original_state);

        let lhs = this.statement_a({ parse_absolute_value: false });

        if (this.token.token_type !== '|') {
          throw (e);
        }

        this.advance();

        let rhs = this.statement_a({ parse_absolute_value: false });

        return ['|', lhs, rhs];

      }
      catch (e2) {
        throw (e);  // throw original error
      }
    }
  }

  statement_a({ inside_absolute_value = 0, parse_absolute_value = true } = {}) {

    var lhs = this.statement_b({
      inside_absolute_value: inside_absolute_value,
      parse_absolute_value: parse_absolute_value
    });

    while (this.token.token_type === 'OR') {

      let operation = this.token.token_type.toLowerCase();

      this.advance();

      let rhs = this.statement_b({
        inside_absolute_value: inside_absolute_value,
        parse_absolute_value: parse_absolute_value
      });

      lhs = [operation, lhs, rhs];
    }

    return lhs;
  }

  statement_b(params) {
    // split AND into second statement to give higher precedence than OR

    var lhs = this.relation(params);

    while (this.token.token_type === 'AND') {

      let operation = this.token.token_type.toLowerCase();

      this.advance();

      let rhs = this.relation(params);

      lhs = [operation, lhs, rhs];
    }

    return lhs;
  }


  relation(params) {

    if (this.token.token_type === 'NOT' || this.token.token_type === '!') {
      this.advance();
      return ['not', this.relation(params)];
    }

    var lhs = this.expression(params);

    while ((this.token.token_type === '=') || (this.token.token_type === 'NE') ||
      (this.token.token_type === '<') || (this.token.token_type === '>') ||
      (this.token.token_type === 'LE') || (this.token.token_type === 'GE') ||
      (this.token.token_type === 'IN') || (this.token.token_type === 'NOTIN') ||
      (this.token.token_type === 'NI') || (this.token.token_type === 'NOTNI') ||
      (this.token.token_type === 'SUBSET') || (this.token.token_type === 'NOTSUBSET') ||
      (this.token.token_type === 'SUPERSET') || (this.token.token_type === 'NOTSUPERSET')) {

      let operation = this.token.token_type.toLowerCase();

      let inequality_sequence = 0;

      if ((this.token.token_type === '<') || (this.token.token_type === 'LE')) {
        inequality_sequence = -1;
      } else if ((this.token.token_type === '>') || (this.token.token_type === 'GE')) {
        inequality_sequence = 1;
      }

      this.advance();
      let rhs = this.expression(params);

      if (inequality_sequence === -1) {
        if ((this.token.token_type === '<') || this.token.token_type === 'LE') {
          // sequence of multiple < or <=
          let strict = ['tuple'];
          if (operation === '<')
            strict.push(true)
          else
            strict.push(false)

          let args = ['tuple', lhs, rhs];
          while ((this.token.token_type === '<') || this.token.token_type === 'LE') {
            if (this.token.token_type === '<')
              strict.push(true)
            else
              strict.push(false)

            this.advance();
            args.push(this.expression(params));
          }
          lhs = ['lts', args, strict];
        } else {
          lhs = [operation, lhs, rhs];
        }

      } else if (inequality_sequence === 1) {
        if ((this.token.token_type === '>') || this.token.token_type === 'GE') {
          // sequence of multiple > or >=
          let strict = ['tuple'];
          if (operation === '>')
            strict.push(true)
          else
            strict.push(false)

          let args = ['tuple', lhs, rhs];
          while ((this.token.token_type === '>') || this.token.token_type === 'GE') {
            if (this.token.token_type === '>')
              strict.push(true)
            else
              strict.push(false)

            this.advance();
            args.push(this.expression(params));
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
    if (this.token.token_type === '+') {
      plus_begin = true;
      this.advance();
    }

    let negative_begin = false;
    if (this.token.token_type === '-') {
      negative_begin = true;
      this.advance();
    }

    var lhs = this.term(params);

    if (negative_begin || plus_begin) {
      if (lhs === false) {
        return (plus_begin ? "+" : "") + (negative_begin ? "-" : "");
      } else if (typeof lhs === "string" && [...lhs].every(x => ["+", "-"].includes(x))) {
        return (plus_begin ? "+" : "") + (negative_begin ? "-" : "") + lhs;
      }
    }

    if (lhs === false) {
      lhs = '\uff3f';
    }

    if (negative_begin) {
      if (lhs > 0) {
        lhs = -lhs;
      } else {
        lhs = ['-', lhs];
      }
    }

    if (plus_begin) {
      lhs = ['+', lhs];
    }

    while (['+', '-', 'UNION', 'INTERSECT', 'PERP', 'PARALLEL'].includes(this.token.token_type)) {

      let operation = this.token.token_type.toLowerCase();
      let negative = false;
      let positive_then_negative = false;

      if (this.token.token_type === '-') {
        operation = '+';
        negative = true;
        this.advance();
      } else {
        this.advance();
        if (operation === '+' && this.token.token_type === '-') {
          negative = true;
          positive_then_negative = true;
          this.advance();
        }
      }
      let rhs = this.term(params);

      if (operation === "+") {
        if (rhs === false
          && (typeof lhs === "number" || typeof lhs === "string")
        ) {
          if (positive_then_negative) {
            return lhs + "+-";
          } else if (negative) {
            return lhs + "-"
          } else {
            return lhs + "+";
          }
        } else if (typeof rhs === "string" && [...rhs].every(x => ["+", "-"].includes(x))
          && (typeof lhs === "number" || typeof lhs === "string")
        ) {
          if (positive_then_negative) {
            return lhs + "+-" + rhs;;
          } else if (negative) {
            return lhs + "-" + rhs;
          } else {
            return lhs + "+" + rhs;
          }
        }
      }

      if (rhs === false) {
        rhs = '\uff3f';
      }

      if (negative) {
        if (rhs > 0) {
          rhs = -rhs;
        } else {
          rhs = ['-', rhs];
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

      if (this.token.token_type === '*') {
        this.advance();
        if (lhs === false) {
          lhs = '\uff3f';
        }
        let rhs = this.factor(params);
        if (rhs === false) {
          rhs = '\uff3f';
        }
        lhs = ['*', lhs, rhs];
        keepGoing = true;
      } else if (this.token.token_type === '/') {
        this.advance();
        if (lhs === false) {
          lhs = '\uff3f';
        }
        let rhs = this.factor(params);
        if (rhs === false) {
          rhs = '\uff3f';
        }
        lhs = ['/', lhs, rhs];
        keepGoing = true;
      } else {
        // this is the one case where a | could indicate a closing absolute value
        let params2 = Object.assign({}, params);
        params2.allow_absolute_value_closing = true;
        let rhs = this.nonMinusFactor(params2);
        if (rhs !== false) {
          if (lhs === false) {
            lhs = '\uff3f';
          }
          lhs = ['*', lhs, rhs];
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
              post_unit_converted = this.convert_units_in_term(["*", ...operands.slice(ind + 1)]);
            }
            let unit = op;
            if (unit_object.substitute) {
              unit = unit_object.substitute;
            }
            let unit_tree = ["unit", unit, post_unit_converted];

            if (ind === 0) {
              return unit_tree;
            } else {
              return ["*", ...operands.slice(0, ind), unit_tree];
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
              unit_tree = ["unit", ["*", ...operands.slice(0, ind)], unit]
            }
            if (ind === n_ops - 1) {
              return unit_tree;
            } else {
              return this.convert_units_in_term(["*", unit_tree, ...operands.slice(ind + 1)])
            }
          }
        }
      }
      return tree;
    } else if (operator === "/") {
      return ["/", this.convert_units_in_term(operands[0]), this.convert_units_in_term(operands[1])]
    }

    return tree;

  }

  factor(params) {

    if (this.token.token_text === "+") {
      this.advance();

      let factor = this.factor(params);
      if (factor === false) {
        return '+';
      } else if (typeof factor === "string" && [...factor].every(x => ["+", "-"].includes(x))) {
        return "+" + factor;
      } else {
        return ['+', factor]
      }
    }

    if (this.token.token_type === '-') {
      this.advance();

      let factor = this.factor(params);
      if (factor > 0) {
        return -factor;
      } else if (factor === false) {
        return '-';
      } else if (typeof factor === "string" && [...factor].every(x => ["+", "-"].includes(x))) {
        return "-" + factor;
      } else {
        return ['-', factor];
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
    while (this.token.token_type === '^' || this.token.token_type === '!' || this.token.token_type === "'") {
      if (result === false) {
        result = '\uff3f';
      }
      if (this.token.token_type === "^") {
        this.advance();
        let superscript = this.get_subsuperscript(params);
        result = ['^', result, superscript];
      } else if (this.token.token_type === '!') {
        result = ['apply', 'factorial', result]
        this.advance();
      } else {
        result = ['prime', result];
        this.advance();
      }
    }

    return result;
  }


  get_subsuperscript({ parse_absolute_value }) {
    if (["+", "-", "PERP"].includes(this.token.token_type)) {
      let subresult = this.token.token_type.toLowerCase();
      this.advance();
      return subresult;
    } else {
      let subresult = this.baseFactor({ parse_absolute_value, in_subsuperscript_with_no_delimiters: true });
      if (subresult === false) {
        subresult = '\uff3f';
      }
      return subresult;
    }
  }

  baseFactor({ inside_absolute_value = 0,
    parse_absolute_value = true,
    allow_absolute_value_closing = false,
    in_subsuperscript_with_no_delimiters = false,
  } = {}) {

    var result = false;

    if (this.token.token_type === 'NUMBER') {
      result = parseFloat(this.token.token_text);
      this.advance();
    } else if (this.token.token_type === 'INFINITY') {
      result = Infinity;
      this.advance();
    } else if (this.token.token_type === 'VAR' || this.token.token_type === 'VARMULTICHAR') {
      result = this.token.token_text;

      if (this.appliedFunctionSymbols.includes(result) || this.functionSymbols.includes(result)) {

        let must_apply = false
        if (this.appliedFunctionSymbols.includes(result))
          must_apply = true;

        this.advance();

        if (this.token.token_type === '_') {
          this.advance();

          let subscript = this.get_subsuperscript({ parse_absolute_value });

          if (result === "log" && subscript === 10) {
            result = "log10";
          } else {
            result = ['_', result, subscript];
          }
        }

        if (in_subsuperscript_with_no_delimiters) {
          if (must_apply) {
            result = ['apply', result, '\uff3f'];
          }
        } else {

          while (this.token.token_type === "'") {
            result = ['prime', result];
            this.advance();
          }

          while (this.token.token_type === '^') {
            this.advance();

            let superscript = this.get_subsuperscript({ parse_absolute_value });

            result = ['^', result, superscript];
          }

          if (this.token.token_type === '(') {
            this.advance();
            let parameters = this.statement_list();

            if (this.token.token_type !== ')') {
              throw new ParseError('Expecting )', this.lexer.location);
            }
            this.advance();

            if (parameters[0] === 'list') {
              // rename from list to tuple
              parameters[0] = 'tuple';
            }

            result = ['apply', result, parameters];
          } else {
            // if was an applied function symbol,
            // cannot omit argument
            if (must_apply) {
              if (!this.allowSimplifiedFunctionApplication)
                throw new ParseError("Expecting ( after function",
                  this.lexer.location);

              // if allow simplied function application
              // let the argument be the next factor
              let arg = this.factor({ parse_absolute_value: parse_absolute_value });
              if (arg === false) {
                arg = '\uff3f';
              }
              result = ['apply', result, arg];
            }
          }
        }

        return result;  // have function so took care of subscript already

      } else if (this.operatorSymbols.includes(result)) {
        this.advance();

        if (this.token.token_type === '(') {
          this.advance();
          let args = this.statement_list();

          if (this.token.token_type !== ')') {
            throw new ParseError('Expecting )', this.lexer.location);
          }
          this.advance();

          if (args[0] === 'list') {
            // remove list
            result = [result, ...args.slice(1)];
          } else {
            result = [result, args];
          }

        } else {
          // let the argument be the next factor
          let arg = this.factor({ parse_absolute_value: parse_absolute_value });
          if (arg === false) {
            arg = '\uff3f';
          }
          result = [result, arg];
        }

      } else {

        // determine if may be a derivative in Leibniz notation
        if (this.parseLeibnizNotation) {

          let original_state = this.return_state();

          let r = this.leibniz_notation();

          if (r) {
            // successfully parsed derivative in Leibniz notation, so return
            return r;
          }
          else {
            // didn't find a properly format Leibniz notation
            // so reset state and continue
            this.set_state(original_state);
          }
        }

        // determine if should split text into single letter factors
        let split = this.splitSymbols;

        if (split) {
          if (this.token.token_type === 'VARMULTICHAR' ||
            this.unsplitSymbols.includes(result) ||
            result.length === 1) {
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

          for (let i = result.length - 1; i >= 0; i--) {
            this.lexer.unput(" ");
            this.lexer.unput(result[i]);
          }
          this.advance();

          return this.baseFactor({
            inside_absolute_value: inside_absolute_value,
            parse_absolute_value: parse_absolute_value,
            allow_absolute_value_closing: allow_absolute_value_closing
          });
        } else {
          this.advance();
        }
      }
    } else if (this.token.token_type === '(' || this.token.token_type === '[' ||
      this.token.token_type === '{' || this.token.token_type === 'LANGLE') {
      let token_left = this.token.token_type;
      let expected_right, other_right;
      if (this.token.token_type === '(') {
        expected_right = ')';
        other_right = ']';
      } else if (this.token.token_type === '[') {
        expected_right = ']';
        other_right = ')';
      } else if (this.token.token_type === "{") {
        expected_right = '}';
        other_right = null;
      } else {
        expected_right = 'RANGLE';
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
          throw new ParseError('Expecting ' + expected_right,
            this.lexer.location);
        } else if (this.token.token_type !== other_right) {
          throw new ParseError('Expecting ) or ]', this.lexer.location);
        }

        // half-open interval
        result[0] = 'tuple';
        result = ['interval', result];
        let closed;
        if (token_left === '(')
          closed = ['tuple', false, true];
        else
          closed = ['tuple', true, false];
        result.push(closed);

      } else if (n_elements >= 2) {
        if (token_left === '(') {
          result[0] = 'tuple';
        } else if (token_left === '[') {
          result[0] = 'array';
        } else if (token_left === '{') {
          result[0] = 'set';
        } else {
          result[0] = 'altvector';
        }
      } else if (token_left === '{') {
        if (result[0] === '|' || result[0] === ':') {
          result = ['set', result];  // set builder notation
        }
        else {
          result = ['set', result]; // singleton set
        }
      }

      this.advance();

    } else if (this.token.token_type === '|' && parse_absolute_value &&
      (inside_absolute_value === 0 || !allow_absolute_value_closing)) {

      // allow the opening of an absolute value here if either
      // - we aren't already inside an absolute value (inside_absolute_value==0), or
      // - we don't allows an absolute value closing
      // otherwise, skip this token so that will drop out the factor (and entire statement)
      // to where the absolute value will close

      inside_absolute_value += 1;

      this.advance();

      result = this.statement({ inside_absolute_value: inside_absolute_value });
      result = ['apply', 'abs', result];

      if (this.token.token_type !== '|') {
        throw new ParseError('Expecting |', this.lexer.location);
      }

      this.advance();
    } else if (this.token.token_type === 'ANGLE') {

      this.advance();

      if (this.token.token_type === '(') {

        this.advance();
        let parameters = this.statement_list();

        if (this.token.token_type !== ')') {
          throw new ParseError('Expecting ' + ')',
            this.lexer.location);
        }
        this.advance();

        if (parameters[0] === 'list') {
          // remove list
          result = ['angle', ...parameters.slice(1)];
        } else if (parameters[0] === "*") {
          // not sure how to interpret this result, but leave it as an angle with the one argument
          result = ['angle', parameters];
        }
      } else {

        // have an angle not followed by (
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
    }



    if (this.token.token_type === '_') {
      if (result === false) {
        result = '\uff3f';
      }

      this.advance();

      let subscript = this.get_subsuperscript({ parse_absolute_value });

      result = ['_', result, subscript];

    }

    return result;
  }


  leibniz_notation() {
    // attempt to find and return a derivative in Leibniz notation
    // if unsuccessful, return false

    var result = this.token.token_text;

    if (!(this.token.token_type === 'VAR' && (result[0] === "d" || result[0] === "∂")
      && (result.length === 1 || (result.length === 2 && /[a-zA-Z]/.exec(result[1]))))) {
      return false;
    }

    // found one of these two possibilities for start of derivative are
    // - dx or ∂x (no space, x is a single letter)
    // - d or ∂

    let deriv_symbol = result[0];

    let n_deriv = 1;

    let var1 = "";
    let var2s = [];
    let var2_exponents = [];

    if (result.length === 2)
      var1 = result[1];
    else { // result is length 1

      // since have just a d or ∂
      // must be followed by a ^ or a VARMULTICHAR/VAR with no ∂
      this.advance();
      if (this.token.token_type === 'VARMULTICHAR' ||
        (this.token.token_type === 'VAR' && !this.token.token_text.includes('∂'))
      ) {
        var1 = this.token.token_text;
      }

      else {
        // since not VARMULTICHAR, must be a ^ next
        if (this.token.token_type !== '^') {
          return false;
        }

        // so far have d or ∂ followed by ^
        // must be followed by an integer
        this.advance();

        if (this.token.token_type !== 'NUMBER') {
          return false;
        }

        n_deriv = parseFloat(this.token.token_text);
        if (!Number.isInteger(n_deriv)) {
          return false;
        }

        // see if next character is single character
        this.advance();

        // either a VAR with no ∂
        // or a VARMULTICHAR
        if ((this.token.token_type === 'VAR' && !this.token.token_text.includes('∂'))
          || this.token.token_type === 'VARMULTICHAR') {
          var1 = this.token.token_text;
        }
        else {
          return false;
        }
      }
    }

    // next character must be a /

    this.advance(); // allow a space this time

    if (this.token.token_type !== '/')
      return false;

    // find sequence of
    // derivative symbol followed by a single character or VARMULTICHAR (with no space)
    // optionally followed by a ^ and an integer (with no spaces)
    // (with spaces allowed between elements of sequence)
    // End when sum of exponents meets or exceeds n_deriv

    let exponent_sum = 0;

    this.advance(); // allow space just after the /

    while (true) {

      // next must either be
      // - a VAR whose first character matches derivative symbol
      //   and whose second character is a letter, or
      // - a single character VAR that matches derivative symbol
      //   which must be followed by a VARMULTICHAR/VAR with no ∂

      if (this.token.token_type !== 'VAR' || this.token.token_text[0] !== deriv_symbol) {
        return false;
      }

      if (this.token.token_text.length > 2) {
        // Put extra characters back on lexer
        this.lexer.unput(this.token.token_text.slice(2));

        // keep just two character token
        this.token.token_text = this.token.token_text.slice(0, 2);

      }

      let token_text = this.token.token_text;

      // derivative symbol and variable together
      if (token_text.length === 2) {
        if (/[a-zA-Z]/.exec(token_text[1]))
          var2s.push(token_text[1])
        else {
          return false;
        }
      }
      else { // token text was just the derivative symbol
        this.advance();

        if (!((this.token.token_type === 'VAR' && !this.token.token_text.includes('∂'))
          || this.token.token_type === 'VARMULTICHAR')) {
          return false;
        }
        var2s.push(this.token.token_text);
      }

      // have derivative and variable, now check for optional ^ followed by number

      let this_exponent = 1;

      let lastWasSpace = false;

      this.advance({ remove_initial_space: false });
      // if last token was a space advance to next non-space token
      if (this.token.token_type === "SPACE") {
        lastWasSpace = true;
        this.advance();
      }

      if (this.token.token_type === '^') {

        this.advance();

        if (this.token.token_type !== 'NUMBER') {
          return false;
        }

        this_exponent = parseFloat(this.token.token_text);
        if (!Number.isInteger(this_exponent)) {
          return false;
        }

        lastWasSpace = false;

        this.advance({ remove_initial_space: false });
        // if last token was a space advance to next non-space token
        if (this.token.token_type === "SPACE") {
          lastWasSpace = true;
          this.advance();
        }

      }
      var2_exponents.push(this_exponent);
      exponent_sum += this_exponent;

      if (exponent_sum > n_deriv) {
        return false;
      }

      // possibly found derivative
      if (exponent_sum === n_deriv) {

        // check to make sure next token isn't another VAR or VARMULTICHAR
        // in this case, the derivative isn't separated from what follows
        if (!lastWasSpace && (this.token.token_type === "VAR" || this.token.token_type === "VARMULTICHAR")) {
          return false;
        }

        // found derivative!

        // if last token was a space advance to next non-space token
        if (this.token.token_type === "SPACE")
          this.advance();

        let result_name = "derivative_leibniz"
        if (deriv_symbol === "∂")
          result_name = "partial_" + result_name;

        result = [result_name];

        if (n_deriv === 1)
          result.push(var1);
        else
          result.push(["tuple", var1, n_deriv]);

        let r2 = []
        for (let i = 0; i < var2s.length; i += 1) {
          if (var2_exponents[i] === 1)
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

export default textToAst;
