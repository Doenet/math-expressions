var _ = require('underscore');
var variables_in_ast = require('../expression/variables')._variables_in_ast;
var functions_in_ast = require('../expression/variables')._functions_in_ast;

function is_integer(expression, assumptions) {
    // returns true if
    //   - expression is a literal integer
    //   - expression is a variable explicitly assumed to be integer
    //   - expression is a product, sum, power, or negation of the above
    // returns false if
    //   - expression is a literal non-integer number
    //   - expression is a variable explicitly assumed to be non-integer
    //   - expression involves an operator that doesn't return a number
    // otherwise, return undefined
    //
    // expression is a math-expression
    //
    // if assumptions in undefined, get assumptions from expression context

    if(assumptions === undefined)
	assumptions = expression.context.assumptions;

    return is_integer_ast(expression.tree, assumptions);

}

function is_integer_ast(tree, assumptions) {
    // see description of is_integer

    if(!Array.isArray(assumptions))
	return undefined;

    if(typeof tree === 'number')
	return Number.isInteger(tree);

    if(typeof tree === 'string') {

	var assume_operator = assumptions[0];
	var assume_operands = assumptions.slice(1);

	var negate_assumptions = false;
	while(assume_operator === 'not') {
	    negate_assumptions = !negate_assumptions;
	    assumptions = assume_operands[0];
	    assume_operator = assumptions[0];
	    assume_operands = assumptions.slice(1);
	}
	
	if(assume_operator === 'in')
	    if(assume_operands[0]===tree && assume_operands[1] === 'Z')
		return !negate_assumptions;
	if(assume_operator === 'ni')
	    if(assume_operands[1]===tree && assume_operands[0] === 'Z')
		return !negate_assumptions;
	if(assume_operator === 'notin')
	    if(assume_operands[0]===tree && assume_operands[1] === 'Z')
		return negate_assumptions;
	if(assume_operator === 'notni')
	    if(assume_operands[1]===tree && assume_operands[0] === 'Z')
		return negate_assumptions;
	
	// if isn't a simple And, just give up
	if(assume_operator !== 'and')
	    return undefined;

	var found_true=false, found_false=false;
	for(var i=0; i < assume_operands.length; i++) {
	    var result = is_integer_ast(tree, assume_operands[i]);
	    if(result === true)
		found_true = true;
	    else if(result === false)
		found_false = true;
	}

	if(found_true) {
	    if(found_false)
		return undefined;
	    else
		return !negate_assumptions;
	}
	else {
	    if(found_false)
		return negate_assumptions;
	    else
		return undefined;
	}
    }

    if(Array.isArray(tree)) {

	var operator = tree[0];
	var operands = tree.slice(1);

	if(operator === '-')
	    return is_integer_ast(operands[0], assumptions);
	
	if(operator === '*' || operator === '^') {

	    var all_integers =
		_.every(operands,
			function (v) {
			    return is_integer_ast(v, assumptions);
			});

	    if(all_integers)
		return true;
	    else {
		return undefined;

	    }
	}

	if(operator === '+') {

	    var n_non_integers=0;

	    for(var i=0; i < operands.length; i++) {
		var result = is_integer_ast(operands[i], assumptions);

		if(result === false) {
		    if(n_non_integers > 0)
			return undefined;
		    n_non_integers += 1;
		}
		if(result === undefined)
		    return undefined;
	    }

	    if(n_non_integers == 0)
		return true;
	    else // only one non-integer
		return false;
	}
	
	if(operator === '/' || operator === 'apply' || operator === 'prime')
	    return undefined;

	// other operators don't return numbers
	return false;
    }

    return false;
}


