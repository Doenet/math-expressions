import me from '../lib/math-expressions';

describe("basic arithmetic operions", function () {

    it("addition", function () {
	expect(me.fromText('a+b').add(me.fromText('c+b')).simplify().tree).toEqual(
	    me.fromText('a+2b+c').simplify().tree)
    });
    
    it("subtraction", function () {
	expect(me.from('a+b').subtract(me.from('c+b')).simplify().tree).toEqual(
	    me.from('a-c').simplify().tree)
    });

    it("multiplication", function () {
	expect(me.from('a+b').multiply(me.from('c+b')).simplify().tree).toEqual(
	    me.from('(a+b)(c+b)').simplify().tree)
    });
    it("division", function () {
	expect(me.from('a+b').divide(me.from('c+b')).simplify().tree).toEqual(
	    me.from('(a+b)/(c+b)').simplify().tree)
    });
    it("power", function () {
	expect(me.from('a+b').pow(me.from('c+b')).simplify().tree).toEqual(
	    me.from('(a+b)^(c+b)').simplify().tree)
    });
    it("modulo", function () {
	expect(me.from('a+b').mod(me.from('c+b')).simplify().tree).toEqual(
	    ['apply', 'mod', ['tuple', me.from('(a+b)').simplify().tree, me.from('(c+b)').simplify().tree]]);
    });
    

});
   

