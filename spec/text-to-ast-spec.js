var astToText = require('../lib/parser').ast.to.text;
var textToAst = require('../lib/parser').text.to.ast;
var _ = require('underscore');
var ParseError = require('../lib/error').ParseError;
var Context = require('../lib/math-expressions');

describe("text to ast", function() {
    var trees = {
	'1+x+3': ['+',1,'x',3],
	"1 + - x": ['+',1,['-','x']],
	"1 - x": ['+',1,['-','x']],
	"1 - - x": ['+',1,['-',['-','x']]],
	"1 + x/2": ['+',1,['/','x',2]],
	'1-x-3': ['+',1,['-','x'],['-',3]],	
	'x^2': ['^', 'x', 2],
	'-x^2': ['-',['^', 'x', 2]],
	'-3^2': ['-',['^', 3, 2]],
	'x^47': ['^', 'x', 47],
	'x^ab': ['*', ['^', 'x', 'a'], 'b'],
	'x^a!':  ['^', 'x', ['apply', 'factorial', 'a']],
	'x*y*z': ['*','x','y','z'],
	'xyz': ['*','x','y','z'],
	'in': ['*', 'i', 'n'],
	'ni': ['*', 'n', 'i'],
	'x*y*z*w': ['*','x','y','z','w'],
	'xyzw': ['*','x', 'y', 'z', 'w'],
	'(x*y)*(z*w)': ['*','x','y','z','w'],
	'c*(a+b)': ['*', 'c', ['+', 'a', 'b']],
	'(a+b)*c': ['*', ['+', 'a', 'b'], 'c'],
	'|x|': ['apply', 'abs','x'],
	'a!': ['apply', 'factorial','a'],
	'theta': 'theta',
	'cos(theta)': ['apply', 'cos','theta'],
	'x!': ['apply', 'factorial','x'],
	'|sin(|x|)|': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
	'sin(θ)': ['apply', 'sin', 'theta'],
	'|x+3=2|': ['apply', 'abs', ['=', ['+', 'x', 3], 2]],
	'x_y_z': ['_', 'x', ['_','y','z']],
	'x_(y_z)': ['_', 'x', ['_','y','z']],
	'(x_y)_z': ['_', ['_', 'x', 'y'],'z'],
	'x^y^z': ['^', 'x', ['^','y','z']],
	'x^(y^z)': ['^', 'x', ['^','y','z']],
	'(x^y)^z': ['^', ['^', 'x', 'y'],'z'],
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
	'sin(x)\'': ['prime', ['apply', 'sin', 'x']],
	'sin\'(x)': ['apply', ['prime', 'sin'], 'x'],
	'f\'\'(x)': ['apply', ['prime', ['prime', 'f']],'x'],
	'sin(x)\'\'': ['prime', ['prime', ['apply','sin','x']]],
	'f(x)^t_y': ['^', ['apply', 'f','x'], ['_','t','y']],
	'f_t(x)': ['apply', ['_', 'f', 't'], 'x'],
	'f(x)_t': ['_', ['apply', 'f', 'x'], 't'],
	'f^2(x)': ['apply', ['^', 'f', 2], 'x'],
	'f(x)^2': ['^', ['apply', 'f', 'x'],2],
	'f\'^a(x)': ['apply', ['^', ['prime', 'f'], 'a'], 'x'],
	'f^a\'(x)': ['apply', ['^', 'f', ['prime', 'a']], 'x'],
	'f_a^b\'(x)': ['apply', ['^', ['_', 'f', 'a'], ['prime', 'b']],'x'],
	'f_a\'^b(x)': ['apply', ['^', ['prime', ['_', 'f','a']],'b'],'x'],
	'sin x': ['apply', 'sin', 'x'],
	'f x': ['*', 'f', 'x'],
	'sin^xyz': ['*', ['apply', ['^', 'sin', 'x'], 'y'], 'z'],
	'sin xy': ['*', ['apply', 'sin', 'x'], 'y'],
	'sin^2(x)': ['apply', ['^', 'sin', 2], 'x'],
	'x^2!': ['^', 'x', ['apply', 'factorial', 2]],
	'x^2!!': ['^', 'x', ['apply', 'factorial', ['apply', 'factorial', 2]]],
	'x_t^2': ['^', ['_', 'x', 't'], 2],
	'x_f^2': ['_', 'x', ['^', 'f', 2]],
	'x_t\'': ['prime', ['_', 'x', 't']],
	'x_f\'': ['_', 'x', ['prime', 'f']],
	'(x,y,z)': ['tuple', 'x', 'y', 'z'],
	'(x,y)-[x,y]': ['+', ['tuple','x','y'], ['-', ['array','x','y']]],
	'2[z-(x+1)]': ['*', 2, ['+', 'z', ['-', ['+', 'x', 1]]]],
	'{1,2,x}': ['set', 1, 2, 'x'],
	'(1,2]': ['interval', ['tuple', 1, 2], ['tuple', false, true]],
	'[1,2)': ['interval', ['tuple', 1, 2], ['tuple', true, false]],
	'[1,2]': ['array', 1, 2 ],
	'(1,2)': ['tuple', 1, 2 ],
	'x=a': ['=', 'x', 'a'],
	'x=y=1': ['=', 'x', 'y', 1],
	'7 != 2': ['ne', 7, 2],
	'7 ≠ 2': ['ne', 7, 2],
	'not x=y': ['not', ['=', 'x', 'y']],
	'!x=y': ['not', ['=', 'x', 'y']],
	'!(x=y)': ['not', ['=', 'x', 'y']],
	'x>y': ['>', 'x','y'],
	'x>=y': ['ge', 'x','y'],
	'x≥y': ['ge', 'x','y'],
	'x>y>z': ['gts', ['tuple', 'x', 'y','z'], ['tuple', true, true]],
	'x>y>=z': ['gts', ['tuple', 'x', 'y','z'], ['tuple', true, false]],
	'x>=y>z': ['gts', ['tuple', 'x', 'y','z'], ['tuple', false, true]],
	'x>=y>=z': ['gts', ['tuple', 'x', 'y','z'], ['tuple', false, false]],
	'x<y': ['<', 'x','y'],
	'x<=y': ['le', 'x','y'],
	'x≤y': ['le', 'x','y'],
	'x<y<z': ['lts', ['tuple', 'x', 'y','z'], ['tuple', true, true]],
	'x<y<=z': ['lts', ['tuple', 'x', 'y','z'], ['tuple', true, false]],
	'x<=y<z': ['lts', ['tuple', 'x', 'y', 'z'], ['tuple', false, true]],
	'x<=y<=z': ['lts', ['tuple', 'x', 'y', 'z'], ['tuple', false, false]],
	'x<y>z': ['>', ['<', 'x', 'y'], 'z'],
	'A subset B': ['subset', 'A', 'B'],
	'A ⊂ B': ['subset', 'A', 'B'],
	'A notsubset B': ['notsubset', 'A', 'B'],
	'A ⊄ B': ['notsubset', 'A', 'B'],
	'A superset B': ['superset', 'A', 'B'],
	'A ⊃ B': ['superset', 'A', 'B'],
	'A notsuperset B': ['notsuperset', 'A', 'B'],
	'A ⊅ B': ['notsuperset', 'A', 'B'],
	'x elementof A': ['in', 'x', 'A'],
	'x ∈ A': ['in', 'x', 'A'],
	'x notelementof A': ['notin', 'x', 'A'],
	'x ∉ A': ['notin', 'x', 'A'],
	'A containselement x': ['ni', 'A', 'x'],
	'A ∋ x': ['ni', 'A', 'x'],
	'A notcontainselement x': ['notni', 'A', 'x'],
	'A ∌ x': ['notni', 'A', 'x'],
	'A union B': ['union', 'A', 'B'],
	'A ∪ B': ['union', 'A', 'B'],
	'A intersect B': ['intersect', 'A', 'B'],
	'A ∩ B': ['intersect', 'A', 'B'],
	'A and B': ['and', 'A', 'B'],
	'A & B': ['and', 'A', 'B'],
	'A && B': ['and', 'A', 'B'],
	'A ∧ B': ['and', 'A', 'B'],
	'A or B': ['or', 'A', 'B'],
	'A ∨ B': ['or', 'A', 'B'],
	'A ∧ B ∧ C': ['and', 'A', 'B', 'C'],
	'A ∨ B ∨ C': ['or', 'A', 'B', 'C'],
	'A and B or C': ['or', ['and', 'A', 'B'], 'C'],
	'A or B and C': ['or', 'A', ['and', 'B', 'C']],
	'!x=1': ['not', ['=', 'x', 1]],
	'!(x=1)': ['not', ['=', 'x', 1]],
	'!(x=y) or z != w': ['or', ['not', ['=','x','y']], ['ne','z','w']],
	'1.2E3': 1200,
	'1.2E+3': 1200,
	'3.1E-3': 0.0031,
	'1.2e-3': ['+', ['*', 1.2, 'e'], ['-', 3]],
    };

    _.each( _.keys(trees), function(string) {
	it("parses " + string, function() {
	    expect(textToAst(string)).toEqual(trees[string]);
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
	'x,y': "Invalid location of ','",
	'sin': "Unexpected end of input",
	'sin+cos': "Invalid location of '+'",
	'\\cos(x)': "Invalid symbol '\\'",
    }

    _.each( _.keys(bad_inputs), function(string) {
	it("throws " + string, function() {
	    expect(function() {textToAst(string)}).toThrowError(ParseError, bad_inputs[string]);
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
	'1/2',
	'-2',
	'x+y-z+w',
	'-x-y+z-w',
	'x^2(x-3)',
	'x^2(x-3)-z^3e^(2x+1)+x/(x-1)',
	'-1/x+((x-3)x)/((x-3)(x+4))',
	'(x/y)/(z/w)',
	'x!',
	'n!',
	'17!',
	'(x+1)!',
	'(x^2+1)!',
	'(n+1)!',
	'(n-1)!',
	'x_(n+1)!',
	'x^2',
	'sin(x)',
	'sin(3)',
	'cos(x)',
	'cos(3)',
	'tan(x)',
	'tan(3)',
	'sec(x)',
	'sec(3)',
	['theta','θ'],
	'csc(x)',
	'csc(3)',		
	'arcsin(x)',
	'arcsin(3)',
	'arccos(x)',
	'arccos(3)',
	'arctan(x)',
	'arctan(3)',
	'arccsc(x)',
	'arccsc(3)',
	'arcsec(x)',
	'arcsec(3)',
	'arccot(x)',
	'arccot(3)',
	'log(x)',
	'log(3)',
	'log(exp(x))',
	'e^x',
	'sqrt(x)',
	'sqrt(4)',
	'1/sqrt(3)',	
	'1/sqrt(-x)',
	'x+y+z',	
	'sin(3x)',
	'sin(3x)^2',
	'sin(x)^2 + cos(x)^2',
	'sin(x)^2 / cos(x)^2',
	'sin(x+y+z)^2',
	'sqrt(x+y+z)',
	'sqrt(sqrt(x))',
	'sqrt(1/(x+y))',
	'log(-x^2)',
	'|3|',
	'sin(|x|)',
	'|sin(|x|)|',
	'|sin(||x||)|',
	'||x|+|y|+|z||',
	'|x+y < z|',
	['infinity','∞'],
	"sin(x)'",
	"sin(x)''",
	'f(x)',
	"f'(x)",
	"f''(x)",
	"f(x)'",
	"sin'(x)",
	['sin x', 'sin(x)'],
	['sin xy', 'sin(x)y'],
	['sin^xyz', 'sin^x(y)z'],
	['y(x)', 'yx'],
	["y'(x)", "y'x"],
	'x^22',
	'x^ab',
	['x^y^z', 'x^(y^z)'],
	'(x^y)^z',
	['x_y_z', 'x_(y_z)'],
	'(x_y)_z',
	'x_y^z',
	['x^y_z', 'x^(y_z)'],
	'f^2',
	'f^2(x)',
	'f(x)^2',
	'f_t',
	'f_t(x)',
	'f_t^2(x)',
	'f_t\'(x)',
	'f\'^2(x)',
	'f_t\'^2(x)',
	'f_(s+t)\'\'^2(x)',
	'sin(x)\'',
	'x_(s+t)\'\'',
	'(x-1-2)^2',
	'(a,b)',
	'(a,b]',
	'[a,b)',
	'[a,b]',
	'{a,b}',
	'{a,b,c}',
	'{a}',
	'(a,b,c)',
	'[a,b,c]',
	'[a,b,c] + (a,b]',
	'x=y',
	'x=y=z',
	'x>y',
	['x>=y', 'x≥y'],
	'x>y>z',
	['x>y>=z', 'x>y≥z'],
	['x>=y>z', 'x≥y>z'],
	['x>=y>=z', 'x≥y≥z'],
	'x<y',
	['x<=y', 'x≤y'],
	'x<y<z',
	['x<y<=z', 'x<y≤z'],
	['x<=y<z','x≤y<z'],
	['x<=y<=z','x≤y≤z'],
	['A union B', 'A ∪ B'],
	['A intersect B', 'A ∩ B'],
	'C = A ∩ B',
	['A=1 & B=2', '(A=1) and (B=2)'],
	'A or B',
	'(A and B) or C',
	'A and (B or C)',
	'not(A and B)',
	'(A and B) < C',
	'(not A) = B',
	'(A and B) > (C and D) > (E and F)',
	'(A and B) + (C and D)',
	'(A and B) ∪ (C and D)',
	'(A and B) ∩ (C and D)',
	['x/y/z/w', '((x/y)/z)/w'],
	['x(x-1)/z', '(x(x-1))/z'],
	['A && B or C', '(A and B) or C'],
	['A or B & C', 'A or (B and C)'],
	['!A or B', '(not A) or B'],
	['A=1 or B=x/y', '(A=1) or (B=x/y)'],
	['x elementof (a,b)', 'x ∈ (a,b)'],
	['x notelementof (a,b)', 'x ∉ (a,b)'],
	['(a,b) containselement x', '(a,b) ∋ x'],
	['(a,b) notcontainselement x', '(a,b) ∌ x'],
	['(a,b) subset (c,d)', '(a,b) ⊂ (c,d)'],
	['(a,b) notsubset (c,d)', '(a,b) ⊄ (c,d)'],
	['(a,b) superset (c,d)', '(a,b) ⊃ (c,d)'],
	['(a,b) notsuperset (c,d)', '(a,b) ⊅ (c,d)'],
    ];

    _.each( inputs, function(input) {
	it(input, function() {
	    if(Array.isArray(input))
		expect(astToText(textToAst(input[0])).replace(/ /g,'')).toEqual(input[1].replace(/ /g,''));
	    else
		expect(astToText(textToAst(input)).replace(/ /g,'')).toEqual(input.replace(/ /g,''));
	});	
    });


    // Additional round trips to ast should not alter the strings at all
    _.each( inputs, function(input) {
	it(input, function() {
	    if(Array.isArray(input))
		expect(astToText(textToAst(astToText(textToAst(input[0]))))).toEqual(astToText(textToAst(input[0])));
	    else
		expect(astToText(textToAst(astToText(textToAst(input))))).toEqual(astToText(textToAst(input)));
	});	
    });


    it("unsplit context", function () {

	Context.unsplitSymbols =  [];
	expect(Context.fromText('3pi').tree).toEqual(['*', 3, 'p', 'i']);

	Context.unsplitSymbols.push('pi');
	expect(Context.fromText('3pi').tree).toEqual(['*', 3, 'pi']);

    });

    it("function symbol context", function () {
	Context.functionSymbols = [];
	expect(Context.fromText('f(x)+h(y)').tree).toEqual(
	    ['+',['*', 'f', 'x'], ['*', 'h', 'y']]);
	
	Context.functionSymbols.push('f');
	expect(Context.fromText('f(x)+h(y)').tree).toEqual(
	    ['+',['apply', 'f', 'x'], ['*', 'h', 'y']]);
	
	Context.functionSymbols.push('h');
	expect(Context.fromText('f(x)+h(y)').tree).toEqual(
	    ['+',['apply', 'f', 'x'], ['apply', 'h', 'y']]);
	
	Context.functionSymbols.push('x');
	expect(Context.fromText('f(x)+h(y)').tree).toEqual(
	    ['+',['apply', 'f', 'x'], ['apply', 'h', 'y']]);
	
    });
    
    it("applied function symbol context", function () {
	Context.appliedFunctionSymbols = [];
	expect(Context.fromText('sin(x) + custom(y)').tree).toEqual(
	    ['+', ['*', 's', 'i', 'n', 'x'], ['*', 'c', 'u', 's', 't', 'o', 'm', 'y']]);
	expect(Context.fromText('sin x + custom y').tree).toEqual(
	    ['+', ['*', 's', 'i', 'n', 'x'], ['*', 'c', 'u', 's', 't', 'o', 'm', 'y']]);

	Context.appliedFunctionSymbols.push('custom');
	expect(Context.fromText('sin(x) + custom(y)').tree).toEqual(
	    ['+', ['*', 's', 'i', 'n', 'x'], ['apply', 'custom', 'y']]);
	expect(Context.fromText('sin x + custom y').tree).toEqual(
	    ['+', ['*', 's', 'i', 'n', 'x'], ['apply', 'custom', 'y']]);
	
	Context.appliedFunctionSymbols.push('sin');
	expect(Context.fromText('sin(x) + custom(y)').tree).toEqual(
	    ['+', ['apply', 'sin', 'x'], ['apply', 'custom', 'y']]);
	expect(Context.fromText('sin x + custom y').tree).toEqual(
	    ['+', ['apply', 'sin', 'x'], ['apply', 'custom', 'y']]);

    });

});
