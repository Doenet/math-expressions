var Expression = require('../math-expressions');
var parser = require('../parser');

exports.f = function(bindings) {
    return parser.ast.to.function( this.tree )( bindings );
};
    
exports.evaluate = exports.f;

exports.complex_evaluate = function(bindings) {
    return parser.ast.to.complexFunction( this.tree )( bindings );
};

exports.real_evaluate = function(bindings) {
    return parser.ast.to.realFunction( this.tree )( bindings );
};

exports.finite_field_evaluate = function(bindings, modulus) {
    return parser.ast.to.finiteField( this.tree, modulus )( bindings );
};

function substitute_ast(tree, bindings) {
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	if (tree in bindings)
	    return bindings[tree];
	
	return tree;
    }    
    
    var operator = tree[0];
    var operands = tree.slice(1);
    
    var result = [operator].concat( operands.map( function(v,i) { return substitute_ast(v,bindings); } ) );
    return result;
};

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

