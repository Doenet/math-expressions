var astToText = require('../lib/parser').ast.to.text;
var textToAst = require('../lib/parser').text.to.ast;
var _ = require('underscore');

describe("ast to text", function() {
    it('|4|', function() {    
	expect(textToAst('|4|')).toEqual(['abs',4]);
    });
    
    var inputs = [
	'3+4',
	'1/2',
	'-2',
	'x^2',
	'sin x',
	'sin 3',
	'cos x',
	'cos 3',
	'tan x',
	'tan 3',
	'sec x',
	'sec 3',
	'csc x',
	'csc 3',		
	'arcsin x',
	'arcsin 3',
	'arccos x',
	'arccos 3',
	'arctan x',
	'arctan 3',
	'arccsc x',
	'arccsc 3',
	'arcsec x',
	'arcsec 3',
	'arccot x',
	'arccot 3',
	'log x',
	'log 3',
	'log(exp x)',
	'e^x',
	'sqrt x',
	'sqrt 4',
	'1/sqrt 3',	
	'1/sqrt(-x)',
	'1+2+3',	
	'x+y+z',	
	'sin(3x)',
	'sin^2(3x)',
	'sin^2 x + cos^2 x',
	'|3|',
	'3+(-4)',
    ];

    _.each( inputs, function(input) {
	it(input, function() {
	    expect(astToText(textToAst(input)).replace(/ /g,'')).toEqual(input.replace(/ /g,''));
	});	
    });

    _.each( inputs, function(input) {
	it(input, function() {
	    expect(astToText(textToAst(astToText(textToAst(input)))).replace(/ /g,'')).toEqual(input.replace(/ /g,''));
	});	
    });    
});
