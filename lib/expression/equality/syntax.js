import tree from '../../trees/basic';

export const equals = function (expr, other) {
    return tree.equal( expr.tree, other.tree );
};
