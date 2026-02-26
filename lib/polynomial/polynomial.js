import { get_tree } from "../trees/util.js";
import * as simplify from "../expression/simplify.js";
import math from "../mathjs.js";
import { operators as operators_in } from "../expression/variables.js";
import { evaluate_to_constant } from "../expression/evaluation.js";
import * as default_order from "../trees/default_order.js";
import _ from "underscore";
import * as sv from "../polynomial/single-var-poly.js";

function expression_to_polynomial(expr_or_tree) {
  var tree = get_tree(expr_or_tree);

  if (typeof tree === "string") {
    if (
      (tree === "pi" && math.define_pi) ||
      (tree === "i" && math.define_i) ||
      (tree === "e" && math.define_e)
    )
      return tree; // treat as number
    else return ["polynomial", tree, [[1, 1]]]; // treat a polynomial variable
  }

  if (typeof tree === "number") return tree;

  var c = evaluate_to_constant(tree, { remove_units_first: false });
  if (Number.isFinite(c)) {
    return simplify.simplify(tree);
  }

  if (!Array.isArray(tree)) return false;

  // if contains invalid operators, it's not a polynomial
  if (
    !operators_in(tree).every((v) =>
      ["+", "-", "*", "^", "/", "_", "prime"].includes(v),
    )
  )
    return false;

  var operator = tree[0];
  var operands = tree.slice(1);

  if (operator === "+") {
    let result = operands.map(expression_to_polynomial);

    // return false if any operand returned false
    if (!result.every((v) => v !== false)) return false;

    return result.reduce((u, v) => polynomial_add(u, v));
  } else if (operator === "-") {
    let result = expression_to_polynomial(operands[0]);

    if (!result) return false;

    return polynomial_neg(result);
  } else if (operator === "*") {
    let result = operands.map(expression_to_polynomial);

    // return false if any operand returned false
    if (!result.every((v) => v !== false)) return false;

    return result.reduce((u, v) => polynomial_mul(u, v));
  } else if (operator === "^") {
    let base = operands[0];
    let subresult = expression_to_polynomial(base);

    // if subresult itself is false, then don't have a polynomial
    if (subresult === false) return false;

    let pow = simplify.simplify(operands[1]);

    // if pow isn't a literal nonnegative integer
    if (typeof pow !== "number" || pow < 0 || !Number.isInteger(pow)) {
      let pow_num = evaluate_to_constant(pow, { remove_units_first: false });

      // check if pow is a rational number with a small base
      if (Number.isFinite(pow_num)) {
        let pow_fraction_pre = math.fraction(pow_num);

        let pow_fraction = {
          n: math.number(pow_fraction_pre.n),
          d: math.number(pow_fraction_pre.d),
          s: math.number(pow_fraction_pre.s),
        };

        if (pow_fraction.d <= 100) {
          if (pow_fraction.s < 0) base = ["^", base, ["/", -1, pow_fraction.d]];
          else base = ["^", base, ["/", 1, pow_fraction.d]];

          var results = ["polynomial", simplify.simplify(base), []];

          results[2].push([pow_fraction.n, 1]);

          return results;
        }
      }

      // just return entire tree as a polynomial variable
      return ["polynomial", tree, [[1, 1]]];
    }

    if (pow === 0) {
      return 1;
    }
    if (pow === 1) {
      return subresult;
    }

    return polynomial_pow(subresult, pow);
  } else if (operator === "/") {
    var denom = operands[1];

    var denom_num = evaluate_to_constant(denom, { remove_units_first: false });

    if (!Number.isFinite(denom_num)) {
      // return entire tree as polynomial variable
      return ["polynomial", tree, [[1, 1]]];
    }

    var numer_result = expression_to_polynomial(operands[0]);

    return polynomial_mul(numer_result, ["/", 1, denom_num]);
  } else {
    // return entire tree as polynomial variable
    return ["polynomial", tree, [[1, 1]]];
  }
}

function poly_to_terms(f) {
  if (!Array.isArray(f) || f[0] !== "polynomial") {
    return f;
  }

  let terms = ["polynomial_terms"];
  let focus = f;
  let var_powers = [];
  let current_term = [];

  while (f !== 0) {
    focus = f;
    var_powers = [];
    while (Array.isArray(focus) && focus[0] === "polynomial") {
      let x = focus[1];
      let terms = focus[2];
      let exp = terms[terms.length - 1][0];
      focus = terms[terms.length - 1][1];
      var_powers.push([x, exp]);
    }
    if (var_powers.length !== 0) current_term = ["monomial", focus, var_powers];
    else current_term = focus;

    terms.push(current_term);
    f = polynomial_sub(f, mono_to_poly(current_term));
  }

  return terms;
}

function terms_to_poly(f) {
  if (!Array.isArray(f) || f[0] !== "polynomial_terms") {
    return f;
  }

  let len = f.length;
  let poly = 0;

  for (var i = 1; i < len; i = i + 1) {
    poly = polynomial_add(poly, mono_to_poly(f[i]));
  }

  return poly;
}

function polynomials_in_same_leading_variable(p, q) {
  // If both polynomials have same leading variable, return unchanged.
  // Else, rewrite the polymomial whose leading variable comes later
  // as a polynomial that is constant in leading variable of other

  if (p[1] !== q[1]) {
    if (default_order.compare_function(p[1], q[1]) < 0) {
      // variable p[1] is earlier in default order
      // so write q as a polynomial constant in p[1]
      q = ["polynomial", p[1], [[0, q]]];
    } else {
      // variable q[1] is earlier in default order
      // so write p as a polynomial constant in q[1]
      p = ["polynomial", q[1], [[0, p]]];
    }
  }

  return [p, q];
}

function polynomial_add(p, q) {
  //being called on empty polynomials

  if (p[0] !== "polynomial") {
    if (q[0] !== "polynomial") return simplify.simplify(["+", p, q]);
    else {
      // write p as a constant polynomial in q's first variable
      p = ["polynomial", q[1], [[0, p]]];
    }
  } else {
    if (q[0] !== "polynomial") {
      // write q as a constant polynomial in p's first variable
      q = ["polynomial", p[1], [[0, q]]];
    } else {
      // if needed, rewrite polynomials so have same first variable
      let tmp = polynomials_in_same_leading_variable(p, q);
      p = tmp[0];
      q = tmp[1];
    }
  }

  // at this point, both q and p are polynomials with same first variable

  let sum = ["polynomial", p[1], []];

  let p_terms = p[2];
  let q_terms = q[2];
  let sum_terms = sum[2];

  let len_p = p_terms.length;
  let len_q = q_terms.length;
  let i = 0;
  let j = 0;

  while (i < len_p || j < len_q) {
    if (i === len_p) {
      if (q_terms[j][1]) sum_terms.push(q_terms[j]);
      j = j + 1;
    } else if (j === len_q) {
      if (p_terms[i][1]) sum_terms.push(p_terms[i]);
      i = i + 1;
    } else if (p_terms[i][0] === q_terms[j][0]) {
      let temp = polynomial_add(p_terms[i][1], q_terms[j][1]);
      if (temp) sum_terms.push([p_terms[i][0], temp]);
      i = i + 1;
      j = j + 1;
    } else if (p_terms[i][0] < q_terms[j][0]) {
      if (p_terms[i][1]) sum_terms.push(p_terms[i]);
      i = i + 1;
    } else {
      if (q_terms[j][1]) sum_terms.push(q_terms[j]);
      j = j + 1;
    }
  }

  // all terms canceled
  if (sum_terms.length === 0) return 0;

  // only a term that is constant in leading variable is left
  if (sum_terms.length === 1 && sum_terms[0][0] === 0) return sum_terms[0][1];

  return sum;
}

