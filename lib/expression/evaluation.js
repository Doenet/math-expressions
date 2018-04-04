import { get_tree } from '../trees/util';
import astToMathjsObj from '../converters/ast-to-mathjs';

var astToMathjs = new astToMathjsObj();

const f = function(expr) {
    return astToMathjs.convert( expr.tree ).eval;
};

const evaluate = function(expr, bindings) {
    return f(expr)(bindings);
};

// export const finite_field_evaluate = function(expr, bindings, modulus) {
//     return parser.ast.to.finiteField( expr.tree, modulus )( bindings );
// };

const evaluate_to_constant = function(expr_or_tree) {
    // evaluate to number by converting tree to number
    // and calling without arguments

    // return null if couldn't evaluate to constant (e.g., contains a variable)
    // otherwise returns constant
    // NOTE: constant could be a math.js complex number object

    var tree = get_tree(expr_or_tree);

    var f = astToMathjs.convert( tree ).eval;

    var num=null;
    try {
	num = f();
    }
    catch (e) {};

    return num;
};

export default { f, evaluate, evaluate_to_constant };
