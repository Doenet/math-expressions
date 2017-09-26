var astToMathTree = require('../lib/parser').ast.to.mathTree;
var textToAst = require('../lib/parser').text.to.ast;
var textToMathTree = require('../lib/parser').text.to.mathTree;
var to_intervals = require('../lib/expression/normalization/tuples')._to_intervals_ast;
var _ = require('underscore');
var math = require('../lib/mathjs');

describe("ast to math tree", function() {

    // Inputs that are strings should result in equal math.js syntax trees
    // if processed text->ast->math tree or processed directly via math.parse.
    // If an input is a 2D array, then processing the first string via
    // text->ast->math tree should equal to processing the second via math.parse.
    // Apply math.simplify before comparisons
    var inputs = [
	'x+2*y',
	'(x-1)/3-x^2/(1-z+y)',
	['sin(2y)/3', 'sin(2*y)/3'],
	'csc(pi/2)',
	'x < y',
	'x <= y',
	'x > y',
	'x >= y',
	['a = b+c', 'a == b+c'],
	'a != b + c',
	'a and b or c',
	'(a and b) or c',
	'a and (b or c)',
	'not (a or b)',
	'a and b and c',
	'a or b or c',
	['x < y < z', 'x < y and y < z'],
	['x < y <= z', 'x < y and y <= z'],
	['x <= y < z', 'x <= y and y < z'],
	['x <= y <= z', 'x <= y and y <= z'],
	['x > y > z', 'x > y and y > z'],
	['x > y >= z', 'x > y and y >= z'],
	['x >= y > z', 'x >= y and y > z'],
	['x >= y >= z', 'x >= y and y >= z'],
	['x elementof (a,b]', 'x > a and x <= b'],
	['x notelementof (a,b]', 'not(x > a and x <= b)'],
	['x elementof [a,b)', 'x >= a and x < b'],
	['x notelementof [a,b)', 'not(x >= a and x < b)'],
	['(a,b] containselement x', 'x > a and x <= b'],
	['(a,b] notcontainselement x', 'not(x > a and x <= b)'],
    ];


    _.each( inputs, function(input) {
	it(input, function() {
	    if(Array.isArray(input)) {
		expect(math.simplify(textToMathTree(input[0]))
		       .equals(math.simplify(math.parse(input[1]))))
		    .toBeTruthy();
	    }
	    else {
		expect(math.simplify(textToMathTree(input))
		       .equals(math.simplify(math.parse(input))))
		    .toBeTruthy();
	    }
	});	
    });


    // in addition, apply to_intervals, i.e., process via
    // text -> ast -> to_intervals -> math tree
    var inputs = [
	['x elementof (a,b)', 'x > a and x < b'],
	['x notelementof (a,b)', 'not(x > a and x < b)'],
	['x elementof [a,b]', 'x >= a and x <= b'],
	['x notelementof [a,b]', 'not(x >= a and x <= b)'],
	['(a,b) containselement x', 'x > a and x < b'],
	['(a,b) notcontainselement x', 'not(x > a and x < b)'],
	['[a,b] containselement x', 'x >= a and x <= b'],
	['[a,b] notcontainselement x', 'not(x >= a and x <= b)'],
	['(a,b) subset (c,d)', 'a >= c and b <= d'],
	['(a,b) subset (c,d]', 'a >= c and b <= d'],
	['(a,b) subset [c,d)', 'a >= c and b <= d'],
	['(a,b) subset [c,d]', 'a >= c and b <= d'],
	['[a,b] subset (c,d)', 'a > c and b < d'],
	['[a,b] subset (c,d]', 'a > c and b <= d'],
	['[a,b] subset [c,d)', 'a >= c and b < d'],
	['[a,b] subset [c,d]', 'a >= c and b <= d'],
	['[a,b) subset (c,d)', 'a > c and b <= d'],
	['[a,b) subset (c,d]', 'a > c and b <= d'],
	['[a,b) subset [c,d)', 'a >= c and b <= d'],
	['[a,b) subset [c,d]', 'a >= c and b <= d'],
	['(a,b] subset (c,d)', 'a >= c and b < d'],
	['(a,b] subset (c,d]', 'a >= c and b <= d'],
	['(a,b] subset [c,d)', 'a >= c and b < d'],
	['(a,b] subset [c,d]', 'a >= c and b <= d'],
	['(a,b) superset (c,d)', 'c >= a and d <= b'],
	['(a,b] superset (c,d)', 'c >= a and d <= b'],
	['[a,b) superset (c,d)', 'c >= a and d <= b'],
	['[a,b] superset (c,d)', 'c >= a and d <= b'],
	['(a,b) superset [c,d]', 'c > a and d < b'],
	['(a,b] superset [c,d]', 'c > a and d <= b'],
	['[a,b) superset [c,d]', 'c >= a and d < b'],
	['[a,b] superset [c,d]', 'c >= a and d <= b'],
	['(a,b) superset [c,d)', 'c > a and d <= b'],
	['(a,b] superset [c,d)', 'c > a and d <= b'],
	['[a,b) superset [c,d)', 'c >= a and d <= b'],
	['[a,b] superset [c,d)', 'c >= a and d <= b'],
	['(a,b) superset (c,d]', 'c >= a and d < b'],
	['(a,b] superset (c,d]', 'c >= a and d <= b'],
	['[a,b) superset (c,d]', 'c >= a and d < b'],
	['[a,b] superset (c,d]', 'c >= a and d <= b'],
	['(a,b) notsubset (c,d)', 'not(a >= c and b <= d)'],
	['(a,b) notsuperset (c,d)', 'not(c >= a and d <= b)'],
    ];


    _.each( inputs, function(input) {
	it(input, function() {
	    if(Array.isArray(input)) {
		expect(math.simplify(astToMathTree(to_intervals(textToAst(input[0]))))
		       .equals(math.simplify(math.parse(input[1]))))
		    .toBeTruthy();
	    }
	    else {
		expect(math.simplify(astToMathTree(to_intervals(textToAst(input))))
		       .equals(math.simplify(math.parse(input))))
		    .toBeTruthy();
	    }
	});	
    });

    
});
