// The two limits of the DOENET_INTEGRATION §3 inverse-trig fold, closed at the
// library boundary: exact values off the π/12 lattice, and the symbolic
// identities relating the trig functions to their inverses.
//
// Grading never needed these — `equals` decides all of them by sampling — so
// what changes here is what `simplify()` and `.tree` *show* a student.
import { describe, it, expect } from "vitest";
import me from "../lib/math-expressions";

const tree = (s: string) => me.fromText(s).simplify().tree;
const eq = (a: string, b: string) => me.fromText(a).equals(me.fromText(b), {});

/// A fold happened *and* it produced the right value. Both halves matter: the
/// value alone was already right before any of this (`equals` samples), and
/// the tree alone would not say the fold is correct. "Happened" is read as
/// "the outermost function is gone from the result" — the closed forms are
/// radicals, so they are `apply` nodes themselves.
const foldsTo = (input: string, closed: string) => {
  const fn = input.slice(0, input.indexOf("("));
  expect(JSON.stringify(tree(input)), `${input} should fold`).not.toContain(`"${fn}"`);
  expect(eq(input, closed), `${input} = ${closed}`).toBe(true);
};

describe("exact values on the pentagonal lattice", () => {
  // cos 36° = (1+√5)/4 and its relatives are in the surd ring, so they fold
  // beside the π/12 values. Their partners at the same angles — sin 36° =
  // √(10−2√5)/4 — need a radical nested one level deeper than the ring holds,
  // and decline rather than turn into a float.
  it("folds the half of the lattice that is representable", () => {
    for (const [input, closed] of [
      ["cos(pi/5)", "(1+sqrt(5))/4"],
      ["sin(pi/10)", "(sqrt(5)-1)/4"],
      ["sin(3pi/10)", "(1+sqrt(5))/4"],
      ["cos(2pi/5)", "(sqrt(5)-1)/4"],
      ["sec(pi/5)", "sqrt(5)-1"],
      ["csc(pi/10)", "1+sqrt(5)"],
    ] as const) {
      foldsTo(input, closed);
    }
  });

  it("leaves the half that needs a nested radical alone", () => {
    for (const s of ["sin(pi/5)", "cos(pi/10)", "tan(pi/5)"]) {
      expect(tree(s)[0], s).toBe("apply");
    }
  });

  it("inverts them too, since the inverse walks the same tables", () => {
    expect(tree("acos((1+sqrt(5))/4)")).toEqual(["/", "pi", 5]);
    expect(tree("asin((sqrt(5)-1)/4)")).toEqual(["/", "pi", 10]);
    expect(tree("acos((sqrt(5)-1)/4)")).toEqual(["/", ["*", 2, "pi"], 5]);
    // These need the general reciprocal in the exact ring: sec(π/12) is
    // 4/(√6+√2), a two-term denominator, which used to make the fold decline.
    expect(tree("asec(sqrt(6)-sqrt(2))")).toEqual(["/", "pi", 12]);
    expect(tree("acot(2+sqrt(3))")).toEqual(["/", "pi", 12]);
  });
});

describe("inverse trig parity", () => {
  it("pulls the sign out", () => {
    expect(tree("asin(-x)")).toEqual(["-", ["apply", "asin", "x"]]);
    expect(tree("atan(-x)")).toEqual(["-", ["apply", "atan", "x"]]);
    // acos reflects through pi/2 instead of being odd.
    expect(tree("acos(-x)")).toEqual(["+", "pi", ["-", ["apply", "acos", "x"]]]);
    expect(tree("asec(-x)")).toEqual(["+", "pi", ["-", ["apply", "asec", "x"]]]);
    // acot is odd here because the library defines it as atan(1/z) — under the
    // (0, pi) convention it would not be.
    expect(tree("acot(-x)")).toEqual(["-", ["apply", "acot", "x"]]);
  });

  it("does not invent an angle for an argument off the lattice", () => {
    expect(tree("asin(-3)")).toEqual(["-", ["apply", "asin", 3]]);
  });
});

describe("compositions of a trig function with an inverse", () => {
  it("cancels a function against its own inverse", () => {
    for (const [f, g] of [
      ["sin", "asin"],
      ["cos", "acos"],
      ["tan", "atan"],
      ["sec", "asec"],
      ["csc", "acsc"],
      ["cot", "acot"],
    ] as const) {
      expect(tree(`${f}(${g}(x))`), `${f} of ${g}`).toBe("x");
    }
  });

  it("closes the mixed pairs in radicals", () => {
    for (const [input, closed] of [
      ["cos(asin(x))", "sqrt(1-x^2)"],
      ["sin(acos(x))", "sqrt(1-x^2)"],
      ["tan(asin(x))", "x/sqrt(1-x^2)"],
      ["sec(atan(x))", "sqrt(1+x^2)"],
      ["cos(atan(x))", "1/sqrt(1+x^2)"],
      ["csc(asin(x))", "1/x"],
      ["cot(atan(x))", "1/x"],
    ] as const) {
      foldsTo(input, closed);
    }
  });

  it("does not fold the other direction, which is branch-dependent", () => {
    // asin(sin 3) is pi − 3, not 3, so there is no unconditional rewrite.
    for (const s of ["asin(sin(x))", "acos(cos(x))", "atan(tan(x))"]) {
      expect(tree(s)[0], s).toBe("apply");
    }
  });
});

describe("complementary inverse pairs", () => {
  it("sums a pair to a right angle", () => {
    expect(tree("asin(x)+acos(x)")).toEqual(["/", "pi", 2]);
    expect(tree("acsc(x)+asec(x)")).toEqual(["/", "pi", 2]);
    expect(tree("3asin(x)+3acos(x)")).toEqual(["/", ["*", 3, "pi"], 2]);
    expect(tree("y*asin(x)+y*acos(x)")).toEqual(["/", ["*", "pi", "y"], 2]);
  });

  it("leaves the sums that have no unconditional value", () => {
    // atan(u) + acot(u) is pi/2 for positive u and -pi/2 for negative u, with
    // acot defined as atan(1/u) — so there is nothing to fold to. Note that
    // `equals` answers `true` against pi/2 for the symbolic form while
    // answering `false` at u = -1; folding here would have written that
    // sampling gap into the simplified tree, where it is much harder to undo.
    expect(tree("atan(x)+acot(x)")[0]).toBe("+");
    expect(eq("atan(-1)+acot(-1)", "-pi/2")).toBe(true);
    expect(tree("asin(x)+acos(y)")[0]).toBe("+");
    expect(tree("2asin(x)+acos(x)")[0]).toBe("+");
  });
});

describe("grading is unchanged — equals already decided all of this", () => {
  it("still compares equal, folded or not", () => {
    for (const [a, b] of [
      ["cos(asin(x))", "sqrt(1-x^2)"],
      ["asin(x)+acos(x)", "pi/2"],
      ["asin(-x)", "-asin(x)"],
      ["acos((1+sqrt(5))/4)", "pi/5"],
      // Including the ones that do not fold: the nested-radical half of the
      // pentagonal lattice grades exactly as before.
      ["sin(pi/5)", "sqrt(10-2sqrt(5))/4"],
    ] as const) {
      expect(eq(a, b), `${a} = ${b}`).toBe(true);
    }
  });
});
