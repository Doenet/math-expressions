var me = require('../lib/math-expressions')
var _ = require('underscore');

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
    });

    it("vector3", function() {
	expect(me.from('(x,y,z)').to_intervals().tree).toEqual(
	    ['tuple', 'x', 'y', 'z']);
    });

    it("interval and vector3", function() {
	expect(me.from('(x,y)+(x,y,z)').to_intervals().tuples_to_vectors().tree)
	    .toEqual(['+',
		      ['interval', ['tuple', 'x', 'y'], ['tuple', false, false]],
		      ['vector', 'x', 'y', 'z']]);
    });


});

