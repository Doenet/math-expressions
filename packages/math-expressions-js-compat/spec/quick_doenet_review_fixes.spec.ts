// Regressions for the review of the Doenet-compatibility round, on the surfaces
// that only exist in JS (the compat drop-in) or only show up through it.
//
// The Rust-side counterparts live in
// `math-expressions-rs/tests/doenet_review_fixes.rs`; these are the ones where
// the defect *is* the JS wrapper — a mirrored `toJSON`, a swallowed callback
// exception — plus a boundary check that the shape pass agrees with itself
// whichever way an expression was built.
import { describe, it, expect } from "vitest";
import me from "../lib/math-expressions";

describe("the vector/matrix shape pass flattens first", () => {
  const perform = (s: string) =>
    me.fromText(s).perform_vector_matrix_additions_scalar_multiplications().tree;

  // `+` is parsed left-nested (`x+y+z` → `Add[Add[x,y],z]`) while the JS tree
  // spelling is flat, so the pass — which reads one node's operand list — saw
  // both containers only via `fromAst`. Through `fromText` it silently returned
  // its input, leaving `+` on top, which is exactly the shape that stops
  // `checkEquality` from grading componentwise.
  it("combines tuples across a sum of three or more addends", () => {
    expect(perform("x+(1,2)+(3,4)")).toEqual([
      "+",
      "x",
      ["tuple", ["+", 1, 3], ["+", 2, 4]],
    ]);
    expect(perform("1+(1,2)+(3,4)")).toEqual([
      "+",
      1,
      ["tuple", ["+", 1, 3], ["+", 2, 4]],
    ]);
    expect(perform("x+(1,2)+2(3,4)")).toEqual([
      "+",
      "x",
      ["tuple", ["+", 1, ["*", 3, 2]], ["+", 2, ["*", 4, 2]]],
    ]);
  });

  it("agrees whichever way the expression was built", () => {
    const viaText = perform("x+(1,2)+(3,4)");
    const viaAst = me
      .fromAst(["+", "x", ["tuple", 1, 2], ["tuple", 3, 4]])
      .perform_vector_matrix_additions_scalar_multiplications().tree;
    expect(viaText).toEqual(viaAst);
  });

  it("keeps the addends in the order they were written", () => {
    expect(perform("(1,2)+(3,4)+x")).toEqual([
      "+",
      ["tuple", ["+", 1, 3], ["+", 2, 4]],
      "x",
    ]);
  });

  it("does not let an empty container swallow its scalar", () => {
    // Distributing into zero components consumes the factor and records it
    // nowhere, so `3·()` came back as `()`.
    expect(
      me
        .fromAst(["*", 3, ["tuple"]])
        .perform_vector_matrix_additions_scalar_multiplications().tree,
    ).toEqual(["*", 3, ["tuple"]]);
  });
});

describe("the context does not mirror `toJSON`", () => {
  // The loop that re-exposes every `Expression` method as an expression-first
  // function on the context skipped anything already `in Context` — which does
  // not cover `toJSON`, since that lives on `Expression.prototype` and not on
  // `Object.prototype`. `JSON.stringify` then called it with the *property key*
  // as the expression argument.
  it("leaves `JSON.stringify` of the context alone", () => {
    expect("toJSON" in me).toBe(false);
    // No `{objectType:"math-expression"}` envelope: that is the exact shape
    // `Context.reviver` recognizes, so persisting and reviving turned the
    // library context itself into an Expression.
    for (const holder of [{ me }, [me]]) {
      expect(JSON.stringify(holder)).not.toContain("math-expression");
    }
  });

  it("does not throw out of a plain `JSON.stringify`", () => {
    // The key was parsed as an expression, so a key that is not valid math
    // threw — a `panic = "abort"` hazard anywhere this sits on a Rust-invoked
    // path, and a bizarre error anywhere else.
    for (const key of ["(", "\\frac", "x)", ""]) {
      expect(() => JSON.stringify({ [key]: me })).not.toThrow();
    }
  });

  it("still serializes and revives an expression", () => {
    const x = me.fromText("m*e");
    const round = JSON.parse(JSON.stringify({ x }), me.reviver);
    expect(round.x.tree).toEqual(["*", "m", "e"]);
    expect(round.x.equals(x)).toBe(true);
  });
});

