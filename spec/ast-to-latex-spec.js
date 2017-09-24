var _ = require('underscore');
var textToLatex = require('../lib/parser').text.to.latex;
var astToLatex = require('../lib/parser').ast.to.latex;

describe("ast to latex", function() {
    var texts = {
	'1+x+3': '1 + x + 3',
	'1-x-3': '1 - x - 3',
	'1 + x^2 + 3x^5': '1 + x^{2} + 3 \\, x^{5}',
	'|x|': '\\left|x\\right|',
	'sin^2 x': '\\sin^{2}\\left(x\\right)',
	'log x': '\\log\\left(x\\right)',
	'log |x|': '\\log\\left(\\left|x\\right|\\right)',
	'ln x': '\\ln\\left(x\\right)',
	'ln |x|': '\\ln\\left(\\left|x\\right|\\right)',
	'sin^2 (3x)': '\\sin^{2}\\left(3 \\, x\\right)',
	'sin x': '\\sin\\left(x\\right)',
	'x!': 'x!',
	'17!': '17!',
	'sqrt(-x)': '\\sqrt{- x}',
	'x^y z': 'x^{y} \\, z',
	'2^(2^x)': '2^{2^{x}}',
	'(2^x)^y': '\\left(2^{x}\\right)^{y}',
	'x^(2y) z': 'x^{2 \\, y} \\, z',
	'n!': 'n!',
	'1/(x^2 + x + 1)': '\\frac{1}{x^{2} + x + 1}',
	'oo': '\\infty',	
    };

    _.each( _.keys(texts), function(text) {
	it("converts " + text + " into " + texts[text], function() {
	    expect(textToLatex(text)).toEqual(texts[text]);
	});	
    });    

    it("vector", function() {
        expect(astToLatex(['vector', 1, 'x']).replace(/ /g,'')).toEqual('\\left(1,x\\right)');
    });
        
});
