
var operators_in_ast = require('./variables')._operators_in_ast;
var functions_in_ast = require('./variables')._functions_in_ast;

var analytic_operators = ['+', '-', '*', '/', '^'];
var analytic_functions = ["exp", "log", "log10", "sqrt", "factorial", "gamma", "erf", "acos", "acosh", "acot", "acoth", "acsc", "acsch", "asec", "asech", "asin", "asinh", "atan", "atanh", "cos", "cosh", "cot", "coth", "csc", "csch", "sec", "sech", "sin", "sinh", "tan", "tanh"];


function is_analytic_ast(tree) {

    var operators = operators_in_ast(tree);
    for (var i=0; i < operators.length; i++ ) {
	var oper = operators[i];
	if(analytic_operators.indexOf(oper) === -1)
	    return false;
    }

    var functions = functions_in_ast(tree);
    for (var i=0; i < functions.length; i++ ) {
	var fun = functions[i];
	if(analytic_functions.indexOf(fun) === -1)
	    return false;
    }

    return true;
}


exports._is_analytic_ast = is_analytic_ast;

exports.isAnalytic = function(expr) {
    return is_analytic_ast(expr.tree);
}
