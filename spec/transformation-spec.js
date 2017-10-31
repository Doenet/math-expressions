var me = require('../lib/math-expressions');
var trees = require('../lib/trees');

describe("expand factors", function () {

    it("expand polynomial", function () {

	expect(trees.equal(me.fromText("(a+x)(b-y)").expand().tree, me.fromText("ab-ay+xb-xy").tree)).toBeTruthy();

    });

    it("expand expression", function () {

	var factored = me.fromText("x(sin(x)-cos(y))(3log(z)+be^a)(ts+q^2)");
	var expanded = me.fromText("x sin(x) 3 log(z) ts + x sin(x) 3 log(z) q^2 + x sin(x) be^a ts + x sin(x) be^a q^2  - x cos(y) 3 log(z) ts - x cos(y) 3 log(z) q^2 - x cos(y) be^a ts - x cos(y) be^a q^2");
	var factored_expanded = factored.expand();
	expect(trees.equal(factored_expanded.tree,expanded.tree)).toBeTruthy();

    });

    it("expand negative", function () {
	expect(trees.equal(me.fromText('-(x+y)').expand().tree,
			   me.fromText('-x-y').tree)).toBeTruthy();
    });

});
