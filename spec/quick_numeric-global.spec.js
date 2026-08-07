// numeric.js builds most of its helpers at load time with the `Function`
// constructor, and the generated bodies reference a bare `numeric` (e.g.
// `_s = numeric.dim(x)`). Those functions are evaluated in global scope, so
// that reference resolves only if `numeric` is a property of the global object.
// numeric.js publishes itself there through Node's `global`, which does not
// exist in a browser or a web worker — so `lib/mathjs.js` has to publish it.
//
// Running under Node, `global` is present and numeric would register itself
// anyway, which would make this test pass with or without that code. So delete
// `global` before the first import to reproduce the shape a browser sees.

describe("numeric's global registration", function () {
  let hadGlobal, savedGlobal, hadNumeric, savedNumeric;

  beforeEach(function () {
    hadGlobal = "global" in globalThis;
    savedGlobal = globalThis.global;
    hadNumeric = "numeric" in globalThis;
    savedNumeric = globalThis.numeric;
  });

  afterEach(function () {
    if (hadGlobal) globalThis.global = savedGlobal;
    else delete globalThis.global;
    if (hadNumeric) globalThis.numeric = savedNumeric;
    else delete globalThis.numeric;
  });

  it("survives a runtime with no `global`", async function () {
    delete globalThis.global;
    delete globalThis.numeric;

    const { default: math } = await import("../lib/mathjs.js");

    expect(globalThis.numeric).toBeTypeOf("object");
    expect(globalThis.numeric.dim).toBeTypeOf("function");

    // `dopri` reaches numeric's generated `add`/`mul`/`sub` helpers, so it
    // throws `ReferenceError: numeric is not defined` if the registration is
    // missing. x' = x from x(0) = 1 integrates to e.
    const solution = math.dopri(0, 1, [1], (t, x) => [x[0]], 1e-6, 1000);
    expect(solution.at(1)[0]).toBeCloseTo(Math.E, 5);
  });
});
