import me from '../lib/math-expressions';
import * as trees from '../lib/trees/basic';

describe("evaluate_numbers", function () {

  it("addition", function () {
    expect(me.from("4+x-2").evaluate_numbers().tree).toEqual(['+', 'x', 2]);
    expect(me.from("4+x-2").evaluate_numbers({ skip_ordering: true }).tree).toEqual(['+', 4, 'x', -2]);

    expect(me.from("x+0").evaluate_numbers().tree).toEqual('x');
    expect(me.from("x+0").evaluate_numbers({ skip_ordering: true }).tree).toEqual('x');

    expect(me.from("Infinity + 3").evaluate_numbers().tree).toEqual(Infinity);
    expect(me.from("Infinity + Infinity").evaluate_numbers().tree).toEqual(Infinity);
    expect(me.from("Infinity - Infinity").evaluate_numbers().tree).toEqual(NaN);

    expect(me.from("1+++3").evaluate_numbers().tree).toEqual(4);
    expect(me.from("1---3").evaluate_numbers().tree).toEqual(-2);

    expect(me.from("3---x+++5").evaluate_numbers().tree).toEqual(["+", ['-', 'x'], 8]);

  });

  it("collapse unary minus", function () {
    expect(me.from("x-2").evaluate_numbers().tree).toEqual(['+', 'x', -2]);
  });

  it("unary minus of product", function () {
    expect(me.from("x-2uv").evaluate_numbers().tree).toEqual(
      ['+', ['*', -2, 'u', 'v'], 'x']);

  });
  it("unary minus of quotient", function () {
    expect(me.from("x-2/(uv)").evaluate_numbers().tree).toEqual(
      ['+', 'x', ['/', -2, ['*', 'u', 'v']]]);
    expect(me.from("x-2u/v").evaluate_numbers().tree).toEqual(
      ['+', ['/', ['*', -2, 'u'], 'v'], 'x']);

  });

  it("negative zero", function () {
    expect(me.from("6/-0").evaluate_numbers().tree).toEqual(-Infinity);
    expect(me.from("-6/-0").evaluate_numbers().tree).toEqual(Infinity);
    expect(Object.is(me.from("-0").evaluate_numbers().tree, 0)).toEqual(true);
  });

  it("multiplication", function () {
    expect(me.from("3*2*x*4").evaluate_numbers().tree).toEqual(
      ['*', 24, 'x']);
    expect(me.from("3*2*x*4").evaluate_numbers({ skip_ordering: true }).tree).toEqual(
      ['*', 6, 'x', 4]);

    expect(me.from("3*2*x*0").evaluate_numbers().tree).toEqual(0);
    expect(me.from("3*2*x*0").evaluate_numbers({ skip_ordering: true }).tree).toEqual(0);

    expect(me.from("(2-1)x").evaluate_numbers().tree).toEqual('x');

    expect(me.from("(-1+2-1)x").evaluate_numbers().tree).toEqual(0);

    expect(me.from("(-1+2-2)x").evaluate_numbers().tree).toEqual(
      ['-', 'x']);

    expect(me.from("4(x)(-2)").evaluate_numbers().tree).toEqual(
      ['*', -8, 'x']);
    expect(me.from("4(x)(-2)").evaluate_numbers({ skip_ordering: true }).tree).toEqual(
      ['*', 4, 'x', -2]);

    expect(me.from("0*Infinity").evaluate_numbers().tree).toEqual(NaN);

    expect(me.from("2*2*2*2*2*2*2*2*2*2*2*2*2*2").evaluate_numbers().tree)
      .toEqual(16384);

    expect(me.from("i*i").evaluate_numbers().tree).toEqual(-1);
    expect(me.from("i*i*i").evaluate_numbers().tree).toEqual(['-', 'i']);

    expect(me.from("3*i*2*i*x*4").evaluate_numbers().evaluate_numbers().tree).toEqual(
      ['*', -24, 'x']);

    expect(me.from("3*i*2*i*x*4*i").evaluate_numbers().evaluate_numbers().tree).toEqual(
      ['*', -24, 'i', 'x']);

  });

  it("division", function () {
    expect(me.from("2x/2").evaluate_numbers().tree).toEqual('x');
    expect(me.from("2/2x").evaluate_numbers().tree).toEqual('x');
    expect(me.from("2/(2x)").evaluate_numbers().tree).toEqual(
      ['/', 1, 'x']);

    expect(me.from("1/0").evaluate_numbers().tree).toEqual(Infinity);
    expect(me.from("0*(1/(0))").evaluate_numbers().tree).toEqual(NaN);
    expect(me.from("1/Infinity").evaluate_numbers().tree).toEqual(0);
    expect(me.from("0/0").evaluate_numbers().tree).toEqual(NaN);

    expect(me.from("2/(0x)").evaluate_numbers().tree).toEqual(Infinity);
    expect(me.from("(2-2)/(0x)").evaluate_numbers().tree).toEqual(NaN);
    expect(me.from("(2-2)*(1/(0x))").evaluate_numbers().tree).toEqual(NaN);

    expect(me.from("(2-2)/(2x)").evaluate_numbers().tree).toEqual(
      ['/', 0, 'x']);

    me.add_assumption(me.from('x > 0'));
    expect(me.from("(2-2)/(2x)").evaluate_numbers().tree).toEqual(0);
    me.clear_assumptions();

    expect(me.fromText('x/2').evaluate_numbers().tree).toEqual(['/', 'x', 2]);
    expect(me.fromText('x/2').evaluate_numbers({ max_digits: 3 }).tree).toEqual(['*', 0.5, 'x']);

    expect(me.fromText('x/3').evaluate_numbers().tree).toEqual(['/', 'x', 3]);
    expect(me.fromText('x/3').evaluate_numbers({ max_digits: 5 }).tree).toEqual(['/', 'x', 3]);
    expect(me.fromText('x/3').evaluate_numbers({ max_digits: Infinity }).tree).toEqual(['*', 1 / 3, 'x']);

    expect(me.fromText('3/i').evaluate_numbers().tree).toEqual(['*', -3, "i"]);

  });

  it("power", function () {
    expect(me.from("x^0").evaluate_numbers().tree).toEqual(['^', 'x', 0]);
    me.add_assumption(me.from('x!= 0'));
    expect(me.from("x^0").evaluate_numbers().tree).toEqual(1);
    me.clear_assumptions();

    expect(me.from("(3-3)^0").evaluate_numbers().tree).toEqual(NaN);

    expect(me.from("(3x-3x)^0").evaluate_numbers().tree).toEqual(NaN);

    expect(me.from("(4-3)^7").evaluate_numbers().tree).toEqual(1);

    expect(me.from("(5-3)^3").evaluate_numbers().tree).toEqual(8);

    expect(me.from("1^t").evaluate_numbers().tree).toEqual(1);

    expect(me.from("1^t 5^x").evaluate_numbers().tree).toEqual(['^', 5, 'x']);

    expect(me.from("i^2").evaluate_numbers().tree).toEqual(-1);
    expect(me.from("i^3").evaluate_numbers().tree).toEqual(['-', 'i']);
    expect(me.from("i^4").evaluate_numbers().tree).toEqual(1);

  });


  it("combination", function () {
    expect(me.from("1x^2-3 +0x^2 + 4 -2x^2 -3 + 5x^2").evaluate_numbers().tree).toEqual(
      ['+', ['*', -2, ['^', 'x', 2]], ['^', 'x', 2], ['*', 5, ['^', 'x', 2]], -2]
    );
    expect(me.from("1x^2-3 +0x^2 + 4 -2x^2 -3 + 5x^2").evaluate_numbers({ skip_ordering: true }).tree).toEqual(
      ['+', ['^', 'x', 2], 1, ['*', -2, ['^', 'x', 2]], -3, ['*', 5, ['^', 'x', 2]]]
    );
  });


  it("to constant", function () {

    expect(me.from("log(e)x+log(1)y").evaluate_numbers().tree).toEqual(
      ["+", ["*", "x", ["apply", "log", "e"]], ["*", "y", ["apply", "log", 1]]]
    );
    expect(me.from("log(e)x+log(1)y").evaluate_numbers({ evaluate_functions: true }).tree).toEqual('x');

    expect(me.from("cos(0)x+sin(pi/2)y").evaluate_numbers().tree).toEqual(
      ["+", ["*", "x", ["apply", "cos", 0]], ["*", "y", ["apply", "sin", ["/", "pi", 2]]]]
    );

    expect(me.from("cos(0)x+sin(pi/2)y").evaluate_numbers({ evaluate_functions: true }).tree).toEqual(
      ['+', 'x', 'y']
    );

    expect(me.from("log(0.5/0.3)").evaluate_numbers({ max_digits: Infinity }).tree).toEqual(
      ["apply", "log", 5 / 3]
    );

    expect(me.from("log(0.5/0.3)").evaluate_numbers({ max_digits: Infinity, evaluate_functions: true }).tree).toEqual(
      me.math.log(5 / 3)
    );

  });

  it("to decimals", function () {

    expect(me.fromText('pi').evaluate_numbers().tree).toEqual('pi');
    expect(me.fromText('pi').evaluate_numbers({ max_digits: Infinity }).tree).toBeCloseTo(Math.PI);
    expect(me.fromText('0.5 pi').evaluate_numbers().tree).toEqual(['*', 0.5, 'pi']);
    expect(me.fromText('0.5 pi').evaluate_numbers({ max_digits: Infinity }).tree).toBeCloseTo(0.5 * Math.PI);
    expect(me.fromText('pi/2').evaluate_numbers().tree).toEqual(['/', 'pi', 2]);
    expect(me.fromText('pi/2').evaluate_numbers({ max_digits: Infinity }).tree).toBeCloseTo(0.5 * Math.PI);
    expect(me.fromText('0.5 7').evaluate_numbers().tree).toEqual(3.5);

    expect(me.fromAst(1 / 3).evaluate_numbers().tree).toBeCloseTo(1 / 3);
    expect(me.fromAst(0.5).evaluate_numbers().tree).toBeCloseTo(1 / 2);
    expect(me.fromAst(['+', 0.5, 1 / 3]).evaluate_numbers().tree).toBeCloseTo(0.5 + 1 / 3);
    expect(me.fromText("1/3").evaluate_numbers().tree).toEqual(["/", 1, 3]);
    expect(me.fromText("1/3").evaluate_numbers({ max_digits: Infinity }).tree).toBeCloseTo(1 / 3);
    expect(me.fromText("1/2").evaluate_numbers().tree).toEqual(["/", 1, 2]);
    expect(me.fromText("1/2").evaluate_numbers({ max_digits: 1 }).tree).toEqual(1 / 2);
    expect(me.fromText("1/2+1/2").evaluate_numbers().tree).toEqual(1);
    expect(me.fromText("1/2+1/3").evaluate_numbers().tree).toEqual(["/", 5, 6]);
    expect(me.fromText("1/2+1/3").evaluate_numbers({ max_digits: Infinity }).tree).toBeCloseTo(5 / 6);

  })


  it("set small zero", function () {

    expect(me.fromText('10x+5E-15').evaluate_numbers({ set_small_zero: true }).tree).toEqual(['*', 10, 'x']);
    expect(me.fromText('10x+5E-15').evaluate_numbers().tree).toEqual(
      ['+', ['*', 10, 'x'], 5E-15]);
    expect(me.fromText('10x+5E-15').evaluate_numbers({ set_small_zero: 2E-15 }).tree).toEqual(
      ['+', ['*', 10, 'x'], 5E-15]);
    expect(me.fromText('10x+5E-15').evaluate_numbers({ set_small_zero: 1E-13 }).tree).toEqual(['*', 10, 'x']);
    let tree = me.fromText('0.0001^4x').evaluate_numbers().tree;
    expect(tree.length).toEqual(3);
    expect(tree[0]).toEqual('*');
    expect(tree[1]).toBeCloseTo(1E-16);
    expect(tree[2]).toEqual('x');
    expect(me.fromText('0.0001^4x').evaluate_numbers({ set_small_zero: true }).tree).toEqual(0);

    expect(me.fromText('sin(pi)x').evaluate_numbers(
      { evaluate_functions: true, max_digits: Infinity, set_small_zero: true }
    ).tree).toEqual(0);

  })


  it("negative zero becomes zero", function () {

    expect(Object.is(me.fromText('-0').tree, 0)).toBeFalsy();
    expect(Object.is(me.fromText('-0').evaluate_numbers().tree, 0)).toBeTruthy();

  })

  it("can get negative infinity by reciprocal of negative zero", function () {

    expect(me.fromText("1/((-1)(0))").evaluate_numbers().tree).toEqual(-Infinity);

  })


  it("with blanks not evaluated", function () {
    expect(me.fromText("1+2+").evaluate_numbers().tree).toEqual(['+', 1, 2, '\uff3f']);
  })

});

