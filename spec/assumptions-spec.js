var me = require ('../lib/math-expressions');
var is_integer = require('../lib/assumptions/field.js').is_integer;
var is_real = require('../lib/assumptions/field.js').is_real;
var is_nonzero = require('../lib/assumptions/field.js').is_nonzero;
var is_nonnegative = require('../lib/assumptions/field.js').is_nonnegative;
var is_nonpositive = require('../lib/assumptions/field.js').is_nonpositive;
var is_positive = require('../lib/assumptions/field.js').is_positive;
var is_negative = require('../lib/assumptions/field.js').is_negative;
var trees = require('../lib/trees/basic');

describe("add and get assumptions", function () {

    it("single variable", function () {
	me.clear_assumptions();
	me.add_assumption(me.from('x>0'));
	expect(trees.equal(me.get_assumptions('x'),me.from('x>0').tree)).toBeTruthy();
	expect(trees.equal(me.assumptions.get_assumptions('x'),me.from('x>0').tree)).toBeTruthy();
	expect(trees.equal(me.get_assumptions(['x']),me.from('x>0').tree)).toBeTruthy();
	expect(trees.equal(me.assumptions.get_assumptions(['x']),me.from('x>0').tree)).toBeTruthy();

	me.clear_assumptions();
	expect(me.get_assumptions('x')).toEqual(undefined);
	expect(me.assumptions.get_assumptions('x')).toEqual(undefined);

	me.assumptions.add_assumption(me.from('x<=0'));
	expect(trees.equal(me.get_assumptions('x'),me.from('x<=0').tree)).toBeTruthy();
	expect(trees.equal(me.assumptions.get_assumptions('x'),me.from('x<=0').tree)).toBeTruthy();

	me.add_assumption(me.from('x > -2').tree);
	expect(trees.equal(me.get_assumptions('x'),me.from('x<=0 and x > -2').tree)).toBeTruthy();
	expect(trees.equal(me.assumptions.get_assumptions('x'),me.from('x<=0 and x > -2').tree)).toBeTruthy();

	me.clear_assumptions();
	
	me.assumptions.add_assumption(me.from('x != 0').tree);
	expect(trees.equal(me.get_assumptions('x'),me.from('x!=0').tree)).toBeTruthy();
	expect(trees.equal(me.assumptions.get_assumptions('x'),me.from('x !=0').tree)).toBeTruthy();

	me.clear_assumptions();

    });
	
    it("multiple variables", function () {

	me.clear_assumptions();
	me.add_assumption(me.from('x <=0'));
	expect(trees.equal(me.get_assumptions('x'),me.from('x<=0').tree)).toBeTruthy();
	
	expect(me.get_assumptions('y')).toEqual(undefined);

	me.add_assumption(me.from('x < y+1'));
	expect(trees.equal(me.get_assumptions('x'),me.from('x<=0 and x < y+1').tree)).toBeTruthy();
	expect(trees.equal(me.get_assumptions('y'),me.from('x < y+1').tree)).toBeTruthy();
	expect(me.get_assumptions('z')).toEqual(undefined);

	me.clear_assumptions();

	me.assumptions.add_assumption(me.from('a < b < c'));
	expect(trees.equal(me.get_assumptions('b'),me.from('a < b and b < c').tree)).toBeTruthy();
	expect(trees.equal(me.assumptions.get_assumptions('b'),me.from('a < b and b < c').tree)).toBeTruthy();

	// this doesn't work yet
	expect(trees.equal(me.get_assumptions('a'),me.from('a < b and a < c').tree)).toBeTruthy();

	me.clear_assumptions();
    });


    it("multiple inequalities", function () {

	me.clear_assumptions();
	me.add_assumption(me.from("a < b <=c"));
	expect(trees.equal(me.get_assumptions('b'),me.from('a < b and b <= c').tree)).toBeTruthy();
	me.clear_assumptions();
    });


    
    var element_interval_tests = [
	['x elementof (a,b)', 'x > a and x < b'],
	['x notelementof (a,b)', 'x <= a or x >= b'],
	['(a,b) containselement x', 'x > a and x < b'],
	['(a,b) notcontainselement x', 'x <= a or x >= b'],
	['x elementof (a,b]', 'x > a and x <= b'],
	['x notelementof (a,b]', 'x <= a or x > b'],
	['(a,b] containselement x', 'x > a and x <= b'],
	['(a,b] notcontainselement x', 'x <= a or x > b'],
	['x elementof [a,b]', 'x >= a and x <= b'],
	['x notelementof [a,b]', 'x < a or x > b'],
	['[a,b] containselement x', 'x >= a and x <= b'],
	['[a,b] notcontainselement x', 'x < a or x > b'],
	['x elementof [a,b)', 'x >= a and x < b'],
	['x notelementof [a,b)', 'x < a or x >= b'],
	['[a,b) containselement x', 'x >= a and x < b'],
	['[a,b) notcontainselement x', 'x < a or x >= b'],
    ];
    
    element_interval_tests.forEach(function(input) {
	it("element interval: " + input, function() {
	    me.clear_assumptions();
	    me.add_assumption(me.from(input[0]));
	    expect(trees.equal(me.get_assumptions('x'),me.from(input[1]).tree)).toBeTruthy();
	    me.clear_assumptions();
	});	
    });

    
    var interval_tests = [
	['(a,b) subset (c,d)', 'a >=c and b <= d'],
	['(a,b) notsubset (c,d)', 'a < c or b > d'],
	['(a,b) superset (c,d)', 'a <= c and b >= d'],
	['(a,b) notsuperset (c,d)', 'a > c or b < d'],
	['(a,b] subset (c,d)', 'a >=c and b < d'],
	['(a,b] notsubset (c,d)', 'a < c or b >= d'],
	['(a,b] superset (c,d)', 'a <= c and b >= d'],
	['(a,b] notsuperset (c,d)', 'a > c or b < d'],
	['[a,b] subset (c,d)', 'a >c and b < d'],
	['[a,b] notsubset (c,d)', 'a <= c or b >= d'],
	['[a,b] superset (c,d)', 'a <= c and b >= d'],
	['[a,b] notsuperset (c,d)', 'a > c or b < d'],
	['[a,b) subset (c,d)', 'a >c and b <= d'],
	['[a,b) notsubset (c,d)', 'a <= c or b > d'],
	['[a,b) superset (c,d)', 'a <= c and b >= d'],
	['[a,b) notsuperset (c,d)', 'a > c or b < d'],
	['(a,b) subset (c,d]', 'a >=c and b <= d'],
	['(a,b) notsubset (c,d]', 'a < c or b > d'],
	['(a,b) superset (c,d]', 'a <= c and b > d'],
	['(a,b) notsuperset (c,d]', 'a > c or b <= d'],
	['(a,b] subset (c,d]', 'a >=c and b <= d'],
	['(a,b] notsubset (c,d]', 'a < c or b > d'],
	['(a,b] superset (c,d]', 'a <= c and b >= d'],
	['(a,b] notsuperset (c,d]', 'a > c or b < d'],
	['[a,b] subset (c,d]', 'a >c and b <= d'],
	['[a,b] notsubset (c,d]', 'a <= c or b > d'],
	['[a,b] superset (c,d]', 'a <= c and b >= d'],
	['[a,b] notsuperset (c,d]', 'a > c or b < d'],
	['[a,b) subset (c,d]', 'a >c and b <= d'],
	['[a,b) notsubset (c,d]', 'a <= c or b > d'],
	['[a,b) superset (c,d]', 'a <= c and b > d'],
	['[a,b) notsuperset (c,d]', 'a > c or b <= d'],
	['(a,b) subset [c,d]', 'a >=c and b <= d'],
	['(a,b) notsubset [c,d]', 'a < c or b > d'],
	['(a,b) superset [c,d]', 'a < c and b > d'],
	['(a,b) notsuperset [c,d]', 'a >= c or b <= d'],
	['(a,b] subset [c,d]', 'a >=c and b <= d'],
	['(a,b] notsubset [c,d]', 'a < c or b > d'],
	['(a,b] superset [c,d]', 'a < c and b >= d'],
	['(a,b] notsuperset [c,d]', 'a >= c or b < d'],
	['[a,b] subset [c,d]', 'a >=c and b <= d'],
	['[a,b] notsubset [c,d]', 'a < c or b > d'],
	['[a,b] superset [c,d]', 'a <= c and b >= d'],
	['[a,b] notsuperset [c,d]', 'a > c or b < d'],
	['[a,b) subset [c,d]', 'a >=c and b <= d'],
	['[a,b) notsubset [c,d]', 'a < c or b > d'],
	['[a,b) superset [c,d]', 'a <= c and b > d'],
	['[a,b) notsuperset [c,d]', 'a > c or b <= d'],
	['(a,b) subset [c,d)', 'a >=c and b <= d'],
	['(a,b) notsubset [c,d)', 'a < c or b > d'],
	['(a,b) superset [c,d)', 'a < c and b >= d'],
	['(a,b) notsuperset [c,d)', 'a >= c or b < d'],
	['(a,b] subset [c,d)', 'a >=c and b < d'],
	['(a,b] notsubset [c,d)', 'a < c or b >= d'],
	['(a,b] superset [c,d)', 'a < c and b >= d'],
	['(a,b] notsuperset [c,d)', 'a >= c or b < d'],
	['[a,b] subset [c,d)', 'a >=c and b < d'],
	['[a,b] notsubset [c,d)', 'a < c or b >= d'],
	['[a,b] superset [c,d)', 'a <= c and b >= d'],
	['[a,b] notsuperset [c,d)', 'a > c or b < d'],
	['[a,b) subset [c,d)', 'a >=c and b <= d'],
	['[a,b) notsubset [c,d)', 'a < c or b > d'],
	['[a,b) superset [c,d)', 'a <= c and b >= d'],
	['[a,b) notsuperset [c,d)', 'a > c or b < d'],
    ];
    
    interval_tests.forEach(function(input) {
	it("interval containment: " + input, function() {
	    me.clear_assumptions();
	    me.add_assumption(me.from(input[0]));
	    expect(trees.equal(me.get_assumptions(['a', 'b']),me.from(input[1]).tree)).toBeTruthy();
	    me.clear_assumptions();
	});	
    });

    it("avoid redundancies", function () {

	me.clear_assumptions();
	me.add_assumption(me.from('x <=0'));
	expect(trees.equal(me.get_assumptions('x'),me.from('x<=0').tree)).toBeTruthy();

	me.add_assumption(me.from('x <=0'));
	expect(trees.equal(me.get_assumptions('x'),me.from('x<=0').tree)).toBeTruthy();
	
	me.add_assumption(me.from('-1 < x <=0'));
	expect(trees.equal(me.get_assumptions('x'),me.from('x<=0 and x>-1').tree)).toBeTruthy();
	
	me.clear_assumptions();

	// this doesn't work
	me.add_assumption(me.from('x < 0'));
	me.add_assumption(me.from('x <= 0'));
	expect(trees.equal(me.get_assumptions('x'),me.from('x<0').tree)).toBeTruthy();
	
	me.clear_assumptions();
	
    });


});


