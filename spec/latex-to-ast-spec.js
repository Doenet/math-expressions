var astToLatex = require('../lib/parser').ast.to.latex;
var latexToAst = require('../lib/parser').latex.to.ast;
var _ = require('underscore');
var ParseError = require('../lib/error').ParseError;
var Context = require('../lib/math-expressions');

describe("latex to ast", function() {
    var trees = {
	'\\frac{1}{2} x': ['*',['/',1,2],'x'],	
	'1+x+3': ['+',1,'x',3],
	'1-x-3': ['+',1,['-','x'],['-',3]],	
	"1 + - x": ['+',1,['-','x']],
	"1 - - x": ['+',1,['-',['-','x']]],
	'x^2': ['^', 'x', 2],
	'\\log x': ['apply', 'log', 'x'],
	'\\ln x': ['apply', 'ln', 'x'],
	'-x^2': ['-',['^', 'x', 2]],
	'|x|': ['apply', 'abs','x'],
	'|\\sin|x||': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
		'x^47': ['^', 'x', 47],
	'x^ab': ['*', ['^', 'x', 'a'], 'b'],
	'x^a!':  ['^', 'x', ['apply', 'factorial', 'a']],
	'xyz': ['*','x','y','z'],
	'c(a+b)': ['*', 'c', ['+', 'a', 'b']],
	'(a+b)c': ['*', ['+', 'a', 'b'], 'c'],
	'a!': ['apply', 'factorial','a'],
	'\\theta': 'theta',
	'theta': ['*', 't', 'h', 'e', 't', 'a'],
	'\\cos(\\theta)': ['apply', 'cos','theta'],
	'cos(x)': ['*', 'c', 'o', 's', 'x'],
	'|\\sin(|x|)|': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
	'\\blah(x)': ['*', 'blah', 'x'],
	'|x+3=2|': ['apply', 'abs', ['=', ['+', 'x', 3], 2]],
	'x_y_z': ['_', 'x', ['_','y','z']],
	'x_{y_z}': ['_', 'x', ['_','y','z']],
	'{x_y}_z': ['_', ['_', 'x', 'y'],'z'],
	'x^y^z': ['^', 'x', ['^','y','z']],
	'x^{y^z}': ['^', 'x', ['^','y','z']],
	'{x^y}^z': ['^', ['^', 'x', 'y'],'z'],
	'x^y_z': ['^', 'x', ['_','y','z']],
	'x_y^z': ['^', ['_','x','y'],'z'],
	'xyz!': ['*','x','y', ['apply', 'factorial', 'z']],
	'x': 'x',
	'f': 'f',
	'fg': ['*', 'f','g'],
	'f+g': ['+', 'f', 'g'],
	'f(x)': ['apply', 'f', 'x'],
	'f(x,y,z)': ['apply', 'f', ['tuple', 'x', 'y', 'z']],
	'fg(x)': ['*', 'f', ['apply', 'g', 'x']],
	'fp(x)': ['*', 'f', 'p', 'x'],
	'fx': ['*', 'f', 'x'],
	'f\'': ['prime', 'f'],
	'fg\'': ['*', 'f', ['prime', 'g']],
	'f\'g': ['*', ['prime', 'f'], 'g'],
	'f\'g\'\'': ['*', ['prime', 'f'], ['prime', ['prime', 'g']]],
	'x\'': ['prime', 'x'],
	'f\'(x)' : ['apply', ['prime', 'f'], 'x'],
	'f(x)\'' : ['prime', ['apply', 'f', 'x']],
	'\\sin(x)\'': ['prime', ['apply', 'sin', 'x']],
	'\\sin\'(x)': ['apply', ['prime', 'sin'], 'x'],
	'f\'\'(x)': ['apply', ['prime', ['prime', 'f']],'x'],
	'\\sin(x)\'\'': ['prime', ['prime', ['apply','sin','x']]],
	'f(x)^t_y': ['^', ['apply', 'f','x'], ['_','t','y']],
	'f_t(x)': ['apply', ['_', 'f', 't'], 'x'],
	'f(x)_t': ['_', ['apply', 'f', 'x'], 't'],
	'f^2(x)': ['apply', ['^', 'f', 2], 'x'],
	'f(x)^2': ['^', ['apply', 'f', 'x'],2],
	'f\'^a(x)': ['apply', ['^', ['prime', 'f'], 'a'], 'x'],
	'f^a\'(x)': ['apply', ['^', 'f', ['prime', 'a']], 'x'],
	'f_a^b\'(x)': ['apply', ['^', ['_', 'f', 'a'], ['prime', 'b']],'x'],
	'f_a\'^b(x)': ['apply', ['^', ['prime', ['_', 'f','a']],'b'],'x'],
	'\\sin x': ['apply', 'sin', 'x'],
	'f x': ['*', 'f', 'x'],
	'\\sin^xyz': ['*', ['apply', ['^', 'sin', 'x'], 'y'], 'z'],
	'\\sin xy': ['*', ['apply', 'sin', 'x'], 'y'],
	'\\sin^2(x)': ['apply', ['^', 'sin', 2], 'x'],
	'\\exp(x)': ['apply', 'exp', 'x'],
	'e^x': ['^', 'e', 'x'],
	'x^2!': ['^', 'x', ['apply', 'factorial', 2]],
	'x^2!!': ['^', 'x', ['apply', 'factorial', ['apply', 'factorial', 2]]],
	'x_t^2': ['^', ['_', 'x', 't'], 2],
	'x_f^2': ['_', 'x', ['^', 'f', 2]],
	'x_t\'': ['prime', ['_', 'x', 't']],
	'x_f\'': ['_', 'x', ['prime', 'f']],
	'(x,y,z)': ['tuple', 'x', 'y', 'z'],
	'(x,y)-[x,y]': ['+', ['tuple','x','y'], ['-', ['array','x','y']]],
	'2[z-(x+1)]': ['*', 2, ['+', 'z', ['-', ['+', 'x', 1]]]],
	'\\{1,2,x\\}': ['set', 1, 2, 'x'],
	'\\{x, x\\}': ['set', 'x', 'x'],
	'\\{x\\}': ['set', 'x'],
	'(1,2]': ['interval', ['tuple', 1, 2], ['tuple', false, true]],
	'[1,2)': ['interval', ['tuple', 1, 2], ['tuple', true, false]],
	'[1,2]': ['array', 1, 2 ],
	'(1,2)': ['tuple', 1, 2 ],
	'1,2,3': ['list', 1, 2, 3],
	'x=a': ['=', 'x', 'a'],
	'x=y=1': ['=', 'x', 'y', 1],
	'7 \\ne 2': ['ne', 7, 2],
	'7 \\neq 2': ['ne', 7, 2],
	'\\lnot x=y': ['not', ['=', 'x', 'y']],
	'\\lnot (x=y)': ['not', ['=', 'x', 'y']],
	'x>y': ['>', 'x','y'],
	'x \\gt y': ['>', 'x','y'],
	'x \\ge y': ['ge', 'x','y'],
	'x \\geq y': ['ge', 'x','y'],
	'x>y>z': ['gts', ['tuple', 'x', 'y','z'], ['tuple', true, true]],
	'x>y \\ge z': ['gts', ['tuple', 'x', 'y','z'], ['tuple', true, false]],
	'x \\ge y>z': ['gts', ['tuple', 'x', 'y','z'], ['tuple', false, true]],
	'x \\ge y \\ge z': ['gts', ['tuple', 'x', 'y','z'], ['tuple', false, false]],
	'x<y': ['<', 'x','y'],
	'x \\lt y': ['<', 'x','y'],
	'x \\le y': ['le', 'x','y'],
	'x \\leq y': ['le', 'x','y'],
	'x<y<z': ['lts', ['tuple', 'x', 'y','z'], ['tuple', true, true]],
	'x<y \\le z': ['lts', ['tuple', 'x', 'y','z'], ['tuple', true, false]],
	'x \\le y<z': ['lts', ['tuple', 'x', 'y', 'z'], ['tuple', false, true]],
	'x \\le y \\le z': ['lts', ['tuple', 'x', 'y', 'z'], ['tuple', false, false]],
	'x<y>z': ['>', ['<', 'x', 'y'], 'z'],
	'A \\subset B': ['subset', 'A', 'B'],
	'A \\not\\subset B': ['notsubset', 'A', 'B'],
	'A \\supset B': ['superset', 'A', 'B'],
	'A \\not\\supset B': ['notsuperset', 'A', 'B'],
	'x \\in A': ['in', 'x', 'A'],
	'x \\notin A': ['notin', 'x', 'A'],
	'x \\not\\in A': ['notin', 'x', 'A'],
	'A \\ni x': ['ni', 'A', 'x'],
	'A \\not\\ni x': ['notni', 'A', 'x'],
	'A \\cup B': ['union', 'A', 'B'],
	'A \\cap B': ['intersect', 'A', 'B'],
	'A \\land B': ['and', 'A', 'B'],
	'A \\wedge B': ['and', 'A', 'B'],
	'A \\lor B': ['or', 'A', 'B'],
	'A \\vee B': ['or', 'A', 'B'],
	'A \\land B \\lor C': ['and', 'A', 'B', 'C'],
	'A \\lor B \\lor C': ['or', 'A', 'B', 'C'],
	'A \\land B \\lor C': ['or', ['and', 'A', 'B'], 'C'],
	'A \\lor B \\land C': ['or', 'A', ['and', 'B', 'C']],
	'\\lnot x=1': ['not', ['=', 'x', 1]],
	'\\lnot(x=1)': ['not', ['=', 'x', 1]],
	'\\lnot(x=y) \\lor z \\ne w': ['or', ['not', ['=','x','y']], ['ne','z','w']],
	'1.2E3': 1200,
	'1.2E+3': 1200,
	'3.1E-3': 0.0031,
	'1.2e-3': ['+', ['*', 1.2, 'e'], ['-', 3]],
    };

    _.each( _.keys(trees), function(string) {
	it("parses " + string, function() {
	    expect(latexToAst(string)).toEqual(trees[string]);
	});	
    });    
    
    // inputs that should throw an error
    var bad_inputs = {
	'1++1': "Invalid location of '+'",
	')1++1': "Invalid location of ')'",
	'(1+1': "Expected )",
	'x-y-': "Unexpected end of input",
	'|x| |y|': "Invalid location of '|'",
	'_x': "Invalid location of _",
	'x_': "Unexpected end of input",
	'x@2': "Invalid symbol '@'",
	'|y/v': "Expected |",
	'x+^2': "Invalid location of ^",
	'x/\'y': "Invalid location of '",
	'[1,2,3)': "Expected ]",
	'(1,2,3]': "Expected )",
	'[x)': "Expected ]",
	'(x]': "Expected )",
	'\\sin': "Unexpected end of input",
	'\\sin+\\cos': "Invalid location of '+'",
    }

    _.each( _.keys(bad_inputs), function(string) {
	it("throws " + string, function() {
	    expect(function() {latexToAst(string)}).toThrowError(ParseError, bad_inputs[string]);
	});	
    });    
    
    // Inputs that are strings should render as exactly the same string
    // (other than white space changes) after one round trip to ast.
    // For inputs that are arrays, the first component should render to
    // be exactly as the second component (other than white space changes)
    // after one round trip to ast.
    var inputs = [
	'3+4',
	'3-4',
	'1+2+3',	
	['1/2', '\\frac{1}{2}'],
	'-2',
	'x+y-z+w',
	'-x-y+z-w',
	'x^{2}(x-3)',
	['x^2(x-3)-z^3e^{2x+1}+x/(x-1)','x^{2}(x-3)-z^{3}e^{2x+1}+\\frac{x}{x-1}'],
	['-1/x+((x-3)x)/((x-3)(x+4))','\\frac{-1}{x}+\\frac{(x-3)x}{(x-3)(x+4)}'],
	['(x/y)/(z/w)','\\frac{\\frac{x}{y}}{\\frac{z}{w}}'],
	'x!',
	'n!',
	'17!',
	'(x+1)!',
	'(x^{2}+1)!',
	'(n+1)!',
	'(n-1)!',
	['x_(n+1)!', 'x_{n+1}!'],
	'x^{2}',
	['\\sin x', '\\sin(x)'],
	'\\theta',	
	'\\theta^{2}',
	['\\sin 3', '\\sin(3)'],
	['\\cos x', '\\cos(x)'],
	['\\cos 3', '\\cos(3)'],
	['\\tan x', '\\tan(x)'],
	['\\tan 3', '\\tan(3)'],
	['\\sec x', '\\sec(x)'],
	['\\sec 3', '\\sec(3)'],
	['\\csc x', '\\csc(x)'],
	['\\csc 3', '\\csc(3)'],
	['\\arcsin x', '\\arcsin(x)'],
	['\\arcsin 3', '\\arcsin(3)'],
	['\\arccos x', '\\arccos(x)'],
	['\\arccos 3', '\\arccos(3)'],
	['\\arctan x', '\\arctan(x)'],
	['\\arctan 3', '\\arctan(3)'],
	['\\arccsc x', '\\arccsc(x)'],
	['\\arccsc 3', '\\arccsc(3)'],
	['\\arcsec x', '\\arcsec(x)'],
	['\\arcsec 3', '\\arcsec(3)'],
	['\\arccot x', '\\arccot(x)'],
	['\\arccot 3', '\\arccot(3)'],
	['\\asin x', '\\arcsin(x)'],
	['\\log x', '\\log(x)'],
	['\\log 3', '\\log(3)'],
	['\\ln x', '\\ln(x)'],
	['\\log e^{x}','\\log\\left(e^{x}\\right)'],
	'e^{x}',
	'\\exp(x)',
	['\\blah(x)', '\\blah x'],
	'\\sqrt{x}',
	'\\sqrt{4}',
	'\\frac{1}{\\sqrt{3}}',	
	'\\frac{1}{\\sqrt{-x}}',
	'\\sin\\left(3\\,x\\right)',
	'\\sin\\left (3\\,x\\right )',  // this really gets written...	
	'\\sin^{2}\\left(3\\,x\\right)',
	['\\sin^{2}x+\\cos^{2}x','\\sin^{2}\\left(x\\right)+\\cos^{2}\\left(x\\right)'],
	['\\frac{\\sin^{2}x}{\\cos^{2}x}','\\frac{\\sin^{2}\\left(x\\right)}{\\cos^{2}\\left(x\\right)}'],
	'\\sin^{3}\\left(x+y\\right)',
	'\\sin^{3}\\left  (x+y\\right  )',	
	'\\sqrt{x+y}',
	'\\sqrt{\\sqrt{x}}',
	'\\sqrt{\\frac{1}{x+y}}',
	'\\log(-x^{2})',
	'\\left|3\\right|',
	['\\sin\\left|x\\right|', '\\sin\\left(\\left|x\\right|\\right)'],
	['\\left|\\sin\\left|x\\right|\\right|','\\left|\\sin\\left(\\left|x\\right|\\right)\\right|'],
	['|\\sin||x|||','|\\sin(||x||)|'],
	'||x|+|y|+|z||',
	'|x+y < z|',
	'\\infty',
	"\\sin(x)'",
	"\\sin(x)''",
	'f(x)',
	"f'(x)",
	"f''(x)",
	"f(x)'",
	"\\sin'(x)",
	['\\sin x', '\\sin(x)'],
	['\\sin xy', '\\sin(x)y'],
	['\\sin^xyz', '\\sin^{x}(y)z'],
	['y(x)', 'yx'],
	["y'(x)", "y'x"],
	['x^22', 'x^{22}'],
	['x^ab', 'x^{a}b'],
	['x^y^z', 'x^{y^{z}}'],
	['(x^y)^z','(x^{y})^{z}'],
	['x_y_z', 'x_{y_{z}}'],
	['(x_y)_z','(x_{y})_{z}'],
	['x_y^z', 'x_{y}^{z}'],
	['x^y_z', 'x^{y_{z}}'],
	['f^2', 'f^{2}'],
	['f^2(x)', 'f^{2}(x)'],
	['f(x)^2', 'f(x)^{2}'],
	['f_t', 'f_{t}'],
	['f_t(x)', 'f_{t}(x)'],
	['f_t^2(x)', 'f_{t}^{2}(x)'],
	['f_t\'(x)', 'f_{t}\'(x)'],
	['f\'^2(x)', 'f\'^{2}(x)'],
	['f_t\'^2(x)', 'f_{t}\'^{2}(x)'],
	['f_(s+t)\'\'^2(x)', 'f_{s+t}\'\'^{2}(x)'],
	['f_{s+t}\'\'^2(x)', 'f_{s+t}\'\'^{2}(x)'],
	'\\sin(x)\'',
	['x_(s+t)\'\'','x_{s+t}\'\''],
	['(x-1-2)^2','(x-1-2)^{2}'],
	'(a,b)',
	'(a,b]',
	'[a,b)',
	'[a,b]',
	'\\{a,b\\}',
	'\\{a,b,c\\}',
	'\\{a\\}',
	'(a,b,c)',
	'[a,b,c]',
	'[a,b,c] + (a,b]',
	'a,b,c',
	'a,b',
	'x=y',
	'x=y=z',
	'x>y',
	'x \\ge y',
	'x>y>z',
	'x>y \\ge z',
	'x \\ge y>z',
	'x \\ge y \\ge z',
	'x<y',
	'x \\le y',
	'x<y<z',
	'x<y \\le z',
	'x \\le y<z',
	'x \\le y \\le z',
	'A \\cup B',
	'A \\cap B',
	'C = A \\cap B',
	['A=1 \\land B=2', '(A=1) \\land (B=2)'],
	'A \\lor B',
	'(A \\land B) \\lor C',
	'A \\land (B \\lor C)',
	'\\lnot(A \\land B)',
	'(A \\land B) < C',
	'(\\lnot A) = B',
	'(A \\land B) > (C \\land D) > (E \\land F)',
	'(A \\land B) + (C \\land D)',
	'(A \\land B) \\cup (C \\land D)',
	'(A \\land B) \\cap (C \\land D)',
	['x/y/z/w', '\\frac{\\frac{\\frac{x}{y}}{z}}{w}'],
	['x(x-1)/z', '\\frac{x(x-1)}{z}'],
	['A \\land B \\lor C', '(A \\land B) \\lor C'],
	['A \\lor B \\land C', 'A \\lor (B \\land C)'],
	['\\lnot A \\lor B', '(\\lnot A) \\lor B'],
	['A=1 \\lor B=x/y', '(A=1) \\lor (B=\\frac{x}{y})'],
	'x \\in (a,b)',
	['x \\not\\in (a,b)', 'x \\notin (a,b)'],
	'(a,b) \\ni x',
	'(a,b) \\not\\ni x',
	'(a,b) \\subset (c,d)',
	'(a,b) \\not\\subset (c,d)',
	'(a,b) \\supset (c,d)',
	'(a,b) \\not\\supset (c,d)',
    ];

    function clean(text) {
	return text
	    .replace(/\\left/g,'')
	    .replace(/\\right/g,'')
	    .replace(/\\,/g,'')
	    .replace(/ /g,'');
    }

    _.each( inputs, function(input) {
	it(input, function() {
	    if(Array.isArray(input))
		expect(clean(astToLatex(latexToAst(input[0])))).toEqual(clean(input[1]));
	    else
		expect(clean(astToLatex(latexToAst(input)))).toEqual(clean(input));
	    
	});	
    });

    // Additional round trips to ast should not alter the strings at all
    _.each( inputs, function(input) {
	it(input, function() {
	    if(Array.isArray(input))
		expect(astToLatex(latexToAst(astToLatex(latexToAst(input[0]))))).toEqual(astToLatex(latexToAst(input[0])));
	    else
		expect(astToLatex(latexToAst(astToLatex(latexToAst(input))))).toEqual(astToLatex(latexToAst(input)));
	});	
    });


    it("function symbol context", function () {
	Context.set_to_default();
	
	Context.parser_parameters.functionSymbols = [];
	expect(Context.fromLatex('f(x)+h(y)').tree).toEqual(
	    ['+',['*', 'f', 'x'], ['*', 'h', 'y']]);
	
	Context.parser_parameters.functionSymbols.push('f');
	expect(Context.fromLatex('f(x)+h(y)').tree).toEqual(
	    ['+',['apply', 'f', 'x'], ['*', 'h', 'y']]);
	
	Context.parser_parameters.functionSymbols.push('h');
	expect(Context.fromLatex('f(x)+h(y)').tree).toEqual(
	    ['+',['apply', 'f', 'x'], ['apply', 'h', 'y']]);
	
	Context.parser_parameters.functionSymbols.push('x');
	expect(Context.fromLatex('f(x)+h(y)').tree).toEqual(
	    ['+',['apply', 'f', 'x'], ['apply', 'h', 'y']]);

	Context.set_to_default();
    });
    
    it("applied function symbol context", function () {
	Context.set_to_default();
	
	Context.parser_parameters.appliedFunctionSymbols = [];
	expect(Context.fromLatex('\\sin(x) + \\custom(y)').tree).toEqual(
	    ['+', ['*', 'sin', 'x'], ['*', 'custom', 'y']]);
	expect(Context.fromLatex('\\sin x + \\custom y').tree).toEqual(
	    ['+', ['*', 'sin', 'x'], ['*', 'custom', 'y']]);

	Context.parser_parameters.appliedFunctionSymbols.push('custom');
	expect(Context.fromLatex('\\sin(x) + \\custom(y)').tree).toEqual(
	    ['+', ['*', 'sin', 'x'], ['apply', 'custom', 'y']]);
	expect(Context.fromLatex('\\sin x + \\custom y').tree).toEqual(
	    ['+', ['*', 'sin', 'x'], ['apply', 'custom', 'y']]);
	
	Context.parser_parameters.appliedFunctionSymbols.push('sin');
	expect(Context.fromLatex('\\sin(x) + \\custom(y)').tree).toEqual(
	    ['+', ['apply', 'sin', 'x'], ['apply', 'custom', 'y']]);
	expect(Context.fromLatex('\\sin x + \\custom y').tree).toEqual(
	    ['+', ['apply', 'sin', 'x'], ['apply', 'custom', 'y']]);

	Context.set_to_default();
	
    });


});
