/**
 * TypeScript validation file for math-expressions type definitions
 *
 * This file tests that the type definitions in index.d.ts are correct
 * and provide proper type checking for TypeScript consumers.
 *
 * Run: npx tsc --noEmit types-test.ts
 */

import MathExpression, {
  Expression,
  Tree,
  EqualsOptions,
  Bindings,
  isTree,
} from "./index";

// ========== Test factory methods ==========

const expr1: Expression = MathExpression.fromText("x^2 + 2*x + 1");
const expr2: Expression = MathExpression.fromLatex("\\frac{x+1}{2}");
const expr3: Expression = MathExpression.from("sin(x)");
const expr4: Expression = MathExpression.parse("cos(theta)");

const tree: Tree = ["+", 1, ["*", 2, "x"], ["^", "x", 2]];
const expr5: Expression = MathExpression.fromAst(tree);

// ========== Test type guard ==========

// Test isTree type guard function
const validTree1: boolean = isTree(5);
const validTree2: boolean = isTree("x");
const validTree3: boolean = isTree(["+", 1, 2]);
const validTree4: boolean = isTree({ invalid: "object" });

// Type guard usage - narrows type
const unknownValue: unknown = ["+", 1, "x"];
if (isTree(unknownValue)) {
  // TypeScript now knows unknownValue is a Tree
  const narrowedTree: Tree = unknownValue;
}

console.log("✓ Type guard tests passed");

const simplified: Expression = expr1.simplify();
const expanded: Expression = expr1.expand();
const factored: Expression = expr1.factor();

// Derivative with optional story parameter
const derivative1: Expression = expr1.derivative("x");
const story: string[] = [];
const derivative2: Expression = expr1.derivative("x", story);

// Arithmetic operations
const sum: Expression = expr1.add(expr2);
const difference: Expression = expr1.subtract(expr2);
const product: Expression = expr1.multiply(expr2);
const quotient: Expression = expr1.divide(expr2);
const power: Expression = expr1.pow(2);

// Normalization
const normalized1: Expression = expr1.normalize_function_names();
const normalized2: Expression = expr1.normalize_applied_functions();
const normalized3: Expression = expr1.normalize_negative_numbers();

// Other transformations
const rounded1: Expression = expr1.round_numbers_to_precision(5);
const rounded2: Expression = expr1.round_numbers_to_decimals(3);
const substituted: Expression = expr1.substitute({ x: expr2, y: tree });

// ========== Test inspection methods ==========

const variables: string[] = expr1.variables();
const variablesWithSubs: string[] = expr1.variables(true);
const operators: string[] = expr1.operators();
const functions: string[] = expr1.functions();

// Equality testing with options
const isEqual1: boolean = expr1.equals(expr2);
const options: EqualsOptions = {
  relative_tolerance: 1e-10,
  absolute_tolerance: 1e-12,
  allowed_error_in_numbers: 0.001,
  allow_blanks: false,
};
const isEqual2: boolean = expr1.equals(expr2, options);

// Alternative equality methods
const syntaxEqual: boolean = expr1.equalsViaSyntax(expr2, options);
const complexEqual: boolean = expr1.equalsViaComplex(expr2, options);
const realEqual: boolean = expr1.equalsViaReal(expr2, options);
const finiteFieldEqual: boolean = expr1.equalsViaFiniteField(expr2, options);

// Evaluation
const bindings: Bindings = { x: 5, y: 3.14 };
const result1: number | import("./index").Complex = expr1.evaluate(bindings);
const result2: number | null = expr1.evaluate_to_constant();
const result3: number | null = expr1.evaluate_to_constant({
  remove_units_first: true,
  scale_based_on_unit: false,
  nan_for_non_numeric: true,
});

const finiteFieldResult: number = expr1.finite_field_evaluate({ x: 2 }, 7);

// Analyticity check
const isAnalytic1: boolean = expr1.isAnalytic();
const isAnalytic2: boolean = expr1.isAnalytic(["x", "y"]);

// Get evaluator function
const evaluatorFunc = expr1.f();
const resultFromFunc: number | import("./index").Complex = evaluatorFunc({
  x: 5,
});

// Pattern matching
const pattern: Tree = ["+", "a", "b"];
const matchResult = expr1.match(pattern);
if (matchResult !== null) {
  const aValue: Tree = matchResult.a;
  const bValue: Tree = matchResult.b;
}

// ========== Test formatting methods ==========

const textStr: string = expr1.toString();
const latexStr1: string = expr1.toLatex();
const latexStr2: string = expr1.tex();
const xmlStr: string = expr1.toXML();
const glslStr: string = expr1.toGLSL();
const guppyStr: string = expr1.toGuppy();

// ========== Test JSON serialization ==========

const json = expr1.toJSON();
const objectType: string = json.objectType;
const jsonTree: Tree = json.tree;
if (json.assumptions) {
  const assumptions = json.assumptions;
}

// ========== Test assumptions ==========

