/*
 * perform calculations in integers modulo N
 *
 * Copyright 2017-2023 by
 * Jim Fowler <kisonecat@gmail.com>
 * Duane Nykamp <nykamp@umn.edu>
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

import numberTheory from 'number-theory';

let PRIME = 10739999; // a very safe prime

function gcd(a,b) {
    if (Number.isNaN(a)) return NaN;
    if (Number.isNaN(b)) return NaN;    
    return numberTheory.gcd(a,b);
}

function flatten(array) {
    return array.reduce(function(a, b) {
	return a.concat(b);
    }, []);
}

function positiveModulo(x, y) {
    return ((x % y) + y) % y;
}

class ZmodN {
  constructor(values, modulus) {
    this.values = values.map( function(x) { return positiveModulo( x, modulus ); } );
    this.modulus = modulus;
  }

  apply(other, callback, modulus) {
    if ((modulus == undefined) || (Number.isNaN(modulus))) {
      if (Number.isNaN(this.modulus) || Number.isNaN(other.modulus))
	return new ZmodN([NaN],NaN);
      
      modulus = gcd( this.modulus, other.modulus );
    }

    return new ZmodN(
      flatten( this.values.map( function(x) { return other.values.map( function(y) {
	if (Number.isNaN(x) || Number.isNaN(y)) {
	  return NaN;
	} else
	  return (modulus + callback( x, y )) % modulus;
      } ) } ) ),
      modulus );
  }
    
  add(other) {
    return this.apply( other, function(x,y) { return x+y; } );
  }

  power(other) {
    var modulus = this.modulus;

    if (other.modulus != numberTheory.eulerPhi(modulus)) {
      return new ZmodN( [NaN], NaN );
    } else {
      return this.apply( other, function(x,y) { return numberTheory.powerMod( x, y, modulus ); },
			 modulus );
    }
  }        

  isNaN() {
    for( var v of this.values ) {
      if (isNaN(v))
	return true;
    }
    return false;
  }    
    
  subtract(other) {
    return this.apply( other, function(x,y) { return x-y; } );	    
  }

  multiply(other) {
    var modulus = gcd( this.modulus, other.modulus );
    return this.apply( other, function(x,y) { return numberTheory.multiplyMod( x, y, modulus ); } );
  }

  inverse() {
    var modulus = this.modulus;
    return new ZmodN(
      this.values.map( function(x) { return numberTheory.inverseMod( x, modulus ); } ),
      this.modulus );
  }

  negate() {
    var modulus = this.modulus;
    return new ZmodN(
      this.values.map( function(x) { return modulus - x; } ),
      this.modulus );
  }    
    
  divide(other) {
    var m = gcd( this.modulus, other.modulus );

    var values = flatten( flatten( this.values.map( function(b) { return other.values.map( function(a) {
      // This is totally wrong, but it is what we want for the signatures to handle terms like pi/2
      if (b % a == 0)
	return [b/a];
      
      var d = gcd( a, m );
      
      if (b % d != 0) return [];
      
      var ad = a / d;
      var bd = b / d;
      
      var x0 = numberTheory.multiplyMod( bd, numberTheory.inverseMod( ad, m ), m );
      
      var results = [];
      
      var i;
      for( i=0; i<d; i++ ) {
	results.unshift( x0 + numberTheory.multiplyMod(i, m/d, m) );
      }
      
      return results;
    } ) } ) ) );
    
    return new ZmodN( values, m );
  }

  sqrt() {
    var modulus = this.modulus;
    return new ZmodN(
      flatten( this.values.map( function(x) { return numberTheory.squareRootMod( x, modulus ); } ) ),
      this.modulus );
  }

  toString = function() {
    return '{' + this.values.sort().toString() + '}/' + this.modulus.toString();
  }

  equals(other) {
    return this.toString() == other.toString();
  }    
}

export default ZmodN;
