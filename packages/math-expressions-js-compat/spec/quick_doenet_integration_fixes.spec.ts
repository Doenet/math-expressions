// The reproducers from `active-plans/DOENET_INTEGRATION.md` §2–§5, run at the
// library boundary exactly as that document writes them.
//
// The Rust-side counterparts live in
// `math-expressions-rs/tests/doenet_integration_fixes.rs`. These are here
// because the boundary is where DoenetML meets the engine — `.tree`, the
// printers, and the `simplify="numbers"` attribute — and because a fix that
// works in the core but not through wasm is not a fix.
import { describe, it, expect } from "vitest";
import me from "../lib/math-expressions";

describe("§2 — evaluate_numbers folds numbers without collecting like terms", () => {
  // `simplify="numbers"` is "fold numeric constants, leave the symbolic
  // structure alone". Folding `x² + 3x²` into `4x²` is correct but not
  // numeric, and it made the attribute indistinguishable from
  // `simplify="full"` — erasing a documented public distinction.
  it("leaves like symbolic terms as written", () => {
    expect(me.fromText("x^2+3x^2").evaluate_numbers({}).tree).toEqual([
      "+",
      ["^", "x", 2],
      ["*", 3, ["^", "x", 2]],
    ]);
    expect(me.fromText("1x+4x").evaluate_numbers({}).tree).toEqual([
      "+",
      "x",
      ["*", 4, "x"],
    ]);
    // The assertion DoenetML's core makes, and whose failure arrived as a bare
    // `unreachable` trap under `panic = "abort"` (§1, `simplify_math`).
    expect(me.fromText("x+2+x+3+4").evaluate_numbers({}).toString()).toBe("x + x + 9");
  });

  it("still folds the numeric part, across intervening symbolic terms", () => {
    expect(me.fromText("4+x-2").evaluate_numbers({}).tree).toEqual(["+", "x", 2]);
    expect(me.fromText("3*2*x*4").evaluate_numbers({}).tree).toEqual(["*", 24, "x"]);
    // Cancellation is not a shortening rewrite — it has to survive, or
    // `evaluate_numbers` cannot see a zero it is asked about.
    expect(me.fromText("3x-3x").evaluate_numbers({}).tree).toBe(0);
  });

  it("does not change simplify(), which is meant to be aggressive", () => {
    expect(me.fromText("x^2+3x^2").simplify().tree).toEqual(["*", 4, ["^", "x", 2]]);
  });

  it("leaves the skip_ordering form alone", () => {
    expect(me.fromText("1+x+2").evaluate_numbers({ skip_ordering: true }).tree).toEqual(
      ["+", 1, "x", 2],
    );
  });
});

describe("§3 — inverse trig folds at its exact values", () => {
  // The parse was already better than legacy's, which read `sin^(-1)` as
  // `1/sin`. Only the evaluation was missing, so `simplifyOnCompare` could not
  // grade an answer written that way.
  it("folds the identities from the report", () => {
    expect(me.fromText("sin^(-1)(1)").simplify().tree).toEqual(["/", "pi", 2]);
    expect(me.fromText("cos^(-1)(1)").simplify().tree).toBe(0);
    expect(me.fromText("tan^(-1)(1)").simplify().tree).toEqual(["/", "pi", 4]);
  });

  it("compares equal to the closed form, which is what grading needs", () => {
    for (const [a, b] of [
      ["asin(1)", "pi/2"],
      ["acos(-1/2)", "2pi/3"],
      ["atan(sqrt(3))", "pi/3"],
      ["asec(2)", "pi/3"],
      ["acsc(2)", "pi/6"],
      ["acot(1)", "pi/4"],
    ]) {
      expect(me.fromText(`${a} - (${b})`).simplify().tree, `${a} = ${b}`).toBe(0);
    }
  });

  it("leaves a value that is not a rational multiple of pi symbolic", () => {
    for (const s of ["asin(2)", "atan(1/3)", "asin(x)"]) {
      expect(me.fromText(s).simplify().tree[0], s).toBe("apply");
    }
  });
});

