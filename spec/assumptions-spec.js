var me = require ('../lib/math-expressions');
var is_integer = require('../lib/assumptions/field.js').is_integer;
var is_real = require('../lib/assumptions/field.js').is_real;
var is_nonzero = require('../lib/assumptions/field.js').is_nonzero;
var is_nonnegative = require('../lib/assumptions/field.js').is_nonnegative;
var is_nonpositive = require('../lib/assumptions/field.js').is_nonpositive;
var is_positive = require('../lib/assumptions/field.js').is_positive;
var is_negative = require('../lib/assumptions/field.js').is_negative;


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
	expect(is_integer(me.fromText('abs(x)'))).toEqual(undefined);
	expect(is_integer(me.fromText('sin(y)'))).toEqual(undefined);

    });

    it("other operators are not integers", function () {
	expect(is_integer(me.fromText('(5,2)').tuples_to_vectors())).toEqual(false);
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
	expect(is_integer(me.fromText('n^2'))).toEqual(true);
	expect(is_integer(me.fromText('n^3'))).toEqual(true);
	expect(is_integer(me.fromText('n^(-3)'))).toEqual(undefined);
	expect(is_integer(me.fromText('n^n'))).toEqual(undefined);

	me.assumptions = me.from('n ∈ Z and n > 0').tree;
	expect(is_integer(me.fromText('n^n'))).toEqual(true);
	expect(is_integer(me.fromText('n^3'))).toEqual(true);

	me.assumptions = me.from('n ∈ Z and n < 0').tree;
	expect(is_integer(me.fromText('n^3'))).toEqual(true);
	expect(is_integer(me.fromText('n^2'))).toEqual(true);

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
	expect(is_integer(me.fromText('n^m'))).toEqual(undefined);

	me.assumptions = me.from('n ∈ Z and m elementof Z and m > 0').tree;
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

	me.assumptions = me.from('not(n ∉ Z or m elementof Z)').simplify().tree;
	expect(is_integer(me.fromText('m'))).toEqual(false);
	expect(is_integer(me.fromText('m+1'))).toEqual(false);
	expect(is_integer(me.fromText('m+n+1'))).toEqual(false);
	expect(is_integer(me.fromText('mn'))).toEqual(undefined);

	
	me.assumptions = me.from('n ∉ Z and m notelementof Z').tree;
	expect(is_integer(me.fromText('m+n+1'))).toEqual(undefined);
	expect(is_integer(me.fromText('mn'))).toEqual(undefined);

	
    });

});


