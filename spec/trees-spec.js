var trees = require('../lib/trees/basic.js')
var flatten = require('../lib/trees/flatten.js')

var me = require ('../lib/math-expressions');
function TREE(s) {
    return me.fromText(s).tree;
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

    it("children of (1+(2+3)", function () {
	expect(flatten.allChildren(['+', 1, ['+', 2, 3]])).toEqual([1,2,3]);
    });
    
    it("children of ((1+2)+3)", function () {
	expect(flatten.allChildren(['+', ['+', 1, 2], 3])).toEqual([1,2,3]);
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

    it("specify variables", function () {
	expect(trees.match( TREE('x+y'), TREE('a+b'))).toBeTruthy();
	expect(trees.match( TREE('x+y'), TREE('a+b'), {variables: {a: true, b: true}})).toBeTruthy();
	expect(trees.match( TREE('x+y'), TREE('a+b'), {variables: {a: true}})).toBeFalsy();
	expect(trees.match( TREE('x+y'), TREE('a+b'), {variables: {b: true}})).toBeFalsy();
    });
    
    it("matching with function conditions", function () {
	var pattern = TREE('a+b');
	function isString(s) { return (typeof s === 'string');}
	function isNumber(s) { return (typeof s === 'number');}

	expect(trees.match( TREE('2x+y'), pattern)).toBeTruthy();
	expect(trees.match( TREE('2x+y'), pattern, {variables: {a: true, b: isString}})).toBeTruthy();
	expect(trees.match( TREE('x+2y'), pattern)).toBeTruthy();
 	expect(trees.match( TREE('x+2y'), pattern, {variables: {a: true, b: isString}})).toBeFalsy();

	expect(trees.match( TREE('x+2'), pattern, {variables: {a: isString, b: isNumber}})).toBeTruthy();
	expect(trees.match( TREE('x+2'), pattern, {variables: {b: isString, a: isNumber}})).toBeFalsy();

    });

    it("matching with regular expression conditions", function () {
	var pattern = TREE('a+b');

	expect(trees.match( TREE('2x+3y'), pattern)).toBeTruthy();
	expect(trees.match( TREE('2x+3y'), pattern,
			    {variables: {a: true, b: /y/}})).toBeTruthy();
	expect(trees.match( TREE('2x+3y'), pattern,
			    {variables: {a: true, b: /^y$/}})).toBeFalsy();
	expect(trees.match( TREE('2x+3y'), pattern,
			    {variables: {a: /.*/, b: /y/}})).toBeTruthy();

 	expect(trees.match( TREE('2x+3y'), pattern,
			    {variables: {a: true, b: /^[a-zA-Z]$/}})).toBeFalsy();
 	expect(trees.match( TREE('2x+3y'), TREE('a+3b'),
			    {variables: {a: true, b: /^[a-zA-Z]$/}})).toBeTruthy();

	expect(trees.match( TREE('x+2'), pattern,
			    {variables: {a: /^[a-zA-Z]$/, b: /^\d+$/}})).toBeTruthy();
	expect(trees.match( TREE('x+2'), pattern,
			    {variables: {b: /^[a-zA-Z]$/, a: /^\d+$/}})).toBeFalsy();

    });

    it("invalid matching conditions fail gracefully", function () {

	expect(trees.match( TREE('2x+y'),  TREE('a+b'),
			    {variables: {a: false, b: 'h'}})).toBeFalsy();

    });

    
    it("match with permutation", function () {
	var pattern = TREE('e^(ax^2+bx+c)');
	function isNumber(s) { return (typeof s === 'number');}
	
	expect(trees.match( TREE('e^(0.3s^2+3s+7)'), pattern,
			    {variables: { a: isNumber, b: isNumber, c: isNumber,
					  x: /^[a-zA-Z]$/ }}
			  )).toBeTruthy();

	expect(trees.match( TREE('e^(7+3s+s^2*0.3)'), pattern,
			    {variables: { a: isNumber, b: isNumber, c: isNumber,
					  x: /^[a-zA-Z]$/ }}
			  )).toBeFalsy();

	expect(trees.match( TREE('e^(7+3s+s^2*0.3)'), pattern,
			    {variables: { a: isNumber, b: isNumber, c: isNumber,
					  x: /^[a-zA-Z]$/ },
			     allow_permutations: true}
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

	var match = trees.match(
	    TREE('0.3s^2-3s+7'), pattern,
	    {variable: { a: isNumber, b: isNumber, c: isNumber,
			 x: /^[a-zA-Z]$/ },
	     allow_permutations: true}
	);
	
	expect(match).toBeTruthy();
	expect(match['a']==0.3 && trees.equal(match['b'],['-', 3])
	       && match['c'] == 7).toBeTruthy();

	expect(trees.match(
	    TREE('s^2-3s+7'), pattern,
	    { variables: { a: isNumber, b: isNumber, c: isNumber,
			   x: /^[a-zA-Z]$/ },
	      allow_permutations: true}
	)).toBeFalsy();

	match = trees.match(
	    TREE('s^2-3s+7'), pattern,
	    {variables: { a: isNumber, b: isNumber, c: isNumber,
			  x: /^[a-zA-Z]$/ },
	     allow_permutations: true,
	     allow_implicit_identities: ['a']}
	);
	expect(match).toBeTruthy();
	expect(match['a']==1 && trees.equal(match['b'],['-', 3])
	       && match['c'] == 7).toBeTruthy();


	expect(trees.match(
	    TREE('s^2-3s'), pattern,
	    {variables: { a: isNumber, b: isNumber, c: isNumber,
			  x: /^[a-zA-Z]$/ },
	     allow_permutations: true,
	     allow_implicit_identities: ['a']}
	)).toBeFalsy();
	
	match = trees.match(
	    TREE('s^2-3s'), pattern,
	    {variables: { a: isNumber, b: isNumber, c: isNumber,
			  x: /^[a-zA-Z]$/ },
	     allow_permutations: true,
	     allow_implicit_identities: ['a', 'c']}
	);
	expect(match).toBeTruthy();
	expect(match['a']==1 && trees.equal(match['b'],['-', 3])
	       && match['c'] == 0).toBeTruthy();
	

	match = trees.match(
	    TREE('2y-y'), TREE('mx+nx'),
	    {variables: { m: isNumber, n: isNumber,
			  x: true },
	     allow_permutations: true,
	     allow_implicit_identities: ['m', 'n']}
	);
	expect(match).toBeTruthy();
	expect(match['m']).toEqual(2);
	expect(match['n']).toEqual(-1);
	expect(match['x']).toEqual('y');

	
	match = trees.match(TREE('m'), TREE('a^b'),
			    { allow_implicit_identities: ['b']});
       
	expect(match).toBeTruthy();
	expect(match['a']).toEqual('m');
	expect(match['b']).toEqual(1);
       
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

	expect(trees.match(
	    TREE('s^2-3s+7'), pattern,
	    {variables: { a: isNumber, b: isNumber, c: isNumber,
			  x: /^[a-zA-Z]$/ },
	     allow_permutations: true,
	     allow_implicit_identities: ['b']}
	)).toBeFalsy();
	expect(trees.match(
	    TREE('s^2+1s+7'), pattern,
	    {variables: { a: isNumber, b: isNumber, c: isNumber,
			  x: /^[a-zA-Z]$/ },
	     allow_permutations: true,
	     allow_implicit_identities: ['b']}
	)).toBeTruthy();
	expect(trees.match(
	    TREE('s^2+s+7'), pattern,
	    {variables: { a: isNumber, b: isNumber, c: isNumber,
			  x: /^[a-zA-Z]$/ },
	     allow_permutations: true,
	     allow_implicit_identities: ['b']}
	)).toBeTruthy();
	
    });


    it("match chunks", function() {
	
	function isNumber(s) {
	    if (typeof s === 'number')
		return true;
	    if (Array.isArray(s) && s[0] === '-' && (typeof s[1] === 'number'))
		return true;
	    return false;
	}

	var pattern = TREE('a+b');

	var match = trees.match(TREE('xyz+5a-2'), pattern);

	expect(match).toBeTruthy();
	expect(trees.equal(match['a'], TREE('xyz'))
	       && trees.equal(match['b'], TREE('5a-2'))).toBeTruthy();

    	match = trees.match(
	    TREE('xyz+5a-2'), pattern,
	    {variables: {a: true, b: isNumber}});

	expect(match).toBeTruthy();
	expect(trees.equal(match['a'], TREE('xyz+5a'))
	       && trees.equal(match['b'], TREE('-2'))).toBeTruthy();

        match = trees.match(
	    TREE('xyz+5a-2'), pattern,
	    {variables: {a: isNumber, b: true}});

	expect(match).toEqual(false);
	
        match = trees.match(
	    TREE('xyz+5a-2'), pattern,
	    {variables: {a: isNumber, b: true}, allow_permutations: true});

	expect(match).toBeTruthy();
	expect(trees.equal(match['b'], TREE('xyz+5a'))
	       && trees.equal(match['a'], TREE('-2'))).toBeTruthy();

	pattern = TREE('a+b+c');

        match = trees.match(
	    TREE('xyz+5a-2+2q-3/u'), pattern,
	    {variables: {a: isNumber, b: true, c: isNumber},
	     allow_permutations: true});
	
	expect(match).toEqual(false);
	
        match = trees.match(
	    TREE('xyz+7+5a-2+2q-3/u'), pattern,
	    {variables: {a: isNumber, b: true, c: isNumber},
	     allow_permutations: true});
	
	expect(match).toBeTruthy();

    });

    it("extended match", function() {

	function isNumber(s) {
	    if (typeof s === 'number')
		return true;
	    if (Array.isArray(s) && s[0] === '-' && (typeof s[1] === 'number'))
		return true;
	    return false;
	}
	
	var match = trees.match(
	    TREE('x+z/2+y+1+y'), TREE('b+a+b'),
	    {variables: {a: isNumber, b: true}}
	);

	expect(match).toEqual(false);

	match = trees.match(
	    TREE('x+z/2+y+1+y'), TREE('b+a+b'),
	    {variables: {a: isNumber, b: true},
	     allow_permutations: true}
	);

	expect(match).toEqual(false);

	match = trees.match(
	    TREE('x+z/2+y+1+y'), TREE('b+a+b'),
	    {variables: {a: isNumber, b: true},
	     allow_extended_match: true}
	);

	expect(match).toBeTruthy();
	expect(match["a"]).toEqual(1);
	expect(match["b"]).toEqual('y');
	expect(match["_skipped_before"]).toEqual(['x',TREE('z/2')]);
	
	match = trees.match(
	    TREE('x+z/2+1+y+y'), TREE('b+a+b'),
	    {variables: {a: isNumber, b: true},
	     allow_extended_match: true}
	);

	expect(match).toEqual(false);

	match = trees.match(
	    TREE('x+y+z/2+1+y'), TREE('b+a+b'),
	    {variables: {a: isNumber, b: true},
	     allow_extended_match: true}
	);

	expect(match).toEqual(false);

	match = trees.match(
	    TREE('x+y+1+y+z/2'), TREE('b+a+b'),
	    {variables: {a: isNumber, b: true},
	     allow_extended_match: true}
	);

	expect(match).toBeTruthy();
	expect(match["a"]).toEqual(1);
	expect(match["b"]).toEqual('y');
	expect(match["_skipped_before"]).toEqual(['x']);
	expect(match["_skipped"]).toEqual([TREE('z/2')]);
	
	match = trees.match(
	    TREE('x+y+z/2+1+y'), TREE('b+a+b'),
	    {variables: {a: isNumber, b: true},
	     allow_extended_match: true, allow_permutations: true}
	);

	expect(match).toBeTruthy();
	expect(match["a"]).toEqual(1);
	expect(match["b"]).toEqual('y');
	expect(match["_skipped"]).toEqual(['x',TREE('z/2')]);
	
    });

    it("trig extended match", function() {

	function isNumber(s) {
	    if (typeof s === 'number')
		return true;
	    if (Array.isArray(s) && s[0] === '-' && (typeof s[1] === 'number'))
		return true;
	    return false;
	}
	
	var match = trees.match(
	    TREE('x+z/2+cos(2y)^2+5+sin(2y)^2+y'),
	    TREE('cos(b)^2+a+sin(b)^2') ,
	    {variables: {a: isNumber, b: true}});

	expect(match).toEqual(false);
	
	match = trees.match(
	    TREE('x+z/2+cos(2y)^2+5+sin(2y)^2+y'),
	    TREE('cos(b)^2+a+sin(b)^2') ,
	    {variables: {a: isNumber, b: true},
	     allow_permutations: true});

	expect(match).toEqual(false);
	
	match = trees.match(
	    TREE('x+z/2+cos(2y)^2+5+sin(2y)^2+y'),
	    TREE('cos(b)^2+a+sin(b)^2') ,
	    {variables: {a: isNumber, b: true},
	     allow_extended_match: true});

	expect(match).toBeTruthy();
	expect(match['a']).toEqual(5);
	expect(match['b']).toEqual(TREE('2y'));
	expect(match['_skipped_before']).toEqual(['x', TREE('z/2')]);
	expect(match['_skipped']).toEqual(['y']);
	
	match = trees.match(
	    TREE('x+z/2+cos(2y)^2+5+sin(2y)^2+y'),
	    TREE('cos(b)^2+a+sin(b)^2') ,
	    {variables: {a: isNumber, b: true},
	     allow_extended_match: true, allow_permutations: true});

	expect(match).toBeTruthy();
	expect(match['a']).toEqual(5);
	expect(match['b']).toEqual(TREE('2y'));
	expect(match['_skipped']).toEqual(['x','y', TREE('z/2')]);

	match = trees.match(
	    TREE('z/2+cos(2y)^2+x+5+sin(2y)^2+y'),
	    TREE('cos(b)^2+a+sin(b)^2') ,
	    {variables: {a: isNumber, b: true},
	     allow_extended_match: true});

	expect(match).toEqual(false);
	
	match = trees.match(
	    TREE('x+z/2+cos(2y)^2+x+5+sin(2y)^2+y'),
	    TREE('cos(b)^2+a+sin(b)^2') ,
	    {variables: {a: isNumber, b: true},
	     allow_extended_match: true, allow_permutations: true});

	expect(match).toBeTruthy();
	expect(match['a']).toEqual(5);
	expect(match['b']).toEqual(TREE('2y'));
	expect(match['_skipped']).toEqual(['x','x', 'y', TREE('z/2')]);

	match = trees.match(
	    TREE('z/2+q*cos(2y)^2+x+5+q*sin(2y)^2+y'),
	    TREE('c*cos(b)^2+a+c*sin(b)^2') ,
	    {variables: {a: isNumber, b: true, c: true},
	     allow_extended_match: true, allow_permutations: true});

	expect(match).toBeTruthy();
	expect(match['a']).toEqual(5);
	expect(match['b']).toEqual(TREE('2y'));
	expect(match['c']).toEqual('q');
	expect(match['_skipped']).toEqual(['x', 'y', TREE('z/2')]);

	match = trees.match(
	    TREE('z/2+cos(2y)^2+x+5+sin(2y)^2+y'),
	    TREE('c*cos(b)^2+a+c*sin(b)^2') ,
	    {variables: {a: isNumber, b: true, c: true},
	     allow_extended_match: true, allow_permutations: true});

	expect(match).toEqual(false);

	match = trees.match(
	    TREE('z/2+cos(2y)^2+x+5+sin(2y)^2+y'),
	    TREE('c*cos(b)^2+a+c*sin(b)^2') ,
	    {variables: {a: isNumber, b: true, c: true},
	     allow_extended_match: true, allow_permutations: true,
	     allow_implicit_identities: ['c']});

	expect(match).toBeTruthy();
	expect(match['a']).toEqual(5);
	expect(match['b']).toEqual(TREE('2y'));
	expect(match['c']).toEqual(1);
	expect(match['_skipped']).toEqual(['x', 'y', TREE('z/2')]);

	match = trees.match(
	    TREE('z/2+q*cos(2y)^2+x+5+sin(2y)^2+y'),
	    TREE('c*cos(b)^2+a+c*sin(b)^2') ,
	    {variables: {a: isNumber, b: true, c: true},
	     allow_extended_match: true, allow_permutations: true,
	     allow_implicit_identities: ['c']});

	expect(match).toEqual(false);


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

	transformed = trees.applyAllTransformations(factored, transformations);

	expect(trees.equal(expanded, transformed)).toBeTruthy();
	
    });
       
    it("trig transformation", function () {

	var transformationTrig = [
	    TREE('cos(b)^2+sin(b)^2') , 1]

	var original = TREE('cos(2y)^2+1+sin(2y)^2+y/2');
	
	var result = trees.applyAllTransformations(
	    original, [transformationTrig]);

	expect(trees.equal(result,original)).toBeTruthy();
	
	transformationTrig = [
	    TREE('cos(b)^2+sin(b)^2') , 1,
	    {allow_extended_match: true, allow_permutations: true}]
	result = trees.applyAllTransformations(
	    original, [transformationTrig]);
	expect(result).toEqual(TREE('1+1+y/2'));

	transformationTrig = [
	    TREE('cos(b)^2+sin(b)^2') , 1,
	    {allow_extended_match: true, allow_permutations: true,
	     evaluate_numbers: true}]
	result = trees.applyAllTransformations(
	    original, [transformationTrig]);
	expect(result).toEqual(TREE('2+y/2'));

	original = TREE('q*cos(2y)^2+1+q*sin(2y)^2+y/2');
	result = trees.applyAllTransformations(
	    original, [transformationTrig]);
	expect(trees.equal(result,original)).toBeTruthy();

	transformationTrig = [
	    TREE('x*cos(b)^2+x*sin(b)^2') , 'x',
	    {allow_extended_match: true, allow_permutations: true,
	     evaluate_numbers: true}]
	result = trees.applyAllTransformations(
	    original, [transformationTrig]);
	expect(result).toEqual(TREE('1+q+y/2'));

	original = TREE('q*t*cos(2y)^2+1+q*t*sin(2y)^2+y/2');
	result = trees.applyAllTransformations(
	    original, [transformationTrig]);

	expect(result).toEqual(TREE('1+q*t+y/2'));

	original = TREE('cos(2y)^2+1+sin(2y)^2+y/2');
	transformationTrig = [
	    TREE('x*cos(b)^2+x*sin(b)^2') , 'x',
	    {allow_extended_match: true, allow_permutations: true,
	     evaluate_numbers: true}]
	result = trees.applyAllTransformations(
	    original, [transformationTrig]);
	expect(trees.equal(result,original)).toBeTruthy();

	transformationTrig = [
	    TREE('x*cos(b)^2+x*sin(b)^2') , 'x',
	    {allow_extended_match: true, allow_permutations: true,
	     evaluate_numbers: true, allow_implicit_identities: ['x']}]
	result = trees.applyAllTransformations(
	    original, [transformationTrig]);
	expect(result).toEqual(TREE('2+y/2'));

	original = TREE('cos(2y)^2+1+q*sin(2y)^2+y/2');
	result = trees.applyAllTransformations(
	    original, [transformationTrig]);
	expect(trees.equal(result,original)).toBeTruthy();
	
	original = TREE('q*t*cos(2y)^2+1+q*t*sin(2y)^2+y/2');
	result = trees.applyAllTransformations(
	    original, [transformationTrig]);
	expect(result).toEqual(TREE('1+q*t+y/2'));
	
	original = flatten.unflattenRight(original);
	result = trees.applyAllTransformations(
	    original, [transformationTrig]);
	expect(result).toEqual(TREE('1+q*t+y/2'));

	original = flatten.unflattenLeft(flatten.flatten(original));
	result = trees.applyAllTransformations(
	    original, [transformationTrig]);
	expect(result).toEqual(TREE('1+q*t+y/2'));

	
    });

    it("combine like terms", function () {

	function isNumber(s) {
	    if (typeof s === 'number')
		return true;
	    if (Array.isArray(s) && s[0] === '-' && (typeof s[1] === 'number'))
		return true;
	    return false;
	}

	var transformation = [
	    TREE('nx+mx') , TREE('(n+m)x'),
	    {allow_extended_match: true, evaluate_numbers: true,
	     allow_permutations: true, allow_implicit_identities: ['m', 'n'],
	     variables: {m: isNumber, n:isNumber, x: true}
	    },
	]

	var result = trees.applyAllTransformations(
	    me.fromText('3x+4y-2x').collapse_unary_minus().tree,
	    [transformation]);
	expect(trees.equal(result, TREE('x+4y'))).toBeTruthy();

	result = trees.applyAllTransformations(
	    TREE('3x+4y-x*2+qx-3x-y+x'), [transformation]);

	expect(trees.equal(result, TREE('-x+3y+qx'))).toBeTruthy();

    });

    it("make sure replaces with 0", function () {
	var transformation = [TREE('x^n*x^m') , TREE('x^(n+m)')]
	
	var result = trees.applyAllTransformations(
	    TREE('z^0z^5'), [transformation]);
	expect(result).toEqual(TREE('z^(0+5)'));
	
    });
    
});

