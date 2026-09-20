// The three DOENET_INTEGRATION items that were still open, checked at the
// library boundary — which is where DoenetML meets them, and where each was
// originally reported as a reproducible one-liner.
import { describe, it, expect } from "vitest";
import me from "../lib/math-expressions";

describe("evaluate_to_constant reports an infinite value", () => {
  // Item 1. An unbounded endpoint is ordinary — [-∞, ∞] is the default domain
  // of a function curve — and `null` is not a way to say so: it crashes
  // `fromAst` on the way back into a tree, and reads as 0 to `Math.max` where
  // it does not crash, silently turning an unbounded endpoint into a bounded
  // one.
  it("returns the infinity it was given", () => {
    expect(me.fromAst(-Infinity).evaluate_to_constant()).toBe(-Infinity);
    expect(me.fromAst(Infinity).evaluate_to_constant()).toBe(Infinity);
    expect(me.fromAst(-10).evaluate_to_constant()).toBe(-10); // unchanged
  });

  it("survives the round trip back into a tree", () => {
    const back = me.fromAst(me.fromAst(-Infinity).evaluate_to_constant());
    expect(back.tree).toEqual(-Infinity);
  });

  it("reaches infinity through arithmetic too", () => {
    expect(me.fromText("1/0").evaluate_to_constant()).toBe(Infinity);
    expect(me.fromText("Infinity+1").evaluate_to_constant()).toBe(Infinity);
    expect(me.fromText("1/Infinity").evaluate_to_constant()).toBe(0);
  });

  // "No numeric value" has exactly one spelling, and it is `NaN` — legacy's.
  // This method answered `null` for a free variable for a while, on the theory
  // that "undecided" was worth telling apart from "decided to be NaN". It is,
  // but not at this cost: `null` coerces to `0`, satisfies `<=`, and slips past
  // `Number.isNaN`, so an expression with no value read as a real one on every
  // consumer that had not been individually taught otherwise.
  //
  // A caller that really wants the distinction still has `variables()`.
  it("reports both an indeterminate form and a free variable as NaN", () => {
    expect(me.fromText("Infinity-Infinity").evaluate_to_constant()).toBeNaN();
    expect(me.fromText("0/0").evaluate_to_constant()).toBeNaN();
    expect(me.fromText("x+1").evaluate_to_constant()).toBeNaN();
    // The distinction, for anyone who needs it.
    expect(me.fromText("x+1").variables()).toEqual(["x"]);
    expect(me.fromText("0/0").variables()).toEqual([]);
  });

  // The property that makes `NaN` the right marker and `null` the wrong one:
  // it survives being computed with. Each of these is a shape that actually
  // occurred on a grading path while the sentinel was `null` — a width, a
  // coordinate average, a slope, a comparison, a distance.
  it("poisons arithmetic and comparison rather than reading as zero", () => {
    const noValue = me.fromText("x+1").evaluate_to_constant();
    expect(noValue + 5).toBeNaN();
    expect(noValue * 2).toBeNaN();
    expect(Number(noValue)).toBeNaN();
    expect((noValue + 3) / 2).toBeNaN();
    expect(noValue <= 1).toBe(false);
    expect(noValue >= -3).toBe(false);
    expect(Number.isNaN(noValue)).toBe(true);
    expect(Number.isFinite(noValue)).toBe(false);
    // A blank answer — the `<constrainTo>` / `<isBetween>` shape.
    expect(me.fromText("＿").evaluate_to_constant() < 1).toBe(false);
  });

  // Legacy returned a math.js complex object for a non-real value rather than
  // discarding it. The wasm entry point reports only the real case, so the
  // wrapper falls back to `evaluate_to_complex`.
  it("keeps a complex value instead of dropping it", () => {
    const v = me.fromText("i").evaluate_to_constant();
    expect(v.re).toBe(0);
    expect(v.im).toBe(1);
    expect(me.fromText("2+3").evaluate_to_constant()).toBe(5); // still a number
  });
});