function terms_poly_add(p, q) {
  let sum = [];
  let term = [];

  if (!Array.isArray(p) || p[0] !== "polynomial_terms") {
    if (p === 0) return q;
    if (!Array.isArray(q) || q[0] !== "polynomial_terms")
      return simplify.simplify(["+", p, q]);
    let len = q.length;
    for (let i = 0; i < len - 1; i = i + 1) {
      sum.push(q[i]);
    }
    if (
      !Array.isArray(q[len - 1]) ||
      q[len - 1][0] !== "monomial" ||
      q[len - 1][2].length === 0
    ) {
      //if the smallest term of q is constant
      term = simplify.simplify(["+", p, q[len - 1]]);
      if (term !== 0) sum.push(term);
    } else {
      sum.push(q[len - 1]);
      sum.push(p);
    }
    return sum;
  }

  if (!Array.isArray(q) || q[0] !== "polynomial_terms") {
    if (q === 0) return p;
    let len = p.length;
    for (let i = 0; i < len - 1; i = i + 1) {
      sum.push(p[i]);
    }
    if (
      !Array.isArray(
        p[len - 1] ||
          p[len - 1][0] !== "monomial" ||
          p[len - 1][2].length === 0,
      )
    ) {
      term = simplify.simplify(["+", p[len - 1], q]);
      if (term !== 0) sum.push(term);
    } else {
      sum.push(p[len - 1]);
      sum.push(q);
    }
    return sum;
  }

  sum.push("polynomial_terms");
  let len_p = p.length;
  let len_q = q.length;
  let i = 1;
  let j = 1;
  while (i < len_p && j < len_q) {
    if (!Array.isArray(p[i]) || p[i][0] !== "monomial") {
      if (!Array.isArray(q[j]) || q[j][0] !== "monomial") {
        term = simplify.simplify(["+", p[i], q[j]]);
        if (term !== 0) sum.push(term);
        i = i + 1;
        j = j + 1;
        break;
      }
      sum.push(q[j]);
      j = j + 1;
    } else if (!Array.isArray(q[j]) || q[j][0] !== "monomial") {
      sum.push(p[i]);
      i = i + 1;
    } else if (mono_less_than(p[i], q[j])) {
      sum.push(q[j]);
      j = j + 1;
    } else if (mono_less_than(q[j], p[i])) {
      sum.push(p[i]);
      i = i + 1;
    } else {
      term = simplify.simplify(["+", p[i][1], q[j][1]]);
      if (term !== 0) sum.push(["monomial", term, p[i][2]]);
      i = i + 1;
      j = j + 1;
    }
  }

  while (i < len_p) {
    sum.push(p[i]);
    i = i + 1;
  }

  while (j < len_q) {
    sum.push(q[j]);
    j = j + 1;
  }

  if (sum.length === 1) return 0;

  if (sum.length === 2 && (!Array.isArray(sum[1]) || sum[1][0] !== "monomial"))
    return sum[1];

  return sum;
}

function polynomial_neg(p) {
  if (p[0] !== "polynomial") {
    return simplify.simplify(["-", p]);
  }

  let result = ["polynomial", p[1], []];
  let p_terms = p[2];
  let result_terms = result[2];

  let len = p_terms.length;

  for (var i = 0; i < len; i = i + 1) {
    result_terms.push([p_terms[i][0], polynomial_neg(p_terms[i][1])]);
  }

  return result;
}

function terms_poly_neg(p) {
  if (!Array.isArray(p) || p[0] !== "polynomial_terms") {
    return simplify.simplify(["-", p]);
  }

  let result = ["polynomial_terms"];
  let len = p.length;

  for (var i = 1; i < len; i = i + 1) {
    if (!Array.isArray(p[i]) || p[i][0] !== "monomial")
      result.push(simplify.simplify(["-", p[i]]));
    else {
      result.push(["monomial", simplify.simplify(["-", p[i][1]]), p[i][2]]);
    }
  }
  return result;
}

function polynomial_sub(p, q) {
  return polynomial_add(p, polynomial_neg(q));
}

function terms_poly_sub(p, q) {
  return terms_poly_add(p, terms_poly_neg(q));
}

function polynomial_mul(p, q) {
  if (p[0] !== "polynomial") {
    if (q[0] !== "polynomial") {
      return simplify.simplify(["*", p, q]);
    } else if (p) {
      let prod = ["polynomial", q[1], []];
      let q_terms = q[2];
      let prod_terms = prod[2];
      for (let term of q_terms) {
        if (term[1]) prod_terms.push([term[0], polynomial_mul(p, term[1])]);
      }
      return prod;
    }
  } else {
    if (q && q[0] !== "polynomial") {
      let prod = ["polynomial", p[1], []];
      let p_terms = p[2];
      let prod_terms = prod[2];
      for (let term of p_terms) {
        if (term[1]) prod_terms.push([term[0], polynomial_mul(term[1], q)]);
      }
      return prod;
    }
  }

  // two non-constant polynomials
  // if needed, rewrite polynomials so have same first variable
  let tmp = polynomials_in_same_leading_variable(p, q);
  p = tmp[0];
  q = tmp[1];

  let p_terms = p[2];
  let q_terms = q[2];

  let prod = ["polynomial", p[1], []];
  let prod_terms = prod[2];
  let p_len = p_terms.length;
  let q_len = q_terms.length;

  //find the degrees that will occur in the product
  let degrees = [];
  for (let term_p of p_terms) {
    for (let term_q of q_terms) {
      let found = false;
      let current_deg = term_p[0] + term_q[0];
      for (let deg of degrees) {
        if (current_deg === deg) {
          found = true;
          break;
        }
      }
      if (!found) degrees.push(current_deg);
    }
  }

  degrees.sort(function (a, b) {
    return a - b;
  });

  //this is where the product is computed
  for (let deg of degrees) {
    let sum = 0;
    let i = 0;
    while (i < p_len && p_terms[i][0] <= deg) {
      let j = 0;
      while (j < q_len && q_terms[j][0] <= deg) {
        if (p_terms[i][0] + q_terms[j][0] === deg) {
          sum = polynomial_add(
            sum,
            polynomial_mul(p_terms[i][1], q_terms[j][1]),
          );
          break;
        }
        j = j + 1;
      }
      i = i + 1;
    }
    if (sum) prod_terms.push([deg, sum]);
  }

  return prod;
}

