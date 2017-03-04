exports.equalsViaComplex = require('./equality/complex.js').equals;
exports.equalsViaReal = require('./equality/real.js').equals;
exports.equalsViaSyntax = require('./equality/syntax.js').equals;
//exports.equalsViaFiniteField = require('./equality/finite-field.js').equals;

exports.equals = function(other) {
    if (this.equalsViaSyntax(other)) {
	return true;
    } else if (this.equalsViaComplex(other)) {
	return true;
    } else if (this.equalsViaReal(other)) {
	return true;	
    } else {
	return false;
    }
};
