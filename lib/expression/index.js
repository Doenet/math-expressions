import simplify from './simplify.js';
import differentiation from './differentiation.js';
import normalization from './normalization';
import sign_error from './sign-error.js';
import arithmetic from './arithmetic.js';
import analytic from './analytic.js';
import transformation from './transformation.js';
import solve from './solve.js';
import sets from './sets.js';
import matrix from './matrix.js';
import variables from './variables.js';
import printing from './printing.js';
import equality from './equality.js';
import integration from './integration.js';
import rational from './rational.js';

export const expression_to_tree = [
    simplify,
    differentiation,
    normalization,
    sign_error,
    arithmetic,
    analytic,
    transformation,
    solve,
    sets,
    matrix,
    rational,
];

export const expression_to_other = [
    variables,
    printing,
    equality,
    integration,
];
