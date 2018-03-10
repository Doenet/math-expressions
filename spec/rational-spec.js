var me = require('../lib/math-expressions');
var trees = require('../lib/trees/basic');
var rational = require('../lib/expression/rational');
var simplify = require('../lib/expression/simplify');

describe("reduce rational expressions", function () {

    it("reduce rational of polynomials", function () {
	var f1 = me.from('a+b');
	var f2 = me.from('c+d');
	var f3 = me.from('e+f');
	
	var ratfun = (f1.multiply(f2).expand()).divide(f3.multiply(f2).expand());
	var rat_new = ratfun.reduce_rational().simplify();

	var rat_expect = f1.divide(f3).simplify();
	
	expect(rat_new.tree).toEqual(rat_expect.tree);
    });

    it("reduce rational of transcendentals", function () {
	var f1 = me.from('a+cos(x)');
	var f2 = me.from('c+sin(y)');
	var f3 = me.from('e+atan(z)');
	
	var ratfun = (f1.multiply(f2).expand()).divide(f3.multiply(f2).expand());
	var rat_new = ratfun.reduce_rational().simplify();

	var rat_expect = f1.divide(f3).simplify();
	
	expect(rat_new.tree).toEqual(rat_expect.tree);
    });
});
