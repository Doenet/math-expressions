var me = require('../lib/math-expressions');
var trees = require('../lib/trees/basic');
var poly = require('../lib/polynomial/polynomial');
var simplify = require('../lib/expression/simplify');

describe("reduce rational expression", function () {
         var poly_sets = [
                          [['x', 'x'], ['1', '1']],
                          [['5x', '5x'], ['1', '1']],
                          [['5x', '5y'], ['x', 'y']],
                          [['2x+x', 'x'], ['3', '1']],
                          [['2xy+y^2', 'yx^2'], ['2x+y', 'x^2']],
                          [['(1+y)(x+z)', '(1+y)(x+x^2)'], ['x+z', 'x+x^2']],
                          [['(x^2+x+y)(z+t)', '(u^4+x)(2z+2t)'], ['0.5x^2+0.5x+0.5y', 'u^4+x']],
                          [['(xy+zy)(t+v)', '(t+v)(x^5y)'], ['x+z', 'x^5']],
                          [['x sin(x)-x', 'x'], ['sin(x)-1', '1']],
                          [['xs-ys', 'x^2s-z^2s'], ['x-y', 'x^2-z^2']],
                          [['x sin(x)-y sin(x)', 'x^2sin(x)-z^2sin(x)'], ['x-y', 'x^2-z^2']],
                          [['(sin(x))^2', 'sin(x)'], ['sin(x)', '1']],
                          [['sin(x)cos(y)', 'cos(y)sin(y)'], ['sin(x)', 'sin(y)']],
                          [['t^100', 't'], ['t^99', '1']],
                          [['t^8-t', 't'], ['t^7-1', '1']],
                          [['t^1000000-t', 't'], ['t^999999-1', '1']],
                          [['(a+b)(c+d)','(a+b)(e+f)'], ['c+d', 'e+f']],
                          [['(ac+ad+bc+bd)','(ae+af+be+bf)'], ['(c + d)','(e + f)']],
                          [['(a+sin(x))(c+cos(y))','((a+sin(x))(e+exp(z)))'], ['(c + cos(y))','(e + exp(z))']]
                          ]
         
         poly_sets.forEach(function(example) {
                           it(example, function() {
                              let top = poly.expression_to_polynomial(me.fromText(example[0][0]));
                              let bottom = poly.expression_to_polynomial(me.fromText(example[0][1]));
                              let new_top = poly.expression_to_polynomial(me.fromText(example[1][0]));
                              let new_bottom = poly.expression_to_polynomial(me.fromText(example[1][1]));
                              expect(poly.reduce_rational_expression(top,bottom)).toEqual([new_top, new_bottom]);
                              });
                           });
         });

describe("gcd", function () {
         var poly_sets = [
                          [['x', 'x'], 'x']
                          ]
         
         poly_sets.forEach(function(example) {
                           it(example, function() {
                              let top = poly.expression_to_polynomial(me.fromText(example[0][0]));
                              let bottom = poly.expression_to_polynomial(me.fromText(example[0][1]));
                              let gcd = poly.expression_to_polynomial(me.fromText(example[1]));
                              expect(poly.poly_gcd(top,bottom)).toEqual(gcd);
                              });
                           });
         });

describe("gcd", function () {
         var poly_sets = [
                          [5, ['/', 1, 2], 1],
                          [["polynomial", "x", [[1,1]]], 1, 1],
                          [["polynomial", "x", [[1,5]]], -1, 1],
                          [["polynomial", "x", [[1,1]]], ["polynomial", "x", [[2,1]]], ["polynomial", "x", [[1,1]]]],
                          [["polynomial", "x", [[1,1]]], ["polynomial", "y", [[1,1]]], 1],
                          [["polynomial", "x", [[1,["polynomial", "y", [[2,7]]]]]], ["polynomial", "x", [[2,["polynomial", "y", [[1,['/',1,2]]]]]]], ["polynomial", "x", [[1,["polynomial", "y", [[1,1]]]]]]],
                          [["polynomial", "x", [[0,1], [1,1]]], ["polynomial", "y", [[1,1]]], 1],
                          [["polynomial", "x", [[0,["polynomial", "y", [[2,1]]]], [1,["polynomial", "y", [[1,1]]]]]], ["polynomial", "y", [[1,1], [2,1]]], ["polynomial", "y", [[1,1]]]]
                          ]
         
         poly_sets.forEach(function(example) {
                           it(example, function() {
                              expect(poly.poly_gcd(example[0],example[1])).toEqual(example[2]);
                              });
                           });
         });

