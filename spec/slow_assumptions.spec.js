import me from '../lib/math-expressions';
import { is_integer } from '../lib/assumptions/element_of_sets.js';
import { is_real } from '../lib/assumptions/element_of_sets.js';
import { is_complex } from '../lib/assumptions/element_of_sets.js';
import { is_nonzero } from '../lib/assumptions/element_of_sets.js';
import { is_nonnegative } from '../lib/assumptions/element_of_sets.js';
import { is_nonpositive } from '../lib/assumptions/element_of_sets.js';
import { is_positive } from '../lib/assumptions/element_of_sets.js';
import { is_negative } from '../lib/assumptions/element_of_sets.js';
import * as trees from '../lib/trees/basic';
import { simplify } from '../lib/expression/simplify';
import {default_order} from '../lib/trees/default_order';

let ordered_trees_equal = (a,b) => trees.equal(default_order(a), default_order(b));

describe("add and get assumptions", function () {

    it("single variable", function () {
	me.clear_assumptions();
	me.add_assumption(me.from('x>0'));
	expect(ordered_trees_equal(me.get_assumptions('x'),me.from('x>0').tree)).toBeTruthy();
	expect(ordered_trees_equal(me.assumptions.get_assumptions('x'),me.from('x>0').tree)).toBeTruthy();
	expect(ordered_trees_equal(me.get_assumptions([['x']]),me.from('x>0').tree)).toBeTruthy();
	expect(ordered_trees_equal(me.assumptions.get_assumptions([['x']]),me.from('x>0').tree)).toBeTruthy();

	me.clear_assumptions();
	expect(me.get_assumptions('x')).toEqual(undefined);
	expect(me.assumptions.get_assumptions('x')).toEqual(undefined);

	me.assumptions.add_assumption(me.from('x<=0'));
	expect(ordered_trees_equal(me.get_assumptions('x'),me.from('x<=0').tree)).toBeTruthy();
	expect(ordered_trees_equal(me.assumptions.get_assumptions('x'),me.from('x<=0').tree)).toBeTruthy();

	me.add_assumption(me.from('x > -2').tree);
	expect(ordered_trees_equal(me.get_assumptions('x'),simplify(me.from('x<=0 and x > -2').tree))).toBeTruthy();
	expect(ordered_trees_equal(me.assumptions.get_assumptions('x'),simplify(me.from('x<=0 and x > -2').tree))).toBeTruthy();

	me.clear_assumptions();
	
	me.assumptions.add_assumption(me.from('x != 0').tree);
	expect(ordered_trees_equal(me.get_assumptions('x'),me.from('x!=0').tree)).toBeTruthy();
	expect(ordered_trees_equal(me.assumptions.get_assumptions('x'),me.from('x !=0').tree)).toBeTruthy();

	me.clear_assumptions();

    });
	
    it("multiple variables", function () {

	me.clear_assumptions();
	me.add_assumption(me.from('x >=0'));
	expect(ordered_trees_equal(me.get_assumptions('x'),me.from('x>=0').tree)).toBeTruthy();
	
	expect(me.get_assumptions('y')).toEqual(undefined);

	me.add_assumption(me.from('x < y+1'));
	expect(ordered_trees_equal(me.get_assumptions('x'),me.from('x>=0 and x < y+1').evaluate_numbers().tree)).toBeTruthy();
	expect(ordered_trees_equal(me.get_assumptions('y'),me.from('y > x-1 and y >-1').evaluate_numbers().tree)).toBeTruthy();
	expect(me.get_assumptions('z')).toEqual(undefined);

	me.clear_assumptions();

	me.assumptions.add_assumption(me.from('a < b < c'));
	expect(ordered_trees_equal(me.get_assumptions('b'),me.from('a < b and b < c').tree)).toBeTruthy();

	expect(ordered_trees_equal(me.assumptions.get_assumptions('b'),me.from('a < b and b < c').tree)).toBeTruthy();

	expect(ordered_trees_equal(me.get_assumptions('a'),me.from('a < b and a < c').tree)).toBeTruthy();

	me.clear_assumptions();
    });


    it("multiple inequalities", function () {

	me.clear_assumptions();
	me.add_assumption(me.from("a < b <=c"));
	expect(ordered_trees_equal(me.get_assumptions('b'),me.from('a < b and b <= c').tree)).toBeTruthy();
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
	    expect(ordered_trees_equal(me.get_assumptions('x'),me.from(input[1]).tree)).toBeTruthy();
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
	    expect(ordered_trees_equal(me.get_assumptions([['a', 'b']]),me.from(input[1]).tree)).toBeTruthy();
	    me.clear_assumptions();
	});	
    });

    it("avoid redundancies", function () {

	me.clear_assumptions();
	me.add_assumption(me.from('x <=0'));
	expect(ordered_trees_equal(me.get_assumptions('x'),me.from('x<=0').tree)).toBeTruthy();

	me.add_assumption(me.from('x <=0'));
	expect(ordered_trees_equal(me.get_assumptions('x'),me.from('x<=0').tree)).toBeTruthy();
	
	me.add_assumption(me.from('-1 < x <=0'));
	expect(ordered_trees_equal(me.get_assumptions('x'),simplify(me.from('x<=0 and x>-1').tree))).toBeTruthy();
	
	me.clear_assumptions();
	
    });

    it("generic assumptions", function () {

	me.clear_assumptions();
	me.add_generic_assumption(me.from('x elementof R'));
	expect(ordered_trees_equal(me.get_assumptions('x'),me.from('x elementof R').tree)).toBeTruthy();
	expect(ordered_trees_equal(me.get_assumptions('y'),me.from('y elementof R').tree)).toBeTruthy();
	expect(ordered_trees_equal(me.get_assumptions('z'),me.from('z elementof R').tree)).toBeTruthy();

	me.add_assumption(me.from('x != 3'), true);
	expect(ordered_trees_equal(me.get_assumptions('x'),me.from('x != 3').tree)).toBeTruthy();
	expect(ordered_trees_equal(me.get_assumptions('y'),me.from('y elementof R').tree)).toBeTruthy();
	expect(ordered_trees_equal(me.get_assumptions('z'),me.from('z elementof R').tree)).toBeTruthy();

	me.add_assumption(me.from('y < z'), true);
	me.add_assumption(me.from('x != 3'));
	expect(ordered_trees_equal(me.get_assumptions('x'),me.from('x != 3').tree)).toBeTruthy();
	expect(ordered_trees_equal(me.get_assumptions('y'),me.from('y < z').tree)).toBeTruthy();
	expect(ordered_trees_equal(me.get_assumptions('z'),me.from('y < z').tree)).toBeTruthy();
	
	me.clear_assumptions();
	me.assumptions.add_generic_assumption(me.from('x < y'));
	expect(ordered_trees_equal(me.get_assumptions('x'),me.from('x < y').tree)).toBeTruthy();
	expect(me.get_assumptions('y')).toEqual(undefined);;
	expect(ordered_trees_equal(me.get_assumptions('z'),me.from('z < y').tree)).toBeTruthy();
	
	me.clear_assumptions();
	me.add_generic_assumption(me.from('x elementof R'));
	me.add_assumption(me.from('x != 3'));
	me.add_assumption(me.from('y != 4'), true);
	me.add_assumption(me.from('a < b'));
	me.add_assumption(me.from('c < d'), true);

	
	expect(ordered_trees_equal(me.get_assumptions('x'),
			   me.from('x != 3 and x elementof R')
			   .tree)).toBeTruthy();
		    
	expect(ordered_trees_equal(me.get_assumptions('y'),me.from(
	    'y != 4').tree)).toBeTruthy();

	expect(ordered_trees_equal(
	    me.get_assumptions('a'),
	    me.from('a < b and a elementof R and b elementof R').tree))
	    .toBeTruthy();
	expect(ordered_trees_equal(me.get_assumptions('c'),
			   me.from('c < d').tree)).toBeTruthy();

	me.clear_assumptions();

    });


    it("adding and removing assumptions", function () {

	me.clear_assumptions();

	me.add_assumption(me.from('x > 0'))
	expect(ordered_trees_equal(me.get_assumptions('x'),me.from(
	    'x>0').tree)).toBeTruthy();

	me.add_assumption(me.from('y > 1'), true)
	expect(ordered_trees_equal(me.get_assumptions('y'),me.from(
	    'y>1').tree)).toBeTruthy();

	expect(me.get_assumptions('z')).toEqual(undefined);
	
	me.add_generic_assumption(me.from('x elementof Z'), true)
	expect(ordered_trees_equal(me.get_assumptions('z'),me.from(
	    'z elementof Z').tree)).toBeTruthy();

	me.add_assumption(me.from('z > 2'));
	expect(ordered_trees_equal(me.get_assumptions('z'),me.from(
	    'z elementof Z and z > 2').tree)).toBeTruthy();

	me.remove_assumption(me.from('z elementof Z'));
	expect(ordered_trees_equal(me.get_assumptions('z'),me.from(
	    'z > 2').tree)).toBeTruthy();

	me.remove_assumption(me.from('z > 2'));
	expect(me.get_assumptions('z')).toEqual(undefined);

	me.add_assumption(me.from('z > 5'));
	expect(ordered_trees_equal(me.get_assumptions('z'),me.from(
	    'z > 5').tree)).toBeTruthy();

	me.add_assumption(me.from('z < 9'));
	expect(ordered_trees_equal(me.get_assumptions('z'),me.from(
	    'z > 5 and z < 9').tree)).toBeTruthy();

	me.remove_assumption(me.from('z <= 9'));
	expect(ordered_trees_equal(me.get_assumptions('z'),me.from(
	    'z > 5 and z < 9').tree)).toBeTruthy();

	me.remove_assumption(me.from('5 <= z < 9'));
	expect(ordered_trees_equal(me.get_assumptions('z'),me.from(
	    'z > 5').tree)).toBeTruthy();

	me.remove_assumption(me.from('5 < z <= 9'));
	expect(me.get_assumptions('z')).toEqual(undefined);

	expect(ordered_trees_equal(me.get_assumptions('w'),me.from(
	    'w elementof Z').tree)).toBeTruthy();
	
	me.add_assumption(me.from('w > 3'), true);
	expect(ordered_trees_equal(me.get_assumptions('w'),me.from(
	    'w >3').tree)).toBeTruthy();

	me.clear_assumptions();
    });



    it("removing assumptions with generic", function () {
	me.clear_assumptions();
	
	me.add_generic_assumption(me.from('x elementof Z'));
	expect(ordered_trees_equal(me.get_assumptions('a'),me.from(
	    'a elementof Z').tree)).toBeTruthy();
	expect(ordered_trees_equal(me.get_assumptions('b'),me.from(
	    'b elementof Z').tree)).toBeTruthy();

	me.add_assumption(me.from('a+b< 1'));
	expect(ordered_trees_equal(me.get_assumptions('a'),me.from(
	    'a elementof Z and a<1-b and b elementof Z').tree)).toBeTruthy();
	expect(ordered_trees_equal(me.get_assumptions('b'),me.from(
	    'b elementof Z and b < 1-a and a elementof Z').tree)).toBeTruthy();

	me.remove_assumption(me.from('b \\in Z'));
	expect(ordered_trees_equal(me.get_assumptions('a'),me.from(
	    'a elementof Z and a<1-b').tree)).toBeTruthy();
	expect(ordered_trees_equal(me.get_assumptions('b'),me.from(
	    'b<1-a and a elementof Z').tree)).toBeTruthy();

	me.remove_assumption(me.from('a+b< 1'));
	expect(ordered_trees_equal(me.get_assumptions('a'),me.from(
	    'a elementof Z').tree)).toBeTruthy();
	expect(me.get_assumptions('b')).toEqual(undefined);

	me.clear_assumptions();

    });

    it("removing generic assumptions", function () {
	me.clear_assumptions();

	me.add_generic_assumption(me.from('x elementof Z'))
	expect(ordered_trees_equal(me.get_assumptions('q'),me.from(
	    'q elementof Z').tree)).toBeTruthy();

	me.remove_generic_assumption(me.from('x elementof Z'))
	expect(me.get_assumptions('q')).toEqual(undefined);

	me.add_generic_assumption(me.from('a < x < b'))
	expect(ordered_trees_equal(me.get_assumptions('q'),
			   me.from('a<q and q<b').tree)).toBeTruthy();
	
	me.add_assumption(me.from('q != 0'));
	expect(ordered_trees_equal(me.get_assumptions('q'),
			   me.from('a<q and q<b and q !=0').tree)).toBeTruthy();
	expect(ordered_trees_equal(me.get_assumptions('r'),
			   me.from('a<r and r<b').tree)).toBeTruthy();

	me.remove_generic_assumption(me.from('b > x'))
	expect(ordered_trees_equal(me.get_assumptions('q'),
			   me.from('a<q and q<b and q !=0').tree)).toBeTruthy();
	expect(ordered_trees_equal(me.get_assumptions('r'),
			   me.from('a<r').tree)).toBeTruthy();
	
	me.remove_generic_assumption(me.from('x > a'))
	expect(ordered_trees_equal(me.get_assumptions('q'),
			   me.from('a<q and q<b and q !=0').tree)).toBeTruthy();
	expect(me.get_assumptions('r',{known_variables:['a','b']})).toEqual(undefined);
	
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
	me.add_assumption(me.from('Z ∋ n'));
	expect(is_integer(me.fromText('n'))).toEqual(true);
	
	me.clear_assumptions();
	me.add_assumption(me.from('n ∉ Z'));
	expect(is_integer(me.fromText('n'))).toEqual(false);

	me.clear_assumptions();
	me.add_assumption(me.from('Z ∌ n'));
	expect(is_integer(me.fromText('n'))).toEqual(false);
	
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


describe("is positive/negative/zero/real/complex", function() {

    var literals = [
	['3.2', true, false, false, true, true, true, true],
	['0', true, false, true, false, false, true, true],
	['-5.12', false, true, true, false, true, true, true],
	['8+12.3', true, false, false, true, true, true, true],
	['13-13', true, false, true, false, false, true, true],
	['13-13.7', false, true, true, false, true, true, true],
	['2.1*3.6', true, false, false, true, true, true, true],
	['21.3*(-31.2)', false, true, true, false, true, true, true],
	['21.3*(4-4)*(-31.2)', true, false, true, false, false, true, true],
	['2.2/3.5', true, false, false, true, true, true, true],
	['-2.2/3.5', false, true, true, false, true, true, true],
	['2.2/-3.5', false, true, true, false, true, true, true],
	['-2.2/-3.5', true, false, false, true, true, true, true],
	['-2.2/(5-5)', false, false, false, false, true, false, false],
	['(-6+6)/(3-5)', true, false, true, false, false, true, true],
	['(-6+6)/(5-5)', false, false, false, false, undefined, false, false],
	['abs(-5)', true, false, false, true, true, true, true],
	['sin(0)', true, false, true, false, false, true, true],
	['sqrt(-4)', false, false, false, false, true, false, true],
	['0^0', false, false, false, false, undefined, false, false],
	['3.9-3.2i', false, false, false, false, true, false, true],
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
	    expect(is_complex(me.fromText(input[0]))).toEqual(input[7]);
	});
    });


    var variables = [
	['y', undefined, undefined, undefined, undefined, undefined, undefined, undefined],
	['x+y', undefined, undefined, undefined, undefined, undefined, undefined, undefined],
	['x-y', undefined, undefined, undefined, undefined, undefined, undefined, undefined],
	['x*y', undefined, undefined, undefined, undefined, undefined, undefined, undefined],
	['x/y', undefined, undefined, undefined, undefined, undefined, undefined, undefined],
	['x^y', undefined, undefined, undefined, undefined, undefined, undefined, undefined],
	['abs(x)', undefined, undefined, undefined, undefined, undefined, undefined, undefined],
	['sin(y)', undefined, undefined, undefined, undefined, undefined, undefined, undefined],
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
	    expect(is_complex(me.fromText(input[0]))).toEqual(input[7]);
	});
    });


    var operators = [
	['(5,2)', false, false, false, false, false, false, false],
	['5=3', false, false, false, false, undefined, false, false],
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
	    expect(is_complex(me.fromText(input[0]))).toEqual(input[7]);
	});
    });


    var assumptions = [
	[undefined,
	 undefined, undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R',
	 undefined, undefined, undefined, undefined, undefined, true, true],
	['x elementof R and x != 0',
	 undefined, undefined, undefined, undefined, true, true, true],
	['x elementof C',
	 undefined, undefined, undefined, undefined, undefined, undefined, true],
	['x elementof C and x != 0',
	 undefined, undefined, undefined, undefined, true, undefined, true],
	['x != 0',
	 undefined, undefined, undefined, undefined, true, undefined, undefined],
	['x > 0',
	 true, false, false, true, true, true, true, true],
	['x >= 0',
	 true, false, undefined, undefined, undefined, true, true],
	['x < 0',
	 false, true, true, false, true, true, true],
	['x <= 0',
	 undefined, undefined, true, false, undefined, true, true],
	['x = 0',
	 true, false, true, false, false, true, true],
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
	    expect(is_complex(me.fromText('x'))).toEqual(input[7]);
	});
    });

    
    var assumptions_negative = [
	[undefined,
	 undefined, undefined, undefined, undefined, undefined, undefined, undefined],
	['x elementof R',
	 undefined, undefined, undefined, undefined, undefined, true, true],
	['x elementof R and x != 0',
	 undefined, undefined, undefined, undefined, true, true, true],
	['x elementof C',
	 undefined, undefined, undefined, undefined, undefined, undefined, true],
	['x elementof C and x != 0',
	 undefined, undefined, undefined, undefined, true, undefined, true],
	['x != 0',
	 undefined, undefined, undefined, undefined, true, undefined, undefined],
	['x > 0',
	 false, true, true, false, true, true, true],
	['x >= 0',
	 undefined, undefined, true, false, undefined, true, true],
	['x < 0',
	 true, false, false, true, true, true, true],
	['x <= 0',
	 true, false, undefined, undefined, undefined, true, true],
	['x = 0',
	 true, false, true, false, false, true, true],
    ];	
	

    assumptions_negative.forEach(function(input) {
	it("assumptions negative: " + input, function() {
	    me.clear_assumptions();
	    me.add_assumption(me.from(input[0]));
	    expect(is_nonnegative(me.fromText('-x'))).toEqual(input[1]);
	    expect(is_negative(me.fromText('-x'))).toEqual(input[2]);
	    expect(is_nonpositive(me.fromText('-x'))).toEqual(input[3]);
	    expect(is_positive(me.fromText('-x'))).toEqual(input[4]);
	    expect(is_nonzero(me.fromText('-x'))).toEqual(input[5]);
	    expect(is_real(me.fromText('-x'))).toEqual(input[6]);
	    expect(is_complex(me.fromText('-x'))).toEqual(input[7]);
	});
    });


    it("subtract identical is zero", function() {
	me.clear_assumptions();
	expect(is_nonzero(me.fromText('x-x'))).toEqual(false);
	expect(is_nonzero(me.fromText('x-x').evaluate_numbers())).toEqual(false);
	expect(is_nonzero(me.fromText('3x-3x'))).toEqual(false);
	expect(is_nonzero(me.fromText('3x-3x').evaluate_numbers())).toEqual(false);

    });

    var sum_tests=[

	[undefined,
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],

	['x elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof R and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],

	['x elementof R and x !=  0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and x !=  0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and x !=  0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and x !=  0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof R and x !=  0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof R and x !=  0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and x !=  0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and x !=  0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and x !=  0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and x !=  0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and x !=  0 and y = 0',
	 undefined, undefined, undefined, undefined, true,
	 true, true],

	['x elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],

	['x elementof C and x != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y = 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],

	['x != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y = 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],

	['x > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x > 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x > 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x > 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x > 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x > 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x > 0 and y > 0',
	 true, false, false, true, true,
	 true, true],
	['x > 0 and y >= 0',
	 true, false, false, true, true,
	 true, true],
	['x > 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x > 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x > 0 and y = 0',
	 true, false, false, true, true,
	 true, true],

	['x >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x >= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x >= 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x >= 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x >= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y > 0',
	 true, false, false, true, true,
	 true, true],
	['x >= 0 and y >= 0',
	 true, false, undefined, undefined, undefined,
	 true, true],
	['x >= 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x >= 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x >= 0 and y = 0',
	 true, false, undefined, undefined, undefined,
	 true, true],

	['x < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x < 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x < 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x < 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x < 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x < 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x < 0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x < 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x < 0 and y < 0',
	 false, true, true, false, true,
	 true, true],
	['x < 0 and y <= 0',
	 false, true, true, false, true,
	 true, true],
	['x < 0 and y = 0',
	 false, true, true, false, true,
	 true, true],

	['x <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x <= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x <= 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x <= 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x <= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x <= 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x <= 0 and y < 0',
	 false, true, true, false, true,
	 true, true],
	['x <= 0 and y <= 0',
	 undefined, undefined, true, false, undefined,
	 true, true],
	['x <= 0 and y = 0',
	 undefined, undefined, true, false, undefined,
	 true, true],

	['x = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x = 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x = 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 true, true],
	['x = 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x = 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x = 0 and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x = 0 and y > 0',
	 true, false, false, true, true,
	 true, true],
	['x = 0 and y >= 0',
	 true, false, undefined, undefined, undefined,
	 true, true],
	['x = 0 and y < 0',
	 false, true, true, false, true,
	 true, true],
	['x = 0 and y <= 0',
	 undefined, undefined, true, false, undefined,
	 true, true],
	['x = 0 and y = 0',
	 true, false, true, false, false,
	 true, true],
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
	    expect(is_complex(me.fromText('x+y'))).toEqual(input[7]);
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
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],

	['x elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof R and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],

	['x elementof R and x != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and x != 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof R and x != 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof R and x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and x != 0 and y = 0',
	 undefined, undefined, undefined, undefined, true,
	 true, true],

	['x elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],

	['x elementof C and x != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y = 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],

	['x != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y = 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],

	['x > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x > 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x > 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x > 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x > 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x > 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x > 0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x > 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x > 0 and y < 0',
	 true, false, false, true, true,
	 true, true],
	['x > 0 and y <= 0',
	 true, false, false, true, true,
	 true, true],
	['x > 0 and y = 0',
	 true, false, false, true, true,
	 true, true],

	['x >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x >= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x >= 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x >= 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x >= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x >= 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x >= 0 and y < 0',
	 true, false, false, true, true,
	 true, true],
	['x >= 0 and y <= 0',
	 true, false, undefined, undefined, undefined,
	 true, true],
	['x >= 0 and y = 0',
	 true, false, undefined, undefined, undefined,
	 true, true],

	['x < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x < 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x < 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x < 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x < 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x < 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x < 0 and y > 0',
	 false, true, true, false, true,
	 true, true],
	['x < 0 and y >= 0',
	 false, true, true, false, true,
	 true, true],
	['x < 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x < 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x < 0 and y = 0',
	 false, true, true, false, true,
	 true, true],

	['x <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x <= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x <= 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x <= 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x <= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y > 0',
	 false, true, true, false, true,
	 true, true],
	['x <= 0 and y >= 0',
	 undefined, undefined, true, false, undefined,
	 true, true],
	['x <= 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x <= 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x <= 0 and y = 0',
	 undefined, undefined, true, false, undefined,
	 true, true],

	['x = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x = 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x = 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 true, true],
	['x = 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x = 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x = 0 and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x = 0 and y > 0',
	 false, true, true, false, true,
	 true, true],
	['x = 0 and y >= 0',
	 undefined, undefined, true, false, undefined,
	 true, true],
	['x = 0 and y < 0',
	 true, false, false, true, true,
	 true, true],
	['x = 0 and y <= 0',
	 true, false, undefined, undefined, undefined,
	 true, true],
	['x = 0 and y = 0',
	 true, false, true, false, false,
	 true, true],
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
	    expect(is_complex(me.fromText('x-y'))).toEqual(input[7]);
	    me.clear_assumptions();
	});
    });


    var product_tests=[
	
	[undefined,
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y = 0',
	 true, false, true, false, false,
	 true, true],

	['x elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof R and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y = 0',
	 true, false, true, false, false,
	 true, true],


	['x elementof R and x != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined],
	['x elementof R and x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 true, true],
	['x elementof R and x != 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof R and x != 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof R and x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x elementof R and x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, true,
	 true, true],
	['x elementof R and x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, true,
	 true, true],
	['x elementof R and x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and x != 0 and y = 0',
	 true, false, true, false, false,
	 true, true],

	['x elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y = 0',
	 true, false, true, false, false,
	 true, true],


	['x elementof C and x != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined],
	['x elementof C and x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof C and x != 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof C and x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x elementof C and x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof C and x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof C and x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and x != 0 and y = 0',
	 true, false, true, false, false,
	 true, true],

	['x != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y = 0',
	 true, false, true, false, false,
	 true, true],

	['x > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x > 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x > 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 true, true],
	['x > 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x > 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x > 0 and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x > 0 and y > 0',
	 true, false, false, true, true,
	 true, true],
	['x > 0 and y >= 0',
	 true, false, undefined, undefined, undefined,
	 true, true],
	['x > 0 and y < 0',
	 false, true, true, false, true,
	 true, true],
	['x > 0 and y <= 0',
	 undefined, undefined, true, false, undefined,
	 true, true],
	['x > 0 and y = 0',
	 true, false, true, false, false,
	 true, true],

	['x >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x >= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x >= 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x >= 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x >= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y > 0',
	 true, false, undefined, undefined, undefined,
	 true, true],
	['x >= 0 and y >= 0',
	 true, false, undefined, undefined, undefined,
	 true, true],
	['x >= 0 and y < 0',
	 undefined, undefined, true, false, undefined,
	 true, true],
	['x >= 0 and y <= 0',
	 undefined, undefined, true, false, undefined,
	 true, true],
	['x >= 0 and y = 0',
	 true, false, true, false, false,
	 true, true],

	['x < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x < 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x < 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 true, true],
	['x < 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x < 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x < 0 and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x < 0 and y > 0',
	 false, true, true, false, true,
	 true, true],
	['x < 0 and y >= 0',
	 undefined, undefined, true, false, undefined,
	 true, true],
	['x < 0 and y < 0',
	 true, false, false, true, true,
	 true, true],
	['x < 0 and y <= 0',
	 true, false, undefined, undefined, undefined,
	 true, true],
	['x < 0 and y = 0',
	 true, false, true, false, false,
	 true, true],

	['x <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x <= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x <= 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x <= 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x <= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y > 0',
	 undefined, undefined, true, false, undefined,
	 true, true],
	['x <= 0 and y >= 0',
	 undefined, undefined, true, false, undefined,
	 true, true],
	['x <= 0 and y < 0',
	 true, false, undefined, undefined, undefined,
	 true, true],
	['x <= 0 and y <= 0',
	 true, false, undefined, undefined, undefined,
	 true, true],
	['x <= 0 and y = 0',
	 true, false, true, false, false,
	 true, true],

	['x = 0',
	 true, false, true, false, false,
	 true, true],
	['x = 0 and y elementof R',
	 true, false, true, false, false,
	 true, true],
	['x = 0 and y elementof R and y != 0',
	 true, false, true, false, false,
	 true, true],
	['x = 0 and y elementof C',
	 true, false, true, false, false,
	 true, true],
	['x = 0 and y elementof C and y != 0',
	 true, false, true, false, false,
	 true, true],
	['x = 0 and y != 0',
	 true, false, true, false, false,
	 true, true],
	['x = 0 and y > 0',
	 true, false, true, false, false,
	 true, true],
	['x = 0 and y >= 0',
	 true, false, true, false, false,
	 true, true],
	['x = 0 and y < 0',
	 true, false, true, false, false,
	 true, true],
	['x = 0 and y <= 0',
	 true, false, true, false, false,
	 true, true],
	['x = 0 and y = 0',
	 true, false, true, false, false,
	 true, true],
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
	    expect(is_complex(me.fromText('xy'))).toEqual(input[7]);
	    me.clear_assumptions();
	});
    });

    var quotient_tests=[
	
	[undefined,
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	
	['x elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x elementof R and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	
	['x elementof R and x != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 true, true],
	['x elementof R and x != 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and x != 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof R and x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x elementof R and x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, true,
	 true, true],
	['x elementof R and x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, true,
	 true, true],
	['x elementof R and x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and x != 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	
	['x elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof C and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	
	['x elementof C and x != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof C and x != 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and x != 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof C and x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x elementof C and x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof C and x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof C and x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof C and x != 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	
	['x != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x != 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	
	['x > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x > 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x > 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 true, true],
	['x > 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x > 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x > 0 and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x > 0 and y > 0',
	 true, false, false, true, true,
	 true, true],
	['x > 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x > 0 and y < 0',
	 false, true, true, false, true,
	 true, true],
	['x > 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x > 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	
	['x >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x >= 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x >= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y > 0',
	 true, false, undefined, undefined, undefined,
	 true, true],
	['x >= 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y < 0',
	 undefined, undefined, true, false, undefined,
	 true, true],
	['x >= 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	
	['x < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x < 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x < 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 true, true],
	['x < 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x < 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x < 0 and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x < 0 and y > 0',
	 false, true, true, false, true,
	 true, true],
	['x < 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x < 0 and y < 0',
	 true, false, false, true, true,
	 true, true],
	['x < 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x < 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	
	['x <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	['x <= 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x <= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y > 0',
	 undefined, undefined, true, false, undefined,
	 true, true],
	['x <= 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y < 0',
	 true, false, undefined, undefined, undefined,
	 true, true],
	['x <= 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	
	['x = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x = 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x = 0 and y elementof R and y != 0',
	 true, false, true, false, false,
	 true, true],
	['x = 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x = 0 and y elementof C and y != 0',
	 true, false, true, false, false,
	 true, true],
	['x = 0 and y != 0',
	 true, false, true, false, false,
	 true, true],
	['x = 0 and y > 0',
	 true, false, true, false, false,
	 true, true],
	['x = 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x = 0 and y < 0',
	 true, false, true, false, false,
	 true, true],
	['x = 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x = 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
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
	    expect(is_complex(me.fromText('x/y'))).toEqual(input[7]);
	    me.clear_assumptions();
	});
    });

    var power_tests=[
	
	[undefined,
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	
	['x elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x elementof R and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x elementof R and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],

	['x elementof R and x != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x elementof R and x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof R and x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof R and x != 0 and y elementof C',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof R and x != 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof R and x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x elementof R and x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof R and x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof R and x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof R and x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x elementof R and x != 0 and y = 0',
	 true, false, false, true, true,
	 true, true],

	['x != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y elementof R',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y elementof C',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y > 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y >= 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y < 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y <= 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x != 0 and y = 0',
	 true, false, false, true, true,
	 true, true],

	['x > 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x > 0 and y elementof R',
	 true, false, false, true, true,
	 true, true],
	['x > 0 and y elementof R and y != 0',
	 true, false, false, true, true,
	 true, true],
	['x > 0 and y elementof C',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x > 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x > 0 and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x > 0 and y > 0',
	 true, false, false, true, true,
	 true, true],
	['x > 0 and y >= 0',
	 true, false, false, true, true,
	 true, true],
	['x > 0 and y < 0',
	 true, false, false, true, true,
	 true, true],
	['x > 0 and y <= 0',
	 true, false, false, true, true,
	 true, true],
	['x > 0 and y = 0',
	 true, false, false, true, true,
	 true, true],

	['x >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y > 0',
	 true, false, undefined, undefined, undefined,
	 true, true],
	['x >= 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x >= 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],

	['x < 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x < 0 and y elementof R',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x < 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x < 0 and y elementof C',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x < 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x < 0 and y != 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, undefined],
	['x < 0 and y > 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x < 0 and y >= 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x < 0 and y < 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x < 0 and y <= 0',
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	['x < 0 and y = 0',
	 true, false, false, true, true,
	 true, true],

	['x <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y > 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	['x <= 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x <= 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],

	['x = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x = 0 and y elementof R',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x = 0 and y elementof R and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x = 0 and y elementof C',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x = 0 and y elementof C and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x = 0 and y != 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x = 0 and y > 0',
	 true, false, true, false, false,
	 true, true],
	['x = 0 and y >= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x = 0 and y < 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x = 0 and y <= 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
	['x = 0 and y = 0',
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, undefined],
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
	    expect(is_complex(me.fromText('x^y'))).toEqual(input[7]);
	    me.clear_assumptions();
	});
    });

});

describe("assumptions", function () {
    
    it("integer implies real/complex", function () {
	me.clear_assumptions();
	me.add_assumption(me.from('y elementof Z'));
	expect(is_integer(me.from('y'))).toEqual(true);
	expect(is_real(me.from('y'))).toEqual(true);
	expect(is_complex(me.from('y'))).toEqual(true);
	me.clear_assumptions();
	me.add_assumption(me.from('y notelementof Z'));
	expect(is_integer(me.from('y'))).toEqual(false);
	expect(is_real(me.from('y'))).toEqual(undefined);
	expect(is_complex(me.from('y'))).toEqual(undefined);
	me.clear_assumptions();
	me.add_assumption(me.from('not(y elementof Z)'));
	expect(is_integer(me.from('y'))).toEqual(false);
	expect(is_real(me.from('y'))).toEqual(undefined);
	expect(is_complex(me.from('y'))).toEqual(undefined);
	me.clear_assumptions();
	me.add_assumption(me.from('not(y notelementof Z)'));
	expect(is_integer(me.from('y'))).toEqual(true);
	expect(is_real(me.from('y'))).toEqual(true);
	expect(is_complex(me.from('y'))).toEqual(true);
	me.clear_assumptions();
	
    });

    it("real implies complex", function () {
	me.clear_assumptions();
	me.add_assumption(me.from('y elementof R'));
	expect(is_real(me.from('y'))).toEqual(true);
	expect(is_complex(me.from('y'))).toEqual(true);
	me.clear_assumptions();
	me.add_assumption(me.from('y notelementof R'));
	expect(is_real(me.from('y'))).toEqual(false);
	expect(is_complex(me.from('y'))).toEqual(undefined);
	me.clear_assumptions();
	me.add_assumption(me.from('not(y elementof R)'));
	expect(is_real(me.from('y'))).toEqual(false);
	expect(is_complex(me.from('y'))).toEqual(undefined);
	me.clear_assumptions();
	me.add_assumption(me.from('not(y notelementof R)'));
	expect(is_real(me.from('y'))).toEqual(true);
	expect(is_complex(me.from('y'))).toEqual(true);
	me.clear_assumptions();
	
    });

    it("recursive assumptions no infinite loop", function() {
	me.clear_assumptions();
	me.add_assumption(me.from("x=x"));
	expect(is_real(me.from('x'))).toEqual(undefined);
	me.clear_assumptions();
	me.add_assumption(me.from("x=y=x"));
	expect(is_real(me.from('x'))).toEqual(undefined);
    });


    it("combined assumptions", function() {
	me.clear_assumptions();
	me.add_assumption(me.from("y elementof R"));
	me.add_assumption(me.from("x=y"));
	expect(is_real(me.from('x'))).toEqual(true);

	me.clear_assumptions();
	me.add_assumption(me.from("y < -1"));
	me.add_assumption(me.from("y >= x"));
	me.add_assumption(me.from("u < x"));
	expect(is_negative(me.from('y'))).toEqual(true);
	expect(is_negative(me.from('x'))).toEqual(true);
	expect(is_negative(me.from('u'))).toEqual(true);

	me.clear_assumptions();
	me.add_assumption(me.from("y < -1"));
	me.add_assumption(me.from("x > 1"));
	me.add_assumption(me.from("u < y-x"));
	expect(is_negative(me.from('y'))).toEqual(true);
	expect(is_positive(me.from('x'))).toEqual(true);
	expect(is_negative(me.from('u'))).toEqual(true);

	me.clear_assumptions();
	me.add_assumption(me.from("y + 1 < 0"));
	me.add_assumption(me.from("x > 1"));
	me.add_assumption(me.from("u < y-x"));
	expect(is_negative(me.from('y'))).toEqual(true);
	expect(is_positive(me.from('x'))).toEqual(true);
	expect(is_negative(me.from('u'))).toEqual(true);

	me.clear_assumptions();
	me.add_assumption(me.from("y < -1"));
	me.add_assumption(me.from("x > -1"));
	me.add_assumption(me.from("u < y-x"));
	expect(is_negative(me.from('u'))).toEqual(true);

	me.clear_assumptions();
	me.add_assumption(me.from("y elementof R"));
	me.add_assumption(me.from("x ne y"));
	expect(is_real(me.from('x'))).toEqual(undefined);
	me.clear_assumptions();

	me.clear_assumptions();
	me.add_assumption(me.from("x < 0 or x > 0"));
	expect(is_nonzero(me.from('x'))).toEqual(true);

	me.clear_assumptions();
	
    });


    it("combined assumptions, negated", function() {
	me.clear_assumptions();
	me.add_assumption(me.from("x < 0"));
	me.add_assumption(me.from("y != x"));
	expect(is_negative(me.from('y'))).toEqual(undefined);

	me.clear_assumptions();
	me.add_assumption(me.from("x < 0"));
	me.add_assumption(me.from("not(y = x)"));
	expect(is_negative(me.from('y'))).toEqual(undefined);

	me.clear_assumptions();
	me.add_assumption(me.from("x < 0"));
	me.add_assumption(me.from("y = x"));
	expect(is_negative(me.from('y'))).toEqual(true);

	me.clear_assumptions();
	me.add_assumption(me.from("x < 0"));
	me.add_assumption(me.from("not(y != x)"));
	expect(is_negative(me.from('y'))).toEqual(true);
	
	me.clear_assumptions();
	me.add_assumption(me.from("x != 0"));
	me.add_assumption(me.from("y != x"));
	expect(is_nonzero(me.from('y'))).toEqual(undefined);

	me.clear_assumptions();
	me.add_assumption(me.from("x != 0"));
	me.add_assumption(me.from("not(y = x)"));
	expect(is_nonzero(me.from('y'))).toEqual(undefined);

	me.clear_assumptions();
	me.add_assumption(me.from("x != 0"));
	me.add_assumption(me.from("y = x"));
	expect(is_nonzero(me.from('y'))).toEqual(true);

	me.clear_assumptions();
	me.add_assumption(me.from("x != 0"));
	me.add_assumption(me.from("not(y != x)"));
	expect(is_nonzero(me.from('y'))).toEqual(true);
	
	me.clear_assumptions();
	me.add_assumption(me.from("x elementof R"));
	me.add_assumption(me.from("y != x"));
	expect(is_real(me.from('y'))).toEqual(undefined);

	me.clear_assumptions();
	me.add_assumption(me.from("x elementof R"));
	me.add_assumption(me.from("not(y = x)"));
	expect(is_real(me.from('y'))).toEqual(undefined);

	me.clear_assumptions();
	me.add_assumption(me.from("x elementof R"));
	me.add_assumption(me.from("y = x"));
	expect(is_real(me.from('y'))).toEqual(true);

	me.clear_assumptions();
	me.add_assumption(me.from("x elementof R"));
	me.add_assumption(me.from("not(y != x)"));
	expect(is_real(me.from('y'))).toEqual(true);
	
	me.clear_assumptions();
	me.add_assumption(me.from("x elementof Z"));
	me.add_assumption(me.from("y != x"));
	expect(is_integer(me.from('y'))).toEqual(undefined);

	me.clear_assumptions();
	me.add_assumption(me.from("x elementof Z"));
	me.add_assumption(me.from("not(y = x)"));
	expect(is_integer(me.from('y'))).toEqual(undefined);

	me.clear_assumptions();
	me.add_assumption(me.from("x elementof Z"));
	me.add_assumption(me.from("y = x"));
	expect(is_integer(me.from('y'))).toEqual(true);

	me.clear_assumptions();
	me.add_assumption(me.from("x elementof Z"));
	me.add_assumption(me.from("not(y != x)"));
	expect(is_integer(me.from('y'))).toEqual(true);
	
	me.clear_assumptions();

    });

	//negated with and
    
    var assumptions_negated = [
	['not (x elementof R)',
	 false, false, false, false, undefined, false, undefined],
	['not (x notelementof R)',
	 undefined, undefined, undefined, undefined, undefined, true, true],
	['not (x notelementof C)',
	 undefined, undefined, undefined, undefined, undefined, undefined, true],
	['not (x elementof C)',
	 undefined, undefined, undefined, undefined, undefined, undefined, false],
	['not(x != 0)',
	 true, false, true, false, false, true, true],
	['not(not(x != 0))',
	 undefined, undefined, undefined, undefined, true, undefined, undefined],
	['not(x > 0)',
	 undefined, undefined, true, false, undefined, true, true],
	['not(not(x > 0))',
	 true, false, false, true, true, true, true, true],
	['not(x >= 0)',
	 false, true, true, false, true, true, true],
	['not(not(x >= 0))',
	 true, false, undefined, undefined, undefined, true, true],
	['not(x < 0)',
	 true, false, undefined, undefined, undefined, true, true],
	['not(not(x < 0))',
	 false, true, true, false, true, true, true],
	['not(x <= 0)',
	 true, false, false, true, true, true, true, true],
	['not(not(x <= 0))',
	 undefined, undefined, true, false, undefined, true, true],
	['not(x = 0)',
	 undefined, undefined, undefined, undefined, true, undefined, undefined],
	['not(not(x = 0))',
	 true, false, true, false, false, true, true],
	
    ];	
	
    assumptions_negated.forEach(function(input) {
	it("assumptions: " + input, function() {
	    me.clear_assumptions();
	    me.add_assumption(me.from(input[0]));
	    expect(is_nonnegative(me.fromText('x'))).toEqual(input[1]);
	    expect(is_negative(me.fromText('x'))).toEqual(input[2]);
	    expect(is_nonpositive(me.fromText('x'))).toEqual(input[3]);
	    expect(is_positive(me.fromText('x'))).toEqual(input[4]);
	    expect(is_nonzero(me.fromText('x'))).toEqual(input[5]);
	    expect(is_real(me.fromText('x'))).toEqual(input[6]);
	    expect(is_complex(me.fromText('x'))).toEqual(input[7]);
	});
    });


    it("negative to power of integer", function () {

	me.clear_assumptions();
	me.add_assumption(me.from('x elementof Z'));
	me.add_assumption(me.from('y < 0'));
	expect(is_real(me.from('y^x'))).toEqual(true);
	expect(is_positive(me.from('y^x'))).toEqual(undefined);
	expect(is_positive(me.from('y^2'))).toEqual(true);
	expect(is_nonnegative(me.from('y^x'))).toEqual(undefined);
	expect(is_nonnegative(me.from('y^2'))).toEqual(true);

	expect(is_positive(me.from('y^(2x)'))).toEqual(true);
	expect(is_nonnegative(me.from('y^(2x)'))).toEqual(true);
	me.clear_assumptions();
	
	me.clear_assumptions();
	me.add_assumption(me.from('x elementof Z'));
	me.add_assumption(me.from('y <= 0'));
	expect(is_real(me.from('y^x'))).toEqual(undefined);
	expect(is_positive(me.from('y^2'))).toEqual(undefined);
	expect(is_nonnegative(me.from('y^2'))).toEqual(true);

	me.add_assumption(me.from('x > 0'));
	expect(is_real(me.from('y^x'))).toEqual(true);
	
	expect(is_positive(me.from('y^(2x)'))).toEqual(undefined);
	expect(is_nonnegative(me.from('y^(2x)'))).toEqual(true);
	me.clear_assumptions();
	
    });


		// although this passes, skip test as setting define_i, etc.,
		// no longer changes mathjs, so doesn't change it everywhere.
    it.skip("define constants", function () {
	me.clear_assumptions();
	me.add_assumption(me.from('x elementof R'));
	expect(is_nonzero(me.from('i'))).toEqual(true);
	expect(is_real(me.from('ix'))).toEqual(false);
	expect(is_complex(me.from('ix'))).toEqual(true);

	me.math.define_i = false;
	expect(is_nonzero(me.from('i'))).toEqual(undefined);
	expect(is_real(me.from('ix'))).toEqual(undefined);
	expect(is_complex(me.from('ix'))).toEqual(undefined);
	
	me.math.define_i = true;

	me.clear_assumptions();
	me.add_assumption(me.from('x > 0'));
	expect(is_positive(me.from('pi*x'))).toEqual(true);
	expect(is_real(me.from('pi*x'))).toEqual(true);

	me.math.define_pi = false;
	expect(is_positive(me.from('pi*x'))).toEqual(undefined);
	expect(is_real(me.from('pi*x'))).toEqual(undefined);
	
	me.math.define_pi = true;

	expect(is_positive(me.from('x*e^x'))).toEqual(true);
	expect(is_positive(me.from('x*exp(x)'))).toEqual(true);

	me.math.define_e = false;
	expect(is_positive(me.from('x*e^x'))).toEqual(undefined);
	expect(is_positive(me.from('x*exp(x)'))).toEqual(true);
	
	me.math.define_e = true;
	me.clear_assumptions();
	
	
    });
    
    it("strict pow", function () {
	me.clear_assumptions();
	expect(is_nonzero(me.from('0^0'))).toEqual(undefined);
	expect(is_real(me.from('0^0'))).toEqual(false);
	expect(is_complex(me.from('0^0'))).toEqual(false);
	expect(is_nonzero(me.from('(0/0)^0'))).toEqual(undefined);
	expect(is_real(me.from('(0/0)^0'))).toEqual(false);
	expect(is_complex(me.from('(0/0)^0'))).toEqual(false);
	expect(is_nonzero(me.from('(1/0)^0'))).toEqual(undefined);
	expect(is_real(me.from('(1/0)^0'))).toEqual(false);
	expect(is_complex(me.from('(1/0)^0'))).toEqual(false);
	
	me.math.pow_strict = false;
	expect(is_nonzero(me.from('0^0'))).toEqual(true);
	expect(is_real(me.from('0^0'))).toEqual(true);
	expect(is_complex(me.from('0^0'))).toEqual(true);
	expect(is_nonzero(me.from('(0/0)^0'))).toEqual(true);
	expect(is_real(me.from('(0/0)^0'))).toEqual(true);
	expect(is_complex(me.from('(0/0)^0'))).toEqual(true);
	expect(is_nonzero(me.from('(1/0)^0'))).toEqual(true);
	expect(is_real(me.from('(1/0)^0'))).toEqual(true);
	expect(is_complex(me.from('(1/0)^0'))).toEqual(true);

	me.math.pow_strict = true;
	
    });


    it("logical combinations", function () {
	me.clear_assumptions();
	me.add_assumption(me.from("x elementof R and x notelementof R"))
	expect(is_real(me.from('x'))).toEqual(true);

	me.clear_assumptions();
	me.add_assumption(me.from("x < 0 or x > 5"))
	expect(is_nonzero(me.from("x"))).toEqual(true);
	expect(is_real(me.from("x"))).toEqual(true);
	expect(is_positive(me.from("x"))).toEqual(undefined);

	me.clear_assumptions();
	me.add_assumption(me.from("not (x elementof [0,5])"))
	expect(is_nonzero(me.from("x"))).toEqual(true);
	expect(is_real(me.from("x"))).toEqual(true);
	expect(is_positive(me.from("x"))).toEqual(undefined);
	
	me.clear_assumptions();
	me.add_assumption(me.from("x > 0 or (x elementof (4,8))"))
	expect(is_nonzero(me.from("x"))).toEqual(true);
	expect(is_real(me.from("x"))).toEqual(true);
	expect(is_positive(me.from("x"))).toEqual(true);
	
	me.clear_assumptions();
	me.add_assumption(me.from("x >= 0 or (x elementof (4,8))"))
	expect(is_nonzero(me.from("x"))).toEqual(undefined);
	expect(is_real(me.from("x"))).toEqual(true);
	expect(is_positive(me.from("x"))).toEqual(undefined);
	expect(is_nonnegative(me.from("x"))).toEqual(true);
	
	me.clear_assumptions();
	me.add_assumption(me.from("x=0 or x=1 or x=2 or x=3 or x=4 or x=5"))
	expect(is_nonzero(me.from("x"))).toEqual(undefined);
	expect(is_real(me.from("x"))).toEqual(true);
	expect(is_positive(me.from("x"))).toEqual(undefined);
	expect(is_nonnegative(me.from("x"))).toEqual(true);
	expect(is_integer(me.from("x"))).toEqual(true);
	
	me.clear_assumptions();
	me.add_assumption(me.from("x > 2 and (x < 7 or x > 8)"))
	expect(is_nonzero(me.from("x"))).toEqual(true);
	expect(is_real(me.from("x"))).toEqual(true);
	expect(is_positive(me.from("x"))).toEqual(true);
	expect(is_nonnegative(me.from("x"))).toEqual(true);
	
	me.clear_assumptions();
	me.add_assumption(me.from("x < 7 and (x > 2 or x < -8)"))
	expect(is_nonzero(me.from("x"))).toEqual(true);
	expect(is_real(me.from("x"))).toEqual(true);
	expect(is_positive(me.from("x"))).toEqual(undefined);
	expect(is_nonnegative(me.from("x"))).toEqual(undefined);
	
	me.clear_assumptions();
	me.add_assumption(me.from("x elementof C and x notelementof R"))
	me.add_assumption(me.from("y elementof R"));
	expect(is_real(me.from("x"))).toEqual(false);
	expect(is_complex(me.from("x"))).toEqual(true);
	expect(is_positive(me.from("x"))).toEqual(false);
	expect(is_negative(me.from("x"))).toEqual(false);
	expect(is_nonpositive(me.from("x"))).toEqual(false);
	expect(is_nonnegative(me.from("x"))).toEqual(false);
	expect(is_real(me.from("xy"))).toEqual(false);
	expect(is_complex(me.from("xy"))).toEqual(true);
	expect(is_positive(me.from("xy"))).toEqual(false);
	expect(is_negative(me.from("xy"))).toEqual(false);
	expect(is_nonpositive(me.from("xy"))).toEqual(false);
	expect(is_nonnegative(me.from("xy"))).toEqual(false);

	me.clear_assumptions();

    });


    var function_tests = [
	["sin(x)", "x elementof R",
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
	["sin(x)", "x elementof C",
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	["sqrt(x)", "x elementof C",
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	["sqrt(x)", "x elementof R",
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	["sqrt(x)", "x >=0",
	 true, false, undefined, undefined, undefined,
	 true, true],
	["sqrt(x)", "x >0",
	 true, false, false, true, true,
	 true, true],
	["exp(x)", "x elementof C",
	 undefined, undefined, undefined, undefined, true,
	 undefined, true],
	["exp(x)", "x elementof R",
	 true, false, false, true, true,
	 true, true],
	["exp(x)", "x >=0",
	 true, false, false, true, true,
	 true, true],
	["exp(x)", "x >0",
	 true, false, false, true, true,
	 true, true],
	["abs(x)", "x elementof C",
	 true, false, undefined, undefined, undefined,
	 true, true],
	["abs(x)", "x elementof R",
	 true, false, undefined, undefined, undefined,
	 true, true],
	["abs(x)", "x >=0",
	 true, false, undefined, undefined, undefined,
	 true, true],
	["abs(x)", "x >0",
	 true, false, false, true, true,
	 true, true],
	["abs(x)", "x != 0 and x elementof C",
	 true, false, false, true, true,
	 true, true],
	["abs(x)", "x != 0 and x elementof R",
	 true, false, false, true, true,
	 true, true],
	["log(x)", "x elementof C",
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	["log(x)", "x elementof R",
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	["log(x)", "x >=0",
	 undefined, undefined, undefined, undefined, undefined,
	 undefined, true],
	["log(x)", "x >0",
	 undefined, undefined, undefined, undefined, undefined,
	 true, true],
    ]
    
    function_tests.forEach(function(input) {
	it("function tests: " + input, function() {
	    me.clear_assumptions();
	    me.add_assumption(me.from(input[1]));
	    expect(is_nonnegative(me.fromText(input[0]))).toEqual(input[2]);
	    expect(is_negative(me.fromText(input[0]))).toEqual(input[3]);
	    expect(is_nonpositive(me.fromText(input[0]))).toEqual(input[4]);
	    expect(is_positive(me.fromText(input[0]))).toEqual(input[5]);
	    expect(is_nonzero(me.fromText(input[0]))).toEqual(input[6]);
	    expect(is_real(me.fromText(input[0]))).toEqual(input[7]);
	    expect(is_complex(me.fromText(input[0]))).toEqual(input[8]);
	    me.clear_assumptions();
	});
    });

    it("sign integer", function() {
	me.clear_assumptions();
	me.add_assumption(me.from('x elementof R'));
	expect(is_integer(me.from('sign(x)'))).toEqual(true);

	me.clear_assumptions();
	me.add_assumption(me.from('x elementof C'));
	expect(is_integer(me.from('sign(x)'))).toEqual(undefined);

	me.clear_assumptions();
    });
    
});


describe("derived assumptions", function () {

    it("combined equality", function () {
	
	me.clear_assumptions();
	me.add_assumption(me.from("x=a"));
	me.add_assumption(me.from("a=b"));
	me.add_assumption(me.from("b=c"));

	expect(ordered_trees_equal(
	    me.get_assumptions("x"),
	    me.fromText("x=a and x=b and x=c").tree)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("a"),
	    me.fromText("a=x and a=b and a=c").tree)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("b"),
	    me.fromText("b=a and b=x and b=c").tree)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("c"),
	    me.fromText("c=a and c=b and c=x").tree)).toBeTruthy();

	me.clear_assumptions();
	
    });
    
    it("combined inequality", function () {

	me.clear_assumptions();
	me.add_assumption(me.from("x<a"));
	me.add_assumption(me.from("a<b"));
	me.add_assumption(me.from("b<c"));

	expect(ordered_trees_equal(
	    me.get_assumptions("x"),
	    me.fromText("x<a and x<b and x<c").tree)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("a"),
	    me.fromText("a>x and a<b and a<c").tree)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("b"),
	    me.fromText("b>x and b>a and b<c").tree)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("c"),
	    me.fromText("c>x and c>a and c>b").tree)).toBeTruthy();

	me.clear_assumptions();
	me.add_assumption(me.from("x<a"));
	me.add_assumption(me.from("a<b"));
	me.add_assumption(me.from("b<=c"));

	expect(ordered_trees_equal(
	    me.get_assumptions("x"),
	    me.fromText("x<a and x<b and x<c").tree)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("a"),
	    me.fromText("a>x and a<b and a<c").tree)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("b"),
	    me.fromText("b>x and b>a and b<=c").tree)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("c"),
	    me.fromText("c>x and c>a and c >=b").tree)).toBeTruthy();

	me.clear_assumptions();
	me.add_assumption(me.from("x<a"));
	me.add_assumption(me.from("a<=b"));
	me.add_assumption(me.from("b<=c"));

	expect(ordered_trees_equal(
	    me.get_assumptions("x"),
	    me.fromText("x<a and x<b and x<c").tree)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("a"),
	    me.fromText("a>x and a<=b and a<=c").tree)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("b"),
	    me.fromText("b>x and b>=a and b<=c").tree)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("c"),
	    me.fromText("c>x and c>=a and c >=b").tree)).toBeTruthy();

	me.clear_assumptions();
	me.add_assumption(me.from("x<a"));
	me.add_assumption(me.from("a>=b"));
	me.add_assumption(me.from("b<=c"));

	expect(ordered_trees_equal(
	    me.get_assumptions("x"),
	    me.fromText("x<a").tree)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("a"),
	    me.fromText("a>x and a>=b").tree)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("b"),
	    me.fromText("b<=a and b<=c").tree)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("c"),
	    me.fromText("c >=b").tree)).toBeTruthy();

	me.clear_assumptions();
	
    });

    it("combine containment", function () {

	me.clear_assumptions();
	me.add_assumption(me.fromText("x elementof A"));
	me.add_assumption(me.fromText("A subset B"));
	expect(ordered_trees_equal(
	    me.get_assumptions("x"),
	    me.fromText("x elementof A and x elementof B").tree
	)).toBeTruthy();
	me.add_assumption(me.fromText("B subset C"));
	expect(ordered_trees_equal(
	    me.get_assumptions("x"),
	    me.fromText("x elementof A and x elementof B and x elementof C").tree
	)).toBeTruthy();

	me.clear_assumptions();
	me.add_assumption(me.fromText("x notelementof A"));
	me.add_assumption(me.fromText("A superset B"));
	expect(ordered_trees_equal(
	    me.get_assumptions("x"),
	    me.fromText("x notelementof A and x notelementof B").tree
	)).toBeTruthy();
	me.add_assumption(me.fromText("C subset B"));
	expect(ordered_trees_equal(
	    me.get_assumptions("x"),
	    me.fromText("x notelementof A and x notelementof B and x notelementof C").tree
	)).toBeTruthy();

	me.clear_assumptions();
	me.add_assumption(me.fromText("x elementof A"));
	me.add_assumption(me.fromText("x notelementof B"));
	expect(ordered_trees_equal(
	    me.get_assumptions("A"),
	    me.fromText("x elementof A and A notsubset B").tree
	)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("B"),
	    me.fromText("x notelementof B and A notsubset B").tree
	)).toBeTruthy();
	me.add_assumption(me.fromText("B superset C"));
	expect(ordered_trees_equal(
	    me.get_assumptions("A"),
	    me.fromText("x elementof A and A notsubset B and A notsubset C").tree
	)).toBeTruthy();

	me.clear_assumptions();
	me.add_assumption(me.fromText("A subset B"));
	me.add_assumption(me.fromText("B subset C"));
	me.add_assumption(me.fromText("B superset D"));
	me.add_assumption(me.fromText("x notelementof B"));
	me.add_assumption(me.fromText("E notsubset B"));
	expect(ordered_trees_equal(
	    me.get_assumptions("A"),
	    me.fromText("A subset B and A subset C and x notelementof A and E notsubset A").tree
	)).toBeTruthy();

	me.clear_assumptions();
	me.add_assumption(me.fromText("A notsubset B"));
	me.add_assumption(me.fromText("B superset C"));
	expect(ordered_trees_equal(
	    me.get_assumptions("A"),
	    me.fromText("A notsubset B and A notsubset C").tree
	)).toBeTruthy();

	me.clear_assumptions();
	me.add_assumption(me.fromText("A superset B"));
	me.add_assumption(me.fromText("B superset C"));
	me.add_assumption(me.fromText("B subset D"));
	me.add_assumption(me.fromText("x elementof B"));
	me.add_assumption(me.fromText("E notsuperset B"));
	expect(ordered_trees_equal(
	    me.get_assumptions("A"),
	    me.fromText("A superset B and A superset C and x elementof A and E notsuperset A").tree
	)).toBeTruthy();

	me.clear_assumptions();
	me.add_assumption(me.fromText("A notsuperset B"));
	me.add_assumption(me.fromText("B subset C"));
	expect(ordered_trees_equal(
	    me.get_assumptions("A"),
	    me.fromText("A notsuperset B and A notsuperset C").tree
	)).toBeTruthy();

	me.clear_assumptions();
	
    });
    
    it("retrieve additional", function () {

	me.clear_assumptions();

	me.add_assumption(me.fromText("x > a"));
	me.add_assumption(me.fromText("a^2 > 2"));
	expect(ordered_trees_equal(
	    me.get_assumptions("x"),
	    me.fromText("x > a and a^2 > 2").tree
	)).toBeTruthy();

	me.clear_assumptions();
	me.add_assumption(me.fromText("x > a"));
	me.add_assumption(me.fromText("a^2 + y^2> 2"));
	me.add_assumption(me.fromText("exp(y) > y"));
	expect(ordered_trees_equal(
	    me.get_assumptions("x"),
	    me.fromText("x > a and a^2 + y^2 > 2 and exp(y) > y").tree
	)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("a"),
	    me.fromText("x > a and a^2 + y^2 > 2 and exp(y) > y").tree
	)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("y"),
	    me.fromText("x > a and a^2 + y^2 > 2 and exp(y) > y").tree
	)).toBeTruthy();

	me.clear_assumptions();
	me.add_assumption(me.from("x > y^3"));
	me.add_assumption(me.from("y>z"));
	me.add_assumption(me.from("z>0"));
	expect(ordered_trees_equal(
	    me.get_assumptions("x"),
	    me.fromText("x > y^3 and y>z and y > 0").tree
	)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("y"),
	    me.fromText("x > y^3 and y>z and y > 0").tree
	)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("z"),
	    me.fromText("x > y^3 and y>z and z > 0").tree
	)).toBeTruthy();

	me.clear_assumptions();
	
	
    });


    it("don't retrieve additional", function () {

	me.clear_assumptions();
	me.add_assumption(me.fromText("a > b"));
	me.add_assumption(me.fromText("c > b"));
	expect(ordered_trees_equal(
	    me.get_assumptions("a"),
	    me.fromText("a>b").tree
	)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("b"),
	    me.fromText("a>b and c>b").tree
	)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("c"),
	    me.fromText("c>b").tree
	)).toBeTruthy();
	

	me.clear_assumptions();
	me.add_assumption(me.fromText("a containselement b"));
	me.add_assumption(me.fromText("c containselement b"));
	expect(ordered_trees_equal(
	    me.get_assumptions("a"),
	    me.fromText("b elementof a").tree
	)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("b"),
	    me.fromText("b elementof a and b elementof c").tree
	)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions("c"),
	    me.fromText("b elementof c").tree
	)).toBeTruthy();
	
	me.clear_assumptions();
	
    });
    
    
});

describe("assumptions on expressions", function () {

    it("linear inequality", function () {

	me.clear_assumptions();
	
	me.add_assumption(me.from("q > x"));
	expect(ordered_trees_equal(
	    me.get_assumptions("q"),
	    me.fromText("q > x").tree
	)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions(me.from("q-x")),
	    me.fromText("q-x > 0").tree
	)).toBeTruthy();
	expect(ordered_trees_equal(
	    me.get_assumptions(me.from("q-x").tree),
	    me.fromText("q-x > 0").tree
	)).toBeTruthy();

	me.add_assumption(me.from("a < x"));
	
	expect(ordered_trees_equal(
	    me.get_assumptions(me.from("q-a").tree),
	    me.fromText("q-a > 0 and q-a > q-x and q-a > x-a").tree
	)).toBeTruthy();

	
	me.add_assumption(me.from("a elementof Z"));
	expect(ordered_trees_equal(
	    me.get_assumptions(me.from("q-a").tree),
	    me.fromText("q-a > 0 and q-a > q-x and q-a > x-a and a elementof Z").tree
	)).toBeTruthy();


	me.clear_assumptions();
	me.add_assumption(me.from("3a+4b > 2c+6d"));
	me.add_assumption(me.from("c+3d > 0"));
	expect(ordered_trees_equal(
	    me.from(me.get_assumptions(me.from("3a+4b").tree)).tree,
	    me.fromText("3a+4b > 0 and 3a+4b > 2c+6d and -3d < c and c/-3 < d").evaluate_numbers().tree
	)).toBeTruthy();
	
	me.clear_assumptions();
	me.add_assumption(me.from('a+b< 1'));
	expect(ordered_trees_equal(
	    me.from(me.get_assumptions(me.from("abc").tree)).tree,
	    me.fromText("a < 1-b and b < 1-a").tree
	)).toBeTruthy();
	
	
	
    });

});

// find example where must recurse on normalize_assumptions
