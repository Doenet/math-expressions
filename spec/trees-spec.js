var trees = require('../lib/trees/basic.js')
var flatten = require('../lib/trees/flatten.js')

var Expression = require ('../lib/math-expressions');
function TREE(s) {
    return Expression.fromText(s).tree;
}

describe("tree basics", function() {
    it("cos x == cos x", function() {
	expect(trees.equal( TREE('cos x'), TREE('cos x') )).toBeTruthy();
    });

    it("cos x !== cos y", function() {
	expect(trees.equal( TREE('cos x'), TREE('cos y') )).toBeFalsy();
    });

    it("(1+2+3) unflattens right to (1+(2+3))", function() {
	expect(trees.equal( flatten.unflattenRight( ['+', 1, 2, 3]),
			    ['+', 1, ['+', 2, 3]])).toBeTruthy();
    });

    it("(1+2+3) unflattens left to ((1+2)+3)", function() {
	expect(trees.equal( flatten.unflattenLeft( ['+', 1, 2, 3]),
			    ['+', ['+', 1, 2], 3])).toBeTruthy();
    });

    it("(1+(2+3)) flattens to (1+2+3)", function() {
	expect(trees.equal( flatten.flatten( ['+', 1, ['+', 2, 3]]),
			    ['+', 1, 2, 3] )).toBeTruthy();
    });
    
    it("((1+2)+3) flattens to (1+2+3)", function() {
	expect(trees.equal( flatten.flatten( ['+', ['+', 1, 2], 3]),
			    ['+', 1, 2, 3] )).toBeTruthy();
    });

    it("x+y becomes 1+2 when x:=1 and y:=2", function() {
	expect(trees.equal( trees.substitute( ['+', 'x', 'y'], {x:1,y:2} ),
			    ['+', 1, 2] )).toBeTruthy();
    });

    it("x+y becomes y^2+x^2 when x := y^2 and y := x^2", function() {
	expect(trees.equal( trees.substitute( TREE('x+y'), {x:TREE('y^2'),y:TREE('x^2')} ),
			    TREE('y^2 + x^2') )).toBeTruthy();
    });

    it("substituting cos(x+y)/sin(x-y) with x:=1 and y:=2", function() {
	expect(trees.equal( trees.substitute( TREE('cos(x+y)/sin(x-y)'), {x:1,y:2} ),
			    TREE('cos(1+2)/sin(1-2)') )).toBeTruthy();
    });

    it("substituting x*y/(x^y) with x := (y+1) and y := (x+2)", function() {
	expect(trees.equal( trees.substitute( TREE('x*y/(x^y)'), {x:TREE('y+1'),y:TREE('x+2')} ),
			    TREE('(y+1)*(x+2)/((y+1)^(x+2))') )).toBeTruthy();
    });


});

