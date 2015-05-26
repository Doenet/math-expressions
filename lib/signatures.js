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

/**
 * @param Number	real
 * @param Number	imaginary
 */

FiniteFieldSubset

var _ = require('underscore');

// Taken from wikipedia
function gcd(a,b) {
    if (a < 0) a = -a;
    if (b < 0) b = -b;
    if (b > a) {var temp = a; a = b; b = temp;}
    while (true) {
        if (b == 0) return a;
        a %= b;
        if (a == 0) return b;
        b %= a;
    }
}

// Taken from wikipedia
function modInverse(a, n) {
    var t = 0;
    var newt = 1;    
    var r = n;
    var newr = a;
    
    while(newr != 0) {
        var quotient = Math.floor(r/newr);

	var oldt = t;
	t = newt;
	newt = oldt - quotient * newt;

	var oldr = r;
	r = newr;
	newr = oldr - quotient * newr;
    }

    if(r > 1)
	return NaN;
    
    if (t < 0)
	t = t + n;
    
    return t;
}

function allPairs( xs, ys, f ) {
    return _.uniq( _.flatten( _.map( xs, function(x) {
	return _.map( ys, function(y) {
	    return f(x,y);
	});
    })));
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

	return new FiniteFieldElement( (x + y) % m, m );
    },

    subtract: function(other) {
	var m = gcd(this.modulus, other.modulus);

	return new FiniteFieldElement( (x - y + m) % m, m );	
    },    

    multiply: function(other) {
	var m = gcd(this.modulus, other.modulus);

	return new FiniteFieldElement( (x * y) % m, m );		
    },

    divide: function(other) {
	var m = gcd(this.modulus, other.modulus);
	
	return new FiniteFieldElement( (x * modInverse(y,m)) % m, m );			
    },
};

exports.FiniteFieldElement = FiniteFieldElement;

function FiniteFieldSubset(elements, modulus) {
    this.elements = elements;
    this.modulus = modulus;
}

FiniteFieldSubset.prototype = {
    /* The modulus we are working with respect to
     * 
     * @type Number
     */
    modulus: 0,
    
    /* The subset of the finite field Z mod modulus
     * 
     * @type Array of numbers
     */
    elements: [],
    
    add: function(other) {
	var m = gcd(this.modulus, other.modulus);
	
	return new FiniteFieldSubset( allPairs( this.elements, other.elements, function(x,y) { return (x + y) % m; } ), m );
    },

    subtract: function(other) {
	var m = gcd(this.modulus, other.modulus);
	
	return new FiniteFieldSubset( allPairs( this.elements, other.elements, function(x,y) { return (x - y + m) % m; } ), m );
    },    

    sub: this.subtract,
    
    multiply: function(other) {
	var m = gcd(this.modulus, other.modulus);
	
	return new FiniteFieldSubset( allPairs( this.elements, other.elements, function(x,y) { return (x * y) % m; } ), m );
    },

    mul: this.multiply,

    divide: function(other) {
	var m = gcd(this.modulus, other.modulus);
	
	return new FiniteFieldSubset( allPairs( this.elements, other.elements, function(x,y) { return (x * modInverse(y,m)) % m; } ), m );
    },
};

exports.FiniteFieldSubset = FiniteFieldSubset;
