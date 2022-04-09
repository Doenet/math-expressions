import me from '../lib/math-expressions';
import { equal as tree_equal } from '../lib/trees/basic';

describe("expand factors", function () {

  test("expand polynomial", function () {
    expect(tree_equal(me.fromText("(a+x)(b-y)").expand().tree, me.fromText("ab-ay+xb-xy").evaluate_numbers().tree)).toBeTruthy();
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

  let expr5 = me.fromAst("x").substitute({x: NaN});
  expect(expr5.tree).toBeNaN();

  let expr6 = me.fromAst("x").substitute({x: Infinity});
  expect(expr6.tree).toBe(Infinity);

  let expr7 = me.fromAst("x").substitute({x: -Infinity});
  expect(expr7.tree).toBe(-Infinity);

  let expr8 = me.fromAst(["+", "x", "y"]).substitute({y: -0});
  let expr8a = me.fromText('x-0')
  expect(tree_equal(expr8.tree, expr8a.tree)).toBeTruthy();

  let expr9 = me.fromAst(["+", "x", ["-", "y"]]).substitute({y: -0});
  let expr9a = me.fromText('x--0')
  expect(tree_equal(expr9.tree, expr9a.tree)).toBeTruthy();

});

test("substitute component", function () {

  let expr = me.fromText('x^2, y, z')
  let expr2 = expr.substitute_component(1, me.fromText('q^2'));
  let expr2a = me.fromText('x^2, q^2, z')

  expect(tree_equal(expr2.tree, expr2a.tree)).toBeTruthy();

  let expr3 = expr.substitute_component(0, 5);
  let expr3a = me.fromText('5, y, z');

  expect(tree_equal(expr3.tree, expr3a.tree)).toBeTruthy();

  let expr4= me.substitute_component(expr, 2, ['*', 4, 'q']);
  let expr4a = me.fromText('x^2, y, 4q');

  expect(tree_equal(expr4.tree, expr4a.tree)).toBeTruthy();

  expr = me.fromText('(a,b,c,d)');
  expr2 = expr.substitute_component(3, me.fromText('e'));
  expr2a = me.fromText('(a,b,c,e)');
  expect(tree_equal(expr2.tree, expr2a.tree)).toBeTruthy();

  expr3 = expr.substitute_component([2], 3);
  expr3a = me.fromText('(a,b,3,d)');
  expect(tree_equal(expr3.tree, expr3a.tree)).toBeTruthy();


  expr = me.fromText('(a,(b0,b1), (c0,(c10,c11, c12), c2), d)');
  expr2 = expr.substitute_component(1, 'x');
  expr2a = me.fromText('(a, x, (c0,(c10,c11, c12), c2), d)');
  expect(tree_equal(expr2.tree, expr2a.tree)).toBeTruthy();

  expr3 = expr.substitute_component([1,0], 'x');
  expr3a = me.fromText('(a,(x,b1), (c0,(c10,c11, c12), c2), d)');
  expect(tree_equal(expr3.tree, expr3a.tree)).toBeTruthy();

  expr4 = expr.substitute_component([2,1,2], 'x');
  expr4a = me.fromText('(a,(b0,b1), (c0,(c10,c11, x), c2), d)');
  expect(tree_equal(expr4.tree, expr4a.tree)).toBeTruthy();

});

test("get component", function () {

  let expr = me.fromText('x^2, y, z')
  let expr2 = expr.get_component(1);
  let expr2a = me.fromText('y')

  expect(tree_equal(expr2.tree, expr2a.tree)).toBeTruthy();

  let expr3 = expr.get_component(0);
  let expr3a = me.fromText('x^2');

  expect(tree_equal(expr3.tree, expr3a.tree)).toBeTruthy();

  let expr4= me.get_component(expr, 2);
  let expr4a = me.fromText('z');

  expect(tree_equal(expr4.tree, expr4a.tree)).toBeTruthy();

  expr = me.fromText('(a,b,c,d)');
  expr2 = expr.get_component(3);
  expr2a = me.fromText('d');
  expect(tree_equal(expr2.tree, expr2a.tree)).toBeTruthy();

  expr3 = expr.get_component([2]);
  expr3a = me.fromText('c');
  expect(tree_equal(expr3.tree, expr3a.tree)).toBeTruthy();


  expr = me.fromText('(a,(b0,b1), (c0,(c10,c11, c12), c2), d)');
  expr2 = expr.get_component(1);
  expr2a = me.fromText('(b0,b1)');
  expect(tree_equal(expr2.tree, expr2a.tree)).toBeTruthy();

  expr3 = expr.get_component([1,0]);
  expr3a = me.fromText('b0');
  expect(tree_equal(expr3.tree, expr3a.tree)).toBeTruthy();

  expr4 = expr.get_component([2,1,2]);
  expr4a = me.fromText('c12');
  expect(tree_equal(expr4.tree, expr4a.tree)).toBeTruthy();

});