describe("tree matching", function () {
    it("x+y matches the pattern a+b", function() {
	expect(trees.match( TREE('x+y'), TREE('a+b') )).toEqual( {a:'x', b:'y'} );
    });

    it("x+y does not match the pattern a*b", function() {
	expect(trees.match( TREE('x+y'), TREE('a*b') )).toBeFalsy();
    });

    it("match must match entire tree", function() {
	expect(trees.match( TREE('x+y/z'), TREE('a/b') )).toBeFalsy();
    });

    it("match must be consistent", function () {
	var target1 = TREE('x+y/z');
	var target2 = TREE('x+y/x');
	expect(trees.match( target1, TREE('a+b/c'))).toBeTruthy();
	expect(trees.match( target1, TREE('a+b/a'))).toBeFalsy();
	expect(trees.match( target2, TREE('a+b/a'))).toBeTruthy();
    });	

    it("by default even multicharacter patterns are placeholders", function () {
	expect(trees.match( ['+', 'x', 'y'], ['+', 'a', 'bc'])).toBeTruthy();
	expect(trees.match( ['+', 'x', 'bc'], ['+', 'a', 'bc'])).toBeTruthy();
    });

    it("numbers match exactly", function () {
	expect(trees.match( TREE('3x+5'), TREE('ab+5'))).toBeTruthy();
	expect(trees.match( TREE('3x+5'), TREE('ab+6'))).toBeFalsy();
    });

    it("addition matches subtraction but not vice-versa", function () {
	expect(trees.match( TREE('x-y'), TREE('a+b'))).toBeTruthy();
	expect(trees.match( TREE('x+y'), TREE('a-b'))).toBeFalsy();
    });

    if("specify variables", function () {
	expect(trees.match( TREE('x+y'), TREE('a+b'))).toBeTruthy();
	expect(trees.match( TREE('x+y'), TREE('a+b')), {a: true, b: true}).toBeTruthy();
	expect(trees.match( TREE('x+y'), TREE('a+b')), {a: true}).toBeFalsy();
	expect(trees.match( TREE('x+y'), TREE('a+b')), {b: true}).toBeFalsy();
    });
    
    it("matching with function conditions", function () {
	var pattern = TREE('a+b');
	function isString(s) { return (typeof s === 'string');}
	function isNumber(s) { return (typeof s === 'number');}

	expect(trees.match( TREE('2x+y'), pattern)).toBeTruthy();
	expect(trees.match( TREE('2x+y'), pattern, {a: true, b: isString})).toBeTruthy();
	expect(trees.match( TREE('x+2y'), pattern)).toBeTruthy();
 	expect(trees.match( TREE('x+2y'), pattern, {a: true, b: isString})).toBeFalsy();

	expect(trees.match( TREE('x+2'), pattern, {a: isString, b: isNumber})).toBeTruthy();
	expect(trees.match( TREE('x+2'), pattern, {b: isString, a: isNumber})).toBeFalsy();

    });

    it("matching with regular expression conditions", function () {
	var pattern = TREE('a+b');

	expect(trees.match( TREE('2x+3y'), pattern)).toBeTruthy();
	expect(trees.match( TREE('2x+3y'), pattern, {a: true, b: /y/})).toBeTruthy();
	expect(trees.match( TREE('2x+3y'), pattern, {a: true, b: /^y$/})).toBeFalsy();
	expect(trees.match( TREE('2x+3y'), pattern, {a: /.*/, b: /y/})).toBeTruthy();

 	expect(trees.match( TREE('2x+3y'), pattern, {a: true, b: /^[a-zA-Z]$/})).toBeFalsy();
 	expect(trees.match( TREE('2x+3y'), TREE('a+3b'), {a: true, b: /^[a-zA-Z]$/})).toBeTruthy();

	expect(trees.match( TREE('x+2'), pattern, {a: /^[a-zA-Z]$/, b: /^\d+$/})).toBeTruthy();
	expect(trees.match( TREE('x+2'), pattern, {b: /^[a-zA-Z]$/, a: /^\d+$/})).toBeFalsy();

    });

    it("invalid matching conditions fail gracefully", function () {

	expect(trees.match( TREE('2x+y'),  TREE('a+b'), {a: false, b: 'h'})).toBeFalsy();

    });

    
    it("match with permutation", function () {
	var pattern = TREE('e^(ax^2+bx+c)');
	function isNumber(s) { return (typeof s === 'number');}
	
	expect(trees.match( TREE('e^(0.3s^2+3s+7)'), pattern,
			    { a: isNumber, b: isNumber, c: isNumber,
			      x: /^[a-zA-Z]$/ }
			  )).toBeTruthy();

	expect(trees.match( TREE('e^(7+3s+s^2*0.3)'), pattern,
			    { a: isNumber, b: isNumber, c: isNumber,
			      x: /^[a-zA-Z]$/ }
			  )).toBeFalsy();

	expect(trees.match( TREE('e^(7+3s+s^2*0.3)'), pattern,
			    { a: isNumber, b: isNumber, c: isNumber,
			      x: /^[a-zA-Z]$/ },
			    {allow_permutations: true}
			  )).toBeTruthy();
	
    });
    

    it("match with implicit identity", function () {
	var pattern = TREE('ax^2+bx+c');
	
	function isNumber(s) {
	    if (typeof s === 'number')
		return true;
	    if (Array.isArray(s) && s[0] === '-' && (typeof s[1] === 'number'))
		return true;
	    return false;
	}

	var match = trees.match( TREE('0.3s^2-3s+7'), pattern,
				 { a: isNumber, b: isNumber, c: isNumber,
				   x: /^[a-zA-Z]$/ },
				 {allow_permutations: true}
			       );
	
	expect(match).toBeTruthy();
	expect(match['a']==0.3 && trees.equal(match['b'],['-', 3])
	       && match['c'] == 7).toBeTruthy();

	expect(trees.match( TREE('s^2-3s+7'), pattern,
			    { a: isNumber, b: isNumber, c: isNumber,
			      x: /^[a-zA-Z]$/ },
			    {allow_permutations: true}
			  )).toBeFalsy();

	match = trees.match( TREE('s^2-3s+7'), pattern,
			     { a: isNumber, b: isNumber, c: isNumber,
			       x: /^[a-zA-Z]$/ },
			     { allow_permutations: true,
			       allow_implicit_identities: ['a']}
			   );
	expect(match).toBeTruthy();
	expect(match['a']==1 && trees.equal(match['b'],['-', 3])
	       && match['c'] == 7).toBeTruthy();


	expect(trees.match( TREE('s^2-3s'), pattern,
			    { a: isNumber, b: isNumber, c: isNumber,
			      x: /^[a-zA-Z]$/ },
			    { allow_permutations: true,
			      allow_implicit_identities: ['a']}
			  )).toBeFalsy();
	
	match = trees.match( TREE('s^2-3s'), pattern,
			     { a: isNumber, b: isNumber, c: isNumber,
			      x: /^[a-zA-Z]$/ },
			     { allow_permutations: true,
			       allow_implicit_identities: ['a', 'c']}
			   );
	expect(match).toBeTruthy();
	expect(match['a']==1 && trees.equal(match['b'],['-', 3])
	       && match['c'] == 0).toBeTruthy();
	
       
    });


    it("consistency with with implicit identity", function () {
	var pattern = TREE('bx^2+bx+c');

	function isNumber(s) {
	    if (typeof s === 'number')
		return true;
	    if (Array.isArray(s) && s[0] === '-' && (typeof s[1] === 'number'))
		return true;
	    return false;
	}

	expect(trees.match( TREE('s^2-3s+7'), pattern,
			    { a: isNumber, b: isNumber, c: isNumber,
			      x: /^[a-zA-Z]$/ },
			    { allow_permutations: true,
			      allow_implicit_identities: ['b']}
			  )).toBeFalsy();
	expect(trees.match( TREE('s^2+1s+7'), pattern,
			    { a: isNumber, b: isNumber, c: isNumber,
			      x: /^[a-zA-Z]$/ },
			    { allow_permutations: true,
			      allow_implicit_identities: ['b']}
			  )).toBeTruthy();
	expect(trees.match( TREE('s^2+s+7'), pattern,
			    { a: isNumber, b: isNumber, c: isNumber,
			      x: /^[a-zA-Z]$/ },
			    { allow_permutations: true,
			      allow_implicit_identities: ['b']}
			  )).toBeTruthy();
	
    });
	
});

