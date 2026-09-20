// Upstream requests 08 and 09, checked at the library boundary — the form
// DoenetML filed them in.
import { describe, it, expect } from "vitest";
import me from "../lib/math-expressions";
import textToAst from "../lib/converters/text-to-ast";

describe("display rounding is exact at large magnitudes (08)", () => {
  // `<number>` defaults to displayDigits=3, displayDecimals=2, so this is the
  // path every number a student sees goes through — not an edge case. Asking
  // for 2 decimal places of an integer is a no-op; it was returning
  // 1.9999999999999997e21, because the pass computed `(v*100).round()/100` and
  // 2e23 is not representable.
  it("leaves a value that is already an integer alone", () => {
    expect(me.round_numbers_to_precision_plus_decimals(2e21, 3, 2).tree).toBe(
      2e21,
    );
    expect(me.round_numbers_to_precision_plus_decimals(-2e21, 3, 2).tree).toBe(
      -2e21,
    );
    expect(me.round_numbers_to_precision_plus_decimals(6.02e23, 3, 2).tree).toBe(
      6.02e23,
    );
  });

  // Asking for more digits used to be the workaround: it returned the exact
  // answer where 3 did not. All the ways of asking have to agree.
  it("agrees with the precision-only and decimals-only passes", () => {
    expect(me.round_numbers_to_precision(2e21, 3).tree).toBe(2e21);
    expect(me.round_numbers_to_precision_plus_decimals(2e21, 15, 2).tree).toBe(
      2e21,
    );
    expect(me.round_numbers_to_decimals(2e21, 2).tree).toBe(2e21);
  });

  it("still rounds what it is supposed to round", () => {
    expect(me.round_numbers_to_decimals(2.345, 2).tree).toBe(2.35);
    expect(me.round_numbers_to_precision(12345.6789, 3).tree).toBe(12300);
    expect(
      me.round_numbers_to_precision_plus_decimals(12345.6789, 3, 2).tree,
    ).toBe(12345.68);
    // Rounding is of the float's *shortest decimal spelling* — what a reader
    // sees — not of the exact binary value it holds. 2.675 is stored as
    // 2.67499999999999982…, so reading the stored value would give 2.67; this
    // asserted that until it was checked against mathjs, which legacy rounded
    // through: `format(2.675, {notation:"fixed", precision:2})` is "2.68".
    expect(me.round_numbers_to_decimals(2.675, 2).tree).toBe(2.68);
  });

  // A non-finite value has no decimal expansion to round. (`.tree` reports it
  // in the tagged form — that is the separate leak DoenetML has open, not
  // something rounding introduces; `evaluate_to_constant` reads back a plain
  // `Infinity`.)
  it("passes non-finite values through untouched", () => {
    expect(
      me.round_numbers_to_precision_plus_decimals(Infinity, 3, 2).tree,
    ).toEqual(Infinity);
    expect(
      me
        .round_numbers_to_precision_plus_decimals(-Infinity, 3, 2)
        .evaluate_to_constant(),
    ).toBe(-Infinity);
    expect(me.round_numbers_to_decimals(-Infinity, 2).tree).toEqual(-Infinity);
  });
});

describe("parseScientificNotation (09)", () => {
  // Reported as having no effect. It has one — for uppercase `E`, which is the
  // only marker legacy ever accepted, because `e` is Euler's number in this
  // grammar. The repro used `7e-12`, so it was reading Euler's number both
  // times and the flag looked inert.
  const convert = (text: string, parseScientificNotation: boolean) =>
    new textToAst({ parseScientificNotation }).convert(text);

  it("is honored, on uppercase E", () => {
    expect(convert("7E-12", true)).toBe(7e-12);
    expect(convert("3.2E-12", true)).toBe(3.2e-12);
    expect(convert("7E-12", false)).toEqual(["+", ["*", 7, "E"], -12]);
  });

  it("leaves lowercase e as Euler's number, either way", () => {
    const euler = ["+", ["*", 7, "e"], -12];
    expect(convert("7e-12", true)).toEqual(euler);
    expect(convert("7e-12", false)).toEqual(euler);
    // Pinned by the legacy spec suite, which is why it cannot simply change.
    expect(convert("1.2e-3", true)).toEqual(["+", ["*", 1.2, "e"], -3]);
  });

  it("requires the exponent to end the expression or close a group", () => {
    expect(convert("3.1E-3 + 2", true)).toEqual(["+", ["*", 3.1, "E"], -3, 2]);
    expect(convert("(3.1E-3, 1E2)", true)).toEqual(["tuple", 0.0031, 100]);
  });
});
