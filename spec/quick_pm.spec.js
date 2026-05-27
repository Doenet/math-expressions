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

describe("pm symbolic equality (simplify + equalsViaSyntax)", () => {
  // This block exercises the explicit symbolic path: simplify both sides
  // first, then strict syntax compare. It pins down which transformations
  // canonicalize pm expressions to the same tree (the things `default_order`
  // and the new pm simplify rules cover) — independent of numerical sampling.
  const same = (a, b) =>
    me.fromLatex(a).simplify().equalsViaSyntax(me.fromLatex(b).simplify());

  test("reordering pm terms canonicalizes to the same tree", () => {
    expect(same("5 \\pm 3 \\pm 4", "5 \\pm 4 \\pm 3")).toBe(true);
    expect(same("a \\pm 2 b", "\\pm 2 b + a")).toBe(true);
  });
  test("scaling rule pulls constants inside pm: 2 · ±x → ±(2x)", () => {
    // After simplify, 2·±b and ±(2b) should land on the same canonical form.
    expect(
      me
        .fromAst(["*", 2, ["pm", "b"]])
        .simplify()
        .equalsViaSyntax(me.fromAst(["pm", ["*", 2, "b"]]).simplify()),
    ).toBe(true);
  });
  test("negation rule: -(±x) canonicalizes to ±x", () => {
    expect(
      me
        .fromAst(["-", ["pm", "x"]])
        .simplify()
        .equalsViaSyntax(me.fromAst(["pm", "x"]).simplify()),
    ).toBe(true);
  });
  test("does NOT canonicalize different pm terms together", () => {
    // simplification deliberately avoids combining independent pm terms
    expect(same("5 \\pm 3 \\pm 4", "5 \\pm 7")).toBe(false);
    expect(same("\\pm 3 + \\pm 3", "2 \\pm 3")).toBe(false);
  });
  test("does NOT distribute multiplication over a sum containing pm", () => {
    // simplify doesn't auto-distribute c·(a + ±b); these stay distinct trees
    expect(same("2(a \\pm b)", "2 a \\pm 2 b")).toBe(false);
  });
  test("does NOT have a pm-of-negative-number rule: ±3 vs ±(-3) stay distinct", () => {
    // ["pm", -3] and ["pm", 3] denote the same set, but no simplify rule
    // collapses them — that would require a transformation, not just
    // ambiguity removal
    expect(same("5 \\pm 3", "5 \\pm (-3)")).toBe(false);
  });
});