function mono_mul(p, q) {
  if (!Array.isArray(p) || p[0] !== "monomial") {
    if (!Array.isArray(q) || q[0] !== "monomial")
      return simplify.simplify(["*", p, q]);
    return ["monomial", simplify.simplify(["*", p, q[1]]), q[2]];
  }

  if (!Array.isArray(q) || q[0] !== "monomial")
    return ["monomial", simplify.simplify(["*", p[1], q]), p[2]];

  let var_terms = [];
  let len_p_terms = p[2].length;
  let len_q_terms = q[2].length;
  let i = 0;
  let j = 0;

  while (i < len_p_terms && j < len_q_terms) {
    if (default_order.compare_function(p[2][i][0], q[2][j][0]) < 0) {
      var_terms.push(p[2][i]);
      i = i + 1;
    } else if (default_order.compare_function(q[2][j][0], p[2][i][0]) < 0) {
      var_terms.push(q[2][j]);
      j = j + 1;
    } else {
      var_terms.push([p[2][i][0], p[2][i][1] + q[2][j][1]]);
      i = i + 1;
      j = j + 1;
    }
  }

  while (i < len_p_terms) {
    var_terms.push(p[2][i]);
    i = i + 1;
  }

  while (j < len_q_terms) {
    var_terms.push(q[2][j]);
    j = j + 1;
  }

  return ["monomial", simplify.simplify(["*", p[1], q[1]]), var_terms];
}

function terms_poly_mul(p, q) {
  if (!Array.isArray(p) || p[0] !== "polynomial_terms") {
    if (!Array.isArray(q) || q[0] !== "polynomial_terms")
      return simplify.simplify(["*", p, q]);
    let prod = ["polynomial_terms"];
    let len_q = q.length;
    for (let j = 1; j < len_q; j = j + 1) {
      prod.push(mono_mul(q[j], p));
    }
    return prod;
  }

  if (!Array.isArray(q) || q[0] !== "polynomial_terms") {
    let prod = ["polynomial_terms"];
    let len_p = p.length;
    for (let i = 1; i < len_p; i = i + 1) {
      prod.push(mono_mul(q, p[i]));
    }
    return prod;
  }

  let prod = [];
  let len_p = p.length;
  let len_q = q.length;
  for (let i = 1; i < len_p; i = i + 1) {
    for (let j = 1; j < len_q; j = j + 1) {
      prod.push(mono_mul(p[i], q[j]));
    }
  }

  prod.sort(function (a, b) {
    if (mono_less_than(a, b)) return 1;
    if (mono_less_than(b, a)) return -1;
    return 0;
  });

  let result = ["polynomial_terms"];
  let len = prod.length;
  result.push(prod[0]);
  let i = 1;
  let j = 1;
  while (i < len) {
    if (mono_less_than(prod[i], result[j])) {
      result.push(prod[i]);
      j = j + 1;
      i = i + 1;
    } else {
      if (!Array.isArray(result[j]) || result[j][0] !== "monomial")
        result[j] = simplify.simplify(["+", result[j], prod[i]]);
      else
        result[j] = [
          "monomial",
          simplify.simplify(["+", result[j][1], prod[i][1]]),
          result[j][2],
        ];
      i = i + 1;
    }
  }

  return result;
}

function polynomial_pow(p, e) {
  if (isNaN(e) || e < 0 || !Number.isInteger(e)) return undefined;

  let res = 1;

  while (e > 0) {
    if (e & 1) {
      // odd exponent
      res = polynomial_mul(res, p);
    }

    p = polynomial_mul(p, p);

    e >>= 1; // divide by 2 and truncate
  }

  return res;
}

function polynomial_to_expression(p) {
  if (!Array.isArray(p) || p[0] !== "polynomial") return p;

  let x = p[1];
  let terms = p[2];

  let result = [];

  let len_terms = terms.length;
  for (var i = 0; i < len_terms; i = i + 1) {
    if (terms[i][1]) {
      if (terms[i][0] === 0) result.push(polynomial_to_expression(terms[i][1]));
      else if (terms[i][0] === 1)
        result.push(["*", polynomial_to_expression(terms[i][1]), x]);
      else
        result.push([
          "*",
          polynomial_to_expression(terms[i][1]),
          ["^", x, terms[i][0]],
        ]);
    }
  }

  if (result.length === 0) return 0;
  else if (result.length === 1) result = result[0];
  else result.unshift("+");

  return simplify.simplify(result);
}

function initial_term(p) {
  //takes a polynomial ["polynomial", "var"...] and returns the initial term according to lexicographic order, in form ["monomial", coefficient, [[variable1, power1], ...]]

  if (!Array.isArray(p) || p[0] !== "polynomial") return p;

  let var_powers = [];

  while (Array.isArray(p) && p[0] === "polynomial") {
    let x = p[1];
    let terms = p[2];
    let exp = terms[terms.length - 1][0];
    p = terms[terms.length - 1][1];
    var_powers.push([x, exp]);
  }

  return ["monomial", p, var_powers];
}

function pt_initial_term(p) {
  if (!Array.isArray(p) || p[0] !== "polynomial_terms") return p;

  return p[1];
}

function mono_less_than(left, right) {
  //takes two monomials ["monomial", coeff, terms array] and returns true if left is less than right in lexicographic order.
  //stringify vars before calling this

  if (!Array.isArray(right) || right[0] !== "monomial") return false; //if right is constant, always false

  if (!Array.isArray(left) || left[0] !== "monomial") return true; //if left is constant and right is not, always true

  let left_vars = left[2];
  let right_vars = right[2];
  let left_length = left_vars.length;
  let right_length = right_vars.length;
  var shorter;
  if (left_length < right_length) shorter = left_length;
  else shorter = right_length;

  for (var i = 0; i < shorter; i++) {
    if (left_vars[i][0] !== right_vars[i][0]) {
      if (
        default_order.compare_function(left_vars[i][0], right_vars[i][0]) < 0
      ) {
        // left variable is earlier in default order
        return false;
      } else {
        // right variable is earlier in default order
        return true;
      }
    }
    if (left_vars[i][1] < right_vars[i][1]) {
      // left power is lower
      return true;
    }
    if (left_vars[i][1] > right_vars[i][1]) {
      // right power is lower
      return false;
    }
  }
  if (left_length === right_length || shorter === right_length) {
    // same monomial, except possibly coefficient, or same until left is longer
    return false;
  } else {
    // same until right is longer
    return true;
  }
}

