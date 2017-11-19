// standard functions
// these functions are all defined in math.js

var get_tree = require('../trees/util').get_tree;

// functions with one argument 
var functions = ["abs", "exp", "log", "log10", "sign", "sqrt", "conj", "im", "re", "factorial", "gamma", "erf", "acos", "acosh", "acot", "acoth", "acsc", "acsch", "asec", "asech", "asin", "asinh", "atan", "atanh", "cos", "cosh", "cot", "coth", "csc", "csch", "sec", "sech", "sin", "sinh", "tan", "tanh"];


//"intersect", "cross", "det", "diag", "dot", "eye", " inv", " sort", " trace", " transpose", "max", "mean", "median", "min", "mode", "nthRoot"
// "ceil", "fix", "floor", "round"

for (var i=0; i < functions.length; i++) {
    (function() {
	var fun = functions[i];
	module.exports[fun] = function(expr_or_tree) {
	    return ['apply', fun, get_tree(expr_or_tree)];
	}
    }());
}

// function of two variables
module.exports['atan2'] =  function(expr_or_tree1, expr_or_tree2) {
    return ['apply', 'atan2', ['tuple', get_tree(expr_or_tree1),
			       get_tree(expr_or_tree2)]];
}
