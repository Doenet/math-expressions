var simplify = require('./simplify.js' );
var differentiation = require('./differentiation.js' );
var normalization = require('./normalization' );
var sign_error = require('./sign-error.js');
var arithmetic = require('./arithmetic.js');
var analytic = require('./analytic.js');
var transformation = require('./transformation.js');
var solve = require('./solve.js');
var sets = require('./sets.js');
var matrix = require('./matrix.js');
var evaluation = require('./evaluation.js');
var variables = require('./variables.js' );
var printing = require('./printing.js' );
var equality = require('./equality.js');
var integration = require('./integration.js');


exports.expression_to_tree = [
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
]

exports.expression_to_other = [
    evaluation,
    variables,
    printing,
    equality,
    integration,
];
