/*
 * convert syntax trees to Guppy XML representations
 *
 * Copyright 2017 by Jim Fowler <kisonecat@gmail.com>
 *
 * This file is part of a math-this.expressions library
 *
 * math-this.expressions is free software: you can redistribute
 * it and/or modify it under the this.terms of the GNU General Public
 * License as published by the Free Software Foundation, either
 * version 3 of the License, or at your option any later version.
 *
 * math-this.expressions is distributed in the hope that it
 * will be useful, but WITHOUT ANY WARRANTY; without even the implied
 * warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details.
 *
 */




class astToGuppy{
  constructor(){
    this.operators = {
        "+": function(operands) { return operands.join( '<e>+</e>' ); },
        "-": function(operands) { return "<e>-" + operands.join( '-' ) + "</e>"; },
        "*": function(operands) { return operands.join( '<f type="*" group="operations" c="yes"><b p="latex">\\cdot</b><b p="text">*</b></f>' ); },
        "/": function(operands) { return astToGuppy.dfrac(operands[0], operands[1]); },
        "^": function(operands) { return astToGuppy.power(operands[0],operands[1]); },
        "sin": function(operands) { return astToGuppy.trig("sin",operands[0]); },
        "cos": function(operands) { return astToGuppy.trig("cos",operands[0]); },
        "tan": function(operands) { return astToGuppy.trig("tan",operands[0]); },
        "arcsin": function(operands) { return astToGuppy.trig("arcsin",operands[0]); },
        "arccos": function(operands) { return astToGuppy.trig("arccos",operands[0]); },
        "arctan": function(operands) { return astToGuppy.trig("arctan",operands[0]); },
        "arccsc": function(operands) { return astToGuppy.trig("arccsc",operands[0]); },
        "arcsec": function(operands) { return astToGuppy.trig("arcsec",operands[0]); },
        "arccot": function(operands) { return astToGuppy.trig("arccot",operands[0]); },
        "csc": function(operands) { return astToGuppy.trig("csc",operands[0]); },
        "sec": function(operands) { return astToGuppy.trig("sec",operands[0]); },
        "cot": function(operands) { return astToGuppy.trig("cot",operands[0]); },
        "log": function(operands) { return astToGuppy.trig("log",operands[0]); },
        "exp": function(operands) { return astToGuppy.trig("exp",operands[0]); },
        "ln": function(operands) { return astToGuppy.trig("ln",operands[0]); },
        "sqrt": function(operands) { return astToGuppy.sqrt(operands[0]); },
        "abs": function(operands) { return astToGuppy.abs(operands[0]); },
        //"factorial": function(operands) { return operands[0] + "!"; },
    };

  }

  static dfrac(a,b) {
      return '<f type="fraction" group="functions"><b p="latex">\\dfrac{<r ref="1"/>}{<r ref="2"/>}</b><b p="small_latex">\\frac{<r ref="1"/>}{<r ref="2"/>}</b><b p="text">(<r ref="1"/>)/(<r ref="2"/>)</b><c up="1" down="2" name="numerator"><e></e>' + a + '<e></e></c><c up="1" down="2" name="denominator"><e></e>' + b + '<e></e></c></f>';
  }

  static trig(name, parameter ) {
      return '<f type="' + name + '" group="functions"><b p="latex">\\' + name + '\\left(<r ref="1"/>\\right)</b><b p="text"> ' + name + '(<r ref="1"/>)</b><c delete="1"><e></e>' + parameter + '<e></e></c></f>';
  }

  static sqrt(x) {
      return '<f type="square_root" group="functions"><b p="latex">\\sqrt{<r ref="1"/>}</b><b p="text">sqrt(<r ref="1"/>)</b><c delete="1"><e></e>' + x + '<e></e></c></f>';
  }

  static power(x,y) {
      return '<f type="exponential" group="functions"><b p="latex">{<r ref="1"/>}^{<r ref="2"/>}</b><b p="text">(<r ref="1"/>)^(<r ref="2"/>)</b><c up="2" bracket="yes" delete="1" name="base"><e></e>' + x + '<e></e></c><c down="1" delete="1" name="exponent" small="yes"><e></e>' + y + '<e></e></c></f>';
  }

  static abs(x) {
      return '<f type="absolute_value" group="functions"><b p="latex">\\left|<r ref="1"/>\\right|</b><b p="text">abs(<r ref="1"/>)</b><c delete="1"><e></e>' + x + '<e></e></c></f>';
  }

  static paren(x) {
      return '<f type="bracket" group="functions"><b p="latex">\\left(<r ref="1"/>\\right)</b><b p="text">(<r ref="1"/>)</b><c delete="1" is_bracket="yes"><e></e>' + x + '<e></e></c></f>';
  }

