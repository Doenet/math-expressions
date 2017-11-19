var _ = require('underscore');
var variables_in = require('../expression/variables').variables;
var functions_in = require('../expression/variables').functions;
var astToFunction = require('../ast-to-function.js').astToFunction;
var unflattenRight = require('../trees/flatten').unflattenRight;
var trees = require('../trees/basic');
var math = require('../mathjs');

// functions that map from one set to another
var functions = {
    C: {},
    R: {},
    nonzeroC: {},
    nonneg: {},
    pos: {},
};

functions.C.nonneg = ["abs"];
functions.C.nonzero = ["exp"];
functions.C.R = ["abs", "arg"];
functions.C.C = ["abs", "arg", "exp", "sign", "cos", "cosh", "sin", "sinh",
		 "erf", "sqrt", "log", "ln", "log10"];
functions.R.pos = ["exp"];
functions.R.nonneg = ["abs", "exp", "arg"];
functions.R.R = ["abs", "arg", "exp", "sign",  "cos", "cosh", "sin", "sinh",
		 "erf"];
functions.R.Z = ["sign"];
functions.nonzeroC.pos = ["abs"];
functions.nonneg.nonneg = ["abs", "exp", "arg", "sqrt", "erf"];
functions.nonzeroC.nonzero = ["abs"];
functions.nonneg.R = [... new Set(functions.R.R.concat(
    functions.nonneg.nonneg))];
functions.pos.pos = ["abs", "exp", "sqrt", "erf"];
functions.pos.nonneg = functions.pos.pos;
functions.pos.nonzero = ["abs", "exp", "sqrt", "erf"];
functions.pos.R = functions.nonneg.R.concat(["log", "ln", "log10"]);


function evaluate_to_constant(tree) {
    // evaluate to number by converting tree to number
    // and calling without arguments

    // return null if couldn't evaluate to constant (e.g., contains a variable)
    // otherwise returns constant

    var f = astToFunction(tree);

    var num=null;
    try {
	num = f();
    }
    catch (e) {};

    return num;
}


function negate_adjust(result, negate_assumptions) {
    if(result)
	return !negate_assumptions;
    if(result===false)
	return negate_assumptions;
    return undefined
}

function narrow_assumptions(assumptions, original_assumptions) {
    // find part of original assumptions after remove assumptions

    if(!Array.isArray(original_assumptions))
	return [];

    var operator = original_assumptions[0];
    var operands = original_assumptions.slice(1);
    if(operator != 'and')
	return [];

    var remaining_assumptions = [];

    for(var i=0; i<operands.length; i++) {
	if(!trees.equal(operands[i], assumptions))
	    remaining_assumptions.push(operands[i]);
    }
    if(remaining_assumptions.length == 0)
	return [];
    if(remaining_assumptions.legnth == 1)
	return remaining_assumptions[0];
    return [operator].concat(remaining_assumptions);
    
}


function simple_assumption_combination(assume_operator, left, right) {
    // For assume_operator='and' or assume_operator='or'
    // attempt to deduce wehther or not assumption was met
    // given results from left and right.
    // Results left and right are either true, false, or undefind (ambiguous)
    
    // Note: assumptions are restrictions on possible values.
    // If assume_operator is AND, then having a test passing
    // with either condition (restriction) is sufficient to pass test.
    // Not attemping logic where only a combination of restrictions
    // will lead to a pass.

    if(assume_operator === 'and') {
	// if one is undefined (i.e., ambiguous)
	// result is determined by other
	if(left === undefined)
	    return right;
	if(right === undefined)
	    return left;

	// meeting either restriction means both conditions are enough
	return left || right;
    }
    else { //assume_operator === 'or';

	// Since either condition could apply,
	// the test must pass for both

	// if either test is ambiguous, overall result is ambiguous
	if(left === undefined || right === undefined)
	    return undefined;

	// if tests disagree, result is ambiguous
	if(left != right)
	    return undefined;

	return left;
    }
}

	    

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

    var tree = expression.tree;
    
    var original_assumptions = assumptions.get_assumptions(
	variables_in(tree));
    
    return is_integer_ast(expression.tree, assumptions, original_assumptions);

}

