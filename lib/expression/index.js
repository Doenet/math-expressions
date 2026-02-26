import * as simplify from "./simplify.js";
import * as differentiation from "./differentiation.js";
import * as normalization from "./normalization/index.js";
import * as sign_error from "./sign_error.js";
import * as arithmetic from "./arithmetic.js";
import * as analytic from "./analytic.js";
import * as transformation from "./transformation.js";
import * as solve from "./solve.js";
import * as sets from "./sets.js";
import * as matrix from "./matrix.js";
import * as variables from "./variables.js";
import * as printing from "./printing.js";
import * as equality from "./equality.js";
import * as integration from "./integration.js";
import * as rational from "./rational.js";
import * as evaluation from "./evaluation.js";
import * as round from "./round.js";
import * as match from "./match.js";

export const expression_to_tree = [
  simplify,
  differentiation,
  normalization,
  arithmetic,
  transformation,
  solve,
  sets,
  matrix,
  rational,
  round,
];

export const expression_to_other = [
  variables,
  printing,
  equality,
  integration,
  evaluation,
  analytic,
  sign_error,
  match,
];
