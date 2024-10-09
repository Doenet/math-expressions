import me from "../lib/math-expressions";

describe("round to significant digits", function () {
  it("single number", function () {
    let expr = me.fromText("1.234567890123456789");
    expect(expr.round_numbers_to_precision(100).tree).toEqual(
      1.2345678901234567,
    );
    expect(expr.round_numbers_to_precision(14).tree).toEqual(1.2345678901235);
    expect(expr.round_numbers_to_precision(10).tree).toEqual(1.23456789);
    expect(expr.round_numbers_to_precision(4).tree).toEqual(1.235);

    expr = me.fromText("12345678901234567890000000000");
    expect(expr.round_numbers_to_precision(100).tree).toEqual(
      12345678901234568000000000000,
    );
    expect(expr.round_numbers_to_precision(14).tree).toEqual(
      12345678901235000000000000000,
    );
    expect(expr.round_numbers_to_precision(10).tree).toEqual(
      12345678900000000000000000000,
    );
    expect(expr.round_numbers_to_precision(4).tree).toEqual(
      12350000000000000000000000000,
    );

    expr = me.fromText("0.000000000000000000001234567890123456789");
    expect(expr.round_numbers_to_precision(100).tree).toEqual(
      0.0000000000000000000012345678901234568,
    );
    expect(expr.round_numbers_to_precision(14).tree).toEqual(
      0.0000000000000000000012345678901235,
    );
    expect(expr.round_numbers_to_precision(10).tree).toEqual(
      0.00000000000000000000123456789,
    );
    expect(expr.round_numbers_to_precision(4).tree).toEqual(
      0.000000000000000000001235,
    );

    expect(me.fromAst(0).round_numbers_to_precision(10).tree).toEqual(0);
    expect(me.fromAst(Infinity).round_numbers_to_precision(10).tree).toEqual(
      Infinity,
    );
    expect(me.fromAst(-Infinity).round_numbers_to_precision(10).tree).toEqual(
      -Infinity,
    );
  });

  it("expression", function () {
    let expr = me.fromText(
      "exp(1.234567890123456789x+9.876543210987654321)/(5+8520203156.435345956432x)",
    );
    expect(
      expr
        .round_numbers_to_precision(100)
        .equals(
          me.fromText(
            "exp(1.2345678901234567 x + 9.876543210987654)/(5 + 8520203156.435346 x)",
          ),
        ),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_precision(14)
        .equals(
          me.fromText(
            "exp(1.2345678901235 x + 9.8765432109877)/(5 + 8520203156.4353 x)",
          ),
        ),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_precision(10)
        .equals(
          me.fromText("exp(1.23456789 x + 9.876543211)/(5 + 8520203156 x)"),
        ),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_precision(4)
        .equals(me.fromText("exp(1.235 x + 9.877)/(5 + 8520000000 x)")),
    ).toBeTruthy();
  });

  it("don't round fractions", function () {
    let expr = me.fromText("3/7x + 381439619649.253 y");
    expect(
      expr
        .round_numbers_to_precision(100)
        .equals(me.fromText("3/7x + 381439619649.253 y")),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_precision(14)
        .equals(me.fromText("3/7x + 381439619649.25 y")),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_precision(10)
        .equals(me.fromText("3/7x + 381439619600 y")),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_precision(4)
        .equals(me.fromText("3/7x + 381400000000 y")),
    ).toBeTruthy();
  });

  it("don't round pi or e", function () {
    let expr = me.fromText("3/7e + 381439619649.253 pi");
    expect(
      expr
        .round_numbers_to_precision(100)
        .equals(me.fromText("3/7exp(1) + 381439619649.253 pi")),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_precision(14)
        .equals(me.fromText("3/7exp(1) + 381439619649.25 pi")),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_precision(10)
        .equals(me.fromText("3/7exp(1) + 381439619600 pi")),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_precision(4)
        .equals(me.fromText("3/7exp(1) + 381400000000 pi")),
    ).toBeTruthy();
  });
});

