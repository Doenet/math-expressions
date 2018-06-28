import me from '../lib/math-expressions';
import { equal as tree_equal } from '../lib/trees/basic';

describe("expand factors", function () {

  test("expand polynomial", function () {
    expect(tree_equal(me.fromText("(a+x)(b-y)").expand().tree, me.fromText("ab-ay+xb-xy").tree)).toBeTruthy();
    expect(tree_equal(me.fromText("(a-x)^2").expand().tree, me.fromText("a^2 -2ax + x^2").evaluate_numbers().tree)).toBeTruthy();
    expect(tree_equal(me.fromText("(a-x+c)^2").expand().tree, me.fromText("a^2 +x^2+c^2 -2ax +2ac-2xc").evaluate_numbers().tree)).toBeTruthy();
    expect(tree_equal(me.fromText("(a-x)^3").expand().tree, me.fromText("a^3 -3a^2x + 3ax^2-x^3").evaluate_numbers().tree)).toBeTruthy();
    expect(tree_equal(me.fromText("(a-x+c)^3").expand().tree, me.fromText("a^3 -x^3+c^3 -3a^2x +3a^2c+3ax^2+3x^2c+3ac^2-3xc^2-6axc").evaluate_numbers().tree)).toBeTruthy();

  });

  test("expand expression", function () {

    var factored = me.fromText("x(sin(x)-cos(y))(3log(z)+be^a)(ts+q^2)");
    var expanded = me.fromText("x sin(x) 3 log(z) ts + x sin(x) 3 log(z) q^2 + x sin(x) be^a ts + x sin(x) be^a q^2  - x cos(y) 3 log(z) ts - x cos(y) 3 log(z) q^2 - x cos(y) be^a ts - x cos(y) be^a q^2");
    var factored_expanded = factored.expand();
    expect(tree_equal(factored_expanded.tree,expanded.evaluate_numbers().tree)).toBeTruthy();

  });

  test("expand negative", function () {
    expect(tree_equal(me.fromText('-(x+y)').expand().tree,
		      me.fromText('-x-y').evaluate_numbers().tree)).toBeTruthy();
  });

});


describe("expand relations", function () {

  test("equality", function () {
    expect(tree_equal(me.fromText('a=b=c').expand_relations().tree,
		      me.fromText('a=b and b=c').tree)).toBeTruthy();

    expect(tree_equal(me.fromText('1+3/x=x-y=c^2q=log(z)').expand_relations().tree,
		      me.fromText('1+3/x=x-y and x-y=c^2q and c^2q=log(z)').tree)).toBeTruthy();

  });
  
  test("inequality", function () {
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

test("substitution", function () {

  let expr = me.fromText('2x^2+3y')
  let expr2 = expr.substitute({x: me.fromText('z^2')});
  let expr2a = me.fromText('2(z^2)^2+3y')

  expect(tree_equal(expr2.tree, expr2a.tree)).toBeTruthy();

  let expr3 = expr.substitute({y: 5});
  let expr3a = me.fromText('2x^2+3*5')
  
  expect(tree_equal(expr3.tree, expr3a.tree)).toBeTruthy();

  let expr4= me.substitute(expr, {x: ['*', 4, 'q']});
  let expr4a = me.fromText('2(4q)^2+3y')
  
  expect(tree_equal(expr4.tree, expr4a.tree)).toBeTruthy();
  
});
