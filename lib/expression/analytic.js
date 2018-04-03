import { get_tree } from '../trees/util';
import { operators } from './variables';
import { functions } from './variables';

var analytic_operators = ['+', '-', '*', '/', '^'];
var analytic_functions = ["exp", "log", "log10", "sqrt", "factorial", "gamma", "erf", "acos", "acosh", "acot", "acoth", "acsc", "acsch", "asec", "asech", "asin", "asinh", "atan", "atanh", "cos", "cosh", "cot", "coth", "csc", "csch", "sec", "sech", "sin", "sinh", "tan", "tanh", 'arcsin', 'arccos', 'arctan', 'arccsc', 'arcsec', 'arccot', 'cosec'];


function isAnalytic(expr_or_tree, params) {

    var tree = get_tree(expr_or_tree);

    var allow_abs = false;
    if (params !== undefined && params.hasOwnProperty('allow_abs'))
	allow_abs = params['allow_abs'];
    
    var operators_found = operators(tree);
    for (var i=0; i < operators.length; i++ ) {
	var oper = operators[i];
	if(analytic_operators.indexOf(oper) === -1)
	    return false;
    }

    var functions = functions(tree);
    for (var i=0; i < functions.length; i++ ) {
	var fun = functions[i];
	if(analytic_functions.indexOf(fun) === -1) {
	    if((!allow_abs) || fun !== "abs")
		return false;
	}
    }

    return true;
}

export { isAnalytic };
