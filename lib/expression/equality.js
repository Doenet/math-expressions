exports.equalsViaComplex = require('./equality/complex.js').equals;
exports.equalsViaReal = require('./equality/real.js').equals;
exports.equalsViaSyntax = require('./equality/syntax.js').equals;
//exports.equalsViaFiniteField = require('./equality/finite-field.js').equals;
var equalsDiscreteInfinite = require('./equality/discrete_infinite_set').equals;

exports.equals = function(expr, other) {
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