describe("scientific notation past the magnitude threshold", () => {
  // Item 3. Legacy had no threshold of its own — it called `toString()` and
  // switched whenever the result contained an `e` — so the switch is the
  // ECMAScript rule, which is what `avoidScientificNotation` is named against.
  it("switches where JavaScript's toString does", () => {
    expect(me.fromAst(1e20).toString()).toEqual("100000000000000000000");
    expect(me.fromAst(1e21).toString()).toEqual("1 * 10^21");
    expect(me.fromAst(1e-6).toString()).toEqual("0.000001");
    expect(me.fromAst(1e-7).toString()).toEqual("1 * 10^(-7)");
  });

  it("spells it per output format", () => {
    expect(me.fromAst(1.23e22).toString()).toEqual("1.23 * 10^22");
    expect(me.fromAst(1.23e22).toLatex()).toEqual("1.23 \\cdot 10^{22}");
    expect(me.fromAst(1.23e-11).toString()).toEqual("1.23 * 10^(-11)");
    expect(me.fromAst(1.23e-11).toLatex()).toEqual("1.23 \\cdot 10^{-11}");
  });

  it("honors avoidScientificNotation", () => {
    const opts = { avoidScientificNotation: true };
    expect(me.fromAst(1.23e30).toString(opts)).toEqual(
      "1230000000000000000000000000000",
    );
    expect(me.fromAst(1.23e-12).toLatex(opts)).toEqual("0.00000000000123");
  });

  // The rendered form is the product it is spelled as, so it parenthesises as
  // a power's base — and re-parses, which the bare `1.23e22` form would not.
  it("stays re-parseable", () => {
    const s = me.fromAst(1.23e-11).toString();
    expect(me.fromText(s).evaluate_to_constant()).toBeCloseTo(1.23e-11, 20);
  });
});

describe("a float survives the AST boundary unchanged", () => {
  // Not one of the three, but it blocked the item-3 output from being right:
  // serde_json's float parser is off by one ulp on some literals, so the tree
  // held a different number than was passed in and the mantissa printed as
  // 1.2300000000000001.
  it("round-trips the exact double", () => {
    for (const v of [1.23e-26, 1.23e-11, 0.1, 1.5e300, 4.35e-15]) {
      expect(me.fromAst(v).tree, String(v)).toBe(v);
    }
  });
});

describe("add_unit takes the shapes its declaration promises", () => {
  // The wasm entry point is `add_unit(unit: &str)`, and wasm-bindgen reads a
  // non-string argument as a pointer/length pair into linear memory. The
  // published declaration says `Expression | Tree`, as legacy's did, so both
  // documented spellings used to fail — an `Expression` with
  // `RuntimeError: memory access out of bounds`, an array `Tree` with
  // `arg.charCodeAt is not a function`. Neither is something a caller can
  // recover from, and the first corrupts the wasm heap rather than throwing at
  // the boundary.
  const expected = me.fromText("50%").tree;

  it("accepts a unit name", () => {
    expect(me.fromAst(50).add_unit("%").tree).toEqual(expected);
  });

  it("accepts an Expression, as the declaration says", () => {
    expect(me.fromAst(50).add_unit(me.fromText("%")).tree).toEqual(expected);
  });

  it("accepts a string Tree, as the declaration says", () => {
    expect(me.fromAst(50).add_unit("deg").tree).toEqual(
      me.fromText("50deg").tree,
    );
  });

  it("still scales back out through remove_units", () => {
    expect(
      me.fromAst(50).add_unit(me.fromText("%")).evaluate_to_constant(),
    ).toBe(0.5);
  });
});

