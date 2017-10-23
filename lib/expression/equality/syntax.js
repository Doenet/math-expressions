var trees = require('../../trees.js');

exports.equals = function (expr, other) {
    return trees.equal( expr.tree, other.tree );
};
