var _ = require('underscore');
var ComplexNumber = require('./complex-number').ComplexNumber;
var parser = require('./parser');
var textToAst = parser.text.to.ast;
var astToLatex = require('./parser').ast.to.latex;
var astToFunction = require('./parser').ast.to.function;
var astToComplexFunction = require('./parser').ast.to.complexFunction;

/****************************************************************/
// replace variables in an AST by another AST
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
    
    var result = [operator].concat( _.map( operands, function(v,i) { return substitute_ast(v,bindings); } ) );
    return result;
};

function tree_match( haystack, needle ) {
    var match = {};

    if (typeof needle === 'string') {
	match[needle] = haystack;
	return match;
    }

    if (typeof haystack === 'number') {
	if (typeof needle === 'number') {
	    if (needle === haystack) {
		return {};
	    }
	}

	return null;
    }

    if (typeof haystack === 'string') {
	if (typeof needle === 'string') {
	    match[needle] = haystack;
	    return match;
	}

	return null;
    }

    var haystack_operator = haystack[0];
    var haystack_operands = haystack.slice(1);

    var needle_operator = needle[0];
    var needle_operands = needle.slice(1);

    if (haystack_operator === needle_operator) {
	if (haystack_operands.length >= needle_operands.length) {
	    var matches = {}

	    _.each( needle_operands, function(i) {
		var new_matches = tree_match( haystack_operands[i], needle_operands[i] );
		
		if (new_matches === null) {
		    matches = null;
		}

		if (matches != null) {
		    matches = $.extend( matches, new_matches );
		}
	    } );

	    if (matches != null) {
		matches = $.extend( matches, { remainder: haystack_operands.slice( needle_operands.length ) } );
	    }

	    return matches;
	}

	return null;
    }

    return null;
};

function subtree_matches(haystack, needle) {
    if (typeof haystack === 'number') {
	return (typeof needle === 'string');
    }    
    
    if (typeof haystack === 'string') {
	return (typeof needle === 'string');
    }    

    var match = tree_match( haystack, needle );
    if (match != null) {
	return true;
    }

    var operator = haystack[0];
    var operands = haystack.slice(1);

    var any_matches = false;

    $.each( operands, function(i) {
	if (subtree_matches(operands[i], needle))
	    any_matches = true;
    } );

    return any_matches;
};

function replace_subtree(haystack, needle, replacement) {
    if (typeof haystack === 'number') {
	return haystack;
    }    
    
    if (typeof haystack === 'string') {
	if (typeof needle === 'string')
	    if (needle === haystack)
		return replacement;
	
	return haystack;
    }    

    var match = tree_match( haystack, needle );
    if (match != null) {
	return substitute_ast( replacement, match ).concat( match.remainder );
    }

    var operator = haystack[0];
    var operands = haystack.slice(1);

    return [operator].concat( _.map( operands, function(v,i) { return replace_subtree(v, needle, replacement); } ) );
};

function associate_ast( tree, op ) {
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	return tree;
    }    

    var operator = tree[0];
    var operands = tree.slice(1);
    operands = _.map( operands, function(v,i) { 
	return associate_ast(v, op); } );

    if (operator == op) {
	var result = [];
	
	for( var i=0; i<operands.length; i++ ) {
	    if ((typeof operands[i] !== 'number') && (typeof operands[i] !== 'string') && (operands[i][0] === op)) {
		result = result.concat( operands[i].slice(1) );
	    } else {
		result.push( operands[i] );
	    }
	}

	operands = result;
    }

    return [operator].concat( operands );
}

function remove_identity( tree, op, identity ) {
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	return tree;
    }    

    var operator = tree[0];
    var operands = tree.slice(1);
    operands = _.map( operands, function(v,i) { return remove_identity(v, op, identity); } );

    if (operator == op) {
	operands = _.filter(operands, function (a) { return a != identity; });
	if (operands.length == 0)
	    operands = [identity];

	if (operands.length == 1)
	    return operands[0];
    }

    return [operator].concat( operands );
}

