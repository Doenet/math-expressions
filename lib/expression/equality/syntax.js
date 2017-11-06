var tree = require('../../trees/basic');

exports.equals = function (expr, other) {
    return tree.equal( expr.tree, other.tree );
};