describe("evaluate_to_constant", function () {

  it("check pi e", function () {
    expect(me.from("pi").evaluate_to_constant()).toBeCloseTo(Math.PI);
    expect(me.from("2pi").evaluate_to_constant()).toBeCloseTo(2 * Math.PI);
    expect(me.from("e").evaluate_to_constant()).toBeCloseTo(Math.E);
    expect(me.from("2e").evaluate_to_constant()).toBeCloseTo(2 * Math.E);

  });

  it("with blanks gives null", function () {
    expect(me.fromText("1+2+").evaluate_to_constant()).toEqual(null);
  })

  it("multiple pluses and minus", function () {
    expect(me.fromText("1+++2").evaluate_to_constant()).toEqual(3);
    expect(me.fromText("1+-+2").evaluate_to_constant()).toEqual(-1);
    expect(me.fromText("1---2").evaluate_to_constant()).toEqual(-1);
    expect(me.fromText("1-+-2").evaluate_to_constant()).toEqual(3);
  })

});

describe("collect like terms and factor", function () {

  it("like terms", function () {
    expect(trees.equal(
      me.fromText("x+y+x").collect_like_terms_factors().tree,
      me.fromText("2x+y").tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("3m+2n-m").collect_like_terms_factors().tree,
      me.fromText("2m+2n").tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("-q+2v-uv-3u+7uv+4q+3u-5v-2uv").collect_like_terms_factors().tree,
      me.fromText("3q-3v+4uv").evaluate_numbers().tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("uv+uv").collect_like_terms_factors().tree,
      me.fromText("2uv").evaluate_numbers().tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("u/v+u/v").collect_like_terms_factors().tree,
      me.fromText("2u/v").evaluate_numbers().tree
    )).toBeTruthy();

    expect(me.fromText("u/v-u/v").collect_like_terms_factors().tree)
      .toEqual(0);

    expect(trees.equal(
      me.fromText("x+x/2").collect_like_terms_factors().tree,
      me.fromText("3x/2").tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("x-x/2").collect_like_terms_factors().tree,
      me.fromText("x/2").tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("3x/4+x/2").collect_like_terms_factors().tree,
      me.fromText("5x/4").tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("3x/4-x/2").collect_like_terms_factors().tree,
      me.fromText("x/4").tree
    )).toBeTruthy();

    expect(me.fromText("1/((-1)(0))").collect_like_terms_factors().tree).toEqual(-Infinity);

    expect(trees.equal(
      me.fromText("3C^+ - 2C^+").collect_like_terms_factors().tree,
      me.fromText("C^+").tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("3C^+ - 2C^- +2C^+-C^-").collect_like_terms_factors().tree,
      me.fromText("5C^+-3C^-").evaluate_numbers().tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("3C^(2+) - 2C^(3-) +2C^(2+)-C^(3-)").collect_like_terms_factors().tree,
      me.fromText("5C^(2+)-3C^(3-)").evaluate_numbers().tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("3C_+ - 2C_- +2C_+-C_-").collect_like_terms_factors().tree,
      me.fromText("5C_+-3C_-").evaluate_numbers().tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("3C_(2+) - 2C_(3-) +2C_(2+)-C_(3-)").collect_like_terms_factors().tree,
      me.fromText("5C_(2+)-3C_(3-)").evaluate_numbers().tree
    )).toBeTruthy();

  });

  it("like factors", function () {
    expect(trees.equal(
      me.fromText("3xyx").collect_like_terms_factors().tree,
      me.fromText("3x^2y").evaluate_numbers().tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("-5u/v*u^3/w/v^5").collect_like_terms_factors().tree,
      me.fromText("-5*u^4/(v^6*w)").evaluate_numbers().tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("(3x/y)^3*x/(zy^2)/(yz^2)^2")
        .collect_like_terms_factors().tree,
      me.fromText("27x^4/(y^7z^5)").tree
    )).toBeTruthy();
  });

  it("like factors with assumptions", function () {

    me.clear_assumptions();
    expect(trees.equal(
      me.fromText("y*3x/y*x/z/y^2").collect_like_terms_factors().tree,
      me.fromText("3yx^2/(y^3z)").evaluate_numbers().tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText('y/y/y^2').collect_like_terms_factors().tree,
      me.fromText('y/y^3').tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText('y*y^(-1)*y^(-2)').collect_like_terms_factors().tree,
      me.fromText('y/y^3').tree
    )).toBeTruthy();

    me.add_assumption(me.fromText("y != 0"));

    expect(trees.equal(
      me.fromText("y*3x/y*x/z/y^2").collect_like_terms_factors().tree,
      me.fromText("3x^2/(y^2z)").evaluate_numbers().tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText('y/y/y^2').collect_like_terms_factors().tree,
      me.fromText('1/y^2').tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText('y*y^(-1)*y^(-2)').collect_like_terms_factors().tree,
      me.fromText('1/y^2').tree
    )).toBeTruthy();

    me.clear_assumptions();

  });

  it("like terms and factors", function () {

    me.add_assumption(me.fromText("v>0"));
    expect(trees.equal(
      me.fromText('u/v+uv/v^2+vuv/v^3-vuv^2/v^2/v/v')
        .collect_like_terms_factors().tree,
      me.fromText('2u/v').tree
    )).toBeTruthy();

    me.clear_assumptions();

  });

  it("like terms and factors infinity digits", function () {

    expect(trees.equal(
      me.fromText("a/9 -3/9 - a/9")
        .collect_like_terms_factors(undefined, Infinity).tree,
      me.fromText('-1/3').collect_like_terms_factors(undefined, Infinity).tree
    )).toBeTruthy();

    me.clear_assumptions();

  });

  it("collect combine numbers", function () {

    expect(me.fromText('2c+6c/(-3)').collect_like_terms_factors().tree)
      .toEqual(0);

  });

  it("speed tests", function () {

    expect(trees.equal(
      me.fromText('abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabc')
        .collect_like_terms_factors().tree,
      me.fromText('-a^3b^3c^3d^2e^2f^2g^2h^2j^2k^2l^2m^2n^2o^2p^2q^2r^2s^2t^2u^2v^2w^2x^2y^2z^2').tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText('a+b+c+d+e+f+g+h+i+j+k+l+m+n+o+p+q+r+s+t+u+v+w+x+y+z+a+b+c+d+e+f+g+h+i+j+k+l+m+n+o+p+q+r+s+t+u+v+w+x+y+z+a+b+c')
        .collect_like_terms_factors().tree,
      me.fromText('3a+3b+3c+2d+2e+2f+2g+2h+2i+2j+2k+2l+2m+2n+2o+2p+2q+2r+2s+2t+2u+2v+2w+2x+2y+2z').tree
    )).toBeTruthy();

  });

  it("don't turn pi to decimal", function () {
    expect(me.fromText('pi/2')
      .collect_like_terms_factors().tree).toEqual(['/', 'pi', 2])

  })

  it("treat exp like power", function () {
    expect(trees.equal(
      me.fromText("e^3 e^5").collect_like_terms_factors().tree,
      me.fromText("e^8").evaluate_numbers().tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("exp(3) exp(5)").collect_like_terms_factors().tree,
      me.fromText("exp(8)").evaluate_numbers().tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("-5e^(-t)").collect_like_terms_factors().tree,
      me.fromText("-5/e^t").evaluate_numbers().tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("-5exp(-t)").collect_like_terms_factors().tree,
      me.fromText("-5/exp(t)").evaluate_numbers().tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("e^(3)/e^(-5)").collect_like_terms_factors().tree,
      me.fromText("e^8").evaluate_numbers().tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("exp(3)/exp(-5)").collect_like_terms_factors().tree,
      me.fromText("exp(8)").evaluate_numbers().tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("e^(3)/e^(5)").collect_like_terms_factors().tree,
      me.fromText("1/e^2").evaluate_numbers().tree
    )).toBeTruthy();

    expect(trees.equal(
      me.fromText("exp(3)/exp(5)").collect_like_terms_factors().tree,
      me.fromText("1/exp(2)").evaluate_numbers().tree
    )).toBeTruthy();

  })

  it("with blanks untouched", function () {
    expect(me.fromText("/5+").collect_like_terms_factors().tree).toEqual(['+', ["/", "\uff3f", 5], '\uff3f']);
    expect(me.fromText("(3*) + 5*").collect_like_terms_factors().tree).toEqual(['+', ["*", 3, "\uff3f"], ["*", 5, '\uff3f']]);
  })

});

