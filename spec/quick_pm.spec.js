import me from "../lib/math-expressions";
import {
  contains_pm,
  expand_pm_signs,
  count_pm,
} from "../lib/expression/pm.js";

describe("pm helpers", () => {
  test("contains_pm detects pm at the root", () => {
    expect(contains_pm(["pm", 3])).toBe(true);
  });
  test("contains_pm detects pm nested under +", () => {
    expect(contains_pm(["+", 5, ["pm", 3]])).toBe(true);
  });
  test("contains_pm returns false when no pm anywhere", () => {
    expect(contains_pm(["+", 5, ["-", 3]])).toBe(false);
    expect(contains_pm("x")).toBe(false);
    expect(contains_pm(5)).toBe(false);
  });
  test("count_pm counts independent pm operators", () => {
    expect(count_pm(["+", 5, ["pm", 3], ["pm", 4]])).toBe(2);
    expect(count_pm(["+", 5, ["-", 3]])).toBe(0);
  });
  test("expand_pm_signs enumerates 2^n sign assignments", () => {
    const variants = expand_pm_signs(["+", 5, ["pm", 3]]);
    expect(variants).toHaveLength(2);
    expect(variants).toEqual(
      expect.arrayContaining([
        ["+", 5, 3],
        ["+", 5, ["-", 3]],
      ]),
    );
  });
  test("expand_pm_signs returns 4 for two independent pm", () => {
    const variants = expand_pm_signs(["+", 5, ["pm", 3], ["pm", 4]]);
    expect(variants).toHaveLength(4);
  });
  test("expand_pm_signs throws when count exceeds MAX_PM_COUNT", () => {
    // Build a tree with 11 pm operators — one over the limit.
    let tree = "x";
    for (let i = 0; i < 11; i++) {
      tree = ["+", tree, ["pm", i]];
    }
    expect(() => expand_pm_signs(tree)).toThrow(/plus-minus/);
  });
});

describe("pm round-trip via converters", () => {
  test("latex → ast → latex", () => {
    expect(me.fromLatex("5 \\pm 3").toLatex()).toEqual("5 \\pm 3");
    expect(me.fromLatex("5 \\pm 3 \\pm 4").toLatex()).toEqual(
      "5 \\pm 3 \\pm 4",
    );
    expect(me.fromLatex("\\pm 3").toLatex()).toEqual("\\pm 3");
  });
  test("text input → latex (unicode ±)", () => {
    expect(me.fromText("5 ± 3").toLatex()).toEqual("5 \\pm 3");
  });
  test("text input → latex (ascii plusminus)", () => {
    expect(me.fromText("5 plusminus 3").toLatex()).toEqual("5 \\pm 3");
  });
  test("ast → text (ascii)", () => {
    expect(
      me.fromAst(["+", 5, ["pm", 3]]).toString({ output_unicode: false }),
    ).toEqual("5 plusminus 3");
  });
});

describe("pm strict syntax equality", () => {
  test("same tree is strict-equal", () => {
    expect(
      me.fromLatex("5 \\pm 3").equalsViaSyntax(me.fromLatex("5 \\pm 3")),
    ).toBe(true);
  });
  test("ambiguity-only normalization: pm of -3 vs unary minus", () => {
    // ["pm", -3] and ["pm", ["-", 3]] differ syntactically but the existing
    // normalize_negative_numbers pass folds ["-", 3] into -3; strict-syntax
    // equality should still see them as equal.
    expect(
      me.fromAst(["pm", -3]).equalsViaSyntax(me.fromAst(["pm", ["-", 3]])),
    ).toBe(true);
  });
  test("reordering does NOT make pm expressions strict-equal", () => {
    expect(
      me
        .fromLatex("5 \\pm 3 \\pm 4")
        .equalsViaSyntax(me.fromLatex("5 \\pm 4 \\pm 3")),
    ).toBe(false);
  });
});

describe("pm looser symbolic equality (via .equals + simplify)", () => {
  test("reordering pm terms is equal after simplification", () => {
    expect(
      me.fromLatex("5 \\pm 3 \\pm 4").equals(me.fromLatex("5 \\pm 4 \\pm 3")),
    ).toBe(true);
  });
  test("scaling into pm: 2(a ± b) equals 2a ± 2b", () => {
    expect(
      me.fromLatex("2(a \\pm b)").equals(me.fromLatex("2 a \\pm 2 b")),
    ).toBe(true);
  });
  test("pm symmetry: 5 ± 3 equals 5 ± (-3)", () => {
    expect(me.fromLatex("5 \\pm 3").equals(me.fromLatex("5 \\pm (-3)"))).toBe(
      true,
    );
  });
  test("does NOT combine different pm terms (5 ± 3 ± 4 != 5 ± 7)", () => {
    expect(me.fromLatex("5 \\pm 3 \\pm 4").equals(me.fromLatex("5 \\pm 7"))).toBe(
      false,
    );
  });
  test("does NOT combine two ±3 into 2±3", () => {
    expect(
      me.fromLatex("\\pm 3 + \\pm 3").equals(me.fromLatex("2 \\pm 3")),
    ).toBe(false);
  });
  test("expanding (a ± b)(c ± d) into 4 ± terms is NOT equal", () => {
    expect(
      me
        .fromLatex("(a \\pm b)(c \\pm d)")
        .equals(me.fromLatex("a c \\pm a d \\pm b c \\pm b d")),
    ).toBe(false);
  });
  test("pm vs no pm is not equal", () => {
    expect(me.fromLatex("5 \\pm 3").equals(me.fromLatex("5"))).toBe(false);
  });
  test("function/variable overlap (`f` used as both) is handled with pm", () => {
    // Exercises the function-variable disambiguation path inside
    // pm_equals_numerical — `f` appears as both a free variable and an
    // applied function in the same expression.
    expect(
      me.fromText("f(x) + ± f").equals(me.fromText("f(x) + ± f")),
    ).toBe(true);
    expect(
      me.fromText("f(x) + ± f").equals(me.fromText("± f + f(x)")),
    ).toBe(true);
  });
});

