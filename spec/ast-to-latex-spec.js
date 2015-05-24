var _ = require('underscore');
var textToLatex = require('../lib/parser').text.to.latex;

describe("ast to latex", function() {
    var texts = {
	'1+x+3': '1 + x + 3',
	'|x|': '\\left|x\\right|',
	'sin^2 x': '\\sin^2 x',
    };

    _.each( _.keys(texts), function(text) {
	it(text, function() {
	    expect(textToLatex(text)).toEqual(texts[text]);
	});	
    });    
        
});
