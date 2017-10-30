var me = require('../lib/math-expressions');

describe("expand factors", function () {

    it("expand polynomial", function () {

	expect(me.fromText("(a+x)(b-y)").expand().equals(me.fromText("ab-ay+xb-xy"))).toBeTruthy();

    });

    it("expand expression", function () {

	var factored = me.fromText("x(sin(x)-cos(y))(3log(z)+be^a)(ts+q^2)");
	var expanded = me.fromText("x sin(x) 3 log(z) ts + x sin(x) 3 log(z) q^2 + x sin(x) be^a ts + x sin(x) be^a q^2  - x cos(y) 3 log(z) ts - x cos(y) 3 log(z) q^2 - x cos(y) be^a ts - x cos(y) be^a q^2");
	var factored_expanded = factored.expand();
	console.log("About to check equality of complicated expression");
	expect(factored_expanded.equals(expanded)).toBeTruthy();
	console.log("Finished checking equality of complicated expression");

    });

});