describe("pm inside structural containers (tuples, vectors, relations)", () => {
  test("tuple component containing pm equals itself", () => {
    expect(
      me.fromText("(5 ± 3, 4)").equals(me.fromText("(5 ± 3, 4)")),
    ).toBe(true);
  });
  test("tuple component containing pm respects component order", () => {
    // tuples are ordered, so swapping components should not be equal
    expect(
      me.fromText("(5 ± 3, 4)").equals(me.fromText("(4, 5 ± 3)")),
    ).toBe(false);
  });
  test("tuple with pm in one component and matching non-pm value is not equal", () => {
    expect(
      me.fromText("(5 ± 3, 4)").equals(me.fromText("(8, 4)")),
    ).toBe(false);
  });
  test("tuple with pm: reordered pm-bearing component (commutative under pm) still equal", () => {
    // 5 ± 3 should be set-equal to 3 ± 5 (both yield {8, 2} and {8, -2}? no
    // actually {3+5, 3-5} = {8, -2} vs {5+3, 5-3} = {8, 2}; not the same set)
    // so this should be NOT equal
    expect(
      me.fromText("(5 ± 3, 4)").equals(me.fromText("(3 ± 5, 4)")),
    ).toBe(false);
  });
  test("vector component containing pm equals itself", () => {
    expect(
      me
        .fromLatex("\\langle 5 \\pm 3, 4 \\rangle")
        .equals(me.fromLatex("\\langle 5 \\pm 3, 4 \\rangle")),
    ).toBe(true);
  });
  test("equation with pm on one side compares via standard form", () => {
    // x = 5 ± 3   <=>   x - 5 ∓ 3 = 0
    // Compare to:  x - 5 = ± 3, whose standard form is x - 5 ∓ 3 = 0.
    // These represent the same equation set {x = 8, x = 2}.
    expect(
      me.fromText("x = 5 ± 3").equals(me.fromText("x - 5 = ± 3")),
    ).toBe(true);
  });
  test("equation with pm: y = 5 ± 3 equals y = 5 ± 3", () => {
    expect(
      me.fromText("y = 5 ± 3").equals(me.fromText("y = 5 ± 3")),
    ).toBe(true);
  });
  test("equation with pm: y = 5 ± 3 NOT equal to y = 5 + 3", () => {
    expect(
      me.fromText("y = 5 ± 3").equals(me.fromText("y = 8")),
    ).toBe(false);
  });
});

describe("pm with allowed_error_in_numbers tolerance", () => {
  test("5 ± 3 vs 5.05 ± 3 is true with allowed_error_in_numbers=0.1", () => {
    expect(
      me.fromText("5 ± 3").equals(me.fromText("5.05 ± 3"), {
        allowed_error_in_numbers: 0.1,
      }),
    ).toBe(true);
  });
  test("5 ± 3 vs 5.05 ± 3 is false without allowed_error_in_numbers", () => {
    expect(me.fromText("5 ± 3").equals(me.fromText("5.05 ± 3"))).toBe(false);
  });
  test("5 ± 3 vs 5.5 ± 3 is false even with allowed_error_in_numbers=0.01", () => {
    // 0.01 * 5 = 0.05 allowed, but the actual error is 0.5; should fail.
    expect(
      me.fromText("5 ± 3").equals(me.fromText("5.5 ± 3"), {
        allowed_error_in_numbers: 0.01,
      }),
    ).toBe(false);
  });
});

describe("pm simplification preserves independence", () => {
  test("5 ± 3 ± 4 does not collapse", () => {
    const tree = me.fromLatex("5 \\pm 3 \\pm 4").simplify().tree;
    expect(tree).toEqual(["+", 5, ["pm", 3], ["pm", 4]]);
  });
  test("non-pm numerics fold: 5 + a + 4 ± 3 → 9 + a ± 3", () => {
    const tree = me.fromLatex("5 + a + 4 \\pm 3").simplify().tree;
    // sort/flatten may reorder; just check the set of operands
    expect(tree[0]).toBe("+");
    expect(tree.slice(1)).toEqual(expect.arrayContaining(["a", 9, ["pm", 3]]));
    expect(tree.slice(1)).toHaveLength(3);
  });
  test("-(±x) absorbs to ±x", () => {
    expect(me.fromAst(["-", ["pm", "x"]]).simplify().tree).toEqual([
      "pm",
      "x",
    ]);
  });
});