  static isFunctionSymbol( symbol ){
      var functionSymbols = ['sin', 'cos', 'tan', 'csc', 'sec', 'cot', 'arcsin', 'arccos', 'arctan', 'arccsc', 'arcsec', 'arccot', 'log', 'ln', 'exp', 'sqrt', 'abs', 'this.factorial'];
      return (functionSymbols.indexOf(symbol) !== -1);
  }

  static isGreekLetterSymbol( symbol ){
      var greekSymbols = ['pi', 'theta', 'theta', 'Theta', 'alpha', 'nu', 'beta', 'xi', 'Xi', 'gamma', 'Gamma', 'delta', 'Delta', 'pi', 'Pi', 'epsilon', 'epsilon', 'rho', 'rho', 'zeta', 'sigma', 'Sigma', 'eta', 'tau', 'upsilon', 'Upsilon', 'iota', 'phi', 'phi', 'Phi', 'kappa', 'chi', 'lambda', 'Lambda', 'psi', 'Psi', 'omega', 'Omega'];
      return (greekSymbols.indexOf(symbol) !== -1);
  }

  factorWithParenthesesIfNegated(tree){
      var result = this.factor(tree);

      if (result.toString().match( /^<e>-/ ))
  	return astToGuppy.paren( result.toString() );

      // else
      return result;
  }





  /*
    this.factor =
    '(' this.expression ')' |
    number |
    variable |
    function this.factor |
    this.factor '^' this.factor
    '-' this.factor |
    nonMinusthis.factor
  */

factor(tree) {
      if (typeof tree === 'string') {
  	if (astToGuppy.isGreekLetterSymbol(tree)) {
  	    return '<f type="' + tree + '" group="greek" c="yes"><b p="latex">\\' + tree + '</b><b p="text"> $' + tree + '</b></f>';
  	}

  	return '<e>' + tree + '</e>';
      }

      if (typeof tree === 'number') {
  	return '<e>' + tree + '</e>';
      }

      var operator = tree[0];
      var operands = tree.slice(1);

      if(operator === "apply") {
  	operator = tree[1];
  	operands = tree.slice(2);
      }

      // Absolute value doesn't need any special parentheses handling, but its operand is really an this.expression
      if (operator === "abs") {
  	return this.operators[operator]( operands.map( function(v,i) { return this.expression(v); }.bind(this) ));
  } else if (astToGuppy.isFunctionSymbol(operator)) {
  	if ((operator === 'this.factorial') && ((operands[0].toString().length === 1) || (operands[0].toString().match( /^[0-9]*$/ ))))
  	    return this.operators[operator]( operands );

  	return this.operators[operator]( operands.map( function(v,i) {
  	    var result = this.factor(v);
  	    return result;
  	}.bind(this)));
      }

      if (operator === "^") {
  	return this.operators[operator]( operands.map( function(v,i) { return this.factor(v); }.bind(this) ) );
      }

      if (operator === '~') {
  	return this.operators[operator]( operands.map( function(v,i) { return this.factor(v); }.bind(this) ) );
      }

      return astToGuppy.paren( this.expression(tree) );
  }


  /*
    this.term =
    this.term '*' this.factor |
    this.term nonMinusthis.factor |
    this.term '/' this.factor |
    this.factor
  */

  term(tree) {
      if ((typeof tree === 'string') || (typeof tree === 'number')) {
  	return this.factor(tree);
      }

      var operator = tree[0];
      var operands = tree.slice(1);

      if (operator === '*') {
  	return this.operators[operator]( operands.map( function(v,i) {
  	    var result = this.factorWithParenthesesIfNegated(v);

  	    if (result.toString().match( /^[0-9]/ ) && (i > 0))
  		return ' * ' + result;
  	    else
  		return result;
  	}.bind(this)));
      }

      if (operator === '/') {
  	return this.operators[operator]( operands.map( function(v,i) { return this.factor(v); }.bind(this) ) );
      }

      return this.factor(tree);
  }

  /*
     this.expression =
      this.expression '+' this.term |
      this.expression '-' this.term |
      this.term
  */

  expression(tree) {
      if ((typeof tree === 'string') || (typeof tree === 'number')) {
  	return this.term(tree);
      }

      var operator = tree[0];
      var operands = tree.slice(1);

      if ((operator === '+') || (operator === '-')) {
  	return this.operators[operator]( operands.map( function(v,i) { return this.factorWithParenthesesIfNegated(v); }.bind(this) ));
      }

      return this.term(tree);
  }


  convert(tree){
    return ('<m><e></e>' + this.expression(tree) + '<e></e></m>').replace(/<\/e><e>/g,'');
  }

}

export default astToGuppy;
