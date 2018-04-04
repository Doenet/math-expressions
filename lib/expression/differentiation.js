import math from 'mathjs';
import { get_tree } from '../trees/util';
import { variables } from './variables';
import { substitute } from '../trees/basic';
import { simplify } from './simplify';
import textToAstObj from '../converters/text-to-ast';
import astToLatexObj from '../converters/ast-to-latex';

const textToAst = new textToAstObj();
const astToLatex = new astToLatexObj();

var derivatives = {
    "sin": textToAst.convert('cos x'),
    "cos": textToAst.convert('-(sin x)'),
    "tan": textToAst.convert('(sec x)^2'),
    "cot": textToAst.convert('-((csc x)^2)'),
    "sec": textToAst.convert('(sec x)*(tan x)'),
    "csc": textToAst.convert('-(csc x)*(cot x)'),
    "sqrt": textToAst.convert('1/(2*sqrt(x))'),
    "log": textToAst.convert('1/x'),
    "ln": textToAst.convert('1/x'),
    "exp": textToAst.convert('exp(x)'),
    "arcsin": textToAst.convert('1/sqrt(1 - x^2)'),
    "arccos": textToAst.convert('-1/sqrt(1 - x^2)'),
    "arctan": textToAst.convert('1/(1 + x^2)'),
    "arccsc": textToAst.convert('-1/(sqrt(-1/x^2 + 1)*x^2)'),
    "arcsec": textToAst.convert('1/(sqrt(-1/x^2 + 1)*x^2)'),
    "arccot": textToAst.convert('-1/(1 + x^2)'),
    "abs": textToAst.convert('abs(x)/x'),
};


