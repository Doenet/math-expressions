var parser = require('../parser');
var get_tree = require('../trees/util').get_tree;

exports.f = function(expr) {
    return parser.ast.to.function( expr.tree );
};
    
exports.evaluate = function(expr, bindings) {
    return exports.f(expr)(bindings);
}

exports.finite_field_evaluate = function(expr, bindings, modulus) {
    return parser.ast.to.finiteField( expr.tree, modulus )( bindings );
};

exports.evaluate_to_constant = function(expr_or_tree) {
    // evaluate to number by converting tree to number
    // and calling without arguments

    // return null if couldn't evaluate to constant (e.g., contains a variable)
    // otherwise returns constant
    // NOTE: constant could be a math.js complex number object

    var tree = get_tree(expr_or_tree);

    var f= parser.ast.to.function(tree);
    
    var num=null;
    try {
	num = f();
    }
    catch (e) {};

    return num;
}
