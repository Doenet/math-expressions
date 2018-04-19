import { equal as tree_equal } from '../../trees/basic';

export const equals = function (expr, other) {
    return tree_equal( expr.tree, other.tree );
};
