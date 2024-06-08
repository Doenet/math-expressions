import textToAst from '../lib/converters/text-to-ast';
import { ParseError } from '../lib/converters/error';

var converter = new textToAst();

var trees = {
  '1+x+3': ['+', 1, 'x', 3],
  "1 + - x": ['+', 1, ['-', 'x']],
  "1 - x": ['+', 1, ['-', 'x']],
  "1 - - x": ['+', 1, ['-', ['-', 'x']]],
  "1 + x/2": ['+', 1, ['/', 'x', 2]],
  '1-x-3': ['+', 1, ['-', 'x'], -3],
  '1.+x+3.0': ['+', 1, 'x', 3],
  '1-3': ['+', 1, -3],
  '1-0': ['+', 1, ['-', 0]],
  'x^2': ['^', 'x', 2],
  '-x^2': ['-', ['^', 'x', 2]],
  '-3^2': ['-', ['^', 3, 2]],
  'x^47': ['^', 'x', 47],
  'x^ab': ['*', ['^', 'x', 'a'], 'b'],
  'x^(a!)': ['^', 'x', ['apply', 'factorial', 'a']],
  'x^a!': ['apply', 'factorial', ['^', 'x', 'a']],
  'log x': ['apply', 'log', 'x'],
  'ln x': ['apply', 'ln', 'x'],
  'log_10 x': ['apply', 'log10', 'x'],
  'x*y*z': ['*', 'x', 'y', 'z'],
  'xyz': ['*', 'x', 'y', 'z'],
  'xyz2': 'xyz2',
  'in': ['*', 'i', 'n'],
  'ni': ['*', 'n', 'i'],
  'x*y*z*w': ['*', 'x', 'y', 'z', 'w'],
  'xyzw': ['*', 'x', 'y', 'z', 'w'],
  '(x*y)*(z*w)': ['*', 'x', 'y', 'z', 'w'],
  'c*(a+b)': ['*', 'c', ['+', 'a', 'b']],
  '(a+b)*c': ['*', ['+', 'a', 'b'], 'c'],
  '|x|': ['apply', 'abs', 'x'],
  'a!': ['apply', 'factorial', 'a'],
  'theta': 'theta',
  'cos(theta)': ['apply', 'cos', 'theta'],
  'x!': ['apply', 'factorial', 'x'],
  '|sin(|x|)|': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
  'sin(θ)': ['apply', 'sin', 'theta'],
  '|x+3=2|': ['apply', 'abs', ['=', ['+', 'x', 3], 2]],
  'x_y_z': ['_', 'x', ['_', 'y', 'z']],
  'x_(y_z)': ['_', 'x', ['_', 'y', 'z']],
  '(x_y)_z': ['_', ['_', 'x', 'y'], 'z'],
  'x^y^z': ['^', ['^', 'x', 'y'], 'z'],
  'x^(y^z)': ['^', 'x', ['^', 'y', 'z']],
  '(x^y)^z': ['^', ['^', 'x', 'y'], 'z'],
  'x^y_z': ['^', 'x', ['_', 'y', 'z']],
  'x_y^z': ['^', ['_', 'x', 'y'], 'z'],
  'f_g_z': ['_', 'f', ['_', 'g', 'z']],
  'f_(g_z)': ['_', 'f', ['_', 'g', 'z']],
  '(f_g)_z': ['_', ['_', 'f', 'g'], 'z'],
  'f^g^z': ['^', ['^', 'f', 'g'], 'z'],
  'f^(g^z)': ['^', 'f', ['^', 'g', 'z']],
  '(f^g)^z': ['^', ['^', 'f', 'g'], 'z'],
  'f^g_z': ['^', 'f', ['_', 'g', 'z']],
  'f_g^z': ['^', ['_', 'f', 'g'], 'z'],
  'xyz!': ['*', 'x', 'y', ['apply', 'factorial', 'z']],
  'x': 'x',
  'f': 'f',
  'fg': ['*', 'f', 'g'],
  'f+g': ['+', 'f', 'g'],
  'f(x)': ['apply', 'f', 'x'],
  'f(x,y,z)': ['apply', 'f', ['tuple', 'x', 'y', 'z']],
  'fg(x)': ['*', 'f', ['apply', 'g', 'x']],
  'fp(x)': ['*', 'f', 'p', 'x'],
  'fx': ['*', 'f', 'x'],
  'f\'': ['prime', 'f'],
  'f′': ['prime', 'f'],
  'fg\'': ['*', 'f', ['prime', 'g']],
  'f\'g': ['*', ['prime', 'f'], 'g'],
  'f\'g\'\'': ['*', ['prime', 'f'], ['prime', ['prime', 'g']]],
  'x\'': ['prime', 'x'],
  'f\'(x)': ['apply', ['prime', 'f'], 'x'],
  'f(x)\'': ['prime', ['apply', 'f', 'x']],
  'sin(x)\'': ['prime', ['apply', 'sin', 'x']],
  'sin\'(x)': ['apply', ['prime', 'sin'], 'x'],
  'f\'\'(x)': ['apply', ['prime', ['prime', 'f']], 'x'],
  'sin(x)\'\'': ['prime', ['prime', ['apply', 'sin', 'x']]],
  'f(x)^t_y': ['^', ['apply', 'f', 'x'], ['_', 't', 'y']],
  'f_t(x)': ['apply', ['_', 'f', 't'], 'x'],
  'f(x)_t': ['*', ['apply', 'f', 'x'], ['_', '\uff3f', 't']],
  'f^2(x)': ['apply', ['^', 'f', 2], 'x'],
  'f(x)^2': ['^', ['apply', 'f', 'x'], 2],
  'f\'^a(x)': ['apply', ['^', ['prime', 'f'], 'a'], 'x'],
  'f^(a\')(x)': ['apply', ['^', 'f', ['prime', 'a']], 'x'],
  'f^a\'(x)': ['*', ['prime', ['^', 'f', 'a']], 'x'],
  'f_a^(b\')(x)': ['apply', ['^', ['_', 'f', 'a'], ['prime', 'b']], 'x'],
  'f_a^b\'(x)': ['*', ['prime', ['^', ['_', 'f', 'a'], 'b']], 'x'],
  'f_a\'^b(x)': ['apply', ['^', ['prime', ['_', 'f', 'a']], 'b'], 'x'],
  'f_(a\')^b(x)': ['apply', ['^', ['_', 'f', ['prime', 'a']], 'b'], 'x'],
  'sin x': ['apply', 'sin', 'x'],
  'f x': ['*', 'f', 'x'],
  'sin^xyz': ['*', ['apply', ['^', 'sin', 'x'], 'y'], 'z'],
  'sin xy': ['*', ['apply', 'sin', 'x'], 'y'],
  'sin^2(x)': ['apply', ['^', 'sin', 2], 'x'],
  'x^2!': ['apply', 'factorial', ['^', 'x', 2]],
  'x^2!!': ['apply', 'factorial', ['apply', 'factorial', ['^', 'x', 2]]],
  'x_t^2': ['^', ['_', 'x', 't'], 2],
  'x_(f^2)': ['_', 'x', ['^', 'f', 2]],
  'x_f^2': ['^', ['_', 'x', 'f'], 2],
  'x_t\'': ['prime', ['_', 'x', 't']],
  'x_(f\')': ['_', 'x', ['prime', 'f']],
  'x_f\'': ['prime', ['_', 'x', 'f']],
  'x^^': ['^', ['^', 'x', '\uff3f'], '\uff3f'],
  'f^^': ['^', ['^', 'f', '\uff3f'], '\uff3f'],
  'sin^^': ['apply', ['^', ['^', 'sin', '\uff3f'], '\uff3f'], '\uff3f'],
  'y^f(x)': ['*', ['^', 'y', 'f'], 'x'],
  'y^(f(x))': ['^', 'y', ['apply', 'f', 'x']],
  'y^f_2(x)': ['*', ['^', 'y', ['_', 'f', 2]], 'x'],
  'y^(f_2(x))': ['^', 'y', ['apply', ['_', 'f', 2], 'x']],
  'y^f^2(x)': ['*', ['^', ['^', 'y', 'f'], 2], 'x'],
  'y^(f^2(x))': ['^', 'y', ['apply', ['^', 'f', 2], 'x']],
  'y^f_1^2(x)': ['*', ['^', ['^', 'y', ['_', 'f', 1]], 2], 'x'],
  'y^(f_1^2(x))': ['^', 'y', ['apply', ['^', ['_', 'f', 1], 2], 'x']],
  'y^f': ['^', 'y', 'f'],
  'y^(f)': ['^', 'y', 'f'],
  'y^f_2': ['^', 'y', ['_', 'f', 2]],
  'y^(f_2)': ['^', 'y', ['_', 'f', 2]],
  'y^f^2': ['^', ['^', 'y', 'f'], 2],
  'y^(f^2)': ['^', 'y', ['^', 'f', 2]],
  'y^f_1^2': ['^', ['^', 'y', ['_', 'f', 1]], 2],
  'y^(f_1^2)': ['^', 'y', ['^', ['_', 'f', 1], 2]],
  'y^sin(x)': ['*', ['^', 'y', ['apply', 'sin', '\uff3f']], 'x'],
  'y^(sin(x))': ['^', 'y', ['apply', 'sin', 'x']],
  'y^sin_2(x)': ['*', ['^', 'y', ['apply', ['_', 'sin', 2], '\uff3f']], 'x'],
  'y^(sin_2(x))': ['^', 'y', ['apply', ['_', 'sin', 2], 'x']],
  'y^sin^2(x)': ['*', ['^', ['^', 'y', ['apply', 'sin', '\uff3f']], 2], 'x'],
  'y^(sin^2(x))': ['^', 'y', ['apply', ['^', 'sin', 2], 'x']],
  'y^sin_1^2(x)': ['*', ['^', ['^', 'y', ['apply', ['_', 'sin', 1], '\uff3f']], 2], 'x'],
  'y^(sin_1^2(x))': ['^', 'y', ['apply', ['^', ['_', 'sin', 1], 2], 'x']],
  'y^sin': ['^', 'y', ['apply', 'sin', '\uff3f']],
  'y^(sin)': ['^', 'y', ['apply', 'sin', '\uff3f']],
  'y^sin_2': ['^', 'y', ['apply', ['_', 'sin', 2], '\uff3f']],
  'y^(sin_2)': ['^', 'y', ['apply', ['_', 'sin', 2], '\uff3f']],
  'y^sin^2': ['^', ['^', 'y', ['apply', 'sin', '\uff3f']], 2],
  'y^(sin^2)': ['^', 'y', ['apply', ['^', 'sin', 2], '\uff3f']],
  'y^sin_1^2': ['^', ['^', 'y', ['apply', ['_', 'sin', 1], '\uff3f']], 2],
  'y^(sin_1^2)': ['^', 'y', ['apply', ['^', ['_', 'sin', 1], 2], '\uff3f']],
  'f_y(x)_z': ['*', ['apply', ['_', 'f', 'y'], 'x'], ['_', '\uff3f', 'z']],
  'f(x)\'^2': ['^', ['prime', ['apply', 'f', 'x']], 2],
  'f(x)\'^2\'\'': ['prime', ['prime', ['^', ['prime', ['apply', 'f', 'x']], 2]]],
  'f(x)\'^2\'\'^a': ['^', ['prime', ['prime', ['^', ['prime', ['apply', 'f', 'x']], 2]]], 'a'],
  'x\'^2': ['^', ['prime', 'x'], 2],
  'x\'^2\'\'': ['prime', ['prime', ['^', ['prime', 'x'], 2]]],
  'x\'^2\'\'^a': ['^', ['prime', ['prime', ['^', ['prime', 'x'], 2]]], 'a'],
  '(x,y,z)': ['tuple', 'x', 'y', 'z'],
  '⟨x,y,z⟩': ['altvector', 'x', 'y', 'z'], // langle and rangle delimiters
  '〈x,y,z〉': ['altvector', 'x', 'y', 'z'], // langle and rangle delimiters
  '(x,y)-[x,y]': ['+', ['tuple', 'x', 'y'], ['-', ['array', 'x', 'y']]],
  '2[z-(x+1)]': ['*', 2, ['+', 'z', ['-', ['+', 'x', 1]]]],
  '{1,2,x}': ['set', 1, 2, 'x'],
  '{x, x}': ['set', 'x', 'x'],
  '{x}': ['set', 'x'],
  '{-x}': ['set', ['-', 'x']],
  '(1,2]': ['interval', ['tuple', 1, 2], ['tuple', false, true]],
  '[1,2)': ['interval', ['tuple', 1, 2], ['tuple', true, false]],
  '[1,2]': ['array', 1, 2],
  '(1,2)': ['tuple', 1, 2],
  '1,2,3': ['list', 1, 2, 3],
  'x=a': ['=', 'x', 'a'],
  'x=y=1': ['=', 'x', 'y', 1],
  'x=(y=1)': ['=', 'x', ['=', 'y', 1]],
  '(x=y)=1': ['=', ['=', 'x', 'y'], 1],
  '7 != 2': ['ne', 7, 2],
  '7 ≠ 2': ['ne', 7, 2],
  'not x=y': ['not', ['=', 'x', 'y']],
  '!x=y': ['not', ['=', 'x', 'y']],
  '!(x=y)': ['not', ['=', 'x', 'y']],
  'not(x=y)': ['not', ['=', 'x', 'y']],
  '!x=1': ['not', ['=', 'x', 1]],
  '!(x=1)': ['not', ['=', 'x', 1]],
  '!(x=y) or z != w': ['or', ['not', ['=', 'x', 'y']], ['ne', 'z', 'w']],
  'x>y': ['>', 'x', 'y'],
  'x>=y': ['ge', 'x', 'y'],
  'x≥y': ['ge', 'x', 'y'],
  'x>y>z': ['gts', ['tuple', 'x', 'y', 'z'], ['tuple', true, true]],
  'x>y>=z': ['gts', ['tuple', 'x', 'y', 'z'], ['tuple', true, false]],
  'x>=y>z': ['gts', ['tuple', 'x', 'y', 'z'], ['tuple', false, true]],
  'x>=y>=z': ['gts', ['tuple', 'x', 'y', 'z'], ['tuple', false, false]],
  'x<y': ['<', 'x', 'y'],
  'x<=y': ['le', 'x', 'y'],
  'x≤y': ['le', 'x', 'y'],
  'x<y<z': ['lts', ['tuple', 'x', 'y', 'z'], ['tuple', true, true]],
  'x<y<=z': ['lts', ['tuple', 'x', 'y', 'z'], ['tuple', true, false]],
  'x<=y<z': ['lts', ['tuple', 'x', 'y', 'z'], ['tuple', false, true]],
  'x<=y<=z': ['lts', ['tuple', 'x', 'y', 'z'], ['tuple', false, false]],
  'x<y>z': ['>', ['<', 'x', 'y'], 'z'],
  'A subset B': ['subset', 'A', 'B'],
  'A ⊂ B': ['subset', 'A', 'B'],
  'A subseteq B': ['subseteq', 'A', 'B'],
  'A ⊆ B': ['subseteq', 'A', 'B'],
  'A notsubset B': ['notsubset', 'A', 'B'],
  'A ⊄ B': ['notsubset', 'A', 'B'],
  'A notsubseteq B': ['notsubseteq', 'A', 'B'],
  'A ⊈ B': ['notsubseteq', 'A', 'B'],
  'A superset B': ['superset', 'A', 'B'],
  'A ⊃ B': ['superset', 'A', 'B'],
  'A superseteq B': ['superseteq', 'A', 'B'],
  'A ⊇ B': ['superseteq', 'A', 'B'],
  'A notsuperset B': ['notsuperset', 'A', 'B'],
  'A ⊅ B': ['notsuperset', 'A', 'B'],
  'A notsuperseteq B': ['notsuperseteq', 'A', 'B'],
  'A ⊉ B': ['notsuperseteq', 'A', 'B'],
  'forall x': ['forall', 'x'],
  '∀x': ['forall', 'x'],
  'forall x elementof A': ['forall', ['in', 'x', 'A']],
  '∀x∈A': ['forall', ['in', 'x', 'A']],
  'exists x': ['exists', 'x'],
  '∃x': ['exists', 'x'],
  'exists x elementof A': ['exists', ['in', 'x', 'A']],
  '∃x∈A': ['exists', ['in', 'x', 'A']],
  'x elementof A': ['in', 'x', 'A'],
  'x ∈ A': ['in', 'x', 'A'],
  'x notelementof A': ['notin', 'x', 'A'],
  'x ∉ A': ['notin', 'x', 'A'],
  'A containselement x': ['ni', 'A', 'x'],
  'A ∋ x': ['ni', 'A', 'x'],
  'A notcontainselement x': ['notni', 'A', 'x'],
  'A ∌ x': ['notni', 'A', 'x'],
  'A union B': ['union', 'A', 'B'],
  'A ∪ B': ['union', 'A', 'B'],
  'A intersect B': ['intersect', 'A', 'B'],
  'A ∩ B': ['intersect', 'A', 'B'],
  'A and B': ['and', 'A', 'B'],
  'A & B': ['and', 'A', 'B'],
  'A && B': ['and', 'A', 'B'],
  'A ∧ B': ['and', 'A', 'B'],
  'A or B': ['or', 'A', 'B'],
  'A ∨ B': ['or', 'A', 'B'],
  'A ∧ B ∧ C': ['and', 'A', 'B', 'C'],
  'A ∨ B ∨ C': ['or', 'A', 'B', 'C'],
  'A and B or C': ['or', ['and', 'A', 'B'], 'C'],
  'A or B and C': ['or', 'A', ['and', 'B', 'C']],


  'A implies B': ['implies', 'A', 'B'],
  'A ⇒ B': ['implies', 'A', 'B'],
  'A ⟹ B': ['implies', 'A', 'B'],
  'A impliedby B': ['impliedby', 'A', 'B'],
  'A ⇐ B': ['impliedby', 'A', 'B'],
  'A ⟸ B': ['impliedby', 'A', 'B'],
  'A iff B': ['iff', 'A', 'B'],
  'A ⇔ B': ['iff', 'A', 'B'],
  'A ⟺ B': ['iff', 'A', 'B'],
  'A and not B ⟹ not C or D': ['implies', ['and', 'A', ['not', 'B']], ['or', ['not', 'C'], 'D']],
  'A and not B ⟸ not C or D': ['impliedby', ['and', 'A', ['not', 'B']], ['or', ['not', 'C'], 'D']],
  'A and not B ⟺ not C or D': ['iff', ['and', 'A', ['not', 'B']], ['or', ['not', 'C'], 'D']],
  'A rightarrow B': ['rightarrow', 'A', 'B'],
  'A → B': ['rightarrow', 'A', 'B'],
  'A ⟶ B': ['rightarrow', 'A', 'B'],
  'A leftarrow B': ['leftarrow', 'A', 'B'],
  'A ← B': ['leftarrow', 'A', 'B'],
  'A ⟵ B': ['leftarrow', 'A', 'B'],
  'A leftrightarrow B': ['leftrightarrow', 'A', 'B'],
  'A ↔ B': ['leftrightarrow', 'A', 'B'],
  'A ⟷ B': ['leftrightarrow', 'A', 'B'],
  'A and not B → not C or D': ['rightarrow', ['and', 'A', ['not', 'B']], ['or', ['not', 'C'], 'D']],
  'A and not B ← not C or D': ['leftarrow', ['and', 'A', ['not', 'B']], ['or', ['not', 'C'], 'D']],
  'A and not B ↔ not C or D': ['leftrightarrow', ['and', 'A', ['not', 'B']], ['or', ['not', 'C'], 'D']],
  'x elementof emptyset': ['in', 'x', 'emptyset'],
  'x ∈ ∅': ['in', 'x', 'emptyset'],

  '1.2E3': 1200,
  '1.2E+3  ': 1200,
  '3.1E-3 ': 0.0031,
  '3.1E- 3 ': ['+', ['*', 3.1, 'E'], -3],
  '3.1E -3 ': ['+', ['*', 3.1, 'E'], -3],
  '3.1E - 3 ': ['+', ['*', 3.1, 'E'], -3],
  '3.1E-3 + 2 ': ['+', ['*', 3.1, 'E'], -3, 2],
  '(3.1E-3) + 2': ['+', 0.0031, 2],
  'sin((3.1E-3)x)': ['apply', 'sin', ['*', 0.0031, 'x']],
  'sin( 3.1E-3 x)': ['apply', 'sin', ['+', ['*', 3.1, 'E'], ['-', ['*', 3, 'x']]]],
  '|3.1E-3|': ['apply', 'abs', 0.0031],
  '(3.1E-3, 1E2)': ['tuple', 0.0031, 100],
  '(3.1E-3, 1E2]': ["interval", ["tuple", 0.0031, 100], ["tuple", false, true]],
  '{ 3.1E-3, 1E2 }': ['set', 0.0031, 100],
  '1.2e-3': ['+', ['*', 1.2, 'e'], -3],
  '+2': ['+', 2],
  'oo': Infinity,
  '+oo': ['+', Infinity],
  'dx/dt=q': ['=', ['derivative_leibniz', 'x', ['tuple', 't']], 'q'],
  'dx / dt = q': ['=', ['derivative_leibniz', 'x', ['tuple', 't']], 'q'],
  'd x/dt = q': ['=', ['derivative_leibniz', 'x', ['tuple', 't']], 'q'],
  'dx/d t = q': ['=', ['derivative_leibniz', 'x', ['tuple', 't']], 'q'],
  '(dx)/(dt)=q': ['=', ["/", ["*", "d", "x"], ["*", "d", "t"]], 'q'],
  'dx_2/dt = q': ['=', ["*", ["/", ["*", "d", ["_", "x", 2]], "d"], "t"], 'q'],
  'd hello1 /d bye2 = q': ['=', ['derivative_leibniz', 'hello1', ['tuple', 'bye2']], 'q'],
  'd^2x/dt^2=q': ['=', ['derivative_leibniz', ["tuple", 'x', 2], ["tuple", ["tuple", 't', 2]]], 'q'],
  'd ^ 2 x / d t ^ 2 = q': ['=', ['derivative_leibniz', ["tuple", 'x', 2], ["tuple", ["tuple", 't', 2]]], 'q'],
  'd^2x/dt^3 = q': ['=', ["*", ["/", ["*", ["^", "d", 2], "x"], "d"], ["^", "t", 3]], 'q'],
  'd^2x/dsdt=q': ['=', ['derivative_leibniz', ["tuple", 'x', 2], ['tuple', "s", "t"]], 'q'],
  'd^2x/dsdta=q': ["=", ["*", ["/", ["*", ["^", "d", 2], "x"], "d"], "s", "d", "t", "a"], "q"],
  'd^2x/ds dt=q': ['=', ['derivative_leibniz', ["tuple", 'x', 2], ['tuple', "s", "t"]], 'q'],
  'd ^ 2 x / d s d t =q': ['=', ['derivative_leibniz', ["tuple", 'x', 2], ['tuple', "s", "t"]], 'q'],
  'd^2 hello1 /d bye2^2 = q': ['=', ['derivative_leibniz', ['tuple', 'hello1', 2], ['tuple', ['tuple', 'bye2', 2]]], 'q'],
  'd^3x/dsdt^2 a=q': ['=', ['*', ['derivative_leibniz', ["tuple", 'x', 3], ['tuple', "s", ["tuple", "t", 2]]], 'a'], 'q'],
  'd^3x/dsdt^2a=q': ["=", ["*", ["/", ["*", ["^", "d", 3], "x"], "d"], "s", "d", ["^", "t", 2], "a"], "q"],
  'd^3x/dsdt=q': ["=", ["*", ["/", ["*", ["^", "d", 3], "x"], "d"], "s", "d", "t"], "q"],
  'd^3x/ds^2dt=q': ['=', ['derivative_leibniz', ["tuple", 'x', 3], ['tuple', ["tuple", "s", 2], "t"]], 'q'],
  'd^3κ/dξdβ^2 ♡=q': ['=', ['*', ['derivative_leibniz', ["tuple", 'kappa', 3], ['tuple', "xi", ["tuple", "beta", 2]]], 'heart'], 'q'],
  'd^3κ/dξdβ^2♡=q': ["=", ["*", ["/", ["*", ["^", "d", 3], "kappa"], "d"], "xi", "d", ["^", "beta", 2], "heart"], "q"],
  'dx/d2 = q': ['=', ["/", ["*", "d", "x"], "d2"], 'q'],
  'd2/dt = q': ['=', ["*", ["/", "d2", "d"], "t"], 'q'],
  'dxy/dt = q': ['=', ["*", ["/", ["*", "d", "x", "y"], "d"], "t"], 'q'],
  '∂x/∂t=q': ['=', ['partial_derivative_leibniz', 'x', ['tuple', 't']], 'q'],
  '∂x / ∂t = q': ['=', ['partial_derivative_leibniz', 'x', ['tuple', 't']], 'q'],
  '∂ x/∂t = q': ['=', ['partial_derivative_leibniz', 'x', ['tuple', 't']], 'q'],
  '∂x/∂ t = q': ['=', ['partial_derivative_leibniz', 'x', ['tuple', 't']], 'q'],
  '∂^3κ/∂ξ∂β^2 ♡=q': ['=', ['*', ['partial_derivative_leibniz', ["tuple", 'kappa', 3], ['tuple', "xi", ["tuple", "beta", 2]]], 'heart'], 'q'],
  '3|x|': ['*', 3, ['apply', 'abs', 'x']],
  '|a|b|c|': ['*', ['apply', 'abs', 'a'], 'b', ['apply', 'abs', 'c']],
  '|a|*b*|c|': ['*', ['apply', 'abs', 'a'], 'b', ['apply', 'abs', 'c']],
  '|a*|b|*c|': ['apply', 'abs', ['*', 'a', ['apply', 'abs', 'b'], 'c']],
  '|a(q|b|r)c|': ['apply', 'abs', ['*', 'a', 'q', ['apply', 'abs', 'b'], 'r', 'c']],
  'r=1|x': ['|', ['=', 'r', 1], 'x'],
  '{ x | x > 0 }': ['set', ['|', 'x', ['>', 'x', 0]]],
  'r=1:x': [':', ['=', 'r', 1], 'x'],
  '{ x : x > 0 }': ['set', [':', 'x', ['>', 'x', 0]]],
  '...': ['ldots'],
  '1,2,3,...': ['list', 1, 2, 3, ['ldots']],
  '(1,2,3,...)': ['tuple', 1, 2, 3, ['ldots']],
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
  'sin': ["apply", "sin", '\uff3f'],
  'sin+cos': ["apply", "sin", ['+', ["apply", "cos", '\uff3f']]],
  '/a': ["/", '\uff3f', "a"],
  'a/': ["/", "a", '\uff3f'],
  '/': ["/", '\uff3f', '\uff3f'],
  'C^+': ["^", "C", "+"],
  'C^-': ["^", "C", "-"],
  'C^+x': ["*", ["^", "C", "+"], "x"],
  'C^-x': ["*", ["^", "C", "-"], "x"],
  'C^+2': ["*", ["^", "C", "+"], 2],
  'C^-2': ["*", ["^", "C", "-"], 2],
  'C^(++)': ["^", "C", "++"],
  'C^(--)': ["^", "C", "--"],
  'C^(+++)': ["^", "C", "+++"],
  'C^(---)': ["^", "C", "---"],
  'C^(++++)': ["^", "C", "++++"],
  'C^(----)': ["^", "C", "----"],
  'C^(+++++)': ["^", "C", "+++++"],
  'C^(-----)': ["^", "C", "-----"],
  'C^(++++++)': ["^", "C", "++++++"],
  'C^(------)': ["^", "C", "------"],
  'C^(2+)': ["^", "C", '2+'],
  'C^(2-)': ["^", "C", '2-'],
  'C^(2++)': ["^", "C", '2++'],
  'C^(2--)': ["^", "C", '2--'],
  'C^(2+++)': ["^", "C", '2+++'],
  'C^(2---)': ["^", "C", '2---'],
  'C^(2++++)': ["^", "C", '2++++'],
  'C^(2----)': ["^", "C", '2----'],
  'C^(2+++++)': ["^", "C", '2+++++'],
  'C^(2-----)': ["^", "C", '2-----'],
  'C_+': ["_", "C", "+"],
  'C_-': ["_", "C", "-"],
  'C_(++)': ["_", "C", "++"],
  'C_(--)': ["_", "C", "--"],
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
  'f^(++)': ["^", "f", "++"],
  'f^(--)': ["^", "f", "--"],
  'f^(+++)': ["^", "f", "+++"],
  'f^(---)': ["^", "f", "---"],
  'f^(++++)': ["^", "f", "++++"],
  'f^(----)': ["^", "f", "----"],
  'f^(+++++)': ["^", "f", "+++++"],
  'f^(-----)': ["^", "f", "-----"],
  'f^(++++++)': ["^", "f", "++++++"],
  'f^(------)': ["^", "f", "------"],
  'f^(2+)': ["^", "f", '2+'],
  'f^(2-)': ["^", "f", '2-'],
  'f^(2++)': ["^", "f", '2++'],
  'f^(2--)': ["^", "f", '2--'],
  'f^(2+++)': ["^", "f", '2+++'],
  'f^(2---)': ["^", "f", '2---'],
  'f^(2++++)': ["^", "f", '2++++'],
  'f^(2----)': ["^", "f", '2----'],
  'f^(2+++++)': ["^", "f", '2+++++'],
  'f^(2-----)': ["^", "f", '2-----'],
  'f_+': ["_", "f", "+"],
  'f_-': ["_", "f", "-"],
  'f_(++)': ["_", "f", "++"],
  'f_(--)': ["_", "f", "--"],
  'f_+x': ["*", ["_", "f", "+"], "x"],
  'f_-x': ["*", ["_", "f", "-"], "x"],
  'f_+2': ["*", ["_", "f", "+"], 2],
  'f_-2': ["*", ["_", "f", "-"], 2],
  '_6^14C': ["*", ["^", ["_", '\uff3f', 6], 14], "C"],
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
  '++()': ['+', ['+', '\uff3f']],
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
  'x^2----': ['+', ["^", "x", 2], ["-", "---"]],
  'x^2-----': ['+', ["^", "x", 2], ["-", "----"]],
  'x^2------': ['+', ["^", "x", 2], ["-", "-----"]],
  'x^2-+': ['+', ["^", "x", 2], ['-', '+']],
  'x^2+-': ['+', ["^", "x", 2], ['-', '\uff3f']],
  'x^2++-': ['+', ["^", "x", 2], '+-'],
  'x^2--+': ['+', ["^", "x", 2], ['-', '-+']],
  'x^2-++---++++': ['+', ["^", "x", 2], ['-', '++---++++']],
  'x^2+--+++----': ['+', ["^", "x", 2], ['-', '-+++----']],
  '^+': ["^", "\uff3f", "+"],
  '^-': ["^", "\uff3f", "-"],
  'x^/(3-)': ["/", ["^", "x", "\uff3f"], "3-"],
  '1+2+': ["+", 1, 2, '\uff3f'],
  '3C^+x': ["*", 3, ["^", "C", "+"], "x"],
  '3C^-x': ["*", 3, ["^", "C", "-"], "x"],
  '5+()': ['+', 5, '\uff3f'],
  '5+()+2': ['+', 5, '\uff3f', 2],
  're(x)': ["apply", "re", "x"],
  'im(x)': ["apply", "im", "x"],
  'det(A)': ["apply", "det", "A"],
  'trace(A)': ["apply", "trace", "A"],
  'nPr(x,y)': ["apply", "nPr", ["tuple", "x", "y"]],
  'nCr(x,y)': ["apply", "nCr", ["tuple", "x", "y"]],
  'binom(x,y)': ["binom", "x", "y"],
  'vec(a)': ["vec", "a"],
  'linesegment(A,B)': ["linesegment", "A", "B"],
  'linesegment(A\',B\')': ["linesegment", ["prime", "A"], ["prime", "B"]],
  'floor(a)': ["apply", "floor", "a"],
  'ceil(a)': ["apply", "ceil", "a"],
  'round(a)': ["apply", "round", "a"],
  'x perp y': ["perp", "x", "y"],
  'x ⟂ y': ["perp", "x", "y"],
  'x^(perp)': ["^", "x", "perp"],
  'x^(⟂)': ["^", "x", "perp"],
  'x^perp': ["^", "x", "perp"],
  'x^⟂': ["^", "x", "perp"],
  'x_(perp)': ["_", "x", "perp"],
  'x_(⟂)': ["_", "x", "perp"],
  'x_perp': ["_", "x", "perp"],
  'x_⟂': ["_", "x", "perp"],
  'f^(perp)': ["^", "f", "perp"],
  'f^(⟂)': ["^", "f", "perp"],
  'f^perp': ["^", "f", "perp"],
  'f^⟂': ["^", "f", "perp"],
  'f_(perp)': ["_", "f", "perp"],
  'f_(⟂)': ["_", "f", "perp"],
  'f_perp': ["_", "f", "perp"],
  'f_⟂': ["_", "f", "perp"],
  'x parallel y': ["parallel", "x", "y"],
  'x ∥ y': ["parallel", "x", "y"],
  'angle': "angle",
  'angle ABC': ["angle", "A", "B", "C"],
  'angle A_1B_xC_+': ["angle", ["_", "A", 1], ["_", "B", "x"], ["_", "C", "+"]],
  'angle A^1B^xC^+': ["angle", ["^", "A", 1], ["^", "B", "x"], ["^", "C", "+"]],
  'angle A\'B\'\'C\'\'\'': ["angle", ["prime", "A"], ["prime", ["prime", "B"]], ["prime", ["prime", ["prime", "C"]]]],
  'angle A+B': ["+", ["angle", "A"], "B"],
  'angle A*B': ["*", ["angle", "A"], "B"],
  'angle (ABC)': ["angle", ["*", "A", "B", "C"]],
  'angle (A,B,C)': ["angle", "A", "B", "C"],
  '∠': "angle",
  '∠ABC': ["angle", "A", "B", "C"],
  '∠A_1B_xC_+': ["angle", ["_", "A", 1], ["_", "B", "x"], ["_", "C", "+"]],
  '∠A^1B^xC^+': ["angle", ["^", "A", 1], ["^", "B", "x"], ["^", "C", "+"]],
  '∠A\'B\'\'C\'\'\'': ["angle", ["prime", "A"], ["prime", ["prime", "B"]], ["prime", ["prime", ["prime", "C"]]]],
  '∠A+B': ["+", ["angle", "A"], "B"],
  '∠A*B': ["*", ["angle", "A"], "B"],
  '∠(ABC)': ["angle", ["*", "A", "B", "C"]],
  '∠(A,B,C)': ["angle", "A", "B", "C"],
  '$x': ["unit", "$", "x"],
  'x%': ["unit", "x", "%"],
  'x deg': ["unit", "x", "deg"],
  '$xy': ["unit", "$", ["*", "x", "y"]],
  'x%y': ["*", ["unit", "x", "%"], "y"],
  'x deg y': ["*", ["unit", "x", "deg"], "y"],
  'y$x': ["*", "y", ["unit", "$", "x"]],
  'yx%': ["unit", ["*", "y", "x"], "%"],
  'yx deg': ["unit", ["*", "y", "x"], "deg"],
  'y$xz': ["*", "y", ["unit", "$", ["*", "x", "z"]]],
  'yx%z': ["*", ["unit", ["*", "y", "x"], "%"], "z"],
  'yx deg z': ["*", ["unit", ["*", "y", "x"], "deg"], "z"],
  '$x%': ["unit", "$", ["unit", "x", "%"]],
  '%x$': ["*", "%", "x", "$"],
  'x%y$z': ["*", ["unit", "x", "%"], "y", ["unit", "$", "z"]],

};

