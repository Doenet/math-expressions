/*
 * signature functions for javascript
 *
 * Copyright 2014-2015 by Jim Fowler <kisonecat@gmail.com>
 *
 * Based on the recommendation of Joseph O'Rourke to look at
 *
 *   Schwartz, Jacob T. "Fast probabilistic algorithms for
 *   verification of polynomial identities." Journal of the ACM (JACM)
 *   27.4 (1980): 701-717.
 * 
 * and
 *
 *   Gonnet, Gaston H. "Determining equivalence of expressions in
 *   random polynomial time." Proceedings of the 16th ACM Symposium on
 *   Theory of Computing. ACM, 1984.
 *
 * math-expressions is free software: you can redistribute it and/or
 * modify it under the terms of the GNU General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or at your option any later version.
 * 
 * math-expressions is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 * General Public License for more details.
 * 
 */

var _ = require('underscore');
var NumberTheory = require('./number-theory');

var math_functions = {
    "+": function(operands) { var result = 0; operands.forEach(function(x) { result += x; }); return result; },
    "-": function(operands) { var result = operands[0]; operands.slice(1).forEach(function(x) { result -= x; }); return result; },
    "*": function(operands) { var result = operands[0]; operands.slice(1).forEach(function(x) { result *= x; }); return result; },
    "/": function(operands) { var result = operands[0]; operands.slice(1).forEach(function(x) { result /= x; }); return result; },
    "~": function(operands) { var result = 0; operands.forEach(function(x) { result -= x; }); return result; },
    "sin": function(operands) { return Math.sin(operands[0]); },
    "cos": function(operands) { return Math.cos(operands[0]); },
    "tan": function(operands) { return Math.tan(operands[0]); },
    "arcsin": function(operands) { return Math.asin(operands[0]); },
    "arccos": function(operands) { return Math.acos(operands[0]); },
    "arctan": function(operands) { return Math.atan(operands[0]); },
    "arccsc": function(operands) { return Math.asin(1.0/operands[0]); },
    "arcsec": function(operands) { return Math.acos(1.0/operands[0]); },
    "arccot": function(operands) { return Math.atan(1.0/operands[0]); },
    "csc": function(operands) { return 1.0/Math.sin(operands[0]); },
    "sec": function(operands) { return 1.0/Math.cos(operands[0]); },
    "cot": function(operands) { return 1.0/Math.tan(operands[0]); },
    "sqrt": function(operands) { return Math.sqrt(operands[0]); },
    "log": function(operands) { return Math.log(operands[0]); },
    "exp": function(operands) { return Math.exp(operands[0]); },    
    "^": function(operands) { return Math.pow(operands[0], operands[1]); },
    "abs": function(operands) { return Math.abs(operands[0]); },
    
    "factorial": function(operands) { return (new ComplexNumber(operands[0],0)).factorial().real_part(); },
    "gamma": function(operands) { return (new ComplexNumber(operands[0],0)).gamma().real_part(); },
    
    "apply": function(operands) { return NaN; },
};

function isProbablyZero( tree ) {

    
}


// modulus is the prime at level 0
function Levels(modulus)
{
    // modulus should be 1 mod 4, so that i exists
    
    this[0].modulus = modulus;
    this[0].characteristic = modulus;

    this[0].e = NumberTheory.primitiveRoot( modulus );
    this[0].g = NumberTheory.randomPrimitiveRoot( modulus );    
    this[0].pi = NumberTheory.randomPrimitiveRoot( modulus );
    this[0].i = NumberTheory.powerMod( g, NumberTheory.eulerPhi(modulus) / 4, modulus );

    this.height = 1;
}

Levels.prototype.extend = function()
{
    this[height].modulus = NumberTheory.eulerPhi( this[height-1].modulus );

    var characteristic = this[height].modulus + 1;
    while( ! NumberTheory.isProbablyPrime( characteristic ) )
	characteristic += this[height].modulus;

    this[height].pi = NumberTheory.eulerPhi( this[height-1].modulus ) / 2;
    
    this.height++;

    return this;
}


