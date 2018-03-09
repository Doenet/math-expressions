var equalsViaComplex = require('./equality/complex.js').equals;
var equalsViaReal = require('./equality/real.js').equals;
var equalsViaSyntax = require('./equality/syntax.js').equals;
//var equalsViaFiniteField = require('./equality/finite-field.js').equals;
var equalsDiscreteInfinite = require('./equality/discrete_infinite_set').equals;

exports.equalsViaComplex = equalsViaComplex;
exports.equalsViaReal = equalsViaReal;
exports.equalsViaSyntax = equalsViaSyntax;
//exports.equalsViaFiniteField = equalsViaFiniteField;

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
