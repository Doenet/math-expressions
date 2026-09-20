// Three grading paths DoenetML calls that the compat layer did not answer, or
// answered differently from the JS library. Each is checked at the boundary,
// which is where DoenetML meets them.
import { describe, it, expect } from "vitest";
import me from "../lib/math-expressions";

describe("sign-error grading (numSignErrorsMatched)", () => {
  // `me.equalSpecifiedSignErrors` was simply absent, so *any* award carrying
  // `numSignErrorsMatched` threw on submit and the answer never registered as
  // submitted at all.
  const target = me.fromText("x^2-2x+3");
  const eqf = (a: any, b: any) => a.equals(b);
  const one = (s: string) =>
    me.equalSpecifiedSignErrors(me.fromText(s), target, {
      equalityFunction: eqf,
      n_sign_errors: 1,
    });

  it("accepts a single flipped sign and rejects two", () => {
    expect(one("x^2+2x+3")).toBe(true); // the middle term's sign
    expect(one("x^2-2x-3")).toBe(true); // the constant's sign
    expect(one("x^2+2x-3")).toBe(false); // both — that is two errors
  });

  it("reports how many flips it took", () => {
    expect(
      me.equalWithSignErrors(me.fromText("x^2-2x+3"), target, {
        equalityFunction: eqf,
      }),
    ).toEqual({ matched: true, n_sign_errors: 0 });
    expect(
      me.equalWithSignErrors(me.fromText("x^2+2x+3"), target, {
        equalityFunction: eqf,
      }),
    ).toEqual({ matched: true, n_sign_errors: 1 });
  });
});

describe("default_order (simplify=normalizeOrder)", () => {
  // Was a no-op returning `this`, so an attribute whose entire job is to sort
  // did nothing and two orderings of one sum never matched.
  it("sorts without evaluating", () => {
    const a = me.fromText("1x^2+2-0x^2+3+x^2+3x^2+7+4").default_order();
    const b = me.fromText("4-0x^2 +7+ (x^2)1+3+x^2+2+(x^2)3").default_order();
    expect(a.equalsViaSyntax(b)).toBe(true);
    // Every term survives: the constants stay unfolded (7 and 4 are still two
    // terms, not 11) and the `0x^2` term is still there.
    const operands = a.tree.slice(1);
    expect(operands.filter((t: any) => t === 7 || t === 4).length).toBe(2);
    expect(JSON.stringify(operands)).toContain('["*",0,["^","x",2]]');
  });
});

describe("exp and e^ are one spelling", () => {
  it("normalize_function_names folds them together", () => {
    const a = me.fromText("-5e^(-t)").normalize_function_names().simplify();
    const b = me.fromText("-5exp(-t)").normalize_function_names().simplify();
    expect(a.equalsViaSyntax(b)).toBe(true);
    // ...and the reciprocal spellings land there too.
    const c = me.fromText("-5/e^t").normalize_function_names().simplify();
    const d = me.fromText("-5/exp(t)").normalize_function_names().simplify();
    expect(c.equalsViaSyntax(d)).toBe(true);
  });
});

