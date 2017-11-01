var astToMathTree = require('../lib/parser').ast.to.mathTree;
var mathTreeToAst = require('../lib/parser').mathTree.to.ast;
var _ = require('underscore');
var math = require('../lib/mathjs');


describe("math tree to ast", function() {
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
	'x^ab': ['^', 'x', 'ab'], // doesn't split symbols
	'x^a!':  ['^', 'x', ['apply', 'factorial', 'a']],
	'x*y*z': ['*','x','y','z'],
	'xyz': 'xyz', // doesn't split symbols
	'x*y*z*w': ['*','x','y','z','w'],
	'(x*y)*(z*w)': ['*','x','y','z','w'],
	'c*(a+b)': ['*', 'c', ['+', 'a', 'b']],
	'(a+b)*c': ['*', ['+', 'a', 'b'], 'c'],
	'abs(x)': ['apply', 'abs','x'],
	'a!': ['apply', 'factorial','a'],
	'theta': 'theta',
	'cos(theta)': ['apply', 'cos','theta'],
	'x!': ['apply', 'factorial','x'],
	'abs(sin(abs(x)))': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
	'abs(x+3==2)': ['apply', 'abs', ['=', ['+', 'x', 3], 2]],
	'x^y^z': ['^', 'x', ['^','y','z']],
	'x^(y^z)': ['^', 'x', ['^','y','z']],
	'(x^y)^z': ['^', ['^', 'x', 'y'],'z'],
	'x': 'x',
	'f': 'f',
	'f(x)': ['apply', 'f', 'x'],
	'f(x,y,z)': ['apply', 'f', ['tuple', 'x', 'y', 'z']],
	'f(x)^2': ['^', ['apply', 'f', 'x'],2],
	'x^2!': ['^', 'x', ['apply', 'factorial', 2]],
	'x^2!!': ['^', 'x', ['apply', 'factorial', ['apply', 'factorial', 2]]],
	'x==a': ['=', 'x', 'a'],
	'7 != 2': ['ne', 7, 2],
	'not x==y': ['=', ['not', 'x'], 'y'],  // different precendence than our parser!!!!
	'x>y': ['>', 'x','y'],
	'x>=y': ['ge', 'x','y'],
	'x<y': ['<', 'x','y'],
	'x<=y': ['le', 'x','y'],
	'A and B': ['and', 'A', 'B'],
	'A or B': ['or', 'A', 'B'],
	'A and B or C': ['or', ['and', 'A', 'B'], 'C'],
	'A or B and C': ['or', 'A', ['and', 'B', 'C']],
	'1.2E3': 1200,
	'1.2E+3': 1200,
	'3.1E-3': 0.0031,
	'1.2e-3': 0.0012,
    };

    _.each( _.keys(trees), function(string) {
	it("parses " + string, function() {
	    expect(mathTreeToAst(math.parse(string))).toEqual(trees[string]);
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
	'x^2(x-3)-z^3exp(2x+1)+x/(x-1)',
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
	'csc(x)',
	'csc(3)',		
	'log(x)',
	'log(3)',
	'log(exp(x))',
	'exp(x)',
	'x+y+z',	
	'sin(3x)',
	'sin(3x)^2',
	'sin(x)^2 + cos(x)^2',
	'sin(x)^2 / cos(x)^2',
	'sin(x+y+z)^2',
	'log(-x^2)',
	'abs(3)',
	'sin(abs(x))',
	'x^22',
	'x^ab',
	'x^y^z',
	'(x^y)^z',
	'f(x)',
	'f(x)^2',
	'(x-1-2)^2',
	'x==y',
	'x==y==z',
	'x>y',
	'x>=y',
	'x<y',
	'x<=y',
	'A==1 and B==2',
	'A or B',
	'(A and B) or C',
	'A and (B or C)',
	'not(A and B)',
	'(A and B) < C',
	'(not A) == B',
	'(A and B) > (C and D) > (E and F)',
	'(A and B) + (C and D)',
	'x/y/z/w',
	'x(x-1)/z',
	'(not A) or B',
	'A==1 or B==x/y',
    ];

    _.each( inputs, function(input) {
	it(input, function() {
	    expect(math.simplify(astToMathTree(mathTreeToAst(math.parse(input)))).equals(math.simplify(math.parse(input)))).toBeTruthy();
	});	
    });

    // Additional round trips to ast should not alter the strings at all
    _.each( inputs, function(input) {
	it(input, function() {
	    expect(math.simplify(astToMathTree(mathTreeToAst(astToMathTree(mathTreeToAst(math.parse(input)))))).equals(math.simplify(math.parse(input)))).toBeTruthy();
	});	
    });
});