describe("lcm", function () {
         var poly_sets = [
                          [5, ['/', 1, 2], 1],
                          [["polynomial", "x", [[1,1]]], 1, ["polynomial", "x", [[1,1]]]],
                          [["polynomial", "x", [[1,5]]], -1, ["polynomial", "x", [[1,1]]]],
                          [["polynomial", "x", [[1,1]]], ["polynomial", "x", [[2,1]]], ["polynomial", "x", [[2,1]]]],
                          [["polynomial", "x", [[1,1]]], ["polynomial", "y", [[1,1]]], ["polynomial", "x", [[1,["polynomial", "y", [[1,1]]]]]]],
                          [["polynomial", "x", [[1,["polynomial", "y", [[2,7]]]]]], ["polynomial", "x", [[2,["polynomial", "y", [[1,['/',1,2]]]]]]], ["polynomial", "x", [[2,["polynomial", "y", [[2,1]]]]]]],
                          [["polynomial", "x", [[0,1], [1,1]]], ["polynomial", "y", [[1,1]]], ["polynomial", "x", [[0,["polynomial", "y", [[1,1]]]], [1,["polynomial", "y", [[1,1]]]]]]],
                          [["polynomial", "x", [[0,["polynomial", "y", [[2,1]]]], [1,["polynomial", "y", [[1,1]]]]]], ["polynomial", "y", [[1,1], [2,1]]], ["polynomial", "x", [[0,["polynomial", "y", [[2,1], [3,1]]]], [1,["polynomial", "y", [[1,1], [2,1]]]]]]]
         ]
         
         poly_sets.forEach(function(example) {
                           it(example, function() {
                              expect(poly.poly_lcm(example[0],example[1])).toEqual(example[2]);
                              });
                           });
         });

describe("grobner", function () {
         var poly_sets = [
                          [[0], [0]],
                          [[0,5], [1]],
                          [[["polynomial", "x", [[1,1]]], 1], [1]],
                          [[["polynomial", "x", [[1,2]]], 1], [1]],
                          [[["polynomial", "x", [[1,1]]], ["polynomial", "x", [[2,1]]]], [["polynomial", "x", [[1,1]]]]],
                          [[["polynomial", "x", [[1,1]]], ["polynomial", "x", [[0,1], [2,1]]]], [1]],
                           [[["polynomial", "x", [[1,1]]], ["polynomial", "x", [[2,["polynomial", "y", [[2,1]]]]]]], [["polynomial", "x", [[1,1]]]]],
                          [[["polynomial", "x", [[1,2]]], ["polynomial", "x", [[2,["polynomial", "y", [[2,7]]]]]]], [["polynomial", "x", [[1,1]]]]]
                          ]
         
         poly_sets.forEach(function(red) {
                           it(red, function() {
                              expect(poly.reduced_grobner(red[0])).toEqual(red[1]);
                              });
                           });
         });

