import astToText from '../lib/converters/ast-to-text';

var converter = new astToText();


const objectsToTest = [
  {
    'ast': ['*', ['/', 1, 2], 'x'],
    'text': '(1/2) x'
  },
  {
    'ast': ['+', 1, 'x', 3],
    'text': '1 + x + 3'
  },
  {
    'ast': ['+', 1, ['-', 'x'],
      ['-', 3]
    ],
    'text': '1 - x - 3'
  },
  {
    'ast': ['+', 1, ['-', 'x'],
      -3
    ],
    'text': '1 - x - 3'
  },
  {
    'ast': ['+', ['-', 'x'],
      ['-', 0]
    ],
    'text': '-x - 0'
  },
  {
    'ast': ['+', ['-', 'x'],
      -0
    ],
    'text': '-x + 0'
  },
  {
    'ast': ['+', -5,
      -3
    ],
    'text': '-5 - 3'
  },
  {
    'ast': ['-', 0],
    'text': '-0'
  },
  {
    'ast': -0,
    'text': '0'
  },
  {
    'ast': ['+', 1, ['-', ['-', 'x']]],
    'text': '1 - -x'
  },
  {
    'ast': ['apply', 'log', 'x'],
    'text': 'log(x)'
  },
  {
    'ast': ['apply', 'ln', 'x'],
    'text': 'ln(x)'
  },
  {
    'ast': ['apply', 'abs', 'x'],
    'text': '|x|'
  },
  {
    'ast': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
    'text': '|sin(|x|)|'
  },
  {
    'ast': ['^', 'x', 2],
    'text': 'x^2'
  },
  {
    'ast': ['-', ['^', 'x', 2]],
    'text': '-x^2'
  },
  {
    'ast': ['+', 'a', ['-', ['^', 'x', 2]]],
    'text': 'a - x^2'
  },
  {
    'ast': ['^', 'x', 47],
    'text': 'x^47'
  },
  {
    'ast': ['*', ['^', 'x', 'a'], 'b'],
    'text': 'x^a b'
  },
  {
    'ast': ['^', 'x', ['apply', 'factorial', 'a']],
    'text': 'x^a!'
  },
  {
    'ast': ['vector', 1, 2],
    'text': '( 1, 2 )'
  },
  {
    'ast': ['altvector', 'x', 'y'],
    'text': '⟨ x, y ⟩', // langle and rangle delimiters
  },
  {
    'ast': ['*', 'x', 'y', 'z'],
    'text': 'x y z'
  },
  {
    'ast': ['*', 'c', ['+', 'a', 'b']],
    'text': 'c (a + b)'
  },
  {
    'ast': ['*', ['+', 'a', 'b'], 'c'],
    'text': '(a + b) c'
  },
  {
    'ast': ['apply', 'factorial', 'a'],
    'text': 'a!'
  },
  {
    'ast': 'theta',
    'text': 'θ'
  },
  {
    'ast': ['*', 't', 'h', 'e', 't', 'a'],
    'text': 't h e t a'
  },
  {
    'ast': ['apply', 'cos', 'theta'],
    'text': 'cos(θ)'
  },
  {
    'ast': ['*', 'c', 'o', 's', 'x'],
    'text': 'c o s x'
  },
  {
    'ast': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
    'text': '|sin(|x|)|'
  },
  {
    'ast': ['*', 'blah', 'x'],
    'text': 'blah x'
  },
  {
    'ast': ['apply', 'abs', ['=', ['+', 'x', 3], 2]],
    'text': '|x + 3 = 2|'
  },
  {
    'ast': ['_', 'x', ['_', 'y', 'z']],
    'text': 'x_(y_z)'
  },
  {
    'ast': ['_', 'x', ['_', 'y', 'z']],
    'text': 'x_(y_z)'
  },
  {
    'ast': ['_', ['_', 'x', 'y'], 'z'],
    'text': '(x_y)_z'
  },
  {
    'ast': ['^', 'x', ['^', 'y', 'z']],
    'text': 'x^(y^z)'
  },
  {
    'ast': ['^', 'x', ['^', 'y', 'z']],
    'text': 'x^(y^z)'
  },
  {
    'ast': ['^', ['^', 'x', 'y'], 'z'],
    'text': '(x^y)^z'
  },
  {
    'ast': ['^', 'x', ['_', 'y', 'z']],
    'text': 'x^(y_z)'
  },
  {
    'ast': ['^', ['_', 'x', 'y'], 'z'],
    'text': 'x_y^z'
  },
  {
    'ast': ['*', 'x', 'y', ['apply', 'factorial', 'z']],
    'text': 'x y z!'
  },
  {
    'ast': 'x',
    'text': 'x'
  },
  {
    'ast': 'f',
    'text': 'f'
  },
  {
    'ast': ['*', 'f', 'g'],
    'text': 'f g'
  },
  {
    'ast': ['+', 'f', 'g'],
    'text': 'f + g'
  },
  {
    'ast': ['apply', 'f', 'x'],
    'text': 'f(x)'
  },
  {
    'ast': ['apply', 'f', ['tuple', 'x', 'y', 'z']],
    'text': 'f( x, y, z )'
  },
  {
    'ast': ['*', 'f', ['apply', 'g', 'x']],
    'text': 'f g(x)'
  },
  {
    'ast': ['*', 'f', 'p', 'x'],
    'text': 'f p x'
  },
  {
    'ast': ['*', 'f', 'x'],
    'text': 'f x'
  },
  {
    'ast': ['prime', 'f'],
    'text': "f'"
  },
  {
    'ast': ['*', 'f', ['prime', 'g']],
    'text': "f g'"
  },
  {
    'ast': ['*', ['prime', 'f'], 'g'],
    'text': "f' g"
  },
  {
    'ast': ['*', ['prime', 'f'],
      ['prime', ['prime', 'g']]
    ],
    'text': "f' g''"
  },
  {
    'ast': ['prime', 'x'],
    'text': "x'"
  },
  {
    'ast': ['apply', ['prime', 'f'], 'x'],
    'text': "f'(x)"
  },
  {
    'ast': ['prime', ['apply', 'f', 'x']],
    'text': "f(x)'"
  },
  {
    'ast': ['prime', ['apply', 'sin', 'x']],
    'text': "sin(x)'"
  },
  {
    'ast': ['apply', ['prime', 'sin'], 'x'],
    'text': "sin'(x)"
  },
  {
    'ast': ['apply', ['prime', ['prime', 'f']], 'x'],
    'text': "f''(x)"
  },
  {
    'ast': ['prime', ['prime', ['apply', 'sin', 'x']]],
    'text': "sin(x)''"
  },
  {
    'ast': ['^', ['apply', 'f', 'x'],
      ['_', 't', 'y']
    ],
    'text': 'f(x)^(t_y)'
  },
  {
    'ast': ['apply', ['_', 'f', 't'], 'x'],
    'text': 'f_t(x)'
  },
  {
    'ast': ['_', ['apply', 'f', 'x'], 't'],
    'text': 'f(x)_t'
  },
  {
    'ast': ['apply', ['^', 'f', 2], 'x'],
    'text': 'f^2(x)'
  },
  {
    'ast': ['^', ['apply', 'f', 'x'], 2],
    'text': 'f(x)^2'
  },
  {
    'ast': ['apply', ['^', ['prime', 'f'], 'a'], 'x'],
    'text': "f'^a(x)"
  },
  {
    'ast': ['apply', ['^', 'f', ['prime', 'a']], 'x'],
    'text': "f^(a')(x)"
  },
  {
    'ast': ['apply', ['^', ['_', 'f', 'a'],
      ['prime', 'b']
    ], 'x'],
    'text': "f_a^(b')(x)"
  },
  {
    'ast': ['apply', ['^', ['prime', ['_', 'f', 'a']], 'b'], 'x'],
    'text': "f_a'^b(x)"
  },
  {
    'ast': ['apply', 'sin', 'x'],
    'text': 'sin(x)'
  },
  {
    'ast': ['*', ['apply', ['^', 'sin', 'x'], 'y'], 'z'],
    'text': 'sin^x(y) z'
  },
  {
    'ast': ['*', ['apply', 'sin', 'x'], 'y'],
    'text': 'sin(x) y'
  },
  {
    'ast': ['apply', ['^', 'sin', 2], 'x'],
    'text': 'sin^2(x)'
  },
  {
    'ast': ['apply', 'exp', 'x'],
    'text': 'exp(x)'
  },
  {
    'ast': ['^', 'e', 'x'],
    'text': 'e^x'
  },
  {
    'ast': ['^', 'x', ['apply', 'factorial', 2]],
    'text': 'x^2!'
  },
  {
    'ast': ['^', 'x', ['apply', 'factorial', ['apply', 'factorial', 2]]],
    'text': 'x^2!!'
  },
  {
    'ast': ['^', ['_', 'x', 't'], 2],
    'text': 'x_t^2'
  },
  {
    'ast': ['_', 'x', ['^', 'f', 2]],
    'text': 'x_(f^2)'
  },
  {
    'ast': ['prime', ['_', 'x', 't']],
    'text': "x_t'"
  },
  {
    'ast': ['_', 'x', ['prime', 'f']],
    'text': "x_(f')"
  },
  {
    'ast': ['tuple', 'x', 'y', 'z'],
    'text': '( x, y, z )'
  },
  {
    'ast': ['+', ['tuple', 'x', 'y'],
      ['-', ['array', 'x', 'y']]
    ],
    'text': '( x, y ) - [ x, y ]'
  },
  {
    'ast': ['*', 2, ['+', 'z', ['-', ['+', 'x', 1]]]],
    'text': '2 (z - (x + 1))'
  },
  {
    'ast': ['set', 1, 2, 'x'],
    'text': '{ 1, 2, x }'
  },
  {
    'ast': ['set', 'x', 'x'],
    'text': '{ x, x }'
  },
  {
    'ast': ['set', 'x'],
    'text': '{ x }'
  },
  {
    'ast': ['interval', ['tuple', 1, 2],
      ['tuple', false, true]
    ],
    'text': '( 1, 2 ]'
  },
  {
    'ast': ['array', 1, 2],
    'text': '[ 1, 2 ]'
  },
  {
    'ast': ['tuple', 1, 2],
    'text': '( 1, 2 )'
  },
  {
    'ast': ['list', 1, 2, 3],
    'text': '1, 2, 3'
  },
  {
    'ast': ['=', 'x', 'a'],
    'text': 'x = a'
  },
  {
    'ast': ['=', 'x', 'y', 1],
    'text': 'x = y = 1'
  },
  {
    'ast': ['=', 'x', ['=', 'y', 1]],
    'text': 'x = (y = 1)'
  },
  {
    'ast': ['=', ['=', 'x', 'y'], 1],
    'text': '(x = y) = 1'
  },
  {
    'ast': ['ne', 7, 2],
    'text': '7 ≠ 2'
  },
  {
    'ast': ['not', ['=', 'x', 'y']],
    'text': 'not (x = y)'
  },
  {
    'ast': ['not', ['=', 'x', 'y']],
    'text': 'not (x = y)'
  },
  {
    'ast': ['>', 'x', 'y'],
    'text': 'x > y'
  },
  {
    'ast': ['ge', 'x', 'y'],
    'text': 'x ≥ y'
  },
  {
    'ast': ['gts', ['tuple', 'x', 'y', 'z'],
      ['tuple', true, true]
    ],
    'text': 'x > y > z'
  },
  {
    'ast': ['gts', ['tuple', 'x', 'y', 'z'],
      ['tuple', true, false]
    ],
    'text': 'x > y ≥ z'
  },
  {
    'ast': ['gts', ['tuple', 'x', 'y', 'z'],
      ['tuple', false, true]
    ],
    'text': 'x ≥ y > z'
  },
  {
    'ast': ['gts', ['tuple', 'x', 'y', 'z'],
      ['tuple', false, false]
    ],
    'text': 'x ≥ y ≥ z'
  },
  {
    'ast': ['<', 'x', 'y'],
    'text': 'x < y'
  },
  {
    'ast': ['le', 'x', 'y'],
    'text': 'x ≤ y'
  },
  {
    'ast': ['lts', ['tuple', 'x', 'y', 'z'],
      ['tuple', true, true]
    ],
    'text': 'x < y < z'
  },
  {
    'ast': ['lts', ['tuple', 'x', 'y', 'z'],
      ['tuple', true, false]
    ],
    'text': 'x < y ≤ z'
  },
  {
    'ast': ['lts', ['tuple', 'x', 'y', 'z'],
      ['tuple', false, true]
    ],
    'text': 'x ≤ y < z'
  },
  {
    'ast': ['lts', ['tuple', 'x', 'y', 'z'],
      ['tuple', false, false]
    ],
    'text': 'x ≤ y ≤ z'
  },
  {
    'ast': ['>', ['<', 'x', 'y'], 'z'],
    'text': '(x < y) > z'
  },
  {
    'ast': ['subset', 'A', 'B'],
    'text': 'A ⊂ B'
  },
  {
    'ast': ['notsubset', 'A', 'B'],
    'text': 'A ⊄ B'
  },
  {
    'ast': ['superset', 'A', 'B'],
    'text': 'A ⊃ B'
  },
  {
    'ast': ['notsuperset', 'A', 'B'],
    'text': 'A ⊅ B'
  },
  {
    'ast': ['in', 'x', 'A'],
    'text': 'x ∈ A'
  },
  {
    'ast': ['notin', 'x', 'A'],
    'text': 'x ∉ A'
  },
  {
    'ast': ['ni', 'A', 'x'],
    'text': 'A ∋ x'
  },
  {
    'ast': ['notni', 'A', 'x'],
    'text': 'A ∌ x'
  },
  {
    'ast': ['union', 'A', 'B'],
    'text': 'A ∪ B'
  },
  {
    'ast': ['intersect', 'A', 'B'],
    'text': 'A ∩ B'
  },
  {
    'ast': ['and', 'A', 'B'],
    'text': 'A and B'
  },
  {
    'ast': ['and', 'A', 'B'],
    'text': 'A and B'
  },
  {
    'ast': ['or', 'A', 'B'],
    'text': 'A or B'
  },
  {
    'ast': ['and', 'A', 'B', 'C'],
    'text': 'A and B and C'
  },
  {
    'ast': ['or', 'A', 'B', 'C'],
    'text': 'A or B or C'
  },
  {
    'ast': ['or', ['and', 'A', 'B'], 'C'],
    'text': '(A and B) or C'
  },
  {
    'ast': ['or', 'A', ['and', 'B', 'C']],
    'text': 'A or (B and C)'
  },
  {
    'ast': ['not', ['=', 'x', 1]],
    'text': 'not (x = 1)'
  },
  {
    'ast': ['not', ['=', 'x', 1]],
    'text': 'not (x = 1)'
  },
  {
    'ast': ['or', ['not', ['=', 'x', 'y']],
      ['ne', 'z', 'w']
    ],
    'text': '(not (x = y)) or (z ≠ w)'
  },
  {
    'ast': ['+', ['*', 1.2, 'e'],
      ['-', 3]
    ],
    'text': '1.2 e - 3'
  },
  {
    'ast': ['+', ['*', 1.2, 'e'],
      -3
    ],
    'text': '1.2 e - 3'
  },
  {
    'ast': Infinity,
    'text': '∞'
  },
  {
    'ast': ['*', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j'],
    'text': 'a b c d e f g h i j'
  },
  {
    'ast': 2,
    'text': '2'
  },

  {
    'ast': '',
    'text': ''
  },
  {
    'ast':  ['matrix', ['tuple', 2, 2], ['tuple', ['tuple', 'a', 'b'], ['tuple', 'c', 'd']]],
    'text': '[ [ a, b ], [ c, d ] ]',
  },
  {
    'ast': ['matrix', ['tuple', 1, 2], ['tuple', ['tuple', ['+', 'a', ['*', 3, 'y']], ['*', 2, ['apply', 'sin', 'theta']]]]],
    'text': '[ [ a + 3 y, 2 sin(θ) ] ]',
  },
  {
    'ast': ['matrix', ['tuple', 2, 3], ['tuple', ['tuple', 8, 0, 0], ['tuple', 1, 2, 3]]],
    'text': '[ [ 8, 0, 0 ], [ 1, 2, 3 ] ]',
  },
  {
    'ast': ['derivative_leibniz', 'x', ['tuple', 't']],
    'text': 'dx/dt',
  },
  {
    'ast': ['derivative_leibniz', ['tuple', 'x', 2], ['tuple', ['tuple', 't', 2]]],
    'text': 'd^2x/dt^2',
  },
  {
    'ast': ['derivative_leibniz', ['tuple', 'mu', 2], ['tuple', 'tau', 'xi']],
    'text': 'd^2μ/dτdξ',
  },
  {
    'ast': ['derivative_leibniz', ['tuple', 'x', 3], ['tuple', 's', ['tuple', 't', 2]]],
    'text': 'd^3x/dsdt^2',
  },
  {
    'ast': ['derivative_leibniz', ['tuple', 'x', 3], ['tuple', ['tuple', 's', 2], ['tuple', 't', 1]]],
    'text': 'd^3x/ds^2dt',
  },
  {
    'ast': ['partial_derivative_leibniz', 'x', ['tuple', 't']],
    'text': '∂x/∂t',
  },
  {
    'ast': ['partial_derivative_leibniz', ['tuple', 'x', 2], ['tuple', ['tuple', 't', 2]]],
    'text': '∂^2x/∂t^2',
  },
  {
    'ast': ['partial_derivative_leibniz', ['tuple', 'mu', 2], ['tuple', 'tau', 'xi']],
    'text': '∂^2μ/∂τ∂ξ',
  },
  {
    'ast': ['partial_derivative_leibniz', ['tuple', 'x', 3], ['tuple', 's', ['tuple', 't', 2]]],
    'text': '∂^3x/∂s∂t^2',
  },
  {
    'ast': ['partial_derivative_leibniz', ['tuple', 'x', 3], ['tuple', ['tuple', 's', 2], ['tuple', 't', 1]]],
    'text': '∂^3x/∂s^2∂t',
  },
  {
    'ast': ["*","a",["apply","abs","x"]],
    'text': 'a |x|',
  },
  {
    'ast': ["*",["apply","abs","a"],"b",["apply","abs","c"]],
    'text': '|a| b |c|',
  },
  {
    'ast': ["apply","abs",["*","a",["apply","abs","b"],"c"]],
    'text': '|a |b| c|',
  },
  {
    'ast': ['|', 'A', 'B'],
    'text': 'A | B',
  },
  {
    'ast': [':', 'A', 'B'],
    'text': 'A : B',
  },
  {
    'ast': ['apply', 'P', ['|', ['>', 'X', 1], ['=', 'A', 'B']]],
    'text': 'P(X > 1 | A = B)',
  },
  {
    'ast': ['apply', 'P', [':', ['>', 'X', 1], ['=', 'A', 'B']]],
    'text': 'P(X > 1 : A = B)',
  },
  {
    'ast': ['set', ['|', 'x', ['>', 'x', 0]]],
    'text': '{ x | x > 0 }',
  },
  {
    'ast': ['set', [':', 'x', ['>', 'x', 0]]],
    'text': '{ x : x > 0 }',
  },
  {
    'ast': ['ldots'],
    'text': '...',
  },
  {
    'ast': ['list', 1, 2, 3, ['ldots']],
    'text': '1, 2, 3, ...',
  },
  {
    'ast': ['tuple', 1, 2, 3, ['ldots']],
    'text': '( 1, 2, 3, ... )',
  },
  {
    'ast': 0.0000000000123,
    'text': '1.23 * 10^(-11)',
  },
  {
    'ast': 12300000000000000000000,
    'text': '1.23 * 10^22',
  },
  {
    'ast': ['^', 0.0000000000123, 5],
    'text': '(1.23 * 10^(-11))^5',
  },
  {
    'ast': ['^', -3, 'x'],
    'text': '(-3)^x',
  },
  {
    'ast': ['^', -3, 2],
    'text': '(-3)^2',
  },
  {
    'ast': ['^', ['-', 3], 2],
    'text': '(-3)^2',
  },
  {
    'ast': ['apply', 're', 'x'],
    'text': 're(x)',
  },
  {
    'ast': ['apply', 'im', 'x'],
    'text': 'im(x)',
  },
  {
    'ast': ['apply', 'nCr', ['tuple', 'x', 'y']],
    'text': 'nCr( x, y )',
  },
  {
    'ast': ['apply', 'nPr', ['tuple', 'x', 'y']],
    'text': 'nPr( x, y )',
  },
  {
    'ast': ['binom', 'x', 'y'],
    'text': 'binom( x, y )',
  },
  {
    'ast': ['vec', 'a'],
    'text': 'vec(a)',
  },
  {
    'ast': ['apply', 'floor', 'a'],
    'text': 'floor(a)',
  },
  {
    'ast': ['apply', 'ceil', 'a'],
    'text': 'ceil(a)',
  },
  {
    'ast': ['apply', 'round', 'a'],
    'text': 'round(a)',
  },
  {
    'ast': ['perp', 'x', 'y'],
    'text': 'x ⟂ y',
  },
  {
    'ast': ['^', 'x', 'perp'],
    'text': 'x^⟂',
  },



]


for (let objectToTest of objectsToTest) {
  test("parses " + objectToTest.ast + ' to ' + objectToTest.text, () => {
    expect(converter.convert(objectToTest.ast)).toEqual(objectToTest.text);
  });

}


test("pad to digits", function () {

  let converter = new astToText({padToDigits: 5 });

  expect(converter.convert(123E28)).toEqual("1.2300 * 10^30")
  expect(converter.convert(123E14)).toEqual("12300000000000000")
  expect(converter.convert(123E8)).toEqual("12300000000")
  expect(converter.convert(123000)).toEqual("123000")
  expect(converter.convert(12300)).toEqual("12300")
  expect(converter.convert(1230)).toEqual("1230.0")
  expect(converter.convert(123)).toEqual("123.00")
  expect(converter.convert(12.3)).toEqual("12.300")
  expect(converter.convert(1.23)).toEqual("1.2300")
  expect(converter.convert(.123)).toEqual("0.12300")
  expect(converter.convert(.0123)).toEqual("0.012300")
  expect(converter.convert(.00123)).toEqual("0.0012300")
  expect(converter.convert(.000123)).toEqual("0.00012300")
  expect(converter.convert(123E-8)).toEqual("0.0000012300")
  expect(converter.convert(123E-14)).toEqual("1.2300 * 10^(-12)")
  expect(converter.convert(123E-28)).toEqual("1.2300 * 10^(-26)")

  expect(converter.convert(['*', 123, ['^', 10, 28]])).toEqual("123.00 * 10^28")
  expect(converter.convert(['*', 123, ['^', 10, -28]])).toEqual("123.00 * 10^(-28)")

  expect(converter.convert(NaN)).toEqual("NaN")

});

test("pad to decimals", function () {

  let converter = new astToText({padToDecimals: 5 });

  expect(converter.convert(123E28)).toEqual("1230000000000000000000000000000.00000")
  expect(converter.convert(123E14)).toEqual("12300000000000000.00000")
  expect(converter.convert(123E8)).toEqual("12300000000.00000")
  expect(converter.convert(123000)).toEqual("123000.00000")
  expect(converter.convert(12300)).toEqual("12300.00000")
  expect(converter.convert(1230)).toEqual("1230.00000")
  expect(converter.convert(123)).toEqual("123.00000")
  expect(converter.convert(12.3)).toEqual("12.30000")
  expect(converter.convert(1.23)).toEqual("1.23000")
  expect(converter.convert(.123)).toEqual("0.12300")
  expect(converter.convert(.0123)).toEqual("0.01230")
  expect(converter.convert(.00123)).toEqual("0.00123")
  expect(converter.convert(.000123)).toEqual("0.000123")
  expect(converter.convert(123E-8)).toEqual("0.00000123")
  expect(converter.convert(123E-14)).toEqual("1.23 * 10^(-12)")
  expect(converter.convert(123E-28)).toEqual("1.23 * 10^(-26)")

  expect(converter.convert(['*', 123, ['^', 10, 28]])).toEqual("123.00000 * 10^28")
  expect(converter.convert(['*', 123, ['^', 10, -28]])).toEqual("123.00000 * 10^(-28)")

  expect(converter.convert(NaN)).toEqual("NaN")

});

//
// describe("ast to text", function() {
//     it("sum of two numbers", function() {
// 	expect(astToText(['+',3,4]).replace(/ /g,'')).toEqual('3+4');
//     });
//
//     it("sum of three terms", function() {
//         expect(astToText(['+',3,4,'x']).replace(/ /g,'')).toEqual('3+4+x');
//     });
//
//     it("nested sum", function() {
//         expect(astToText(['+',3,['+',4,'x']]).replace(/ /g,'')).toEqual('3+(4+x)');
//     });
//
//     it("factorial", function() {
//         expect(astToText(['apply', 'factorial',3]).replace(/ /g,'')).toEqual('3!');
//     });
//
//     it("factorial", function() {
//         expect(astToText(['apply', 'factorial',['+','x','1']]).replace(/ /g,'')).toEqual('(x+1)!');
//     });
//
//     it("sum of positive and negative number", function() {
//         expect(astToText(['+',3,-4]).replace(/ /g,'')).toEqual('3-4');
//     });
//
//     it("product of positive and negative number", function() {
//         expect(astToText(['*',3,-4]).replace(/ /g,'')).toEqual('3(-4)');
//     });
//
//     it("product of positive numbers", function() {
//         expect(astToText(['*',3,4]).replace(/ /g,'')).toEqual('3*4');
//     });
//
//     it("sin^2 (3x)", function() {
//         expect(astToText(['apply', ['^','sin',2],['*',3,'x']]).replace(/ /g,'')).toEqual('sin^2(3x)');
//     });
//
//     it("arcsec(3x)", function() {
//         expect(astToText(['apply','arcsec',['*',3,'x']]).replace(/ /g,'')).toEqual('arcsec(3x)');
//     });
//
//     it("theta", function() {
//         expect(astToText(['+', 1, 'theta']).replace(/ /g,'')).toEqual('1+θ');
//     });
//
//     it("factorial", function() {
//         expect(astToText(['apply', 'factorial', 17]).replace(/ /g,'')).toEqual('17!');
//     });
//
//     it("vector", function() {
//         expect(astToText(['vector', 1, 'x']).replace(/ /g,'')).toEqual('(1,x)');
//     });
//
//     it("throws error apply", function() {
// 	expect(function () {astToText(['sin', 'x'])}).toThrowError();
//     });
//
//     it("throws error lts", function() {
// 	expect(function () {astToText(['lts', 'x', 'y', 'z'])}).toThrowError();
//     });
//
//     it("throws error gts", function() {
// 	expect(function () {astToText(['gts', 'x', 'y', 'z'])}).toThrowError();
//     });
//
//     it("throws error interval", function() {
// 	expect(function () {astToText(['interval', 'x', 'y'])}).toThrowError();
//     });
//
//
//
// });
