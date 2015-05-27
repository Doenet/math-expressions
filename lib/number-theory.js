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
// Number.MAX_SAFE_INTEGER == 9007199254740991, so multiplication would work if the modulus is less than sqrt(Number.MAX_SAFE_INTEGER) approx 94906265

function multiplyMod( a, b, m ) {
    // For small enough numbers, we can multiply without overflowing
    if ((a < 94906265) && (b < 94906265))
	return (a*b) % m;
    
    var d = 0;
    var mp2 = m / 2; // Bitshifts in javascript reduce everything to 32-bit ints, but with division we can get 53-bit of resolutions as a float
    
    if (a >= m) a %= m;
    if (b >= m) b %= m;
    
    for (var i = 0; i < 53; i++) {
	d = (d >= mp2) ? (2 * d - m) : (2 * d);
	
	if (a >= 4503599627370496) {
            d += b;
	    a = a - 4503599627370495;
	}
	
	if (d > m) d -= m;
	
	a *= 2;
    }
    
    return d;
}

exports.multiplyMod = multiplyMod;

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
    // The code below fails on negative inputs
    if (a < 0)
	a = (a % n) + n;

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
// Could be optimized
var isPrime = _.memoize(function(p) {
    return (factor(p).length == 1);
});

exports.isPrime = isPrime;

////////////////////////////////////////////////////////////////
// From http://en.wikipedia.org/wiki/Exponentiation_by_squaring
function powerMod(x,n,m) {
    if (n < 0) {
	return inverseMod(powerMod(x,-n,m),m);
    }
    
    var result = 1;
    while (n != 0) {
	if ( n % 2 == 1 ) {
	    //result = (result * x) % m;
	    result = multiplyMod( result, x, m );
	    n -= 1;
	}
	//x = (x * x) % m;
	x = multiplyMod( x, x, m );2,112272535095293
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
	    // b could be so large that b*b overflows
	    flips ^= ((b % 8)*(b % 8) - 1)/8;
	    a /= 2;
	}

	// (1 on b) = 1
	if (a == 1)
	    return (flips & 1) ? (-1) : 1;

	// Now a and b are coprime and odd, so "QR" applies
	// By reducing modulo 4, I avoid the possibility that (a-1)*(b-1) overflows
	flips ^= ((a % 4)-1) * ((b % 4)-1) / 4;

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
var squareRootMod = function(n,modulus) {
    var m = 1;
    var results = [0];
    
    _.each( factor( modulus ), function(p) {
	var s = squareRootModPrime( n, p );

	if (gcd(m,p) == 1) {
	    // Chinese remainder theorem
	    var combined = [];

	    _.each( results, function(r) {
		// find a lift of r mod m and s mod p
		combined.unshift( r * p * inverseMod( p, m ) + s * m * inverseMod( m, p ) );
		combined.unshift( r * p * inverseMod( p, m ) - s * m * inverseMod( m, p ) );
	    });

	    results = _.uniq( combined );
	} else {
	    // Hensel's lemma

	    /*
	    Set f(x) = x^2 - n
	    
	    Then f(r + t*m) = (r + t*m)^2 - n
	      = r^2 - n + 2*r*t*m + t*m*t*m
	      = f(r) + 2*r*t*m + t*m*t*m

	    So we want to find t so that 0 equiv f(r+t*m) equiv f(r) + 2*r*t*m mod (m*p)

	    Now f(r) = z m, so

	    0 equiv (z + 2*r*t)*m mod (m*p)

	    so p divides (z + 2*r*t)

	    so solving for t yields  t = (-z) * (1/(2*r)) mod p

	    and z = f(r) / m

	    giving the formula
	    */
	    
	    results = _.map( results, function(r) {
		return r + ((-((r*r - n) / m) * inverseMod(2 * r, p) ) % p) * m;
	    });
	}

	m = m * p;
    });

    return _.map( results, function(r) { return ((r % modulus) + modulus) % modulus; });
}

exports.squareRootMod = squareRootMod;

////////////////////////////////////////////////////////////////
// to cache the discrete log tables
var babyStepGiantStepTables = {};

function discreteLog( x, generator, modulus ) {
    // normalize x to be positive
    x = ((x % modulus) + modulus) % modulus;
    
    var m = Math.ceil(Math.sqrt(modulus));

    var hash = {};

    if (babyStepGiantStepTables[modulus] === undefined) {
	babyStepGiantStepTables[modulus] = {};
    }

    if (babyStepGiantStepTables[modulus][generator] === undefined) {
	babyStepGiantStepTables[modulus][generator] = {};

	hash = babyStepGiantStepTables[modulus][generator];
	
	for( var j = 0; j < m; j++ ) {
            // Compute generatorj and store the pair (j, generator^j) in a table.
	    hash[powerMod( generator, j, modulus )] = j;
	}
    } else {
	hash = babyStepGiantStepTables[modulus][generator];
    }

    var generatorInverseM = powerMod( generator, -m, modulus );
    
    var gamma = x;

    for( var i = 0; i < m; i++ ) {
	// Check to see if gamma is the second component (generatorj) of any pair in the table.
	if (hash[gamma] !== undefined) {
            //If so, return im + j.
	    return (multiplyMod( i, m, modulus ) + hash[gamma]) % modulus;
	} else {
            //If not, gamma ← gamma • generator−m.
	    gamma = multiplyMod( gamma, generatorInverseM, modulus );
	}
    }

    return NaN;
}

exports.discreteLog = discreteLog;
