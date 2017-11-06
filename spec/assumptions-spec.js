var me = require ('../lib/math-expressions');
var is_integer = require('../lib/assumptions/field.js').is_integer;
var is_real = require('../lib/assumptions/field.js').is_real;
var is_nonzero = require('../lib/assumptions/field.js').is_nonzero;


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
	expect(is_integer(me.fromText('2/1'))).toEqual(undefined);
    });

    it("functions are unknown", function () {
	expect(is_integer(me.fromText('abs(5)'))).toEqual(undefined);
	expect(is_integer(me.fromText('sin(0)'))).toEqual(undefined);

    });

    it("other operators are not integers", function () {
	expect(is_integer(me.fromText('(5,2)'))).toEqual(false);
	expect(is_integer(me.fromText('5=3'))).toEqual(false);

    });

    it("via assumptions", function () {
	me.assumptions = me.from('n ∈ Z').tree;

	expect(is_integer(me.fromText('n'))).toEqual(true);
	expect(is_integer(me.fromText('-n'))).toEqual(true);
	expect(is_integer(me.fromText('nm'))).toEqual(undefined);
	expect(is_integer(me.fromText('x'))).toEqual(undefined);
	expect(is_integer(me.fromText('n + 3'))).toEqual(true);
	expect(is_integer(me.fromText('5n'))).toEqual(true);
	expect(is_integer(me.fromText('5.5n'))).toEqual(undefined);
	expect(is_integer(me.fromText('n^3'))).toEqual(true);
	//expect(is_integer(me.fromText('n^(-3)'))).toEqual(undefined);

	me.assumptions = me.from('not(n ∈ Z)').tree;
	expect(is_integer(me.fromText('n'))).toEqual(false);
	me.assumptions = me.from('not(not(n ∈ Z))').tree;
	expect(is_integer(me.fromText('n'))).toEqual(true);

	
	me.assumptions = me.from('n ∈ R').tree;
	expect(is_integer(me.fromText('n + 3'))).toEqual(undefined);
	
	me.assumptions = me.from('n ∈ Z and m elementof Z').tree;
	expect(is_integer(me.fromText('n*m'))).toEqual(true);
	expect(is_integer(me.fromText('n/m'))).toEqual(undefined);
	expect(is_integer(me.fromText('n+m'))).toEqual(true);
	expect(is_integer(me.fromText('n-m'))).toEqual(true);
	expect(is_integer(me.fromText('n^m'))).toEqual(true);
	
	me.assumptions = me.from('n ∈ Z and m notelementof Z').tree;
	expect(is_integer(me.fromText('m'))).toEqual(false);
	expect(is_integer(me.fromText('m+1'))).toEqual(false);
	expect(is_integer(me.fromText('m+n+1'))).toEqual(false);
	expect(is_integer(me.fromText('mn'))).toEqual(undefined);

	me.assumptions = me.from('not(n ∉ Z) and not (m elementof Z)').tree;
	expect(is_integer(me.fromText('m'))).toEqual(false);
	expect(is_integer(me.fromText('m+1'))).toEqual(false);
	expect(is_integer(me.fromText('m+n+1'))).toEqual(false);
	expect(is_integer(me.fromText('mn'))).toEqual(undefined);

	// haven't implemented not (A or B) = not A or not B in assumptions
	me.assumptions = me.from('not(n ∉ Z or m elementof Z)').tree;
	// expect(is_integer(me.fromText('m'))).toEqual(false);
	// expect(is_integer(me.fromText('m+1'))).toEqual(false);
	// expect(is_integer(me.fromText('m+n+1'))).toEqual(false);
	// expect(is_integer(me.fromText('mn'))).toEqual(undefined);

	
	me.assumptions = me.from('n ∉ Z and m notelementof Z').tree;
	expect(is_integer(me.fromText('m+n+1'))).toEqual(undefined);
	expect(is_integer(me.fromText('mn'))).toEqual(undefined);

	
    });

    
    
});