describe("other simplify", function () {

  it("add tuples", function () {
    expect(me.fromText("(a,b)+(c,d)").simplify().tree).toEqual(me.fromText("(a+c, b+d)").tree);
    expect(me.fromText("(a,b)+(c,d)+(e,f)+(g,h)").simplify().tree).toEqual(me.fromText("(a+c+e+g, b+d+f+h)").tree);
    expect(me.fromText("(a,b)+(c,d,2)+(e,f)+(g,h,3)+9").simplify().tree).toEqual(me.fromText("(a+e, b+f) + (c+g, d+h, 5)+9").default_order().tree);
  })

  it("add vectors", function () {
    expect(me.fromText("(a,b)+(c,d)").tuples_to_vectors().simplify().tree).toEqual(me.fromText("(a+c, b+d)").tuples_to_vectors().tree);
    expect(me.fromText("(a,b)+(c,d)+(e,f)+(g,h)").tuples_to_vectors().simplify().tree).toEqual(me.fromText("(a+c+e+g, b+d+f+h)").tuples_to_vectors().tree);
    expect(me.fromText("(a,b)+(c,d,2)+(e,f)+(g,h,3)+9").tuples_to_vectors().simplify().tree).toEqual(me.fromText("(a+e, b+f) + (c+g, d+h, 5)+9").default_order().tuples_to_vectors().tree);
  })

  it("add tuples and vectors", function () {
    expect(me.fromAst(["+", ["vector", "a", "b"], ["tuple", "c", "d"]]).simplify().tree).toEqual(me.fromText("(a+c, b+d)").tuples_to_vectors().tree);
  })

  it("don't add intervals", function () {
    expect(me.fromText("(a,b)+(c,d)").to_intervals().simplify().tree).toEqual(me.fromText("(a,b)+(c,d)").to_intervals().tree);
  })

})

