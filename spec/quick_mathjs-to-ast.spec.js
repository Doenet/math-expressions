import mathjsToAst from "../lib/converters/mathjs-to-ast";
import math from "../lib/mathjs";
import flatten from "../lib/converters/flatten";
var converter = new mathjsToAst();

var trees = {
  "1+x+3": ["+", 1, "x", 3],
  "1 + - x": ["+", 1, ["-", "x"]],
  "1 - x": ["+", 1, ["-", "x"]],
  "1 - - x": ["+", 1, ["-", ["-", "x"]]],
  "1 + x/2": ["+", 1, ["/", "x", 2]],
  "1-x-3": ["+", 1, ["-", "x"], ["-", 3]],
  "x^2": ["^", "x", 2],
  "-x^2": ["-", ["^", "x", 2]],
  "-3^2": ["-", ["^", 3, 2]],
  "x^47": ["^", "x", 47],
  "x^ab": ["^", "x", "ab"],
  "x^a!": ["^", "x", ["apply", "factorial", "a"]],
  "x*y*z": ["*", "x", "y", "z"],
  // 'xyz': ['*','x','y','z'],
  xyz2: "xyz2",
  // 'in': ['*', 'i', 'n'],
  // 'ni': ['*', 'n', 'i'],
  "x*y*z*w": ["*", "x", "y", "z", "w"],
  // 'xyzw': ['*','x', 'y', 'z', 'w'],
  "(x*y)*(z*w)": ["*", "x", "y", "z", "w"],
  "c*(a+b)": ["*", "c", ["+", "a", "b"]],
  "(a+b)*c": ["*", ["+", "a", "b"], "c"],
  // '|x|': ['apply', 'abs','x'],
  "a!": ["apply", "factorial", "a"],
  theta: "theta",
  "cos(theta)": ["apply", "cos", "theta"],
  "x!": ["apply", "factorial", "x"],
  // '|sin(|x|)|': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
  "sin(θ)": ["apply", "sin", "θ"],
  // '|x+3=2|': ['apply', 'abs', ['=', ['+', 'x', 3], 2]],
  // 'x_y_z': ['_', 'x', ['_','y','z']],
  // 'x_(y_z)': ['_', 'x', ['_','y','z']],
  // '(x_y)_z': ['_', ['_', 'x', 'y'],'z'],
  // 'x^y^z': ['^', 'x', ['^','y','z']],
  // 'x^(y^z)': ['^', 'x', ['^','y','z']],
  // '(x^y)^z': ['^', ['^', 'x', 'y'],'z'],
  // 'x^y_z': ['^', 'x', ['_','y','z']],
  // 'x_y^z': ['^', ['_','x','y'],'z'],
  // 'xyz!': ['*','x','y', ['apply', 'factorial', 'z']],
  x: "x",
  f: "f",
  // 'fg': ['*', 'f','g'],
  "f+g": ["+", "f", "g"],
  "f(x)": ["apply", "f", "x"],
  "f(x,y,z)": ["apply", "f", ["tuple", "x", "y", "z"]],
  // 'fg(x)': ['*', 'f', ['apply', 'g', 'x']],
  // 'fp(x)': ['*', 'f', 'p', 'x'],
  // 'fx': ['*', 'f', 'x'],
  // 'f\'': ['prime', 'f'],
  // 'fg\'': ['*', 'f', ['prime', 'g']],
  // 'f\'g': ['*', ['prime', 'f'], 'g'],
  // 'f\'g\'\'': ['*', ['prime', 'f'], ['prime', ['prime', 'g']]],
  // 'x\'': ['prime', 'x'],
  // 'f\'(x)' : ['apply', ['prime', 'f'], 'x'],
  // 'f(x)\'' : ['prime', ['apply', 'f', 'x']],
  // 'sin(x)\'': ['prime', ['apply', 'sin', 'x']],
  // 'sin\'(x)': ['apply', ['prime', 'sin'], 'x'],
  // 'f\'\'(x)': ['apply', ['prime', ['prime', 'f']],'x'],
  // 'sin(x)\'\'': ['prime', ['prime', ['apply','sin','x']]],
  // 'f(x)^t_y': ['^', ['apply', 'f','x'], ['_','t','y']],
  // 'f_t(x)': ['apply', ['_', 'f', 't'], 'x'],
  // 'f(x)_t': ['_', ['apply', 'f', 'x'], 't'],
  // 'f^2(x)': ['apply', ['^', 'f', 2], 'x'],
  // 'f(x)^2': ['^', ['apply', 'f', 'x'],2],
  // 'f\'^a(x)': ['apply', ['^', ['prime', 'f'], 'a'], 'x'],
  // 'f^a\'(x)': ['apply', ['^', 'f', ['prime', 'a']], 'x'],
  // 'f_a^b\'(x)': ['apply', ['^', ['_', 'f', 'a'], ['prime', 'b']],'x'],
  // 'f_a\'^b(x)': ['apply', ['^', ['prime', ['_', 'f','a']],'b'],'x'],
  // 'sin x': ['apply', 'sin', 'x'],
  // 'f x': ['*', 'f', 'x'],
  // 'sin^xyz': ['*', ['apply', ['^', 'sin', 'x'], 'y'], 'z'],
  // 'sin xy': ['*', ['apply', 'sin', 'x'], 'y'],
  // 'sin^2(x)': ['apply', ['^', 'sin', 2], 'x'],
  // 'x^2!': ['^', 'x', ['apply', 'factorial', 2]],
  // 'x^2!!': ['^', 'x', ['apply', 'factorial', ['apply', 'factorial', 2]]],
  // 'x_t^2': ['^', ['_', 'x', 't'], 2],
  // 'x_f^2': ['_', 'x', ['^', 'f', 2]],
  // 'x_t\'': ['prime', ['_', 'x', 't']],
  // 'x_f\'': ['_', 'x', ['prime', 'f']],
  // '(x,y,z)': ['tuple', 'x', 'y', 'z'],
  // '(x,y)-[x,y]': ['+', ['tuple','x','y'], ['-', ['array','x','y']]],
  // '2[z-(x+1)]': ['*', 2, ['+', 'z', ['-', ['+', 'x', 1]]]],
  // '{1,2,x}': ['set', 1, 2, 'x'],
  // '{x, x}': ['set', 'x', 'x'],
  // '{x}': ['set', 'x'],
  // '(1,2]': ['interval', ['tuple', 1, 2], ['tuple', false, true]],
  // '[1,2)': ['interval', ['tuple', 1, 2], ['tuple', true, false]],
  // '[1,2]': ['array', 1, 2 ],
  // '(1,2)': ['tuple', 1, 2 ],
  // '1,2,3': ['list', 1, 2, 3],
  // 'x=a': ['=', 'x', 'a'],
  // 'x=y=1': ['=', 'x', 'y', 1],
  // 'x=(y=1)': ['=', 'x', ['=', 'y', 1]],
  // '(x=y)=1': ['=', ['=','x', 'y'], 1],
  // '7 != 2': ['ne', 7, 2],
  // '7 ≠ 2': ['ne', 7, 2],
  // 'not x=y': ['not', ['=', 'x', 'y']],
  // '!x=y': ['not', ['=', 'x', 'y']],
  // '!(x=y)': ['not', ['=', 'x', 'y']],
  // 'x>y': ['>', 'x','y'],
  // 'x>=y': ['ge', 'x','y'],
  // 'x≥y': ['ge', 'x','y'],
  // 'x>y>z': ['gts', ['tuple', 'x', 'y','z'], ['tuple', true, true]],
  // 'x>y>=z': ['gts', ['tuple', 'x', 'y','z'], ['tuple', true, false]],
  // 'x>=y>z': ['gts', ['tuple', 'x', 'y','z'], ['tuple', false, true]],
  // 'x>=y>=z': ['gts', ['tuple', 'x', 'y','z'], ['tuple', false, false]],
  // 'x<y': ['<', 'x','y'],
  // 'x<=y': ['le', 'x','y'],
  // 'x≤y': ['le', 'x','y'],
  // 'x<y<z': ['lts', ['tuple', 'x', 'y','z'], ['tuple', true, true]],
  // 'x<y<=z': ['lts', ['tuple', 'x', 'y','z'], ['tuple', true, false]],
  // 'x<=y<z': ['lts', ['tuple', 'x', 'y', 'z'], ['tuple', false, true]],
  // 'x<=y<=z': ['lts', ['tuple', 'x', 'y', 'z'], ['tuple', false, false]],
  // 'x<y>z': ['>', ['<', 'x', 'y'], 'z'],
  // 'A subset B': ['subset', 'A', 'B'],
  // 'A ⊂ B': ['subset', 'A', 'B'],
  // 'A notsubset B': ['notsubset', 'A', 'B'],
  // 'A ⊄ B': ['notsubset', 'A', 'B'],
  // 'A superset B': ['superset', 'A', 'B'],
  // 'A ⊃ B': ['superset', 'A', 'B'],
  // 'A notsuperset B': ['notsuperset', 'A', 'B'],
  // 'A ⊅ B': ['notsuperset', 'A', 'B'],
  // 'x elementof A': ['in', 'x', 'A'],
  // 'x ∈ A': ['in', 'x', 'A'],
  // 'x notelementof A': ['notin', 'x', 'A'],
  // 'x ∉ A': ['notin', 'x', 'A'],
  // 'A containselement x': ['ni', 'A', 'x'],
  // 'A ∋ x': ['ni', 'A', 'x'],
  // 'A notcontainselement x': ['notni', 'A', 'x'],
  // 'A ∌ x': ['notni', 'A', 'x'],
  // 'A union B': ['union', 'A', 'B'],
  // 'A ∪ B': ['union', 'A', 'B'],
  // 'A intersect B': ['intersect', 'A', 'B'],
  // 'A ∩ B': ['intersect', 'A', 'B'],
  // 'A and B': ['and', 'A', 'B'],
  // 'A & B': ['and', 'A', 'B'],
  // 'A && B': ['and', 'A', 'B'],
  // 'A ∧ B': ['and', 'A', 'B'],
  // 'A or B': ['or', 'A', 'B'],
  // 'A ∨ B': ['or', 'A', 'B'],
  // 'A ∧ B ∧ C': ['and', 'A', 'B', 'C'],
  // 'A ∨ B ∨ C': ['or', 'A', 'B', 'C'],
  // 'A and B or C': ['or', ['and', 'A', 'B'], 'C'],
  // 'A or B and C': ['or', 'A', ['and', 'B', 'C']],
  // '!x=1': ['not', ['=', 'x', 1]],
  // '!(x=1)': ['not', ['=', 'x', 1]],
  // '!(x=y) or z != w': ['or', ['not', ['=','x','y']], ['ne','z','w']],
  // '1.2E3': 1200,
  // '1.2E+3': 1200,
  // '3.1E-3': 0.0031,
  // '1.2e-3': ['+', ['*', 1.2, 'e'], ['-', 3]],
  // '+2': 2,
  // 'oo': Infinity,
  // '+oo': Infinity,
  // 'dx/dt': ['derivative_leibniz', 'x', 't'],
  // 'dx / dt': ['derivative_leibniz', 'x', 't'],
  // 'd x/dt': ["*", ["/", ["*", "d", "x"], "d"], "t"],
  // '(dx)/(dt)': ["/", ["*", "d", "x"], ["*", "d", "t"]],
  // 'dx_2/dt': ["*", ["/", ["*", "d", ["_", "x", 2]], "d"], "t"],
  // 'd^2x/dt^2': ['derivative_leibniz_mult', 2, 'x', 't'],
  // 'd^2x/dt^3': ["*", ["/", ["*", ["^", "d", 2], "x"], "d"], ["^", "t", 3]],
};

Object.keys(trees).forEach(function (string) {
  test("parses " + string, () => {
    expect(flatten(converter.convert(math.parse(string)))).toEqual(
      trees[string],
    );
  });
});

// for (let objectToTest of objectsToTest) {
//   test("parses " + objectToTest.mathjs + ' to ' + objectToTest.ast, () => {
//     expect(converter.convert(JSON.parse(objectToTest.mathjs,math.json.reviver))).toEqual(objectToTest.ast);
//   });
// }
