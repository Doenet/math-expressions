import me from '../lib/math-expressions';

describe("discrete infinite", function () {

  test("basic equality", function () {
    
    var set1 = me.create_discrete_infinite_set(
      { offsets: me.fromText("pi/4"), periods: me.fromText("pi")});
    var set2 = me.create_discrete_infinite_set(
      { offsets: me.fromText("pi/4, 5pi/4"), periods: me.fromText("2pi")});
    expect(set1.equals(set2)).toBeTruthy();
    
    set1 = me.create_discrete_infinite_set(
      { offsets: me.fromText("pi/4"), periods: me.fromText("2*pi")});
    expect(set1.equals(set2)).toBeFalsy();
    
    set1 = me.create_discrete_infinite_set(
      { offsets: me.fromText("9*pi/4"), periods: me.fromText("pi")});
    expect(set1.equals(set2)).toBeTruthy();

    set1 = me.create_discrete_infinite_set(
      { offsets: me.fromText("7*pi/4"), periods: me.fromText("pi")});
    expect(set1.equals(set2)).toBeFalsy();


  });


  test("overcounting", function () {

    var set1 = me.create_discrete_infinite_set(
      { offsets: me.fromText("1"), periods: me.fromText("5")});
    var set2 = me.create_discrete_infinite_set(
      { offsets: me.fromText("1, 1, 6, 11, 16, 21"), periods: me.fromText("10")});
    expect(set1.equals(set2)).toBeTruthy();

    set2 = me.create_discrete_infinite_set(
      { offsets: me.fromText("1, 1, 6, 11, 16, 22"), periods: me.fromText("10")});
    expect(set1.equals(set2)).toBeFalsy();
  });
  
  test("variables", function () {

    var set1 = me.create_discrete_infinite_set(
      { offsets: me.fromText("a"), periods: me.fromText("3")});
    var set2 = me.create_discrete_infinite_set(
      { offsets: me.fromText("a, a+3, a+6"), periods: me.fromText("9")});
    expect(set1.equals(set2)).toBeTruthy();

    set1 = me.create_discrete_infinite_set(
      { offsets: me.fromText("b"), periods: me.fromText("3")});
    expect(set1.equals(set2)).toBeFalsy();
    
  });

  test("assumptions", function () {

    me.clear_assumptions();
    var set1 = me.create_discrete_infinite_set(
      { offsets: me.fromText("a"), periods: me.fromText("c")});
    var set2 = me.create_discrete_infinite_set(
      { offsets: me.fromText("a, a+c"), periods: me.fromText("2c")});

    expect(set1.equals(set2)).toBeFalsy();
    
    me.add_assumption(me.from("c != 0"));
    expect(set1.equals(set2)).toBeTruthy();
    
    me.clear_assumptions();

  });

  test("compare with list", function () {
    var set = me.create_discrete_infinite_set(
      { offsets: me.fromText("0"), periods: me.fromText("7"),
	min_index: me.fromText("0")});
    var list1 = me.fromText("0, 7, 14, 21, ...");
    var list2 = me.fromText("-14, -7, 0, 7, 14, 21, ...");
    var list3 = me.fromText("0, 7, 14, 21");
    var list4 = me.fromText("0, 7, ...");

    expect(set.equals(list1)).toBeTruthy();
    expect(set.equals(list2)).toBeFalsy();
    expect(set.equals(list3)).toBeFalsy();
    expect(set.equals(list4)).toBeFalsy();

    set = me.create_discrete_infinite_set(
      { offsets: me.fromText("0"), periods: me.fromText("7"),
	min_index: me.fromText("-2")});

    expect(set.equals(list1)).toBeFalsy();
    expect(set.equals(list2)).toBeTruthy();
    expect(set.equals(list3)).toBeFalsy();
    expect(set.equals(list4)).toBeFalsy();
    

  });
  
});