function is_integer_ast(tree, assumptions, original_assumptions) {
    // see description of is_integer

    if(typeof assumptions !== 'object')
	assumptions = [];

    if(original_assumptions===undefined)
	original_assumptions = assumptions;

    if(typeof tree === 'number')
	return Number.isInteger(tree);

    // if can convert to constant, evaluate directly
    var c = evaluate_to_constant(tree);
    if(c !== null) {
	return Number.isInteger(c);
    }
    
    if(typeof tree === 'string') {

	var assume;
	if(Array.isArray(assumptions))
	    assume = assumptions;
	else
	    assume = assumptions.get_assumptions(tree);
	if(!Array.isArray(assume))
	    return undefined;

	var assume_with_negations = assume;
	
	var assume_operator = assume[0];
	var assume_operands = assume.slice(1);

	var negate_assumptions = false;
	while(assume_operator === 'not') {
	    negate_assumptions = !negate_assumptions;
	    assume = assume_operands[0];
	    if(!Array.isArray(assume))
		return undefined;
	    assume_operator = assume[0];
	    assume_operands = assume.slice(1);
	}
	
	if(assume_operator === 'in')
	    if(assume_operands[0]===tree && assume_operands[1] === 'Z')
		return negate_adjust(true, negate_assumptions);
	if(assume_operator === 'notin')
	    if(assume_operands[0]===tree && assume_operands[1] === 'Z')
		return negate_adjust(false, negate_assumptions);
	
	
	// haven't negated, then determining tree is not real or complex
	//  means it is not an integer
	if(negate_assumptions===false) {
	    if(assume_operator === 'notin') {
		if(assume_operands[0]===tree && assume_operands[1] === 'R')
		    return false;
		if(assume_operands[0]===tree && assume_operands[1] === 'C')
		    return false;
	    }
	}
	// if have negated, then determining tree is real or complex
	// means it is not an integer
	else {
	    if(assume_operator === 'in') {
		if(assume_operands[0]===tree && assume_operands[1] === 'R')
		    return false;
		if(assume_operands[0]===tree && assume_operands[1] === 'Z')
		    return false;
	    }
	}
	// assume equality has been expanded so that has two arguments
	if((assume_operator === '=' && !negate_assumptions) ||
	   (assume_operator === 'ne' && negate_assumptions)) {
	    // if assumption is "tree=something"
	    // check if something is integer
	    // (but without the assumption to avoid infinite loop)

	    var new_assumptions = narrow_assumptions(assume_with_negations,
						     original_assumptions);
	    if(assume_operands[0]===tree)
		return is_integer_ast(assume_operands[1], new_assumptions);
	    if(assume_operands[1]===tree)
		return is_integer_ast(assume_operands[0], new_assumptions);
	}
	    
	// if isn't a simple And or Or, just give up
	if(assume_operator !== 'and' && assume_operator !== 'or')
	    return undefined;

	// shouldn't have negated assumptions
	if(negate_assumptions)
	    return undefined;

	// if more than two operands, unflatten
	// (OK, since not attempting logic where only combined restrictions
	// lead to a passing test)
	if(assume_operands.length > 2) {
	    assume = unflattenRight(assume);
	    assume_operands = assume.slice(1);
	}

	var result_left = is_integer_ast(tree, assume_operands[0],
					 original_assumptions);
	var result_right = is_integer_ast(tree, assume_operands[1],
					  original_assumptions);
	return simple_assumption_combination(assume_operator, result_left,
					     result_right)
    }

    if(Array.isArray(tree)) {

	var assume;
	if(Array.isArray(assumptions))
	    assume = assumptions;
	else
	    assume = assumptions.get_assumptions(variables_in(tree));
	
	var operator = tree[0];
	var operands = tree.slice(1);

	if(operator === '-')
	    return is_integer_ast(operands[0], assume, original_assumptions);
	
	if(operator === '*') {
	    
	    var all_integers = operands.every(
		function (v) {
		    return is_integer_ast(v, assume, original_assumptions);
		});

	    if(all_integers)
		return true;
	    else {
		return undefined;

	    }
	}
    
	if(operator === '^') {

	    var base_nonzero = is_nonzero_ast(operands[0], assume,
					      original_assumptions);

	    if(!base_nonzero) {
		var pow_positive = is_positive_ast(operands[1], assume, true,
						   original_assumptions);

		if(pow_positive) {
		    if(base_nonzero === false)
			return true;  // 0^positive
		}
		else {
		    return undefined; // (possibly zero)^(possibly nonpositive)
		}
		
	    }
	    else { // nonzero base
		var pow_nonzero = is_nonzero_ast(operands[1], assume,
						 original_assumptions);
		if(pow_nonzero === false)
		    return true;   // nonzero^0

	    }

	    var base_integer = is_integer_ast(operands[0], assume,
					      original_assumptions);
	    var pow_integer =  is_integer_ast(operands[1], assume,
					      original_assumptions);

	    if(!base_integer)
		return base_integer;
	    
	    if(!pow_integer)
		return undefined;  // don't check for cases like 9^(1/2)
	    
	    var pow_nonneg= is_positive_ast(operands[1], assume, false,
					    original_assumptions);

	    if(pow_nonneg)
		return true;
	    else
		return undefined;
	    
	    
	}
	if(operator === '+') {

	    var n_non_integers=0;

	    for(var i=0; i < operands.length; i++) {
		var result = is_integer_ast(operands[i], assume,
					    original_assumptions);

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


	// check for functions that map certain sets to integers
	if(operator === 'apply') {
	    if(functions.C.Z && functions.C.Z.includes(operands[0]) &&
	       is_complex_ast(operands[1], assume, original_assumptions))
		return true;
	    if(functions.R.Z && functions.R.Z.includes(operands[0]) &&
	       is_real_ast(operands[1], assume, original_assumptions))
		return true;
	    if(functions.nonzeroC.Z && functions.nonzeroC.Z.includes(operands[0])
	       && is_nonzero_ast(operands[1], assume, original_assumptions)
	       && is_complex_ast(operands[1], assume, original_assumptions)
	      )
		return true;
	    if(functions.nonneg.Z && functions.nonneg.Z.includes(operands[0]) &&
	       is_positive_ast(operands[1], assume, false, original_assumptions))
		return true;
	    if(functions.pos.Z && functions.pos.Z.includes(operands[0]) &&
	       is_positive_ast(operands[1], assume, true, original_assumptions))
		return true;
	    return undefined;
	}
	
	if(operator === '/' || operator === 'prime')
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

    var tree = expression.tree;
    
    var original_assumptions = assumptions.get_assumptions(
	variables_in(tree));
    
    return is_real_ast(expression.tree, assumptions, original_assumptions);

}


function is_real_ast(tree, assumptions, original_assumptions) {
    // see description of is_real

    if(typeof assumptions !== 'object')
	assumptions = [];

    if(original_assumptions===undefined)
	original_assumptions = assumptions;

    if(typeof tree === 'number')
	return Number.isFinite(tree);
    
    // if can convert to constant, evaluate directly
    var c = evaluate_to_constant(tree);

    if(c !== null) {
	if(typeof c === 'number') {
	    return Number.isFinite(c);
	}
	return false;
    }
    
    if(typeof tree === 'string') {

	var assume;
	if(Array.isArray(assumptions))
	    assume = assumptions;
	else
	    assume = assumptions.get_assumptions(tree);

	if(!Array.isArray(assume))
	    return undefined;

	var assume_with_negations = assume;
	
	var assume_operator = assume[0];
	var assume_operands = assume.slice(1);

	var negate_assumptions = false;
	while(assume_operator === 'not') {
	    negate_assumptions = !negate_assumptions;
	    assume = assume_operands[0];
	    if(!Array.isArray(assume))
		return undefined;
	    assume_operator = assume[0];
	    assume_operands = assume.slice(1);
	}
	
	if(assume_operator === 'in')
	    if(assume_operands[0]===tree && assume_operands[1] === 'R')
		return negate_adjust(true, negate_assumptions);
	if(assume_operator === 'notin')
	    if(assume_operands[0]===tree && assume_operands[1] === 'R')
		return negate_adjust(false, negate_assumptions);
	// haven't negated, then determining tree is integer means it is real
	if(negate_assumptions===false) {
	    if(assume_operator === 'in')
		if(assume_operands[0]===tree && assume_operands[1] === 'Z')
		    return true;
	}
	// if have negated, then determining tree is not integer,
	// means it is an integer, hence real
	else {
	    if(assume_operator === 'notin')
		if(assume_operands[0]===tree && assume_operands[1] === 'Z')
		    return true;
	}

	// if assumptions is an inequality involving variable
	// then return true
	if(assume_operator === '<' || assume_operator === 'le') {
	    
	    var variables_in_inequality = variables_in(assume);
	    var functions_in_inequality = functions_in(assume);
	    
	    if(variables_in_inequality.indexOf(tree) !== -1
	      && functions_in_inequality.length === 0)
		return true;  // don't negate adjust
	}
	
	// assume equality has been expanded so that has two arguments
	if((assume_operator === '=' && !negate_assumptions) ||
	   (assume_operator === 'ne' && negate_assumptions)) {
	    // if assumption is "tree=something"
	    // check if something is real
	    // (but without the assumption to avoid infinite loop)

	    var new_assumptions = narrow_assumptions(assume_with_negations,
						     original_assumptions);
	    if(assume_operands[0]===tree)
		return is_real_ast(assume_operands[1], new_assumptions);
	    if(assume_operands[1]===tree)
		return is_real_ast(assume_operands[0], new_assumptions);
	}

	    
	// if isn't a simple And or Or, just give up
	if(assume_operator !== 'and' && assume_operator !== 'or')
	    return undefined;

	// shouldn't have negated assumptions
	if(negate_assumptions)
	    return undefined;

	// if more than two operands, unflatten
	// (OK, since not attempting logic where only combined restrictions
	// lead to a passing test)
	if(assume_operands.length > 2) {
	    assume = unflattenRight(assume);
	    assume_operands = assume.slice(1);
	}

	var result_left = is_real_ast(tree, assume_operands[0],
				      original_assumptions);
	var result_right = is_real_ast(tree, assume_operands[1],
				       original_assumptions);
	return simple_assumption_combination(assume_operator, result_left,
					     result_right)
    }

    if(Array.isArray(tree)) {

	var assume;
	if(Array.isArray(assumptions))
	    assume = assumptions;
	else
	    assume = assumptions.get_assumptions(variables_in(tree));
	

	var operator = tree[0];
	var operands = tree.slice(1);

	if(operator === '-')
	    return is_real_ast(operands[0], assume, original_assumptions);
	
	if(operator === '*' || operator === '+') {

	    // if more than two terms, unflatten
	    if(operands.length > 2) {
		tree = unflattenRight(tree);
		operands = tree.slice(1);
	    }

	    if(operator==='*') {
		// one confirmed zero factor makes product zero
		if((is_nonzero_ast(operands[0], assume, original_assumptions)
		    === false) ||
		   (is_nonzero_ast(operands[1], assume, original_assumptions)
		    === false))
		    return true;
	    }		
	    
	    var left_real = is_real_ast(operands[0], assume,
					original_assumptions);
	    var right_real = is_real_ast(operands[1], assume,
					 original_assumptions);

	    if(left_real && right_real)
		return true;
	    
	    // can confirm that is not real
	    // if one term/factor is real and the other is not real
	    if((left_real && right_real===false) ||
	       (right_real && left_real===false))
		return false

	    return undefined;
	    
	}

	if(operator === '^') {
	    
	    var base_nonzero = is_nonzero_ast(operands[0], assume,
					      original_assumptions);
	    var pow_positive = is_positive_ast(operands[1], assume, true,
					       original_assumptions);

	    if(!base_nonzero) {

		if(pow_positive) {
		    if(base_nonzero === false)
			return true;  // 0^positive
		}
		else {
		    return undefined; // (possibly zero)^(possibly nonpositive)
		}
		
	    }
	    else { // nonzero base
		var pow_nonzero = is_nonzero_ast(operands[1], assume,
						 original_assumptions);
		if(pow_nonzero === false)
		    return true;   // nonzero^0

	    }

	    var base_real = is_real_ast(operands[0], assume,
					original_assumptions);
	    var pow_real = is_real_ast(operands[1], assume,
				       original_assumptions);

	    if(!(base_real && pow_real))
		return undefined;

	    var base_nonnegative = is_positive_ast(operands[0], assume, false,
						   original_assumptions);

	    if(!base_nonnegative) {
		// if base might be negative
		// then power must be an integer
		// (already excluded 0^0)

		var pow_integer = is_integer_ast(operands[1], assume,
						 original_assumptions);
		if(pow_integer)
		    return true;
		else
		    return undefined
	    }

	    var base_positive = is_positive_ast(operands[0], assume, true,
						original_assumptions);

	    if(!base_positive) {
		// if base might be zero
		// then power must be positive
		if(pow_positive)
		    return true;
		else
		    return undefined;
	    }
	    
	    // base is positive, power is real
	    return true;
	    
	}
	
	if(operator === '/') {
	    // if can't be sure denominator is nonzero
	    if(!is_nonzero_ast(operands[1], assume, original_assumptions))
		return undefined;

	    // zero numerator
	    if(is_nonzero_ast(operands[0], assume, original_assumptions) === false)
		return true;

	    if(!(is_real_ast(operands[0], assume, original_assumptions)
		 && is_real_ast(operands[1], assume, original_assumptions)))
		return undefined;

	    return true;
	    
	}
	
	// check for functions that map certain sets to reals
	if(operator === 'apply') {
	    if(functions.C.R && functions.C.R.includes(operands[0]) &&
	       is_complex_ast(operands[1], assume, original_assumptions))
		return true;
	    if(functions.R.R && functions.R.R.includes(operands[0]) &&
	       is_real_ast(operands[1], assume, original_assumptions))
		return true;
	    if(functions.nonzeroC.R && functions.nonzeroC.R.includes(operands[0])
	       && is_nonzero_ast(operands[1], assume, original_assumptions)
	       && is_complex_ast(operands[1], assume, original_assumptions)
	      )
		return true;
	    if(functions.nonneg.R && functions.nonneg.R.includes(operands[0]) &&
	       is_positive_ast(operands[1], assume, false, original_assumptions))
		return true;
	    if(functions.pos.R && functions.pos.R.includes(operands[0]) &&
	       is_positive_ast(operands[1], assume, true, original_assumptions))
		return true;
	    return undefined;
	}
	
	if(operator === 'prime')
	    return undefined;

	// other operators don't return numbers
	return false;
    }

    return false;
}

function is_complex(expression, assumptions) {
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

    var tree = expression.tree;
    
    var original_assumptions = assumptions.get_assumptions(
	variables_in(tree));
    
    return is_complex_ast(expression.tree, assumptions, original_assumptions);

}


function is_complex_ast(tree, assumptions, original_assumptions) {
    // see description of is_complex

    if(typeof assumptions !== 'object')
	assumptions = [];

    if(original_assumptions===undefined)
	original_assumptions = assumptions;

    if(typeof tree === 'number')
	return Number.isFinite(tree);

    
    // if can convert to constant, evaluate directly
    var c = evaluate_to_constant(tree);
    if(c !== null) {
	if(typeof c === 'number') {
	    return Number.isFinite(c);
	}
	if(c.re !== undefined && Number.isFinite(c.re)
	   && c.im !== undefined && Number.isFinite(c.im) )
	    return true;
	
	return false;
    }
    
    if(typeof tree === 'string') {

	var assume;
	if(Array.isArray(assumptions))
	    assume = assumptions;
	else
	    assume = assumptions.get_assumptions(tree);

	if(!Array.isArray(assume))
	    return undefined;

	var assume_with_negations = assume;
	
	var assume_operator = assume[0];
	var assume_operands = assume.slice(1);

	var negate_assumptions = false;
	while(assume_operator === 'not') {
	    negate_assumptions = !negate_assumptions;
	    assume = assume_operands[0];
	    if(!Array.isArray(assume))
		return undefined;
	    assume_operator = assume[0];
	    assume_operands = assume.slice(1);
	}

	if(assume_operator === 'in')
	    if(assume_operands[0]===tree && assume_operands[1] === 'C')
		return negate_adjust(true, negate_assumptions);
	if(assume_operator === 'notin')
	    if(assume_operands[0]===tree && assume_operands[1] === 'C')
		return negate_adjust(false, negate_assumptions);
	// haven't negated, then determining tree is integer or real
	// means it is complex
	if(negate_assumptions===false) {
	    if(assume_operator === 'in') {
		if(assume_operands[0]===tree && assume_operands[1] === 'Z')
		    return true;
		if(assume_operands[0]===tree && assume_operands[1] === 'R')
		    return true;
	    }
	}
	// if have negated, then determining tree is not integer or not real,
	// means it is an integer/real, hence complex
	else {
	    if(assume_operator === 'notin') {
		if(assume_operands[0]===tree && assume_operands[1] === 'Z')
		    return true;
		if(assume_operands[0]===tree && assume_operands[1] === 'R')
		    return true;
	    }
	}

	// if assumptions is an inequality involving variable
	// then must be real, hence complex, so return true
	if(assume_operator === '<' || assume_operator === 'le') {
	    
	    var variables_in_inequality = variables_in(assume);
	    var functions_in_inequality = functions_in(assume);
	    
	    if(variables_in_inequality.indexOf(tree) !== -1
	      && functions_in_inequality.length === 0)
		return true;  // don't negate adjust
	}
	
	// assume equality has been expanded so that has two arguments
	if((assume_operator === '=' && !negate_assumptions) ||
	   (assume_operator === 'ne' && negate_assumptions)) {
	    // if assumption is "tree=something"
	    // check if something is complex
	    // (but without the assumption to avoid infinite loop)

	    var new_assumptions = narrow_assumptions(assume_with_negations,
						     original_assumptions);
	    if(assume_operands[0]===tree)
		return is_complex_ast(assume_operands[1], new_assumptions);
	    if(assume_operands[1]===tree)
		return is_complex_ast(assume_operands[0], new_assumptions);
	}
	
	// if isn't a simple And or Or, just give up
	if(assume_operator !== 'and' && assume_operator !== 'or')
	    return undefined;

	// shouldn't have negated assumptions
	if(negate_assumptions)
	    return undefined;

	// if more than two operands, unflatten
	// (OK, since not attempting logic where only combined restrictions
	// lead to a passing test)
	if(assume_operands.length > 2) {
	    assume = unflattenRight(assume);
	    assume_operands = assume.slice(1);
	}

	var result_left = is_complex_ast(tree, assume_operands[0],
					 original_assumptions);
	var result_right = is_complex_ast(tree, assume_operands[1],
					  original_assumptions);
	return simple_assumption_combination(assume_operator, result_left,
					     result_right)
    }

    if(Array.isArray(tree)) {

	var assume;
	if(Array.isArray(assumptions))
	    assume = assumptions;
	else
	    assume = assumptions.get_assumptions(variables_in(tree));
	

	var operator = tree[0];
	var operands = tree.slice(1);

	if(operator === '-')
	    return is_complex_ast(operands[0], assume, original_assumptions);
	
	if(operator === '*' || operator === '+') {

	    if(operator==='*') {
		// one confirmed zero factor makes product zero
		if(!operands.every(v => is_nonzero_ast(v, assume, original_assumptions) !== false))
		    return true;
	    }
	    
	    var all_complex = operands.every(v => is_complex_ast(v, assume, original_assumptions));
	    
	    if(all_complex)
		return true;
	    else {
		return undefined;

	    }
	}

	if(operator === '^') {
	    
	    var base_nonzero = is_nonzero_ast(operands[0], assume,
					      original_assumptions);
	    var pow_positive = is_positive_ast(operands[1], assume, true,
					       original_assumptions);

	    if(!base_nonzero) {

		if(pow_positive) {
		    if(base_nonzero === false)
			return true;  // 0^positive
		}
		else {
		    return undefined; // (possibly zero)^(possibly nonpositive)
		}
		
	    }
	    else { // nonzero base
		var pow_nonzero = is_nonzero_ast(operands[1], assume,
						 original_assumptions);
		if(pow_nonzero === false)
		    return true;   // nonzero^0

	    }

	    var base_complex = is_complex_ast(operands[0], assume,
					      original_assumptions);
	    var pow_complex = is_complex_ast(operands[1], assume,
					     original_assumptions);

	    if(base_complex && pow_complex)
		return true;
	    else
		return undefined;
	    
	}
	
	if(operator === '/') {
	    // if can't be sure denominator is nonzero
	    if(!is_nonzero_ast(operands[1], assume, original_assumptions))
		return undefined;

	    // zero numerator
	    if(is_nonzero_ast(operands[0], assume, original_assumptions) === false)
		return true;

	    if(!(is_complex_ast(operands[0], assume, original_assumptions)
		 && is_complex_ast(operands[1], assume, original_assumptions)))
		return undefined;

	    return true;
	    
	}

	// check for functions that map certain sets to complex numbers
	if(operator === 'apply') {
	    if(functions.C.C && functions.C.C.includes(operands[0]) &&
	       is_complex_ast(operands[1], assume, original_assumptions))
		return true;
	    if(functions.R.C && functions.R.C.includes(operands[0]) &&
	       is_real_ast(operands[1], assume, original_assumptions))
		return true;
	    if(functions.nonzeroC.C && functions.nonzeroC.C.includes(operands[0])
	       && is_nonzero_ast(operands[1], assume, original_assumptions)
	       && is_complex_ast(operands[1], assume, original_assumptions)
	      )
		return true;
	    if(functions.nonneg.C && functions.nonneg.C.includes(operands[0]) &&
	       is_positive_ast(operands[1], assume, false, original_assumptions))
		return true;
	    if(functions.pos.C && functions.pos.C.includes(operands[0]) &&
	       is_positive_ast(operands[1], assume, true, original_assumptions))
		return true;
	    return undefined;
	}
	
	if(operator === 'prime')
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

    var tree = expression.tree;
    
    var original_assumptions = assumptions.get_assumptions(
	variables_in(tree));
    
    return is_nonzero_ast(expression.tree, assumptions, original_assumptions);

}


function is_nonzero_ast(tree, assumptions, original_assumptions) {
    // see description of is_nonzero

    if(typeof assumptions !== 'object')
	assumptions = [];

    if(original_assumptions===undefined)
	original_assumptions = assumptions;

    if(typeof tree === 'number')
	return tree != 0;


    // if can convert to constant, evaluate directly
    var c = evaluate_to_constant(tree);
    if(c !== null) {
	if(typeof c === 'number') {
	    if(Number.isFinite(c))
		return c != 0;
	    if(Number.isNaN(c))
		return false;
	    return true;  // consider infinity to be nonzero
	}
	if(c.re !== undefined && c.im !== undefined &&
	   (c.re != 0 || c.im !=0) )
	    return true;
	return false;
    }
    
    if(typeof tree === 'string') {

	var assume;
	if(Array.isArray(assumptions))
	    assume = assumptions;
	else
	    assume = assumptions.get_assumptions(tree);

	if(!Array.isArray(assume))
	    return undefined;

	var assume_with_negations = assume;
	
	var assume_operator = assume[0];
	var assume_operands = assume.slice(1);

	var negate_assumptions = false;
	while(assume_operator === 'not') {
	    negate_assumptions = !negate_assumptions;
	    assume = assume_operands[0];
	    if(!Array.isArray(assume))
		return undefined;
	    assume_operator = assume[0];
	    assume_operands = assume.slice(1);
	}

	var new_assumptions = narrow_assumptions(assume_with_negations,
						 original_assumptions);
	
	// assume equality has been expanded so that has two arguments
	if((assume_operator === '=' && !negate_assumptions) ||
	   (assume_operator === 'ne' && negate_assumptions)) {
	    // if assumption is "tree=something"
	    // check if something is nonzero
	    // (but without the assumption to avoid infinite loop)

	    if(assume_operands[0]===tree)
		return is_nonzero_ast(assume_operands[1], new_assumptions);
	    if(assume_operands[1]===tree)
		return is_nonzero_ast(assume_operands[0], new_assumptions);
	}

	if((assume_operator === 'ne' && !negate_assumptions) ||
	   (assume_operator === '=' && negate_assumptions)) {
	    // if assumption is "tree!=something"
	    // check if something is zero

	    if(assume_operands[0]===tree) {
		if(is_nonzero_ast(assume_operands[1], new_assumptions)===false)
		    return true
	    }
	    if(assume_operands[1]===tree) {
		if(is_nonzero_ast(assume_operands[0], new_assumptions)===false)
		    return true;
	    }
	}
    
	// assume assumptions are ordered so greater than doesn't appear
	if(assume_operator === '<') {
	    if(!negate_assumptions) {
		if(assume_operands[0]===tree) {
		    if(is_negative_ast(
			assume_operands[1], new_assumptions, false))
			return true;
		}
		if(assume_operands[1]===tree) {
		    if(is_positive_ast(
			assume_operands[0], new_assumptions, false))
			return true;
		}
	    }
	    else {
		// negated, becomes ge
		if(assume_operands[0]===tree) {
		    if(is_positive_ast(
			assume_operands[1], new_assumptions, true))
			return true;
		}
		if(assume_operands[1]===tree) {
		    if(is_negative_ast(
			assume_operands[0], new_assumptions, true))
			return true;
		}
		
	    }
	}
	if(assume_operator === 'le') {
	    if(!negate_assumptions) {
		if(assume_operands[0]===tree) {
		    if(is_negative_ast(
			assume_operands[1], new_assumptions, true))
			return true;
		}
		if(assume_operands[1]===tree) {
		    if(is_positive_ast(
			assume_operands[0], new_assumptions, true))
			return true;
		}
	    }
	    else {
		// negated, so becomes >
		if(assume_operands[0]===tree) {
		    if(is_positive_ast(
			assume_operands[1], new_assumptions, false))
			return true;
		}
		if(assume_operands[1]===tree) {
		    if(is_negative_ast(
			assume_operands[0], new_assumptions, false))
			return true;
		}
		
	    }
	}
	
	// if isn't a simple And or Or, just give up
	if(assume_operator !== 'and' && assume_operator !== 'or')
	    return undefined;

	// shouldn't have negated assumptions
	if(negate_assumptions)
	    return undefined;

	// if more than two operands, unflatten
	// (OK, since not attempting logic where only combined restrictions
	// lead to a passing test)
	if(assume_operands.length > 2) {
	    assume = unflattenRight(assume);
	    assume_operands = assume.slice(1);
	}

	var result_left = is_nonzero_ast(tree, assume_operands[0],
					 original_assumptions);
	var result_right = is_nonzero_ast(tree, assume_operands[1],
					  original_assumptions);
	return simple_assumption_combination(assume_operator, result_left,
					     result_right)
    }

    if(Array.isArray(tree)) {

	var assume;
	if(Array.isArray(assumptions))
	    assume = assumptions;
	else
	    assume = assumptions.get_assumptions(variables_in(tree));
	
	var operator = tree[0];
	var operands = tree.slice(1);

	if(operator === '-')
	    return is_nonzero_ast(operands[0], assume, original_assumptions);
	
	if(operator === '+') {

	    // if more than two terms, unflatten
	    if(operands.length > 2) {
		tree = unflattenRight(tree);
		operands = tree.slice(1);
	    }

	    // if operands are opposite
	    // (trees.equal removes duplicate negatives through default_order)
	    // TODO: check if operands aren't infinite
	    if(trees.equal(operands[0], ['-', operands[1]]))
		return false;
	    
	    // can definitely determine nonzero if one term is zero
	    // or if both have the same sign

	    var nonzero_left = is_nonzero_ast(operands[0], assume,
					      original_assumptions);
	    var nonzero_right = is_nonzero_ast(operands[1], assume,
					       original_assumptions);

	    // if one is known to be zero, return result from other
	    if(nonzero_left===false)
		return nonzero_right;
	    if(nonzero_right===false)
		return nonzero_left;


	    // if one of both aren't real
	    // decide now
	    var real_left = is_real_ast(operands[0], assume,
					original_assumptions);
	    var real_right = is_real_ast(operands[1], assume,
					 original_assumptions);

	    if(!real_left || !real_right) {
		if(real_left===true) {
		    if(real_right === false)
			return true;
		    return undefined;
		}
		if(real_right===true) {
		    if(real_left===false)
			return true;
		    return undefined;
		}
		return undefined;
	    }
	    
	    // if reach here, both are real
	    
	    var nonneg_left = is_positive_ast(operands[0], assume, false,
					      original_assumptions);
	    var nonneg_right = is_positive_ast(operands[1], assume, false,
					       original_assumptions);

	    var positive_left = is_positive_ast(operands[0], assume, true,
						original_assumptions);
	    var positive_right = is_positive_ast(operands[1], assume, true,
						 original_assumptions);

	    // positive + nonnegative is nonzero
	    if( (nonneg_left && positive_right)
		|| (positive_left && nonneg_right))
		return true;
	    
	    // negative + nonpositive is nonzero
	    if( (nonneg_left===false && positive_right===false)
		|| (positive_left===false && nonneg_right===false))
		return true;
	    
	    
	    // have terms of both signs (or undefined sign)
	    // so can't determine if nonzero by this approach
	    return undefined;
	}
	
	if(operator === '*') {
	    var all_nonzero = true;
	    for(var i=0; i < operands.length; i++) {
		
		var result = is_nonzero_ast(operands[i], assume,
					    original_assumptions);
		
		if(result===false)
		    return false;  // found a zero factor
		if(result===undefined)
		    all_nonzero=false;
	    }

	    if(all_nonzero)
		return true;
	    else
		return undefined;
	}
	    
	if(operator === '/') {

	    var result = is_nonzero_ast(operands[0], assume,
					original_assumptions);

	    if(is_nonzero_ast(operands[1], assume, original_assumptions))
		return result;
	    else
		return undefined;
	}

	if(operator === '^') {
	    
	    var base_nonzero = is_nonzero_ast(operands[0], assume,
					      original_assumptions);
	    
	    if(!base_nonzero) {
		var pow_positive = is_positive_ast(operands[1], assume, true,
						   original_assumptions);
		
		if(pow_positive && (base_nonzero === false))
		    return false;  // 0^positive
		
		return undefined;
	    }
	    else { // nonzero base
		return true;

		// TODO? positive^(-infinity) =? 0
	    }
	    
	}
	
	// check for functions that map certain sets to nonzeros
	if(operator === 'apply') {
	    if(functions.C.nonzero && functions.C.nonzero.includes(operands[0]) &&
	       is_complex_ast(operands[1], assume, original_assumptions))
		return true;
	    if(functions.R.nonzero && functions.R.nonzero.includes(operands[0]) &&
	       is_real_ast(operands[1], assume, original_assumptions))
		return true;
	    if(functions.nonzeroC.nonzero && functions.nonzeroC.nonzero.includes(operands[0])
	       && is_nonzero_ast(operands[1], assume, original_assumptions)
	       && is_complex_ast(operands[1], assume, original_assumptions)
	      )
		return true;
	    if(functions.nonneg.nonzero && functions.nonneg.nonzero.includes(operands[0]) &&
	       is_positive_ast(operands[1], assume, false, original_assumptions))
		return true;
	    if(functions.pos.nonzero && functions.pos.nonzero.includes(operands[0]) &&
	       is_positive_ast(operands[1], assume, true, original_assumptions))
		return true;
	    return undefined;
	}

	if(operator === 'prime')
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
    //   - experssion is a product of an odd number of negative numbers
    //   - expression involves an operator that doesn't return a number
    // otherwise, return undefined
    //
    // expression is a math-expression
    //
    // if assumptions in undefined, get assumptions from expression context

    if(assumptions === undefined)
	assumptions = expression.context.assumptions;

    var tree = expression.tree;
    
    var original_assumptions = assumptions.get_assumptions(
	variables_in(tree));
    
    return is_positive_ast(expression.tree, assumptions, false,
			   original_assumptions);

}

function is_positive(expression, assumptions) {
    // returns true if
    //   - expression is a literal positive number
    //   - expression is a variable explicitly assumed to be positive
    //   - expression is a product or quotient of the above
    //   - expression is the negation of real that is negative
    //   - expression is a power of a positive to a nonzero real
    //   - expression is a power of a positive to a real
    // returns false if
    //   - expression is literal nonpositive
    //   - expression is a variable explicitly assumed to be nonpositive
    //   - expression is a negative of a nonnegative number
    //   - expression is a product involving a zero
    //   - experssion is a product of an odd number of nonpositive numbers
    //   - expression involves an operator that doesn't return a number
    // otherwise, return undefined
    //
    // expression is a math-expression
    //
    // if assumptions in undefined, get assumptions from expression context

    if(assumptions === undefined)
	assumptions = expression.context.assumptions;

    var tree = expression.tree;
    
    var original_assumptions = assumptions.get_assumptions(
	variables_in(tree));
    
    return is_positive_ast(expression.tree, assumptions, true,
			   original_assumptions);
}


function is_positive_ast(tree, assumptions, strict, original_assumptions) {
    // see description of is_nonnegative

    if(typeof assumptions != 'object')
	assumptions = [];

    if(original_assumptions===undefined)
	original_assumptions = assumptions;

    if(strict === undefined)
	strict = true;

    if(typeof tree === 'number') {
	if(strict)
	    return tree > 0;
	else {
	    return tree >= 0;
	}
    }

    var is_real = is_real_ast(tree, assumptions, original_assumptions);
    if(!is_real)
	return is_real;

    // if can convert to constant, evaluate directly
    var c = evaluate_to_constant(tree);

    if(c !== null) {
	if(typeof c === 'number') {
	    if(Number.isFinite(c))
		return (strict ? c > 0 : c >= 0);
	    return false;
	}
	return false;
    }

    if(typeof tree === 'string') {

	var assume;
	if(Array.isArray(assumptions))
	    assume = assumptions;
	else
	    assume = assumptions.get_assumptions(tree);

	if(!Array.isArray(assume))
	    return undefined;

	var assume_with_negations = assume;
	
	var assume_operator = assume[0];
	var assume_operands = assume.slice(1);

	var negate_assumptions = false;
	while(assume_operator === 'not') {
	    negate_assumptions = !negate_assumptions;
	    assume = assume_operands[0];
	    if(!Array.isArray(assume))
		return undefined;
	    assume_operator = assume[0];
	    assume_operands = assume.slice(1);
	}

	var new_assumptions = narrow_assumptions(assume_with_negations,
						 original_assumptions);
	
	// assume that equality has been expanded so that
	// have only two operands
	if((assume_operator === '=' && !negate_assumptions) ||
	   (assume_operator === 'ne' && negate_assumptions)) {
	    // if assumption is "tree=something"
	    // check if something is positive
	    // (but without the assumption to avoid infinite loop)

	    if(assume_operands[0]===tree)
		return is_positive_ast(assume_operands[1], new_assumptions,
				       strict);
	    if(assume_operands[1]===tree)
		return is_positive_ast(assume_operands[0], new_assumptions,
				       strict);
	}

	// assume assumptions are ordered so greater than doesn't appear
	if(assume_operator === '<') {
	    if(!negate_assumptions) {
		if(assume_operands[0]===tree) {
		    if(is_negative_ast(assume_operands[1], new_assumptions,
				       false))
			return false;
		}
		if(assume_operands[1]===tree) {
		    if(is_positive_ast(assume_operands[0], new_assumptions,
				       false))
			return true;
		}
	    }
	    else {
		// negated, so becomes ge
		if(assume_operands[0]===tree) {
		    if(is_positive_ast(assume_operands[1], new_assumptions,
				       strict))
			return true;
		}
		if(assume_operands[1]===tree) {
		    if(is_negative_ast(assume_operands[0], new_assumptions,
				       !strict))
			return false;
		}
		
	    }
	}
	if(assume_operator === 'le') {
	    if(!negate_assumptions) {
		if(assume_operands[0]===tree) {
		    if(is_negative_ast(assume_operands[1], new_assumptions,
				       !strict))
			return false;
		}
		if(assume_operands[1]===tree) {
		    if(is_positive_ast(assume_operands[0], new_assumptions,
				       strict))
			return true;
		}
	    }
	    else {
		// negated, so becomes >
		if(assume_operands[0]===tree) {
		    if(is_positive_ast(assume_operands[1], new_assumptions,
				       false))
			return true;
		}
		if(assume_operands[1]===tree) {
		    if(is_negative_ast(assume_operands[0], new_assumptions,
				       false))
			return false;
		}
		
	    }
	}

	
	// if isn't a simple And or Or, just give up
	if(assume_operator !== 'and' && assume_operator !== 'or')
	    return undefined;
	
	// shouldn't have negated assumptions
	if(negate_assumptions)
	    return undefined;

	// if more than two operands, unflatten
	// (OK, since not attempting logic where only combined restrictions
	// lead to a passing test)
	if(assume_operands.length > 2) {
	    assume = unflattenRight(assume);
	    assume_operands = assume.slice(1);
	}

	var result_left = is_positive_ast(tree, assume_operands[0], strict,
					  original_assumptions);
	var result_right = is_positive_ast(tree, assume_operands[1], strict,
					   original_assumptions);
	return simple_assumption_combination(assume_operator, result_left,
					     result_right)
    }
    
    if(Array.isArray(tree)) {
	
	var assume;
	if(Array.isArray(assumptions))
	    assume = assumptions;
	else
	    assume = assumptions.get_assumptions(variables_in(tree));
	
	var operator = tree[0];
	var operands = tree.slice(1);

	if(operator === '-')
	    return is_negative_ast(operands[0], assume, strict,
				   original_assumptions);
	
	if(operator === '+') {

	    // if more than two terms, unflatten
	    if(operands.length > 2) {
		tree = unflattenRight(tree);
		operands = tree.slice(1);
	    }

	    
	    var nonneg_left = is_positive_ast(operands[0], assume, false,
					      original_assumptions);
	    var nonneg_right = is_positive_ast(operands[1], assume, false,
					       original_assumptions);

	    var positive_left = is_positive_ast(operands[0], assume, true,
						original_assumptions);
	    var positive_right = is_positive_ast(operands[1], assume, true,
						 original_assumptions);


	    if(strict) {
		// positive + nonnegative is positive
		if( (nonneg_left && positive_right)
		    || (positive_left && nonneg_right))
		    return true;
	    }
	    else {
		// nonnegative + nonnegative is nonnegative
		if(nonneg_left && nonneg_right)
		    return true;
	    }

	    if(strict) {
		// nonpositive + nonpositive is nonpositive
		if(positive_left===false && positive_right===false)
		    return false;
	    }
	    else {
		// negative + nonpositive is negative
		if( (nonneg_left===false && positive_right===false)
		    || (positive_left===false && nonneg_right===false))
		    return false;
	    }
	    
	    // have terms of both signs (or undefined sign)
	    // so can't determine if positive or negative by this approach
	    return undefined;
	}
	
	if(operator === '*') {
	    // one confirmed zero factor makes product zero
	    if(!operands.every(v => is_nonzero_ast(v, assume, original_assumptions) !== false))
		return !strict;

	    // if more than two terms, unflatten
	    if(operands.length > 2) {
		tree = unflattenRight(tree);
		operands = tree.slice(1);
	    }

	    var real_left = is_real_ast(operands[0], assume,
					original_assumptions);
	    var real_right = is_real_ast(operands[1], assume,
					 original_assumptions);

	    // if can't determine if real, can't determine positivity
	    if(real_left===undefined || real_right===undefined)
		return undefined;

	    // if one nonreal, return false
	    // if two nonreals, return undefined
	    if(real_left===false) {
		if(real_right===false)
		    return undefined;
		else
		    return false;
	    }
	    else if(real_right===false)
		return false;

	    // if reach here, both factors are real
	    
	    var nonneg_left = is_positive_ast(operands[0], assume, false,
					      original_assumptions);
	    var nonneg_right = is_positive_ast(operands[1], assume, false,
					       original_assumptions);

	    var positive_left = is_positive_ast(operands[0], assume, true,
						original_assumptions);
	    var positive_right = is_positive_ast(operands[1], assume, true,
						 original_assumptions);

	    if(strict) {
		// product of two positives or two negatives is positive
		if((positive_left && positive_right)
		   || (nonneg_left===false && nonneg_right===false))
		    return true;
		
		// product of nonnegative and nonpositive is nonpositive
		if((positive_left==false && nonneg_right)
		   || (nonneg_left && positive_right==false))
		    return false;
	    }
	    else {
		// product of two nonnegatives or two nonpositives
		// is nonnegative
		if((nonneg_left && nonneg_right)
		   || (positive_left===false && positive_right===false))
		    return true;
		
		// product of positive and negative is negative
		if((positive_left && nonneg_right===false)
		   || (nonneg_left===false && positive_right))
		    return false;
	    }

	    // couldn't figure out sign via above algorithm
	    return undefined;
	    
	}
	    
	if(operator === '/') {

	    // if can't be sure denominator is nonzero
	    if(!is_nonzero_ast(operands[1], assume, original_assumptions))
		return undefined;

	    // zero numerator
	    if(is_nonzero_ast(operands[0], assume,
			      original_assumptions) === false)
		return !strict;

	    var denom_pos = is_positive_ast(operands[1], assume, true,
					    original_assumptions);
	    if(denom_pos === undefined)
		return undefined;

	    // if denominator is negative, sign is swapped
	    // so need opposite strictness for numerator
	    var numer_strict = denom_pos ? strict : !strict;
	    var numer_pos = is_positive_ast(operands[0], assume,
					    numer_strict,
					    original_assumptions);
	    
	    if(numer_pos === undefined)
		return undefined;
	    
	    if(numer_pos === true) {
		if(denom_pos == true)
		    return true;
		else
		    return false;
	    }
	    else {
		if(denom_pos === true)
		    return false;
		else
		    return true;
	    }
	}
	if(operator === '^') {
	    
	    var base_nonzero = is_nonzero_ast(operands[0], assume,
					      original_assumptions);

	    if(!base_nonzero) {
		var pow_positive = is_positive_ast(operands[1], assume, true,
						   original_assumptions);

		if(pow_positive) {
		    if(base_nonzero === false)
			return !strict;  // 0^positive
		    if(strict)
			return undefined // (possibly 0)^positive
		}
		else {
		    return undefined; // (possibly zero)^(possibly nonpositive)
		}
		
	    }
	    else { // nonzero base
		var pow_nonzero = is_nonzero_ast(operands[1], assume,
						 original_assumptions);
		if(pow_nonzero === false)
		    return true;   // nonzero^0

	    }

	    var base_real = is_real_ast(operands[0], assume,
					original_assumptions);

	    if(base_real !== true) {
		return undefined;
	    }
		
	    var base_positive = is_positive_ast(operands[0], assume, strict,
						original_assumptions);

	    if(!base_positive) {
		// if base could be negative
		// (already excluded zero base if strict)
		// then only way to be
		// positive (non_negative if not strict)
		// is if pow is an even integer

		// since haven't implemented is_even, only check
		// if have a constant that is an even integer
		// TODO: determine if x/2 is an integer
		var pow_over_two = ['/', operands[1], 2];
		if(is_integer_ast(pow_over_two, assume, original_assumptions))
		    return true;

		return undefined;
	    }

	    // base must be nonnegative
	    var pow_real = is_real_ast(operands[1], assume,
				       original_assumptions);

	    if(pow_real)
		return true;  // since already excluded 0^0
	    else
		return undefined;
	    
	}
	
	// check for functions that map certain sets to nonnegatives
	if(operator === 'apply'  && !strict) {
	    if(functions.C.nonneg && functions.C.nonneg.includes(operands[0]) &&
	       is_complex_ast(operands[1], assume, original_assumptions))
		return true;
	    if(functions.R.nonneg && functions.R.nonneg.includes(operands[0]) &&
	       is_real_ast(operands[1], assume, original_assumptions))
		return true;
	    if(functions.nonzeroC.nonneg && functions.nonzeroC.nonneg.includes(operands[0])
	       && is_nonzero_ast(operands[1], assume, original_assumptions)
	       && is_complex_ast(operands[1], assume, original_assumptions)
	      )
		return true;
	    if(functions.nonneg.nonneg && functions.nonneg.nonneg.includes(operands[0]) &&
	       is_positive_ast(operands[1], assume, false, original_assumptions))
		return true;
	    if(functions.pos.nonneg && functions.pos.nonneg.includes(operands[0]) &&
	       is_positive_ast(operands[1], assume, true, original_assumptions))
		return true;
	    return undefined;
	}
	// check for functions that map certain sets to integers
	if(operator === 'apply' && strict) {
	    if(functions.C.pos && functions.C.pos.includes(operands[0]) &&
	       is_complex_ast(operands[1], assume, original_assumptions))
		return true;
	    if(functions.R.pos && functions.R.pos.includes(operands[0]) &&
	       is_real_ast(operands[1], assume, original_assumptions))
		return true;
	    if(functions.nonzeroC.pos && functions.nonzeroC.pos.includes(operands[0])
	       && is_nonzero_ast(operands[1], assume, original_assumptions)
	       && is_complex_ast(operands[1], assume, original_assumptions)
	      )
		return true;
	    if(functions.nonneg.pos && functions.nonneg.pos.includes(operands[0]) &&
	       is_positive_ast(operands[1], assume, false, original_assumptions))
		return true;
	    if(functions.pos.pos && functions.pos.pos.includes(operands[0]) &&
	       is_positive_ast(operands[1], assume, true, original_assumptions))
		return true;
	    return undefined;
	}
	
	if(operator === 'prime')
	    return undefined;

	// other operators don't return numbers
	return false;
    }

    return false;
}
	

function is_negative(expression, assumptions) {
    if(assumptions === undefined)
	assumptions = expression.context.assumptions;

    var tree = expression.tree;
    
    var original_assumptions = assumptions.get_assumptions(
	variables_in(tree));
    
    return is_negative_ast(expression.tree, assumptions, true,
			   original_assumptions);
}

function is_negative_ast(tree, assumptions, strict, original_assumptions) {
    if(strict === undefined)
	strict = true;
    
    var real = is_real_ast(tree, assumptions, original_assumptions);

    if(real === true) {
	var nonneg = is_positive_ast(tree, assumptions, !strict,
				     original_assumptions);
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

    var tree = expression.tree;
    
    var original_assumptions = assumptions.get_assumptions(
	variables_in(tree));
    
    return is_negative_ast(expression.tree, assumptions, false,
			   original_assumptions);
}

exports.is_integer = is_integer;
exports.is_real = is_real;
exports.is_complex = is_complex;
exports.is_nonzero = is_nonzero
exports.is_nonnegative = is_nonnegative
exports.is_positive = is_positive
exports.is_nonpositive = is_nonpositive
exports.is_negative = is_negative

exports.is_integer_ast = is_integer_ast;
exports.is_real_ast = is_real_ast;
exports.is_complex_ast = is_complex_ast;
exports.is_nonzero_ast = is_nonzero_ast;
exports.is_positive_ast = is_positive_ast
exports.is_negative_ast = is_negative_ast