describe("reduce", function () {
         var poly_sets = [
                          [[0], [0]],
                          [[["polynomial", "x", [[1,1]]]], [["polynomial", "x", [[1,1]]]]],
                          [[["polynomial", "x", [[1,1]]], ["polynomial", "y", [[1,1]]]], [["polynomial", "x", [[1,1]]], ["polynomial", "y", [[1,1]]]]],
                          [[["polynomial", "x", [[1,1]]], ["polynomial", "x", [[2,1]]],  ["polynomial", "y", [[1,1]]]], [["polynomial", "x", [[1,1]]], ["polynomial", "y", [[1,1]]]]],
                           [[["polynomial", "x", [[1,1]]], ["polynomial", "x", [[0,1], [2,1]]],  ["polynomial", "y", [[1,1]]]], [1]],
                          [[["polynomial", "x", [[1,1]]], ["polynomial", "x", [[2,["polynomial", "y", [[2,1]]]]]]], [["polynomial", "x", [[1,1]]]]]
         ]
         
         poly_sets.forEach(function(red) {
                           it(red, function() {
                              expect(poly.reduce(red[0])).toEqual(red[1]);
                              });
                           });
         });

describe("division algorithm", function () {
         var divisions = [
                          [["polynomial", "x", [[1, 1]]], [["polynomial", "x", [[1, 1]]]], [[[0, 1]], 0]],
                          [["polynomial", "x", [[0, 2], [1, 5], [3, 1]]], [["polynomial", "x", [[1,1]]]], [[[0, ["monomial", 1, [["x", 2]]]], [0,5]], 2]],
                          [["polynomial", "x", [[0, 2], [1, 5], [2, 5]]], [["polynomial", "x", [[0,1], [1,1]]]], [[[0, ["monomial", 5, [["x", 1]]]]], 2]],
                          [["polynomial", "x", [[0, 2], [1, 5], [3, 1]]], [["polynomial", "x", [[0,-1], [1,1]]]], [[[0, ["monomial", 1, [["x", 2]]]], [0, ["monomial", 1, [["x", 1]]]], [0,6]], 8]],
                          [["polynomial", "x", [[1, 1]]], [["polynomial", "x", [[2, 1]]]], [[], ["polynomial", "x", [[1, 1]]]]],
                          [["polynomial", "x", [[1, 1]]], [["polynomial", "y", [[1, 1]]]], [[], ["polynomial", "x", [[1, 1]]]]],
                          [["polynomial", "x", [[0, 1], [1, 1], [2, 1]]], [["polynomial", "x", [[2, 1]]], ["polynomial", "x", [[1, 1]]]], [[[0, 1], [1, 1]], 1]],
                          [["polynomial", "x", [[0, 1], [1, ["polynomial", "y", [[1,1]]]], [2, 1]]], [["polynomial", "y", [[1,1]]]], [[[0, ["monomial", 1, [["x", 1]]]]], ["polynomial", "x", [[0,1], [2,1]]]]],
                          [1, [["polynomial", "x", [[1, 1]]]], [[], 1]],
                          [["polynomial", "x", [[1, 1]]], [1], [[[0, ["monomial", 1, [["x", 1]]]]], 0]]
                          ]
         
         
         divisions.forEach(function(divs) {
                           it(divs, function() {
                              expect(poly.poly_div(divs[0], divs[1])).toEqual(divs[2]);
                              });
                           });
         
         });