function is_real(expression, assumptions) {
    // returns true if
    //   - expression is a literal number
    //   - expression is a variable explicitly assumed to be real
    //   - assumptions include inequality involving variable
    //   - expression is a product, sum, power, or negation of the above
    //   - expression is a quotient of a real with a non-zero real
    // returns false if
    //   - expression is a variable explicitly assumed to be non-real
    //   - expression involves an operator that doesn't return a number
    // otherwise, return undefined
    //
    // expression is a math-expression
    //
    // if assumptions in undefined, get assumptions from expression context

    if(assumptions === undefined)
	assumptions = expression.context.assumptions;

    return is_real_ast(expression.tree, assumptions);

}


function is_real_ast(tree, assumptions) {
    // see description of is_real

    if(!Array.isArray(assumptions))
	return undefined;

    if(is_integer(tree, assumptions))
	return true;
    
    if(typeof tree === 'number')
	return true;

    if(typeof tree === 'string') {

	var assume_operator = assumptions[0];
	var assume_operands = assumptions.slice(1);

	var negate_assumptions = false;
	while(assume_operator === 'not') {
	    negate_assumptions = !negate_assumptions;
	    assumptions = assume_operands[0];
	    assume_operator = assumptions[0];
	    assume_operands = assumptions.slice(1);
	}
	
	if(assume_operator === 'in')
	    if(assume_operands[0]===tree && assume_operands[1] === 'R')
		return !negate_assumptions;
	if(assume_operator === 'ni')
	    if(assume_operands[1]===tree && assume_operands[0] === 'R')
		return !negate_assumptions;
	if(assume_operator === 'notin')
	    if(assume_operands[0]===tree && assume_operands[1] === 'R')
		return negate_assumptions;
	if(assume_operator === 'notni')
	    if(assume_operands[1]===tree && assume_operands[0] === 'R')
		return negate_assumptions;
	

	// if assumptions is an inequality involving variable
	// then return true
	if(assume_operator === '>' || assume_operator === '>='
	   || assume_operator === '<' || assume_operator === '<='
	   || assume_operator === 'lts' || assume_operator === 'gts') {
	    
	    var variables_in_inequality = variables_in_ast(assumptions);
	    var functions_in_inequality = functions_in_ast(assumptions);
	    if(variables_in_inequality.indexOf(tree) !== -1
	      && functions_in_inequality.length === 0)
		return !negate_assumptions;
	}
    
	// if isn't a simple And, just give up
	if(assume_operator !== 'and')
	    return undefined;
    
	var found_true=false, found_false=false;
	for(var i=0; i < assume_operands.length; i++) {
	    var result = is_real_ast(tree, assume_operands[i]);
	    if(result === true)
		found_true = true;
	    else if(result === false)
		found_false = true;
	}

	if(found_true) {
	    if(found_false)
		return undefined;
	    else
		return !negate_assumptions;
	}
	else {
	    if(found_false)
		return negate_assumptions;
	    else
		return undefined;
	}
    }

    if(Array.isArray(tree)) {

	var operator = tree[0];
	var operands = tree.slice(1);

	if(operator === '-')
	    return is_real_ast(operands[0], assumptions);
	
	if(operator === '*' || operator === '+' || operator === '^') {
	    
	    var all_integers =
		_.every(operands,
			function (v) {
			    return is_real_ast(v, assumptions);
			});
	    if(all_integers)
		return true;
	    else {
		return undefined;

	    }
	}
	    
	if(operator === '/') {
	    if(!(is_real_ast(operands[0], assumptions)
		 && is_real_ast(operands[1], assumption)))
		return undefined;

	    if(is_nonzero_ast(operands[1],assumptions))
		return true;
	    else
		return undefined;
	    
	}
	
	if(operator === 'apply' || operator === 'prime')
	    return undefined;

	// other operators don't return numbers
	return false;
    }

    return false;
}

