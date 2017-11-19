exports.expression_to_tree = [
    require('./simplify.js' ),
    require('./differentiation.js' ),
    require('./normalization' ),
    require('./sign-error.js'),
    require('./arithmetic.js'),
    require('./analytic.js'),
    require('./transformation.js'),
]

exports.expression_to_other = [
    require('./evaluation.js'),
    require('./variables.js' ),
    require('./printing.js' ),
    require('./equality.js'),
    require('./integration.js'),
];