describe("is positive/negative/zero", function() {

    it("literals", function () {
	expect(is_nonnegative(me.fromText('3.2'))).toEqual(true);
	expect(is_nonnegative(me.fromText('0'))).toEqual(true);
	expect(is_nonnegative(me.fromText('-5.12'))).toEqual(false);
	expect(is_positive(me.fromText('3.2'))).toEqual(true);
	expect(is_positive(me.fromText('0'))).toEqual(false);
	expect(is_positive(me.fromText('-5.12'))).toEqual(false);
	expect(is_negative(me.fromText('3.2'))).toEqual(false);
	expect(is_negative(me.fromText('0'))).toEqual(false);
	expect(is_negative(me.fromText('-5.12'))).toEqual(true);
	expect(is_nonpositive(me.fromText('3.2'))).toEqual(false);
	expect(is_nonpositive(me.fromText('0'))).toEqual(true);
	expect(is_nonpositive(me.fromText('-5.12'))).toEqual(true);
	expect(is_nonzero(me.fromText('3.2'))).toEqual(true);
	expect(is_nonzero(me.fromText('0'))).toEqual(false);
	expect(is_nonzero(me.fromText('-5.12'))).toEqual(true);
	
    });

    it("operations on literals", function() {
	expect(is_nonnegative(me.fromText('8+12.3'))).toEqual(true);
	expect(is_nonnegative(me.fromText('13-13'))).toEqual(true);
	expect(is_nonnegative(me.fromText('13-13.7'))).toEqual(false);
	expect(is_nonnegative(me.fromText('2.1*3.6'))).toEqual(true);
	expect(is_nonnegative(me.fromText('21.3*(-31.2)'))).toEqual(false);
	expect(is_nonnegative(me.fromText('21.3*(4-4)*(-31.2)'))).toEqual(true);
	expect(is_nonnegative(me.fromText('2.2/3.5'))).toEqual(true);
	expect(is_nonnegative(me.fromText('-2.2/3.5'))).toEqual(false);
	expect(is_nonnegative(me.fromText('2.2/-3.5'))).toEqual(false);
	expect(is_nonnegative(me.fromText('-2.2/-3.5'))).toEqual(true);
	expect(is_nonnegative(me.fromText('-2.2/(5-5)'))).toEqual(false);
	expect(is_nonnegative(me.fromText('(-6+6)/(3-5)'))).toEqual(true);
	expect(is_nonnegative(me.fromText('(-6+6)/(5-5)'))).toEqual(false);
	expect(is_nonnegative(me.fromText('abs(-5)'))).toEqual(true);
	expect(is_nonnegative(me.fromText('sin(0)'))).toEqual(true);
	expect(is_nonnegative(me.fromText('sqrt(-4)'))).toEqual(false);

	expect(is_negative(me.fromText('8+12.3'))).toEqual(false);
	expect(is_negative(me.fromText('13-13'))).toEqual(false);
	expect(is_negative(me.fromText('13-13.7'))).toEqual(true);
	expect(is_negative(me.fromText('2.1*3.6'))).toEqual(false);
	expect(is_negative(me.fromText('21.3*(-31.2)'))).toEqual(true);
	expect(is_negative(me.fromText('21.3*(4-4)*(-31.2)'))).toEqual(false);
	expect(is_negative(me.fromText('2.2/3.5'))).toEqual(false);
	expect(is_negative(me.fromText('-2.2/3.5'))).toEqual(true);
	expect(is_negative(me.fromText('2.2/-3.5'))).toEqual(true);
	expect(is_negative(me.fromText('-2.2/-3.5'))).toEqual(false);
	expect(is_negative(me.fromText('-2.2/(5-5)'))).toEqual(false);
	expect(is_negative(me.fromText('(-6+6)/(3-5)'))).toEqual(false);
	expect(is_negative(me.fromText('(-6+6)/(5-5)'))).toEqual(false);
	expect(is_negative(me.fromText('abs(-5)'))).toEqual(false);
	expect(is_negative(me.fromText('sin(0)'))).toEqual(false);
	expect(is_negative(me.fromText('sqrt(-4)'))).toEqual(false);

	expect(is_nonpositive(me.fromText('8+12.3'))).toEqual(false);
	expect(is_nonpositive(me.fromText('13-13'))).toEqual(true);
	expect(is_nonpositive(me.fromText('13-13.7'))).toEqual(true);
	expect(is_nonpositive(me.fromText('2.1*3.6'))).toEqual(false);
	expect(is_nonpositive(me.fromText('21.3*(-31.2)'))).toEqual(true);
	expect(is_nonpositive(me.fromText('21.3*(4-4)*(-31.2)'))).toEqual(true);
	expect(is_nonpositive(me.fromText('2.2/3.5'))).toEqual(false);
	expect(is_nonpositive(me.fromText('-2.2/3.5'))).toEqual(true);
	expect(is_nonpositive(me.fromText('2.2/-3.5'))).toEqual(true);
	expect(is_nonpositive(me.fromText('-2.2/-3.5'))).toEqual(false);
	expect(is_nonpositive(me.fromText('-2.2/(5-5)'))).toEqual(false);
	expect(is_nonpositive(me.fromText('(-6+6)/(3-5)'))).toEqual(true);
	expect(is_nonpositive(me.fromText('(-6+6)/(5-5)'))).toEqual(false);
	expect(is_nonpositive(me.fromText('abs(-5)'))).toEqual(false);
	expect(is_nonpositive(me.fromText('sin(0)'))).toEqual(true);
	expect(is_nonpositive(me.fromText('sqrt(-4)'))).toEqual(false);

	expect(is_positive(me.fromText('8+12.3'))).toEqual(true);
	expect(is_positive(me.fromText('13-13'))).toEqual(false);
	expect(is_positive(me.fromText('13-13.7'))).toEqual(false);
	expect(is_positive(me.fromText('2.1*3.6'))).toEqual(true);
	expect(is_positive(me.fromText('21.3*(-31.2)'))).toEqual(false);
	expect(is_positive(me.fromText('21.3*(4-4)*(-31.2)'))).toEqual(false);
	expect(is_positive(me.fromText('2.2/3.5'))).toEqual(true);
	expect(is_positive(me.fromText('-2.2/3.5'))).toEqual(false);
	expect(is_positive(me.fromText('2.2/-3.5'))).toEqual(false);
	expect(is_positive(me.fromText('-2.2/-3.5'))).toEqual(true);
	expect(is_positive(me.fromText('-2.2/(5-5)'))).toEqual(false);
	expect(is_positive(me.fromText('(-6+6)/(3-5)'))).toEqual(false);
	expect(is_positive(me.fromText('(-6+6)/(5-5)'))).toEqual(false);
	expect(is_positive(me.fromText('abs(-5)'))).toEqual(true);
	expect(is_positive(me.fromText('sin(0)'))).toEqual(false);
	expect(is_positive(me.fromText('sqrt(-4)'))).toEqual(false);

	expect(is_nonzero(me.fromText('8+12.3'))).toEqual(true);
	expect(is_nonzero(me.fromText('13-13'))).toEqual(false);
	expect(is_nonzero(me.fromText('13-13.7'))).toEqual(true);
	expect(is_nonzero(me.fromText('2.1*3.6'))).toEqual(true);
	expect(is_nonzero(me.fromText('21.3*(-31.2)'))).toEqual(true);
	expect(is_nonzero(me.fromText('21.3*(4-4)*(-31.2)'))).toEqual(false);
	expect(is_nonzero(me.fromText('2.2/3.5'))).toEqual(true);
	expect(is_nonzero(me.fromText('-2.2/3.5'))).toEqual(true);
	expect(is_nonzero(me.fromText('2.2/-3.5'))).toEqual(true);
	expect(is_nonzero(me.fromText('-2.2/-3.5'))).toEqual(true);
	expect(is_nonzero(me.fromText('-2.2/(5-5)'))).toEqual(true);
	expect(is_nonzero(me.fromText('(-6+6)/(3-5)'))).toEqual(false);
	expect(is_nonzero(me.fromText('(-6+6)/(5-5)'))).toEqual(true);
	expect(is_nonzero(me.fromText('abs(-5)'))).toEqual(true);
	expect(is_nonzero(me.fromText('sin(0)'))).toEqual(false);
	expect(is_nonzero(me.fromText('sqrt(-4)'))).toEqual(true);

    });

    it("variables are unknown", function () {
	expect(is_nonnegative(me.fromText('abs(x)'))).toEqual(undefined);
	expect(is_nonnegative(me.fromText('sin(y)'))).toEqual(undefined);
	expect(is_negative(me.fromText('abs(x)'))).toEqual(undefined);
	expect(is_negative(me.fromText('sin(y)'))).toEqual(undefined);
	expect(is_nonpositive(me.fromText('abs(x)'))).toEqual(undefined);
	expect(is_nonpositive(me.fromText('sin(y)'))).toEqual(undefined);
	expect(is_positive(me.fromText('abs(x)'))).toEqual(undefined);
	expect(is_positive(me.fromText('sin(y)'))).toEqual(undefined);
	expect(is_nonzero(me.fromText('abs(x)'))).toEqual(undefined);
	expect(is_nonzero(me.fromText('sin(y)'))).toEqual(undefined);

    });

    it("other operators are not integers", function () {
	expect(is_nonnegative(me.fromText('(5,2)').tuples_to_vectors())).toEqual(false);
	expect(is_nonnegative(me.fromText('5=3'))).toEqual(false);
	expect(is_negative(me.fromText('(5,2)').tuples_to_vectors())).toEqual(false);
	expect(is_negative(me.fromText('5=3'))).toEqual(false);
	expect(is_nonpositive(me.fromText('(5,2)').tuples_to_vectors())).toEqual(false);
	expect(is_nonpositive(me.fromText('5=3'))).toEqual(false);
	expect(is_positive(me.fromText('(5,2)').tuples_to_vectors())).toEqual(false);
	expect(is_positive(me.fromText('5=3'))).toEqual(false);
	expect(is_nonzero(me.fromText('(5,2)').tuples_to_vectors())).toEqual(false);
	expect(is_nonzero(me.fromText('5=3'))).toEqual(false);

    });

    
    
});
