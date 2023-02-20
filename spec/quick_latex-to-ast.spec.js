import latexToAst from '../lib/converters/latex-to-ast';
import { ParseError } from '../lib/converters/error';

var converter = new latexToAst();

var trees = {
  
  '\\frac{1}{2} x': ['*',['/',1,2],'x'],
  '1+x+3': ['+',1,'x',3],
  '1-x-3': ['+',1,['-','x'],-3],
  "1 + - x": ['+',1,['-','x']],
  "1 - - x": ['+',1,['-',['-','x']]],
  '1.+x+3.0': ['+',1,'x',3],
  '1-3': ['+',1,-3],
  '1-0': ['+',1,['-',0]],
  'x^2': ['^', 'x', 2],
  '\\log x': ['apply', 'log', 'x'],
  '\\ln x': ['apply', 'ln', 'x'],
  '\\log_{10} x': ['apply', 'log10', 'x'],
  '-x^2': ['-',['^', 'x', 2]],
  '|x|': ['apply', 'abs','x'],
  '|\\sin|x||': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
  'x^47': ['*', ['^', 'x', 4], 7],
  'x^ab': ['*', ['^', 'x', 'a'], 'b'],
  'x^a3': ['*', ['^', 'x', 'a'], 3],
  'x^a!':  ['^', 'x', ['apply', 'factorial', 'a']],
  'f^47': ['*', ['^', 'f', 4], 7],
  'f^ab': ['*', ['^', 'f', 'a'], 'b'],
  'f^a3': ['*', ['^', 'f', 'a'], 3],
  'x_47': ['*', ['_', 'x', 4], 7],
  'x_ab': ['*', ['_', 'x', 'a'], 'b'],
  'x_a3': ['*', ['_', 'x', 'a'], 3],
  'f_47': ['*', ['_', 'f', 4], 7],
  'f_ab': ['*', ['_', 'f', 'a'], 'b'],
  'f_a3': ['*', ['_', 'f', 'a'], 3],
  'xyz': ['*','x','y','z'],
  'c(a+b)': ['*', 'c', ['+', 'a', 'b']],
  '(a+b)c': ['*', ['+', 'a', 'b'], 'c'],
  'a!': ['apply', 'factorial','a'],
  '\\theta': 'theta',
  'theta': ['*', 't', 'h', 'e', 't', 'a'],
  '\\cos(\\theta)': ['apply', 'cos','theta'],
  'cos(x)': ['*', 'c', 'o', 's', 'x'],
  '|\\sin(|x|)|': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
  '\\operatorname{blah}(x)': ['*', 'blah', 'x'],
  '|x+3=2|': ['apply', 'abs', ['=', ['+', 'x', 3], 2]],
  'x_y_z': ['_', 'x', ['_','y','z']],
  'x_{y_z}': ['_', 'x', ['_','y','z']],
  '{x_y}_z': ['_', ['_', 'x', 'y'],'z'],
  'x^y^z': ['^', 'x', ['^','y','z']],
  'x^{y^z}': ['^', 'x', ['^','y','z']],
  '{x^y}^z': ['^', ['^', 'x', 'y'],'z'],
  'x^y_z': ['^', 'x', ['_','y','z']],
  'x_y^z': ['^', ['_','x','y'],'z'],
  'xyz!': ['*','x','y', ['apply', 'factorial', 'z']],
  'x': 'x',
  'f': 'f',
  'fg': ['*', 'f','g'],
  'f+g': ['+', 'f', 'g'],
  'f(x)': ['apply', 'f', 'x'],
  'f(x,y,z)': ['apply', 'f', ['tuple', 'x', 'y', 'z']],
  'fg(x)': ['*', 'f', ['apply', 'g', 'x']],
  'fp(x)': ['*', 'f', 'p', 'x'],
  'fx': ['*', 'f', 'x'],
  'f\'': ['prime', 'f'],
  'fg\'': ['*', 'f', ['prime', 'g']],
  'f\'g': ['*', ['prime', 'f'], 'g'],
  'f\'g\'\'': ['*', ['prime', 'f'], ['prime', ['prime', 'g']]],
  'x\'': ['prime', 'x'],
  'f\'(x)' : ['apply', ['prime', 'f'], 'x'],
  'f(x)\'' : ['prime', ['apply', 'f', 'x']],
  '\\sin(x)\'': ['prime', ['apply', 'sin', 'x']],
  '\\sin\'(x)': ['apply', ['prime', 'sin'], 'x'],
  'f\'\'(x)': ['apply', ['prime', ['prime', 'f']],'x'],
  '\\sin(x)\'\'': ['prime', ['prime', ['apply','sin','x']]],
  'f(x)^t_y': ['^', ['apply', 'f','x'], ['_','t','y']],
  'f_t(x)': ['apply', ['_', 'f', 't'], 'x'],
  'f(x)_t': ['_', ['apply', 'f', 'x'], 't'],
  'f^2(x)': ['apply', ['^', 'f', 2], 'x'],
  'f(x)^2': ['^', ['apply', 'f', 'x'],2],
  'f\'^a(x)': ['apply', ['^', ['prime', 'f'], 'a'], 'x'],
  'f^a\'(x)': ['apply', ['^', 'f', ['prime', 'a']], 'x'],
  'f_a^b\'(x)': ['apply', ['^', ['_', 'f', 'a'], ['prime', 'b']],'x'],
  'f_a\'^b(x)': ['apply', ['^', ['prime', ['_', 'f','a']],'b'],'x'],
  '\\sin x': ['apply', 'sin', 'x'],
  'f x': ['*', 'f', 'x'],
  '\\sin^xyz': ['*', ['apply', ['^', 'sin', 'x'], 'y'], 'z'],
  '\\sin xy': ['*', ['apply', 'sin', 'x'], 'y'],
  '\\sin^2(x)': ['apply', ['^', 'sin', 2], 'x'],
  '\\exp(x)': ['apply', 'exp', 'x'],
  'e^x': ['^', 'e', 'x'],
  'x^2!': ['^', 'x', ['apply', 'factorial', 2]],
  'x^2!!': ['^', 'x', ['apply', 'factorial', ['apply', 'factorial', 2]]],
  'x_t^2': ['^', ['_', 'x', 't'], 2],
  'x_f^2': ['_', 'x', ['^', 'f', 2]],
  'x_t\'': ['prime', ['_', 'x', 't']],
  'x_f\'': ['_', 'x', ['prime', 'f']],
  '(x,y,z)': ['tuple', 'x', 'y', 'z'],
  '(x,y)-[x,y]': ['+', ['tuple','x','y'], ['-', ['array','x','y']]],
  '2[z-(x+1)]': ['*', 2, ['+', 'z', ['-', ['+', 'x', 1]]]],
  '\\{1,2,x\\}': ['set', 1, 2, 'x'],
  '\\{x, x\\}': ['set', 'x', 'x'],
  '\\{x\\}': ['set', 'x'],
  '\\{-x\\}': ['set', ['-','x']],
  '(1,2]': ['interval', ['tuple', 1, 2], ['tuple', false, true]],
  '[1,2)': ['interval', ['tuple', 1, 2], ['tuple', true, false]],
  '[1,2]': ['array', 1, 2 ],
  '(1,2)': ['tuple', 1, 2 ],
  '1,2,3': ['list', 1, 2, 3],
  'x=a': ['=', 'x', 'a'],
  'x=y=1': ['=', 'x', 'y', 1],
  'x=(y=1)': ['=', 'x', ['=', 'y', 1]],
  '(x=y)=1': ['=', ['=','x', 'y'], 1],
  '7 \\ne 2': ['ne', 7, 2],
  '7 \\neq 2': ['ne', 7, 2],
  '\\lnot x=y': ['not', ['=', 'x', 'y']],
  '\\lnot (x=y)': ['not', ['=', 'x', 'y']],
  'x>y': ['>', 'x','y'],
  'x \\gt y': ['>', 'x','y'],
  'x \\ge y': ['ge', 'x','y'],
  'x \\geq y': ['ge', 'x','y'],
  'x>y>z': ['gts', ['tuple', 'x', 'y','z'], ['tuple', true, true]],
  'x>y \\ge z': ['gts', ['tuple', 'x', 'y','z'], ['tuple', true, false]],
  'x \\ge y>z': ['gts', ['tuple', 'x', 'y','z'], ['tuple', false, true]],
  'x \\ge y \\ge z': ['gts', ['tuple', 'x', 'y','z'], ['tuple', false, false]],
  'x<y': ['<', 'x','y'],
  'x \\lt y': ['<', 'x','y'],
  'x \\le y': ['le', 'x','y'],
  'x \\leq y': ['le', 'x','y'],
  'x<y<z': ['lts', ['tuple', 'x', 'y','z'], ['tuple', true, true]],
  'x<y \\le z': ['lts', ['tuple', 'x', 'y','z'], ['tuple', true, false]],
  'x \\le y<z': ['lts', ['tuple', 'x', 'y', 'z'], ['tuple', false, true]],
  'x \\le y \\le z': ['lts', ['tuple', 'x', 'y', 'z'], ['tuple', false, false]],
  'x<y>z': ['>', ['<', 'x', 'y'], 'z'],
  'A \\subset B': ['subset', 'A', 'B'],
  'A \\not\\subset B': ['notsubset', 'A', 'B'],
  'A \\supset B': ['superset', 'A', 'B'],
  'A \\not\\supset B': ['notsuperset', 'A', 'B'],
  'x \\in A': ['in', 'x', 'A'],
  'x \\notin A': ['notin', 'x', 'A'],
  'x \\not\\in A': ['notin', 'x', 'A'],
  'A \\ni x': ['ni', 'A', 'x'],
  'A \\not\\ni x': ['notni', 'A', 'x'],
  'A \\cup B': ['union', 'A', 'B'],
  'A \\cap B': ['intersect', 'A', 'B'],
  'A \\land B': ['and', 'A', 'B'],
  'A \\wedge B': ['and', 'A', 'B'],
  'A \\lor B': ['or', 'A', 'B'],
  'A \\vee B': ['or', 'A', 'B'],
  'A \\land B \\lor C': ['and', 'A', 'B', 'C'],
  'A \\lor B \\lor C': ['or', 'A', 'B', 'C'],
  'A \\land B \\lor C': ['or', ['and', 'A', 'B'], 'C'],
  'A \\lor B \\land C': ['or', 'A', ['and', 'B', 'C']],
  '\\lnot x=1': ['not', ['=', 'x', 1]],
  '\\lnot(x=1)': ['not', ['=', 'x', 1]],
  '\\lnot(x=y) \\lor z \\ne w': ['or', ['not', ['=','x','y']], ['ne','z','w']],
  '1.2E3': 1200,
  '1.2E+3  ': 1200,
  '3.1E-3 ': 0.0031,
  '3.1E- 3 ': ['+', ['*', 3.1, 'E'], -3],
  '3.1E -3 ': ['+', ['*', 3.1, 'E'], -3],
  '3.1E - 3 ': ['+', ['*', 3.1, 'E'], -3],
  '3.1E-3 + 2 ': ['+', ['*', 3.1, 'E'], -3, 2],
  '(3.1E-3 ) + 2': ['+', 0.0031, 2],
  '\\sin((3.1E-3)x)': ['apply', 'sin', ['*', 0.0031, 'x']],
  '\\sin( 3.1E-3 x)': ['apply', 'sin',  ['+', ['*', 3.1, 'E'], ['-', ['*', 3, 'x']]]],
  '\\frac{3.1E-3 }{x}': ['/', 0.0031, 'x'],
  '|3.1E-3|': ['apply', 'abs', 0.0031],
  '|3.1E-3|': ['apply', 'abs', 0.0031],
  '(3.1E-3, 1E2)': ['tuple', 0.0031, 100],
  '(3.1E-3, 1E2]': ["interval", ["tuple", 0.0031, 100], ["tuple", false, true]],
  '\\{ 3.1E-3, 1E2 \\}': ['set', 0.0031, 100],
  '\\begin{matrix} 1E-3 & 3E-12 \\\\ 6E+3& 7E5\\end{matrix}': [ 'matrix', [ 'tuple', 2, 2 ], [ 'tuple', [ 'tuple', 0.001, 3e-12 ], [ 'tuple', 6000, 700000 ] ] ],
  '1.2e-3': ['+', ['*', 1.2, 'e'], -3],
  '+2': ['+', 2],
  '\\infty': Infinity,
  '+\\infty': ['+', Infinity],
  'a b\\,c\\!d\\ e\\>f\\;g\\>h\\quad i \\qquad j': ['*','a','b','c','d','e','f','g','h','i','j'],
  '\\begin{bmatrix}a & b\\\\ c&d\\end{bmatrix}': ['matrix', ['tuple', 2, 2], ['tuple', ['tuple', 'a', 'b'], ['tuple', 'c', 'd']]],
  '\\begin{pmatrix}a & b\\\\ c\\end{pmatrix}': ['matrix', ['tuple', 2, 2], ['tuple', ['tuple', 'a', 'b'], ['tuple', 'c', 0]]],
  '\\begin{matrix}a & b\\\\ &d\\end{matrix}': ['matrix', ['tuple', 2, 2], ['tuple', ['tuple', 'a', 'b'], ['tuple', 0, 'd']]],
  '\\begin{pmatrix}a + 3y & 2\\sin(\\theta)\\end{pmatrix}': ['matrix', ['tuple', 1, 2], ['tuple', ['tuple', ['+', 'a', ['*', 3, 'y']], ['*', 2, ['apply', 'sin', 'theta']]]]],
  '\\begin{bmatrix}3\\\\ \\\\ 4 & 5\\end{bmatrix}': ['matrix', ['tuple', 3, 2], ['tuple', ['tuple', 3, 0], ['tuple', 0, 0], ['tuple', 4, 5]]],
  '\\begin{matrix}8\\\\1&2&3\\end{matrix}': ['matrix', ['tuple', 2, 3], ['tuple', ['tuple', 8, 0, 0], ['tuple', 1, 2, 3]]],
  '\\frac{dx}{dt}=q': ['=', ['derivative_leibniz', 'x', ['tuple', 't']], 'q'],
  '\\frac { dx } { dt } = q': ['=', ['derivative_leibniz', 'x', ['tuple', 't']], 'q'],
  '\\frac{d x}{dt}': ['derivative_leibniz', 'x', ['tuple', 't']],
  '\\frac{dx}{d t}': ['derivative_leibniz', 'x', ['tuple', 't']],
  '\\frac{dx_2}{dt}': ["/", ["*", "d", ["_", "x", 2]], ["*", "d", "t"]],
  '\\frac{dxy}{dt}': ["/", ["*", "d", "x", "y"], ["*", "d", "t"]],
  '\\frac{d^2x}{dt^2}': ['derivative_leibniz', ['tuple', 'x', 2], ['tuple', ['tuple', 't', 2]]],
  '\\frac{d^{2}x}{dt^{ 2 }}': ['derivative_leibniz', ['tuple', 'x', 2], ['tuple', ['tuple', 't', 2]]],
  '\\frac{d^2x}{dt^3}': ["/", ["*", ["^", "d", 2], "x"], ["*", "d", ["^", "t", 3]]],
  '\\frac{d^2x}{dsdt}': ['derivative_leibniz', ['tuple', 'x', 2], ['tuple', 's', 't']],
  '\\frac{d^2x}{dsdta}': ["/", ["*", ["^", "d", 2], "x"], ["*", "d", "s", "d", "t", "a"]],
  '\\frac{d^3x}{ds^2dt}': ['derivative_leibniz', ['tuple', 'x', 3], ['tuple', ['tuple', 's', 2], 't']],
  '\\frac{d^{ 3 }x}{ds^{2}dt}': ['derivative_leibniz', ['tuple', 'x', 3], ['tuple', ['tuple', 's', 2], 't']],
  '\\frac{d^3x}{dsdt^2}': ['derivative_leibniz', ['tuple', 'x', 3], ['tuple', 's', ['tuple', 't', 2]]],
  '\\frac{d^{3}x}{dsdt^{ 2 }}': ['derivative_leibniz', ['tuple', 'x', 3], ['tuple', 's', ['tuple', 't', 2]]],
  '\\frac{d\\theta}{d\\pi}': ['derivative_leibniz', 'theta', ['tuple', 'pi']],
  '\\frac{d\\operatorname{hello}}{d\\operatorname{bye}}': ['derivative_leibniz', 'hello', ['tuple', 'bye']],
  '\\frac{d^2\\theta}{d\\pi^2}': ['derivative_leibniz', ['tuple', 'theta', 2], ['tuple', ['tuple', 'pi', 2]]],
  '\\frac{d^2\\operatorname{hello}}{d\\operatorname{bye}^2}': ['derivative_leibniz', ['tuple', 'hello', 2], ['tuple', ['tuple', 'bye', 2]]],
  '\\frac{d^{2}\\theta}{d\\pi^{ 2 }}': ['derivative_leibniz', ['tuple', 'theta', 2], ['tuple', ['tuple', 'pi', 2]]],
  '\\frac{d^{ 2 }\\operatorname{hello}}{d\\operatorname{bye}^{2}}': ['derivative_leibniz', ['tuple', 'hello', 2], ['tuple', ['tuple', 'bye', 2]]],
  '\\frac{\\partial x}{\\partial t}': ['partial_derivative_leibniz', 'x', ['tuple', 't']],
  '\\frac { \\partial x } { \\partial t } = q': ['=', ['partial_derivative_leibniz', 'x', ['tuple', 't']], 'q'],
  '\\frac{\\partial x_2}{\\partial t}': ["/", ["*", "partial", ["_", "x", 2]], ["*", "partial", "t"]],
  '\\frac{\\partial xy}{\\partial t}': ["/", ["*", "partial", "x", "y"], ["*", "partial", "t"]],
  '\\frac{\\partial^2x}{\\partial t^2}': ['partial_derivative_leibniz', ['tuple', 'x', 2], ['tuple', ['tuple', 't', 2]]],
  '\\frac{\\partial^{2}x}{\\partial t^{ 2 }}': ['partial_derivative_leibniz', ['tuple', 'x', 2], ['tuple', ['tuple', 't', 2]]],
  '\\frac{\\partial ^2x}{\\partial t^3}': ["/", ["*", ["^", "partial", 2], "x"], ["*", "partial", ["^", "t", 3]]],
  '\\frac{\\partial ^2x}{\\partial s\\partial t}': ['partial_derivative_leibniz', ['tuple', 'x', 2], ['tuple', 's', 't']],
  '\\frac{\\partial ^2x}{\\partial s\\partial ta}': ["/", ["*", ["^", "partial", 2], "x"], ["*", "partial", "s", "partial", "t", "a"]],
  '\\frac{\\partial ^3x}{\\partial s^2\\partial t}': ['partial_derivative_leibniz', ['tuple', 'x', 3], ['tuple', ['tuple', 's', 2], 't']],
  '\\frac{\\partial ^{ 3 }x}{\\partial s^{2}\\partial t}': ['partial_derivative_leibniz', ['tuple', 'x', 3], ['tuple', ['tuple', 's', 2], 't']],
  '\\frac{\\partial ^3x}{\\partial s\\partial t^2}': ['partial_derivative_leibniz', ['tuple', 'x', 3], ['tuple', 's', ['tuple', 't', 2]]],
  '\\frac{\\partial ^{3}x}{\\partial s\\partial t^{ 2 }}': ['partial_derivative_leibniz', ['tuple', 'x', 3], ['tuple', 's', ['tuple', 't', 2]]],
  '\\frac{\\partial \\theta}{\\partial \\pi}': ['partial_derivative_leibniz', 'theta', ['tuple', 'pi']],
  '\\frac{\\partial \\operatorname{hello}}{\\partial \\operatorname{bye}}': ['partial_derivative_leibniz', 'hello', ['tuple', 'bye']],
  '\\frac{\\partial ^2\\theta}{\\partial \\pi^2}': ['partial_derivative_leibniz', ['tuple', 'theta', 2], ['tuple', ['tuple', 'pi', 2]]],
  '\\frac{\\partial ^2\\operatorname{hello}}{\\partial \\operatorname{bye}^2}': ['partial_derivative_leibniz', ['tuple', 'hello', 2], ['tuple', ['tuple', 'bye', 2]]],
  '\\frac{\\partial ^{2}\\theta}{\\partial \\pi^{ 2 }}': ['partial_derivative_leibniz', ['tuple', 'theta', 2], ['tuple', ['tuple', 'pi', 2]]],
  '\\frac{\\partial ^{ 2 }\\operatorname{hello}}{\\partial \\operatorname{bye}^{2}}': ['partial_derivative_leibniz', ['tuple', 'hello', 2], ['tuple', ['tuple', 'bye', 2]]],
  '2 \\cdot 3': ['*', 2, 3],
  '2\\cdot3': ['*', 2, 3],
  '2 \\times 3': ['*', 2, 3],
  '2\\times3': ['*', 2, 3],
  '3 \\div 1': ['/', 3, 1],
  '3\\div1': ['/', 3, 1],
  '\\sin2': ['apply', 'sin', 2],
  '3|x|': ['*', 3, ['apply', 'abs', 'x']],
  '|a|b|c|': ['*',['apply', 'abs', 'a'], 'b', ['apply', 'abs', 'c']],
  '|a|*b*|c|': ['*',['apply', 'abs', 'a'], 'b', ['apply', 'abs', 'c']],
  '|a*|b|*c|': ['apply', 'abs', ['*', 'a', ['apply', 'abs', 'b'], 'c']],
  '\\left|a\\left|b\\right|c\\right|': ['apply', 'abs', ['*', 'a', ['apply', 'abs', 'b'], 'c']],
  '|a(q|b|r)c|': ['apply', 'abs', ['*', 'a', 'q', ['apply', 'abs', 'b'], 'r', 'c']],
  'r=1|x': ['|', ['=', 'r', 1], 'x'],
  '\\{ x | x > 0 \\}': ['set', ['|', 'x', ['>', 'x', 0]]],
  'r=1 \\mid x': ['|', ['=', 'r', 1], 'x'],
  '\\{ x \\mid x > 0 \\}': ['set', ['|', 'x', ['>', 'x', 0]]],
  'r=1:x': [':', ['=', 'r', 1], 'x'],
  '\\{ x : x > 0 \\}': ['set', [':', 'x', ['>', 'x', 0]]],
  '\\ldots': ['ldots'],
  '1,2,3,\\ldots': ['list', 1, 2, 3, ['ldots']],
  '(1,2,3,\\ldots)':  ['tuple', 1, 2, 3, ['ldots']],
  'a-2b': ['+', 'a', ['-', ['*', 2, 'b']]],
  'a+-2b': ['+', 'a', ['-', ['*', 2, 'b']]],
  'a+(-2b)': ['+', 'a', ['-', ['*', 2, 'b']]],
  '1++1': ["+", 1, ['+', 1]],
  '1(+1)': ["*", 1, ['+', 1]],
  '1(++1)': ["*", 1, ['+', ['+', 1]]],
  'x+++y': ['+', 'x', ['+', ['+', 'y']]],
  'x-y-': ['+', 'x', ['-', 'y'], ['-', '\uff3f']],
  '_x': ['_', '\uff3f', 'x'],
  'x_': ['_', 'x', '\uff3f'],
  '|y/v': ["|", '\uff3f', ["/", "y", "v"]],
  'x+^2': ["+", "x", ["^", '\uff3f', 2]],
  'x/\'y': ["*", ["/", "x", ["prime", '\uff3f']], "y"],
  '\\sin': ["apply", "sin", '\uff3f'],
  '\\sin+\\cos': ["apply", "sin", ['+', ["apply", "cos", '\uff3f']]],
  '\\frac{}{a}': ["/", '\uff3f', "a"],
  '\\frac{a}{}': ["/", "a", '\uff3f'],
  'C^+': ["^", "C", "+"],
  'C^-': ["^", "C", "-"],
  'C^+x': ["*", ["^", "C", "+"], "x"],
  'C^-x': ["*", ["^", "C", "-"], "x"],
  'C^+2': ["*", ["^", "C", "+"], 2],
  'C^-2': ["*", ["^", "C", "-"], 2],
  'C^{++}': ["^", "C", "++"],
  'C^{--}': ["^", "C", "--"],
  'C^{+++}': ["^", "C", "+++"],
  'C^{---}': ["^", "C", "---"],
  'C^{++++}': ["^", "C", "++++"],
  'C^{----}': ["^", "C", "----"],
  'C^{+++++}': ["^", "C", "+++++"],
  'C^{-----}': ["^", "C", "-----"],
  'C^{++++++}': ["^", "C", "++++++"],
  'C^{------}': ["^", "C", "------"],
  'C^{2+}': ["^", "C", '2+'],
  'C^{2-}': ["^", "C", '2-'],
  'C^{2++}': ["^", "C", '2++'],
  'C^{2--}': ["^", "C", '2--'],
  'C^{2+++}': ["^", "C", '2+++'],
  'C^{2---}': ["^", "C", '2---'],
  'C^{2++++}': ["^", "C", '2++++'],
  'C^{2----}': ["^", "C", '2----'],
  'C^{2+++++}': ["^", "C", '2+++++'],
  'C^{2-----}': ["^", "C", '2-----'],
  'C_+': ["_", "C", "+"],
  'C_-': ["_", "C", "-"],
  'C_{++}': ["_", "C", "++"],
  'C_{--}': ["_", "C", "--"],
  'C_+x': ["*", ["_", "C", "+"], "x"],
  'C_-x': ["*", ["_", "C", "-"], "x"],
  'C_+2': ["*", ["_", "C", "+"], 2],
  'C_-2': ["*", ["_", "C", "-"], 2],
  'f^+': ["^", "f", "+"],
  'f^-': ["^", "f", "-"],
  'f^+x': ["*", ["^", "f", "+"], "x"],
  'f^-x': ["*", ["^", "f", "-"], "x"],
  'f^+2': ["*", ["^", "f", "+"], 2],
  'f^-2': ["*", ["^", "f", "-"], 2],
  'f^{++}': ["^", "f", "++"],
  'f^{--}': ["^", "f", "--"],
  'f^{+++}': ["^", "f", "+++"],
  'f^{---}': ["^", "f", "---"],
  'f^{++++}': ["^", "f", "++++"],
  'f^{----}': ["^", "f", "----"],
  'f^{+++++}': ["^", "f", "+++++"],
  'f^{-----}': ["^", "f", "-----"],
  'f^{++++++}': ["^", "f", "++++++"],
  'f^{------}': ["^", "f", "------"],
  'f^{2+}': ["^", "f", '2+'],
  'f^{2-}': ["^", "f", '2-'],
  'f^{2++}': ["^", "f", '2++'],
  'f^{2--}': ["^", "f", '2--'],
  'f^{2+++}': ["^", "f", '2+++'],
  'f^{2---}': ["^", "f", '2---'],
  'f^{2++++}': ["^", "f", '2++++'],
  'f^{2----}': ["^", "f", '2----'],
  'f^{2+++++}': ["^", "f", '2+++++'],
  'f^{2-----}': ["^", "f", '2-----'],
  'f_+': ["_", "f", "+"],
  'f_-': ["_", "f", "-"],
  'f_{++}': ["_", "f", "++"],
  'f_{--}': ["_", "f", "--"],
  'f_+x': ["*", ["_", "f", "+"], "x"],
  'f_-x': ["*", ["_", "f", "-"], "x"],
  'f_+2': ["*", ["_", "f", "+"], 2],
  'f_-2': ["*", ["_", "f", "-"], 2],
  '_6^{14}C': ["*", ["^", ["_", '\uff3f', 6], 14], "C"],
  '+': '+',
  '++': '++',
  '+++': '+++',
  '++++': '++++',
  '+++++': '+++++',
  '++++++': '++++++',
  '-': '-',
  '--': '--',
  '---': '---',
  '----': '----',
  '-----': '-----',
  '------': '------',
  '-+': '-+',
  '+-': '+-',
  '++-': '++-',
  '--+': '--+',
  '-++---++++': '-++---++++',
  '+--+++----': '+--+++----',
  '+5': ['+', 5],
  '++5': ['+', ['+', 5]],
  '+()': ['+', '\uff3f'],
  '+{}': ['+', '\uff3f'],
  '++()': ['+', ['+', '\uff3f']],
  '++{}': ['+', ['+', '\uff3f']],
  'x+': 'x+',
  'x++': 'x++',
  'x+++': 'x+++',
  'x++++': 'x++++',
  'x+++++': 'x+++++',
  'x++++++': 'x++++++',
  'x-': 'x-',
  'x--': 'x--',
  'x---': 'x---',
  'x----': 'x----',
  'x-----': 'x-----',
  'x------': 'x------',
  'x-+': 'x-+',
  'x+-': 'x+-',
  'x++-': 'x++-',
  'x--+': 'x--+',
  'x-++---++++': 'x-++---++++',
  'x+--+++----': 'x+--+++----',
  'x^2+': ['+', ["^", "x", 2], '\uff3f'],
  'x^2++': ['+', ["^", "x", 2], '+'],
  'x^2+++': ['+', ["^", "x", 2], "++"],
  'x^2++++': ['+', ["^", "x", 2], "+++"],
  'x^2+++++': ['+', ["^", "x", 2], "++++"],
  'x^2++++++': ['+', ["^", "x", 2], "+++++"],
  'x^2-': ['+', ["^", "x", 2], ["-", '\uff3f']],
  'x^2--': ['+', ["^", "x", 2], ["-", '-']],
  'x^2---': ['+', ["^", "x", 2], ["-", "--"]],
  'x^2----': ['+', ["^", "x", 2], ["-","---"]],
  'x^2-----': ['+', ["^", "x", 2], ["-", "----"]],
  'x^2------': ['+', ["^", "x", 2], ["-", "-----"]],
  'x^2-+': ['+', ["^", "x", 2], ['-','+']],
  'x^2+-': ['+', ["^", "x", 2], ['-', '\uff3f']],
  'x^2++-': ['+', ["^", "x", 2], '+-'],
  'x^2--+': ['+', ["^", "x", 2], ['-', '-+']],
  'x^2-++---++++': ['+', ["^", "x", 2], ['-', '++---++++']],
  'x^2+--+++----': ['+', ["^", "x", 2], ['-', '-+++----']],
  '^+': ["^", "\uff3f", "+"],
  '^-': ["^", "\uff3f", "-"],
  '1+2+': ["+", 1, 2, '\uff3f'],
  '3C^+x': ["*", 3, ["^", "C", "+"], "x"],
  '3C^-x': ["*", 3, ["^", "C", "-"], "x"],
  '5+()': ['+', 5, '\uff3f'],
  '5+()+2': ['+', 5, '\uff3f', 2],
  '5+{}': ['+', 5, '\uff3f'],
  '5+{}+2': ['+', 5, '\uff3f', 2],
  '\\Re(x)': ["apply", "re" ,"x"],
  '\\Im(x)': ["apply", "im" ,"x"],
  '\\det(A)': ["apply", "det", "A"],
  '\\trace(A)': ["apply", "trace", "A"],
  '\\operatorname{nCr}(x,y)': ["apply", "nCr", ["tuple", "x", "y"]],
  '\\operatorname{nPr}(x,y)': ["apply", "nPr", ["tuple", "x", "y"]],
  '\\binom{x}{y}': ["binom", "x", "y"],
  '\\vec{a}': ["vec", "a"],
  '\\lfloor a \\rfloor': ["apply", "floor", "a"],
  '\\lceil a \\rceil': ["apply", "ceil", "a"],
  '\\operatorname{round}(a)': ["apply", "round", "a"],
  '\\langle x, y \\rangle': ["altvector", "x", "y"],
  'x \\perp y': ["perp", "x", "y"],
  'x \\bot y': ["perp", "x", "y"],
  'x^{\\perp}': ["^", "x", "perp"],
  'x^{\\bot}': ["^", "x", "perp"],
  'x^\\perp': ["^", "x", "perp"],
  'x^\\bot': ["^", "x", "perp"],
  'x_{\\perp}': ["_", "x", "perp"],
  'x_{\\bot}': ["_", "x", "perp"],
  'x_\\perp': ["_", "x", "perp"],
  'x_\\bot': ["_", "x", "perp"],
  'x \\parallel y': ["parallel", "x", "y"],
  'f^{\\perp}': ["^", "f", "perp"],
  'f^{\\bot}': ["^", "f", "perp"],
  'f^\\perp': ["^", "f", "perp"],
  'f^\\bot': ["^", "f", "perp"],
  'f_{\\perp}': ["_", "f", "perp"],
  'f_{\\bot}': ["_", "f", "perp"],
  'f_\\perp': ["_", "f", "perp"],
  'f_\\bot': ["_", "f", "perp"],
  'f \\parallel y': ["parallel", "f", "y"],
  'x \\| y': ["parallel", "x", "y"],
};