describe("is integer", function() {

    it("literal integers", function() {
	expect(is_integer(me.fromText('6'))).toEqual(true);
	expect(is_integer(me.fromText('6.2'))).toEqual(false);
	expect(is_integer(me.fromText('-5'))).toEqual(true);
	expect(is_integer(me.fromText('-5.3'))).toEqual(false);
	
    });

    it("operations on literals", function() {
	expect(is_integer(me.fromText('8+12'))).toEqual(true);
	expect(is_integer(me.fromText('13-52'))).toEqual(true);
	expect(is_integer(me.fromText('23*23'))).toEqual(true);
	expect(is_integer(me.fromText('58*(-32)'))).toEqual(true);
	expect(is_integer(me.fromText('2/1'))).toEqual(true);
	expect(is_integer(me.fromText('abs(-5)'))).toEqual(true);
	expect(is_integer(me.fromText('sin(0)'))).toEqual(true);
	expect(is_integer(me.fromText('sqrt(4)'))).toEqual(true);
	expect(is_integer(me.fromText('4^(1/2)'))).toEqual(true);
	expect(is_integer(me.fromText('sqrt(-4)'))).toEqual(false);
	
    });

    it("variables are unknown", function () {
	expect(is_integer(me.fromText('y'))).toEqual(undefined);
	expect(is_integer(me.fromText('x+y'))).toEqual(undefined);
	expect(is_integer(me.fromText('x-y'))).toEqual(undefined);
	expect(is_integer(me.fromText('x*y'))).toEqual(undefined);
	expect(is_integer(me.fromText('x/y'))).toEqual(undefined);
	expect(is_integer(me.fromText('x^y'))).toEqual(undefined);
	expect(is_integer(me.fromText('abs(x)'))).toEqual(undefined);
	expect(is_integer(me.fromText('sin(y)'))).toEqual(undefined);

    });

    it("other operators are not integers", function () {
	expect(is_integer(me.fromText('(5,2)').tuples_to_vectors())).toEqual(false);
	expect(is_integer(me.fromText('5=3'))).toEqual(false);

    });

    it("via assumptions", function () {

	me.clear_assumptions();
	me.add_assumption(me.from('n ∈ Z'));
	expect(is_integer(me.fromText('n'))).toEqual(true);
	expect(is_integer(me.fromText('-n'))).toEqual(true);
	expect(is_integer(me.fromText('nm'))).toEqual(undefined);
	expect(is_integer(me.fromText('x'))).toEqual(undefined);
	expect(is_integer(me.fromText('n + 3'))).toEqual(true);
	expect(is_integer(me.fromText('5n'))).toEqual(true);
	expect(is_integer(me.fromText('5.5n'))).toEqual(undefined);
	expect(is_integer(me.fromText('n^2'))).toEqual(true);
	expect(is_integer(me.fromText('n^3'))).toEqual(true);
	expect(is_integer(me.fromText('n^(-3)'))).toEqual(undefined);
	expect(is_integer(me.fromText('n^n'))).toEqual(undefined);

	me.clear_assumptions();
	me.add_assumption(me.from('n ∈ Z and n > 0'));
	expect(is_integer(me.fromText('n^n'))).toEqual(true);
	expect(is_integer(me.fromText('n^3'))).toEqual(true);

	me.clear_assumptions();
	me.add_assumption(me.from('n ∈ Z and n < 0'));
	expect(is_integer(me.fromText('n^3'))).toEqual(true);
	expect(is_integer(me.fromText('n^2'))).toEqual(true);

	me.clear_assumptions();
	me.add_assumption(me.from('not(n ∈ Z)'));
	expect(is_integer(me.fromText('n'))).toEqual(false);
	me.clear_assumptions();
	me.add_assumption(me.from('not(not(n ∈ Z))'));
	expect(is_integer(me.fromText('n'))).toEqual(true);

	me.clear_assumptions();
	me.add_assumption(me.from('n ∈ R'));
	expect(is_integer(me.fromText('n + 3'))).toEqual(undefined);
	
	me.clear_assumptions();
	me.add_assumption(me.from('n ∈ Z and m elementof Z'));
	expect(is_integer(me.fromText('n*m'))).toEqual(true);
	expect(is_integer(me.fromText('n/m'))).toEqual(undefined);
	expect(is_integer(me.fromText('n+m'))).toEqual(true);
	expect(is_integer(me.fromText('n-m'))).toEqual(true);
	expect(is_integer(me.fromText('n^m'))).toEqual(undefined);

	me.clear_assumptions();
	me.add_assumption(me.from('n ∈ Z and m elementof Z and m > 0'));
	expect(is_integer(me.fromText('n^m'))).toEqual(true);
	
	me.clear_assumptions();
	me.add_assumption(me.from('n ∈ Z and m notelementof Z'));
	expect(is_integer(me.fromText('m'))).toEqual(false);
	expect(is_integer(me.fromText('m+1'))).toEqual(false);
	expect(is_integer(me.fromText('m+n+1'))).toEqual(false);
	expect(is_integer(me.fromText('mn'))).toEqual(undefined);

	me.clear_assumptions();
	me.add_assumption(me.from('not(n ∉ Z) and not (m elementof Z)'));
	expect(is_integer(me.fromText('m'))).toEqual(false);
	expect(is_integer(me.fromText('m+1'))).toEqual(false);
	expect(is_integer(me.fromText('m+n+1'))).toEqual(false);
	expect(is_integer(me.fromText('mn'))).toEqual(undefined);

	me.clear_assumptions();
	me.add_assumption(me.from('not(n ∉ Z or m elementof Z)'));
	expect(is_integer(me.fromText('m'))).toEqual(false);
	expect(is_integer(me.fromText('m+1'))).toEqual(false);
	expect(is_integer(me.fromText('m+n+1'))).toEqual(false);
	expect(is_integer(me.fromText('mn'))).toEqual(undefined);

	
	me.clear_assumptions();
	me.add_assumption(me.from('n ∉ Z and m notelementof Z'));
	expect(is_integer(me.fromText('m+n+1'))).toEqual(undefined);
	expect(is_integer(me.fromText('mn'))).toEqual(undefined);

	me.clear_assumptions();
	
    });

});


