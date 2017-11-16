
exports.get_tree = function(expr_or_tree) {

    if(expr_or_tree===undefined)
	return undefined;
    
    var tree;
    if(expr_or_tree.tree !== undefined)
	tree = expr_or_tree.tree;
    else
	tree = expr_or_tree;

    return tree;
}
