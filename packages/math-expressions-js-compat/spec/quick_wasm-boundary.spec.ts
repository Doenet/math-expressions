// Wasm-boundary JSON contracts: `copy()`, `equals_with_options`,
// `structural_equality`, and `check_structural_comparison`. These were only
// declaration-guarded before. The compat `me` API reaches some of them
// (`copy`, `equals(other, options)`, `equalsViaSyntax` → `sameStructure`); the
// remaining structural criteria, the `{ok, why}` structure-only check, and the
// malformed-JSON error contract are only reachable at the raw wasm handle, so
// those cases go through `_wasm` directly. Reliable pass/fail examples mirror
// the Rust `tests/structural.rs` and `tests/equality.rs` matrices.
import me from "../lib/math-expressions";
import wasm from "../lib/_wasm";

describe("copy()", function () {
  it("produces an equal but independent expression", function () {
    const a = me.fromText("sin(x) + 2x^2");
    const b = a.copy();
    expect(b).not.toBe(a); // a fresh wrapper, not the same object
    expect(b.toString()).toEqual(a.toString());
    expect(JSON.stringify(b.tree)).toEqual(JSON.stringify(a.tree));
    expect(b.equals(a)).toBeTruthy();
  });

  it("the copy is fully functional", function () {
    const a = me.fromText("x^3");
    const b = a.copy();
    expect(b.derivative("x").toString()).toEqual(a.derivative("x").toString());
  });

  it("round-trips the tree at the raw boundary", function () {
    const h = wasm.parse_text("(x+1)(x-1)");
    expect(h.copy().tree_json()).toEqual(h.tree_json());
  });
});

describe("equals with options (equals_with_options)", function () {
  it("is exact by default and relaxes under a tolerance option", function () {
    expect(me.fromText("3.14").equals(me.fromText("pi"))).toBeFalsy();
    expect(
      me.fromText("3.14").equals(me.fromText("pi"), { allowed_error_in_numbers: 0.01 }),
    ).toBeTruthy();
  });

  it("carries the numeric option value across the boundary (same pair, opposite results)", function () {
    const a = me.fromText("2.0001*x");
    const b = me.fromText("2*x");
    expect(a.equals(b, { allowed_error_in_numbers: 1e-3 })).toBeTruthy();
    expect(a.equals(b, { allowed_error_in_numbers: 1e-6 })).toBeFalsy();
  });

  it("treats malformed options JSON as an error, not a silent default (raw boundary)", function () {
    const a = wasm.parse_text("3.14");
    const b = wasm.parse_text("pi");
    // A well-formed config is honored...
    expect(a.equals_with_options(b, JSON.stringify({ allowedErrorInNumbers: 0.01 }))).toBe(true);
    // ...but a typo'd config must throw, never grade with default tolerances.
    expect(() => a.equals_with_options(b, "{allowedErrorInNumbers:")).toThrow();
  });
});

describe("structural equality (structural_equality)", function () {
  it("equalsViaSyntax compares structure, not value", function () {
    // `ln(x)` folds to `log(x)` — same structure.
    expect(me.fromText("ln(x)").equalsViaSyntax(me.fromText("log(x)"))).toBeTruthy();
    // `x+y` and `y+x` are value-equal but not the same structure (order matters).
    expect(me.fromText("x+y").equalsViaSyntax(me.fromText("y+x"))).toBeFalsy();
  });

  it("carries the criterion JSON across the raw boundary", function () {
    const factored = wasm.parse_text("(x-1)(x+1)");
    const expanded = wasm.parse_text("x^2-1");
    // factored AND value-equal to the key → true under `factoredCompletely`
    expect(factored.structural_equality(expanded, JSON.stringify("factoredCompletely"))).toBe(true);
    // value-equal to the key but NOT factored → false on structure
    expect(expanded.structural_equality(expanded, JSON.stringify("factoredCompletely"))).toBe(false);
    // an unknown criterion is false, never silently true
    expect(factored.structural_equality(expanded, JSON.stringify("noSuchCriterion"))).toBe(false);
  });
});

describe("check_structural_comparison ({ok, why} JSON)", function () {
  const check = (src: string, comparison: string) =>
    JSON.parse(wasm.parse_text(src).check_structural_comparison(JSON.stringify(comparison)));

  it("recognizes the written form of an expression", function () {
    expect(check("1/2", "reducedFraction").ok).toBe(true);
    expect(check("2/4", "reducedFraction").ok).toBe(false);
    expect(check("0.5", "decimal").ok).toBe(true);
    expect(check("1/2", "decimal").ok).toBe(false);
    expect(check("(x-1)(x+1)", "factoredCompletely").ok).toBe(true);
    expect(check("x^2-1", "factoredCompletely").ok).toBe(false);
  });

  it("reports an unknown criterion as a structured result, not a throw", function () {
    const r = check("x+y", "noSuchCriterion");
    expect(r.ok).toBe(false);
    expect(r.why).toEqual("unknown structural comparison");
  });
});
