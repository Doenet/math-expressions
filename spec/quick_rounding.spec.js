import me from '../lib/math-expressions';

describe("round to significant digits", function () {

  it("single number", function () {

    let expr = me.fromText('1.234567890123456789');
    expect(expr.round_numbers_to_precision(100).tree).toEqual(1.2345678901234567)
    expect(expr.round_numbers_to_precision(14).tree).toEqual(1.2345678901235)
    expect(expr.round_numbers_to_precision(10).tree).toEqual(1.23456789)
    expect(expr.round_numbers_to_precision(4).tree).toEqual(1.235)

    expr = me.fromText('12345678901234567890000000000');
    expect(expr.round_numbers_to_precision(100).tree).toEqual(12345678901234568000000000000)
    expect(expr.round_numbers_to_precision(14).tree).toEqual(12345678901235000000000000000)
    expect(expr.round_numbers_to_precision(10).tree).toEqual(12345678900000000000000000000)
    expect(expr.round_numbers_to_precision(4).tree).toEqual(12350000000000000000000000000)

    expr = me.fromText('0.000000000000000000001234567890123456789');
    expect(expr.round_numbers_to_precision(100).tree).toEqual(0.0000000000000000000012345678901234568)
    expect(expr.round_numbers_to_precision(14).tree).toEqual(0.0000000000000000000012345678901235)
    expect(expr.round_numbers_to_precision(10).tree).toEqual(0.00000000000000000000123456789)
    expect(expr.round_numbers_to_precision(4).tree).toEqual(0.000000000000000000001235)

  });

  it("expression", function () {

    let expr = me.fromText('exp(1.234567890123456789x+9.876543210987654321)/(5+8520203156.435345956432x)');
    expect(expr.round_numbers_to_precision(100).equals(
      me.fromText('exp(1.2345678901234567 x + 9.876543210987654)/(5 + 8520203156.435346 x)'))).toBeTruthy();
    expect(expr.round_numbers_to_precision(14).equals(
      me.fromText('exp(1.2345678901235 x + 9.8765432109877)/(5 + 8520203156.4353 x)'))).toBeTruthy();
    expect(expr.round_numbers_to_precision(10).equals(
      me.fromText('exp(1.23456789 x + 9.876543211)/(5 + 8520203156 x)'))).toBeTruthy();
    expect(expr.round_numbers_to_precision(4).equals(
      me.fromText('exp(1.235 x + 9.877)/(5 + 8520000000 x)'))).toBeTruthy();

  });

  it("don't round fractions", function () {

    let expr = me.fromText('3/7x + 381439619649.253 y');
    expect(expr.round_numbers_to_precision(100).equals(
      me.fromText('3/7x + 381439619649.253 y'))).toBeTruthy();
    expect(expr.round_numbers_to_precision(14).equals(
      me.fromText('3/7x + 381439619649.25 y'))).toBeTruthy();
    expect(expr.round_numbers_to_precision(10).equals(
      me.fromText('3/7x + 381439619600 y'))).toBeTruthy();
    expect(expr.round_numbers_to_precision(4).equals(
      me.fromText('3/7x + 381400000000 y'))).toBeTruthy();

  });


  it("don't round pi or e", function () {

    let expr = me.fromText('3/7e + 381439619649.253 pi');
    expect(expr.round_numbers_to_precision(100).equals(
      me.fromText('3/7exp(1) + 381439619649.253 pi'))).toBeTruthy();
    expect(expr.round_numbers_to_precision(14).equals(
      me.fromText('3/7exp(1) + 381439619649.25 pi'))).toBeTruthy();
    expect(expr.round_numbers_to_precision(10).equals(
      me.fromText('3/7exp(1) + 381439619600 pi'))).toBeTruthy();
    expect(expr.round_numbers_to_precision(4).equals(
      me.fromText('3/7exp(1) + 381400000000 pi'))).toBeTruthy();

  });


});