function is_nonzero(expression, assumptions) {
    // returns true if
    //   - expression is a literal non-zero number
    //   - expression is a variable explicitly assumed to be nonzero
    //   - expression is a product, negation or quotient of the above
    //   - expression is a power of a nonzero to a real
    // returns false if
    //   - expression is literal zero
    //   - expression is a variable explicitly assumed to be zero
    //   - expression is a product involving a zero
    //   - expression involves an operator that doesn't return a number
    // otherwise, return undefined
    //
    // expression is a math-expression
    //
    // if assumptions in undefined, get assumptions from expression context

    if(assumptions === undefined)
	assumptions = expression.context.assumptions;

    return is_nonzero_ast(expression.tree, assumptions);

}


function is_nonzero_ast(tree, assumptions) {
    // see description of is_nonzero

    if(!Array.isArray(assumptions))
	return undefined;

    if(typeof tree === 'number')
	return tree != 0;

    if(typeof tree === 'string') {

	var assume_operator = assumptions[0];
	var assume_operands = assumptions.slice(1);

	if(assume_operator === '=')  {
	    if(assume_operands.indexOf(tree) != -1
	       && assume_operands.indexOf(0) != -1)
		return false;
	}
	if(assume_operator === 'ne') {
	    if(assume_operands[0]===tree && assume_operands[1] === 0)
		return true;
	    if(assume_operands[1]===tree && assume_operands[0] === 0)
		return true;
	}
    
	// if isn't a simple And, just give up
	if(assume_operator !== 'and')
	    return undefined;
    
	var found_true=false, found_false=false;
	for(var i=0; i < assume_operands.length; i++) {
	    var result = is_nonzero_ast(tree, assume_operands[i]);
	    if(result === true)
		found_true = true;
	    else if(result === false)
		found_false = true;
	}

	if(found_true) {
	    if(found_false)
		return undefined;
	    else
		return true;
	}
	else {
	    if(found_false)
		return false;
	    else
		return undefined;
	}
    }

    if(Array.isArray(tree)) {

	var operator = tree[0];
	var operands = tree.slice(1);

	if(operator === '-')
	    return is_nonzero_ast(operands[0], assumptions);
	
	if(operator === '*') {
	    var all_nonzero = true;
	    for(var i=0; i < operands.length; i++) {
		
		var result = is_nonzero_ast(operands[i], assumptions);
		
		if(result===false)
		    return false;
		if(!result)
		    all_nonzero=false;
	    }

	    if(all_nonzero)
		return true;
	    else
		return undefined;
	}
	    
	if(operator === '/') {

	    var result = is_nonzero_ast(operands[0], assumptions);

	    if(result !== true)
		return result;

	    if(is_nonzero_ast(operands[1], assumption))
		return true;
	    else
		return undefined;
	}
	
	if(operator === 'apply' || operator === 'prime')
	    return undefined;

	// other operators don't return numbers
	return false;
    }

    return false;
}


function is_nonnegative(expression, assumptions) {
    // returns true if
    //   - expression is a literal non-negative number
    //   - expression is a variable explicitly assumed to be non-negative
    //   - expression is a product or quotient of the above
    //   - expression is the negation of real that is zero or negative
    //   - expression is a power of a nonnegative to a nonzero real
    //   - expression is a power of a positive to a real
    // returns false if
    //   - expression is literal negative
    //   - expression is a variable explicitly assumed to be negative
    //   - expression is a negative of a positive number
    //   - expression is a product involving a zero
    //   - expression involves an operator that doesn't return a number
    // otherwise, return undefined
    //
    // expression is a math-expression
    //
    // if assumptions in undefined, get assumptions from expression context

    if(assumptions === undefined)
	assumptions = expression.context.assumptions;

    return is_nonzero_ast(expression.tree, assumptions);

}