describe("pm equality via .equals (orchestrator: simplify+syntax, then numeric)", () => {
  // `.equals()` is the orchestrator from lib/expression/equality.js. It tries
  // simplify+equalsViaSyntax first; on miss it falls through to
  // equalsViaFiniteField and equalsViaComplex (which dispatches to
  // pm_equals_numerical when pm is present). The tests below succeed on the
  // numeric leg — simplify alone does NOT canonicalize their trees together
  // (see the "symbolic" block above for the precise demarcation).
  test("scaling: 2(a ± b) equals 2a ± 2b via numeric", () => {
    expect(
      me.fromLatex("2(a \\pm b)").equals(me.fromLatex("2 a \\pm 2 b")),
    ).toBe(true);
  });
  test("pm symmetry: 5 ± 3 equals 5 ± (-3) via numeric (same value set)", () => {
    expect(me.fromLatex("5 \\pm 3").equals(me.fromLatex("5 \\pm (-3)"))).toBe(
      true,
    );
  });
  test("reordering pm terms is equal (succeeds via simplify+syntax already)", () => {
    expect(
      me.fromLatex("5 \\pm 3 \\pm 4").equals(me.fromLatex("5 \\pm 4 \\pm 3")),
    ).toBe(true);
  });
  test("does NOT combine different pm terms (5 ± 3 ± 4 != 5 ± 7)", () => {
    expect(
      me.fromLatex("5 \\pm 3 \\pm 4").equals(me.fromLatex("5 \\pm 7")),
    ).toBe(false);
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

describe("pm interaction with expand()", () => {
  // Distributing a non-pm factor over a sum that contains pm is safe — the
  // value set is unchanged. Distributing a pm-containing factor over a sum
  // is unsound, since the original has one ± choice tied to the whole
  // factor but the distributed form has independent ± per term. expand()
  // is guarded accordingly. (expand already runs collect_like_terms_factors
  // internally, so an explicit `.simplify()` after expand is typically a
  // no-op; we use the simplify+equalsViaSyntax form here when we want to
  // pin down the canonical shape independent of numeric sampling.)

  test("x(y ± z) distributes to xy ± xz (non-pm factor, pm in sum)", () => {
    expect(
      me
        .fromLatex("x(y \\pm z)")
        .expand()
        .equalsViaSyntax(me.fromLatex("x y \\pm x z").simplify()),
    ).toBe(true);
  });

  test("\\pm x(y+z) keeps pm outside the distributed sum (±(xy+xz))", () => {
    // parses as `["pm", x*(y+z)]` — the pm wraps the whole product, so
    // distribution happens inside the pm and remains a single sign choice
    const expanded = me.fromLatex("\\pm x(y+z)").expand().tree;
    expect(expanded[0]).toBe("pm");
    expect(me.fromAst(expanded).equals(me.fromLatex("\\pm x(y+z)"))).toBe(
      true,
    );
  });

  test("(±x)(y+z) is NOT distributed to ±xy ± ±xz (would inflate value set)", () => {
    // The unsafe expansion `(±x)(y+z) → (±x)y + (±x)z` would produce two
    // independent pm operators, changing the value set from 2 to 4. Our
    // guard prevents that match. The result still equals the input.
    const original = me.fromAst(["*", ["pm", "x"], ["+", "y", "z"]]);
    const expanded = original.expand();
    // The expanded form is mathematically equal to the original (the
    // numeric path can confirm because the pm count is preserved):
    expect(original.equals(me.fromAst(expanded.tree))).toBe(true);
    // It should NOT have become a sum of two independent pm operands
    // (which would mean two `pm` operators at the top level of a `+`).
    const sumOfTwoPm =
      Array.isArray(expanded.tree) &&
      expanded.tree[0] === "+" &&
      expanded.tree
        .slice(1)
        .filter((t) => Array.isArray(t) && t[0] === "pm").length >= 2;
    expect(sumOfTwoPm).toBe(false);
  });

  test("(a+b)^3 with pm in a is left unexpanded (avoids spurious sign combinations)", () => {
    // (±k+m)^3 = {(k+m)^3, (m-k)^3}  (2 values). Expanded form
    // ±k^3 + 3k^2 m + ±(3km^2) + m^3 has independent ± in two odd-degree
    // terms (the squared term is sign-invariant), giving 4 distinct
    // values — three of which the original cannot produce. So expand
    // leaves the power alone.
    const expanded = me.fromLatex("(\\pm k + m)^3").expand().tree;
    expect(expanded[0]).toBe("^");
    expect(expanded[2]).toBe(3);
  });

  test("(a+b)^4 with pm in a is left unexpanded for the same reason", () => {
    const expanded = me.fromLatex("(\\pm k + m)^4").expand().tree;
    expect(expanded[0]).toBe("^");
    expect(expanded[2]).toBe(4);
  });

  test("non-pm polynomials still expand normally (no regressions)", () => {
    expect(
      me
        .fromLatex("a(b+c)")
        .expand()
        .equalsViaSyntax(me.fromLatex("a b + a c").simplify()),
    ).toBe(true);
    expect(
      me
        .fromLatex("(a+b)(c+d)")
        .expand()
        .equalsViaSyntax(
          me.fromLatex("a c + a d + b c + b d").simplify(),
        ),
    ).toBe(true);
    expect(
      me
        .fromLatex("(a+b)^3")
        .expand()
        .equalsViaSyntax(
          me.fromLatex("a^3 + 3 a^2 b + 3 a b^2 + b^3").simplify(),
        ),
    ).toBe(true);
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
