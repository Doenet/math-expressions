import { describe, expect, it, afterEach } from "vitest";
import me from "../lib/math-expressions";

// The declaration is ambient (the Rust core keeps it in a thread-local rather
// than baking it into an instance the way `createInstance` did), so every test
// that changes it puts it back.
const DEFAULTS = {
  define_pi: true,
  define_e: true,
  define_i: true,
  sort_constants_first: false,
  // Not a constant declaration, but it rides the same policy object and FFI:
  // strict `0^0` → NaN by default, non-strict → 1 (`me.math.pow_strict`).
  pow_strict: true,
};

afterEach(() => {
  me.setConstantPolicy(DEFAULTS);
});

describe("constant policy", () => {
  it("defaults to all three declared and alphabetical order", () => {
    expect(me.getConstantPolicy()).toEqual(DEFAULTS);
  });

  it("keeps absent keys and rejects unknown ones", () => {
    me.setConstantPolicy({ define_e: false });
    expect(me.getConstantPolicy()).toEqual({ ...DEFAULTS, define_e: false });
    expect(() => me.setConstantPolicy({ define_ee: true })).toThrow();
  });

  it("declares e a variable, so e^x stops being the exponential", () => {
    expect(me.fromText("log(e)").simplify().toString()).toEqual("1");
    me.setConstantPolicy({ define_e: false });
    expect(me.fromText("log(e)").simplify().toString()).toEqual("log(e)");
  });

  it("declares i a variable, so the complex folds stand down", () => {
    expect(me.fromText("i i").simplify().toString()).toEqual("-1");
    me.setConstantPolicy({ define_i: false });
    expect(me.fromText("i i").simplify().toString()).toEqual("i^2");
  });

  it("samples an undeclared constant as a free variable", () => {
    expect(me.fromText("sin(pi)").equals(me.fromText("0"), {})).toBe(true);
    me.setConstantPolicy({ define_pi: false });
    expect(me.fromText("sin(pi)").equals(me.fromText("0"), {})).toBe(false);
  });

  it("does not reorder on the declaration alone", () => {
    const written = "x + e + pi + a";
    const declared = me.fromText(written).simplify().toString();
    me.setConstantPolicy({ define_pi: false, define_e: false, define_i: false });
    expect(me.fromText(written).simplify().toString()).toEqual(declared);
    expect(declared).toEqual("a + e + π + x");
  });

  it("puts declared constants first only when asked", () => {
    expect(me.fromText("r 2 pi").simplify().toString()).toEqual("2 π r");
    me.setConstantPolicy({ sort_constants_first: true });
    // Conventional within the promoted class — π, then e, then i.
    expect(me.fromText("x + e + pi + a").simplify().toString()).toEqual(
      "π + e + a + x",
    );
    expect(me.fromText("2 i pi").simplify().toString()).toEqual("2 π i");
  });

  it("leaves canonical trees comparable across policies", () => {
    // The comparator is policy-blind by design: a stored expression must not
    // stop matching because a document later redeclared a name.
    const declared = me.fromText("x + e + pi + a").simplify().tree;
    me.setConstantPolicy({ define_pi: false, define_e: false, define_i: false });
    expect(me.fromText("x + e + pi + a").simplify().tree).toEqual(declared);
  });
});