describe("tree transformations", function () {

    it("replacing x*y in x*y+z with y/x results in y/x+z", function() {
	var subtree = ['*', 'x', 'y'];
	var replacement = ['/', 'y', 'x'];
	var root = ['+', subtree, 'z'];

	expect(trees.equal( trees.replaceSubtree( root, subtree, replacement ),
			    TREE('y/x + z') )).toBeTruthy();
    });    

    it("applying commutativity to a+b+c finds some permutations", function () {
	var tree = ['+', 'a', ['+', 'b', 'c']];
	var pattern = TREE('a+b');
	var replacement = TREE('b+a');

	var left = ['+', 'a', ['+', 'c', 'b']];
	var right = ['+', ['+', 'b', 'c'], 'a'];
	
	var results = trees.applyTransformationEachSubtree( tree, pattern, replacement);
	expect(results.length).toEqual(2);
	
	expect( ((trees.equal(results[0], left) && trees.equal(results[1], right))) ||
		((trees.equal(results[1], left) && trees.equal(results[0], right))) ).toBeTruthy();
    });
    
    it("applying commutativity and associativity finds some equalities", function() {
	var commutativity = trees.patternTransformer( ['+', 'a', 'b'],
						      ['+', 'b', 'a'] );
	var associativity = trees.patternTransformer( ['+', 'a', ['+', 'b', 'c']],
						      ['+', ['+', 'a', 'b'], 'c'] );

	var left = ['+', 'a', ['+', 'b', ['+', 'c', 'd']]];
	var right = ['+', 'd', ['+', 'b', ['+', 'c', 'a']]];

	expect(trees.equalAfterTransformations( left, right, [commutativity, associativity] )).toBeTruthy();
    });

    it("applying commutativity and associativity finds some non-equalities", function() {
	var commutativity = trees.patternTransformer( ['+', 'a', 'b'],
						      ['+', 'b', 'a'] );
	var associativity = trees.patternTransformer( ['+', 'a', ['+', 'b', 'c']],
						      ['+', ['+', 'a', 'b'], 'c'] );

	var left = ['+', 'a', ['+', 'b', ['+', 'c', 'd']]];
	var right = ['+', 'a', ['+', 'b', ['+', 'c', 'a']]];

	expect(trees.equalAfterTransformations( left, right, [commutativity, associativity], 3 )).toBeFalsy();
    });
    
    it("Apply distributive transformations to expand polynomial", function () {
	
	var transformations = [];
	transformations.push([TREE("a*(b+c)"), TREE("a*b+a*c")]);
	transformations.push([TREE("(a+b)*c"), TREE("a*c+b*c")]);

	var factored = ['*',
			['+', 'a', 'b'],
			'x',
			['+', ['*', 2, 'y'], ['*', 'p','q']]
		       ]

	var expanded = ['+',
			['*', 'a', 'x', 2, 'y'],
			['*', 'a', 'x', 'p', 'q'],
			['*', 'b', 'x', 2, 'y'],
			['*', 'b', 'x', 'p', 'q']];

	factored = flatten.unflattenRight(factored);

	transformed = trees.applyAllTransformations(factored, transformations);

	transformed = flatten.flatten(transformed);
	
	expect(trees.equal(expanded, transformed)).toBeTruthy();
	
    });
       

});