describe("find maximum term divisible by one of the monomials", function () {
         var poly_monos = [
                           [["polynomial", "x", [[1, 1]]], [["monomial", "1", [["x",2]]]],
                            0],
                           [["polynomial", "x", [[1,1]]], [["monomial", 1, [["x",1]]]], [["monomial", 1, [["x",1]]], 0]],
                           [["polynomial", "x", [[0, 1], [2, 3]]], [["monomial", 1, [["x", 2]]]], [["monomial", 3, [["x", 2]]], 0]],
                           [["polynomial", "x", [[1, 5], [3, 2]]], [["monomial", 1, [["x", 4]]]], 0],
                           [["polynomial", "x", [[1, 5], [3, 2]]], [["monomial", 1, [["x", 4]]], ["monomial", 1, [["x", 1]]]], [["monomial", 2, [["x", 3]]], 1]],
                           [["polynomial", "x", [[1, ["polynomial", "y", [[1, 1]]]]]], [["monomial", 4, [["x", 1], ["y", 1]]]], [["monomial", 1, [["x", 1], ["y", 1]]], 0]],
                           [["polynomial", "x", [[0, 1],[1, ["polynomial", "y", [[1, 1], [2, 1]]]], [2, ["polynomial", "y", [[1,1]]]], [3, 1]]], [["monomial", 1, [["y", 1]]]], [["monomial", 1, [["x", 2], ["y", 1]]], 0]],
                           [["polynomial", "x", [[0, 1], [1, ["polynomial", "y", [[1, 1], [2, 1]]]], [2, ["polynomial", "y", [[1,1]]]], [3, 1]]], [["monomial", 1, [["x", 1]]]], [["monomial", 1, [["x", 3]]], 0]],
                           [["polynomial", "x", [[0, 1], [1, ["polynomial", "y", [[1, 1], [2, 1]]]], [2, ["polynomial", "y", [[1,1]]]], [3, 1]]], [["monomial", 1, [["y", 2]]]], [["monomial", 1, [["x", 1], ["y", 2]]], 0]],
                           [["polynomial", "x", [[0, 1], [1, ["polynomial", "y", [[1, 1], [2, 1]]]], [2, ["polynomial", "y", [[1,1]]]], [3, 1]]], [["monomial", 1, [["x", 1], ["y", 1]]]], [["monomial", 1, [["x", 2], ["y", 1]]], 0]],
                           [["polynomial", "x", [[0, 1], [1, ["polynomial", "y", [[1, 1], [2, 1]]]], [2, ["polynomial", "y", [[1,1]]], [3, 1]]]], [["monomial", 1, [["x", 2], ["y", 2]]]], 0],
                           [["polynomial", "x", [[0, 1], [1, ["polynomial", "y", [[1, 1], [2, 1]]]], [2, ["polynomial", "y", [[1,1]]]], [3, 1]]], [["monomial", 1, [["y", 1]]], ["monomial", 1, [["x", 1]]]], [["monomial", 1, [["x", 3]]], 1]],
                           [["polynomial", "x", [[0, 1], [1, ["polynomial", "y", [[1, 1], [2, 1]]]], [2, ["polynomial", "y", [[1,1]]]], [3, 1]]], [["monomial", 1, [["y", 2]]], ["monomial", 1, [["x", 1], ["y", 1]]]], [["monomial", 1, [["x", 2], ["y", 1]]], 1]],
                           [["polynomial", "x", [[0, 1], [1, ["polynomial", "y", [[1, 1], [2, 1]]]], [2, ["polynomial", "y", [[1,1]]]], [3, 1]]], [["monomial", 1, [["x", 4]]], ["monomial", 1, [["x", 2], ["y", 2]]], ["monomial", 1, [["x", 1], ["y", 1]]]], [["monomial", 1, [["x", 2], ["y", 1]]], 2]]
                          ];
         poly_monos.forEach(function(monos) {
                           it(monos, function() {
                              expect(poly.max_div_init(monos[0], monos[1])).toEqual(monos[2]);
                              });
                           });
         });

describe("monomial to polynomial", function () {
         var mono_poly = [
                          [["monomial", 6, [["y",1]]], ["polynomial", "y", [[1, 6]]]],
                          [["monomial", 5, [["x", 3],["y", 2]]], ["polynomial", "x", [[3, ["polynomial", "y", [[2, 5]]]]]]],
                          [["monomial", 1, [["x", 2],["y", 3],["z", 5]]], ["polynomial", "x", [[2, ["polynomial", "y", [[3, ["polynomial", "z", [[5, 1]]]]]]]]]],
                          [7, 7]
                              ];
         mono_poly.forEach(function(monos) {
                               it(monos, function() {
                                  expect(poly.mono_to_poly(monos[0])).toEqual(monos[1]);
                                  });
                               });
         });

