
const get_tree = function(expr_or_tree) {

    if(expr_or_tree===undefined)
	return undefined;
    
    var tree;
    if(expr_or_tree.tree !== undefined)
	tree = expr_or_tree.tree;
    else
	tree = expr_or_tree;

    return tree;
};


function *subsets(arr, m) {
    // returns an iterator over all subsets of array arr
    // up to size m
    
    var n = arr.length;

    if(m===undefined)
	m=n;

    if(m == 0)
	return;
    
    for(let i=0; i < n; i++) {
	yield [arr[i]];
    }

    if(m == 1)
	return
    
    for(let i=0; i < n; i++) {
	let sub = subsets(arr.slice(i+1), m-1);
	for(let val of sub) {
	    yield [arr[i]].concat(val);
	}
    }
}

export { subsets, get_tree };
