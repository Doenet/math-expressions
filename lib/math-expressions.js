import * as assumptions from "./assumptions/assumptions.js";
import math from "./mathjs.js";
import { flatten, unflattenLeft, unflattenRight } from "./trees/flatten.js";
import { expression_to_tree } from "./expression/index.js";
import { expression_to_other } from "./expression/index.js";
import { expression_to_tree as functions_to_tree } from "./functions/index.js";
import * as converters from "./converters/index.js";
import { match } from "./trees/basic.js";
import ZmodN from "./converters/z-mod-n.js";

var textToAst = new converters.textToAstObj();
var latexToAst = new converters.latexToAstObj();
var mmlToAst = new converters.mmlToAstObj();

var utils = { match, flatten, unflattenLeft, unflattenRight };

// Type guard to check if a value is a valid Tree
export function isTree(value) {
  if (
    typeof value === "number" ||
    typeof value === "string" ||
    typeof value === "boolean"
  ) {
    return true;
  }
  if (
    Array.isArray(value) &&
    value.length > 0 &&
    typeof value[0] === "string"
  ) {
    return value.slice(1).every((item) => isTree(item));
  }
  return false;
}

function Expression(ast, context) {
  this.tree = flatten(ast);
  this.context = context;

  this.toJSON = function () {
    let serializedExpression = {
      objectType: "math-expression",
      tree: this.tree,
    };
    let assumptions = {};
    for (let item in this.context.assumptions) {
      if (Object.keys(this.context.assumptions[item]).length > 0) {
        assumptions[item] = this.context.assumptions[item];
      }
    }
    if (Object.keys(assumptions).length > 0) {
      serializedExpression.assumptions = assumptions;
    }

    return serializedExpression;
  };
}

function extend(object, tree_to_expression) {
  // if tree_to_expression, convert ast to expression

  // arguments object is NOT an array
  var args = flatten_array(Array.prototype.slice.call(arguments, 2));

  args.forEach(function (rhs) {
    if (rhs) {
      for (var property in rhs) {
        if (tree_to_expression) {
          (function () {
            var prop = property;
            object[prop] = function () {
              return this.fromAst(rhs[prop].apply(null, arguments));
            };
          })();
        } else object[property] = rhs[property];
      }
    }
  });

  return object;
}

function extend_prototype(object, tree_to_expression) {
  // append a properties to object prepending this as first argument
  // if tree_to_expression, convert ast to expression

  // arguments object is NOT an array
  var args = flatten_array(Array.prototype.slice.call(arguments, 2));

  args.forEach(function (rhs) {
    if (rhs) {
      for (var property in rhs) {
        // prepend this as first argument
        (function () {
          var prop = property;
          object[prop] = function () {
            var arg2 = [this].concat(Array.prototype.slice.call(arguments));
            // convert to expression if output_expression
            if (tree_to_expression)
              return this.context.fromAst(rhs[prop].apply(null, arg2));
            else return rhs[prop].apply(null, arg2);
          };
        })();
      }
    }
  });

  return object;
}

/****************************************************************/
/* Factory methods */

function create_from_multiple(expr, pars) {
  if (Array.isArray(expr) || typeof expr === "number") {
    return new Expression(expr, Context);
  } else if (typeof expr === "string") {
    try {
      return new Expression(textToAst.convert(expr), Context);
    } catch (e_text) {
      try {
        return new Expression(latexToAst.convert(expr), Context);
      } catch (e_latex) {
        try {
          return new Expression(mmlToAst.convert(expr), Context);
        } catch (e_mml) {
          if (expr.indexOf("\\") !== -1) throw e_latex;
          if (expr.indexOf("</") !== -1) throw e_mml;
          throw e_text;
        }
      }
    }
  }
}

function parseText(string, pars) {
  return new Expression(textToAst.convert(string), Context);
}

function parseLatex(string, pars) {
  return new Expression(latexToAst.convert(string), Context);
}

function parseMml(string, pars) {
  return new Expression(mmlToAst.convert(string), Context);
}

var Context = {
  ZmodN: ZmodN,
  assumptions: assumptions.initialize_assumptions(),
  parser_parameters: {},
  from: create_from_multiple,
  fromText: parseText,
  parse: parseText,
  fromLaTeX: parseLatex,
  fromLatex: parseLatex,
  fromTeX: parseLatex,
  fromTex: parseLatex,
  fromMml: parseMml,
  parse_tex: parseLatex,
  converters,
  utils,
  fromAst: function (ast) {
    return new Expression(ast, this);
  },
  set_to_default: function () {
    this.assumptions = assumptions.initialize_assumptions();
    // this.parser_parameters = JSON.parse(JSON.stringify(parser_defaults));
  },
  get_assumptions: function (variables_or_expr, params) {
    return assumptions.get_assumptions(
      this.assumptions,
      variables_or_expr,
      params,
    );
  },
  add_assumption: function (assumption, exclude_generic) {
    return assumptions.add_assumption(
      this.assumptions,
      assumption,
      exclude_generic,
    );
  },
  add_generic_assumption: function (assumption) {
    return assumptions.add_generic_assumption(this.assumptions, assumption);
  },
  remove_assumption: function (assumption) {
    return assumptions.remove_assumption(this.assumptions, assumption);
  },
  remove_generic_assumption: function (assumption) {
    return assumptions.remove_generic_assumption(this.assumptions, assumption);
  },
  clear_assumptions: function () {
    this.assumptions = assumptions.initialize_assumptions();
  },

  math: math,

  reviver: function (key, value) {
    if (
      value &&
      value.objectType === "math-expression" &&
      value.tree !== undefined
    ) {
      let expr = Context.fromAst(value.tree);
      if (value.assumptions !== undefined) {
        expr.assumptions = value.assumptions;
      }
      return expr;
    }
    return value;
  },
  class: Expression,
};

Context.set_to_default();

extend(Context, true, expression_to_tree, functions_to_tree);
extend(Context, false, expression_to_other);

export default Context;

extend_prototype(Expression.prototype, true, expression_to_tree);
extend_prototype(Expression.prototype, false, expression_to_other);

// from https://stackoverflow.com/a/34757676
function flatten_array(ary, ret) {
  ret = ret === undefined ? [] : ret;
  for (var i = 0; i < ary.length; i++) {
    if (Array.isArray(ary[i])) {
      flatten_array(ary[i], ret);
    } else {
      ret.push(ary[i]);
    }
  }
  return ret;
}