describe("monomial division", function () {
         var mono_mono_div = [
                              [["monomial", 1, [["x",1]]], ["monomial", 1 ,[["x",1]]], 1],
                              [["monomial", 1, [["y",1]]], 1, ["monomial", 1, [["y",1]]]],
                              [["monomial", 5, [["x", 3],["y", 2]]], ["monomial", 1, [["x", 1],["y", 1]]], ["monomial", 5, [["x", 2],["y", 1]]]],
                              [["monomial", 1, [["x", 2],["y", 3],["z", 5]]], ["monomial", 1, [["x", 1],["z", 4]]], ["monomial", 1, [["x", 1],["y", 3],["z", 1]]]],
                              [["monomial", 1, [["x", 2],["y", 3],["z", 4]]], ["monomial", 1, [["x", 1],["z", 4]]], ["monomial", 1, [["x", 1],["y", 3]]]],
                              [7, 1, 7],
                              [7, 2, 3.5],
                              [["monomial", 1, [["y",1]]], 3, ["monomial", ['/', 1, 3], [["y",1]]]],
                              [["monomial", 5, [["x", 3],["y", 2]]], ["monomial", 1, [["x", 3],["y", 2]]], 5],
                              [["monomial", 5, [["x", 3],["y", 2]]], ["monomial", 2, [["x", 3],["y", 2]]], 2.5],
                              [["monomial", 5, [["x", 3],["y", 2]]], ["monomial", 2, [["x", 1],["y", 1]]], ["monomial", 2.5, [["x", 2],["y", 1]]]]
                              ];
         mono_mono_div.forEach(function(monos) {
                                            it(monos, function() {
                                               expect(poly.mono_div(monos[0],monos[1])).toEqual(monos[2]);
                                               expect(poly.mono_is_div(monos[0],monos[1])).toBeTruthy();
                                               expect(poly.mono_is_div(monos[0],monos[2])).toBeTruthy();
                                               });
                                            });
         var mono_nondiv = [
                              [1, ["monomial", 1, [["y",1]]]],
                              [["monomial", 1, [["x", 1],["y", 1]]], ["monomial", 5, [["x", 2],["y", 1]]]],
                              [["monomial", 1, [["x", 1],["z", 4]]], ["monomial", 1, [["x", 1],["y", 3],["z", 4]]]],
                            [["monomial", 1, [["x", 3]]], ["monomial", 1, [["y", 1]]]],
                            [["monomial", 1, [["x", 3]]], ["monomial", 1, [["y", 2]]]],
                            [["monomial", 1, [["x", 3]]], ["monomial", 1, [["x", 1], ["y", 1]]]],
                            [["monomial", 1, [["x", 3]]], ["monomial", 1, [["x", 2], ["y", 2]]]]
                            ];
         mono_nondiv.forEach(function(monos) {
                               it(monos, function() {
                                  expect(poly.mono_is_div(monos[0],monos[1])).toBeFalsy();
                                  });
                               });
         });

describe("monomial gcd", function () {
         var mono_mono_gcd = [
                              [["monomial", 1, [["y",1]]], 7, 1],
                              [["monomial", 1, [["x",1]]], ["monomial", 1, [["y",1]]], 1],
                              [["monomial", 1, [["y",1]]], ["monomial", 1, [["y",1]]], ["monomial", 1, [["y",1]]]],
                              [["monomial", 1, [["x", 2],["y", 2]]], ["monomial", 1, [["x", 3],["y", 1]]], ["monomial", 1, [["x", 2],["y", 1]]]],
                              [["monomial", 1, [["x", 2],["y", 3],["z", 4]]], ["monomial", 1, [["a", 5],["z", 5]]], ["monomial", 1, [["z", 4]]]],
                              [["monomial", 1, [["x", 2],["y", 5]]], ["monomial", 1, [["a", 5], ["b", 2], ["c", 7]]], 1]
                              ];
         mono_mono_gcd.forEach(function(monos) {
                                            it(monos, function() {
                                               expect(poly.mono_gcd(monos[0],monos[1])).toEqual(monos[2]);
                                               expect(poly.mono_gcd(monos[1],monos[0])).toEqual(monos[2]);
                                               });
                                            });
         });

