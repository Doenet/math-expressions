var me = require('../lib/math-expressions')
var _ = require('underscore');

describe("normalize function names", function() {

    var trees = {
	'ln(x)': ['apply', 'log', 'x'],
	'e^x': ['apply', 'exp', 'x'],
	'arccsc(x)': ['apply', 'acsc', 'x'],
	'arctan^2(x)': ['apply', ['^', 'atan', 2], 'x'],
	'1+cosec(3*x)': ['+', 1, ['apply', 'csc', ['*', 3, 'x']]],
	'1-e^(x/y)': ['+', 1, ['-', ['apply', 'exp', ['/', 'x', 'y']]]],
	'5/sqrt(2y)': ['/', 5, ['^', ['*', 2, 'y'], 0.5]],
	'ln(e^x)': ['apply', 'log', ['apply', 'exp', 'x']],
	'e^(ln(x))': ['apply', 'exp', ['apply', 'log', 'x']],
	'sqrt(sqrt(x))': ['^', ['^', 'x', 0.5], 0.5],
	
    }
    
  _.each( _.keys(trees), function(string) {
	it(string, function() {
	    expect(me.from(string).normalize_function_names().tree)
		.toEqual(trees[string]);
	});	
    });    

});


describe("normalize applied functions", function() {
    it("derivative inside", function() {
	expect(me.from('f\'(x)').tree).toEqual(['apply', ['prime', 'f'], 'x']);
    });

    it("derivative outside", function() {
	expect(me.from('f(x)\'').tree).toEqual(['prime', ['apply', 'f', 'x']]);
    });

    it("derivative normalized outside", function() {
	expect(me.from('f\'(x)').normalize_applied_functions()).toEqual(
	    me.from('f(x)\''));
    });

    it("derivative normalized outside b", function() {
	expect(me.normalize_applied_functions(me.from('f\'(x)'))).toEqual(
	    me.from('f(x)\''));
    });

    it("exponent inside", function() {
	expect(me.from('f^2(x)').tree).toEqual(['apply', ['^', 'f', 2], 'x']);
    });

    it("exponent outside", function() {
	expect(me.from('f(x)^2').tree).toEqual(['^', ['apply', 'f', 'x'], 2]);
    });

    it("exponent normalized outside", function() {
	expect(me.from('f^2(x)').normalize_applied_functions()).toEqual(
	    me.from('f(x)^2'));
    });

    it("exponent normalized outside b", function() {
	expect(me.normalize_applied_functions(me.from('f^2(x)'))).toEqual(
	    me.from('f(x)^2'));
    });

    it("derivative exponent inside", function() {
	expect(me.from('f\'^2(x)').tree).toEqual(['apply', ['^', ['prime', 'f'], 2], 'x']);
    });

    it("derivative exponent outside", function() {
	expect(me.from('f(x)\'^2').tree).toEqual(['^', ['prime', ['apply', 'f', 'x']], 2]);
    });

    it("derivative exponent normalized outside", function() {
	expect(me.from('f\'^2(x)').normalize_applied_functions()).toEqual(
	    me.from('f(x)\'^2'));
    });

    it("derivative exponent normalized outside b", function() {
	expect(me.normalize_applied_functions(me.from('f\'^2(x)'))).toEqual(
	    me.from('f(x)\'^2'));
    });
});

describe("normalize tuples", function() {
    it("tuple", function() {
	expect(me.from('(x,y)').tree).toEqual(['tuple', 'x', 'y']);
    });

    it("vector", function() {
	expect(me.from('(x,y)').tuples_to_vectors().tree).toEqual(
	    ['vector', 'x', 'y']);
    });

    it("array", function() {
	expect(me.from('[x,y]').tree).toEqual(['array', 'x', 'y']);
    });

    it("open interval", function() {
	expect(me.from('(x,y)').to_intervals().tree).toEqual(
	    ['interval', ['tuple', 'x', 'y'], ['tuple', false, false]]);
    });

    it("closed interval", function() {
	expect(me.from('[x,y]').to_intervals().tree).toEqual(
	    ['interval', ['tuple', 'x', 'y'], ['tuple', true, true]]);
    });
    
    it("vector3", function() {
	expect(me.from('(x,y,z)').tuples_to_vectors().tree).toEqual(
	    ['vector', 'x', 'y', 'z']);
	expect(me.from('(x,y,z)').to_intervals().tree).toEqual(
	    ['tuple', 'x', 'y', 'z']);
    });

    it("interval and vector3", function() {
	expect(me.from('(x,y)+(x,y,z)').to_intervals().tuples_to_vectors().tree)
	    .toEqual(['+',
		      ['interval', ['tuple', 'x', 'y'], ['tuple', false, false]],
		      ['vector', 'x', 'y', 'z']]);
    });

    it("function", function() {
	expect(me.from('f(x,y)').tuples_to_vectors().tree).toEqual(
	    ['apply', 'f', ['tuple', 'x', 'y']]);
	expect(me.from('f(x,y)').to_intervals().tree).toEqual(
	    ['apply', 'f', ['tuple', 'x', 'y']]);
    });
    
});


