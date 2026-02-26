import { get_tree } from "../trees/util.js";
import * as simplify from "../expression/simplify.js";
import math from "../mathjs.js";
import { operators as operators_in } from "../expression/variables.js";
import { evaluate_to_constant } from "../expression/evaluation.js";
import * as default_order from "../trees/default_order.js";
import _ from "underscore";

function single_var(poly) {
  //takes polynomial in terms representation ["polynomial_terms", (highest term, monomial),...,(lowest term, monomial)] returns the variable if the polynomial has only one variable, false otherwise.
  if (!Array.isArray(poly) || poly[0] !== "polynomial_terms") return "_true"; //if polynomial is constant, it's single variable.

  let len = poly.length;
  let vars = new Set();
  for (let i = 1; i < len; i = i + 1) {
    if (Array.isArray(poly[i]) && poly[i][0] === "monomial") {
      if (poly[i][2].length > 1) return "_false";
      vars.add(poly[i][2][0][0]);
      var variable = poly[i][2][0][0];
      if (vars.size > 1) return "_false";
    }
  }
  return variable;
}

function poly_to_sv(poly, variable) {
  //takes a single variable polynomial in terms representation ["polynomial_terms", (highest term, monomial),...,(lowest term, monomial)] with variable given and converts to simpler single variable representation ["sv_poly", var, [[highest deg, coeff],...,[lowest deg, coeff]]]. Should only be called on single variable polynomials.
  if (!Array.isArray(poly) || poly[0] !== "polynomial_terms") return poly; //if polynomial is constant, don't need to convert.

  let len = poly.length;
  let sv = ["sv_poly", variable, []];
  for (let i = 1; i < len; i = i + 1) {
    if (Array.isArray(poly[i]) && poly[i][0] === "monomial")
      sv[2].push([poly[i][2][0][1], poly[i][1]]);
    else sv[2].push([0, poly[i]]);
  }
  return sv;
}

function sv_to_poly(f) {
  //takes a single variable polynomial and converts to terms representation.

  if (!Array.isArray(f) || f[0] !== "sv_poly") return f; //if polynomial is constant, don't need to convert.

  let terms = f[2];
  let len = f[2].length;
  let poly = ["polynomial_terms"];
  for (let i = 0; i < len; i = i + 1) {
    poly.push(["monomial", terms[i][1], [[f[1], terms[i][0]]]]);
  }

  return poly;
}

function sv_deg(poly) {
  //takes a single variable polynomial and returns the degree.
  if (!Array.isArray(poly) || poly[0] !== "sv_poly") return 0;

  return poly[2][0][0];
}

function sv_add(f, g) {
  //takes two single variable polynomials in the same variable and returns their sum
  let coeff_sum = 0;

  if (!Array.isArray(g) || g[0] !== "sv_poly") {
    //if g is constant
    if (g === 0) {
      return f;
    }

    if (!Array.isArray(f) || f[0] !== "sv_poly") {
      //if f is also constant, return their sum as constants
      return simplify.simplify(["+", f, g]);
    }
    let sum = ["sv_poly", f[1], []];
    let len = f[2].length;
    for (let i = 0; i < len - 1; i = i + 1) {
      sum[2].push(f[2][i]);
    }
    let i = len - 1;
    if (f[2][i][0] === 0) {
      coeff_sum = simplify.simplify(["+", f[2][i][1], g]);
      if (coeff_sum !== 0) sum[2].push([0, coeff_sum]);
    } else {
      sum[2].push(f[2][i]);
      sum[2].push([0, g]);
    }
    return sum;
  }

  if (!Array.isArray(f) || f[0] !== "sv_poly") {
    //if f is constant
    if (f === 0) {
      return g;
    }

    let sum = ["sv_poly", g[1], []];
    let len = g[2].length;
    for (let i = 0; i < len - 1; i = i + 1) {
      sum[2].push(g[2][i]);
    }
    let i = len - 1;
    if (g[2][i][0] === 0) {
      coeff_sum = simplify.simplify(["+", f, g[2][i][1]]);
      if (coeff_sum !== 0) sum[2].push([0, coeff_sum]);
    } else {
      sum[2].push(g[2][i]);
      sum[2].push([0, f]);
    }
    return sum;
  }

  let sum = ["sv_poly", f[1], []];
  let len_f = f[2].length;
  let len_g = g[2].length;
  let i = 0;
  let j = 0;
  while (i < len_f && j < len_g) {
    if (f[2][i][0] > g[2][j][0]) {
      sum[2].push(f[2][i]);
      i = i + 1;
    } else if (f[2][i][0] < g[2][j][0]) {
      sum[2].push(g[2][j]);
      j = j + 1;
    } else {
      coeff_sum = simplify.simplify(["+", f[2][i][1], g[2][j][1]]);
      if (coeff_sum !== 0) sum[2].push([f[2][i][0], coeff_sum]);
      i = i + 1;
      j = j + 1;
    }
  }

  while (i < len_f) {
    sum[2].push(f[2][i]);
    i = i + 1;
  }

  while (j < len_g) {
    sum[2].push(g[2][j]);
    j = j + 1;
  }

  if (sum[2].length === 0) return 0;

  if (sum[2][0][0] === 0) return sum[2][0][1]; //if there's only a constant left, return it as a constant

  return sum;
}