describe("expand", function () {

  it("expand polynomials", function () {
    expect(me.from("(x-1)(x+2)").expand().tree).toEqual(me.from("x^2+x-2").tree)
    expect(me.from("2(1-x)(1+x)(-2)").expand().tree).toEqual(me.from("4x^2-4").tree)
    expect(me.from("-9(-y+3z)8(z-2)(-3)").expand().tree).toEqual(me.from("-216 y z + 432 y + 648 z^2 - 1296 z").evaluate_numbers().tree)

  });


  it("expand matrix multiplication", function () {

    let matrix1 = me.fromLatex("\\begin{pmatrix}a & b\\\\c&d\\end{pmatrix}").tree;
    let matrix2 = me.fromLatex("\\begin{pmatrix}e\\\\f\\end{pmatrix}").tree;
    let product = me.fromLatex("\\begin{pmatrix}ae + bf\\\\ce + df\\end{pmatrix}").tree
    let product_g = me.fromLatex("\\begin{pmatrix}aeg + bfg\\\\ceg + dfg\\end{pmatrix}").tree

    let tuple = ["tuple", "e", "f"]
    let product_tuple = me.fromLatex("(ae + bf, ce + df)").tree
    let product_tuple_g = me.fromLatex("(aeg + bfg, ceg + dfg)").tree
    let vector = ["vector", "e", "f"]
    let product_vector = me.fromLatex("(ae + bf, ce + df)").tuples_to_vectors().tree
    let product_vector_g = me.fromLatex("(aeg + bfg, ceg + dfg)").tuples_to_vectors().tree

    expect(me.fromAst(["*", matrix1, matrix2]).expand().tree).toEqual(product)
    expect(me.fromAst(["*", matrix2, matrix1]).expand().tree).toEqual(["*", matrix2, matrix1])
    expect(me.fromAst(["*", matrix1, matrix2, "g"]).expand().tree).toEqual(product_g)
    expect(me.fromAst(["*", "g", matrix1, matrix2]).expand().tree).toEqual(product_g)
    expect(me.fromAst(["*", matrix1, "g", matrix2]).expand().tree).toEqual(product_g)

    expect(me.fromAst(["*", matrix1, tuple]).expand().tree).toEqual(product_tuple)
    expect(me.fromAst(["*", tuple, matrix1]).expand().tree).toEqual(["*", tuple, matrix1])
    expect(me.fromAst(["*", "g", matrix1, tuple]).expand().tree).toEqual(product_tuple_g)
    expect(me.fromAst(["*", matrix1, "g", tuple]).expand().tree).toEqual(product_tuple_g)
    expect(me.fromAst(["*", matrix1, tuple, "g"]).expand().tree).toEqual(product_tuple_g)

    expect(me.fromAst(["*", matrix1, vector]).expand().tree).toEqual(product_vector)
    expect(me.fromAst(["*", vector, matrix1]).expand().tree).toEqual(["*", vector, matrix1])
    expect(me.fromAst(["*", "g", matrix1, vector]).expand().tree).toEqual(product_vector_g)
    expect(me.fromAst(["*", matrix1, "g", vector]).expand().tree).toEqual(product_vector_g)
    expect(me.fromAst(["*", matrix1, vector, "g"]).expand().tree).toEqual(product_vector_g)


    let matrix3 = me.fromLatex("\\begin{pmatrix}1 & -2\\\\3&-4\\end{pmatrix}").tree;
    let product13 = me.fromLatex("\\begin{pmatrix}a + 3b & -2a-4b\\\\c + 3d & -2c-4d\\end{pmatrix}").evaluate_numbers().tree
    let product31 = me.fromLatex("\\begin{pmatrix}a -2c & b - 2d\\\\3a -4c & 3b -4d\\end{pmatrix}").evaluate_numbers().tree

    expect(me.fromAst(["*", matrix1, matrix3]).expand().tree).toEqual(product13)
    expect(me.fromAst(["*", matrix3, matrix1]).expand().tree).toEqual(product31)

    let product132 = me.fromLatex("\\begin{pmatrix}ae + 3be -2af-4bf\\\\ce + 3de -2cf-4df\\end{pmatrix}").evaluate_numbers().tree
    let product312 = me.fromLatex("\\begin{pmatrix}ae -2ce + bf - 2df\\\\3ae -4ce + 3bf -4df\\end{pmatrix}").evaluate_numbers().tree
    let product132_g = me.fromLatex("\\begin{pmatrix}aeg + 3beg -2afg-4bfg\\\\ceg + 3deg -2cfg-4dfg\\end{pmatrix}").evaluate_numbers().tree
    let product312_g = me.fromLatex("\\begin{pmatrix}aeg -2ceg + bfg - 2dfg\\\\3aeg -4ceg + 3bfg -4dfg\\end{pmatrix}").evaluate_numbers().tree

    expect(me.fromAst(["*", matrix1, matrix3, matrix2]).expand().tree).toEqual(product132)
    expect(me.fromAst(["*", matrix3, matrix1, matrix2]).expand().tree).toEqual(product312)

    expect(me.fromAst(["*", "g", matrix1, matrix3, matrix2]).expand().tree).toEqual(product132_g)
    expect(me.fromAst(["*", matrix1, "g", matrix3, matrix2]).expand().tree).toEqual(product132_g)
    expect(me.fromAst(["*", matrix1, matrix3, "g", matrix2]).expand().tree).toEqual(product132_g)
    expect(me.fromAst(["*", matrix1, matrix3, matrix2, "g"]).expand().tree).toEqual(product132_g)

    expect(me.fromAst(["*", "g", matrix3, matrix1, matrix2]).expand().tree).toEqual(product312_g)
    expect(me.fromAst(["*", matrix3, "g", matrix1, matrix2]).expand().tree).toEqual(product312_g)
    expect(me.fromAst(["*", matrix3, matrix1, "g", matrix2]).expand().tree).toEqual(product312_g)
    expect(me.fromAst(["*", matrix3, matrix1, matrix2, "g"]).expand().tree).toEqual(product312_g)

  });

  it("expand with complex numbers", function () {
    expect(me.from("(x-i)(x+i)").expand().tree).toEqual(me.from("x^2+1").tree)
    expect(me.from("(i+ix)(i-x)").expand().tree).toEqual(me.from("-ix^2 -ix -x -1").tree)
  });


});