function mono_gcd(left, right) {
  //takes two monomials ["monomial", coeff, terms array] and returns their greatest common divisor as a monomial
  //stringify vars before calling this

  if (
    !Array.isArray(left) ||
    !Array.isArray(right) ||
    left[0] !== "monomial" ||
    right[0] !== "monomial"
  )
    return 1; //if either is constant, gcd is 1

  let left_vars = left[2];
  let right_vars = right[2];
  let gcd_vars = [];
  let left_length = left_vars.length;
  let right_length = right_vars.length;

  let i = 0;
  let j = 0;
  while (i < left_length && j < right_length) {
    if (left_vars[i][0] === right_vars[j][0]) {
      if (left_vars[i][1] < right_vars[j][1]) {
        gcd_vars.push(left_vars[i]);
      } else {
        gcd_vars.push(right_vars[j]);
      }
      i = i + 1;
      j = j + 1;
    } else if (
      default_order.compare_function(left_vars[i][0], right_vars[j][0]) < 0
    ) {
      i = i + 1;
    } else if (
      default_order.compare_function(right_vars[j][0], left_vars[i][0]) < 0
    ) {
      j = j + 1;
    }
  }

  if (gcd_vars.length === 0) return 1; //if they have no common variables, gcd is 1

  return ["monomial", 1, gcd_vars];
}

function mono_div(top, bottom) {
  //!!This function should only be called if bottom has coefficient 1 and divides the top (e.g., bottom was computed using mono_gcd)!!
  //takes two monomials ["monomial", coeff, terms array] and returns their quotient as a monomial
  //stringify vars before calling this

  if (!Array.isArray(bottom) || bottom[0] !== "monomial") {
    //if bottom is constant
    if (bottom === 1) return top;
    if (!Array.isArray(top) || top[0] !== "monomial")
      //if top is constant
      return simplify.evaluate_numbers(["/", top, bottom]);
    else
      return [top[0], simplify.evaluate_numbers(["/", top[1], bottom]), top[2]]; //shouldn't be passing constants other than 1
  }

  if (!Array.isArray(top) || top[0] !== "monomial")
    //if top is constant and bottom is not
    return undefined;

  let top_vars = top[2];
  let bottom_vars = bottom[2];
  let div_vars = [];
  let top_length = top_vars.length;
  let bottom_length = bottom_vars.length;

  let i = 0;
  let j = 0;
  while (i < top_length && j < bottom_length) {
    if (top_vars[i][0] === bottom_vars[j][0]) {
      if (top_vars[i][1] < bottom_vars[j][1]) {
        return undefined; //does not divide
      } else {
        let diff = top_vars[i][1] - bottom_vars[j][1];
        if (diff !== 0) div_vars.push([top_vars[i][0], diff]);
      }
      i = i + 1;
      j = j + 1;
    } else if (
      default_order.compare_function(top_vars[i][0], bottom_vars[j][0]) < 0
    ) {
      div_vars.push(top_vars[i]);
      i = i + 1;
    } else if (
      default_order.compare_function(bottom_vars[j][0], top_vars[i][0]) < 0
    ) {
      return undefined; //does not divide
    }
  }

  if (j < bottom_length) return undefined;

  while (i < top_length) {
    div_vars.push(top_vars[i]);
    i = i + 1;
  }

  if (div_vars.length === 0) {
    if (bottom[1] === 1)
      return top[1]; //everything canceled, return coefficient of the top
    else return simplify.evaluate_numbers(["/", top[1], bottom[1]]);
  }

  if (bottom[1] === 1) return ["monomial", top[1], div_vars];
  else
    return [
      "monomial",
      simplify.evaluate_numbers(["/", top[1], bottom[1]]),
      div_vars,
    ];
}

function mono_is_div(top, bottom) {
  //takes two monomials ["monomial", coeff, terms array] and returns true if bottom divides top, otherwise returns false
  //stringify vars before calling this

  if (bottom === 0) return false;

  if (!Array.isArray(bottom) || bottom[0] !== "monomial") {
    //if bottom is nonzero constant
    return true;
  }

  if (!Array.isArray(top) || top[0] !== "monomial")
    //if top is constant and bottom is not
    return false;

  let top_vars = top[2];
  let bottom_vars = bottom[2];
  let div_vars = [];
  let top_length = top_vars.length;
  let bottom_length = bottom_vars.length;

  let i = 0;
  let j = 0;
  while (i < top_length && j < bottom_length) {
    if (top_vars[i][0] === bottom_vars[j][0]) {
      if (top_vars[i][1] < bottom_vars[j][1]) {
        return false; //does not divide
      } else {
        let diff = top_vars[i][1] - bottom_vars[j][1];
        if (diff !== 0) div_vars.push([top_vars[i][0], diff]);
      }
      i = i + 1;
      j = j + 1;
    } else if (
      default_order.compare_function(top_vars[i][0], bottom_vars[j][0]) < 0
    ) {
      div_vars.push(top_vars[i]);
      i = i + 1;
    } else if (
      default_order.compare_function(bottom_vars[j][0], top_vars[i][0]) < 0
    ) {
      return false; //does not divide
    }
  }

  if (j < bottom_length) return false;

  return true;
}

function mono_to_poly(mono) {
  //takes a monomial ["monomial", coeff, terms array] and returns the corresponding polynomial ["polynomial", var1, ...]

  if (!Array.isArray(mono) || mono[0] !== "monomial") return mono; //if constant, just return itself

  let num_vars = mono[2].length;
  let i = num_vars - 1;
  let result = mono[1];
  let index = 0;

  while (i >= 0) {
    var coeffs = [];
    coeffs.push([mono[2][i][1], result]);
    result = ["polynomial", mono[2][i][0], coeffs];
    i = i - 1;
  }

  return result;
}

function mono_to_pt(mono) {
  if (!Array.isArray(mono) || mono[0] !== "monomial") return mono;

  return ["polynomial_terms", mono];
}

function max_div_init(f, monos) {
  //f is a polynomial ["polynomial", ...], monos is array of monomials ["monomial", ...]. returns the largest term (a monomial) of f divisible by something
  //in monos, and the index of the divisor.
  //stringify vars before calling this

  if (f === 0) {
    return 0;
  }
  let focus = f;
  let var_powers = [];

  while (Array.isArray(focus) && focus[0] === "polynomial") {
    let x = focus[1];
    let terms = focus[2];
    let exp = terms[terms.length - 1][0];
    focus = terms[terms.length - 1][1];
    var_powers.push([x, exp]);
  }

  let current_term = ["monomial", focus, var_powers];

  let monos_size = monos.length;
  for (var i = 0; i < monos_size; i = i + 1) {
    if (mono_is_div(current_term, monos[i])) {
      return [current_term, i];
    }
  }

  return max_div_init(polynomial_sub(f, mono_to_poly(current_term)), monos);
}

function pt_max_div_init(f, monos) {
  if (f === 0) return 0;

  if (!Array.isArray(f) || f[0] !== "polynomial_terms") {
    let monos_size = monos.length;
    for (let i = 0; i < monos_size; i = i + 1) {
      if (mono_is_div(f, monos[i])) return [f, i];
    }
    return 0;
  }

  let len = f.length;
  let monos_size = monos.length;
  for (let j = 1; j < len; j = j + 1) {
    for (let i = 0; i < monos_size; i = i + 1) {
      if (mono_is_div(f[j], monos[i])) return [f[j], i];
    }
  }

  return 0;
}

