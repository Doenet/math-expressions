import astToGuppy from '../lib/converters/ast-to-guppy';

var converter = new astToGuppy();

const objectsToTest = [
  {
    'ast': ['*', ['/', 1, 2], 'x'],
    'guppy': "<m><e></e><f type=\"bracket\" group=\"functions\"><b p=\"latex\">\\left(<r ref=\"1\"/>\\right)</b><b p=\"text\">(<r ref=\"1\"/>)</b><c delete=\"1\" is_bracket=\"yes\"><e></e><f type=\"fraction\" group=\"functions\"><b p=\"latex\">\\dfrac{<r ref=\"1\"/>}{<r ref=\"2\"/>}</b><b p=\"small_latex\">\\frac{<r ref=\"1\"/>}{<r ref=\"2\"/>}</b><b p=\"text\">(<r ref=\"1\"/>)/(<r ref=\"2\"/>)</b><c up=\"1\" down=\"2\" name=\"numerator\"><e>1</e></c><c up=\"1\" down=\"2\" name=\"denominator\"><e>2</e></c></f><e></e></c></f><f type=\"*\" group=\"operations\" c=\"yes\"><b p=\"latex\">\\cdot</b><b p=\"text\">*</b></f><e>x</e></m>"
  },
  {
    'ast': ['+', 1, 'x', 3],
    'guppy': '<m><e>1+x+3</e></m>'
  },
  {
    'ast': ['+', 1, ['-', 'x'],
      ['-', 3]
    ],
    'guppy': '<m><e>1+</e><f type=\"bracket\" group=\"functions\"><b p=\"latex\">\\left(<r ref=\"1\"/>\\right)</b><b p=\"text\">(<r ref=\"1\"/>)</b><c delete=\"1\" is_bracket=\"yes\"><e>-<e>x</e></e></c></f><e>+</e><f type=\"bracket\" group=\"functions\"><b p=\"latex\">\\left(<r ref=\"1\"/>\\right)</b><b p=\"text\">(<r ref=\"1\"/>)</b><c delete=\"1\" is_bracket=\"yes\"><e>-<e>3</e></e></c></f><e></e></m>'
  },
  {
    'ast': ['+', 1, ['-', ['-', 'x']]],
    'guppy': '<m><e>1+</e><f type=\"bracket\" group=\"functions\"><b p=\"latex\">\\left(<r ref=\"1\"/>\\right)</b><b p=\"text\">(<r ref=\"1\"/>)</b><c delete=\"1\" is_bracket=\"yes\"><e>-<f type=\"bracket\" group=\"functions\"><b p=\"latex\">\\left(<r ref=\"1\"/>\\right)</b><b p=\"text\">(<r ref=\"1\"/>)</b><c delete=\"1\" is_bracket=\"yes\"><e>-<e>x</e></e></c></f></e></c></f><e></e></m>'
  },
  // {
  //   'ast': ['apply', 'log', 'x'],
  //   'guppy': '\\log\\left(x\\right)'
  // },
  // {
  //   'ast': ['apply', 'ln', 'x'],
  //   'guppy': '\\ln\\left(x\\right)'
  // },
  // {
  //   'ast': ['apply', 'abs', 'x'],
  //   'guppy': '\\left|x\\right|'
  // },
  // {
  //   'ast': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
  //   'guppy': '\\left|\\sin\\left(\\left|x\\right|\\right)\\right|'
  // },
  // {
  //   'ast': ['^', 'x', 2],
  //   'guppy': 'x^{2}'
  // },
  // {
  //   'ast': ['-', ['^', 'x', 2]],
  //   'guppy': '- x^{2}'
  // },
  // {
  //   'ast': ['^', 'x', 47],
  //   'guppy': 'x^{47}'
  // },
  // {
  //   'ast': ['*', ['^', 'x', 'a'], 'b'],
  //   'guppy': 'x^{a} \\, b'
  // },
  // {
  //   'ast': ['^', 'x', ['apply', 'factorial', 'a']],
  //   'guppy': 'x^{a!}'
  // },
  // {
  //   'ast': ['*', 'x', 'y', 'z'],
  //   'guppy': 'x \\, y \\, z'
  // },
  // {
  //   'ast': ['*', 'c', ['+', 'a', 'b']],
  //   'guppy': 'c \\, \\left(a + b\\right)'
  // },
  // {
  //   'ast': ['*', ['+', 'a', 'b'], 'c'],
  //   'guppy': '\\left(a + b\\right) \\, c'
  // },
  // {
  //   'ast': ['apply', 'factorial', 'a'],
  //   'guppy': 'a!'
  // },
  // {
  //   'ast': 'theta',
  //   'guppy': '\\theta'
  // },
  // {
  //   'ast': ['*', 't', 'h', 'e', 't', 'a'],
  //   'guppy': 't \\, h \\, e \\, t \\, a'
  // },
  // {
  //   'ast': ['apply', 'cos', 'theta'],
  //   'guppy': '\\cos\\left(\\theta\\right)'
  // },
  // {
  //   'ast': ['*', 'c', 'o', 's', 'x'],
  //   'guppy': 'c \\, o \\, s \\, x'
  // },
  // {
  //   'ast': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
  //   'guppy': '\\left|\\sin\\left(\\left|x\\right|\\right)\\right|'
  // },
  // {
  //   'ast': ['*', 'blah', 'x'],
  //   'guppy': '\\var{blah} \\, x'
  // },
  // {
  //   'ast': ['apply', 'abs', ['=', ['+', 'x', 3], 2]],
  //   'guppy': '\\left|x + 3 = 2\\right|'
  // },
  // {
  //   'ast': ['_', 'x', ['_', 'y', 'z']],
  //   'guppy': 'x_{y_{z}}'
  // },
  // {
  //   'ast': ['_', 'x', ['_', 'y', 'z']],
  //   'guppy': 'x_{y_{z}}'
  // },
  // {
  //   'ast': ['_', ['_', 'x', 'y'], 'z'],
  //   'guppy': '\\left(x_{y}\\right)_{z}'
  // },
  // {
  //   'ast': ['^', 'x', ['^', 'y', 'z']],
  //   'guppy': 'x^{y^{z}}'
  // },
  // {
  //   'ast': ['^', 'x', ['^', 'y', 'z']],
  //   'guppy': 'x^{y^{z}}'
  // },
  // {
  //   'ast': ['^', ['^', 'x', 'y'], 'z'],
  //   'guppy': '\\left(x^{y}\\right)^{z}'
  // },
  // {
  //   'ast': ['^', 'x', ['_', 'y', 'z']],
  //   'guppy': 'x^{y_{z}}'
  // },
  // {
  //   'ast': ['^', ['_', 'x', 'y'], 'z'],
  //   'guppy': 'x_{y}^{z}'
  // },
  // {
  //   'ast': ['*', 'x', 'y', ['apply', 'factorial', 'z']],
  //   'guppy': 'x \\, y \\, z!'
  // },
  // {
  //   'ast': 'x',
  //   'guppy': 'x'
  // },
  // {
  //   'ast': 'f',
  //   'guppy': 'f'
  // },
  // {
  //   'ast': ['*', 'f', 'g'],
  //   'guppy': 'f \\, g'
  // },
  // {
  //   'ast': ['+', 'f', 'g'],
  //   'guppy': 'f + g'
  // },
  // {
  //   'ast': ['apply', 'f', 'x'],
  //   'guppy': 'f\\left(x\\right)'
  // },
  // {
  //   'ast': ['apply', 'f', ['tuple', 'x', 'y', 'z']],
  //   'guppy': 'f\\left( x, y, z \\right)'
  // },
  // {
  //   'ast': ['*', 'f', ['apply', 'g', 'x']],
  //   'guppy': 'f \\, g\\left(x\\right)'
  // },
  // {
  //   'ast': ['*', 'f', 'p', 'x'],
  //   'guppy': 'f \\, p \\, x'
  // },
  // {
  //   'ast': ['*', 'f', 'x'],
  //   'guppy': 'f \\, x'
  // },
  // {
  //   'ast': ['prime', 'f'],
  //   'guppy': "f'"
  // },
  // {
  //   'ast': ['*', 'f', ['prime', 'g']],
  //   'guppy': "f \\, g'"
  // },
  // {
  //   'ast': ['*', ['prime', 'f'], 'g'],
  //   'guppy': "f' \\, g"
  // },
  // {
  //   'ast': ['*', ['prime', 'f'],
  //     ['prime', ['prime', 'g']]
  //   ],
  //   'guppy': "f' \\, g''"
  // },
  // {
  //   'ast': ['prime', 'x'],
  //   'guppy': "x'"
  // },
  // {
  //   'ast': ['apply', ['prime', 'f'], 'x'],
  //   'guppy': "f'\\left(x\\right)"
  // },
  // {
  //   'ast': ['prime', ['apply', 'f', 'x']],
  //   'guppy': "f\\left(x\\right)'"
  // },
  // {
  //   'ast': ['prime', ['apply', 'sin', 'x']],
  //   'guppy': "\\sin\\left(x\\right)'"
  // },
  // {
  //   'ast': ['apply', ['prime', 'sin'], 'x'],
  //   'guppy': "\\sin'\\left(x\\right)"
  // },
  // {
  //   'ast': ['apply', ['prime', ['prime', 'f']], 'x'],
  //   'guppy': "f''\\left(x\\right)"
  // },
  // {
  //   'ast': ['prime', ['prime', ['apply', 'sin', 'x']]],
  //   'guppy': "\\sin\\left(x\\right)''"
  // },
  // {
  //   'ast': ['^', ['apply', 'f', 'x'],
  //     ['_', 't', 'y']
  //   ],
  //   'guppy': 'f\\left(x\\right)^{t_{y}}'
  // },
  // {
  //   'ast': ['apply', ['_', 'f', 't'], 'x'],
  //   'guppy': 'f_{t}\\left(x\\right)'
  // },
  // {
  //   'ast': ['_', ['apply', 'f', 'x'], 't'],
  //   'guppy': 'f\\left(x\\right)_{t}'
  // },
  // {
  //   'ast': ['apply', ['^', 'f', 2], 'x'],
  //   'guppy': 'f^{2}\\left(x\\right)'
  // },
  // {
  //   'ast': ['^', ['apply', 'f', 'x'], 2],
  //   'guppy': 'f\\left(x\\right)^{2}'
  // },
  // {
  //   'ast': ['apply', ['^', ['prime', 'f'], 'a'], 'x'],
  //   'guppy': "f'^{a}\\left(x\\right)"
  // },
  // {
  //   'ast': ['apply', ['^', 'f', ['prime', 'a']], 'x'],
  //   'guppy': "f^{a'}\\left(x\\right)"
  // },
  // {
  //   'ast': ['apply', ['^', ['_', 'f', 'a'],
  //     ['prime', 'b']
  //   ], 'x'],
  //   'guppy': "f_{a}^{b'}\\left(x\\right)"
  // },
  // {
  //   'ast': ['apply', ['^', ['prime', ['_', 'f', 'a']], 'b'], 'x'],
  //   'guppy': "f_{a}'^{b}\\left(x\\right)"
  // },
  // {
  //   'ast': ['apply', 'sin', 'x'],
  //   'guppy': '\\sin\\left(x\\right)'
  // },
  // {
  //   'ast': ['*', ['apply', ['^', 'sin', 'x'], 'y'], 'z'],
  //   'guppy': '\\sin^{x}\\left(y\\right) \\, z'
  // },
  // {
  //   'ast': ['*', ['apply', 'sin', 'x'], 'y'],
  //   'guppy': '\\sin\\left(x\\right) \\, y'
  // },
  // {
  //   'ast': ['apply', ['^', 'sin', 2], 'x'],
  //   'guppy': '\\sin^{2}\\left(x\\right)'
  // },
  // {
  //   'ast': ['apply', 'exp', 'x'],
  //   'guppy': '\\exp\\left(x\\right)'
  // },
  // {
  //   'ast': ['^', 'e', 'x'],
  //   'guppy': 'e^{x}'
  // },
  // {
  //   'ast': ['^', 'x', ['apply', 'factorial', 2]],
  //   'guppy': 'x^{2!}'
  // },
  // {
  //   'ast': ['^', 'x', ['apply', 'factorial', ['apply', 'factorial', 2]]],
  //   'guppy': 'x^{2!!}'
  // },
  // {
  //   'ast': ['^', ['_', 'x', 't'], 2],
  //   'guppy': 'x_{t}^{2}'
  // },
  // {
  //   'ast': ['_', 'x', ['^', 'f', 2]],
  //   'guppy': 'x_{f^{2}}'
  // },
  // {
  //   'ast': ['prime', ['_', 'x', 't']],
  //   'guppy': "x_{t}'"
  // },
  // {
  //   'ast': ['_', 'x', ['prime', 'f']],
  //   'guppy': "x_{f'}"
  // },
  // {
  //   'ast': ['tuple', 'x', 'y', 'z'],
  //   'guppy': '\\left( x, y, z \\right)'
  // },
  // {
  //   'ast': ['+', ['tuple', 'x', 'y'],
  //     ['-', ['array', 'x', 'y']]
  //   ],
  //   'guppy': '\\left( x, y \\right) - \\left[ x, y \\right]'
  // },
  // {
  //   'ast': ['*', 2, ['+', 'z', ['-', ['+', 'x', 1]]]],
  //   'guppy': '2 \\, \\left(z - \\left(x + 1\\right)\\right)'
  // },
  // {
  //   'ast': ['set', 1, 2, 'x'],
  //   'guppy': '\\left\\{ 1, 2, x \\right\\}'
  // },
  // {
  //   'ast': ['set', 'x', 'x'],
  //   'guppy': '\\left\\{ x, x \\right\\}'
  // },
  // {
  //   'ast': ['set', 'x'],
  //   'guppy': '\\left\\{ x \\right\\}'
  // },
  // {
  //   'ast': ['interval', ['tuple', 1, 2],
  //     ['tuple', false, true]
  //   ],
  //   'guppy': '\\left( 1, 2 \\right]'
  // },
  // {
  //   'ast': ['array', 1, 2],
  //   'guppy': '\\left[ 1, 2 \\right]'
  // },
  // {
  //   'ast': ['tuple', 1, 2],
  //   'guppy': '\\left( 1, 2 \\right)'
  // },
  // {
  //   'ast': ['vector', 1, 2],
  //   'guppy': '\\left( 1, 2 \\right)'
  // },
  // {
  //   'ast': ['list', 1, 2, 3],
  //   'guppy': '1, 2, 3'
  // },
  // {
  //   'ast': ['=', 'x', 'a'],
  //   'guppy': 'x = a'
  // },
  // {
  //   'ast': ['=', 'x', 'y', 1],
  //   'guppy': 'x = y = 1'
  // },
  // {
  //   'ast': ['=', 'x', ['=', 'y', 1]],
  //   'guppy': 'x = \\left(y = 1\\right)'
  // },
  // {
  //   'ast': ['=', ['=', 'x', 'y'], 1],
  //   'guppy': '\\left(x = y\\right) = 1'
  // },
  // {
  //   'ast': ['ne', 7, 2],
  //   'guppy': '7 \\ne 2'
  // },
  // {
  //   'ast': ['not', ['=', 'x', 'y']],
  //   'guppy': '\\lnot \\left(x = y\\right)'
  // },
  // {
  //   'ast': ['not', ['=', 'x', 'y']],
  //   'guppy': '\\lnot \\left(x = y\\right)'
  // },
  // {
  //   'ast': ['>', 'x', 'y'],
  //   'guppy': 'x > y'
  // },
  // {
  //   'ast': ['ge', 'x', 'y'],
  //   'guppy': 'x \\ge y'
  // },
  // {
  //   'ast': ['gts', ['tuple', 'x', 'y', 'z'],
  //     ['tuple', true, true]
  //   ],
  //   'guppy': 'x > y > z'
  // },
  // {
  //   'ast': ['gts', ['tuple', 'x', 'y', 'z'],
  //     ['tuple', true, false]
  //   ],
  //   'guppy': 'x > y \\ge z'
  // },
  // {
  //   'ast': ['gts', ['tuple', 'x', 'y', 'z'],
  //     ['tuple', false, true]
  //   ],
  //   'guppy': 'x \\ge y > z'
  // },
  // {
  //   'ast': ['gts', ['tuple', 'x', 'y', 'z'],
  //     ['tuple', false, false]
  //   ],
  //   'guppy': 'x \\ge y \\ge z'
  // },
  // {
  //   'ast': ['<', 'x', 'y'],
  //   'guppy': 'x < y'
  // },
  // {
  //   'ast': ['le', 'x', 'y'],
  //   'guppy': 'x \\le y'
  // },
  // {
  //   'ast': ['lts', ['tuple', 'x', 'y', 'z'],
  //     ['tuple', true, true]
  //   ],
  //   'guppy': 'x < y < z'
  // },
  // {
  //   'ast': ['lts', ['tuple', 'x', 'y', 'z'],
  //     ['tuple', true, false]
  //   ],
  //   'guppy': 'x < y \\le z'
  // },
  // {
  //   'ast': ['lts', ['tuple', 'x', 'y', 'z'],
  //     ['tuple', false, true]
  //   ],
  //   'guppy': 'x \\le y < z'
  // },
  // {
  //   'ast': ['lts', ['tuple', 'x', 'y', 'z'],
  //     ['tuple', false, false]
  //   ],
  //   'guppy': 'x \\le y \\le z'
  // },
  // {
  //   'ast': ['>', ['<', 'x', 'y'], 'z'],
  //   'guppy': '\\left(x < y\\right) > z'
  // },
  // {
  //   'ast': ['subset', 'A', 'B'],
  //   'guppy': 'A \\subset B'
  // },
  // {
  //   'ast': ['notsubset', 'A', 'B'],
  //   'guppy': 'A \\not\\subset B'
  // },
  // {
  //   'ast': ['superset', 'A', 'B'],
  //   'guppy': 'A \\supset B'
  // },
  // {
  //   'ast': ['notsuperset', 'A', 'B'],
  //   'guppy': 'A \\not\\supset B'
  // },
  // {
  //   'ast': ['in', 'x', 'A'],
  //   'guppy': 'x \\in A'
  // },
  // {
  //   'ast': ['notin', 'x', 'A'],
  //   'guppy': 'x \\notin A'
  // },
  // {
  //   'ast': ['ni', 'A', 'x'],
  //   'guppy': 'A \\ni x'
  // },
  // {
  //   'ast': ['notni', 'A', 'x'],
  //   'guppy': 'A \\not\\ni x'
  // },
  // {
  //   'ast': ['union', 'A', 'B'],
  //   'guppy': 'A \\cup B'
  // },
  // {
  //   'ast': ['intersect', 'A', 'B'],
  //   'guppy': 'A \\cap B'
  // },
  // {
  //   'ast': ['and', 'A', 'B'],
  //   'guppy': 'A \\land B'
  // },
  // {
  //   'ast': ['and', 'A', 'B'],
  //   'guppy': 'A \\land B'
  // },
  // {
  //   'ast': ['or', 'A', 'B'],
  //   'guppy': 'A \\lor B'
  // },
  // {
  //   'ast': ['or', 'A', 'B'],
  //   'guppy': 'A \\lor B'
  // },
  // {
  //   'ast': ['and', 'A', 'B', 'C'],
  //   'guppy': 'A \\land B \\land C'
  // },
  // {
  //   'ast': ['or', 'A', 'B', 'C'],
  //   'guppy': 'A \\lor B \\lor C'
  // },
  // {
  //   'ast': ['or', ['and', 'A', 'B'], 'C'],
  //   'guppy': '\\left(A \\land B\\right) \\lor C'
  // },
  // {
  //   'ast': ['or', 'A', ['and', 'B', 'C']],
  //   'guppy': 'A \\lor \\left(B \\land C\\right)'
  // },
  // {
  //   'ast': ['not', ['=', 'x', 1]],
  //   'guppy': '\\lnot \\left(x = 1\\right)'
  // },
  // {
  //   'ast': ['not', ['=', 'x', 1]],
  //   'guppy': '\\lnot \\left(x = 1\\right)'
  // },
  // {
  //   'ast': ['or', ['not', ['=', 'x', 'y']],
  //     ['ne', 'z', 'w']
  //   ],
  //   'guppy': '\\left(\\lnot \\left(x = y\\right)\\right) \\lor \\left(z \\ne w\\right)'
  // },
  // {
  //   'ast': ['+', ['*', 1.2, 'e'],
  //     ['-', 3]
  //   ],
  //   'guppy': '1.2 \\, e - 3'
  // },
  // {
  //   'ast': Infinity,
  //   'guppy': '\\infty'
  // },
  // {
  //   'ast': ['*', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j'],
  //   'guppy': 'a \\, b \\, c \\, d \\, e \\, f \\, g \\, h \\, i \\, j'
  // },
  // {
  //   'ast': 2,
  //   'guppy': 2
  // },
  //
  // {
  //   'ast': '',
  //   'guppy': ''
  // },
  // {
  //   'ast':  ['matrix', ['tuple', 2, 2], ['tuple', ['tuple', 'a', 'b'], ['tuple', 'c', 'd']]],
  //   'guppy': '\\begin{bmatrix} a & b \\\\ c & d \\end{bmatrix}'
  // },
  // {
  //   'ast': ['matrix', ['tuple', 1, 2], ['tuple', ['tuple', ['+', 'a', ['*', 3, 'y']], ['*', 2, ['apply', 'sin', 'theta']]]]],
  //   'guppy': '\\begin{bmatrix} a + 3 \\, y & 2 \\, \\sin\\left(\\theta\\right) \\end{bmatrix}'
  // },
  // {
  //   'ast': ['matrix', ['tuple', 2, 3], ['tuple', ['tuple', 8, 0, 0], ['tuple', 1, 2, 3]]],
  //   'guppy': '\\begin{bmatrix} 8 & 0 & 0 \\\\ 1 & 2 & 3 \\end{bmatrix}'
  // },
  //   {
  //   'ast': ['derivative_leibniz', 'x', 't'],
  //   'guppy': '\\frac{dx}{dt}',
  // },
  // {
  //   'ast': ['derivative_leibniz_mult', 2, 'x', 't'],
  //   'guppy': '\\frac{d^2x}{dt^2}',
  // },


]


for (let objectToTest of objectsToTest) {
  test("parses " + objectToTest.ast + ' to ' + objectToTest.guppy, () => {
    expect(converter.convert(objectToTest.ast)).toEqual(objectToTest.guppy);
  });

}


// test("matrix environment", function () {
//
//   let converter = new astToLatex({matrixEnvironment: "pmatrix" });
//
//   expect(converter.convert(['matrix', ['tuple', 2, 2], ['tuple', ['tuple', 'a', 'b'], ['tuple', 'c', 'd']]])).toEqual('\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}');
//
// });
