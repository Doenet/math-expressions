/*
 * convert syntax trees to GLSL representations
 *
 * Copyright 2014-2018 by
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


const glslOperators = {
    "+": function(operands) { var result = operands[0]; operands.slice(1).forEach(function(rhs) { result = result + "+" + rhs; }); return result; },
  "-": function(operands) { var result = "-" + operands[0]; operands.slice(1).forEach(function(rhs) { result = result + "-" + rhs; }); return result; },
    "~": function(operands) { var result = "vec2(0.0,0.0)"; operands.forEach(function(rhs) { result = result + "-" + rhs; }); return result; },
    "*": function(operands) { var result = operands[0]; operands.slice(1).forEach(function(rhs) { result = "cmul(" + result + "," + rhs + ")"; }); return result; },
    "/": function(operands) { var result = operands[0]; operands.slice(1).forEach(function(rhs) { result = "cdiv(" + result + "," + rhs + ")"; }); return result; },

    "sin": function(operands) { return "csin(" + operands[0] + ")"; },
    "cos": function(operands) { return "ccos(" + operands[0] + ")"; },
    "tan": function(operands) { return "ctan(" + operands[0] + ")"; },

    "arcsin": function(operands) { return "carcsin(" + operands[0] + ")"; },
    "arccos": function(operands) { return "carccos(" + operands[0] + ")"; },
    "arctan": function(operands) { return "carctan(" + operands[0] + ")"; },

    "arccsc": function(operands) { return "carcsin(cdiv(vec2(1.0,0)," + operands[0] + "))"; },
    "arcsec": function(operands) { return "carccos(cdiv(vec2(1.0,0)," + operands[0] + "))"; },
    "arccot": function(operands) { return "carctan(cdiv(vec2(1.0,0)," + operands[0] + "))"; },

    "csc": function(operands) { return "ccsc(" + operands[0] + ")"; },
    "sec": function(operands) { return "csec(" + operands[0] + ")"; },
    "cot": function(operands) { return "ccot(" + operands[0] + ")"; },

    "exp": function(operands) { return "cexp(" + operands[0] + ")"; },    
    
    "sqrt": function(operands) { return "cpower(" + operands[0] + ",vec2(0.5,0.0))"; },
    "log": function(operands) { return "clog(" + operands[0] + ")"; },
    "ln": function(operands) { return "clog(" + operands[0] + ")"; },    
    "^": function(operands) { return "cpower(" + operands[0] + "," + operands[1] + ")"; },
    
    "abs": function(operands) { return "cabs(" + operands[0] + ")"; },
    "apply": function(operands) { return "vec2(NaN,NaN)"; },
};

class astToGLSL {
  constructor() {
  }
  
    convert(tree) {
	if (typeof tree === 'boolean')
	    throw Error("no support for boolean");

	
    if (typeof tree === 'string') {
	if (tree === "e")
	    return "vec2(2.71828182845905,0.0)";
	
	if (tree === "pi")
	    return "vec2(3.14159265358979,0.0)";
	
	if (tree === "i")
	    return "vec2(0.0,1.0)";

	return String(tree);
    }    
    
    if (typeof tree === 'number') {
	return "vec2(" + String(tree) + ",0.0)";
    }
    
    if (("real" in tree) && ("imaginary" in tree))
	return tree;

	if (!Array.isArray(tree)) {
	    throw Error("Invalid ast");
	}

	
    var operator = tree[0];
    var operands = tree.slice(1);

    if(operator === "apply") {
      if(typeof operands[0] !== 'string')
	throw Error("Non string functions not implemented for conversion to GLSL");

	var operator = operands[0];
	var operands = operands.slice(1);
	
	return glslOperators[operator]( operands.map( function(v,i) { return this.convert(v); }.bind(this) ) );
    }
	
    if (operator in glslOperators) {
	return glslOperators[operator]( operands.map( function(v,i) { return this.convert(v); }.bind(this) ) );
    }
    
	throw Error("Operator " + operator + " not implemented for conversion to mathjs");
    }
}


export default astToGLSL;