describe("the parsers reject a non-string rather than reading memory", () => {
  // `parse_text`/`parse_latex` are `(s: &str)` on the Rust side, so a
  // non-string was read as a pointer/length pair into linear memory:
  // `me.fromText(5)` was `RuntimeError: memory access out of bounds`, and
  // `me.fromText(["x"])` was `arg.charCodeAt is not a function`. These are the
  // package's two most-used entry points, and the message is one a student can
  // reach — `<mathInput showPreview>` renders the parser's complaint.
  //
  // Nothing here used to succeed, so this is only a clearer failure. The
  // sibling `add_unit` case above takes a coercion instead, because *its*
  // declaration invites an `Expression | Tree`; `fromText` is declared to take
  // a string, and there is no faithful reading of anything else.
  const bad: [string, unknown][] = [
    ["a number", 5],
    ["an Expression", me.fromText("x")],
    ["an AST tree", ["+", 1, "x"]],
    ["a plain object", {}],
    ["null", null],
    ["undefined", undefined],
  ];

  for (const [what, value] of bad) {
    it(`fromText rejects ${what} with a TypeError`, () => {
      expect(() => me.fromText(value as string)).toThrow(TypeError);
      expect(() => me.fromText(value as string)).toThrow(/expected a string/);
    });

    it(`fromLatex rejects ${what} with a TypeError`, () => {
      expect(() => me.fromLatex(value as string)).toThrow(TypeError);
      expect(() => me.fromLatex(value as string)).toThrow(/expected a string/);
    });
  }

  it("still parses a string, with and without options", () => {
    expect(me.fromText("x+1").tree).toEqual(["+", "x", 1]);
    expect(me.fromText("xy", { splitSymbols: false }).tree).toEqual("xy");
    expect(me.fromLatex("\\frac{1}{2}").tree).toEqual(["/", 1, 2]);
  });

  it("still parses a String object, which wasm-bindgen always read", () => {
    // eslint-disable-next-line no-new-wrappers
    expect(me.fromText(new String("x+1") as unknown as string).tree).toEqual([
      "+",
      "x",
      1,
    ]);
  });

  it("leaves `me.from` free to try both parsers on a real string", () => {
    // `from` catches the text parser's error to retry as LaTeX, so the new
    // throw must not be reachable from there: it guards non-strings, and
    // `from` never hands the parsers one.
    expect(me.from("\\frac{1}{2}").tree).toEqual(["/", 1, 2]);
    expect(me.from(["+", 1, "x"]).tree).toEqual(["+", 1, "x"]);
    expect(me.from(5).tree).toEqual(5);
  });
});

describe("Expression#match and me.utils.match are one implementation", () => {
  // `allow_extended_match` lets a `+`/`*` pattern match a subset of a larger
  // sum or product, and it is handled outside the Rust matcher — by enumerating
  // operand subsets in `trees/flatten.ts`. `Expression#match` used to call the
  // matcher directly, sharing only the option normalizer, so it dropped the
  // option while `me.utils.match` honored it: the same call bound `b` to
  // `y + z` through one entry point and to `y`, with `z` reported as skipped,
  // through the other. The comment there claimed the two could not drift.
  const options = {
    variables: { a: true, b: true },
    allow_extended_match: true,
  };

  it("agrees on an extended match", () => {
    const viaExpression = me
      .fromText("x+y+z")
      .match(me.fromText("a+b"), options);
    const viaUtils = me.utils.match(
      me.fromText("x+y+z").tree,
      me.fromText("a+b").tree,
      options,
    );
    expect(viaExpression).toEqual(viaUtils);
    // ...and that shared answer is the extended one: `z` set aside, not
    // swallowed into `b`.
    expect(viaExpression).toMatchObject({ a: "x", b: "y", _skipped: ["z"] });
  });

  it("still agrees where the option is absent", () => {
    const plain = { variables: { a: true, b: true } };
    expect(me.fromText("x+y+z").match(me.fromText("a+b"), plain)).toEqual(
      me.utils.match(me.fromText("x+y+z").tree, me.fromText("a+b").tree, plain),
    );
  });

  it("keeps the no-options legacy default, where every leaf binds", () => {
    // Not delegated blindly: an absent or empty options object still takes the
    // path where every string leaf in the pattern is a parameter.
    expect(me.fromText("x+y").match(me.fromText("a+b"))).toEqual({
      a: "x",
      b: "y",
    });
    expect(me.fromText("x+y").match(me.fromText("a+b"), {})).toEqual({
      a: "x",
      b: "y",
    });
  });

  it("still parses a string pattern rather than reading it as a leaf", () => {
    expect(me.fromText("x+y").match("a+b")).toEqual({ a: "x", b: "y" });
  });

  it("still answers false where nothing matches", () => {
    expect(me.fromText("x*y").match(me.fromText("a+b"), options)).toEqual(
      false,
    );
  });
});
