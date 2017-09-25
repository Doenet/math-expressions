var parser = require('../parser');
var substitute_ast = require("./substitution").substitute_ast;

exports.f = function(expr, bindings) {
    return parser.ast.to.function( expr.tree )( bindings );
};
    
exports.evaluate = exports.f;

exports.finite_field_evaluate = function(expr, bindings, modulus) {
    return parser.ast.to.finiteField( expr.tree, modulus )( bindings );
};

/*
This must not be used as it reference syntax_tree
exports.substitute = function(bindings) {
    var ast_bindings = new Object();
    
    var alphabet = "abcdefghijklmnopqrstuvwxyz";
    for(var i=0; i<alphabet.length; i++) {
	var c = alphabet.charAt(i);
	if (c in bindings)
	    ast_bindings[c] = bindings[c].syntax_tree;
    }
    
    return Expression.fromAst( substitute_ast( this.syntax_tree, ast_bindings ) );
};

*/
