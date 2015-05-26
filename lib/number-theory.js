/*
 * some number theory functions
 *
 * Copyright 2014-2015 by Jim Fowler <kisonecat@gmail.com>
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

////////////////////////////////////////////////////////////////
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

exports.gcd = gcd;

////////////////////////////////////////////////////////////////
// Taken from wikipedia
function inverseMod(a, n) {
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

exports.inverseMod = inverseMod;


////////////////////////////////////////////////////////////////
// Pollard's rho
// from http://userpages.umbc.edu/~rcampbel/NumbThy/Class/Programming/JavaScript/
var findFactor = _.memoize(function(x) {
    var numsteps=2*Math.floor(Math.sqrt(Math.sqrt(x))), slow=2, fast=slow, i, thegcd;
    for (i=1; i<numsteps; i++){
        slow = (slow*slow + 1) % x;
        fast = (fast*fast + 1) % x;
        fast = (fast*fast + 1) % x;
        if((thegcd=gcd(fast-slow,x)) != 1) {return thegcd;}
    };
    return 1;
});
			   
exports.findFactor = findFactor;			   
			   
////////////////////////////////////////////////////////////////
var factor = _.memoize(function(x) {
    if (x < 0) return factor(-x);
    if (x <= 1) return [];

    var i;
    var bound = Math.floor(Math.sqrt(x)); // being paranoid

    // Pollard's rho sometimes fails (e.g., when x is prime)
    i = findFactor(x);
    if ((i > 1) && (i < x))
	return factor(i).concat( factor(x/i) );

    // Trial division in that case
    for( i=2; i <= bound; i++ ) {
	if ((x % i) == 0) {
	    return factor(i).concat( factor(x/i) );
	}
    }
    
    return [x];
});

exports.factor = factor;

////////////////////////////////////////////////////////////////
// From http://en.wikipedia.org/wiki/Exponentiation_by_squaring
function powerMod(x,n,m) {
	var result = 1;
	while (n != 0) {
	    if ( n % 2 == 1 ) {
		result = (result * x) % m;
		n -= 1;
	    }
	    x = (x * x) % m;
	    n /= 2;
	}
	return result;
}

exports.powerMod = powerMod;

////////////////////////////////////////////////////////////////
// Euler phi function
var eulerPhi = _.memoize(function(x) {
    var product = function( xs ) { return _.reduce(xs, function(memo, num){ return memo * num; }, 1); };
    var factors = _.uniq( factor(x) );
    return x * product( _.map( factors, function(p) { return (p - 1); } ) ) / product( factors );
});

exports.eulerPhi = eulerPhi;

////////////////////////////////////////////////////////////////
var primitiveRoot = _.memoize(function(modulus) {
    var phi_m = eulerPhi(modulus);

    var factors = _.uniq( factor( phi_m ) );

    for( var x=2; x < modulus; x++ )
	if (_.every( factors, function(p) { return powerMod( x, phi_m / p, modulus ) != 1; } ))
	    return x;

    return NaN;
});

exports.primitiveRoot = primitiveRoot;

////////////////////////////////////////////////////////////////
var jacobiSymbol = function(a,b) {
    if (b % 2 == 0) return NaN;
    if (b < 0) return NaN;

    // (a on b) is independent of equivalence class of a mod b
    if (a < 0)
	a = ((a % b) + b);
    
    var flips = 0;
    
    while(true) {
	a = a % b;

	// (0 on b) = 0
	if (a == 0)
	    return 0;

	// Calculation of (2 on b)
	while ((a % 2) == 0) {
	    flips += (b*b - 1)/8;
	    a /= 2;
	}

	// (1 on b) = 1
	if (a == 1)
	    return ((flips % 2) == 0) ? 1 : (-1);

	// Now a and b are coprime and odd, so "QR" applies
	flips += (a-1) * (b-1) / 4;

	var temp = a;
	a = b;
	b = temp;
    }

    // Cannot get here
    return NaN;    
}

exports.jacobiSymbol = jacobiSymbol;

////////////////////////////////////////////////////////////////
var quadraticNonresidue  = _.memoize(function(p) {
    for( var x = 2; x < p; x++ ) {
	if (jacobiSymbol(x,p) == -1)
	    return x;
    }
});

exports.quadraticNonresidue = quadraticNonresidue;

////////////////////////////////////////////////////////////////
// http://en.wikipedia.org/wiki/Tonelli–Shanks_algorithm
//
// I am often employing this in a situation where I have a table for
// discrete logs, so I should just use that table
var squareRootModPrime  = function(n,p) {
    if (jacobiSymbol(n,p) != 1)
	return NaN;

    var Q = p - 1;
    var S = 0;
    while( (Q % 2) == 0 ) {
	Q /= 2;
	S++;
    }

    // Now p - 1 = Q 2^S and Q is odd.

    if ((p % 4) == 3)
	return powerMod( n, (p+1)/4, p );
    
    // So S != 1 (since in that case, p equiv 3 mod 4
    var z = quadraticNonresidue(p);

    var c = powerMod(z, Q, p);

    var R = powerMod(n, (Q+1)/2, p);
    var t = powerMod(n, Q, p);
    var M = S;

    while(true) {

	if ((t % p) == 1) return R;

	// Find the smallest i (0 < i < M) such that t^{2^i} = 1
	var u = t;
	for( var i = 1; i < M; i++ ) {
	    u = (u * u) % p;
	    if (u == 1) break;
	}

	var minimum_i = i;
	i++;
	
	// Set b = c^{2^{M-i-1}}
	var b = c;
	while( i < M ) {
	    b = (b * b) % p;
	    i++;
	}

	M = minimum_i;
	R = (R * b) % p;
	t = (t * b * b) % p;
	c = (b * b) % p;
    }
    
    return NaN;
}

exports.squareRootModPrime = squareRootModPrime;

////////////////////////////////////////////////////////////////
var chineseRemainder = function(n,modulus) {
    
}

exports.squareRootMod = squareRootMod;

////////////////////////////////////////////////////////////////
var squareRootMod = function(n,modulus) {
    var m = 1;
    var results = [];
    
    _.each( factor( modulus ), function(p) {
	var s = squareRootModPrime( n, p );

	var combined = [];

	if (gcd(m,p) == 1) {
	    // Chinese remainder theorem	    
	    _.each( results, function(r) {
		// find a lift of r mod m and s mod p
		combined.unshift( r * p * inverseMod( p, m ) + s * m * inverseMod( m, p ) );
	    });
	} else {
	    // Hensel's lemma
	    _.each( results, function(r) {
	    });	    
	}

	results = _.uniq( combined );
	m = m * p;
    });

    return results;
}

exports.squareRootMod = squareRootMod;
