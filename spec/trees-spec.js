var trees = require('../lib/trees.js')

var Expression = require ('../lib/math-expressions');
function TREE(s) {
    return Expression.fromText(s).tree;
}

describe("via trees", function() {
    it("cos x == cos x", function() {
	expect(trees.equal( TREE('cos x'), TREE('cos x') )).toBeTruthy();
    });

    it("cos x !== cos y", function() {
	expect(trees.equal( TREE('cos x'), TREE('cos y') )).toBeFalsy();
    });

    it("(1+2+3) deassociates to (1+(2+3))", function() {
	expect(trees.equal( trees.deassociate( ['+', 1, 2, 3], '+' ),
			    ['+', 1, ['+', 2, 3]])).toBeTruthy();
    });

    it("(1+(2+3)) associates to (1+2+3)", function() {
	expect(trees.equal( trees.associate( ['+', 1, ['+', 2, 3]], '+' ),
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

    it("x+y matches the pattern a+b", function() {
	expect(trees.match( TREE('x+y'), TREE('a+b') )).toEqual( {a:'x', b:'y'} );
    });

    it("x+y does not match the pattern a*b", function() {
	expect(trees.match( TREE('x+y'), TREE('a*b') )).toBeFalsy();
    });

    it("replacing x*y in x*y+z with y/x results in y/x+z", function() {
	var subtree = ['*', 'x', 'y'];
	var replacement = ['/', 'y', 'x'];
	var root = ['+', subtree, 'z'];

	expect(trees.equal( trees.replace( root, subtree, replacement ),
			    TREE('y/x + z') )).toBeTruthy();
    });    

    it("applying commutativity to a+b+c finds some permutations", function(done) {
	var tree = ['+', 'a', ['+', 'b', 'c']];
	var pattern = TREE('a+b');
	var replacement = TREE('b+a');

	var left = ['+', 'a', ['+', 'c', 'b']];
	var right = ['+', ['+', 'b', 'c'], 'a'];
	
	trees.applyTransformation( tree, pattern, replacement,
				   function(err, results) {
				       expect(results.length).toEqual(2);
				       
				       expect( ((trees.equal(results[0], left) && trees.equal(results[1], right))) ||
					       ((trees.equal(results[1], left) && trees.equal(results[0], right))) ).toBeTruthy();
				       done();
				   });
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

});

