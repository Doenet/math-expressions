var parser = require('../parser');

exports.f = function(expr) {
    return parser.ast.to.function( expr.tree );
};
    
exports.evaluate = function(expr, bindings) {
    return exports.f(expr)(bindings);
}

exports.finite_field_evaluate = function(expr, bindings, modulus) {
    return parser.ast.to.finiteField( expr.tree, modulus )( bindings );
};
