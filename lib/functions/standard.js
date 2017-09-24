// standard functions
// these functions are all defined in math.js

var Expression = require('../math-expressions');

// functions with one argument 
var functions = ["abs", "exp", "log", "log10", "sign", "sqrt", "conj", "im", "re", "factorial", "gamma", "erf", "acos", "acosh", "acot", "acoth", "acsc", "acsch", "asec", "asech", "asin", "asinh", "atan", "atanh", "cos", "cosh", "cot", "coth", "csc", "csch", "sec", "sech", "sin", "sinh", "tan", "tanh"];


//"intersect", "cross", "det", "diag", "dot", "eye", " inv", " sort", " trace", " transpose", "max", "mean", "median", "min", "mode", "nthRoot"
// "ceil", "fix", "floor", "round"

for (var i=0; i < functions.length; i++) {
    (function() {
	var fun = functions[i];
	module.exports[fun] = function(expr) {
	    return Expression.from(['apply', fun, expr.tree]);
	}
    }());
}

// function of two variables
module.exports['atan2'] =  function(expr1, expr2) {
    return Expression.from(['apply', 'atan2', ['tuple', expr1.tree, expr2.tree]]);
}
