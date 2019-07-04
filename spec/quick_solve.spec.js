import me from '../lib/math-expressions';
import * as trees from '../lib/trees/basic';

describe("solve linear", function () {

    it("linear equation", function () {

	expect(trees.equal(me.fromText('3x+4 = 2').solve_linear('x').tree,
			   me.fromText("x = -2/3").evaluate_numbers().tree)).toBeTruthy();
	expect(trees.equal(me.fromText('-3y-2 = y+1').solve_linear('y').tree,
			   me.fromText("y = -3/4").evaluate_numbers().tree)).toBeTruthy();

	expect(me.fromText('2uv-v = 3u+q').solve_linear('u').tree)
	    .toEqual(undefined);

	me.add_assumption(me.from("v < 0"));
	expect(trees.equal(
	    me.fromText('2uv-v = 3u+q').solve_linear('u').simplify_ratios().evaluate_numbers().tree,
	    me.fromText("u = (q+v)/(2v-3)").simplify_ratios().evaluate_numbers().tree
	)).toBeTruthy();
	me.clear_assumptions();
	
    });

    it("nonlinear doesn't work", function () {
	expect(me.fromText("2u^2=1").solve_linear("u").tree).toEqual(undefined);
    });


    it("inequalities", function () {

	expect(trees.equal(me.fromText('3x-2 != 4x+1').solve_linear('x').evaluate_numbers().tree,
			   me.fromText("x != -3").evaluate_numbers().tree)).toBeTruthy();

	expect(trees.equal(me.fromText('2x-4 < 6').solve_linear('x').tree,
			   me.fromText("x < 5").tree)).toBeTruthy();
	expect(trees.equal(me.fromText('2x-4 < 6+4x').solve_linear('x').evaluate_numbers().tree,
			   me.fromText("x > -5").evaluate_numbers().tree)).toBeTruthy();
	expect(trees.equal(me.fromText('-3y -v <= 2xz+r').solve_linear('y').simplify_ratios().tree,
			   me.fromText("y >= -(2xz+r+v)/3").simplify_ratios().tree)).toBeTruthy();
	me.add_assumption(me.from('u < 0'));
	expect(trees.equal(me.fromText('6uv+5r > 3uv+3r').solve_linear('v').simplify_ratios().evaluate_numbers().tree,
			   me.fromText("v < -2r/(3u)").simplify_ratios().evaluate_numbers().tree)).toBeTruthy();
	
	me.clear_assumptions();
	
	expect(me.fromText("3kp >= 3kp-k").solve_linear("p").tree).toEqual(undefined);
	expect(trees.equal(me.fromText('3kp >= 3kp+p').solve_linear('p').tree,
			   me.fromText("p <=0").tree)).toBeTruthy();

	
    });
});
