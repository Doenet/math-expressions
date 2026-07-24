// Compat stub: 'rational' has no Rust equivalent in the port (see
// active-plans/JS_TEST_COVERAGE_AUDIT.md). The module loads so specs importing
// it still run; any use throws, failing just those tests.
function unsupported() {
  throw new Error("math-expressions-js-compat: rational is not implemented");
}
export default new Proxy(function () {}, {
  get: () => unsupported,
  apply: unsupported,
  construct: unsupported,
});
