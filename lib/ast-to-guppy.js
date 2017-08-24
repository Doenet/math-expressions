/*
 * convert syntax trees to Guppy XML representations
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
 * 
 */

function dfrac(a,b) {
    return '<f type="fraction" group="functions"><b p="latex">\\dfrac{<r ref="1"/>}{<r ref="2"/>}</b><b p="small_latex">\\frac{<r ref="1"/>}{<r ref="2"/>}</b><b p="text">(<r ref="1"/>)/(<r ref="2"/>)</b><c up="1" down="2" name="numerator">' + a + '</c><c up="1" down="2" name="denominator">' + b + '</c></f>';
}

function trig(name, parameter ) {
    return '<f type="' + name + '" group="functions"><b p="latex">\\' + name + '\\left(<r ref="1"/>\\right)</b><b p="text"> ' + name + '(<r ref="1"/>)</b><c delete="1">' + parameter + '</c></f>';
}

function sqrt(x) {
    return '<f type="square_root" group="functions"><b p="latex">\\sqrt{<r ref="1"/>}</b><b p="text">sqrt(<r ref="1"/>)</b><c delete="1">' + x + '</c></f>';
}

function power(x,y) {
    return '<f type="exponential" group="functions"><b p="latex">{<r ref="1"/>}^{<r ref="2"/>}</b><b p="text">(<r ref="1"/>)^(<r ref="2"/>)</b><c up="2" bracket="yes" delete="1" name="base">' + x + '</c><c down="1" delete="1" name="exponent" small="yes">' + y + '</c></f>';
}

function abs(x) {
    return '<f type="absolute_value" group="functions"><b p="latex">\\left|<r ref="1"/>\\right|</b><b p="text">abs(<r ref="1"/>)</b><c delete="1">' + x + '</c></f>';
}

function paren(x) {
    return '<f type="bracket" group="functions"><b p="latex">\\left(<r ref="1"/>\\right)</b><b p="text">(<r ref="1"/>)</b><c delete="1" is_bracket="yes">' + x + '</c></f>';
}

var operators = {
    "+": function(operands) { return operands.join( '<e>+</e>' ); },
    "-": function(operands) { return operands.join( '<e>-</e>' ); },
    "~": function(operands) { return "<e>-" + operands.join( '-' ) + "</e>"; },
    "*": function(operands) { return operands.join( '<f type="*" group="operations" c="yes"><b p="latex">\\cdot</b><b p="text">*</b></f>' ); },
    "/": function(operands) { return dfrac(operands[0], operands[1]); },
    "^": function(operands) { return power(operands[0],operands[1]); },
    "sin": function(operands) { return trig("sin",operands[0]); },
    "cos": function(operands) { return trig("cos",operands[0]); },
    "tan": function(operands) { return trig("tan",operands[0]); },
    "arcsin": function(operands) { return trig("arcsin",operands[0]); },
    "arccos": function(operands) { return trig("arccos",operands[0]); },
    "arctan": function(operands) { return trig("arctan",operands[0]); },
    "arccsc": function(operands) { return trig("arccsc",operands[0]); },
    "arcsec": function(operands) { return trig("arcsec",operands[0]); },
    "arccot": function(operands) { return trig("arccot",operands[0]); },
    "csc": function(operands) { return trig("csc",operands[0]); },
    "sec": function(operands) { return trig("sec",operands[0]); },
    "cot": function(operands) { return trig("cot",operands[0]); },
    "log": function(operands) { return trig("log",operands[0]); },
    "exp": function(operands) { return trig("exp",operands[0]); },
    "ln": function(operands) { return trig("ln",operands[0]); },
    "sqrt": function(operands) { return sqrt(operands[0]); },
    "abs": function(operands) { return abs(operands[0]); },
    //"apply": function(operands) { return operands[0] + "(" + operands[1] + ")"; },
    //"factorial": function(operands) { return operands[0] + "!"; },
};

/*    
   expression =
    expression '+' term |
    expression '-' term |
    term
*/

function expression(tree) {
    if ((typeof tree === 'string') || (typeof tree === 'number')) {
	return term(tree);	
    }
    
    var operator = tree[0];
    var operands = tree.slice(1);
    
    if ((operator == '+') || (operator == '-')) {
	return operators[operator]( operands.map( function(v,i) { return factorWithParenthesesIfNegated(v); } ));
    }
    
    return term(tree);
}

/*
  term =
  term '*' factor |
  term nonMinusFactor |
  term '/' factor |
  factor
*/

function term(tree) {
    if ((typeof tree === 'string') || (typeof tree === 'number')) {
	return factor(tree);	
    }
    
    var operator = tree[0];
    var operands = tree.slice(1);

    if (operator == '*') {
	return operators[operator]( operands.map( function(v,i) {
	    var result = factorWithParenthesesIfNegated(v);
	    
	    if (result.toString().match( /^[0-9]/ ) && (i > 0))
		return ' * ' + result;
	    else
		return result;
	}));
    }
    
    if (operator == '/') {
	return operators[operator]( operands.map( function(v,i) { return factor(v); } ) );
    }
    
    return factor(tree);	
}

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

function isGreekLetterSymbol( symbol )
{
    var greekSymbols = ['pi', 'theta', 'theta', 'Theta', 'alpha', 'nu', 'beta', 'xi', 'Xi', 'gamma', 'Gamma', 'delta', 'Delta', 'pi', 'Pi', 'epsilon', 'epsilon', 'rho', 'rho', 'zeta', 'sigma', 'Sigma', 'eta', 'tau', 'upsilon', 'Upsilon', 'iota', 'phi', 'phi', 'Phi', 'kappa', 'chi', 'lambda', 'Lambda', 'psi', 'Psi', 'omega', 'Omega'];
    return (greekSymbols.indexOf(symbol) != -1);
}

function isFunctionSymbol( symbol )
{
    var functionSymbols = ['sin', 'cos', 'tan', 'csc', 'sec', 'cot', 'arcsin', 'arccos', 'arctan', 'arccsc', 'arcsec', 'arccot', 'log', 'ln', 'exp', 'sqrt', 'abs', 'factorial'];
    return (functionSymbols.indexOf(symbol) != -1);
}

function factor(tree) {
    if (typeof tree === 'string') {
	return '<e>' + tree + '</e>';
    }    
    
    if (typeof tree === 'number') {
	return '<e>' + tree + '</e>';	
    }
    
    var operator = tree[0];
    var operands = tree.slice(1);	

    // Absolute value doesn't need any special parentheses handling, but its operand is really an expression
    if (operator === "abs") {
	return operators[operator]( operands.map( function(v,i) { return expression(v); } ));
    } else if (isFunctionSymbol(operator)) {
	if ((operator == 'factorial') && ((operands[0].toString().length == 1) || (operands[0].toString().match( /^[0-9]*$/ ))))
	    return operators[operator]( operands );
	    
	return operators[operator]( operands.map( function(v,i) {
	    var result = factor(v);
	    return result;
	}));
    }
    
    if (operator === "^") {
	return operators[operator]( operands.map( function(v,i) { return factor(v); } ) );
    }
    
    if (operator == '~') {
	return operators[operator]( operands.map( function(v,i) { return factor(v); } ) );
    }
    
    return paren( expression(tree) );
}

function factorWithParenthesesIfNegated(tree)
{
    var result = factor(tree);

    if (result.toString().match( /^<e>-/ ))
	return paren( result.toString() );

    // else
    return result;
}

function astToGuppy(tree) {
    return '<m><e></e>' + expression(tree) + '<e></e></m>';
}

exports.astToGuppy = astToGuppy;