describe("round to decimals", function () {
  it("single number", function () {
    let expr = me.fromText("1.234567890123456789");
    expect(expr.round_numbers_to_decimals(100).tree).toEqual(
      1.2345678901234567,
    );
    expect(expr.round_numbers_to_decimals(13).tree).toEqual(1.2345678901235);
    expect(expr.round_numbers_to_decimals(9).tree).toEqual(1.23456789);
    expect(expr.round_numbers_to_decimals(3).tree).toEqual(1.235);

    expr = me.fromText("12345678901234567890000000000");
    expect(expr.round_numbers_to_decimals(100).tree).toEqual(
      12345678901234568000000000000,
    );
    expect(expr.round_numbers_to_decimals(-15).tree).toEqual(
      12345678901235000000000000000,
    );
    expect(expr.round_numbers_to_decimals(-19).tree).toEqual(
      12345678900000000000000000000,
    );
    expect(expr.round_numbers_to_decimals(-25).tree).toEqual(
      12350000000000000000000000000,
    );

    expr = me.fromText("0.000000000000000000001234567890123456789");
    expect(expr.round_numbers_to_decimals(100).tree).toEqual(
      0.0000000000000000000012345678901234568,
    );
    expect(expr.round_numbers_to_decimals(34).tree).toEqual(
      0.0000000000000000000012345678901235,
    );
    expect(expr.round_numbers_to_decimals(30).tree).toEqual(
      0.00000000000000000000123456789,
    );
    expect(expr.round_numbers_to_decimals(24).tree).toEqual(
      0.000000000000000000001235,
    );

    expect(me.fromAst(0).round_numbers_to_decimals(10).tree).toEqual(0);
    expect(me.fromAst(Infinity).round_numbers_to_decimals(10).tree).toEqual(
      Infinity,
    );
    expect(me.fromAst(-Infinity).round_numbers_to_decimals(10).tree).toEqual(
      -Infinity,
    );
  });

  it("expression", function () {
    let expr = me.fromText(
      "exp(1.234567890123456789x+9876.543210987654321)/(5+8520203156.435345956432x)",
    );
    expect(
      expr
        .round_numbers_to_decimals(100)
        .equals(
          me.fromText(
            "exp(1.2345678901234567 x + 9876.543210987654)/(5 + 8520203156.435346 x)",
          ),
        ),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_decimals(13)
        .equals(
          me.fromText(
            "exp(1.2345678901235 x + 9876.543210987654)/(5 + 8520203156.435346 x)",
          ),
        ),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_decimals(9)
        .equals(
          me.fromText(
            "exp(1.23456789 x + 9876.543210988)/(5 + 8520203156.435346 x)",
          ),
        ),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_decimals(-2)
        .equals(me.fromText("exp(0 x + 9900)/(0 + 8520203200 x)")),
    ).toBeTruthy();
  });

  it("don't round fractions", function () {
    let expr = me.fromText("3123414232/72512351634x + 381439619649.253 y");
    expect(
      expr
        .round_numbers_to_decimals(100)
        .equals(me.fromText("3123414232/72512351634x + 381439619649.253 y")),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_decimals(2)
        .equals(me.fromText("3123414232/72512351634x + 381439619649.25 y")),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_decimals(-2)
        .equals(me.fromText("3123414200/72512351600x + 381439619600 y")),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_decimals(-8)
        .equals(me.fromText("3100000000/72500000000x + 381400000000 y")),
    ).toBeTruthy();
  });

  it("don't round pi or e", function () {
    let expr = me.fromText("3123414232/72512351634e + 381439619649.253 pi");
    expect(
      expr
        .round_numbers_to_decimals(100)
        .equals(
          me.fromText("3123414232/72512351634exp(1) + 381439619649.253 pi"),
        ),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_decimals(2)
        .equals(
          me.fromText("3123414232/72512351634exp(1) + 381439619649.25 pi"),
        ),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_decimals(-2)
        .equals(me.fromText("3123414200/72512351600exp(1) + 381439619600 pi")),
    ).toBeTruthy();
    expect(
      expr
        .round_numbers_to_decimals(-8)
        .equals(me.fromText("3100000000/72500000000exp(1) + 381400000000 pi")),
    ).toBeTruthy();
  });

  describe("round to significant digits plus decimals", function () {
    it("single number", function () {
      let expr = me.fromText("1.234567890123456789");
      expect(
        expr.round_numbers_to_precision_plus_decimals(100, 6).tree,
      ).toEqual(1.2345678901234567);
      expect(expr.round_numbers_to_precision_plus_decimals(14, 6).tree).toEqual(
        1.2345678901235,
      );
      expect(expr.round_numbers_to_precision_plus_decimals(10, 6).tree).toEqual(
        1.23456789,
      );
      expect(expr.round_numbers_to_precision_plus_decimals(4, 6).tree).toEqual(
        1.234568,
      );

      expr = me.fromText("12345678901234567890000000000");
      expect(
        expr.round_numbers_to_precision_plus_decimals(100, -17).tree,
      ).toEqual(12345678901234568000000000000);
      expect(
        expr.round_numbers_to_precision_plus_decimals(14, -17).tree,
      ).toEqual(12345678901235000000000000000);
      expect(
        expr.round_numbers_to_precision_plus_decimals(10, -17).tree,
      ).toEqual(12345678901200000000000000000);
      expect(
        expr.round_numbers_to_precision_plus_decimals(4, -17).tree,
      ).toEqual(12345678901200000000000000000);

      expr = me.fromText("0.000000000000000000001234567890123456789");
      expect(
        expr.round_numbers_to_precision_plus_decimals(100, 27).tree,
      ).toEqual(0.0000000000000000000012345678901234568);
      expect(
        expr.round_numbers_to_precision_plus_decimals(14, 27).tree,
      ).toEqual(0.0000000000000000000012345678901235);
      expect(
        expr.round_numbers_to_precision_plus_decimals(10, 27).tree,
      ).toEqual(0.00000000000000000000123456789);
      expect(expr.round_numbers_to_precision_plus_decimals(4, 27).tree).toEqual(
        0.000000000000000000001234568,
      );

      expect(
        me.fromAst(0).round_numbers_to_precision_plus_decimals(10, 2).tree,
      ).toEqual(0);
      expect(
        me.fromAst(Infinity).round_numbers_to_precision_plus_decimals(10, 2)
          .tree,
      ).toEqual(Infinity);
      expect(
        me.fromAst(-Infinity).round_numbers_to_precision_plus_decimals(10, 2)
          .tree,
      ).toEqual(-Infinity);
    });

    it("fall back to digits or decimals", function () {
      let expr = me.fromText("123456789.0123456789");
      expect(
        expr.round_numbers_to_precision_plus_decimals(4, "bad").tree,
      ).toEqual(123500000);
      expect(
        expr.round_numbers_to_precision_plus_decimals(4, -Infinity).tree,
      ).toEqual(123500000);
      expect(
        expr.round_numbers_to_precision_plus_decimals("bad", -8).tree,
      ).toEqual(100000000);
      expect(expr.round_numbers_to_precision_plus_decimals(0, -8).tree).toEqual(
        100000000,
      );
      expect(
        expr.round_numbers_to_precision_plus_decimals("bad", -9).tree,
      ).toEqual(0);
      expect(expr.round_numbers_to_precision_plus_decimals(0, -9).tree).toEqual(
        0,
      );

      expr = me.fromText("0.00001234567890123456789");
      expect(
        expr.round_numbers_to_precision_plus_decimals("bad", 5).tree,
      ).toEqual(0.00001);
      expect(expr.round_numbers_to_precision_plus_decimals(0, 5).tree).toEqual(
        0.00001,
      );
      expect(
        expr.round_numbers_to_precision_plus_decimals("bad", 4).tree,
      ).toEqual(0);
      expect(expr.round_numbers_to_precision_plus_decimals(0, 4).tree).toEqual(
        0,
      );
      expect(expr.round_numbers_to_precision_plus_decimals(1, 4).tree).toEqual(
        0.00001,
      );
    });

    it("expression", function () {
      let expr = me.fromText(
        "exp(1.234567890123456789x+9.876543210987654321)/(5+8520203156.435345956432x)",
      );
      expect(
        expr
          .round_numbers_to_precision_plus_decimals(100, 5)
          .equals(
            me.fromText(
              "exp(1.2345678901234567 x + 9.876543210987654)/(5 + 8520203156.435346 x)",
            ),
          ),
      ).toBeTruthy();
      expect(
        expr
          .round_numbers_to_precision_plus_decimals(14, 5)
          .equals(
            me.fromText(
              "exp(1.2345678901235 x + 9.8765432109877)/(5 + 8520203156.43535 x)",
            ),
          ),
      ).toBeTruthy();
      expect(
        expr
          .round_numbers_to_precision_plus_decimals(10, 5)
          .equals(
            me.fromText(
              "exp(1.23456789 x + 9.876543211)/(5 + 8520203156.43535 x)",
            ),
          ),
      ).toBeTruthy();
      expect(
        expr
          .round_numbers_to_precision_plus_decimals(4, 5)
          .equals(
            me.fromText("exp(1.23457 x + 9.87654)/(5 + 8520203156.43535 x)"),
          ),
      ).toBeTruthy();
    });

    it("don't round fractions", function () {
      let expr = me.fromText("3/7x + 381439619649.253 y");
      expect(
        expr
          .round_numbers_to_precision_plus_decimals(100, -2)
          .equals(me.fromText("3/7x + 381439619649.253 y")),
      ).toBeTruthy();
      expect(
        expr
          .round_numbers_to_precision_plus_decimals(14, -2)
          .equals(me.fromText("3/7x + 381439619649.25 y")),
      ).toBeTruthy();
      expect(
        expr
          .round_numbers_to_precision_plus_decimals(10, -2)
          .equals(me.fromText("3/7x + 381439619600 y")),
      ).toBeTruthy();
      expect(
        expr
          .round_numbers_to_precision_plus_decimals(4, -2)
          .equals(me.fromText("3/7x + 381439619600 y")),
      ).toBeTruthy();
    });

    it("don't round pi or e", function () {
      let expr = me.fromText("3/7e + 381439619649.253 pi");
      expect(
        expr
          .round_numbers_to_precision_plus_decimals(100, 1)
          .equals(me.fromText("3/7exp(1) + 381439619649.253 pi")),
      ).toBeTruthy();
      expect(
        expr
          .round_numbers_to_precision_plus_decimals(14, 1)
          .equals(me.fromText("3/7exp(1) + 381439619649.25 pi")),
      ).toBeTruthy();
      expect(
        expr
          .round_numbers_to_precision_plus_decimals(10, 1)
          .equals(me.fromText("3/7exp(1) + 381439619649.3 pi")),
      ).toBeTruthy();
      expect(
        expr
          .round_numbers_to_precision_plus_decimals(4, 1)
          .equals(me.fromText("3/7exp(1) + 381439619649.3 pi")),
      ).toBeTruthy();
    });
  });
});