function remove_zeroes( tree ) {
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	return tree;
    }    

    var operator = tree[0];
    var operands = tree.slice(1);
    operands = _.map( operands, function(v,i) { return remove_zeroes(v); } );

    if (operator === "*") {
	for( var i=0; i<operands.length; i++ ) {
	    if (operands[i] === 0)
		return 0;
	}
    }

    return [operator].concat( operands );
}

function collapse_unary_minus( tree ) {
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	return tree;
    }    

    var operator = tree[0];
    var operands = tree.slice(1);
    operands = _.map( operands, function(v,i) { return collapse_unary_minus(v); } );

    if (operator == "~") {
	if (typeof operands[0] === 'number')
	    return -operands[0];
    }

    return [operator].concat( operands );
}

function clean_ast( tree ) {
    tree = associate_ast( tree, '+' );
    tree = associate_ast( tree, '-' );
    tree = associate_ast( tree, '*' );
    tree = remove_identity( tree, '*', 1 );
    tree = collapse_unary_minus( tree );
    tree = remove_zeroes( tree );
    tree = remove_identity( tree, '+', 0 );
    
    return tree;
};

/****************************************************************/
// complex number evaluation code for our AST's


function leaves( tree ) {
    if (typeof tree === 'number') {
	return [tree];
    }

    if (typeof tree === 'string') {
	return [tree];
    }    

    var operator = tree[0];
    var operands = tree.slice(1);

    return _.flatten( _.map( operands, function(v,i) { return leaves(v); } ) );
}

function variables_in_ast( tree ) {
    var result = leaves( tree );

    result = _.filter( result, function(v,i) {
	return (typeof v === 'string') && (v != "e") && (v != "pi")
    });

    result = result.filter(function(itm,i,a){
	return i==result.indexOf(itm);
    });
    
    return result;
}


/****************************************************************/
// convert an AST to a LaTeX expression



/****************************************************************/
// differentiate an AST

var derivatives = {
    "sin": textToAst('cos x'),
    "cos": textToAst('-(sin x)'),
    "tan": textToAst('(sec x)^2'),
    "cot": textToAst('-((csc x)^2)'),
    "sec": textToAst('(sec x)*(tan x)'),
    "csc": textToAst('-(csc x)*(cot x)'),
    "sqrt": textToAst('1/(2*sqrt(x))'),
    "log": textToAst('1/x'),
    "arcsin": textToAst('1/sqrt(1 - x^2)'),
    "arccos": textToAst('-1/sqrt(1 - x^2)'),
    "arctan": textToAst('1/(1 + x^2)'),
    "arccsc": textToAst('-1/(sqrt(-1/x^2 + 1)*x^2)'),
    "arcsec": textToAst('1/(sqrt(-1/x^2 + 1)*x^2)'),
    "arccot": textToAst('-1/(1 + x^2)'),
    "abs": textToAst('abs(x)/x'),
};

