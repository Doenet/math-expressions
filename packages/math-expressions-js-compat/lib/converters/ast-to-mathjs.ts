// Compat stub: 'astToMathjs' has no Rust equivalent in the port (see
// active-plans/JS_TEST_COVERAGE_AUDIT.md). Constructing the converter and
// importing the module both succeed so specs still collect and run; only
// `convert()` throws, failing just those tests.
export default class {
  convert(): never {
    throw new Error("math-expressions-js-compat: astToMathjs is not implemented");
  }
}