function poly_div(f, divs) {
  //takes a polynomial f and an array of polynomials div = [g1,g2,...], and returns a standard expression (according to mult. division) of the form [[[s1,m1],[s2,m2],....], f'] where f=m1g_{s1}+m2g_{s2}+...+f'
  //stringify vars before calling this

  let inits = [];
  let su_mu = [];
  let sp = [];
  let mp = [];
  let f_prime = f;

  for (var g of divs) {
    inits.push(initial_term(g));
  }

  let m = max_div_init(f_prime, inits);

  while (m !== 0) {
    sp = m[1];
    mp = mono_div(m[0], inits[sp]);
    su_mu.push([sp, mp]);
    f_prime = polynomial_sub(
      f_prime,
      polynomial_mul(mono_to_poly(mp), divs[sp]),
    );
    m = max_div_init(f_prime, inits);
  }

  return [su_mu, f_prime];
}

function pt_poly_div(f, divs) {
  let inits = [];
  let su_mu = [];
  let sp = [];
  let mp = [];
  let f_prime = f;

  for (var g of divs) {
    inits.push(pt_initial_term(g));
  }

  let m = pt_max_div_init(f_prime, inits);

  while (m !== 0) {
    sp = m[1];
    mp = mono_div(m[0], inits[sp]);
    su_mu.push([sp, mp]);
    f_prime = terms_poly_sub(f_prime, terms_poly_mul(mono_to_pt(mp), divs[sp]));
    m = pt_max_div_init(f_prime, inits);
  }

  return [su_mu, f_prime];
}

function prereduce(polys) {
  //takes an array of polys, and does some simply reductions: gets rid of 0 polynomials, if there's a constant: just return [1], if there are no nonzero polys: return [0].

  let len = polys.length;
  let new_polys = [];

  //check for 0's, constants
  for (var j = 0; j < len; j++) {
    if (
      polys[j] !== 0 &&
      (!Array.isArray(polys[j]) || polys[j][0] !== "polynomial")
    ) {
      return [1]; //if there's a nonzero constant, return [1]
    }
    if (polys[j] !== 0) {
      new_polys.push(polys[j]);
    }
  }

  if (new_polys.length === 0) return [0];

  return new_polys;
}

function pt_prereduce(polys) {
  //takes an array of polys, and does some simply reductions: gets rid of 0 polynomials, if there's a constant: just return [1], if there are no nonzero polys: return [0].

  let len = polys.length;
  let new_polys = [];

  //check for 0's, constants
  for (var j = 0; j < len; j++) {
    if (
      polys[j] !== 0 &&
      (!Array.isArray(polys[j]) || polys[j][0] !== "polynomial_terms")
    ) {
      return [1]; //if there's a nonzero constant, return [1]
    }
    if (polys[j] !== 0) {
      new_polys.push(polys[j]);
    }
  }

  if (new_polys.length === 0) return [0];

  return new_polys;
}

function reduce_ith(i, polys) {
  //takes an index i and an array polys of polynomials, and reduces the ith polynomial wrt the rest (i.e., finds a std expression and replaces with f'). Returns the reduced polynomial.
  //stringify vars before calling this

  let inits = [];
  let su_mu = [];
  let sp = [];
  let mp = [];
  let f_prime = polys[i];
  let len = polys.length;

  for (var j = 0; j < len; j = j + 1) {
    if (j === i)
      //don't want to cancel out with itself, so put 0 for this initial term instead to avoid it being used
      inits.push(0);
    else inits.push(initial_term(polys[j]));
  }

  let m = max_div_init(f_prime, inits);

  while (m !== 0) {
    sp = m[1];
    mp = mono_div(m[0], inits[sp]);
    su_mu.push([sp, mp]);
    f_prime = polynomial_sub(
      f_prime,
      polynomial_mul(mono_to_poly(mp), polys[sp]),
    );
    m = max_div_init(f_prime, inits);
  }

  return f_prime;

  /*  OLD CODE:

     if (!Array.isArray(polys) || i >= polys.length){
        return undefined;
    }

    let len = polys.length;
    let new_polys = [];

    //check for 0's, constants
    for (var j = 0; j < len; j++ ){
        if (polys[j] !== 0 && (!Array.isArray(polys[j]) || polys[j][0] !== "polynomial")){
            return 1;       //if there's a nonzero constant, return [1]
        }
        if (polys[j] !== 0){
            new_polys.push(polys[j]);
        }
    }

    len = new_polys.length;

    if (len === 0)
        return 0;         //if there were no nonzero polys, return [0]

    if (len === 1)
        return new_polys[0];           //if there's only one poly, don't need to reduce

    let others = [];
    for ( var j = 0; j < len; j++ ){
        if (j !== i)
            others.push(new_polys[j]);
    }

    return poly_div(new_polys[i], others)[1];*/
}

function pt_reduce_ith(i, polys) {
  //takes an index i and an array polys of polynomials, and reduces the ith polynomial wrt the rest (i.e., finds a std expression and replaces with f'). Returns the reduced polynomial.
  //stringify vars before calling this

  let inits = [];
  let su_mu = [];
  let sp = [];
  let mp = [];
  let f_prime = polys[i];
  let len = polys.length;

  for (var j = 0; j < len; j = j + 1) {
    if (j === i)
      //don't want to cancel out with itself, so put 0 for this initial term instead to avoid it being used
      inits.push(0);
    else inits.push(pt_initial_term(polys[j]));
  }

  let m = pt_max_div_init(f_prime, inits);

  while (m !== 0) {
    sp = m[1];
    mp = mono_div(m[0], inits[sp]);
    su_mu.push([sp, mp]);
    f_prime = terms_poly_sub(
      f_prime,
      terms_poly_mul(mono_to_pt(mp), polys[sp]),
    );
    m = pt_max_div_init(f_prime, inits);
  }

  return f_prime;

  /*  OLD CODE:

     if (!Array.isArray(polys) || i >= polys.length){
     return undefined;
     }

     let len = polys.length;
     let new_polys = [];

     //check for 0's, constants
     for (var j = 0; j < len; j++ ){
     if (polys[j] !== 0 && (!Array.isArray(polys[j]) || polys[j][0] !== "polynomial")){
     return 1;       //if there's a nonzero constant, return [1]
     }
     if (polys[j] !== 0){
     new_polys.push(polys[j]);
     }
     }

     len = new_polys.length;

     if (len === 0)
     return 0;         //if there were no nonzero polys, return [0]

     if (len === 1)
     return new_polys[0];           //if there's only one poly, don't need to reduce

     let others = [];
     for ( var j = 0; j < len; j++ ){
     if (j !== i)
     others.push(new_polys[j]);
     }

     return poly_div(new_polys[i], others)[1];*/
}