function derivative_of_ast(tree,x,story) {
    var ddx = '\\frac{d}{d' + x + '} ';

    // Derivative of a constant
    if (typeof tree === 'number') {
	story.push( 'The derivative of a constant is zero, that is, \\(' + ddx + astToLatex(tree) + ' = 0\\).' );
	return 0;
    }

    // Derivative of a more complicated constant 
    if ((variables_in_ast(tree)).indexOf(x) < 0) {
	story.push( 'The derivative of a constant is zero, that is, \\(' + ddx + astToLatex(tree) + ' = 0\\).' );
	return 0;
    }	

    // Derivative of a variable
    if (typeof tree === 'string') {
	if (x === tree) {
	    story.push( 'We know the derivative of the identity function is one, that is, \\(' + ddx + astToLatex(tree) + ' = 1\\).' );
	    return 1;
	}
	
	story.push( 'As far as \\(' + astToLatex(x) + '\\) is concerned, \\(' + astToLatex(tree) + '\\) is constant, so ' + ddx + astToLatex(tree) + ' = 0\\).' );
	return 0;
    }
    
    var operator = tree[0];
    var operands = tree.slice(1);

    // derivative of sum is sum of derivatives
    if ((operator === '+') || (operator === '-') || (operator === '~')) {
	story.push( 'Using the sum rule, \\(' + ddx + astToLatex( tree ) + ' = ' + (_.map( operands, function(v,i) { return ddx + astToLatex(v); } )).join( ' + ' ) + '\\).' );
	var result = [operator].concat( _.map( operands, function(v,i) { return derivative_of_ast(v,x,story); } ) );
	result = clean_ast(result);
	story.push( 'So using the sum rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	return result;
    }
    
    // product rule
    if (operator === '*') {
	var non_numeric_operands = [];
	var numeric_operands = [];

	for( var i=0; i<operands.length; i++ ) {
	    if ((typeof operands[i] === 'number') || ((variables_in_ast(operands[i])).indexOf(x) < 0)) {
		any_numbers = true;
		numeric_operands.push( operands[i] );
	    } else {
		non_numeric_operands.push( operands[i] );
	    } 
	}

	if (numeric_operands.length > 0) {
	    if (non_numeric_operands.length == 0) {
		story.push( 'Since the derivative of a constant is zero, \\(' + ddx + astToLatex( tree ) + ' = 0.\\)' );
		var result = 0;
		return result;
	    }

	    var remaining = ['*'].concat( non_numeric_operands );
	    if (non_numeric_operands.length == 1) 
		remaining = non_numeric_operands[0];



	    if (remaining === x) {
		story.push( 'By the constant multiple rule, \\(' + ddx + astToLatex( tree ) + ' = ' + (_.map( numeric_operands, function(v,i) { return astToLatex(v); } )).join( ' \\cdot ' ) + '\\).' );
		var result = ['*'].concat( numeric_operands );
		result = clean_ast(result);
		return result;
	    }

	    story.push( 'By the constant multiple rule, \\(' + ddx + astToLatex( tree ) + ' = ' + (_.map( numeric_operands, function(v,i) { return astToLatex(v); } )).join( ' \\cdot ' ) + ' \\cdot ' + ddx + '\\left(' + astToLatex(remaining) + '\\right)\\).' );

	    var d = derivative_of_ast(remaining,x,story);
	    var result = ['*'].concat( numeric_operands.concat( [d] ) );
	    result = clean_ast(result);
	    story.push( 'And so \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    return result;
	}

	story.push( 'Using the product rule, \\(' + ddx + astToLatex( tree ) + ' = ' +
		    (_.map( operands, function(v,i) {
			return (_.map( operands, function(w,j) {
			    if (i == j)
				return ddx + '\\left(' + astToLatex(v) + '\\right)';
			    else
				return astToLatex(w);
			})).join( ' \\cdot ' ) })).join( ' + ' ) + '\\).' );

	var inner_operands = operands.slice();

	var result = ['+'].concat( _.map( operands, function(v,i) {
	    return ['*'].concat( _.filter( _.map( inner_operands, function(w,j) {
		if (i == j) {
		    var d = derivative_of_ast(w,x,story);
		    // remove terms that have derivative 1
		    if (d === 1)
			return null;

		    return d;
		} else {
		    return w;
		}
	    } ), function(t) { return t != null; } ) );
	} ) );
	result = clean_ast(result);
	story.push( 'So using the product rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );

	return result;
    }
    
    // quotient rule
    if (operator === '/') {
	var f = operands[0];
	var g = operands[1];

	if ((variables_in_ast(g)).indexOf(x) < 0) {
	    story.push( 'By the constant multiple rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(['/', 1, g]) + ' \\cdot ' + ddx + '\\left(' + astToLatex(f) + '\\right)\\).' );

	    var df = derivative_of_ast(f,x,story);		
	    var quotient_rule = textToAst('(1/g)*d');
	    var result = substitute_ast( quotient_rule, { "d": df, "g": g } );
	    result = clean_ast(result);
	    story.push( 'So \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    
	    return result;		
	}

	if ((variables_in_ast(f)).indexOf(x) < 0) {
	    if (f !== 1) {
		story.push( 'By the constant multiple rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(f) + ' \\cdot ' + ddx + '\\left(' + astToLatex(['/',1,g]) + '\\right)\\).' );
	    }

	    story.push( 'Since \\(\\frac{d}{du} \\frac{1}{u}\\) is \\(\\frac{-1}{u^2}\\), the chain rule gives \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(f) + '\\cdot \\frac{-1}{ ' + astToLatex(g) + '^2' + '} \\cdot ' + ddx + astToLatex( g ) + "\\)." );

	    var a = derivative_of_ast(g,x,story);

	    var quotient_rule = textToAst('f * (-a/(g^2))');
	    var result = substitute_ast( quotient_rule, { "f": f, "a": a, "g": g } );
	    result = clean_ast(result);
	    story.push( 'So since \\(\\frac{d}{du} \\frac{1}{u}\\) is \\(\\frac{-1}{u^2}\\), the chain rule gives \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );

	    return result;
	}

	story.push( 'Using the quotient rule, \\(' + ddx + astToLatex( tree ) + ' = \\frac{' + ddx + '\\left(' + astToLatex(f) + '\\right) \\cdot ' + astToLatex(g) + ' - ' + astToLatex(f) + '\\cdot ' + ddx + '\\left(' + astToLatex(g) + '\\right)}{ \\left( ' + astToLatex(g) + ' \\right)^2} \\).' );

	var a = derivative_of_ast(f,x,story);
	var b = derivative_of_ast(g,x,story);
	var f_prime = a;
	var g_prime = b;

	var quotient_rule = textToAst('(a * g - f * b)/(g^2)');

	var result = substitute_ast( quotient_rule, { "a": a, "b": b, "f": f, "g": g } );
	result = clean_ast(result);
	story.push( 'So using the quotient rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );

	return result;
    }
    
    // power rule
    if (operator === '^') {
	var base = operands[0];
	var exponent = operands[1];
	
	if ((variables_in_ast(exponent)).indexOf(x) < 0) {
	    if ((typeof base === 'string') && (base === 'x')) {
		if (typeof exponent === 'number') {
		    var power_rule = textToAst('n * (f^m)');
		    var result = substitute_ast( power_rule, { "n": exponent, "m": exponent - 1, "f": base } );
		    result = clean_ast(result);
		    story.push( 'By the power rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex( exponent ) + ' \\cdot \\left(' + astToLatex( base ) + '\\right)^{' + astToLatex( ['-', exponent, 1] ) + '}\\).' );
		    return result;
		}

		var power_rule = textToAst('n * (f^(n-1))');
		var result = substitute_ast( power_rule, { "n": exponent, "f": base } );
		result = clean_ast(result);
		story.push( 'By the power rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex( exponent ) + ' \\cdot \\left(' + astToLatex( base ) + '\\right)^{' + astToLatex( ['-', exponent, 1] ) + '}\\).' );

		return result;
	    }

	    if (exponent != 1) {
		story.push( 'By the power rule and the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex( exponent ) + ' \\cdot \\left(' + astToLatex( base ) + '\\right)^{' + astToLatex( ['-', exponent, 1] ) + '} \\cdot ' + ddx + astToLatex( base ) + '\\).' );
	    }

	    var a = derivative_of_ast(base,x,story);

	    if (exponent === 1)
		return a;

	    if (typeof exponent === 'number') {
		var power_rule = textToAst('n * (f^m) * a');
		var result = substitute_ast( power_rule, { "n": exponent, "m": exponent - 1, "f": base, "a" : a } );
		result = clean_ast(result);
		story.push( 'So by the power rule and the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
		return result;
	    }

	    var power_rule = textToAst('n * (f^(n-1)) * a');
	    var result = substitute_ast( power_rule, { "n": exponent, "f": base, "a" : a } );
	    result = clean_ast(result);
	    story.push( 'So by the power rule and the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    return result;
	}
	
	if (base === 'e') {
	    if ((typeof exponent === 'string') && (exponent === x)) {
		var power_rule = textToAst('e^(f)');
		var result = substitute_ast( power_rule, { "f": exponent } );
		result = clean_ast(result);
		story.push( 'The derivative of \\(e^' + astToLatex( x ) + '\\) is itself, that is, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex( tree ) + '\\).' );

		return result;
	    }
	    
	    story.push( 'Using the rule for \\(e^x\\) and the chain rule, we know \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex( tree ) + ' \\cdot ' + ddx + astToLatex( exponent ) + '\\).' );

	    var power_rule = textToAst('e^(f)*d');

	    var d = derivative_of_ast(exponent,x,story);
	    var result = substitute_ast( power_rule, { "f": exponent, "d": d } );
	    result = clean_ast(result);
	    story.push( 'So using the rule for \\(e^x\\) and the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    return result;
	}
	
	if (typeof base === 'number') {
	    if ((typeof exponent === 'string') && (exponent === x)) {
		var power_rule = textToAst('a^(f) * log(a)');
		var result = substitute_ast( power_rule, { "a": base, "f": exponent } );
		result = clean_ast(result);
		story.push( 'The derivative of \\(a^' + astToLatex( x ) + '\\) is \\(a^{' + astToLatex( x ) + '} \\, \\log a\\), that is, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex( result ) + '\\).' );

		return result;
	    }

	    var exp_rule = textToAst('a^(f) * log(a)');
	    var partial_result = substitute_ast( exp_rule, { "a": base, "f": exponent } );

	    story.push( 'Using the rule for \\(a^x\\) and the chain rule, we know \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex( partial_result ) + ' \\cdot ' + ddx + astToLatex( exponent ) + '\\).' );

	    var power_rule = textToAst('a^(b)*log(a)*d');
	    var d = derivative_of_ast(exponent,x,story);
	    var result = substitute_ast( power_rule, { "a": base, "b": exponent, "d": d } );
	    result = clean_ast(result);
	    story.push( 'So using the rule for \\(a^x\\) and the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    return result;		
	}
	
	// general case of a function raised to a function
	var f = base;
	var g = exponent;

	story.push( "Recall the general rule for exponents, namely that \\(\\frac{d}{dx} u(x)^{v(x)} = u(x)^{v(x)} \\cdot \\left( v'(x) \\cdot \\log u(x) + \\frac{v(x) \\cdot u'(x)}{u(x)} \\right)\\).  In this case, \\(u(x) = " +  astToLatex( f ) + "\\) and \\(v(x) = " + astToLatex( g ) + "\\)." );

	var a = derivative_of_ast(f,x,story);
	var b = derivative_of_ast(g,x,story);

	var power_rule = textToAst('(f^g)*(b * log(f) + (g * a)/f)');
	var result = substitute_ast( power_rule, { "a": a, "b": b, "f": f, "g": g } );
	result = clean_ast(result);
	story.push( 'So by the general rule for exponents, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	return result;
    }

    if (operator === "apply") {
	var input = operands[1];

	story.push( 'By the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(substitute_ast( ["apply",operands[0] + "'","x"], { "x": input } )) + " \\cdot " + ddx + astToLatex(input)  + '\\).' );	    

	var result = ['*',
		      substitute_ast( ["apply",operands[0] + "'","x"], { "x": input } ),
		      derivative_of_ast( input, x, story )];
	result = clean_ast(result);		
	story.push( 'So by the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	return result;	    
    }

    // chain rule
    if (operator in derivatives) {
	var input = operands[0];

	if (typeof input == "number") {
	    var result = 0;
	    story.push( 'The derivative of a constant is zero so \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    return result;		
	} else if ((typeof input == "string") && (input == x)) {
	    var result = ['*',
			  substitute_ast( derivatives[operator], { "x": input } )];
	    result = clean_ast(result);
	    story.push( 'It is the case that \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    return result;
	} else if ((typeof input == "string") && (input != x)) {
	    var result = 0;
	    story.push( 'Since the derivative of a constant is zero, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    return result;
	} else {
	    story.push( 'Recall \\(\\frac{d}{du}' + astToLatex( [operator, 'u'] ) + ' = ' +
			astToLatex( derivative_of_ast( [operator, 'u'], 'u', [] ) ) + '\\).' );

	    story.push( 'By the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(substitute_ast( derivatives[operator], { "x": input } )) + " \\cdot " + ddx + astToLatex(input)  + '\\).' );	    

	    var result = ['*',
			  substitute_ast( derivatives[operator], { "x": input } ),
			  derivative_of_ast( input, x, story )];
	    result = clean_ast(result);		
	    story.push( 'So by the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    return result;
	}
    }
    
    return 0;
};

/****************************************************************/
//
// The "story" that the differentiation code produces can be somewhat repetitive
//
// Here we fix this
//

function lowercaseFirstLetter(string)
{
    return string.charAt(0).toLowerCase() + string.slice(1);
}

function simplify_story( story ) {
    // remove neighboring duplicates
    for (var i = story.length - 1; i >= 1; i--) {
	if (story[i] == story[i-1])
	    story.splice( i, 1 );
    }

    // Make it seem obvious that I know I am repeating myself
    for (var i = 0; i < story.length; i++ ) {
	for( var j = i + 1; j < story.length; j++ ) {
	    if (story[i] == story[j]) {
		story[j] = 'Again, ' + lowercaseFirstLetter( story[j] );
	    }
	}
    }

    return story;
};

function randomBindings(variables) {
    var result = {};
    
    _.each( variables, function(v) {
	result[v] = Math.random() * 20.0 - 10.0;
    });

    return result;
};

function randomComplexBindings(variables) {
    var result = {};
    
    _.each( variables, function(v) {
	result[v] = new ComplexNumber( Math.random() * 20.0 - 10.0,  Math.random() * 20.0 - 10.0 );
    });

    return result;
};

function randomComplexBindingsBall(variables,real,imag) {
    var result = {};
    
    _.each( variables, function(v) {
	result[v] = new ComplexNumber( real+Math.random()-.5, imag +Math.random()-.5);
    });

    return result;
};

function randomIntegerBindings(variables) {
    var result = {};
    _.each( variables, function(v) {
        result[v]=new ComplexNumber(Math.floor(Math.random()*30),0);
    });
    return result;
};


function StraightLineProgram(tree)
{
    this.syntax_tree = clean_ast(tree);
}

StraightLineProgram.prototype = {
    f: function(bindings) {
	return astToFunction( this.syntax_tree )( bindings );
    },
    
    evaluate: function(bindings) {
	return astToFunction( this.syntax_tree )( bindings );
    },

    complex_evaluate: function(bindings) {
	return astToComplexFunction( this.syntax_tree )( bindings );
    },

    substitute: function(bindings) {
	var ast_bindings = new Object();

	var alphabet = "abcdefghijklmnopqrstuvwxyz";
	for(var i=0; i<alphabet.length; i++) {
	    var c = alphabet.charAt(i);
	    if (c in bindings)
		ast_bindings[c] = bindings[c].syntax_tree;
	}

	return new StraightLineProgram( substitute_ast( this.syntax_tree, ast_bindings ) );
    },
    
    tex: function() {
	return astToLatex( this.syntax_tree );
    },

    toString: function() {
	return astToText( this.syntax_tree );
    },

    // numerically integrate via midpoint method with respect to 'x'
    integrate: function(x,a,b) {
	var intervals = 100;
	var total = 0.0;
	var bindings = new Object();

        for( var i=0; i < intervals; i++ ) {
	    var sample_point = a + ((b - a) * (i + 0.5) / intervals);
	    bindings[x] = sample_point;
	    total = total + this.evaluate( bindings );
	}

	return total * (b - a) / intervals;
    },

    // FIXME: This should be deleted
    equalsForBinding: function(other,bindings) {
	var epsilon = 0.01;
	var this_evaluated = this.evaluate(bindings);	
	var other_evaluated = other.evaluate(bindings);

	return (Math.abs(this_evaluated/other_evaluated - 1.0) < epsilon) ||
	    (this_evaluated == other_evaluated) ||
	    (isNaN(this_evaluated) && isNaN(other_evaluated));
    },
    
    derivative: function(x) {
	var story = [];
	return new StraightLineProgram(derivative_of_ast( this.syntax_tree, x, story ));
    },

    derivative_story: function(x) {
	var story = [];
	derivative_of_ast( this.syntax_tree, x, story );
	story = simplify_story( story );
	return story;
    },

    variables: function() {
	return variables_in_ast( this.syntax_tree );
    },
    
    equals: function(other) {
	var finite_tries = 0;
	var epsilon = 0.001; 
        var sum_of_differences = 0;
        var sum = 0;
	
	var variables = _.uniq( this.variables() + other.variables() ); 
	

 

 //begin integer case

       for (var i=0;i<variables.length;i++)
            { 
            if (variables[i]=='n') 
                {
                 for (var i=1;i<11;i++)
                     {
                     var bindings = randomIntegerBindings(variables); 
	             var this_evaluated = this.complex_evaluate(bindings); 	
	             var other_evaluated = other.complex_evaluate(bindings); 
	             if (isFinite(this_evaluated.real) && isFinite(other_evaluated.real) &&
		     isFinite(this_evaluated.imaginary) && isFinite(other_evaluated.imaginary)) 
                         {
		         finite_tries++;
                         sum_of_differences = sum_of_differences + this_evaluated.subtract(other_evaluated).modulus()
		         sum = sum + other_evaluated.modulus()                       
                  
                         } 
                     }
               if (finite_tries<1)
                   {return false}


	       if (sum_of_differences < epsilon*sum+(epsilon*epsilon))
               {return true;}
               else
               {return false;} 
                } 
             }

//end integer case      

//converts a variable name to a small offset, for use in the complex case above, via ascii code.

	function varToOffset(s){
		return (s.charCodeAt(0)-96)*0.01	
		}

//begin complex case 
        var points=[]

        for( var i=-10; i < 11; i=i+2)
            {
             for (var j=-10; j<11; j=j+2)
                 {
                  var bindings = {};   
                 _.each( variables, function(v) {
	         bindings[v] = new ComplexNumber(i + varToOffset(v),j+varToOffset(v));
    });
	          var this_evaluated = this.complex_evaluate(bindings); 	
	          var other_evaluated = other.complex_evaluate(bindings);
	          if (isFinite(this_evaluated.real) && isFinite(other_evaluated.real) &&
		  isFinite(this_evaluated.imaginary) && isFinite(other_evaluated.imaginary)) 
                       {
		       finite_tries++;
                       var difference=this_evaluated.subtract(other_evaluated).modulus();
                       sum_of_differences = sum_of_differences + difference ;
		       sum = sum + other_evaluated.modulus();
                       if (difference<.00001 && points.length<3)
                           {points.push([i,j]);}                       
                        } 
                 }
            
            }
           //console.log('first grid check');
           //console.log(bindings);
           //console.log(sum_of_differences)
           //console.log(points)
          if (finite_tries<1)
              {return false}
	  if (sum_of_differences < epsilon*sum+(epsilon*epsilon))
              {return true;}
          else
              {
               //console.log('bad branch case');
               for (i=0;i<points.length;i++)
                  {
                   var ballsum=0;
                   var sum=0;
                   for (j=0;j<20;j++)
                      {
                       var bindings= randomComplexBindingsBall(variables,points[i][0],points[i][1]);
                       var this_evaluated = this.complex_evaluate(bindings); 	
	               var other_evaluated = other.complex_evaluate(bindings);
                       sum=sum+this_evaluated.subtract(other_evaluated).modulus();
                      }
                   //console.log(sum);
                   if (sum<.0001)
                       {return true}
                   
                  }
              return false;
              }  

    },
};

var parse = function(string) {
    return new StraightLineProgram( textToAst(string) );
};

var parse_tex = function (string) {
    return new StraightLineProgram( latexToAst(string) );
};

exports.fromText = parse;
exports.parse = parse;
exports.fromLaTeX = parse_tex;
exports.parse_tex = parse_tex;
