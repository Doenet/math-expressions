var is_associative = { '+': true, '*': true, 'and': true, 'or': true, 'union': true, 'intersect': true };

function flatten(tree) {

	// flatten tree with all associative operators

	if (!Array.isArray(tree))
		return tree;

	var operator = tree[0];
	var operands = tree.slice(1);

	operands = operands.map(function (v, i) {
		return flatten(v);
	});

	if (is_associative[operator]) {
		var result = [];

		for (var i = 0; i < operands.length; i++) {
			if (Array.isArray(operands[i]) && (operands[i][0] === operator) && operands[i].length > 2) {
				result = result.concat(operands[i].slice(1));
			} else {
				result.push(operands[i]);
			}
		}

		operands = result;
	}

	return [operator].concat(operands);
};

export default flatten;