function reduce(polys) {
  //takes an array of polynomials, and reduces them with respect to each other until they can't be reduced anymore. Returns array of reduced polynomials.
  //this could be made more efficient with better bookkeeping if necessary - currently copying sub-arrays a lot, whenever call reduce_ith. Would need to track changes in sub-arrays.
  //stringify vars before calling this

  let i = 0;
  let h = [];
  let new_polys = prereduce(polys);
  let len = new_polys.length;

  if (len === 1) return new_polys; //if there's only one poly, don't need to reduce
  /*prereduce more frequently:
    while (i < len){
        h = reduce_ith(i, new_polys);
        if ( _.isEqual( h, new_polys[i] )){      //from underscore lib to compare arrays with objects
            i=i+1;
        }
        else{
            new_polys[i] = h;
            i = 0;
            new_polys = prereduce(new_polys);
            len = new_polys.length;
        }
    }*/

  /*      This code prereduces less frequently, maybe slightly faster?*/
  let trigger = true;
  while (trigger) {
    trigger = false;
    new_polys = prereduce(new_polys);
    len = new_polys.length;
    i = 0;
    while (i < len) {
      h = reduce_ith(i, new_polys);
      if (!_.isEqual(h, new_polys[i])) {
        //from underscore lib to compare arrays with objects
        new_polys[i] = h;
        trigger = true;
      }
      i = i + 1;
    }
  }

  i = 0;
  let init = [];
  let coeff = 0;
  while (i < len) {
    //leading coeffs should be 1
    init = initial_term(new_polys[i]);
    if (!Array.isArray(init) || init[0] !== "monomial") new_polys[i] = 1;
    else {
      coeff = init[1];
      if (coeff !== 1)
        new_polys[i] = polynomial_mul(new_polys[i], ["/", 1, coeff]);
    }
    i = i + 1;
  }

  return new_polys;
}

function pt_reduce(polys) {
  //takes an array of polynomials, and reduces them with respect to each other until they can't be reduced anymore. Returns array of reduced polynomials.
  //this could be made more efficient with better bookkeeping if necessary - currently copying sub-arrays a lot, whenever call reduce_ith. Would need to track changes in sub-arrays.
  //stringify vars before calling this

  let i = 0;
  let h = [];
  let new_polys = pt_prereduce(polys);
  let len = new_polys.length;

  if (len === 1) return new_polys; //if there's only one poly, don't need to reduce
  /*prereduce more frequently:
     while (i < len){
     h = reduce_ith(i, new_polys);
     if ( _.isEqual( h, new_polys[i] )){      //from underscore lib to compare arrays with objects
     i=i+1;
     }
     else{
     new_polys[i] = h;
     i = 0;
     new_polys = prereduce(new_polys);
     len = new_polys.length;
     }
     }*/

  /*      This code prereduces less frequently, maybe slightly faster?*/
  let trigger = true;
  while (trigger) {
    trigger = false;
    new_polys = pt_prereduce(new_polys);
    len = new_polys.length;
    i = 0;
    while (i < len) {
      h = pt_reduce_ith(i, new_polys);
      if (!_.isEqual(h, new_polys[i])) {
        //from underscore lib to compare arrays with objects
        new_polys[i] = h;
        trigger = true;
      }
      i = i + 1;
    }
  }

  i = 0;
  let init = [];
  let coeff = 0;
  while (i < len) {
    //leading coeffs should be 1
    init = pt_initial_term(new_polys[i]);
    if (!Array.isArray(init) || init[0] !== "monomial") new_polys[i] = 1;
    else {
      coeff = init[1];
      if (coeff !== 1)
        new_polys[i] = terms_poly_mul(new_polys[i], ["/", 1, coeff]);
    }
    i = i + 1;
  }

  return new_polys;
}

function hij(i, j, polys) {
  //takes indices i,j and an array of polynomials. Returns the polynomial hij computed from the ith and jth polys in polynomials, for Buchberger's criterion.
  //stringify vars before calling this

  let init_gi = initial_term(polys[i]);
  let init_gj = initial_term(polys[j]);
  let gcd = mono_gcd(init_gi, init_gj);
  let mij = mono_to_poly(mono_div(init_gi, gcd));
  let mji = mono_to_poly(mono_div(init_gj, gcd));
  let std_exp = poly_div(
    polynomial_sub(
      polynomial_mul(mji, polys[i]),
      polynomial_mul(mij, polys[j]),
    ),
    polys,
  );
  return std_exp[1];
}

function pt_hij(i, j, polys) {
  //takes indices i,j and an array of polynomials. Returns the polynomial hij computed from the ith and jth polys in polynomials, for Buchberger's criterion.
  //stringify vars before calling this

  let init_gi = pt_initial_term(polys[i]);
  let init_gj = pt_initial_term(polys[j]);
  let gcd = mono_gcd(init_gi, init_gj);
  let mij = mono_to_pt(mono_div(init_gi, gcd));
  let mji = mono_to_pt(mono_div(init_gj, gcd));
  let std_exp = pt_poly_div(
    terms_poly_sub(
      terms_poly_mul(mji, polys[i]),
      terms_poly_mul(mij, polys[j]),
    ),
    polys,
  );
  return std_exp[1];
}

function build_hij_table(table, polys) {
  //takes an empty table and an array of polynomials, and fills in table where table[[i,j]]=h_ij for i < j < polys.length. (Look up h_ij based on the index in polys) returns first choice for h
  //stringify vars before calling this

  let len = polys.length;
  let i = 0;
  let j = 1;

  let candidates = [];

  while (j < len) {
    while (i < j) {
      table[[i, j]] = hij(i, j, polys);
      if (table[[i, j]] !== 0) candidates.push(table[[i, j]]);
      i = i + 1;
    }
    i = 0;
    j = j + 1;
  }

  if (candidates.length > 0) return choice_lowest_degree(candidates); //call different choice functions here. Could avoid iterating through again if check as go.

  return 0;
}

function pt_build_hij_table(table, polys) {
  //takes an empty table and an array of polynomials, and fills in table where table[[i,j]]=h_ij for i < j < polys.length. (Look up h_ij based on the index in polys) returns first choice for h
  //stringify vars before calling this

  let len = polys.length;
  let i = 0;
  let j = 1;

  let candidates = [];

  while (j < len) {
    while (i < j) {
      table[[i, j]] = pt_hij(i, j, polys);
      if (table[[i, j]] !== 0) candidates.push(table[[i, j]]);
      i = i + 1;
    }
    i = 0;
    j = j + 1;
  }

  if (candidates.length > 0) return pt_choice_lowest_degree(candidates); //call different choice functions here. Could avoid iterating through again if check as go.

  return 0;
}