describe("substitute: simultaneous, differing from its sibling only in coercion", () => {
  const f = () => me.fromText("sin(x+y)");

  it("substitute binds every variable at once, as the JS library did", () => {
    // No binding sees another's replacement. This was a left-to-right pass
    // here for a while, on the belief that legacy was one; legacy walks the
    // tree once, and `sin(10 (-π) - π)` was capture, not a feature.
    expect(
      f()
        .substitute({ x: me.fromText("10y"), y: me.fromText("-pi") })
        .toString(),
    ).toBe("sin(10 y - π)");
    // The classic swap, which no sequential pass can do. DoenetML's `Line.js`
    // needs exactly this, substituting a line's declared variable names into
    // `a·x + b·y + c`.
    expect(
      f()
        .substitute({ x: me.fromText("y"), y: me.fromText("x") })
        .toString(),
    ).toBe("sin(y + x)");
    // And a substituted value is *not* reopened to the bindings that follow:
    // legacy leaves the inner `c2` standing.
    expect(
      me
        .fromText("c1+1")
        .substitute({ c1: me.fromText("c2"), c2: me.fromText("5") })
        .toString(),
    ).toBe("c2 + 1");
  });

  it("substitute parses a string binding, and still binds it simultaneously; substitute_all takes it as a symbol", () => {
    // The one difference between the two. Legacy `substitute` parses.
    expect(me.fromText("x+1").substitute({ x: "2y" }).toString()).toBe(
      "2 y + 1",
    );
    expect(me.fromText("x+1").substitute_all({ x: "2y" }).toString()).toBe(
      "2y + 1",
    );
    // Coercion is *all* that separates them, which those two lines alone do
    // not show: they hold under a left-to-right pass too. A string binding is
    // parsed into a tree and then bound at the same instant as every other, so
    // the classic swap survives being written as strings — `x → y, y → x` is
    // `y + x`, where substituting one at a time gives `x + x`.
    expect(me.fromText("x+y").substitute({ x: "y", y: "x" }).toString()).toBe(
      "y + x",
    );
    // Both properties in one expression: the bindings are parsed (`3y` is a
    // product, so it prints spaced, against `substitute_all`'s single symbol
    // `3y`), and neither replacement is reopened to the other. Sequentially
    // this reads `2 * 3 * 2 x + 3 * 2 x`, with the first binding's fresh `y`
    // captured by the second.
    expect(
      me.fromText("2x+3y").substitute({ x: "3y", y: "2x" }).toString(),
    ).toBe("2 * 3 y + 3 * 2 x");
    expect(
      me.fromText("2x+3y").substitute_all({ x: "3y", y: "2x" }).toString(),
    ).toBe("2 * 3y + 3 * 2x");
  });

  it("substitute_all binds every variable at once", () => {
    expect(
      f()
        .substitute_all({ x: me.fromText("10y"), y: me.fromText("-pi") })
        .toString(),
    ).toBe("sin(10 y - π)");
    expect(
      f()
        .substitute_all({ x: me.fromText("y"), y: me.fromText("x") })
        .toString(),
    ).toBe("sin(y + x)");
  });
});

describe("odd roots of negatives read on the real branch, in every spelling", () => {
  // The branch used to depend on whether the radicand was a perfect power:
  // `(-8)^(1/3)` folded to the real `-2` while `(-2)^(1/3)` evaluated to the
  // principal complex value, so four DoenetML `<answer>` cases that scored 1
  // on the legacy engine scored 0 (expected `cbrt(-2)`, typed `(-2)^{1/3}`;
  // and their nthroot/latex/decimal variants). Pinned here at the compat
  // boundary; the crate-level matrix is `tests/odd_root_real_branch.rs`.
  const eq = (a: any, b: any) => a.equals(b);

  it("grades the four regressed answer rows as equal again", () => {
    expect(eq(me.fromText("cbrt(-2)"), me.fromLatex("(-2)^{1/3}"))).toBe(true);
    expect(eq(me.fromText("(-2)^(1/3)"), me.fromLatex("\\sqrt[3]{-2}"))).toBe(
      true,
    );
    expect(eq(me.fromText("nthroot(-2,3)"), me.fromLatex("(-2)^{1/3}"))).toBe(
      true,
    );
    expect(
      eq(me.fromText("(-2)^(1/3)"), me.fromText("-1.2599210498948732")),
    ).toBe(true);
  });

  it("keeps the two rows this engine fixed over legacy", () => {
    expect(eq(me.fromText("cbrt(-2)"), me.fromText("-cbrt(2)"))).toBe(true);
    expect(
      eq(me.fromText("cbrt(-2)"), me.fromText("-1.2599210498948732")),
    ).toBe(true);
  });

  it("no longer tells a perfect power apart from its own factorization", () => {
    expect(
      eq(me.fromText("(-8)^(1/3)"), me.fromText("(-2)^(1/3) * 4^(1/3)")),
    ).toBe(true);
  });

  it("evaluates the value, and leaves even roots complex", () => {
    expect(me.fromText("(-2)^(1/3)").evaluate_to_constant()).toBeCloseTo(
      -1.2599210498948732,
      12,
    );
    // An even root of a negative stays on the principal branch: the value
    // comes back complex (`2i`), not as a real root.
    expect(me.fromText("(-4)^(1/2)").evaluate_to_constant()).toMatchObject({
      re: 0,
      im: 2,
    });
  });
});
