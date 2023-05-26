import math from '../../mathjs';
import seedrandom from 'seedrandom';

function randomFiniteFieldBindings(rng, variables, modulus) {
  var result = {};

  variables.forEach(function (v) {
    result[v] = Math.floor(rng() * modulus);
  });

  return result;
}

export const equals = function (expr, other,
  { allow_blanks = false
  } = {}) {

  if (!allow_blanks && (expr.variables().includes('\uFF3F') || other.variables().includes('\uFF3F'))) {
    return false;
  }

  let rng = seedrandom('finite_field_seed');

  let primes = [1181,
                1187,
                1193,
                1201,
                1213,
                1217,
                1223,
                1229,
                1231];

  var variables = [...new Set([...expr.variables(), ...other.variables()])];

  for (let prime of primes) {
    let bindings = randomFiniteFieldBindings( rng, variables, prime );
    try {
      let a = expr.finite_field_evaluate( bindings, prime );
      let b = other.finite_field_evaluate( bindings, prime );
      console.log(a,b);
      if (!(a.isNaN()) && !(b.isNaN()) && (a.values.length > 0) && (b.values.length > 0) && !(a.equals(b)))
        return false;
    }
    catch (e) {
      return undefined;
    }
  }
  
  return undefined;
};