function Levels() {
    
}

var levels = [];

var establishLevel( levels ) {
    var level = levels[modulus];

    if (level === undefined) {
	levels[modulus] = {};
	level = levels[modulus];
    }

    level["e"] = NumberTheory.primitiveRoot( modulus );

    // I think I have to use "folding" to handle the higher levels?

    // Each level has a "modulus" (perhaps phi(p)) but also a "characteristic" (which is a prime q build so that q = k phi(p) + 1)
    // The primitive roots and logs are calculated via the characteristic, but the signatures are then reduced modulo the modulus.
}

function signature( tree, bindings, modulus ) {
    if (typeof tree === 'number') {
	// BADBAD: should handle floats via continued fractions
	return tree % modulus;
    }

    if (typeof tree === 'string') {
	if (tree === "e")
	    return Math.E;

	if (tree === "pi")
	    return Math.PI;

	if (tree in bindings)
	    return bindings[tree];
	
	return tree;
    }    
    
}



function evaluate_ast(tree, bindings) {
    if (typeof tree === 'number') {
	return tree;
    }

    if (typeof tree === 'string') {
	if (tree === "e")
	    return Math.E;

	if (tree === "pi")
	    return Math.PI;

	if (tree in bindings)
	    return bindings[tree];
	
	return tree;
    }    
    
    var operator = tree[0];
    var operands = tree.slice(1);

    if (operator in math_functions) {
	return math_functions[operator]( operands.map( function(v,i) { return evaluate_ast(v,bindings); } ) );
    }
    
    return NaN;
};


function allPairs( xs, ys, f ) {
    return _.flatten( _.map( xs, function(x) {
	return _.map( ys, function(y) {
	    return f(x,y);
	});
    }));
}

function FiniteFieldElement(element, modulus) {
    this.element = element;
    this.modulus = modulus;
}

FiniteFieldElement.prototype = {
    /* The modulus we are working with respect to
     * 
     * @type Number
     */
    modulus: 0,
    
    /* An elment in Z mod modulus
     * 
     * @type Number
     */
    element: 0,
    
    add: function(other) {
	var m = gcd(this.modulus, other.modulus);
	var x = this.element;
	var y = other.element;
	
	return new FiniteFieldElement( (x + y) % m, m );
    },

    subtract: function(other) {
	var m = gcd(this.modulus, other.modulus);
	var x = this.element;
	var y = other.element;
	
	return new FiniteFieldElement( (x - y + m) % m, m );	
    },    

    multiply: function(other) {
	var m = gcd(this.modulus, other.modulus);
	var x = this.element;
	var y = other.element;
	
	return new FiniteFieldElement( (x * y) % m, m );		
    },

    divide: function(other) {
	var m = gcd(this.modulus, other.modulus);
	var x = this.element;
	var y = other.element;
	
	return new FiniteFieldElement( (x * modInverse(y,m)) % m, m );			
    },


};

exports.FiniteFieldElement = FiniteFieldElement;

function FiniteFieldSet(set) {
    this.set = set;
}

FiniteFieldSet.prototype = {
    /* A set of elements drawn from a Finite Field */
    set: [],
};

function uniqueElements( xs ) {
    return _.uniq( xs, false, function(x) { return [x.element, x.modulus]; } );
}

_.each( _.keys( FiniteFieldElement.prototype ),
	function(operation) {
	    FiniteFieldSet.prototype[operation] = function() {
		if(arguments.length == 1) {
		    return new FiniteFieldSet( uniqueElements( allPairs( this.set, arguments[0].set, function(x,y) { return x[operation](y); } ) ) );
		} else {
		    return new FiniteFieldSet( uniqueElements( _.map( this.set, function(x) { return x[operation](); } )));
		}
	    };
	});


exports.FiniteFieldSet = FiniteFieldSet;