function update_hij_table(table, polys, h) {
  //takes an hij table, array of polynomials that the table was built for, and a poynomial h. Updates the hij values for the addition of the polynomial h, h is added with the highest index. return next choice of h.

  let len = polys.length;
  polys.push(h);
  let i = 0;
  let j = 1;

  let candidates = [];
  while (j < len) {
    //this loop updates old hij values.
    while (i < j) {
      if (table[[i, j]] !== 0) {
        table[[i, j]] = hij(i, j, polys);
        if (table[[i, j]] !== 0) candidates.push(table[[i, j]]);
      }
      i = i + 1;
    }
    i = 0;
    j = j + 1;
  }

  while (i < len) {
    //this loop adds values with h
    table[[i, len]] = hij(i, len, polys);
    if (table[[i, len]] !== 0) candidates.push(table[[i, len]]);
    i = i + 1;
  }

  if (candidates.length > 0) return choice_lowest_degree(candidates); //call different choice functions here. Could avoid iterating through again if check as go.

  return 0;
}

function pt_update_hij_table(table, polys, h) {
  //takes an hij table, array of polynomials that the table was built for, and a poynomial h. Updates the hij values for the addition of the polynomial h, h is added with the highest index. return next choice of h.

  let len = polys.length;
  polys.push(h);
  let i = 0;
  let j = 1;

  let candidates = [];
  while (j < len) {
    //this loop updates old hij values.
    while (i < j) {
      if (table[[i, j]] !== 0) {
        table[[i, j]] = pt_hij(i, j, polys);
        if (table[[i, j]] !== 0) candidates.push(table[[i, j]]);
      }
      i = i + 1;
    }
    i = 0;
    j = j + 1;
  }

  while (i < len) {
    //this loop adds values with h
    table[[i, len]] = pt_hij(i, len, polys);
    if (table[[i, len]] !== 0) candidates.push(table[[i, len]]);
    i = i + 1;
  }

  if (candidates.length > 0) return pt_choice_lowest_degree(candidates); //call different choice functions here. Could avoid iterating through again if check as go.

  return 0;
}

function choice_first(polys) {
  return polys[0];
}

function choice_lowest_degree(polys) {
  //takes an array of polynomials, returns the polynomial of lowest degree.

  let inits = [];
  let len = polys.length;

  for (i = 0; i < len; i = i + 1) {
    inits.push(initial_term(polys[i]));
  }

  let min_index = 0;
  for (var i = 1; i < len; i = i + 1) {
    if (inits[i] === 1) return 1;
    if (mono_less_than(inits[i], inits[min_index])) min_index = i;
  }

  return polys[min_index];
}

function pt_choice_lowest_degree(polys) {
  //takes an array of polynomials, returns the polynomial of lowest degree.

  let inits = [];
  let len = polys.length;

  for (i = 0; i < len; i = i + 1) {
    inits.push(pt_initial_term(polys[i]));
  }

  let min_index = 0;
  for (var i = 1; i < len; i = i + 1) {
    if (inits[i] === 1) return 1;
    if (mono_less_than(inits[i], inits[min_index])) min_index = i;
  }

  return polys[min_index];
}

function reduced_grobner(polys) {
  //takes an array of polynomials, returns reduced grobner basis of the ideal they generate.
  //stringify vars before calling this

  let new_polys = reduce(polys);

  let table = {};
  let h = build_hij_table(table, new_polys);
  while (h !== 0) {
    h = update_hij_table(table, new_polys, h);
  }

  new_polys = reduce(new_polys);

  return new_polys;
}

function pt_reduced_grobner(polys) {
  //takes an array of polynomials, returns reduced grobner basis of the ideal they generate.
  //stringify vars before calling this

  let new_polys = pt_reduce(polys);

  let table = {};
  let h = pt_build_hij_table(table, new_polys);
  while (h !== 0) {
    h = pt_update_hij_table(table, new_polys, h);
  }

  new_polys = pt_reduce(new_polys);

  return new_polys;
}

function poly_lcm(f, g) {
  //takes two polynomials f and g, and returns their least common multiple.
  //stringify vars before calling this

  let t = ["polynomial", "_t", [[1, 1]]];
  let one_minus_t = [
    "polynomial",
    "_t",
    [
      [0, 1],
      [1, -1],
    ],
  ];
  let grob = reduced_grobner([
    polynomial_mul(t, f),
    polynomial_mul(one_minus_t, g),
  ]);

  //find term without _t
  let len = grob.length;
  for (var i = 0; i < len; i = i + 1) {
    if (!Array.isArray(grob[i]) || grob[i][0] !== "polynomial") {
      return 1; //if there is a constant in the grobner basis, return 1 (shouldn't have constants other than 1, so could probably just check for 1)
    }
    if (grob[i][1] !== "_t") return grob[i];
  }

  return undefined; //this should never be reached, unless something bad happens?
}

function pt_poly_lcm(f, g) {
  //takes two polynomials f and g, and returns their least common multiple.
  //stringify vars before calling this

  let t = ["polynomial_terms", ["monomial", 1, [["_t", 1]]]];
  let one_minus_t = ["polynomial_terms", ["monomial", -1, [["_t", 1]]], 1];
  let grob = pt_reduced_grobner([
    terms_poly_mul(t, f),
    terms_poly_mul(one_minus_t, g),
  ]);

  //find term without _t
  let len = grob.length;
  for (var i = 0; i < len; i = i + 1) {
    if (!Array.isArray(grob[i]) || grob[i][0] !== "polynomial_terms") {
      return 1; //if there is a constant in the grobner basis, return 1 (shouldn't have constants other than 1, so could probably just check for 1)
    }
    if (grob[i][1][2][0][0] !== "_t") return grob[i];
  }

  return undefined; //this should never be reached, unless something bad happens?
}

function poly_gcd(f, g) {
  //takes two polynomials f and g, and returns their greatest common divisor.
  //stringify vars before calling this

  let lcm = poly_lcm(f, g);
  let fg = polynomial_mul(f, g);
  let std_exp = poly_div(fg, [lcm]);

  let sum = 0;
  let len = std_exp[0].length;
  for (var i = 0; i < len; i = i + 1) {
    sum = polynomial_add(sum, mono_to_poly(std_exp[0][i][1]));
  }

  //divide by leading coeff, so leading coeff of gcd is 1
  let init = initial_term(sum);
  if (!Array.isArray(init) || init[0] !== "monomial") sum = 1;
  else {
    let coeff = init[1];
    if (coeff !== 1) sum = polynomial_mul(sum, ["/", 1, coeff]);
  }

  return sum;
}

function pt_poly_gcd(f, g) {
  //takes two polynomials f and g, and returns their greatest common divisor.
  //stringify vars before calling this

  let lcm = pt_poly_lcm(f, g);
  let fg = terms_poly_mul(f, g);
  let std_exp = pt_poly_div(fg, [lcm]);

  let sum = 0;
  let len = std_exp[0].length;
  for (var i = 0; i < len; i = i + 1) {
    sum = terms_poly_add(sum, mono_to_pt(std_exp[0][i][1]));
  }

  //divide by leading coeff, so leading coeff of gcd is 1
  let init = pt_initial_term(sum);
  if (!Array.isArray(init) || init[0] !== "monomial") sum = 1;
  else {
    let coeff = init[1];
    if (coeff !== 1) sum = terms_poly_mul(sum, ["/", 1, coeff]);
  }

  return sum;
}