Object.keys(trees).forEach(function(string) {
  test("parses " + string, () => {
    expect(converter.convert(string)).toEqual(trees[string]);
  });

});


// inputs that should throw an error
var bad_inputs = {
  // '1++1': "Invalid location of '+'",
  ')1++1': "Invalid location of ')'",
  '(1+1': "Expecting )",
  // 'x-y-': "Unexpected end of input",
  '|x_|': "Invalid location of '|'",
  // '_x': "Invalid location of _",
  // 'x_': "Unexpected end of input",
  'x@2': "Invalid symbol '@'",
  // '|y/v': "Expecting |",
  // 'x+^2': "Invalid location of ^",
  // 'x/\'y': "Invalid location of '",
  '[1,2,3)': "Expecting ]",
  '(1,2,3]': "Expecting )",
  '[x)': "Expecting ]",
  '(x]': "Expecting )",
  // '\\sin': "Unexpected end of input",
  // '\\sin+\\cos': "Invalid location of '+'",
}


Object.keys(bad_inputs).forEach(function(string) {
  test("throws " + string, function() {
    expect(() => {converter.convert(string)}).toThrow(bad_inputs[string]);
  });
});


test("function symbols", function () {
  let converter = new latexToAst({functionSymbols: []});
  expect(converter.convert('f(x)+h(y)')).toEqual(
    ['+',['*', 'f', 'x'], ['*', 'h', 'y']]);

  converter = new latexToAst({functionSymbols: ['f']});
  expect(converter.convert('f(x)+h(y)')).toEqual(
    ['+',['apply', 'f', 'x'], ['*', 'h', 'y']]);

  converter = new latexToAst({functionSymbols: ['f', 'h']});
  expect(converter.convert('f(x)+h(y)')).toEqual(
    ['+',['apply', 'f', 'x'], ['apply', 'h', 'y']]);

  converter = new latexToAst({functionSymbols: ['f', 'h', 'x']});
  expect(converter.convert('f(x)+h(y)')).toEqual(
    ['+',['apply', 'f', 'x'], ['apply', 'h', 'y']]);

});


