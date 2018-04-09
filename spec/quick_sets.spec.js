import me from '../lib/math-expressions';

describe("discrete infinite", function () {

    it("basic equality", function () {
	
	var set1 = me.create_discrete_infinite_set(
	    me.fromText("pi/4"), me.fromText("pi"));
	var set2 = me.create_discrete_infinite_set(
	    me.fromText("pi/4, 5pi/4"), me.fromText("2pi"));
	expect(set1.equals(set2)).toBeTruthy();
	
	set1 = me.create_discrete_infinite_set(
	    me.fromText("pi/4"), me.fromText("2*pi"));
	expect(set1.equals(set2)).toBeFalsy();
	
	set1 = me.create_discrete_infinite_set(
	    me.fromText("9*pi/4"), me.fromText("pi"));
	expect(set1.equals(set2)).toBeTruthy();

	set1 = me.create_discrete_infinite_set(
	    me.fromText("7*pi/4"), me.fromText("pi"));
	expect(set1.equals(set2)).toBeFalsy();


    });


    it("overcounting", function () {

	var set1 = me.create_discrete_infinite_set(
	    me.fromText("1"), me.fromText("5"));
	var set2 = me.create_discrete_infinite_set(
	    me.fromText("1, 1, 6, 11, 16, 21"), me.fromText("10"));
	expect(set1.equals(set2)).toBeTruthy();

	set2 = me.create_discrete_infinite_set(
	    me.fromText("1, 1, 6, 11, 16, 22"), me.fromText("10"));
	expect(set1.equals(set2)).toBeFalsy();
    });
    
    it("variables", function () {

	var set1 = me.create_discrete_infinite_set(
	    me.fromText("a"), me.fromText("3"));
	var set2 = me.create_discrete_infinite_set(
	    me.fromText("a, a+3, a+6"), me.fromText("9"));
	expect(set1.equals(set2)).toBeTruthy();

	set1 = me.create_discrete_infinite_set(
	    me.fromText("b"), me.fromText("3"));
	expect(set1.equals(set2)).toBeFalsy();
	
    });

    it("assumptions", function () {

	me.clear_assumptions();
	var set1 = me.create_discrete_infinite_set(
	    me.fromText("a"), me.fromText("c"));
	var set2 = me.create_discrete_infinite_set(
	    me.fromText("a, a+c"), me.fromText("2c"));

	expect(set1.equals(set2)).toBeFalsy();
	
	me.add_assumption(me.from("c != 0"));
	expect(set1.equals(set2)).toBeTruthy();
	
    });
    
    
});