function sv_neg(f) {
  //takes a single variable polynomial and returns its negation

  if (!Array.isArray(f) || f[0] !== "sv_poly")
    return simplify.simplify(["-", f]);

  let neg_f = ["sv_poly", f[1], []];
  let len = f[2].length;
  for (let i = 0; i < len; i = i + 1) {
    neg_f[2].push([f[2][i][0], simplify.simplify(["-", f[2][i][1]])]);
  }

  return neg_f;
}

function sv_sub(f, g) {
  //takes a single variable polynomial and returns the difference f-g

  return sv_add(f, sv_neg(g));
}

function sv_mul(f, g) {
  //takes two single variable polynomials and returns their product

  if (!Array.isArray(f) || f[0] !== "sv_poly") {
    if (f === 1) {
      return g;
    }

    if (f === 0) {
      return 0;
    }

    if (!Array.isArray(g) || g[0] !== "sv_poly") {
      return simplify.simplify(["*", f, g]);
    }
    let prod = ["sv_poly", g[1], []];
    let len = g[2].length;
    for (let i = 0; i < len; i = i + 1) {
      prod[2].push([g[2][i][0], simplify.simplify(["*", g[2][i][1], f])]);
    }
    return prod;
  }

  if (!Array.isArray(g) || g[0] !== "sv_poly") {
    if (g === 1) {
      return f;
    }

    if (g === 0) {
      return 0;
    }

    let prod = ["sv_poly", f[1], []];
    let len = f[2].length;
    for (let i = 0; i < len; i = i + 1) {
      prod[2].push([f[2][i][0], simplify.simplify(["*", f[2][i][1], g])]);
    }
    return prod;
  }

  let terms = [];
  let len_f = f[2].length;
  let len_g = g[2].length;
  for (let i = 0; i < len_f; i = i + 1) {
    for (let j = 0; j < len_g; j = j + 1) {
      terms.push([
        f[2][i][0] + g[2][j][0],
        simplify.simplify(["*", f[2][i][1], g[2][j][1]]),
      ]);
    }
  }

  terms.sort(function (a, b) {
    return b[0] - a[0];
  });

  let combined = [terms[0]];
  let end = 0;
  let coeff_sum = 0;
  let len = terms.length;
  for (let i = 1; i < len; i = i + 1) {
    end = combined.length - 1;
    if (terms[i][0] === combined[end][0]) {
      coeff_sum = simplify.simplify(["+", combined[end][1], terms[i][1]]);
      if (coeff_sum !== 0) combined[end][1] = coeff_sum;
      else combined.pop();
    } else combined.push(terms[i]);
  }

  return ["sv_poly", f[1], combined];
}

function sv_leading(f) {
  if (!Array.isArray(f) || f[0] !== "sv_poly") return f;
  return ["sv_poly", f[1], [f[2][0]]];
}

function sv_div_lt(term1, term2) {
  if (!Array.isArray(term2) || term2[0] !== "sv_poly") {
    if (!Array.isArray(term1) || term1[0] !== "sv_poly")
      return simplify.simplify(["/", term1, term2]);
    let coeff_ratio = simplify.simplify(["/", term1[2][0][1], term2]);
    return ["sv_poly", term1[1], [[term1[2][0][0], coeff_ratio]]];
  }

  if (!Array.isArray(term1) || term1[0] !== "sv_poly") return undefined;

  if (sv_deg(term1) < sv_deg(term2)) return undefined;

  let coeff_ratio = simplify.simplify(["/", term1[2][0][1], term2[2][0][1]]);
  let deg = term1[2][0][0] - term2[2][0][0];

  if (deg === 0) return coeff_ratio;

  return ["sv_poly", term1[1], [[deg, coeff_ratio]]];
}

function sv_div(f, g) {
  //takes single variable polynomials f,g and returns quotient and remainder [q,r] such that f = gq+r, and deg(r) < deg(g) or r=0.

  let q = 0;
  let r = f;
  let div_lt = 0;
  while (r !== 0 && sv_deg(g) <= sv_deg(r)) {
    div_lt = sv_div_lt(sv_leading(r), sv_leading(g));
    q = sv_add(q, div_lt);
    r = sv_sub(r, sv_mul(div_lt, g));
  }

  return [q, r];
}

function sv_gcd(f, g) {
  //takes single variable polynomials f,g and returns their gcd as a single variable polynomial.

  let h = f;
  let s = g;
  let rem = 0;

  while (s !== 0) {
    rem = sv_div(h, s)[1];
    h = s;
    s = rem;
  }

  if (!Array.isArray(h) || h[0] !== "sv_poly") return 1; //if gcd is constant, return 1

  let leading_coeff = h[2][0][1];
  return sv_div(h, leading_coeff)[0];
}

function sv_reduce_rational(top, bottom) {
  let gcd = gcd(top, bottom);

  if (!Array.isArray(bottom) || bottom[0] !== "sv_poly")
    var bottom_coeff = bottom; //if gcd is constant, return 1
  else var bottom_coeff = bottom[2][0][1];

  let divisor = sv_mul(bottom_coeff, gcd); //we make the leading coefficient of the bottom 1 in order to standardize the form of the rational expression.
  let new_top = sv_div(top, divisor)[0];
  let new_bottom = sv_div(bottom, divisor)[0];

  return [new_top, new_bottom];
}

export {
  single_var,
  poly_to_sv,
  sv_to_poly,
  sv_add,
  sv_neg,
  sv_sub,
  sv_mul,
  sv_leading,
  sv_div_lt,
  sv_div,
  sv_gcd,
};
