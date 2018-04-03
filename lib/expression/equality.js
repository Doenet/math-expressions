import { equals as equalsViaComplex } from './equality/complex.js';
import { equals as equalsViaReal } from './equality/real.js';
import { equals as equalsViaSyntax } from './equality/syntax.js';

//var equalsViaFiniteField = require('./equality/finite-field.js').equals;
import { equals as equalsDiscreteInfinite } from './equality/discrete_infinite_set';

export { equalsViaComplex, equalsViaReal, equalsViaSyntax };

//exports.equalsViaFiniteField = equalsViaFiniteField;

export const equals = function(expr, other) {
    if (expr.equalsViaSyntax(other)) {
	return true;
    } else if (expr.equalsViaComplex(other)) {
	return true;
    // } else if (expr.equalsViaReal(other)) {
    //   	return true;
    } else if(equalsDiscreteInfinite(expr,other)) {
	return true;
    } else {
	return false;
    }
};