describe("is positive/negative/zero/real", function() {

    var literals = [
	['3.2', true, false, false, true, true, true],
	['0', true, false, true, false, false, true],
	['-5.12', false, true, true, false, true, true],
	['8+12.3', true, false, false, true, true, true],
	['13-13', true, false, true, false, false, true],
	['13-13.7', false, true, true, false, true, true],
	['2.1*3.6', true, false, false, true, true, true],
	['21.3*(-31.2)', false, true, true, false, true, true],
	['21.3*(4-4)*(-31.2)', true, false, true, false, false, true],
	['2.2/3.5', true, false, false, true, true, true],
	['-2.2/3.5', false, true, true, false, true, true],
	['2.2/-3.5', false, true, true, false, true, true],
	['-2.2/-3.5', true, false, false, true, true, true],
	['-2.2/(5-5)', false, false, false, false, true, false],
	['(-6+6)/(3-5)', true, false, true, false, false, true],
	['(-6+6)/(5-5)', false, false, false, false, false, false],
	['abs(-5)', true, false, false, true, true, true],
	['sin(0)', true, false, true, false, false, true],
	['sqrt(-4)', false, false, false, false, true, false],
	['0^0', false, false, false, false, false, false, false],
    ]

    literals.forEach(function(input) {
	it("literals: " + input, function() {
	    me.clear_assumptions();
	    expect(is_nonnegative(me.fromText(input[0]))).toEqual(input[1]);
	    expect(is_negative(me.fromText(input[0]))).toEqual(input[2]);
	    expect(is_nonpositive(me.fromText(input[0]))).toEqual(input[3]);
	    expect(is_positive(me.fromText(input[0]))).toEqual(input[4]);
	    expect(is_nonzero(me.fromText(input[0]))).toEqual(input[5]);
	    expect(is_real(me.fromText(input[0]))).toEqual(input[6]);
	});
    });


    var variables = [
	['y', undefined, undefined, undefined, undefined, undefined, undefined],
	['x+y', undefined, undefined, undefined, undefined, undefined, undefined],
	['x-y', undefined, undefined, undefined, undefined, undefined, undefined],
	['x*y', undefined, undefined, undefined, undefined, undefined, undefined],
	['x/y', undefined, undefined, undefined, undefined, undefined, undefined],
	['x^y', undefined, undefined, undefined, undefined, undefined, undefined],
	['abs(x)', undefined, undefined, undefined, undefined, undefined, undefined],
	['sin(y)', undefined, undefined, undefined, undefined, undefined, undefined],
    ]
    
    variables.forEach(function(input) {
	it("variables undefined: " + input, function() {
	    me.clear_assumptions();
	    expect(is_nonnegative(me.fromText(input[0]))).toEqual(input[1]);
	    expect(is_negative(me.fromText(input[0]))).toEqual(input[2]);
	    expect(is_nonpositive(me.fromText(input[0]))).toEqual(input[3]);
	    expect(is_positive(me.fromText(input[0]))).toEqual(input[4]);
	    expect(is_nonzero(me.fromText(input[0]))).toEqual(input[5]);
	    expect(is_real(me.fromText(input[0]))).toEqual(input[6]);
	});
    });


    var operators = [
	['(5,2)', false, false, false, false, false, false],
	['5=3', false, false, false, false, false, false],
    ];

    operators.forEach(function(input) {
	it("operators: " + input, function() {
	    me.clear_assumptions();
	    expect(is_nonnegative(me.fromText(input[0]))).toEqual(input[1]);
	    expect(is_negative(me.fromText(input[0]))).toEqual(input[2]);
	    expect(is_nonpositive(me.fromText(input[0]))).toEqual(input[3]);
	    expect(is_positive(me.fromText(input[0]))).toEqual(input[4]);
	    expect(is_nonzero(me.fromText(input[0]))).toEqual(input[5]);
	    expect(is_real(me.fromText(input[0]))).toEqual(input[6]);
	});
    });


    var assumptions = [
	[undefined,
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x elementof R and x != 0',
	 undefined, undefined, undefined, undefined, true, true],
	['x != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x > 0',
	 true, false, false, true, true, true],
	['x >= 0',
	 true, false, undefined, undefined, undefined, true],
	['x < 0',
	 false, true, true, false, true, true],
	['x <= 0',
	 undefined, undefined, true, false, undefined, true],
	['x = 0',
	 true, false, true, false, false, true],
    ];	
	
    assumptions.forEach(function(input) {
	it("assumptions: " + input, function() {
	    me.clear_assumptions();
	    me.add_assumption(me.from(input[0]));
	    expect(is_nonnegative(me.fromText('x'))).toEqual(input[1]);
	    expect(is_negative(me.fromText('x'))).toEqual(input[2]);
	    expect(is_nonpositive(me.fromText('x'))).toEqual(input[3]);
	    expect(is_positive(me.fromText('x'))).toEqual(input[4]);
	    expect(is_nonzero(me.fromText('x'))).toEqual(input[5]);
	    expect(is_real(me.fromText('x'))).toEqual(input[6]);
	});
    });

    
    var assumptions_negation = [
	[undefined,
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x elementof R and x != 0',
	 undefined, undefined, undefined, undefined, true, true],
	['x != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x > 0',
	 false, true, true, false, true, true],
	['x >= 0',
	 undefined, undefined, true, false, undefined, true],
	['x < 0',
	 true, false, false, true, true, true],
	['x <= 0',
	 true, false, undefined, undefined, undefined, true],
	['x = 0',
	 true, false, true, false, false, true],
    ];	
	

    assumptions_negation.forEach(function(input) {
	it("assumptions: " + input, function() {
	    me.clear_assumptions();
	    me.add_assumption(me.from(input[0]));
	    expect(is_nonnegative(me.fromText('-x'))).toEqual(input[1]);
	    expect(is_negative(me.fromText('-x'))).toEqual(input[2]);
	    expect(is_nonpositive(me.fromText('-x'))).toEqual(input[3]);
	    expect(is_positive(me.fromText('-x'))).toEqual(input[4]);
	    expect(is_nonzero(me.fromText('-x'))).toEqual(input[5]);
	    expect(is_real(me.fromText('-x'))).toEqual(input[6]);
	});
    });


    it("subtract identical is zero", function() {
	me.clear_assumptions();
	expect(is_nonzero(me.fromText('x-x'))).toEqual(false);

    });

    var sum_tests=[

	[undefined,
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y < 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],

	['x != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y = 0',
	 undefined, undefined, undefined, undefined, true, undefined],

	['x > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x > 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x > 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x > 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x > 0 and y > 0',
	 true, false, false, true, true, true],
	['x > 0 and y >= 0',
	 true, false, false, true, true, true],
	['x > 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x > 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x > 0 and y = 0',
	 true, false, false, true, true, true],

	['x >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x >= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x >= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y > 0',
	 true, false, false, true, true, true],
	['x >= 0 and y >= 0',
	 true, false, undefined, undefined, undefined, true],
	['x >= 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x >= 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x >= 0 and y = 0',
	 true, false, undefined, undefined, undefined, true],

	['x < 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x < 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x < 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x < 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x < 0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x < 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x < 0 and y < 0',
	 false, true, true, false, true, true],
	['x < 0 and y <= 0',
	 false, true, true, false, true, true],
	['x < 0 and y = 0',
	 false, true, true, false, true, true],

	['x <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x <= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x <= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x <= 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x <= 0 and y < 0',
	 false, true, true, false, true, true],
	['x <= 0 and y <= 0',
	 undefined, undefined, true, false, undefined, true],
	['x <= 0 and y = 0',
	 undefined, undefined, true, false, undefined, true],

	['x = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x = 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x = 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true, true],
	['x = 0 and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x = 0 and y > 0',
	 true, false, false, true, true, true],
	['x = 0 and y >= 0',
	 true, false, undefined, undefined, undefined, true],
	['x = 0 and y < 0',
	 false, true, true, false, true, true],
	['x = 0 and y <= 0',
	 undefined, undefined, true, false, undefined, true],
	['x = 0 and y = 0',
	 true, false, true, false, false, true],
    ]
    
    
    sum_tests.forEach(function(input) {
	it("via assumptions -- sum: " + input, function() {
	    me.clear_assumptions();
	    me.add_assumption(me.from(input[0]));
	    expect(is_nonnegative(me.fromText('x+y'))).toEqual(input[1]);
	    expect(is_negative(me.fromText('x+y'))).toEqual(input[2]);
	    expect(is_nonpositive(me.fromText('x+y'))).toEqual(input[3]);
	    expect(is_positive(me.fromText('x+y'))).toEqual(input[4]);
	    expect(is_nonzero(me.fromText('x+y'))).toEqual(input[5]);
	    expect(is_real(me.fromText('x+y'))).toEqual(input[6]);
	    me.clear_assumptions();
	});
    });


    var triple_sum_tests=[
	['z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y elementof R and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y elementof R and y != 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y != 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y > 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y >= 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y < 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y <= 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y = 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],

	['x != 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y elementof R and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y elementof R and y != 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y != 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y > 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y >= 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y < 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y <= 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y = 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],

	['x > 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x > 0 and y elementof R and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x > 0 and y elementof R and y != 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x > 0 and y != 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x > 0 and y > 0 and z > 0',
	 true, false, false, true, true, true],
	['x > 0 and y >= 0 and z > 0',
	 true, false, false, true, true, true],
	['x > 0 and y < 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x > 0 and y <= 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x > 0 and y = 0 and z > 0',
	 true, false, false, true, true, true],

	['x >= 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y elementof R and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x >= 0 and y elementof R and y != 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x >= 0 and y != 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y > 0 and z > 0',
	 true, false, false, true, true, true],
	['x >= 0 and y >= 0 and z > 0',
	 true, false, false, true, true, true],
	['x >= 0 and y < 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x >= 0 and y <= 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x >= 0 and y = 0 and z > 0',
	 true, false, false, true, true, true],

	['x < 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x < 0 and y elementof R and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x < 0 and y elementof R and y != 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x < 0 and y != 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x < 0 and y > 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x < 0 and y >= 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x < 0 and y < 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x < 0 and y <= 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x < 0 and y = 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],

	['x <= 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y elementof R and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x <= 0 and y elementof R and y != 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x <= 0 and y != 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y > 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x <= 0 and y >= 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x <= 0 and y < 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x <= 0 and y <= 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x <= 0 and y = 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],

	['x = 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x = 0 and y elementof R and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x = 0 and y elementof R and y != 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x = 0 and y != 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x = 0 and y > 0 and z > 0',
	 true, false, false, true, true, true],
	['x = 0 and y >= 0 and z > 0',
	 true, false, false, true, true, true],
	['x = 0 and y < 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x = 0 and y <= 0 and z > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x = 0 and y = 0 and z > 0',
	 true, false, false, true, true, true],
    ]
    
    triple_sum_tests.forEach(function(input) {
	it("via assumptions -- sum: " + input, function() {
	    me.clear_assumptions();
	    me.add_assumption(me.from(input[0]));
	    expect(is_nonnegative(me.fromText('x+y+z'))).toEqual(input[1]);
	    expect(is_negative(me.fromText('x+y+z'))).toEqual(input[2]);
	    expect(is_nonpositive(me.fromText('x+y+z'))).toEqual(input[3]);
	    expect(is_positive(me.fromText('x+y+z'))).toEqual(input[4]);
	    expect(is_nonzero(me.fromText('x+y+z'))).toEqual(input[5]);
	    expect(is_real(me.fromText('x+y+z'))).toEqual(input[6]);
	    me.clear_assumptions();
	});
    });
    
    var subtraction_tests=[

	[undefined,
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y < 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],

	['x != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y = 0',
	 undefined, undefined, undefined, undefined, true, undefined],

	['x > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x > 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x > 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x > 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x > 0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x > 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x > 0 and y < 0',
	 true, false, false, true, true, true],
	['x > 0 and y <= 0',
	 true, false, false, true, true, true],
	['x > 0 and y = 0',
	 true, false, false, true, true, true],

	['x >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x >= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x >= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x >= 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x >= 0 and y < 0',
	 true, false, false, true, true, true],
	['x >= 0 and y <= 0',
	 true, false, undefined, undefined, undefined, true],
	['x >= 0 and y = 0',
	 true, false, undefined, undefined, undefined, true],

	['x < 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x < 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x < 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x < 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x < 0 and y > 0',
	 false, true, true, false, true, true],
	['x < 0 and y >= 0',
	 false, true, true, false, true, true],
	['x < 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x < 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x < 0 and y = 0',
	 false, true, true, false, true, true],

	['x <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x <= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x <= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y > 0',
	 false, true, true, false, true, true],
	['x <= 0 and y >= 0',
	 undefined, undefined, true, false, undefined, true],
	['x <= 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x <= 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x <= 0 and y = 0',
	 undefined, undefined, true, false, undefined, true],

	['x = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x = 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x = 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true, true],
	['x = 0 and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x = 0 and y > 0',
	 false, true, true, false, true, true],
	['x = 0 and y >= 0',
	 undefined, undefined, true, false, undefined, true],
	['x = 0 and y < 0',
	 true, false, false, true, true, true],
	['x = 0 and y <= 0',
	 true, false, undefined, undefined, undefined, true],
	['x = 0 and y = 0',
	 true, false, true, false, false, true],
    ]
    
    
    subtraction_tests.forEach(function(input) {
	it("via assumptions -- subtration: " + input, function() {
	    me.clear_assumptions();
	    me.add_assumption(me.from(input[0]));
	    expect(is_nonnegative(me.fromText('x-y'))).toEqual(input[1]);
	    expect(is_negative(me.fromText('x-y'))).toEqual(input[2]);
	    expect(is_nonpositive(me.fromText('x-y'))).toEqual(input[3]);
	    expect(is_positive(me.fromText('x-y'))).toEqual(input[4]);
	    expect(is_nonzero(me.fromText('x-y'))).toEqual(input[5]);
	    expect(is_real(me.fromText('x-y'))).toEqual(input[6]);
	    me.clear_assumptions();
	});
    });


    var product_tests=[
	
	[undefined,
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y element of R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y element of R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y < 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y = 0',
	 true, false, true, false, false, true],

	['x elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x elementof R and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and y > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x elementof R and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x elementof R and y < 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x elementof R and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x elementof R and y = 0',
	 true, false, true, false, false, true],


	['x != 0 and x elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and x elementof R and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x != 0 and x elementof R and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true, true],
	['x != 0 and x elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x != 0 and x elementof R and y > 0',
	 undefined, undefined, undefined, undefined, true, true],
	['x != 0 and x elementof R and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x != 0 and x elementof R and y < 0',
	 undefined, undefined, undefined, undefined, true, true],
	['x != 0 and x elementof R and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x != 0 and x elementof R and y = 0',
	 true, false, true, false, false, true],

	['x != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y = 0',
	 true, false, true, false, false, true],

	['x > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x > 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x > 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true, true],
	['x > 0 and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x > 0 and y > 0',
	 true, false, false, true, true, true],
	['x > 0 and y >= 0',
	 true, false, undefined, undefined, undefined, true],
	['x > 0 and y < 0',
	 false, true, true, false, true, true],
	['x > 0 and y <= 0',
	 undefined, undefined, true, false, undefined, true],
	['x > 0 and y = 0',
	 true, false, true, false, false, true],

	['x >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x >= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x >= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y > 0',
	 true, false, undefined, undefined, undefined, true],
	['x >= 0 and y >= 0',
	 true, false, undefined, undefined, undefined, true],
	['x >= 0 and y < 0',
	 undefined, undefined, true, false, undefined, true],
	['x >= 0 and y <= 0',
	 undefined, undefined, true, false, undefined, true],
	['x >= 0 and y = 0',
	 true, false, true, false, false, true],

	['x < 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x < 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x < 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true, true],
	['x < 0 and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x < 0 and y > 0',
	 false, true, true, false, true, true],
	['x < 0 and y >= 0',
	 undefined, undefined, true, false, undefined, true],
	['x < 0 and y < 0',
	 true, false, false, true, true, true],
	['x < 0 and y <= 0',
	 true, false, undefined, undefined, undefined, true],
	['x < 0 and y = 0',
	 true, false, true, false, false, true],

	['x <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x <= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x <= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y > 0',
	 undefined, undefined, true, false, undefined, true],
	['x <= 0 and y >= 0',
	 undefined, undefined, true, false, undefined, true],
	['x <= 0 and y < 0',
	 true, false, undefined, undefined, undefined, true],
	['x <= 0 and y <= 0',
	 true, false, undefined, undefined, undefined, true],
	['x <= 0 and y = 0',
	 true, false, true, false, false, true],

	['x = 0',
	 true, false, true, false, false, true],
	['x = 0 and y elementof R',
	 true, false, true, false, false, true],
	['x = 0 and y elementof R and y != 0',
	 true, false, true, false, false, true],
	['x = 0 and y != 0',
	 true, false, true, false, false, true],
	['x = 0 and y > 0',
	 true, false, true, false, false, true],
	['x = 0 and y >= 0',
	 true, false, true, false, false, true],
	['x = 0 and y < 0',
	 true, false, true, false, false, true],
	['x = 0 and y <= 0',
	 true, false, true, false, false, true],
	['x = 0 and y = 0',
	 true, false, true, false, false, true],
    ]
    
    
    product_tests.forEach(function(input) {
	it("via assumptions -- product: " + input, function() {
	    me.clear_assumptions();
	    me.add_assumption(me.from(input[0]));
	    expect(is_nonnegative(me.fromText('xy'))).toEqual(input[1]);
	    expect(is_negative(me.fromText('xy'))).toEqual(input[2]);
	    expect(is_nonpositive(me.fromText('xy'))).toEqual(input[3]);
	    expect(is_positive(me.fromText('xy'))).toEqual(input[4]);
	    expect(is_nonzero(me.fromText('xy'))).toEqual(input[5]);
	    expect(is_real(me.fromText('xy'))).toEqual(input[6]);
	    me.clear_assumptions();
	});
    });

    var quotient_tests=[
	
	[undefined,
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y < 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	
	['x elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and y > 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x elementof R and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and y < 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x elementof R and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and y = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	
	['x elementof R and x != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true, true],
	['x elementof R and x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x elementof R and x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, true, true],
	['x elementof R and x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, true, true],
	['x elementof R and x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and x != 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	
	['x != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x != 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	
	['x > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x > 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x > 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true, true],
	['x > 0 and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x > 0 and y > 0',
	 true, false, false, true, true, true],
	['x > 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x > 0 and y < 0',
	 false, true, true, false, true, true],
	['x > 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x > 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	
	['x >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x >= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y > 0',
	 true, false, undefined, undefined, undefined, true],
	['x >= 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y < 0',
	 undefined, undefined, true, false, undefined, true],
	['x >= 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	
	['x < 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x < 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x < 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true, true],
	['x < 0 and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x < 0 and y > 0',
	 false, true, true, false, true, true],
	['x < 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x < 0 and y < 0',
	 true, false, false, true, true, true],
	['x < 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x < 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	
	['x <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, true],
	['x <= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y > 0',
	 undefined, undefined, true, false, undefined, true],
	['x <= 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y < 0',
	 true, false, undefined, undefined, undefined, true],
	['x <= 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	
	['x = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x = 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x = 0 and y elementof R and y != 0',
	 true, false, true, false, false, true],
	['x = 0 and y != 0',
	 true, false, true, false, false, true],
	['x = 0 and y > 0',
	 true, false, true, false, false, true],
	['x = 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x = 0 and y < 0',
	 true, false, true, false, false, true],
	['x = 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x = 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
    ]
    
    
    quotient_tests.forEach(function(input) {
	it("via assumptions -- quotient: " + input, function() {
	    me.clear_assumptions();
	    me.add_assumption(me.from(input[0]));
	    expect(is_nonnegative(me.fromText('x/y'))).toEqual(input[1]);
	    expect(is_negative(me.fromText('x/y'))).toEqual(input[2]);
	    expect(is_nonpositive(me.fromText('x/y'))).toEqual(input[3]);
	    expect(is_positive(me.fromText('x/y'))).toEqual(input[4]);
	    expect(is_nonzero(me.fromText('x/y'))).toEqual(input[5]);
	    expect(is_real(me.fromText('x/y'))).toEqual(input[6]);
	    me.clear_assumptions();
	});
    });


    var power_tests=[
	
	[undefined,
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y < 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['y = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],

	['x elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and y > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and y < 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R and y = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],

	['x elementof R and x != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x elementof R and x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x elementof R and x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x elementof R and x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x elementof R and x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x elementof R and x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x elementof R and x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x elementof R and x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x elementof R and x != 0 and y = 0',
	 true, false, false, true, true, true],

	['x != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x != 0 and y = 0',
	 true, false, false, true, true, true],

	['x > 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x > 0 and y elementof R',
	 true, false, false, true, true, true],
	['x > 0 and y elementof R and y != 0',
	 true, false, false, true, true, true],
	['x > 0 and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x > 0 and y > 0',
	 true, false, false, true, true, true],
	['x > 0 and y >= 0',
	 true, false, false, true, true, true],
	['x > 0 and y < 0',
	 true, false, false, true, true, true],
	['x > 0 and y <= 0',
	 true, false, false, true, true, true],
	['x > 0 and y = 0',
	 true, false, false, true, true, true],

	['x >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y > 0',
	 true, false, undefined, undefined, undefined, true],
	['x >= 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x >= 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],

	['x < 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x < 0 and y elementof R',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x < 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x < 0 and y != 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x < 0 and y > 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x < 0 and y >= 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x < 0 and y < 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x < 0 and y <= 0',
	 undefined, undefined, undefined, undefined, true, undefined],
	['x < 0 and y = 0',
	 true, false, false, true, true, true],

	['x <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x <= 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],

	['x = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x = 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x = 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x = 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x = 0 and y > 0',
	 true, false, true, false, false, true],
	['x = 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x = 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x = 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
	['x = 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined, undefined],
    ]
    
    
    power_tests.forEach(function(input) {
	it("via assumptions -- power: " + input, function() {
	    me.clear_assumptions();
	    me.add_assumption(me.from(input[0]));
	    expect(is_nonnegative(me.fromText('x^y'))).toEqual(input[1]);
	    expect(is_negative(me.fromText('x^y'))).toEqual(input[2]);
	    expect(is_nonpositive(me.fromText('x^y'))).toEqual(input[3]);
	    expect(is_positive(me.fromText('x^y'))).toEqual(input[4]);
	    expect(is_nonzero(me.fromText('x^y'))).toEqual(input[5]);
	    expect(is_real(me.fromText('x^y'))).toEqual(input[6]);
	    me.clear_assumptions();
	});
    });


    it("integer implies real", function () {
	me.clear_assumptions();
	me.add_assumption(me.from('y elementof Z'));
	expect(is_integer(me.from('y'))).toEqual(true);
	expect(is_real(me.from('y'))).toEqual(true);
	me.clear_assumptions();
	me.add_assumption(me.from('y notelementof Z'));
	expect(is_integer(me.from('y'))).toEqual(false);
	expect(is_real(me.from('y'))).toEqual(undefined);
	me.clear_assumptions();
	me.add_assumption(me.from('not(y elementof Z)'));
	expect(is_integer(me.from('y'))).toEqual(false);
	expect(is_real(me.from('y'))).toEqual(undefined);
	me.clear_assumptions();
	me.add_assumption(me.from('not(y notelementof Z)'));
	expect(is_integer(me.from('y'))).toEqual(true);
	expect(is_real(me.from('y'))).toEqual(true);
	me.clear_assumptions();
	
    });

    it("recursive assumptions", function() {
	me.clear_assumptions();
	me.add_assumption(me.from("x=x"));
	expect(is_real(me.from('x'))).toEqual(undefined);
	me.clear_assumptions();
	me.add_assumption(me.from("x=y=x"));
	expect(is_real(me.from('x'))).toEqual(undefined);
	me.clear_assumptions();
	me.add_assumption(me.from("y elementof R"));
	me.add_assumption(me.from("x=y"));
	expect(is_real(me.from('x'))).toEqual(true);
	me.clear_assumptions();
    });
    
    // recursive assumptions
    // negate assumptions (not fully implemented)
    // equal, ne, less than, etc., to variable that has assumptions
    // (mixed with negate)
    // (negative)^integer, (negative)^(even integer)
    // integer/not integer means real, even with negate

    
});