Object.keys(trees).forEach(function (string) {
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
  // 'sin': "Unexpected end of input",
  // 'sin+cos': "Invalid location of '+'",
  '\\cos(x)': "Invalid symbol '\\'",
}

Object.keys(bad_inputs).forEach(function (string) {
  test("throws " + string, function () {
    expect(() => { converter.convert(string) }).toThrow(bad_inputs[string]);
  });
});



test("split symbols", function () {

  let converter_default = new textToAst();
  let converter_split = new textToAst({ splitSymbols: true });
  let converter_nosplit = new textToAst({ splitSymbols: false });

  expect(converter_default.convert('xzy')).toEqual(['*', 'x', 'z', 'y']);
  expect(converter_split.convert('xzy')).toEqual(['*', 'x', 'z', 'y']);
  expect(converter_nosplit.convert('xzy')).toEqual('xzy');

});

test("unsplit symbols", function () {

  let converter = new textToAst({ unsplitSymbols: [] });
  expect(converter.convert('3pi')).toEqual(['*', 3, 'p', 'i']);

  converter = new textToAst({ unsplitSymbols: ['pi'] });
  expect(converter.convert('3pi')).toEqual(['*', 3, 'pi']);

});

test("function symbols", function () {

  let converter = new textToAst({ functionSymbols: [] });
  expect(converter.convert('f(x)+h(y)')).toEqual(
    ['+', ['*', 'f', 'x'], ['*', 'h', 'y']]);

  converter = new textToAst({ functionSymbols: ['f'] });
  expect(converter.convert('f(x)+h(y)')).toEqual(
    ['+', ['apply', 'f', 'x'], ['*', 'h', 'y']]);

  converter = new textToAst({ functionSymbols: ['f', 'h'] });
  expect(converter.convert('f(x)+h(y)')).toEqual(
    ['+', ['apply', 'f', 'x'], ['apply', 'h', 'y']]);

  converter = new textToAst({ functionSymbols: ['f', 'h', 'x'] });
  expect(converter.convert('f(x)+h(y)')).toEqual(
    ['+', ['apply', 'f', 'x'], ['apply', 'h', 'y']]);

});

test("applied function symbols", function () {

  let converter = new textToAst({ appliedFunctionSymbols: [] });
  expect(converter.convert('sin(x) + custom(y)')).toEqual(
    ['+', ['*', 's', 'i', 'n', 'x'], ['*', 'c', 'u', 's', 't', 'o', 'm', 'y']]);
  expect(converter.convert('sin x  + custom y')).toEqual(
    ['+', ['*', 's', 'i', 'n', 'x'], ['*', 'c', 'u', 's', 't', 'o', 'm', 'y']]);

  converter = new textToAst({ appliedFunctionSymbols: ['custom'] });
  expect(converter.convert('sin(x) + custom(y)')).toEqual(
    ['+', ['*', 's', 'i', 'n', 'x'], ['apply', 'custom', 'y']]);
  expect(converter.convert('sin x  + custom y')).toEqual(
    ['+', ['*', 's', 'i', 'n', 'x'], ['apply', 'custom', 'y']]);

  converter = new textToAst({ appliedFunctionSymbols: ['custom', 'sin'] });
  expect(converter.convert('sin(x) + custom(y)')).toEqual(
    ['+', ['apply', 'sin', 'x'], ['apply', 'custom', 'y']]);
  expect(converter.convert('sin x  + custom y')).toEqual(
    ['+', ['apply', 'sin', 'x'], ['apply', 'custom', 'y']]);

});

test("allow simplified function application", function () {
  let converter = new textToAst();
  expect(converter.convert('sin x')).toEqual(
    ['apply', 'sin', 'x']);

  converter = new textToAst({ allowSimplifiedFunctionApplication: false });
  expect(() => { converter.convert('sin x') }).toThrow(
    "Expecting ( after function");

  converter = new textToAst({ allowSimplifiedFunctionApplication: true });
  expect(converter.convert('sin x')).toEqual(
    ['apply', 'sin', 'x']);

});

test("parse Leibniz notation", function () {

  let converter = new textToAst();
  expect(converter.convert('dy/dx')).toEqual(
    ['derivative_leibniz', 'y', ['tuple', 'x']]);

  converter = new textToAst({ parseLeibnizNotation: false });
  expect(converter.convert('dy/dx')).toEqual(
    ['*', ['/', ['*', 'd', 'y'], 'd'], 'x']);

  converter = new textToAst({ parseLeibnizNotation: true });
  expect(converter.convert('dy/dx')).toEqual(
    ['derivative_leibniz', 'y', ['tuple', 'x']]);

});

test("parse scientific notation", function () {

  let converter = new textToAst();
  expect(converter.convert('2E^2-3E+2')).toEqual(
    ['+', ['*', 2, ["^", "E", 2]], -300]);

  converter = new textToAst({ parseScientificNotation: false });
  expect(converter.convert('2E^2-3E+2')).toEqual(
    ['+', ['*', 2, ["^", "E", 2]], ['-', ['*', 3, "E"]], 2]);

  converter = new textToAst({ parseScientificNotation: true });
  expect(converter.convert('2E^2-3E+2')).toEqual(
    ['+', ['*', 2, ["^", "E", 2]], -300]);

});



test("conditional probability", function () {

  let converter = new textToAst({ functionSymbols: ["P"] });

  expect(converter.convert("P(A|B)")).toEqual(
    ['apply', 'P', ['|', 'A', 'B']]);

  expect(converter.convert("P(A:B)")).toEqual(
    ['apply', 'P', [':', 'A', 'B']]);

  expect(converter.convert("P(R=1|X>2)")).toEqual(
    ['apply', 'P', ['|', ['=', 'R', 1], ['>', 'X', 2]]]);

  expect(converter.convert("P(R=1:X>2)")).toEqual(
    ['apply', 'P', [':', ['=', 'R', 1], ['>', 'X', 2]]]);

  expect(converter.convert("P( A and B | C or D )")).toEqual(
    ['apply', 'P', ['|', ['and', 'A', 'B'], ['or', 'C', 'D']]]);

  expect(converter.convert("P( A and B : C or D )")).toEqual(
    ['apply', 'P', [':', ['and', 'A', 'B'], ['or', 'C', 'D']]]);

});