describe("monomial order", function () {
    var inc_mono_pairs = [
        [["monomial", 1, [["y",1]]], ["monomial", 2, [["x",1]]]],
        [["monomial", 2, [["x",1]]], ["monomial", 1, [["x",2]]]],
        [["monomial", 1, [["x",2],["y",2]]], ["monomial", 1, [["x",3],["y",1]]]],
        [["monomial", 1, [["x",2],["y",2]]], ["monomial", 1, [["x",2],["y",3]]]],
        [["monomial", 1, [["x",2]]], ["monomial", 1, [["x",2],["y",1]]]],
        [["monomial", 1, [["x",2],["y",2],["z",5]]], ["monomial", 1, [["x",3],["y",1]]]],
                          ];
    
    inc_mono_pairs.forEach(function(pair) {
        it(pair, function() {
           expect(poly.mono_less_than( pair[0], pair[1] )).toBeTruthy();
           expect(poly.mono_less_than( pair[1], pair[0] )).toBeFalsy();
                             });
                           });
         
    var equal_mono_pairs = [
         [["monomial", 1, [["x",1]]], ["monomial", 1, [["x",1]]]],
         [["monomial", 1, [["x",2],["y",2]]], ["monomial", 1, [["x",2],["y",2]]]],
         ]
         
    equal_mono_pairs.forEach(function(pair) {
        it(pair, function() {
            expect(poly.mono_less_than( pair[0], pair[1] )).toBeFalsy();
            expect(poly.mono_less_than( pair[1], pair[0] )).toBeFalsy();
                                   });
                                });
});

describe("initial terms", function () {
    var polys_inits = {
    '1+x^3': ["monomial", 1, [["x",3]]],
    '3-2y^2+5/2y': ["monomial", -2, [["y",2]]],
    '2abc-a-2b+3c': ["monomial", 2, [["a",1],["b",1],["c",1]]],
    'x sin(x)-x': ["monomial", 1, [["x",1],[["apply", "sin", "x"],1]]],
    '(x+3)(2x-4)': ["monomial", 2, [["x",2]]],
    'x/7-2/3+3/4x^2': ["monomial", 0.75, [["x",2]]],
    '9x^(2/3)-pi*x': ["monomial", ['-', 'pi'], [["x",1]]],
    '(5x^2-3x+1)/3': ["monomial", ['/', 5, 3], [["x",2]]],
    '7i+2x+3ix': ["monomial", ['+', 2, ['*', 3, 'i']], [["x", 1]]],
    '6t+2t-5t^2-1+5t^2': ["monomial", 8, [["t", 1]]],
    't-t^1000000000': ["monomial", -1, [["t",1000000000]]],
    '(x+y)^2': ["monomial", 1, [["x",2]]],
    '(s-t)(s+t)': ["monomial", 1, [["s", 2]]],
    '5t^(3.1)': ["monomial", 5, [[[ '^', 't', 0.1 ], 31]]],
    };
         
    Object.keys(polys_inits).forEach(function(string) {
        it("poly " + string, function() {
           expect(poly.initial_term(poly.expression_to_polynomial(me.fromText(string)))).toEqual(polys_inits[string]);
        });
    });
});