describe("dopri surfaces failures instead of returning a plausible trajectory", () => {
  it("rethrows an exception from the derivative", () => {
    // The Rust side must not let it unwind (`panic = "abort"`), so it stops
    // integrating — which on its own handed back the initial condition with
    // only `terminatedEarly` to hint at why.
    expect(() =>
      me.dopri(0, 1, 1, () => {
        throw new Error("boom");
      }),
    ).toThrow("boom");
  });

  it("rejects a derivative of the wrong width", () => {
    // Silently integrating one component of a two-component system is a wrong
    // answer, not a degraded one.
    expect(() => me.dopri(0, 1, [1, 2], () => 5 as unknown as number[])).toThrow(
      /2-component state/,
    );
  });

  it("still solves, and releases its handle idempotently", () => {
    const sol = me.dopri(0, 1, 1, (_x, y) => y as number);
    expect(sol.at(1) as number).toBeCloseTo(Math.E, 4);
    expect(sol.terminatedEarly).toBe(false);
    sol.free();
    sol.free(); // a double free must be a no-op, not "null pointer passed to rust"
  });

  it("solves a system", () => {
    // y'' = -y as a first-order system: y(t) = (sin t, cos t).
    const sol = me.dopri(0, Math.PI / 2, [0, 1], (_x, y) => {
      const v = y as number[];
      return [v[1], -v[0]];
    });
    const end = sol.at(Math.PI / 2) as number[];
    expect(end[0]).toBeCloseTo(1, 4);
    expect(end[1]).toBeCloseTo(0, 4);
    sol.dispose();
  });
});

describe("numeric edges that returned confident wrong answers", () => {
  const AGGREGATES = {
    appliedFunctionSymbols: ["max", "min", "median", "mean", "sum"],
  };

  it("propagates NaN through max/min/median", () => {
    // The sort comparator was `partial_cmp(…).unwrap_or(Equal)`, not a total
    // order once a NaN is present: `max` answered 3 and `min` answered 4 for
    // the same list.
    for (const f of ["max", "min", "median"]) {
      const v = me.fromText(`${f}(4,x,3,2,1)`, AGGREGATES).evaluate({ x: NaN });
      expect(v, `${f} should propagate NaN`).toBeNaN();
    }
  });

  it("still orders correctly without a NaN", () => {
    expect(me.fromText("max(4,3,2,1)", AGGREGATES).evaluate({})).toBe(4);
    expect(me.fromText("min(4,3,2,1)", AGGREGATES).evaluate({})).toBe(1);
    expect(me.fromText("median(1,2,3,4)", AGGREGATES).evaluate({})).toBe(2.5);
  });

  it("is `0`, not `NaN`, for zero over an infinity", () => {
    // `∞^(-1)` is `0`, so this annihilates like any other zero. The fix that
    // correctly made `0/0` indeterminate had made this `NaN` too.
    expect(me.fromAst(["/", 0, { $: "Inf" }]).simplify().tree).toBe(0);
    expect(me.fromAst(["/", 0, { $: "-Inf" }]).simplify().tree).toBe(0);
    expect(me.fromAst(["*", 0, "x"]).simplify().tree).toBe(0);
  });

  it("keeps the genuinely indeterminate forms `NaN`", () => {
    // DoenetML computes an undefined slope as `0/0`; reporting it as `0` calls
    // a degenerate line horizontal.
    for (const ast of [
      ["/", 0, 0],
      ["*", 0, { $: "Inf" }],
      ["*", 0, { $: "None" }],
    ]) {
      expect(me.fromAst(ast).simplify().tree).toEqual(NaN);
    }
  });

  it("does not saturate nCr/nPr on a large float", () => {
    // `n.re.round() as i64` clamped to i64::MAX, giving an answer ~1275× low.
    expect(
      me.fromAst(["apply", "nCr", ["tuple", 1e20, 3]]).simplify().tree as number,
    ).toBeCloseTo(1e60 / 6, -50);
    expect(
      me.fromAst(["apply", "nPr", ["tuple", 1e20, 2]]).simplify().tree as number,
    ).toBeCloseTo(1e40, -31);
    expect(me.fromAst(["apply", "nCr", ["tuple", 5, 3]]).simplify().tree).toBe(10);
  });

  it("does not hang on large exact combinatorics or logarithms", () => {
    // Both are a handful of characters of student input; both took seconds.
    const timed = (f: () => unknown) => {
      const t0 = performance.now();
      f();
      return performance.now() - t0;
    };
    expect(
      timed(() => me.fromAst(["apply", "nCr", ["tuple", ["^", 10, 500], 500]]).simplify()),
    ).toBeLessThan(1500);
    expect(
      timed(() => me.fromAst(["apply", "log2", ["^", 2, 200000]]).simplify()),
    ).toBeLessThan(1500);
    // …and still fold to the right values.
    expect(me.fromAst(["apply", "log2", ["^", 2, 20]]).simplify().tree).toBe(20);
    expect(me.fromText("log10(1000)").simplify().tree).toBe(3);
  });
});