test("applied function symbols", function () {

  let converter = new latexToAst({appliedFunctionSymbols: [],
				  allowedLatexSymbols: ['custom', 'sin']});
  expect(converter.convert('\\sin(x) + \\custom(y)')).toEqual(
    ['+', ['*', 'sin', 'x'], ['*', 'custom', 'y']]);
  expect(converter.convert('\\sin x  + \\custom y')).toEqual(
    ['+', ['*', 'sin', 'x'], ['*', 'custom', 'y']]);

  converter = new latexToAst({appliedFunctionSymbols: ['custom'],
			      allowedLatexSymbols: ['custom', 'sin']});
  expect(converter.convert('\\sin(x) + \\custom(y)')).toEqual(
    ['+', ['*', 'sin', 'x'], ['apply', 'custom', 'y']]);
  expect(converter.convert('\\sin x  + \\custom y')).toEqual(
    ['+', ['*', 'sin', 'x'], ['apply', 'custom', 'y']]);

  converter = new latexToAst({appliedFunctionSymbols: ['custom', 'sin'],
				  allowedLatexSymbols: ['custom', 'sin']});
  expect(converter.convert('\\sin(x) + \\custom(y)')).toEqual(
    ['+', ['apply', 'sin', 'x'], ['apply', 'custom', 'y']]);
  expect(converter.convert('\\sin x  + \\custom y')).toEqual(
    ['+', ['apply', 'sin', 'x'], ['apply', 'custom', 'y']]);

});

