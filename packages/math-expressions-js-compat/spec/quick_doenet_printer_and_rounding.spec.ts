// End-to-end coverage (through the wasm boundary) for the three DoenetML open
// items on the printers and display-rounding:
//
//   8.  display rounding must not turn an exact rational into a decimal when
//       rounding changes nothing;
//   9.  integer powers of the imaginary unit fold (the unambiguous half — the
//       root cases await DoenetML's corpus);
//   10. a negative leading coefficient prints as a subtraction, and the
//       integral keeps its `∫` glyph without spurious parentheses.
import { describe, it, expect } from "vitest";
import me from "../lib/math-expressions";

describe("item 8 — rounding preserves an exact fraction", () => {
  it("keeps 5/2 a fraction when rounding changes nothing", () => {
    // `.simplify()` collapses the `["/",5,2]` division into a single rational —
    // the bare value a computed result (`cos(pi/3) → 1/2`) also is, and the one
    // display rounding used to decimalize.
    const r = me.fromText("5/2").simplify().round_numbers_to_precision(3);
    expect(r.toString()).toBe("5/2");
    expect(r.toLatex()).toBe("\\frac{5}{2}");
  });

  it("keeps 1/3 a fraction too, though rounding would change the value", () => {
    // This asserted `"0.333"` while the rule was "leave it alone only if the
    // rounding is exact". That rule was superseded (see
    // `ops::numbers::is_written_as_fraction`): what decides now is the
    // *spelling*, so a fraction the author wrote stays a fraction however few
    // digits are asked for — which is what the JS library did, having no
    // rational type to decimalize in the first place. Deciding by exactness
    // instead put `1/3` on screen as `0.333` in a fractions lesson, and only
    // for the rationals that had been through `simplify`.
    expect(me.fromText("1/3").simplify().round_numbers_to_precision(3).toString()).toBe(
      "1/3",
    );
  });
});

describe("item 9 — integer powers of i fold", () => {
  it("folds i^2, i^3, i^4", () => {
    expect(me.fromText("i^2").simplify().tree).toBe(-1);
    expect(me.fromText("i^3").simplify().tree).toEqual(["-", "i"]);
    expect(me.fromText("i^4").simplify().tree).toBe(1);
    expect(me.fromText("2i*3i").simplify().tree).toBe(-6);
  });
});

describe("item 9 — numeric roots: prefer real, else principal complex", () => {
  it("folds a square root of a negative number to its principal value", () => {
    // `i` surfaces only when the whole radicand is a perfect square. Otherwise
    // the perfect-square factor comes out and the sign stays under the root,
    // matching the JS oracle (`sqrt(-810)` → `9·sqrt(-10)`).
    expect(me.fromText("sqrt(-1)").simplify().tree).toBe("i");
    expect(me.fromText("sqrt(-4)").simplify().tree).toEqual(["*", 2, "i"]);
    expect(me.fromText("sqrt(-2)").simplify().tree).toEqual([
      "apply",
      "sqrt",
      -2,
    ]);
    expect(me.fromText("sqrt(-8)").simplify().tree).toEqual([
      "*",
      2,
      ["apply", "sqrt", -2],
    ]);
  });

  it("prefers the real root when one exists", () => {
    expect(me.fromText("cbrt(-8)").simplify().tree).toBe(-2);
    expect(me.fromText("nthroot(-8,3)").simplify().tree).toBe(-2);
  });

  it("never folds a variable radicand", () => {
    // sqrt(x^2) keeps its radical — no assumption on x's sign.
    expect(me.fromText("sqrt(x^2)").simplify().tree).toEqual(["apply", "sqrt", ["^", "x", 2]]);
  });
});

describe("item 10 — printer fixes", () => {
  it("prints a negative leading coefficient as a subtraction", () => {
    expect(me.fromAst(["+", "a", ["*", -3, "b"]]).toString()).toBe("a - 3 b");
  });

  it("shows a negative fraction with the sign out front", () => {
    expect(me.fromAst(["/", -2, 3]).toString()).toBe("-2/3");
    expect(me.fromAst(["+", "z", ["/", -2, 3]]).toString()).toBe("z - 2/3");
  });

  it("keeps the integral glyph and drops the parentheses", () => {
    expect(me.fromLatex("\\int_{a}^{b} f(x) dx").toString()).toBe("∫_a^b f(x) dx");
  });
});
