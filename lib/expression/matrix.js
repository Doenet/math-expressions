import * as simplify from "../expression/simplify.js";
import { get_tree } from "../trees/util.js";

function tuple(entries) {
  var expression = [];
  expression.push("tuple");
  var len = entries.length;
  for (let i = 0; i < len; i++) {
    expression.push(entries[i]);
  }
  return expression;
}

function matrix(entries) {
  //entries is an array of arrays of math expressions
  var expression = [];
  expression.push("matrix");
  var r = entries.length;
  var c = entries[0].length;
  for (let i = 1; i < r; i++) {
    if (entries[i].length !== c) {
      //check if columns are equal size
      throw new Error("Matrix dimensions mismatch");
    }
  }
  expression.push(tuple([r, c]));
  let theMatrix = [];
  for (let j = 0; j < r; j++) {
    theMatrix.push(
      tuple(
        entries[j].map(function (v) {
          return v.tree;
        }),
      ),
    );
  }
  expression.push(tuple(theMatrix));
  return expression;
}

function vector_add(v1, v2) {
  v1 = get_tree(v1);
  v2 = get_tree(v2);

  if (
    v1.length !== v2.length ||
    (v1[0] !== "tuple" && v1[0] !== "vector" && v1[0] !== "altvector") ||
    (v2[0] !== "tuple" && v2[0] !== "vector" && v2[0] !== "altvector")
  ) {
    throw new Error(
      "Can't add. Those aren't vectors, or the dimensions don't match",
    );
  }
  var v_sum =
    v1[0] === "altvector" && v2[0] === "altvector" ? ["altvector"] : ["vector"];
  var len = v1.length;
  for (let i = 1; i < len; i = i + 1) {
    v_sum.push(["+", v1[i], v2[i]]);
  }
  return simplify.simplify(v_sum);
}

function scalar_mul(k, v) {
  v = get_tree(v);

  if (v[0] !== "tuple" && v[0] !== "vector" && v[0] !== "altvector") {
    throw new Error("Can't scalar multiply. Isn't a vector");
  }
  var v_prod = v[0] === "altvector" ? ["altvector"] : ["vector"];
  var len = v.length;
  for (let i = 1; i < len; i = i + 1) {
    v_prod.push(["*", v[i], k]);
  }
  return simplify.simplify(v_prod);
}

function vector_sub(v1, v2) {
  return vector_add(v1, scalar_mul(-1, v2));
}

function dot_prod(v1, v2) {
  v1 = get_tree(v1);
  v2 = get_tree(v2);

  if (
    v1.length !== v2.length ||
    (v1[0] !== "tuple" && v1[0] !== "vector" && v1[0] !== "altvector") ||
    (v2[0] !== "tuple" && v2[0] !== "vector" && v2[0] !== "altvector")
  ) {
    throw new Error(
      "Can't take dot product. Those aren't vectors, or the dimensions don't match",
    );
  }
  var sum = 0;
  var term = 0;
  var len = v1.length;
  for (let i = 1; i < len; i = i + 1) {
    term = ["*", v1[i], v2[i]];
    sum = ["+", sum, term];
  }
  return simplify.simplify(sum);
}

function cross_prod(v1, v2) {
  v1 = get_tree(v1);
  v2 = get_tree(v2);

  if (
    (v1[0] !== "tuple" && v1[0] !== "vector" && v1[0] !== "altvector") ||
    (v2[0] !== "tuple" && v2[0] !== "vector" && v2[0] !== "altvector")
  ) {
    throw new Error("Can't take cross product. Those aren't vectors");
  }

  if (v1.length === 3 && v2.length === 3) {
    return simplify.simplify([
      "+",
      ["*", v1[1], v2[2]],
      ["-", ["*", v1[2], v2[1]]],
    ]);
  }

  if (v1.length === 4 && v2.length === 4) {
    var x_coord = ["+", ["*", v1[2], v2[3]], ["-", ["*", v1[3], v2[2]]]];
    var y_coord = ["+", ["*", v1[3], v2[1]], ["-", ["*", v1[1], v2[3]]]];
    var z_coord = ["+", ["*", v1[1], v2[2]], ["-", ["*", v1[2], v2[1]]]];
    let vectorName =
      v1[0] === "altvector" && v2[0] === "altvector" ? "altvector" : "vector";
    return simplify.simplify([vectorName, x_coord, y_coord, z_coord]);
  }

  throw new Error(
    "Can't take cross product. The dimensions aren't both 2 or 3.",
  );
}

//cross product
//matrix: addition, sub, scalar mult, multiplication
//mult matrix by vector

export { matrix, vector_add, scalar_mul, vector_sub, dot_prod, cross_prod };
