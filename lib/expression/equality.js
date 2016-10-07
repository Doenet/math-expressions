exports.equalsViaComplex = require('./equality/complex.js').equals;

exports.equals = function(other) {
    return this.equalsViaComplex(other);
};
