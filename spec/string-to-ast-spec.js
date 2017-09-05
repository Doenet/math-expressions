var astToString = require('../lib/parser').ast.to.string;
var stringToAst = require('../lib/parser').string.to.ast;
var _ = require('underscore');


describe("string to ast", function() {
    var trees = {
	'1+x+3': ['+',1,'x',3],
	"1 + - x": ['+',1,['~','x']],
	"1 - x": ['+',1,['~','x']],
	"1 - - x": ['+',1,['~',['~','x']]],
	"1 + x/2": ['+',1,['/','x',2]],
	'1-x-3': ['+',1,['~','x'],['~',3]],	
	'x^2': ['^', 'x', 2],
	'-x^2': ['~',['^', 'x', 2]],
	'-3^2': ['~',['^', 3, 2]],
	'x*y*z': ['*','x','y','z'],
	'xyz': ['*','x','y','z'],
	'x*y*z*w': ['*','x','y','z','w'],
	'xyzw': ['*','x', 'y', 'z', 'w'],
	'(x*y)*(z*w)': ['*','x','y','z','w'],		
	'|x|': ['abs','x'],
	'a!': ['_factorial','a'],	
	'theta': 'theta',
	'cos(theta)': ['cos','theta'],
	'x!': ['_factorial','x'],	
	'|sin(|x|)|': ['abs', ['sin', ['abs', 'x']]],
	'sin(θ)': ['sin', 'theta'],
	'x_y_z': ['_', 'x', ['_','y','z']],
	'x_(y_z)': ['_', 'x', ['_','y','z']],
	'(x_y)_z': ['_', ['_', 'x', 'y'],'z'],
	'x^y^z': ['^', 'x', ['^','y','z']],
	'x^(y^z)': ['^', 'x', ['^','y','z']],
	'(x^y)^z': ['^', ['^', 'x', 'y'],'z'],
	'x^y_z': ['^', 'x', ['_','y','z']],
	'x_y^z': ['^', ['_','x','y'],'z'],
	'xyz!': ['*','x','y', ['_factorial', 'z']],
	'f\'': ['_prime', 'f'],
	'fg\'': ['*', 'f', ['_prime', 'g']],
	'f\'g': ['*', ['_prime', 'f'], 'g'],
	'f\'g\'\'': ['*', ['_prime', 'f'], ['_prime', ['_prime', 'g']]],
	
    };

    _.each( _.keys(trees), function(string) {
	it("parses " + string, function() {
	    expect(stringToAst(string)).toEqual(trees[string]);
	});	
    });    


    var bad_inputs = {
	'1++1': "Parse Error: Invalid location of '+'",
	')1++1': "Parse Error: Invalid location of ')'",
	'(1+1': "Parse Error: Expected )",
	'x-y-': "Parse Error: Unexpected end of input",
	'sin x': "Parse Error: Expected ( after function",
	'sin^2(x)': "Parse Error: Expected ( after function",
	'|x| |y|': "Parse Error: Invalid location of '|'",
	'_x': "Parse Error: Invalid location of _",
	'x_': "Parse Error: Unexpected end of input",
	'x@2': "Parse Error: Invalid symbol '@'",
	'|y/v': "Parse Error: Expected |",
	'x+^2': "Parse Error: Invalid location of ^",
	'x(!y)': "Parse Error: Invalid location of '!'",
	'x/\'y': "Parse Error: Invalid location of '",
	
    }

    _.each( _.keys(bad_inputs), function(string) {
	it("throws " + string, function() {
	    expect(function() {stringToAst(string)}).toThrow(bad_inputs[string]);
	});	
    });    
    

	
    var inputs = [
	'3+4',
	'1/2',
	'-2',
	'x!',
	'17!',
	'(x+1)!',
	'(x^2+1)!',
	'x^2',
	'sin(x)',
	'sin(3)',
	'cos(x)',
	'cos(3)',
	'tan(x)',
	'tan(3)',
	'sec(x)',
	'sec(3)',
	'theta',
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
	'1+2+3',	
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
	'3-4',
	'|sin(|x|)|',
	'|sin(||x||)|',
	'||x|+|y|+|z||',
	'x!',
	'n!',
	'(n+1)!',
	'(n-1)!',
	'infinity',
    ];

    _.each( inputs, function(input) {
	it(input, function() {
	    expect(astToString(stringToAst(input)).replace(/ /g,'')).toEqual(input.replace(/ /g,''));
	});	
    });

    _.each( inputs, function(input) {
	it(input, function() {
	    expect(astToString(stringToAst(astToString(stringToAst(input)))).replace(/ /g,'')).toEqual(input.replace(/ /g,''));
	});	
    });    
});
