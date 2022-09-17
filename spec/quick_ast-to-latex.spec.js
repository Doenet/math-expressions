import astToLatex from '../lib/converters/ast-to-latex';

var converter = new astToLatex();

const objectsToTest = [
  {
    'ast': ['*', ['/', 1, 2], 'x'],
    'latex': '\\left(\\frac{1}{2}\\right) \\, x'
  },
  {
    'ast': ['+', 1, 'x', 3],
    'latex': '1 + x + 3'
  },
  {
    'ast': ['+', 1, ['-', 'x'],
      ['-', 3]
    ],
    'latex': '1 - x - 3'
  },
  {
    'ast': ['+', 1, ['-', 'x'],
      -3
    ],
    'latex': '1 - x - 3'
  },
  {
    'ast': ['+', ['-', 'x'],
      ['-', 0]
    ],
    'latex': '-x - 0'
  },
  {
    'ast': ['+', ['-', 'x'],
      -0
    ],
    'latex': '-x + 0'
  },
  {
    'ast': ['+', -5,
      -3
    ],
    'latex': '-5 - 3'
  },
  {
    'ast': ["-", 0],
    'latex': '-0'
  },
  {
    'ast': -0,
    'latex': '0'
  },
  {
    'ast': ['+', 1, ['-', ['-', 'x']]],
    'latex': '1 - -x'
  },
  {
    'ast': ['apply', 'log', 'x'],
    'latex': '\\log\\left(x\\right)'
  },
  {
    'ast': ['apply', 'ln', 'x'],
    'latex': '\\ln\\left(x\\right)'
  },
  {
    'ast': ['apply', 'log10', 'x'],
    'latex': '\\log_{10}\\left(x\\right)'
  },
  {
    'ast': ['apply', 'abs', 'x'],
    'latex': '\\left|x\\right|'
  },
  {
    'ast': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
    'latex': '\\left|\\sin\\left(\\left|x\\right|\\right)\\right|'
  },
  {
    'ast': ['^', 'x', 2],
    'latex': 'x^{2}'
  },
  {
    'ast': ['-', ['^', 'x', 2]],
    'latex': '-x^{2}'
  },
  {
    'ast': ['+', 'a', ['-', ['^', 'x', 2]]],
    'latex': 'a - x^{2}'
  },
  {
    'ast': ['^', 'x', 47],
    'latex': 'x^{47}'
  },
  {
    'ast': ['*', ['^', 'x', 'a'], 'b'],
    'latex': 'x^{a} \\, b'
  },
  {
    'ast': ['^', 'x', ['apply', 'factorial', 'a']],
    'latex': 'x^{a!}'
  },
  {
    'ast': ['*', 'x', 'y', 'z'],
    'latex': 'x \\, y \\, z'
  },
  {
    'ast': ['*', 'c', ['+', 'a', 'b']],
    'latex': 'c \\, \\left(a + b\\right)'
  },
  {
    'ast': ['*', ['+', 'a', 'b'], 'c'],
    'latex': '\\left(a + b\\right) \\, c'
  },
  {
    'ast': ['apply', 'factorial', 'a'],
    'latex': 'a!'
  },
  {
    'ast': 'theta',
    'latex': '\\theta'
  },
  {
    'ast': ['*', 't', 'h', 'e', 't', 'a'],
    'latex': 't \\, h \\, e \\, t \\, a'
  },
  {
    'ast': ['apply', 'cos', 'theta'],
    'latex': '\\cos\\left(\\theta\\right)'
  },
  {
    'ast': ['*', 'c', 'o', 's', 'x'],
    'latex': 'c \\, o \\, s \\, x'
  },
  {
    'ast': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
    'latex': '\\left|\\sin\\left(\\left|x\\right|\\right)\\right|'
  },
  {
    'ast': ['*', 'blah', 'x'],
    'latex': '\\var{blah} \\, x'
  },
  {
    'ast': ['apply', 'abs', ['=', ['+', 'x', 3], 2]],
    'latex': '\\left|x + 3 = 2\\right|'
  },
  {
    'ast': ['_', 'x', ['_', 'y', 'z']],
    'latex': 'x_{y_{z}}'
  },
  {
    'ast': ['_', 'x', ['_', 'y', 'z']],
    'latex': 'x_{y_{z}}'
  },
  {
    'ast': ['_', ['_', 'x', 'y'], 'z'],
    'latex': '\\left(x_{y}\\right)_{z}'
  },
  {
    'ast': ['^', 'x', ['^', 'y', 'z']],
    'latex': 'x^{y^{z}}'
  },
  {
    'ast': ['^', 'x', ['^', 'y', 'z']],
    'latex': 'x^{y^{z}}'
  },
  {
    'ast': ['^', ['^', 'x', 'y'], 'z'],
    'latex': '\\left(x^{y}\\right)^{z}'
  },
  {
    'ast': ['^', 'x', ['_', 'y', 'z']],
    'latex': 'x^{y_{z}}'
  },
  {
    'ast': ['^', ['_', 'x', 'y'], 'z'],
    'latex': 'x_{y}^{z}'
  },
  {
    'ast': ['*', 'x', 'y', ['apply', 'factorial', 'z']],
    'latex': 'x \\, y \\, z!'
  },
  {
    'ast': 'x',
    'latex': 'x'
  },
  {
    'ast': 'f',
    'latex': 'f'
  },
  {
    'ast': ['*', 'f', 'g'],
    'latex': 'f \\, g'
  },
  {
    'ast': ['+', 'f', 'g'],
    'latex': 'f + g'
  },
  {
    'ast': ['apply', 'f', 'x'],
    'latex': 'f\\left(x\\right)'
  },
  {
    'ast': ['apply', 'f', ['tuple', 'x', 'y', 'z']],
    'latex': 'f\\left( x, y, z \\right)'
  },
  {
    'ast': ['*', 'f', ['apply', 'g', 'x']],
    'latex': 'f \\, g\\left(x\\right)'
  },
  {
    'ast': ['*', 'f', 'p', 'x'],
    'latex': 'f \\, p \\, x'
  },
  {
    'ast': ['*', 'f', 'x'],
    'latex': 'f \\, x'
  },
  {
    'ast': ['prime', 'f'],
    'latex': "f'"
  },
  {
    'ast': ['*', 'f', ['prime', 'g']],
    'latex': "f \\, g'"
  },
  {
    'ast': ['*', ['prime', 'f'], 'g'],
    'latex': "f' \\, g"
  },
  {
    'ast': ['*', ['prime', 'f'],
      ['prime', ['prime', 'g']]
    ],
    'latex': "f' \\, g''"
  },
  {
    'ast': ['prime', 'x'],
    'latex': "x'"
  },
  {
    'ast': ['apply', ['prime', 'f'], 'x'],
    'latex': "f'\\left(x\\right)"
  },
  {
    'ast': ['prime', ['apply', 'f', 'x']],
    'latex': "f\\left(x\\right)'"
  },
  {
    'ast': ['prime', ['apply', 'sin', 'x']],
    'latex': "\\sin\\left(x\\right)'"
  },
  {
    'ast': ['apply', ['prime', 'sin'], 'x'],
    'latex': "\\sin'\\left(x\\right)"
  },
  {
    'ast': ['apply', ['prime', ['prime', 'f']], 'x'],
    'latex': "f''\\left(x\\right)"
  },
  {
    'ast': ['prime', ['prime', ['apply', 'sin', 'x']]],
    'latex': "\\sin\\left(x\\right)''"
  },
  {
    'ast': ['^', ['apply', 'f', 'x'],
      ['_', 't', 'y']
    ],
    'latex': 'f\\left(x\\right)^{t_{y}}'
  },
  {
    'ast': ['apply', ['_', 'f', 't'], 'x'],
    'latex': 'f_{t}\\left(x\\right)'
  },
  {
    'ast': ['_', ['apply', 'f', 'x'], 't'],
    'latex': 'f\\left(x\\right)_{t}'
  },
  {
    'ast': ['apply', ['^', 'f', 2], 'x'],
    'latex': 'f^{2}\\left(x\\right)'
  },
  {
    'ast': ['^', ['apply', 'f', 'x'], 2],
    'latex': 'f\\left(x\\right)^{2}'
  },
  {
    'ast': ['apply', ['^', ['prime', 'f'], 'a'], 'x'],
    'latex': "f'^{a}\\left(x\\right)"
  },
  {
    'ast': ['apply', ['^', 'f', ['prime', 'a']], 'x'],
    'latex': "f^{a'}\\left(x\\right)"
  },
  {
    'ast': ['apply', ['^', ['_', 'f', 'a'],
      ['prime', 'b']
    ], 'x'],
    'latex': "f_{a}^{b'}\\left(x\\right)"
  },
  {
    'ast': ['apply', ['^', ['prime', ['_', 'f', 'a']], 'b'], 'x'],
    'latex': "f_{a}'^{b}\\left(x\\right)"
  },
  {
    'ast': ['apply', 'sin', 'x'],
    'latex': '\\sin\\left(x\\right)'
  },
  {
    'ast': ['*', ['apply', ['^', 'sin', 'x'], 'y'], 'z'],
    'latex': '\\sin^{x}\\left(y\\right) \\, z'
  },
  {
    'ast': ['*', ['apply', 'sin', 'x'], 'y'],
    'latex': '\\sin\\left(x\\right) \\, y'
  },
  {
    'ast': ['apply', ['^', 'sin', 2], 'x'],
    'latex': '\\sin^{2}\\left(x\\right)'
  },
  {
    'ast': ['apply', 'exp', 'x'],
    'latex': '\\exp\\left(x\\right)'
  },
  {
    'ast': ['^', 'e', 'x'],
    'latex': 'e^{x}'
  },
  {
    'ast': ['^', 'x', ['apply', 'factorial', 2]],
    'latex': 'x^{2!}'
  },
  {
    'ast': ['^', 'x', ['apply', 'factorial', ['apply', 'factorial', 2]]],
    'latex': 'x^{2!!}'
  },
  {
    'ast': ['^', ['_', 'x', 't'], 2],
    'latex': 'x_{t}^{2}'
  },
  {
    'ast': ['_', 'x', ['^', 'f', 2]],
    'latex': 'x_{f^{2}}'
  },
  {
    'ast': ['prime', ['_', 'x', 't']],
    'latex': "x_{t}'"
  },
  {
    'ast': ['_', 'x', ['prime', 'f']],
    'latex': "x_{f'}"
  },
  {
    'ast': ['tuple', 'x', 'y', 'z'],
    'latex': '\\left( x, y, z \\right)'
  },
  {
    'ast': ['+', ['tuple', 'x', 'y'],
      ['-', ['array', 'x', 'y']]
    ],
    'latex': '\\left( x, y \\right) - \\left[ x, y \\right]'
  },
  {
    'ast': ['*', 2, ['+', 'z', ['-', ['+', 'x', 1]]]],
    'latex': '2 \\, \\left(z - \\left(x + 1\\right)\\right)'
  },
  {
    'ast': ['set', 1, 2, 'x'],
    'latex': '\\left\\{ 1, 2, x \\right\\}'
  },
  {
    'ast': ['set', 'x', 'x'],
    'latex': '\\left\\{ x, x \\right\\}'
  },
  {
    'ast': ['set', 'x'],
    'latex': '\\left\\{ x \\right\\}'
  },
  {
    'ast': ['interval', ['tuple', 1, 2],
      ['tuple', false, true]
    ],
    'latex': '\\left( 1, 2 \\right]'
  },
  {
    'ast': ['array', 1, 2],
    'latex': '\\left[ 1, 2 \\right]'
  },
  {
    'ast': ['tuple', 1, 2],
    'latex': '\\left( 1, 2 \\right)'
  },
  {
    'ast': ['vector', 1, 2],
    'latex': '\\left( 1, 2 \\right)'
  },
  {
    'ast': ['list', 1, 2, 3],
    'latex': '1, 2, 3'
  },
  {
    'ast': ['=', 'x', 'a'],
    'latex': 'x = a'
  },
  {
    'ast': ['=', 'x', 'y', 1],
    'latex': 'x = y = 1'
  },
  {
    'ast': ['=', 'x', ['=', 'y', 1]],
    'latex': 'x = \\left(y = 1\\right)'
  },
  {
    'ast': ['=', ['=', 'x', 'y'], 1],
    'latex': '\\left(x = y\\right) = 1'
  },
  {
    'ast': ['ne', 7, 2],
    'latex': '7 \\ne 2'
  },
  {
    'ast': ['not', ['=', 'x', 'y']],
    'latex': '\\lnot \\left(x = y\\right)'
  },
  {
    'ast': ['not', ['=', 'x', 'y']],
    'latex': '\\lnot \\left(x = y\\right)'
  },
  {
    'ast': ['>', 'x', 'y'],
    'latex': 'x > y'
  },
  {
    'ast': ['ge', 'x', 'y'],
    'latex': 'x \\ge y'
  },
  {
    'ast': ['gts', ['tuple', 'x', 'y', 'z'],
      ['tuple', true, true]
    ],
    'latex': 'x > y > z'
  },
  {
    'ast': ['gts', ['tuple', 'x', 'y', 'z'],
      ['tuple', true, false]
    ],
    'latex': 'x > y \\ge z'
  },
  {
    'ast': ['gts', ['tuple', 'x', 'y', 'z'],
      ['tuple', false, true]
    ],
    'latex': 'x \\ge y > z'
  },
  {
    'ast': ['gts', ['tuple', 'x', 'y', 'z'],
      ['tuple', false, false]
    ],
    'latex': 'x \\ge y \\ge z'
  },
  {
    'ast': ['<', 'x', 'y'],
    'latex': 'x < y'
  },
  {
    'ast': ['le', 'x', 'y'],
    'latex': 'x \\le y'
  },
  {
    'ast': ['lts', ['tuple', 'x', 'y', 'z'],
      ['tuple', true, true]
    ],
    'latex': 'x < y < z'
  },
  {
    'ast': ['lts', ['tuple', 'x', 'y', 'z'],
      ['tuple', true, false]
    ],
    'latex': 'x < y \\le z'
  },
  {
    'ast': ['lts', ['tuple', 'x', 'y', 'z'],
      ['tuple', false, true]
    ],
    'latex': 'x \\le y < z'
  },
  {
    'ast': ['lts', ['tuple', 'x', 'y', 'z'],
      ['tuple', false, false]
    ],
    'latex': 'x \\le y \\le z'
  },
  {
    'ast': ['>', ['<', 'x', 'y'], 'z'],
    'latex': '\\left(x < y\\right) > z'
  },
  {
    'ast': ['subset', 'A', 'B'],
    'latex': 'A \\subset B'
  },
  {
    'ast': ['notsubset', 'A', 'B'],
    'latex': 'A \\not\\subset B'
  },
  {
    'ast': ['superset', 'A', 'B'],
    'latex': 'A \\supset B'
  },
  {
    'ast': ['notsuperset', 'A', 'B'],
    'latex': 'A \\not\\supset B'
  },
  {
    'ast': ['in', 'x', 'A'],
    'latex': 'x \\in A'
  },
  {
    'ast': ['notin', 'x', 'A'],
    'latex': 'x \\notin A'
  },
  {
    'ast': ['ni', 'A', 'x'],
    'latex': 'A \\ni x'
  },
  {
    'ast': ['notni', 'A', 'x'],
    'latex': 'A \\not\\ni x'
  },
  {
    'ast': ['union', 'A', 'B'],
    'latex': 'A \\cup B'
  },
  {
    'ast': ['intersect', 'A', 'B'],
    'latex': 'A \\cap B'
  },
  {
    'ast': ['and', 'A', 'B'],
    'latex': 'A \\land B'
  },
  {
    'ast': ['and', 'A', 'B'],
    'latex': 'A \\land B'
  },
  {
    'ast': ['or', 'A', 'B'],
    'latex': 'A \\lor B'
  },
  {
    'ast': ['or', 'A', 'B'],
    'latex': 'A \\lor B'
  },
  {
    'ast': ['and', 'A', 'B', 'C'],
    'latex': 'A \\land B \\land C'
  },
  {
    'ast': ['or', 'A', 'B', 'C'],
    'latex': 'A \\lor B \\lor C'
  },
  {
    'ast': ['or', ['and', 'A', 'B'], 'C'],
    'latex': '\\left(A \\land B\\right) \\lor C'
  },
  {
    'ast': ['or', 'A', ['and', 'B', 'C']],
    'latex': 'A \\lor \\left(B \\land C\\right)'
  },
  {
    'ast': ['not', ['=', 'x', 1]],
    'latex': '\\lnot \\left(x = 1\\right)'
  },
  {
    'ast': ['not', ['=', 'x', 1]],
    'latex': '\\lnot \\left(x = 1\\right)'
  },
  {
    'ast': ['or', ['not', ['=', 'x', 'y']],
      ['ne', 'z', 'w']
    ],
    'latex': '\\left(\\lnot \\left(x = y\\right)\\right) \\lor \\left(z \\ne w\\right)'
  },
  {
    'ast': ['+', ['*', 1.2, 'e'],
      ['-', 3]
    ],
    'latex': '1.2 \\, e - 3'
  },
  {
    'ast': ['+', ['*', 1.2, 'e'],
      -3
    ],
    'latex': '1.2 \\, e - 3'
  },
  {
    'ast': Infinity,
    'latex': '\\infty'
  },
  {
    'ast': ['*', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j'],
    'latex': 'a \\, b \\, c \\, d \\, e \\, f \\, g \\, h \\, i \\, j'
  },
  {
    'ast': 2,
    'latex': '2'
  },

  {
    'ast': '',
    'latex': ''
  },
  {
    'ast':  ['matrix', ['tuple', 2, 2], ['tuple', ['tuple', 'a', 'b'], ['tuple', 'c', 'd']]],
    'latex': '\\begin{bmatrix} a & b \\\\ c & d \\end{bmatrix}'
  },
  {
    'ast': ['matrix', ['tuple', 1, 2], ['tuple', ['tuple', ['+', 'a', ['*', 3, 'y']], ['*', 2, ['apply', 'sin', 'theta']]]]],
    'latex': '\\begin{bmatrix} a + 3 \\, y & 2 \\, \\sin\\left(\\theta\\right) \\end{bmatrix}'
  },
  {
    'ast': ['matrix', ['tuple', 2, 3], ['tuple', ['tuple', 8, 0, 0], ['tuple', 1, 2, 3]]],
    'latex': '\\begin{bmatrix} 8 & 0 & 0 \\\\ 1 & 2 & 3 \\end{bmatrix}'
  },
  {
    'ast': ['derivative_leibniz', 'x', ['tuple', 't']],
    'latex': '\\frac{ dx }{ dt }',
  },
  {
    'ast': ['derivative_leibniz', ['tuple', 'x', 2], ['tuple', ['tuple', 't', 2]]],
    'latex': '\\frac{ d^{2}x }{ dt^{2} }',
  },
  {
    'ast': ['derivative_leibniz', ['tuple', 'mu', 2], ['tuple', 'tau', 'xi']],
    'latex': '\\frac{ d^{2}\\mu }{ d\\tau d\\xi }',
  },
  {
    'ast': ['derivative_leibniz', ['tuple', 'x', 3], ['tuple', 's', ['tuple', 't', 2]]],
    'latex': '\\frac{ d^{3}x }{ ds dt^{2} }',
  },
  {
    'ast': ['derivative_leibniz', ['tuple', 'x', 3], ['tuple', ['tuple', 's', 2], ['tuple', 't', 1]]],
    'latex': '\\frac{ d^{3}x }{ ds^{2} dt }',
  },
  {
    'ast': ['partial_derivative_leibniz', 'x', ['tuple', 't']],
    'latex': '\\frac{ \\partial x }{ \\partial t }',
  },
  {
    'ast': ['partial_derivative_leibniz', ['tuple', 'x', 2], ['tuple', ['tuple', 't', 2]]],
    'latex': '\\frac{ \\partial^{2}x }{ \\partial t^{2} }',
  },
  {
    'ast': ['partial_derivative_leibniz', ['tuple', 'mu', 2], ['tuple', 'tau', 'xi']],
    'latex': '\\frac{ \\partial^{2}\\mu }{ \\partial \\tau \\partial \\xi }',
  },
  {
    'ast': ['partial_derivative_leibniz', ['tuple', 'x', 3], ['tuple', 's', ['tuple', 't', 2]]],
    'latex': '\\frac{ \\partial^{3}x }{ \\partial s \\partial t^{2} }',
  },
  {
    'ast': ['partial_derivative_leibniz', ['tuple', 'x', 3], ['tuple', ['tuple', 's', 2], ['tuple', 't', 1]]],
    'latex': '\\frac{ \\partial^{3}x }{ \\partial s^{2} \\partial t }',
  },
  {
    'ast': ["*","a",["apply","abs","x"]],
    'latex': 'a \\, \\left|x\\right|',
  },
  {
    'ast': ["*",["apply","abs","a"],"b",["apply","abs","c"]],
    'latex': '\\left|a\\right| \\, b \\, \\left|c\\right|',
  },
  {
    'ast': ["apply","abs",["*","a",["apply","abs","b"],"c"]],
    'latex': '\\left|a \\, \\left|b\\right| \\, c\\right|',
  },
  {
    'ast': ['|', 'A', 'B'],
    'latex': 'A \\mid B',
  },
  {
    'ast': [':', 'A', 'B'],
    'latex': 'A : B',
  },
  {
    'ast': ['apply', 'P', ['|', ['>', 'X', 1], ['=', 'A', 'B']]],
    'latex': 'P\\left(X > 1 \\mid A = B\\right)',
  },
  {
    'ast': ['apply', 'P', [':', ['>', 'X', 1], ['=', 'A', 'B']]],
    'latex': 'P\\left(X > 1 : A = B\\right)',
  },
  {
    'ast': ['set', ['|', 'x', ['>', 'x', 0]]],
    'latex': '\\left\\{ x \\mid x > 0 \\right\\}',
  },
  {
    'ast': ['set', [':', 'x', ['>', 'x', 0]]],
    'latex': '\\left\\{ x : x > 0 \\right\\}',
  },
  {
    'ast': ['ldots'],
    'latex': '\\ldots',
  },
  {
    'ast': ['list', 1, 2, 3, ['ldots']],
    'latex': '1, 2, 3, \\ldots',
  },
  {
    'ast': ['tuple', 1, 2, 3, ['ldots']],
    'latex': '\\left( 1, 2, 3, \\ldots \\right)',
  },
  {
    'ast': ['^', ['apply', 'sqrt', 2], 3],
    'latex': '\\left(\\sqrt{2}\\right)^{3}',
  },
  {
    'ast': 0.0000000000123,
    'latex': '1.23 \\cdot 10^{-11}',
  },
  {
    'ast': ['^', 0.0000000000123, 5],
    'latex': '\\left(1.23 \\cdot 10^{-11}\\right)^{5}',
  },
  {
    'ast': ['^', -3, 'x'],
    'latex': '\\left(-3\\right)^{x}',
  },
  {
    'ast': ['^', -3, 2],
    'latex': '\\left(-3\\right)^{2}',
  },
  {
    'ast': ['^', ['-', 3], 2],
    'latex': '\\left(-3\\right)^{2}',
  },
  {
    'ast': ['apply', 're', 'x'],
    'latex': '\\Re\\left(x\\right)',
  },
  {
    'ast': ['apply', 'im', 'x'],
    'latex': '\\Im\\left(x\\right)',
  },



]


for (let objectToTest of objectsToTest) {
  test("parses " + objectToTest.ast + ' to ' + objectToTest.latex, () => {
    expect(converter.convert(objectToTest.ast)).toEqual(objectToTest.latex);
  });

}


test("matrix environment", function () {

  let converter = new astToLatex({matrixEnvironment: "pmatrix" });

  expect(converter.convert(['matrix', ['tuple', 2, 2], ['tuple', ['tuple', 'a', 'b'], ['tuple', 'c', 'd']]])).toEqual('\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}');

});


test("pad to digits", function () {

  let converter = new astToLatex({padToDigits: 5 });

  expect(converter.convert(123E28)).toEqual("1.2300 \\cdot 10^{30}")
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
  expect(converter.convert(123E-14)).toEqual("1.2300 \\cdot 10^{-12}")
  expect(converter.convert(123E-28)).toEqual("1.2300 \\cdot 10^{-26}")

  expect(converter.convert(['*', 123, ['^', 10, 28]])).toEqual("123.00 \\cdot 10^{28}")
  expect(converter.convert(['*', 123, ['^', 10, -28]])).toEqual("123.00 \\cdot 10^{-28}")
  
  expect(converter.convert(NaN)).toEqual("NaN")

});

test("pad to decimals", function () {

  let converter = new astToLatex({padToDecimals: 5 });

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
  expect(converter.convert(123E-14)).toEqual("1.23 \\cdot 10^{-12}")
  expect(converter.convert(123E-28)).toEqual("1.23 \\cdot 10^{-26}")

  expect(converter.convert(['*', 123, ['^', 10, 28]])).toEqual("123.00000 \\cdot 10^{28}")
  expect(converter.convert(['*', 123, ['^', 10, -28]])).toEqual("123.00000 \\cdot 10^{-28}")

  expect(converter.convert(NaN)).toEqual("NaN")

});