function poly_by_divisor(f, d) {
  //takes two polynomials f and d (where d is a divisor of f), and returns f divided by d.
  //!!Only call if d evenly divides f!!
  //stringify vars before calling this

  let std_exp = poly_div(f, [d]);
  let sum = 0;
  let len = std_exp[0].length;
  for (var i = 0; i < len; i = i + 1) {
    sum = polynomial_add(sum, mono_to_poly(std_exp[0][i][1]));
  }
  return sum;
}

function pt_poly_by_divisor(f, d) {
  //takes two polynomials f and d (where d is a divisor of f), and returns f divided by d.
  //!!Only call if d evenly divides f!!
  //stringify vars before calling this

  let std_exp = pt_poly_div(f, [d]);
  let sum = 0;
  let len = std_exp[0].length;
  for (var i = 0; i < len; i = i + 1) {
    sum = terms_poly_add(sum, mono_to_pt(std_exp[0][i][1]));
  }
  return sum;
}

function reduce_rational_expression(top, bottom) {
  //input: top and bottom of a rational expression. top and bottom should be polynomials, ["polynomial", ...]. returns an array with two entries: new_top and new_bottom, which are reduced (gcd of new_top and new_bottom is 1). new_bottom will always have leading coefficient 1 (according to lexicographic order)

  let stringy_top = stringify_vars(top);
  let stringy_bottom = stringify_vars(bottom);
  top = stringy_top[0];
  bottom = stringy_bottom[0];

  let gcd = poly_gcd(top, bottom);
  let denom_coeff = initial_term(bottom)[1];
  let div = polynomial_mul(gcd, denom_coeff);
  let new_top = poly_by_divisor(top, div);
  let new_bottom = poly_by_divisor(bottom, div);
  return [
    destringify_vars(
      destringify_vars(new_top, stringy_top[1]),
      stringy_bottom[1],
    ),
    destringify_vars(
      destringify_vars(new_bottom, stringy_top[1]),
      stringy_bottom[1],
    ),
  ];
}

function pt_reduce_rational_expression(top, bottom) {
  //input: top and bottom of a rational expression. top and bottom should be polynomials, ["polynomial", ...]. returns an array with two entries: new_top and new_bottom, which are reduced (gcd of new_top and new_bottom is 1). new_bottom will always have leading coefficient 1 (according to lexicographic order)

  let stringy_top = stringify_vars(top);
  let stringy_bottom = stringify_vars(bottom);
  top = stringy_top[0];
  bottom = stringy_bottom[0];
  top = poly_to_terms(top);
  bottom = poly_to_terms(bottom);

  let top_var = sv.single_var(top);
  let bottom_var = sv.single_var(bottom);

  if (top_var === "_false" || bottom_var === "_false") {
    //if either is not single variable
    var gcd = pt_poly_gcd(top, bottom);
  } else if (top_var === "_true") {
    //if bottom is single variable, and top is constant, use single variable functions.
    let sv_top = sv.poly_to_sv(top, bottom_var);
    let sv_bottom = sv.poly_to_sv(bottom, bottom_var);
    let sv_gcd = sv.sv_gcd(sv_top, sv_bottom);
    var gcd = sv.sv_to_poly(sv_gcd);
  } else if (bottom_var === "_true") {
    //if top is single variable, and bottom is constant, use single variable functions.
    let sv_top = sv.poly_to_sv(top, top_var);
    let sv_bottom = sv.poly_to_sv(bottom, top_var);
    let sv_gcd = sv.sv_gcd(sv_top, sv_bottom);
    var gcd = sv.sv_to_poly(sv_gcd);
  } else if (top_var !== bottom_var) {
    //if they are single variable in different variables
    var gcd = pt_poly_gcd(top, bottom);
  } else {
    //if they are single variable in the same variable, use single variable functions.
    let sv_top = sv.poly_to_sv(top, top_var);
    let sv_bottom = sv.poly_to_sv(bottom, top_var);
    let sv_gcd = sv.sv_gcd(sv_top, sv_bottom);
    var gcd = sv.sv_to_poly(sv_gcd);
  }

  let denom_coeff = pt_initial_term(bottom)[1];
  let div = terms_poly_mul(gcd, denom_coeff);
  let new_top = pt_poly_by_divisor(top, div);
  let new_bottom = pt_poly_by_divisor(bottom, div);

  new_top = terms_to_poly(new_top);
  new_bottom = terms_to_poly(new_bottom);
  return [
    destringify_vars(
      destringify_vars(new_top, stringy_top[1]),
      stringy_bottom[1],
    ),
    destringify_vars(
      destringify_vars(new_bottom, stringy_top[1]),
      stringy_bottom[1],
    ),
  ];
}

function stringify_vars(f) {
  //takes a polynomial f and converts all variables to strings. Returns the new, stringy polynomial, and a dictionary to convert back to the original degrees.

  if (!Array.isArray(f) || f[0] !== "polynomial") return [f, {}]; //if it's not a polynomial, don't need to change anything.

  let table = {};
  let var_string = "z" + JSON.stringify(f[1]);
  table[var_string] = f[1];
  f[1] = var_string;

  let terms = f[2];
  let current = [];
  for (let f_term of terms) {
    current = stringify_vars(f_term[1]);
    f_term[1] = current[0];
    for (let key in current[1]) {
      table[key] = current[1][key];
    }
  }

  return [f, table];
}

function destringify_vars(f, table) {
  //takes a stringy polynomial and a dictionary {string:actual_variable,...} and returns the polynomial with strings replaced with the actual variable they represent.

  if (!Array.isArray(f) || f[0] !== "polynomial") return f; //it it's not a polynomial, don't need to change anything.

  if (table[f[1]]) f[1] = table[f[1]];

  let terms = f[2];
  for (let f_term of terms) {
    f_term[1] = destringify_vars(f_term[1], table);
  }

  return f;
}

export {
  expression_to_polynomial,
  poly_to_terms,
  terms_to_poly,
  polynomial_add,
  terms_poly_add,
  polynomial_neg,
  terms_poly_neg,
  polynomial_sub,
  terms_poly_sub,
  polynomial_mul,
  mono_mul,
  terms_poly_mul,
  polynomial_pow,
  polynomial_to_expression,
  initial_term,
  pt_initial_term,
  mono_less_than,
  mono_gcd,
  mono_div,
  mono_to_poly,
  mono_is_div,
  max_div_init,
  pt_max_div_init,
  poly_div,
  reduce_ith,
  reduce,
  hij,
  reduced_grobner,
  poly_lcm,
  poly_gcd,
  reduce_rational_expression,
  stringify_vars,
  destringify_vars,
  build_hij_table,
  update_hij_table,
  pt_reduce_rational_expression,
  pt_poly_gcd,
  pt_poly_lcm,
  pt_reduced_grobner,
  pt_reduce,
  pt_prereduce,
  pt_reduce_ith,
  mono_to_pt,
  pt_poly_div,
  pt_poly_by_divisor,
  pt_hij,
};