function is_nonnegative_ast(tree, assumptions) {
    // see description of is_nonnegative

    if(!Array.isArray(assumptions))
	return undefined;

    if(typeof tree === 'number')
	return tree != 0;

    if(typeof tree === 'string') {

	var assume_operator = assumptions[0];
	var assume_operands = assumptions.slice(1);

	if(assume_operator === '=')  {
	    if(assume_operands.indexOf(tree) != -1
	       && assume_operands.indexOf(0) != -1)
		return false;
	}
	if(assume_operator === 'ne') {
	    if(assume_operands[0]===tree && assume_operands[1] === 0)
		return true;
	    if(assume_operands[1]===tree && assume_operands[0] === 0)
		return true;
	}
    
	// if isn't a simple And, just give up
	if(assume_operator !== 'and')
	    return undefined;
    
	var found_true=false, found_false=false;
	for(var i=0; i < assume_operands.length; i++) {
	    var result = is_nonzero_ast(tree, assume_operands[i]);
	    if(result === true)
		found_true = true;
	    else if(result === false)
		found_false = true;
	}

	if(found_true) {
	    if(found_false)
		return undefined;
	    else
		return true;
	}
	else {
	    if(found_false)
		return false;
	    else
		return undefined;
	}
    }

    if(Array.isArray(tree)) {

	var operator = tree[0];
	var operands = tree.slice(1);

	if(operator === '-')
	    return is_nonzero_ast(operands[0], assumptions);
	
	if(operator === '*') {
	    var all_nonzero = true;
	    for(var i=0; i < operands.length; i++) {
		
		var result = is_nonzero_ast(operands[i], assumptions);
		
		if(result===false)
		    return false;
		if(!result)
		    all_nonzero=false;
	    }

	    if(all_nonzero)
		return true;
	    else
		return undefined;
	}
	    
	if(operator === '/') {

	    var result = is_nonzero_ast(operands[0], assumptions);

	    if(result !== true)
		return result;

	    if(is_nonzero_ast(operands[1], assumption))
		return true;
	    else
		return undefined;
	}
	
	if(operator === 'apply' || operator === 'prime')
	    return undefined;

	// other operators don't return numbers
	return false;
    }

    return false;
}


function is_positive(expression, assumptions) {
    if(assumptions === undefined)
	assumptions = expression.context.assumptions;

    return is_positive_ast(expression.tree, assumptions);
}

function is_positive_ast(tree, assumptions) {

    var nonneg = is_nonnegative_ast(tree, assumptions);

    if(nonneg === true)
	return is_nonzero_ast(tree, assumptions);
    return nonneg;
}

function is_negative(expression, assumptions) {
    if(assumptions === undefined)
	assumptions = expression.context.assumptions;

    return is_negative_ast(expression.tree, assumptions);
}

function is_negative_ast(tree, assumptions) {

    var real = is_real_ast(tree, assumptions);

    if(real === true) {
	var nonneg = is_nonnegative_ast(tree, assumptions);
	if(nonneg === false)
	    return true;
	if(nonneg === true)
	    return false;
	return undefined;
    }
    
    return real;
}

function is_nonpositive(expression, assumptions) {
    if(assumptions === undefined)
	assumptions = expression.context.assumptions;

    return is_nonpositive_ast(expression.tree, assumptions);
}

function is_nonpositive_ast(tree, assumptions) {

    var real = is_real_ast(tree, assumptions);

    if(real === true) {
	var pos = is_positive_ast(tree, assumptions);
	if(pos === false)
	    return true;
	if(pos === true)
	    return false;
	return undefined;
    }
    
    return real;
}

exports.is_integer = is_integer;
exports.is_real = is_real;
exports.is_nonzero = is_nonzero
exports.is_nonnegative = is_nonnegative
exports.is_positive = is_positive
exports.is_nonpositive = is_nonpositive
exports.is_negative = is_negative

exports.is_integer_ast = is_integer_ast;
exports.is_real_ast = is_real_ast;
exports.is_nonzero_ast = is_nonzero_ast;
exports.is_nonnegative_ast = is_nonnegative_ast
exports.is_positive_ast = is_positive_ast
exports.is_nonpositive_ast = is_nonpositive_ast
exports.is_negative_ast = is_negative_ast