describe("text to polynomial", function () {

    var polys = {
    '1+x': ["polynomial", "x", [[0,1], [1,1]]],
	'1+x^3': ["polynomial", "x", [[0,1], [3,1]]],
	'3-2y^2+5/2y': ["polynomial", "y", [[0, 3], [1, 2.5], [2,-2]]],
	'2abc-a-2b+3c': ["polynomial", "a", [[0, ["polynomial", "b", [[0, ["polynomial", "c", [[1, 3]]]], [1, -2]]]], [1, ["polynomial", "b", [[0, -1], [1, ["polynomial", "c", [[1, 2]]]]]]]]],
	'x sin(x)-x': ["polynomial", "x", [[1, ["polynomial", ["apply", "sin", "x"], [[0, -1], [1, 1]]]]]],
	'(x+3)(2x-4)': ["polynomial", "x", [[0, -12], [1, 2], [2, 2]]],
	'x/7-2/3+3/4x^2': ["polynomial", "x", [[0, ['/', -2, 3]], [1, ['/', 1, 7]], [2, 0.75]]],
	'9x^(2/3)-pi*x': ["polynomial", "x", [[0, ["polynomial", ['^', 'x', ['/', 1, 3]], [[2, 9]]]], [1, ['-', 'pi']]]],
	'(5x^2-3x+1)/3': ["polynomial", "x", [[0, ['/', 1, 3]], [1, -1], [2, ['/', 5, 3]]]],
	'7i+2x+3ix': ["polynomial", "x", [[0, ['*', 7, 'i']], [1, ['+', 2, ['*', 3, 'i']]]]],
	'6t+2t-5t^2-1+5t^2': ["polynomial", "t", [[0, -1], [1, 8]]],
	'(3,4)': false,
	'0/0': NaN,
	't-t^1000000000': ["polynomial", "t", [[1,1], [1000000000, -1]]],
	'x/y+3x': ["polynomial", 'x', [[0, ["polynomial", ['/', 'x', 'y'], [[1,1]]]], [1, 3]]],
	'(x+y)^2': ["polynomial", 'x', [[0, ["polynomial", "y", [[2,1]]]], [1, ["polynomial", "y", [[1, 2]]]], [2, 1]]],
	'(s-t)(s+t)': ["polynomial", "s", [[0, ["polynomial", "t", [[2,-1]]]], [2, 1]]],
	'5t^(3.1)': ["polynomial", ['^', 't', 0.1], [[31, 5]]],
	'5t^(3.1415)': ["polynomial", ['^', 't', 3.1415], [[1, 5]]],
    };

    Object.keys(polys).forEach(function(string) {
	it("poly " + string, function() {
	    expect(poly.expression_to_polynomial(me.fromText(string))).toEqual(polys[string]);
	});	
    });



    var expressions = [
	'3x^2+2xy-z^2',
	['(s-t)(s+t)', 's^2-t^2'],
	['(x+y)^3', 'x^3 + 3x^2y + 3xy^2 + y^3'],
	'sin(x)y-y^3',
	'9(3y-2x)^3.1',
	'9(3y-2x)^3.1415',
	['(5a^2xyz-3uvw)(5a^2xyz+3uvw)', '25a^4x^2y^2z^2-9u^2v^2w^2'],
	'3qt-qt/(3sr)',
    ];

    function round_trip(expr) {
	return poly.polynomial_to_expression(poly.expression_to_polynomial(
	    me.from(expr)))
    }
    
    // expression should be equal after converting to polynomial and back
    // (if an array, then first element should be converted to second)
    expressions.forEach(function(expr) {
	it(expr, function() {
	    if(Array.isArray(expr)) {
		expect(trees.equal(
		    round_trip(expr[0]),
		    simplify.simplify(me.fromText(expr[1]))
		)).toBeTruthy();
	    }
	    else {
		expect(trees.equal(
		    round_trip(expr),
		    simplify.simplify(me.fromText(expr))
		)).toBeTruthy();
	    }
	});
    });

    // additional round trips should leave expression unchanged
    expressions.forEach(function(expr) {
	it(expr, function() {
	    if(Array.isArray(expr)) {
		expect(trees.equal(
		    round_trip(round_trip(expr[0])),
		    round_trip(expr[0])
		)).toBeTruthy();
	    }
	    else {
		expect(trees.equal(
		    round_trip(round_trip(expr)),
		    round_trip(expr)
		)).toBeTruthy();
	    }
	});
    });

    
});