describe("§4 — log_b(a) reduces to log(a)/log(b)", () => {
  it("makes the difference of the two spellings zero", () => {
    expect(me.fromText("log_b(a)-log(a)/log(b)").simplify().tree).toBe(0);
    expect(me.fromText("log_2(9)").simplify().tree).toEqual([
      "/",
      ["apply", "log", 9],
      ["apply", "log", 2],
    ]);
  });

  it("does not pre-empt an exact based logarithm", () => {
    // This runs before the numeric-application pass in each simplify round, so
    // it has to decline where that pass would answer.
    expect(me.fromText("log_2(8)").simplify().tree).toBe(3);
    expect(me.fromText("log_10(1000)").simplify().tree).toBe(3);
    expect(me.fromText("log_7(343)").simplify().tree).toBe(3);
  });
});

describe("§5 — a fraction stays a fraction in the tree", () => {
  // Decimals parse to exact rationals by design, so `0.5` and `1/2` were the
  // same value and nothing at the boundary could tell them apart. The fix is a
  // spelling carried on the number, not a serializer change — which is why the
  // issue's "~5-line `number_to_js` patch" framing did not work.
  it("keeps a fraction of integers as a fraction, in every rendering", () => {
    for (const [input, tree, text, latex] of [
      ["3/6", ["/", 1, 2], "1/2", "\\frac{1}{2}"],
      ["5/2", ["/", 5, 2], "5/2", "\\frac{5}{2}"],
      ["1/3", ["/", 1, 3], "1/3", "\\frac{1}{3}"],
    ] as const) {
      const e = me.fromText(input).simplify();
      expect(e.tree, `${input} tree`).toEqual(tree);
      expect(e.toString(), `${input} text`).toBe(text);
      expect(e.toLatex(), `${input} latex`).toBe(latex);
    }
    // The assertion DoenetML's core makes (§1, `arithmetic_on_math`).
    expect(me.fromText("3").divide(me.fromText("6")).simplify().toString()).toBe("1/2");
  });

  it("keeps a typed decimal a decimal", () => {
    // The half that a naive "always emit a fraction" patch breaks: `19.9` *is*
    // `Rat(199, 10)`, and must not read back as ["/", 199, 10].
    expect(me.fromText("0.5").simplify().tree).toBe(0.5);
    expect(me.fromText("19.9").simplify().tree).toBe(19.9);
    expect(me.fromText("0.1+0.2").simplify().tree).toBe(0.3);
    expect(me.fromText("19.9").toString()).toBe("19.9");
  });

  it("lets a decimal operand carry through arithmetic", () => {
    expect(me.fromText("1/2+1/4").simplify().tree).toEqual(["/", 3, 4]);
    expect(me.fromText("1/2+0.25").simplify().tree).toBe(0.75);
    expect(me.fromText("0.5x").simplify().tree).toEqual(["*", 0.5, "x"]);
    expect(me.fromText("x/2").simplify().tree).toEqual(["/", "x", 2]);
  });

  it("does not disturb equality — the spelling is not the value", () => {
    expect(me.fromText("0.5").equals(me.fromText("1/2"))).toBe(true);
    expect(me.fromText("3/6").equals(me.fromText("0.5"))).toBe(true);
    expect(me.fromText("3/6").simplify().evaluate_to_constant()).toBe(0.5);
  });

  it("rounds a decimal quantity, never a written fraction", () => {
    // A fraction a student sees stays a fraction (DoenetML open item 8;
    // `displayDigits` defaults to 3, so every rational passes through this
    // path). What decides is the *spelling*, not whether the rounding happens
    // to be exact: this line asserted `0.33` under the earlier
    // exactness-based rule, which kept `5/2` but decimalized `1/3` — and only
    // once it had been through `simplify`, so the same value displayed two ways
    // in one document. See `ops::numbers::is_written_as_fraction`.
    expect(me.fromText("1/3").simplify().round_numbers_to_decimals(2).tree).toEqual([
      "/",
      1,
      3,
    ]);
    // A decimal-spelled rational still rounds — that is what the spelling is
    // for, and it is what keeps `<round>0.5</round>` meaningful.
    expect(me.fromText("0.5").simplify().round_numbers_to_decimals(0).tree).toBe(1);
    expect(me.fromText("3/6").simplify().round_numbers_to_decimals(3).tree).toEqual([
      "/",
      1,
      2,
    ]);
    expect(me.fromText("5/2").round_numbers_to_precision(3).tree).toEqual(["/", 5, 2]);
  });
});
