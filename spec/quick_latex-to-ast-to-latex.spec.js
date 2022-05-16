import latexToAst from '../lib/converters/latex-to-ast';
import astToLatex from '../lib/converters/ast-to-latex';

var converter_latex_to_ast = new latexToAst();
var converter_ast_to_latex = new astToLatex();

var round_trip = input => converter_ast_to_latex.convert(
  converter_latex_to_ast.convert(input));


// Inputs that are strings should render as exactly the same string
// (other than white space changes) after one round trip to ast.
// For inputs that are arrays, the first component should render to
// be exactly as the second component (other than white space changes)
// after one round trip to ast.
var inputs = [
  '3+4',
  '3-4',
  '1+2+3',
  '0-0',
  '-x-0',
  '-0',
  ['1/2', '\\frac{1}{2}'],
  '-2',
  ['+2', '2'],
  'x+y-z+w',
  '-x-y+z-w',
  'x^{2}(x-3)',
  ['x^2(x-3)-z^3e^{2x+1}+x/(x-1)','x^{2}(x-3)-z^{3}e^{2x+1}+\\frac{x}{x-1}'],
  ['-1/x+((x-3)x)/((x-3)(x+4))','-\\frac{1}{x}+\\frac{(x-3)x}{(x-3)(x+4)}'],
  ['(x/y)/(z/w)','\\frac{\\frac{x}{y}}{\\frac{z}{w}}'],
  'x!',
  'n!',
  '17!',
  '(x+1)!',
  '(x^{2}+1)!',
  '(n+1)!',
  '(n-1)!',
  ['x_(n+1)!', 'x_{n+1}!'],
  'x^{2}',
  ['\\sin x', '\\sin(x)'],
  '\\theta',
  '\\theta^{2}',
  ['\\sin 3', '\\sin(3)'],
  ['\\cos x', '\\cos(x)'],
  ['\\cos 3', '\\cos(3)'],
  ['\\tan x', '\\tan(x)'],
  ['\\tan 3', '\\tan(3)'],
  ['\\sec x', '\\sec(x)'],
  ['\\sec 3', '\\sec(3)'],
  ['\\csc x', '\\csc(x)'],
  ['\\csc 3', '\\csc(3)'],
  ['\\arcsin x', '\\arcsin(x)'],
  ['\\arcsin 3', '\\arcsin(3)'],
  ['\\arccos x', '\\arccos(x)'],
  ['\\arccos 3', '\\arccos(3)'],
  ['\\arctan x', '\\arctan(x)'],
  ['\\arctan 3', '\\arctan(3)'],
  ['\\arccsc x', '\\arccsc(x)'],
  ['\\arccsc 3', '\\arccsc(3)'],
  ['\\arcsec x', '\\arcsec(x)'],
  ['\\arcsec 3', '\\arcsec(3)'],
  ['\\arccot x', '\\arccot(x)'],
  ['\\arccot 3', '\\arccot(3)'],
  ['\\asin x', '\\arcsin(x)'],
  ['\\log x', '\\log(x)'],
  ['\\log 3', '\\log(3)'],
  ['\\ln x', '\\ln(x)'],
  ['\\log e^{x}','\\log\\left(e^{x}\\right)'],
  ['\\log_{10} 3', '\\log_{10}(3)'],
  'e^{x}',
  '\\exp(x)',
  ['\\var{blah}(x)', '\\var{blah} x'],
  '\\sqrt{x}',
  '\\sqrt{4}',
  '\\frac{1}{\\sqrt{3}}',
  '\\frac{1}{\\sqrt{-x}}',
  '\\sin\\left(3\\,x\\right)',
  '\\sin\\left (3\\,x\\right )',  // this really gets written...
  '\\sin^{2}\\left(3\\,x\\right)',
  ['\\sin^{2}x+\\cos^{2}x','\\sin^{2}\\left(x\\right)+\\cos^{2}\\left(x\\right)'],
  ['\\frac{\\sin^{2}x}{\\cos^{2}x}','\\frac{\\sin^{2}\\left(x\\right)}{\\cos^{2}\\left(x\\right)}'],
  '\\sin^{3}\\left(x+y\\right)',
  '\\sin^{3}\\left  (x+y\\right  )',
  '\\sqrt{x+y}',
  '\\sqrt{\\sqrt{x}}',
  '\\sqrt{\\frac{1}{x+y}}',
  '\\log(-x^{2})',
  '\\left|3\\right|',
  ['\\sin\\left|x\\right|', '\\sin\\left(\\left|x\\right|\\right)'],
  ['\\left|\\sin\\left|x\\right|\\right|','\\left|\\sin\\left(\\left|x\\right|\\right)\\right|'],
  ['|\\sin||x|||','|\\sin(||x||)|'],
  '||x|+|y|+|z||',
  '|x+y < z|',
  '\\infty',
  "\\sin(x)'",
  "\\sin(x)''",
  'f(x)',
  "f'(x)",
  "f''(x)",
  "f(x)'",
  "\\sin'(x)",
  ['\\sin x', '\\sin(x)'],
  ['\\sin xy', '\\sin(x)y'],
  ['\\sin^xyz', '\\sin^{x}(y)z'],
  ['y(x)', 'yx'],
  ["y'(x)", "y'x"],
  ['x^22', 'x^{2}\\cdot2'],
  ['x^ab', 'x^{a}b'],
  ['x^y^z', 'x^{y^{z}}'],
  ['(x^y)^z','(x^{y})^{z}'],
  ['x_y_z', 'x_{y_{z}}'],
  ['(x_y)_z','(x_{y})_{z}'],
  ['x_y^z', 'x_{y}^{z}'],
  ['x^y_z', 'x^{y_{z}}'],
  ['f^2', 'f^{2}'],
  ['f^2(x)', 'f^{2}(x)'],
  ['f(x)^2', 'f(x)^{2}'],
  ['f_t', 'f_{t}'],
  ['f_t(x)', 'f_{t}(x)'],
  ['f_t^2(x)', 'f_{t}^{2}(x)'],
  ['f_t\'(x)', 'f_{t}\'(x)'],
  ['f\'^2(x)', 'f\'^{2}(x)'],
  ['f_t\'^2(x)', 'f_{t}\'^{2}(x)'],
  ['f_(s+t)\'\'^2(x)', 'f_{s+t}\'\'^{2}(x)'],
  ['f_{s+t}\'\'^2(x)', 'f_{s+t}\'\'^{2}(x)'],
  '\\sin(x)\'',
  ['x_(s+t)\'\'','x_{s+t}\'\''],
  ['(x-1-2)^2','(x-1-2)^{2}'],
  '(a,b)',
  '(a,b]',
  '[a,b)',
  '[a,b]',
  '\\{a,b\\}',
  '\\{a,b,c\\}',
  '\\{a\\}',
  '(a,b,c)',
  '[a,b,c]',
  '[a,b,c] + (a,b]',
  'a,b,c',
  'a,b',
  'x=y',
  'x=y=z',
  'x>y',
  'x \\ge y',
  'x>y>z',
  'x>y \\ge z',
  'x \\ge y>z',
  'x \\ge y \\ge z',
  'x<y',
  'x \\le y',
  'x<y<z',
  'x<y \\le z',
  'x \\le y<z',
  'x \\le y \\le z',
  'A \\cup B',
  'A \\cap B',
  'C = A \\cap B',
  ['A=1 \\land B=2', '(A=1) \\land (B=2)'],
  'A \\lor B',
  '(A \\land B) \\lor C',
  'A \\land (B \\lor C)',
  '\\lnot(A \\land B)',
  '(A \\land B) < C',
  '(\\lnot A) = B',
  '(A \\land B) > (C \\land D) > (E \\land F)',
  '(A \\land B) + (C \\land D)',
  '(A \\land B) \\cup (C \\land D)',
  '(A \\land B) \\cap (C \\land D)',
  ['x/y/z/w', '\\frac{\\frac{\\frac{x}{y}}{z}}{w}'],
  ['x(x-1)/z', '\\frac{x(x-1)}{z}'],
  ['A \\land B \\lor C', '(A \\land B) \\lor C'],
  ['A \\lor B \\land C', 'A \\lor (B \\land C)'],
  ['\\lnot A \\lor B', '(\\lnot A) \\lor B'],
  ['A=1 \\lor B=x/y', '(A=1) \\lor (B=\\frac{x}{y})'],
  'x \\in (a,b)',
  ['x \\not\\in (a,b)', 'x \\notin (a,b)'],
  '(a,b) \\ni x',
  '(a,b) \\not\\ni x',
  '(a,b) \\subset (c,d)',
  '(a,b) \\not\\subset (c,d)',
  '(a,b) \\supset (c,d)',
  '(a,b) \\not\\supset (c,d)',
  ['\\begin{pmatrix}a&b\\\\c&d\\end{pmatrix}', '\\begin{bmatrix}a&b\\\\c&d\\end{bmatrix}'],
  ['\\begin{matrix} \\\\ 1 \\\\ &2 \\\\ &&3 \\end{matrix}', '\\begin{bmatrix}0&0&0\\\\1&0&0\\\\0&2&0\\\\0&0&3\\end{bmatrix}'],
  ['\\begin{matrix} 1 \\\\ &2 \\\\ &&3& \\end{matrix}', '\\begin{bmatrix}1&0&0&0\\\\0&2&0&0\\\\0&0&3&0\\end{bmatrix}'],
  '\\frac{dx}{dt}',
  '\\frac{d x}{d t}',
  ['\\frac{d^2x}{dt^2}', '\\frac{d^{2}x}{dt^{2}}'],
  ['\\frac{d^2 x}{d t^2}','\\frac{d^{2} x}{d t^{2}}'],
  'a |x|',
  ['|a|b|c|', '\\left|a\\right| b \\left|c\\right|'],
  '\\left|a \\left|b\\right| c\\right|',
  ['A | B', 'A \\mid B'],
  'A : B',
  ['A > B | C and D', 'A > B \\mid C and D'],
  'A or B : C < D',
  ['\\{ x_t | t \\in Z \\}', '\\{ x_{t} \\mid t \\in Z \\}'],
];

function clean(text) {
  return text
    .replace(/\\left/g,'')
    .replace(/\\right/g,'')
    .replace(/\\,/g,'')
    .replace(/ /g,'');
}

inputs.forEach(function(input) {
  test(input.toString(), function () {
    if(Array.isArray(input))
      expect(clean(round_trip(input[0]))).toEqual(clean(input[1]));
    else
      expect(clean(round_trip(input))).toEqual(clean(input));
  });

});


// Additional round trips to ast should not alter the strings at all
inputs.forEach(function(input) {
  test(input.toString(), function () {
    if(Array.isArray(input))
      expect(round_trip(round_trip(input[0]))).toEqual(round_trip(input[0]));
    else
      expect(round_trip(round_trip(input))).toEqual(round_trip(input));
  });

});
