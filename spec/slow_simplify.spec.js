import me from '../lib/math-expressions';
import * as trees from '../lib/trees/basic';

describe("evaluate_numbers", function () {

  it("addition", function () {
    expect(me.from("4+x-2").evaluate_numbers().tree).toEqual(['+', 2, 'x']);

    expect(me.from("x+0").evaluate_numbers().tree).toEqual('x');

    expect(me.from("Infinity + 3").evaluate_numbers().tree).toEqual(Infinity);
    expect(me.from("Infinity + Infinity").evaluate_numbers().tree).toEqual(Infinity);
    expect(me.from("Infinity - Infinity").evaluate_numbers().tree).toEqual(NaN);

  });
    
  it("collapse unary minus", function () {
    expect(me.from("x-2").evaluate_numbers().tree).toEqual(['+', -2, 'x']);
  });

  it("unary minus of product", function() {
    expect(me.from("x-2uv").evaluate_numbers().tree).toEqual(
      ['+', 'x', ['*', -2, 'u', 'v']]);

  });
  it("unary minus of quotient", function() {
    expect(me.from("x-2/(uv)").evaluate_numbers().tree).toEqual(
        ['+', 'x', ['/', -2, ['*', 'u', 'v']]]);
    expect(me.from("x-2u/v").evaluate_numbers().tree).toEqual(
        ['+', 'x', ['/', ['*', -2, 'u'], 'v']]);

  });

  it("multiplication", function () {
    expect(me.from("3*2*x*4").evaluate_numbers().tree).toEqual(
        ['*', 24, 'x']);

    expect(me.from("3*2*x*0").evaluate_numbers().tree).toEqual(0);

    expect(me.from("(2-1)x").evaluate_numbers().tree).toEqual('x');

    expect(me.from("(-1+2-1)x").evaluate_numbers().tree).toEqual(0);

    expect(me.from("(-1+2-2)x").evaluate_numbers().tree).toEqual(
        ['-', 'x']);

    expect(me.from("4(x)(-2)").evaluate_numbers().tree).toEqual(
        ['*', -8, 'x']);

    expect(me.from("0*Infinity").evaluate_numbers().tree).toEqual(NaN);

	expect(me.from("2*2*2*2*2*2*2*2*2*2*2*2*2*2").evaluate_numbers().tree)
		.toEqual(16384);

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
  });

  it("power", function () {
    expect(me.from("x^0").evaluate_numbers().tree).toEqual(['^','x', 0]);
    me.add_assumption(me.from('x!= 0'));
    expect(me.from("x^0").evaluate_numbers().tree).toEqual(1);
    me.clear_assumptions();

    expect(me.from("(3-3)^0").evaluate_numbers().tree).toEqual(NaN);

    expect(me.from("(3x-3x)^0").evaluate_numbers().tree).toEqual(NaN);

    expect(me.from("(4-3)^7").evaluate_numbers().tree).toEqual(1);

    expect(me.from("(5-3)^3").evaluate_numbers().tree).toEqual(8);

  });

  it("to constant", function () {

    expect(me.from("log(e)x+log(1)y").evaluate_numbers().tree).toEqual('x');

    expect(me.from("cos(0)x+sin(pi/2)y").evaluate_numbers().tree).toEqual(
        ['+', 'x', 'y']);
  });

  it("to decimals", function () {

    expect(me.fromText('pi').evaluate_numbers().tree).toEqual('pi');
    expect(me.fromText('0.5 pi').evaluate_numbers().tree).toBeCloseTo(0.5*Math.PI, 1E-12);
    expect(me.fromText('pi/2').evaluate_numbers().tree).toEqual(['/', 'pi', 2]);

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
    });

    it("like factors", function () {
	expect(trees.equal(
	    me.fromText("3xyx").collect_like_terms_factors().tree,
	    me.fromText("3x^2y").tree
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
	    me.fromText("3yx^2/(y^3z)").tree
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
	    me.fromText("3x^2/(y^2z)").tree
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
	    me.fromText('a^3b^3c^3d^2e^2f^2g^2h^2i^2j^2k^2l^2m^2n^2o^2p^2q^2r^2s^2t^2u^2v^2w^2x^2y^2z^2').tree
	)).toBeTruthy();
	
	expect(trees.equal(
	    me.fromText('a+b+c+d+e+f+g+h+i+j+k+l+m+n+o+p+q+r+s+t+u+v+w+x+y+z+a+b+c+d+e+f+g+h+i+j+k+l+m+n+o+p+q+r+s+t+u+v+w+x+y+z+a+b+c')
		.collect_like_terms_factors().tree,
	    me.fromText('3a+3b+3c+2d+2e+2f+2g+2h+2i+2j+2k+2l+2m+2n+2o+2p+2q+2r+2s+2t+2u+2v+2w+2x+2y+2z').tree
	)).toBeTruthy();
	
    });
    
});
