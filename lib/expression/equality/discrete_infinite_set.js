import * as simplify from '../simplify';
import * as trans from '../transformation';
import math from '../../mathjs';
import * as assume from '../../assumptions/assumptions';

export const equals = function(expr, other) {

    if(!is_discrete_infinite_set(expr) || !is_discrete_infinite_set(other))
	return false;

    var assumptions = [];
    let a = expr.context.get_assumptions(expr);
    if(a !== undefined)
	assumptions.push(a);
    a = other.context.get_assumptions(other);
    if(a !== undefined)
	assumptions.push(a);
    if(assumptions.length == 0)
	assumptions = undefined;
    else if(assumptions.length == 1)
	assumptions=assumptions[0];
    else
	assumptions = assume.clean_assumptions(['and'].concat(assumptions));

    return contained_in(expr.tree, other.tree, assumptions) &&
	contained_in(other.tree, expr.tree, assumptions);
};


function is_discrete_infinite_set(expr) {

    var tree = expr.tree;
    if(!Array.isArray(tree))
	return false;
    if(tree[0] !== 'discrete_infinite_set')
	return false;
    var operands = tree.slice(1);

    for(var v of operands) {
	if(!Array.isArray(v))
	    return false
	if(v[0] !== 'tuple')
	    return false;
    }

    return true;
}


function contained_in(tree, i_set, assumptions) {
    // true if tree is contained in the discrete infinite set i_set
    // tree is either a discrete infinite set
    // or a tuple of form [offset, period]

    if(tree[0] === 'discrete_infinite_set')
	return tree.slice(1).every(v => contained_in(v, i_set, assumptions));

    // tree is a tuple of the form [offset, period]

    var offset0 = tree[1];
    var period0 = tree[2];

    // normalize to period 1
    offset0 = simplify.simplify(
	['/', offset0, period0], assumptions, Infinity);

    // if(!(typeof offset0 === 'number'))
    // 	return false;

    var tuples = i_set.slice(1);

    // data will be array of form [p, q, offset, period]
    // where offset are period are normalized by period0
    // and p/q is fraction form of period

    var data = [];
    for(let i=0; i<tuples.length; i++) {
	let offset = simplify.simplify(
	    ['/', tuples[i][1], period0], assumptions, Infinity);
	let period = simplify.simplify(
	    ['/', tuples[i][2], period0], assumptions, Infinity);

	if(typeof period !== 'number')
	    return false;

	let frac = math.fraction(period);
	let p = frac.n;
	let q = frac.d;
	data.push([p,q,offset,period]);
    }

    // sort by p
    data.sort();

    // check any with period for which original period is a multiple
    while(true) {
	let p = data[0][0];
	if(p != 1)
	    break;

	let offset = data[0][2];
	let period = data[0][3];

	// offsets match, then we've covered all of tree
	let offset_diff =  simplify.simplify(
	    trans.expand(
		['+', offset, ['-', offset0]]),
	    assumptions, Infinity);
	offset_diff = offset_diff % period;

	if(math.abs(offset_diff) < 1E-10*period)
	    return true;
	else {
	    data.splice(0,1);  // remove first entry from data
	    if(data.length == 0)
		return false;
	}

    }

    var all_ps = [... new Set(data.map(v => v[0]))];

    for(let base_p of all_ps) {
	// find all ps where base_p is a multiple
	let options = data.map(function (v,i) {
	    let m = base_p/v[0];
	    if(Number.isInteger(m))
		return [v[0], m, i];
	}).filter(v=>v);

	let covered = [];

	for(let opt of options) {
	    let p = opt[0];
	    let m = opt[1];
	    let i = opt[2];
	    let offset = data[i][2];
	    let period = data[i][3];


	    for(let j=0; j < p; j++) {

		let offset_diff =  simplify.simplify(
		    trans.expand(
			['+', offset, ['-', ['+', offset0, j]]]),
		    assumptions, Infinity);
		offset_diff = offset_diff % period;

		if(math.abs(offset_diff) < 1E-10*period) {

		    for(let k=0; k<m; k++) {
			covered[j+k*p] = true;
		    }

		    // check to see if covered all;
		    let covered_all = true;
		    for(let ind=0; ind < base_p; ind++) {
			if(!covered[ind]) {
			    covered_all = false;
			    break;
			}
		    }

		    if(covered_all)
			return true;

		    break;
		}
	    }
	}
    }

    return false;

}