MathExpression.add_assumption({ variable: "x", element_of: "RR" });
MathExpression.add_assumption(
  { variable: "n", element_of: ["ZZ", "positive"] },
  false,
);
MathExpression.add_generic_assumption({ variable: "z", element_of: "CC" });

const assumptions = MathExpression.get_assumptions(["x", "y"]);
const assumptions2 = MathExpression.get_assumptions(expr1);

MathExpression.remove_assumption("x");
MathExpression.remove_assumption({ variable: "y" });
MathExpression.remove_generic_assumption({ variable: "z" });
MathExpression.clear_assumptions();

// ========== Test Context properties ==========

// Test Context properties
const contextAssumptions = MathExpression.assumptions;
const parserParams = MathExpression.parser_parameters;
const ZmodN = MathExpression.ZmodN;

// Test LaTeX aliases
const exprViaLaTeX1: Expression = MathExpression.fromLaTeX("x^2");
const exprViaTeX1: Expression = MathExpression.fromTeX("x^2");
const exprViaTex1: Expression = MathExpression.fromTex("x^2");
const exprViaParseTex: Expression = MathExpression.parse_tex("x^2");

console.log("✓ Context properties and aliases type tests passed");

// ========== Test Context methods ==========

MathExpression.set_to_default();

// Access converters and utils
const converters = MathExpression.converters;
const utils = MathExpression.utils;
const math = MathExpression.math;

// Utility functions
const flattened: Tree = utils.flatten(tree);
const unflattenedLeft: Tree = utils.unflattenLeft(tree);
const unflattenedRight: Tree = utils.unflattenRight(tree);
const utilMatchResult = utils.match(tree, pattern);

// JSON reviver
const reviver = MathExpression.reviver;
const parsed = JSON.parse('{"key": "value"}', reviver);

// ========== Test method chaining ==========

const chainedResult: Expression = MathExpression.fromText("x^2 + 2*x + 1")
  .normalize_function_names()
  .normalize_applied_functions()
  .simplify()
  .derivative("x")
  .expand()
  .round_numbers_to_precision(10);

// Test log subscript conversion and subscript to string conversions
const logExpr = math.parse("log(x)");
const normalizedLog = logExpr.log_subscript_to_two_arg_log();
const subscriptStr = logExpr.subscripts_to_strings();
const subscriptStrForced = logExpr.subscripts_to_strings(true);

console.log("✓ Subscript normalization type tests passed");

// Test component methods
const vectorExpr = math.parse("[1, 2, 3]");
const component = vectorExpr.get_component(0);
const substituted_component = vectorExpr.substitute_component(1, 42);

console.log("✓ Component manipulation type tests passed");

// ========== Test tree operations ==========

const numTree: Tree = 42;
const varTree: Tree = "x";
const opTree: Tree = ["+", 1, 2];
const nestedTree: Tree = ["*", ["^", "x", 2], ["sin", ["+", "x", 1]]];

// ========== Test Context factory methods (operate on Trees) ==========

// These are the same methods as on Expression, but work on Trees directly
const contextSimplified: Expression = MathExpression.simplify(nestedTree);
const contextExpanded: Expression = MathExpression.expand(nestedTree);
const contextFactored: Expression = MathExpression.factor(nestedTree);
const contextDerivative: Expression = MathExpression.derivative(
  nestedTree,
  "x",
);

// Arithmetic operations on Trees
const contextSum: Expression = MathExpression.add(opTree, ["+", 3, 4]);
const contextDifference: Expression = MathExpression.subtract(opTree, 1);
const contextProduct: Expression = MathExpression.multiply(opTree, 2);
const contextQuotient: Expression = MathExpression.divide(opTree, 2);
const contextPower: Expression = MathExpression.pow(nestedTree, 2);

// Inspection methods on Trees
const treeVariables: string[] = MathExpression.variables(nestedTree);
const treeOperators: string[] = MathExpression.operators(nestedTree);
const treeFunctions: string[] = MathExpression.functions(nestedTree);

// Formatting methods on Trees
const treeToString: string = MathExpression.toString(nestedTree);
const treeToLatex: string = MathExpression.toLatex(nestedTree);

// Math functions on Trees
const contextSin: Expression = MathExpression.sin(nestedTree);
const contextCos: Expression = MathExpression.cos(nestedTree);
const contextSqrt: Expression = MathExpression.sqrt(opTree);

console.log("✓ Context factory method type tests passed");

// ========== Test Expression class access ==========

const ExpressionClass = MathExpression.class;

// ========== Test that errors are caught ==========

// These should cause TypeScript errors if uncommented:
// const wrongType: string = expr1.evaluate({ x: 5 }); // Error: number | Complex is not assignable to string
// const wrongParam: Expression = expr1.derivative(); // Error: missing required parameter
// const wrongOption: boolean = expr1.equals(expr2, { invalid_option: true }); // Error: invalid option

console.log("✓ All type definitions validated successfully!");
console.log(`Expression: ${textStr}`);
console.log(`LaTeX: ${latexStr1}`);
console.log(`Variables: ${variables.join(", ")}`);
console.log(`Equals test: ${isEqual1}`);
