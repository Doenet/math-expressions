/*
 * test code for number theory functions
 *
 * Copyright 2014-2015 by Jim Fowler <kisonecat@gmail.com>
 *
 * This file is part of a math-expressions library
 * 
 * Some open source application is free software: you can redistribute
 * it and/or modify it under the terms of the GNU General Public
 * License as published by the Free Software Foundation, either
 * version 3 of the License, or at your option any later version.
 * 
 * Some open source application is distributed in the hope that it
 * will be useful, but WITHOUT ANY WARRANTY; without even the implied
 * warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details.
 * 
 */

var NumberTheory = require('../lib/number-theory');
var _ = require('underscore');

describe("number theory", function() {

    // BADBAD: Missing test for random primitive roots
    
    // for k in ps[52], 2^52 - k is prime
    var ps = {
	51: [129, 139, 165, 231, 237, 247, 355, 391, 397, 439],
	52: [47, 143, 173, 183, 197, 209, 269, 285, 335, 395],
	50: [27, 35, 51, 71, 113, 117, 131, 161, 195, 233]
    };

    _.each( _.keys( ps ), function( exponent ) {
	_.each( ps[exponent], function(k) {
	    var p = Math.pow(2,exponent) - k;

	    it("p = 2^" + exponent + "-" + k + " is probably prime", function() {		
		expect(NumberTheory.isProbablyPrime(p)).toBeTruthy();
	    });

	    it("p = 2^" + exponent + "-" + k + "; (p-1)**(p-2) == 2 modulo p", function() {
		expect(NumberTheory.multiplyMod(p - 1, p - 2, p)).toEqual(2);
	    });

	    it("p = 2^" + exponent + "-" + k + "; ((p-1)/2)**(128) == p - 64 modulo p", function() {
		expect(NumberTheory.multiplyMod((p - 1)/2, 128, p)).toEqual( p - 64 );
	    });

	    it("p = 2^" + exponent + "-" + k + "; (p-3)**(p-5) == 15 modulo p", function() {
		expect(NumberTheory.multiplyMod(p - 3, p - 5, p)).toEqual(15);
	    });

	    it("p = 2^" + exponent + "-" + k + "; 17 * (1/17) == 1 modulo p", function() {
		expect(NumberTheory.multiplyMod( 17, NumberTheory.inverseMod(17, p), p )).toEqual(1);
	    });

	    it("p = 2^" + exponent + "-" + k + "; (1/11) * (1/17) == (1/187) modulo p", function() {
		expect(NumberTheory.multiplyMod( NumberTheory.inverseMod(11, p), NumberTheory.inverseMod(17, p), p )).toEqual(NumberTheory.inverseMod(187, p));
	    });	    	    

	    _.each( ps[exponent], function(j) {
		var q = Math.pow(2,exponent) - j;

		if (p != q) {
		    it("p = 2^" + exponent + "-" + k + "; q = 2^" + exponent + "-" + j + "; gcd(p,q) == 1", function() {
			expect(NumberTheory.gcd(p,q)).toEqual(1);
		    });
		}
	    });
	});
    });
    
    it("(2^52 - p - 1)  == 1 modulo 2^52", function() {
        expect(NumberTheory.gcd(34,51)).toEqual(17);
    });    
    
    it("gcd(34,51) == 17", function() {     
        expect(NumberTheory.gcd(34,51)).toEqual(17);
    });

    it("gcd(-34,51) == 17", function() {     
        expect(NumberTheory.gcd(-34,51)).toEqual(17);
    });

    it("gcd(-34,-51) == 17", function() {     
        expect(NumberTheory.gcd(-34,-51)).toEqual(17);
    });

    it("gcd(34,-51) == 17", function() {     
        expect(NumberTheory.gcd(34,-51)).toEqual(17);
    });            

    it("(1/7) * 7 = 1 mod 13", function() {
        expect((NumberTheory.inverseMod(7,13) * 7) % 13).toEqual(1);
    });

    it("(1/-7) * 7 = 1 mod 13", function() {
        expect((NumberTheory.inverseMod(-7,13) * 7) % 13).toEqual(12);
    });    

    it("factor(38675) = [5,5,7,13,17]", function() {
        expect(NumberTheory.factor(38675).sort()).toEqual([5,5,7,13,17].sort());
    });

    it("2**(123456789) mod 104743 = 12373", function() {
        expect(NumberTheory.powerMod(2, 123456789, 104743)).toEqual(12373);
    });

    it("2**(13395) mod 179424779 = 59783755", function() {
        expect(NumberTheory.powerMod(2, 13395, 179424779)).toEqual(59783755);
    });

    it("2**926865135 mod 2038074803 = 513", function() {
        expect(NumberTheory.powerMod(2, 926865135, 2038074803)).toEqual(513);
    });        

    
    it("2**(-123456789) * 12373 mod 104743 = 1", function() {
        expect((NumberTheory.powerMod(2, -123456789, 104743) * 12373) % 104743).toEqual(1);
    });    

    it("phi(351135) == 176256", function() {
        expect(NumberTheory.eulerPhi(351135)).toEqual(176256);
    });

    it("2 generates the units in Z/48619", function () {
	expect(NumberTheory.primitiveRoot(48619)).toEqual(2);
    });

    _.each( [1229, 1231, 1237, 1249, 1259, 1277, 1279, 1283, 1289, 1291, 1297, 1301, 1303, 1307, 1319, 1321, 1327, 1361, 1367, 1373],
	    function(p) {
		it("Fermat's little theorem mod " + p.toString() + " holds", function() {
		    expect(NumberTheory.powerMod( 2, NumberTheory.eulerPhi(p), p )).toEqual(1);
		});
	    });

    _.each( [467, 479, 487, 491, 499, 503, 509, 521, 523, 541, 547, 557, 563, 569, 571, 577, 587, 593, 599, 601], function(p) {
	it("primitive root mod " + p.toString() + " is primitive", function() {
	    var root = NumberTheory.primitiveRoot(p);

	    for( var i=1; i < p - 1; i++ )
		expect(NumberTheory.powerMod(root, i, p)).not.toEqual(1);
	    
            expect(NumberTheory.powerMod(root, p - 1, p)).toEqual(1);
	});

	it("random primitive root mod " + p.toString() + " is primitive", function() {
	    var root = NumberTheory.randomPrimitiveRoot(p);

	    for( var i=1; i < p - 1; i++ )
		expect(NumberTheory.powerMod(root, i, p)).not.toEqual(1);
	    
            expect(NumberTheory.powerMod(root, p - 1, p)).toEqual(1);
	});                    	
    });

    it("jacobiSymbol(2,112272535095293) == -1", function() {
	expect(NumberTheory.jacobiSymbol(2,112272535095293)).toEqual(-1);
    });
    
    _.each( [467, 479, 487, 491, 499, 503, 509, 521, 523, 541, 547, 557, 563, 569, 571, 577, 587, 593, 599, 601], function(a) {
	_.each( [1229, 1231, 1237, 1249, 1259, 1277, 1279, 1283, 1289, 1291, 1297, 1301, 1303, 1307, 1319, 1321, 1327, 1361, 1367, 1373],
		function( p ) {
		    it(a.toString() + " * " + p.toString() + " is not prime", function() {
			expect(NumberTheory.isPrime(a*p)).not.toBeTruthy();
		    });

		    it(a.toString() + " * " + p.toString() + " is not (probably) prime", function() {
			expect(NumberTheory.isProbablyPrime(a*p)).not.toBeTruthy();
		    });		    
		    
		    it("(" + a.toString() + " on " + p.toString() + ") = (-1)**((" + p.toString() + "-1)/2)", function() {
			expect((NumberTheory.jacobiSymbol(a,p) + p) % p).toEqual(NumberTheory.powerMod(a, (p-1)/2, p));
		    });

		    if (NumberTheory.jacobiSymbol(a,p) == 1) {
			it("NumberTheory.squareRootModPrime(" + a.toString() + "," + p.toString() + ") is a square root", function() {
			    var r = NumberTheory.squareRootModPrime(a,p);
			    expect((r * r) % p).toEqual(a % p);
			});
		    }
		});
    });

    _.each( [1229, 1231, 1237, 1249, 1259, 1277, 1279, 1283, 1289, 1291, 1297, 1301, 1303, 1307, 1319, 1321, 1327, 1361, 1367, 1373],
	    function( p ) {
		it("quadraticNonresidue(" + p.toString() + ") is really a nonresidue", function () {
		    expect(NumberTheory.jacobiSymbol(NumberTheory.quadraticNonresidue(p),p)).toEqual(-1);
		});

		it(p.toString() + " is prime", function () {
		    expect(NumberTheory.isPrime(p)).toBeTruthy();
		});

		it(p.toString() + " is probably prime", function () {
		    expect(NumberTheory.isProbablyPrime(p)).toBeTruthy();
		});		
	    });

    ////////////////////////////////////////////////////////////////
    // a bunch of discrete log problems
    var primesGeneratedBy = {
	17: [50111, 51241, 51481, 52009, 52081, 52201, 52321, 53759, 56809, 58439, 59809],
	2: [5179, 5189, 5227, 5261, 5309, 5333, 5387, 5443, 5477, 5483, 5501, 179424779],
	31: [53089, 53881],
	29: [46489, 47041, 47881]
    };
    
    _.each( _.keys( primesGeneratedBy ), function( base ) {
	_.each( primesGeneratedBy[base], function(modulus) {
	    _.each( [100, 200, 300, 400, 234, 1, 10, 17, 137, -1, -2, -3, -4], function(goal) {
		it(base.toString() + "**(log_" + base.toString() + " " + goal.toString() + " mod " + modulus.toString() + ") equiv " + goal.toString() + " mod " + modulus.toString(), function () {
		    expect(NumberTheory.powerMod(base, NumberTheory.discreteLog(goal, base, modulus), modulus)).toEqual((goal + modulus) % modulus);
		});
	    });
	});
    });

});

