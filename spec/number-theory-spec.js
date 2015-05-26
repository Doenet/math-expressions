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
	
    it("gcd(34,51) == 17", function() {     
        expect(NumberTheory.gcd(34,51)).toEqual(17);
    });

    it("(1/7) * 7 = 1 mod 13", function() {
        expect((NumberTheory.inverseMod(7,13) * 7) % 13).toEqual(1);
    });

    it("factor(38675) = [5,5,7,13,17]", function() {
        expect(NumberTheory.factor(38675).sort()).toEqual([5,5,7,13,17].sort());
    });

    it("2**(123456789) mod 104743 = 12373", function() {
        expect(NumberTheory.powerMod(2, 123456789, 104743)).toEqual(12373);
    });

    it("phi(351135) == 176256", function() {
        expect(NumberTheory.eulerPhi(351135)).toEqual(176256);
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
    });

    
    _.each( [467, 479, 487, 491, 499, 503, 509, 521, 523, 541, 547, 557, 563, 569, 571, 577, 587, 593, 599, 601], function(a) {
	_.each( [1229, 1231, 1237, 1249, 1259, 1277, 1279, 1283, 1289, 1291, 1297, 1301, 1303, 1307, 1319, 1321, 1327, 1361, 1367, 1373],
		function( p ) {
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
	    });

    
});