describe("polynomial operations", function () {

    var sums =
	[['x+3', '(x+1)^2', 'x^2+3x+4'],
	 ['x+y', 'w+z', 'w+x+y+z'],
	 ['3y-2x+1', '5y+2x+4', '8y+5'],
	 ['t+s', 't-s', '2t'],
	 ['t+1', '-t+2', '3'],
	 ['1', '-3', '-2'],
	 ['7', 'xy-z', 'xy-z+7'],
	 ['qvu-y^2+5', '-5', 'qvu-y^2'],
	];

    
    sums.forEach(function(item) {
	it("sum: " + item[0] + ' + ' + item[1] + ' = ' + item[2], function () {
	    let p1 = poly.expression_to_polynomial(me.fromText(item[0]));
	    let p2 = poly.expression_to_polynomial(me.fromText(item[1]));
	    let p3 = poly.expression_to_polynomial(me.fromText(item[2]));

	    expect(poly.polynomial_add(p1,p2)).toEqual(p3);
	    expect(poly.polynomial_sub(p3,p1)).toEqual(p2);
	    expect(poly.polynomial_sub(p3,p2)).toEqual(p1);
	});
    });
    
    var negs =
	[['x+3', '-x-3'],
	 ['x+y', '-x-y'],
	 ['3y-2x+1', '-3y+2x-1'],
	 ['4', '-4'],
	 ['-7', '7'],
	];

    
    negs.forEach(function(item) {
	it("neg: -(" + item[0] + ') = ' + item[1], function () {
	    let p1 = poly.expression_to_polynomial(me.fromText(item[0]));
	    let p2 = poly.expression_to_polynomial(me.fromText(item[1]));

	    expect(poly.polynomial_neg(p1)).toEqual(p2);
	    
	});
    });
    
    
    var prods =
	[['x+3', '(x+1)^2', 'x^3+5x^2+7x+3'],
	 ['x+y', 'w+z', 'wx+wy+zx+zy'],
	 ['3y-2x+1', '5y+2x+4', '15y^2-4xy-4x^2-6x+17y+4'],
	 ['2', '-3', '-6'],
	 ['7', 'xy-z', '7xy-7z'],
	 ['qvu-y^2+5', '-5', '-5qvu+5y^2-25'],
	 ['x-y', 'x+y', 'x^2-y^2'],
	 ['x^2+2x+2', 'x^2-2x+2', 'x^4+4'],
	];

    
    prods.forEach(function(item) {
	it("prod: (" + item[0] + ') * (' + item[1] + ') = ' + item[2], function () {
	    let p1 = poly.expression_to_polynomial(me.fromText(item[0]));
	    let p2 = poly.expression_to_polynomial(me.fromText(item[1]));
	    let p3 = poly.expression_to_polynomial(me.fromText(item[2]));

	    expect(poly.polynomial_mul(p1,p2)).toEqual(p3);

	});
    });


    var pows =
	[['x+y', '2', 'x^2+2xy+y^2'],
	 ['3y-2x+1', '2', '9y^2-12xy+4x^2-4x+6y+1'],
	 ['x-y', '3', 'x^3-3x^2y+3xy^2-y^3'],
	 ['x-y', '4', 'x^4-4x^3y+6x^2y^2-4xy^3+y^4'],
	 ['7', '3', '343'],
	 ['3', '7', '2187'],
	 ['qvu-y^2+5', '-5', undefined],
	 ['x+3', 'x+1', undefined],
	];

    
    pows.forEach(function(item) {
	it("pow: (" + item[0] + ') ^ (' + item[1] + ') = ' + item[2], function () {
	    let p1 = poly.expression_to_polynomial(me.fromText(item[0]));
	    let p2 = poly.expression_to_polynomial(me.fromText(item[1]));
	    let p3 = undefined;
	    if(item[2] !== undefined)
		p3 = poly.expression_to_polynomial(me.fromText(item[2]));

	    expect(poly.polynomial_pow(p1,p2)).toEqual(p3);

	});
    });


});
