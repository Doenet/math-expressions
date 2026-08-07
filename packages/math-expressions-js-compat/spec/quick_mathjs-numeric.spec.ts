// `me.math` in the original library was math.js *plus* numeric.js, and callers
// reach for names only numeric provides — DoenetML's `<odeSystem>` integrates
// with `me.math.dopri`. This covers that the drop-in still carries them, and
// that they work outside Node.
//
// numeric.js builds most of its helpers at load time with the `Function`
// constructor, and the generated bodies reference a bare `numeric`, which
// resolves only against the global object. numeric.js registers itself there
// through Node's `global`; under Node that would happen with or without
// `lib/mathjs` doing it too, so `global` is deleted before the first import of
// the module — reproducing the shape a browser or a web worker sees.
//
// Hence no static import of `../lib/mathjs` here: it has to load *after* the
// deletion, inside the test.

type Dopri = (
  t0: number,
  t1: number,
  x0: number[],
  f: (t: number, x: number[]) => number[],
  tolerance: number,
  maxIterations: number,
) => { at: (t: number) => number[] };

describe("numeric functions on me.math", function () {
  let hadGlobal: boolean, savedGlobal: unknown;
  let hadNumeric: boolean, savedNumeric: unknown;

  beforeEach(function () {
    hadGlobal = "global" in globalThis;
    savedGlobal = (globalThis as Record<string, unknown>).global;
    hadNumeric = "numeric" in globalThis;
    savedNumeric = (globalThis as Record<string, unknown>).numeric;
  });

  afterEach(function () {
    const g = globalThis as Record<string, unknown>;
    if (hadGlobal) g.global = savedGlobal;
    else delete g.global;
    if (hadNumeric) g.numeric = savedNumeric;
    else delete g.numeric;
  });

  it("keeps the names math.js has no equivalent of, with no `global`", async function () {
    const g = globalThis as Record<string, unknown>;
    delete g.global;
    delete g.numeric;

    const { default: math } = await import("../lib/mathjs");

    // The registration `lib/mathjs` makes on numeric's behalf.
    expect(typeof g.numeric).toBe("object");
    expect(typeof (g.numeric as Record<string, unknown>).dim).toBe("function");

    // `dopri` has no math.js equivalent, and it reaches numeric's generated
    // `add`/`mul`/`sub` helpers — so it throws `ReferenceError: numeric is not
    // defined` without that registration. x' = x from x(0) = 1 integrates to e.
    // Cast because it comes from numeric, so math.js's types do not name it.
    const { dopri } = math as unknown as { dopri: Dopri };
    expect(typeof dopri).toBe("function");
    const solution = dopri(0, 1, [1], (t, x) => [x[0]], 1e-6, 1000);
    expect(solution.at(1)[0]).toBeCloseTo(Math.E, 5);
  });
});