describe("default order", function () {

    it("order terms", function () {
	expect(me.from("1-3+x-z").default_order().tree).toEqual(
	    me.from("-z+1+x-3").default_order().tree);
    });

    it("order factors", function () {
	expect(me.from("ajsdz").default_order().tree).toEqual(
	    me.from("sazdj").default_order().tree);
    });

    it("order equality", function () {
	expect(me.from("xy=ab").default_order().tree).toEqual(
	    me.from("ab=xy").default_order().tree);
	expect(me.from("y=xy=x+y").default_order().tree).toEqual(
	    me.from("y+x=yx=y").default_order().tree);
    });

    it("normalizes negatives in factors", function () {
	var expr1 = me.from([ '+', 1, [ '*', [ '-', 'x' ], 'y' ] ]);
	var expr2 = me.from([ '+', 1, [ '-', [ '*', 'x', 'y' ] ] ]);
	expect(expr1.default_order().tree).toEqual(expr2.default_order().tree);
    });

    it("removes multiple negatives", function () {
	expect(me.from("3--x").default_order().tree).toEqual(
	    me.from("3+x").default_order().tree);
	expect(me.from("3---x").default_order().tree).toEqual(
	    me.from("3-x").default_order().tree);
    });

    it("normalize negative combination", function () {
	expect(me.from('5+x(-3)').default_order().tree).toEqual(
	    me.from("5-3x").default_order().tree);
	expect(me.from('5-x(-3)').default_order().tree).toEqual(
	    me.from("5+3x").default_order().tree);
	expect(me.from('5-x(--3)').default_order().tree).toEqual(
	    me.from("5-3x").default_order().tree);
	expect(me.from('5--x(-3)').default_order().tree).toEqual(
	    me.from("5-3x").default_order().tree);
	expect(me.from('5--x(--3)').default_order().tree).toEqual(
	    me.from("5+3x").default_order().tree);
    });
    
    it("order expression", function () {
	expect(me.from("(x-2y)*(sin(yz)-3+u)v-3/z").default_order().tree).toEqual(
	    me.from("-3/z+(u-3+sin(zy))(-y*2+x)v").default_order().tree);
    });
    

    it("order equalities and inequalities", function () {
	expect(me.from("abc=x+y=-1/y").default_order().tree).toEqual(
	    me.from("y+x=-1/y=cab").default_order().tree);

	expect(me.from("8/sin(xy) != e^(b-a)").default_order().tree).toEqual(
	    me.from("e^(-a+b) != 8/sin(yx)").default_order().tree);
	
    });

    it("order logicals", function () {
	expect(me.from("(A or B) and (C or D)").default_order().tree).toEqual(
	    me.from("(D or C) and (B or A)").default_order().tree);

    });

    it("order set operations", function () {
	expect(me.from("(A union B) intersect (C union D)").default_order().tree).toEqual(
	    me.from("(D union C) intersect (B union A)").default_order().tree);

    });

    it("order inequalities", function () {
	expect(me.from("a >= b or b < d").default_order().tree).toEqual(
	    me.from("d > b or b <= a").default_order().tree);

	expect(me.from("a >= b >= c > d").default_order().tree).toEqual(
	    me.from("d < c <=b <= a").default_order().tree);
	
    });

    it("order containment", function () {
	expect(me.from("a elementof A and b notelementof B").default_order().tree).toEqual(
	    me.from("B notcontainselement b and A containselement a").default_order().tree);

	expect(me.from("A superset B or C notsubset D").default_order().tree).toEqual(
	    me.from("D notsuperset C or B subset A").default_order().tree);
	
    });

    
});