function derivative(expr_or_tree,x,story) {
    var tree = get_tree(expr_or_tree);

    var ddx = '\\frac{d}{d' + x + '} ';

    // Derivative of a constant
    if (typeof tree === 'number') {
	story.push( 'The derivative of a constant is zero, that is, \\(' + ddx + astToLatex.convert(tree) + ' = 0\\).' );
	return 0;
    }

    // Derivative of a more complicated constant
    if ((variables(tree)).indexOf(x) < 0) {
	story.push( 'The derivative of a constant is zero, that is, \\(' + ddx + astToLatex.convert(tree) + ' = 0\\).' );
	return 0;
    }

    // Derivative of a variable
    if (typeof tree === 'string') {
	if (x === tree) {
	    story.push( 'We know the derivative of the identity function is one, that is, \\(' + ddx + astToLatex.convert(tree) + ' = 1\\).' );
	    return 1;
	}

	// should never get to this line
	// as would have been considered a constant
	story.push( 'As far as \\(' + astToLatex.convert(x) + '\\) is concerned, \\(' + astToLatex.convert(tree) + '\\) is constant, so ' + ddx + astToLatex.convert(tree) + ' = 0\\).' );
	return 0;
    }

    var operator = tree[0];
    var operands = tree.slice(1);

    // derivative of sum is sum of derivatives
    if ((operator === '+') || (operator === '-') || (operator === '~')) {
	story.push( 'Using the sum rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + (operands.map( function(v,i) { return ddx + astToLatex.convert(v); } )).join( ' + ' ) + '\\).' );
	var result = [operator].concat( operands.map( function(v,i) { return derivative(v,x,story); } ) );
	result = simplify(result);
	story.push( 'So using the sum rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(result) + '\\).' );
	return result;
    }

    // product rule
    if (operator === '*') {
	var non_numeric_operands = [];
	var numeric_operands = [];

	for( var i=0; i<operands.length; i++ ) {
	    if ((typeof operands[i] === 'number') || ((variables(operands[i])).indexOf(x) < 0)) {
		any_numbers = true;
		numeric_operands.push( operands[i] );
	    } else {
		non_numeric_operands.push( operands[i] );
	    }
	}

	if (numeric_operands.length > 0) {
	    if (non_numeric_operands.length == 0) {
		story.push( 'Since the derivative of a constant is zero, \\(' + ddx + astToLatex.convert( tree ) + ' = 0.\\)' );
		var result = 0;
		return result;
	    }

	    var remaining = ['*'].concat( non_numeric_operands );
	    if (non_numeric_operands.length == 1)
		remaining = non_numeric_operands[0];



	    if (remaining === x) {
		story.push( 'By the constant multiple rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + (numeric_operands.map( function(v,i) { return astToLatex.convert(v); } )).join( ' \\cdot ' ) + '\\).' );
		var result = ['*'].concat( numeric_operands );
		result = simplify(result);
		return result;
	    }

	    story.push( 'By the constant multiple rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + (numeric_operands.map( function(v,i) { return astToLatex.convert(v); } )).join( ' \\cdot ' ) + ' \\cdot ' + ddx + '\\left(' + astToLatex.convert(remaining) + '\\right)\\).' );

	    var d = derivative(remaining,x,story);
	    var result = ['*'].concat( numeric_operands.concat( [d] ) );
	    result = simplify(result);
	    story.push( 'And so \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(result) + '\\).' );
	    return result;
	}

	story.push( 'Using the product rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' +
		    (operands.map( function(v,i) {
			return (operands.map( function(w,j) {
			    if (i == j)
				return ddx + '\\left(' + astToLatex.convert(v) + '\\right)';
			    else
				return astToLatex.convert(w);
			})).join( ' \\cdot ' ) })).join( ' + ' ) + '\\).' );

	var inner_operands = operands.slice();

	var result = ['+'].concat( operands.map( function(v,i) {
	    return ['*'].concat( inner_operands.map( function(w,j) {
		if (i == j) {
		    var d = derivative(w,x,story);
		    // remove terms that have derivative 1
		    if (d === 1)
			return null;

		    return d;
		} else {
		    return w;
		}
	    } ).filter( function(t) { return t != null; } ) );
	} ) );
	result = simplify(result);
	story.push( 'So using the product rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(result) + '\\).' );

	return result;
    }

    // quotient rule
    if (operator === '/') {
	var f = operands[0];
	var g = operands[1];

	if ((variables(g)).indexOf(x) < 0) {
	    story.push( 'By the constant multiple rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(['/', 1, g]) + ' \\cdot ' + ddx + '\\left(' + astToLatex.convert(f) + '\\right)\\).' );

	    var df = derivative(f,x,story);
	    var quotient_rule = textToAst.convert('(1/g)*d');
	    var result = substitute( quotient_rule, { "d": df, "g": g } );
	    result = simplify(result);
	    story.push( 'So \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(result) + '\\).' );

	    return result;
	}

	if ((variables(f)).indexOf(x) < 0) {
	    if (f !== 1) {
		story.push( 'By the constant multiple rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(f) + ' \\cdot ' + ddx + '\\left(' + astToLatex.convert(['/',1,g]) + '\\right)\\).' );
	    }

	    story.push( 'Since \\(\\frac{d}{du} \\frac{1}{u}\\) is \\(\\frac{-1}{u^2}\\), the chain rule gives \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(f) + '\\cdot \\frac{-1}{ ' + astToLatex.convert(g) + '^2' + '} \\cdot ' + ddx + astToLatex.convert( g ) + "\\)." );

	    var a = derivative(g,x,story);

	    var quotient_rule = textToAst.convert('f * (-a/(g^2))');
	    var result = substitute( quotient_rule, { "f": f, "a": a, "g": g } );
	    result = simplify(result);
	    story.push( 'So since \\(\\frac{d}{du} \\frac{1}{u}\\) is \\(\\frac{-1}{u^2}\\), the chain rule gives \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(result) + '\\).' );

	    return result;
	}

	story.push( 'Using the quotient rule, \\(' + ddx + astToLatex.convert( tree ) + ' = \\frac{' + ddx + '\\left(' + astToLatex.convert(f) + '\\right) \\cdot ' + astToLatex.convert(g) + ' - ' + astToLatex.convert(f) + '\\cdot ' + ddx + '\\left(' + astToLatex.convert(g) + '\\right)}{ \\left( ' + astToLatex.convert(g) + ' \\right)^2} \\).' );

	var a = derivative(f,x,story);
	var b = derivative(g,x,story);
	var f_prime = a;
	var g_prime = b;

	var quotient_rule = textToAst.convert('(a * g - f * b)/(g^2)');

	var result = substitute( quotient_rule, { "a": a, "b": b, "f": f, "g": g } );
	result = simplify(result);
	story.push( 'So using the quotient rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(result) + '\\).' );

	return result;
    }

    // power rule
    if (operator === '^') {
	var base = operands[0];
	var exponent = operands[1];

	if ((variables(exponent)).indexOf(x) < 0) {
	    if ((typeof base === 'string') && (base === 'x')) {
		if (typeof exponent === 'number') {
		    var power_rule = textToAst.convert('n * (f^m)');
		    var result = substitute( power_rule, { "n": exponent, "m": exponent - 1, "f": base } );
		    result = simplify(result);
		    story.push( 'By the power rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert( exponent ) + ' \\cdot \\left(' + astToLatex.convert( base ) + '\\right)^{' + astToLatex.convert( ['-', exponent, 1] ) + '}\\).' );
		    return result;
		}

		var power_rule = textToAst.convert('n * (f^(n-1))');
		var result = substitute( power_rule, { "n": exponent, "f": base } );
		result = simplify(result);
		story.push( 'By the power rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert( exponent ) + ' \\cdot \\left(' + astToLatex.convert( base ) + '\\right)^{' + astToLatex.convert( ['-', exponent, 1] ) + '}\\).' );

		return result;
	    }

	    if (exponent != 1) {
		story.push( 'By the power rule and the chain rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert( exponent ) + ' \\cdot \\left(' + astToLatex.convert( base ) + '\\right)^{' + astToLatex.convert( ['-', exponent, 1] ) + '} \\cdot ' + ddx + astToLatex.convert( base ) + '\\).' );
	    }

	    var a = derivative(base,x,story);

	    if (exponent === 1)
		return a;

	    if (typeof exponent === 'number') {
		var power_rule = textToAst.convert('n * (f^m) * a');
		var result = substitute( power_rule, { "n": exponent, "m": exponent - 1, "f": base, "a" : a } );
		result = simplify(result);
		story.push( 'So by the power rule and the chain rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(result) + '\\).' );
		return result;
	    }

	    var power_rule = textToAst.convert('n * (f^(n-1)) * a');
	    var result = substitute( power_rule, { "n": exponent, "f": base, "a" : a } );
	    result = simplify(result);
	    story.push( 'So by the power rule and the chain rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(result) + '\\).' );
	    return result;
	}

	if (base === 'e' && math.define_e) {
	    if ((typeof exponent === 'string') && (exponent === x)) {
		var power_rule = textToAst.convert('e^(f)');
		var result = substitute( power_rule, { "f": exponent } );
		result = simplify(result);
		story.push( 'The derivative of \\(e^' + astToLatex.convert( x ) + '\\) is itself, that is, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert( tree ) + '\\).' );

		return result;
	    }

	    story.push( 'Using the rule for \\(e^x\\) and the chain rule, we know \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert( tree ) + ' \\cdot ' + ddx + astToLatex.convert( exponent ) + '\\).' );

	    var power_rule = textToAst.convert('e^(f)*d');

	    var d = derivative(exponent,x,story);
	    var result = substitute( power_rule, { "f": exponent, "d": d } );
	    result = simplify(result);
	    story.push( 'So using the rule for \\(e^x\\) and the chain rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(result) + '\\).' );
	    return result;
	}

	if (typeof base === 'number') {
	    if ((typeof exponent === 'string') && (exponent === x)) {
		var power_rule = textToAst.convert('a^(f) * log(a)');
		var result = substitute( power_rule, { "a": base, "f": exponent } );
		result = simplify(result);
		story.push( 'The derivative of \\(a^' + astToLatex.convert( x ) + '\\) is \\(a^{' + astToLatex.convert( x ) + '} \\, \\log a\\), that is, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert( result ) + '\\).' );

		return result;
	    }

	    var exp_rule = textToAst.convert('a^(f) * log(a)');
	    var partial_result = substitute( exp_rule, { "a": base, "f": exponent } );

	    story.push( 'Using the rule for \\(a^x\\) and the chain rule, we know \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert( partial_result ) + ' \\cdot ' + ddx + astToLatex.convert( exponent ) + '\\).' );

	    var power_rule = textToAst.convert('a^(b)*log(a)*d');
	    var d = derivative(exponent,x,story);
	    var result = substitute( power_rule, { "a": base, "b": exponent, "d": d } );
	    result = simplify(result);
	    story.push( 'So using the rule for \\(a^x\\) and the chain rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(result) + '\\).' );
	    return result;
	}

	// general case of a function raised to a function
	var f = base;
	var g = exponent;

	story.push( "Recall the general rule for exponents, namely that \\(\\frac{d}{dx} u(x)^{v(x)} = u(x)^{v(x)} \\cdot \\left( v'(x) \\cdot \\log u(x) + \\frac{v(x) \\cdot u'(x)}{u(x)} \\right)\\).  In this case, \\(u(x) = " +  astToLatex.convert( f ) + "\\) and \\(v(x) = " + astToLatex.convert( g ) + "\\)." );

	var a = derivative(f,x,story);
	var b = derivative(g,x,story);

	var power_rule = textToAst.convert('(f^g)*(b * log(f) + (g * a)/f)');
	var result = substitute( power_rule, { "a": a, "b": b, "f": f, "g": g } );
	result = simplify(result);
	story.push( 'So by the general rule for exponents, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(result) + '\\).' );
	return result;
    }

    if (operator === "apply" && !(operands[0] in derivatives)) {
	// derivative of function whose derivative is not given

	var input = operands[1];

	story.push( 'By the chain rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(substitute( ["apply",operands[0] + "'","x"], { "x": input } )) + " \\cdot " + ddx + astToLatex.convert(input)  + '\\).' );

	var result = ['*',
		      substitute( ["apply",operands[0] + "'","x"], { "x": input } ),
		      derivative( input, x, story )];
	result = simplify(result);
	story.push( 'So by the chain rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(result) + '\\).' );
	return result;
    }

    // chain rule
    if ((operator === "apply" && operands[0] in derivatives) ||
	operator in derivatives) {

	var used_apply = false;
	if(operator === "apply") {
	    operator = operands[0];
	    operands = operands.slice(1);
	    used_apply = true;
	}

	var input = operands[0];

	if (typeof input == "number") {
	    var result = 0;
	    story.push( 'The derivative of a constant is zero so \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(result) + '\\).' );
	    return result;
	} else if ((typeof input == "string") && (input == x)) {
	    var result = ['*',
			  substitute( derivatives[operator], { "x": input } )];
	    result = simplify(result);
	    story.push( 'It is the case that \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(result) + '\\).' );
	    return result;
	} else if ((typeof input == "string") && (input != x)) {
	    var result = 0;
	    story.push( 'Since the derivative of a constant is zero, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(result) + '\\).' );
	    return result;
	} else {
	    var example_ast = [operator,'u'];
	    if(used_apply)
		example_ast = ["apply"].concat(example_ast);
	    story.push( 'Recall \\(\\frac{d}{du}' + astToLatex.convert( example_ast ) + ' = ' +
			astToLatex.convert( derivative( example_ast, 'u', [] ) ) + '\\).' );

	    story.push( 'By the chain rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(substitute( derivatives[operator], { "x": input } )) + " \\cdot " + ddx + astToLatex.convert(input)  + '\\).' );

	    var result = ['*',
			  substitute( derivatives[operator], { "x": input } ),
			  derivative( input, x, story )];
	    result = simplify(result);
	    story.push( 'So by the chain rule, \\(' + ddx + astToLatex.convert( tree ) + ' = ' + astToLatex.convert(result) + '\\).' );
	    return result;
	}
    }

    return 0;
}

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
}

export function derivative(expr, x) {
    var story = [];
    return derivative( expr, x, story );
};

export function derivative_story(expr, x) {
    var story = [];
    derivative( expr, x, story );
    story = simplify_story( story );
    return story;
};

export var derivativeStory = derivative_story;