test("allow simplified function application", function () {
  let converter = new latexToAst();
  expect(converter.convert('\\sin x')).toEqual(
    ['apply', 'sin', 'x']);

  converter = new latexToAst({allowSimplifiedFunctionApplication: false});
  expect(() => {converter.convert('\\sin x')}).toThrow(
    "Expecting ( after function");

  converter = new latexToAst({allowSimplifiedFunctionApplication: true});
  expect(converter.convert('\\sin x')).toEqual(
    ['apply', 'sin', 'x']);

});

test("parse Leibniz notation", function () {

  let converter = new latexToAst();
  expect(converter.convert('\\frac{dy}{dx}')).toEqual(
    ['derivative_leibniz', 'y', ['tuple', 'x']]);

  converter = new latexToAst({parseLeibnizNotation: false});
  expect(converter.convert('\\frac{dy}{dx}')).toEqual(
    ['/', ['*', 'd', 'y'], ['*', 'd', 'x']]);

  converter = new latexToAst({parseLeibnizNotation: true});
  expect(converter.convert('\\frac{dy}{dx}')).toEqual(
    ['derivative_leibniz', 'y', ['tuple', 'x']]);

});


test("parse scientific notation", function () {

  let converter = new latexToAst();
  expect(converter.convert('2E^2-3E+2')).toEqual(
    ['+', ['*', 2, ["^", "E", 2]], -300]);

  converter = new latexToAst({parseScientificNotation: false});
  expect(converter.convert('2E^2-3E+2')).toEqual(
    ['+', ['*', 2, ["^", "E", 2]], ['-', ['*', 3, "E"]], 2]);

  converter = new latexToAst({parseScientificNotation: true});
  expect(converter.convert('2E^2-3E+2')).toEqual(
    ['+', ['*', 2, ["^", "E", 2]], -300]);

});

test("conditional probability", function () {

  let converter = new latexToAst({functionSymbols: ["P"]});

  expect(converter.convert("P(A|B)")).toEqual(
    ['apply', 'P', ['|', 'A', 'B']]);

  expect(converter.convert("P(A:B)")).toEqual(
    ['apply', 'P', [':', 'A', 'B']]);

  expect(converter.convert("P(R=1|X>2)")).toEqual(
    ['apply', 'P', ['|', ['=', 'R', 1], ['>', 'X', 2]]]);

  expect(converter.convert("P(R=1:X>2)")).toEqual(
    ['apply', 'P', [':', ['=', 'R', 1], ['>', 'X', 2]]]);

  expect(converter.convert("P( A \\land B | C \\lor D )")).toEqual(
    ['apply', 'P', ['|', ['and', 'A', 'B'], ['or', 'C', 'D']]]);

  expect(converter.convert("P( A \\land B : C \\lor D )")).toEqual(
    ['apply', 'P', [':', ['and', 'A', 'B'], ['or', 'C', 'D']]]);

});
