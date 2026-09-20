// The two round-off divergences DoenetML found at the boundary, checked here
// because the boundary is where they bite: `<odeSystem>` calls `me.dopri` with
// a JS derivative, and `<sequence type="math">` calls `.equals` on values that
// crossed the JSON AST and so arrived as f64.
//
// Both are places where the Rust engine made a defensible choice that is not
// the JS library's. See the Rust-side tests (`tests/ode.rs`,
// `tests/equality.rs`) for the full contract; these are the caller's view.
import { describe, it, expect } from "vitest";
import me from "../lib/math-expressions";

describe("dopri delivers the accuracy the caller asked for", () => {
  // `y′ = y` to t = 10 grows to ~2·10⁴. A controller that scales its error
  // bound by the solution's own magnitude accepts a third of the steps and
  // drifts outside `tol` while still reporting success; `numeric.dopri`
  // measures the local error absolutely, and DoenetML grades `tolerance` as a
  // relative accuracy on the answer.
  const tol = 1e-6;
  const sol = me.dopri(0, 10, 1, (_x, y) => y as number, tol, 10_000);

  it("holds |y(x) − eˣ| within tol·max(1,|eˣ|) across the interval", () => {
    for (let x = 0; x <= 10; x++) {
      const want = Math.exp(x);
      expect(Math.abs((sol.at(x) as number) - want)).toBeLessThanOrEqual(
        tol * Math.max(1, want),
      );
    }
  });

  it("takes numeric.dopri's step count", () => {
    // Deliberately exact: `maxIterations` is an authored DoenetML attribute,
    // so a differently-paced controller changes what an existing document
    // does even when it is equally accurate.
    expect(sol.x.length).toBe(145);
  });

  it("spends its iteration budget the way numeric did", () => {
    // The second chunk, [10,20] — the run whose budget DoenetML documents:
    // 1000 iterations is not enough, 2000 is.
    const y10 = Math.exp(10);
    const starved = me.dopri(10, 20, y10, (_x, y) => y as number, tol, 1000);
    expect(starved.terminatedEarly).toBe(true);
    expect(starved.x[starved.x.length - 1]).toBeLessThan(20);

    const funded = me.dopri(10, 20, y10, (_x, y) => y as number, tol, 2000);
    expect(funded.terminatedEarly).toBe(false);
    expect(funded.x[funded.x.length - 1]).toBeCloseTo(20, 12);
  });
});

describe("equals tolerates round-off in a computed number", () => {
  // The `<sequence type="math" from=".1" to=".8" step=".1" exclude=".3">`
  // case: the third term is built as `from + step·2` from values that came
  // back through the JSON AST as f64, so it is 0.30000000000000004, and the
  // exclusion is expressed as `.equals`.
  const computed = me
    .fromAst(0.1)
    .add(me.fromAst(0.1).multiply(me.fromAst(2)))
    .expand()
    .simplify();

  it("is the inexact value, not a folded rational", () => {
    expect(computed.tree).toBe(0.30000000000000004);
  });

  it("compares equal to the exact 0.3, in both orders", () => {
    expect(me.fromText(".3").equals(computed)).toBe(true);
    expect(computed.equals(me.fromText(".3"))).toBe(true);
    expect(me.fromAst(0.3).equals(me.fromAst(0.30000000000000004))).toBe(true);
  });

  it("still separates numbers that differ by more than the tolerance", () => {
    expect(me.fromAst(0.3).equals(me.fromAst(0.3000000001))).toBe(false);
    expect(me.fromAst(1).equals(me.fromAst(1.0000001))).toBe(false);
  });

  it("leaves exact-vs-exact alone", () => {
    // Two decimals a person typed are exact quantities: they parse to
    // rationals, and stage 1 stays definitive about them.
    expect(me.fromText("0.3").equals(me.fromText("0.30000000000000004"))).toBe(
      false,
    );
    expect(me.fromText("10^20+1").equals(me.fromText("10^20+2"))).toBe(false);
  });
});
