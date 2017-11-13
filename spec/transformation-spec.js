var me = require('../lib/math-expressions');
var tree_equal = require('../lib/trees/basic').equal;

describe("expand factors", function () {

    it("expand polynomial", function () {

	expect(tree_equal(me.fromText("(a+x)(b-y)").expand().tree, me.fromText("ab-ay+xb-xy").tree)).toBeTruthy();

    });

    it("expand expression", function () {

	var factored = me.fromText("x(sin(x)-cos(y))(3log(z)+be^a)(ts+q^2)");
	var expanded = me.fromText("x sin(x) 3 log(z) ts + x sin(x) 3 log(z) q^2 + x sin(x) be^a ts + x sin(x) be^a q^2  - x cos(y) 3 log(z) ts - x cos(y) 3 log(z) q^2 - x cos(y) be^a ts - x cos(y) be^a q^2");
	var factored_expanded = factored.expand();
	expect(tree_equal(factored_expanded.tree,expanded.tree)).toBeTruthy();

    });

    it("expand negative", function () {
	expect(tree_equal(me.fromText('-(x+y)').expand().tree,
			   me.fromText('-x-y').tree)).toBeTruthy();
    });

});


describe("expand relations", function () {

    it("equality", function () {
	expect(tree_equal(me.fromText('a=b=c').expand_relations().tree,
			  me.fromText('a=b and b=c').tree)).toBeTruthy();

	expect(tree_equal(me.fromText('1+3/x=x-y=c^2q=log(z)').expand_relations().tree,
			  me.fromText('1+3/x=x-y and x-y=c^2q and c^2q=log(z)').tree)).toBeTruthy();

    });
    
    it("inequality", function () {
	expect(tree_equal(me.fromText('a<b<c').expand_relations().tree,
			  me.fromText('a<b and b<c').tree)).toBeTruthy();
	expect(tree_equal(me.fromText('a<=b<c').expand_relations().tree,
			  me.fromText('a<=b and b<c').tree)).toBeTruthy();
	expect(tree_equal(me.fromText('a<=b<=c').expand_relations().tree,
			  me.fromText('a<=b and b<=c').tree)).toBeTruthy();
	expect(tree_equal(me.fromText('a<b<=c').expand_relations().tree,
			  me.fromText('a<b and b<=c').tree)).toBeTruthy();

	expect(tree_equal(me.fromText('a>b>c').expand_relations().tree,
			  me.fromText('a>b and b>c').tree)).toBeTruthy();
	expect(tree_equal(me.fromText('a>=b>c').expand_relations().tree,
			  me.fromText('a>=b and b>c').tree)).toBeTruthy();
	expect(tree_equal(me.fromText('a>=b>=c').expand_relations().tree,
			  me.fromText('a>=b and b>=c').tree)).toBeTruthy();
	expect(tree_equal(me.fromText('a>b>=c').expand_relations().tree,
			  me.fromText('a>b and b>=c').tree)).toBeTruthy();

	expect(tree_equal(me.fromText('1+3/x<=x-y<c^2q<log(z)').expand_relations().tree,
			  me.fromText('1+3/x<=x-y and x-y<c^2q and c^2q<log(z)').tree)).toBeTruthy();
	expect(tree_equal(me.fromText('1+3/x>=x-y>c^2q>=log(z)').expand_relations().tree,
			  me.fromText('1+3/x>=x-y and x-y>c^2q and c^2q>=log(z)').tree)).toBeTruthy();


    });


});
