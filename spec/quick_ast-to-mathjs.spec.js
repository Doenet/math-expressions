import astToMathjs from '../lib/converters/ast-to-mathjs';

var converter = new astToMathjs();


const objectsToTest = [
  {
    'ast': ['*', ['/', 1, 2], 'x'],
    'mathjs': {"args": [{"args": [{"value": 1}, {"value": 2}], "fn": "divide", "implicit": false, "op": "/"}, {"name": "x"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['+', 1, 'x', 3],
    'mathjs': {"args": [{"value": 1}, {"name": "x"}, {"value": 3}], "fn": "add", "implicit": false, "op": "+"}
  },
  {
    'ast': ['+', 1, ['-', 'x'],
      ['-', 3]
    ],
    'mathjs': {"args": [{"value": 1}, {"args": [{"name": "x"}], "fn": "unaryMinus", "implicit": false, "op": "-"}, {"args": [{"value": 3}], "fn": "unaryMinus", "implicit": false, "op": "-"}], "fn": "add", "implicit": false, "op": "+"}
  },
  {
    'ast': ['+', 1, ['-', ['-', 'x']]],
    'mathjs': {"args": [{"value": 1}, {"args": [{"args": [{"name": "x"}], "fn": "unaryMinus", "implicit": false, "op": "-"}], "fn": "unaryMinus", "implicit": false, "op": "-"}], "fn": "add", "implicit": false, "op": "+"}
  },
  {
    'ast': ['apply', 'log', 'x'],
    'mathjs': {"args": [{"name": "x"}], "fn": {"name": "log"}}
  },
  {
    'ast': ['apply', 'ln', 'x'],
    'mathjs': {"args": [{"name": "x"}], "fn": {"name": "ln"}}
  },
  {
    'ast': ['apply', 'abs', 'x'],
    'mathjs': {"args": [{"name": "x"}], "fn": {"name": "abs"}}
  },
  {
    'ast': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
    'mathjs': {"args": [{"args": [{"args": [{"name": "x"}], "fn": {"name": "abs"}}], "fn": {"name": "sin"}}], "fn": {"name": "abs"}}
  },
  {
    'ast': ['^', 'x', 2],
    'mathjs': {"args": [{"name": "x"}, {"value": 2}], "fn": "pow", "implicit": false, "op": "^"}
  },
  {
    'ast': ['-', ['^', 'x', 2]],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"value": 2}], "fn": "pow", "implicit": false, "op": "^"}], "fn": "unaryMinus", "implicit": false, "op": "-"}
  },
  {
    'ast': ['^', 'x', 47],
    'mathjs': {"args": [{"name": "x"}, {"value": 47}], "fn": "pow", "implicit": false, "op": "^"}
  },
  {
    'ast': ['*', ['^', 'x', 'a'], 'b'],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"name": "a"}], "fn": "pow", "implicit": false, "op": "^"}, {"name": "b"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['^', 'x', ['apply', 'factorial', 'a']],
    'mathjs': {"args": [{"name": "x"}, {"args": [{"name": "a"}], "fn": "factorial", "implicit": false, "op": "!"}], "fn": "pow", "implicit": false, "op": "^"}
  },
  {
    'ast': ['vector', 1, 2],
    'mathjs': {"items": [{"value": 1}, {"value": 2}]}
  },
  {
    'ast': ['*', 'x', 'y', 'z'],
    'mathjs': {"args": [{"name": "x"}, {"name": "y"}, {"name": "z"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['*', 'c', ['+', 'a', 'b']],
    'mathjs': {"args": [{"name": "c"}, {"args": [{"name": "a"}, {"name": "b"}], "fn": "add", "implicit": false, "op": "+"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['*', ['+', 'a', 'b'], 'c'],
    'mathjs': {"args": [{"args": [{"name": "a"}, {"name": "b"}], "fn": "add", "implicit": false, "op": "+"}, {"name": "c"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['apply', 'factorial', 'a'],
    'mathjs': {"args": [{"name": "a"}], "fn": "factorial", "implicit": false, "op": "!"}
  },
  {
    'ast': 'theta',
    'mathjs': {"name": "theta"}
  },
  {
    'ast': ['*', 't', 'h', 'e', 't', 'a'],
    'mathjs': {"args": [{"name": "t"}, {"name": "h"}, {"name": "e"}, {"name": "t"}, {"name": "a"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['apply', 'cos', 'theta'],
    'mathjs': {"args": [{"name": "theta"}], "fn": {"name": "cos"}}
  },
  {
    'ast': ['*', 'c', 'o', 's', 'x'],
    'mathjs': {"args": [{"name": "c"}, {"name": "o"}, {"name": "s"}, {"name": "x"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
    'mathjs': {"args": [{"args": [{"args": [{"name": "x"}], "fn": {"name": "abs"}}], "fn": {"name": "sin"}}], "fn": {"name": "abs"}}
  },
  {
    'ast': ['*', 'blah', 'x'],
    'mathjs': {"args": [{"name": "blah"}, {"name": "x"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['apply', 'abs', ['=', ['+', 'x', 3], 2]],
    'mathjs': {"args": [{"args": [{"args": [{"name": "x"}, {"value": 3}], "fn": "add", "implicit": false, "op": "+"}, {"value": 2}], "fn": "equal", "implicit": false, "op": "=="}], "fn": {"name": "abs"}}
  },
  {
    'ast': ['_', 'x', ['_', 'y', 'z']],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['_', 'x', ['_', 'y', 'z']],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['_', ['_', 'x', 'y'], 'z'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['^', 'x', ['^', 'y', 'z']],
    'mathjs': {"args": [{"name": "x"}, {"args": [{"name": "y"}, {"name": "z"}], "fn": "pow", "implicit": false, "op": "^"}], "fn": "pow", "implicit": false, "op": "^"}
  },
  {
    'ast': ['^', 'x', ['^', 'y', 'z']],
    'mathjs': {"args": [{"name": "x"}, {"args": [{"name": "y"}, {"name": "z"}], "fn": "pow", "implicit": false, "op": "^"}], "fn": "pow", "implicit": false, "op": "^"}
  },
  {
    'ast': ['^', ['^', 'x', 'y'], 'z'],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"name": "y"}], "fn": "pow", "implicit": false, "op": "^"}, {"name": "z"}], "fn": "pow", "implicit": false, "op": "^"}
  },
  {
    'ast': ['^', 'x', ['_', 'y', 'z']],
    'mathjs': {"args": [{"name": "x"}, {"name": "NaN"}], "fn": "pow", "implicit": false, "op": "^"}
  },
  {
    'ast': ['^', ['_', 'x', 'y'], 'z'],
    'mathjs':  {"args": [{"name": "NaN"}, {"name": "z"}], "fn": "pow", "implicit": false, "op": "^"}
  },
  {
    'ast': ['*', 'x', 'y', ['apply', 'factorial', 'z']],
    'mathjs': {"args": [{"name": "x"}, {"name": "y"}, {"args": [{"name": "z"}], "fn": "factorial", "implicit": false, "op": "!"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': 'x',
    'mathjs': {"name": "x"}
  },
  {
    'ast': 'f',
    'mathjs': {"name": "f"}
  },
  {
    'ast': ['*', 'f', 'g'],
    'mathjs':  {"args": [{"name": "f"}, {"name": "g"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['+', 'f', 'g'],
    'mathjs': {"args": [{"name": "f"}, {"name": "g"}], "fn": "add", "implicit": false, "op": "+"}
  },
  {
    'ast': ['apply', 'f', 'x'],
    'mathjs': {"args": [{"name": "x"}], "fn": {"name": "f"}}
  },
  {
    'ast': ['apply', 'f', ['tuple', 'x', 'y', 'z']],
    'mathjs': {"args": [{"name": "x"}, {"name": "y"}, {"name": "z"}], "fn": {"name": "f"}}
  },
  {
    'ast': ['*', 'f', ['apply', 'g', 'x']],
    'mathjs': {"args": [{"name": "f"}, {"args": [{"name": "x"}], "fn": {"name": "g"}}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['*', 'f', 'p', 'x'],
    'mathjs': {"args": [{"name": "f"}, {"name": "p"}, {"name": "x"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['*', 'f', 'x'],
    'mathjs': {"args": [{"name": "f"}, {"name": "x"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['prime', 'f'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['*', 'f', ['prime', 'g']],
    'mathjs': {"args": [{"name": "f"}, {"name": "NaN"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['*', ['prime', 'f'], 'g'],
    'mathjs': {"args": [{"name": "NaN"}, {"name": "g"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['*', ['prime', 'f'],
      ['prime', ['prime', 'g']]
    ],
    'mathjs': {"args": [{"name": "NaN"}, {"name": "NaN"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['prime', 'x'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['apply', ['prime', 'f'], 'x'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['prime', ['apply', 'f', 'x']],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['prime', ['apply', 'sin', 'x']],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['apply', ['prime', 'sin'], 'x'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['apply', ['prime', ['prime', 'f']], 'x'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['prime', ['prime', ['apply', 'sin', 'x']]],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['^', ['apply', 'f', 'x'],
      ['_', 't', 'y']
    ],
    'mathjs': {"args": [{"args": [{"name": "x"}], "fn": {"name": "f"}}, {"name": "NaN"}], "fn": "pow", "implicit": false, "op": "^"}
  },
  {
    'ast': ['apply', ['_', 'f', 't'], 'x'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['_', ['apply', 'f', 'x'], 't'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['apply', ['^', 'f', 2], 'x'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['^', ['apply', 'f', 'x'], 2],
    'mathjs': {"args": [{"args": [{"name": "x"}], "fn": {"name": "f"}}, {"value": 2}], "fn": "pow", "implicit": false, "op": "^"}
  },
  {
    'ast': ['apply', ['^', ['prime', 'f'], 'a'], 'x'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['apply', ['^', 'f', ['prime', 'a']], 'x'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['apply', ['^', ['_', 'f', 'a'],
      ['prime', 'b']
    ], 'x'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['apply', ['^', ['prime', ['_', 'f', 'a']], 'b'], 'x'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['apply', 'sin', 'x'],
    'mathjs': {"args": [{"name": "x"}], "fn": {"name": "sin"}}
  },
  {
    'ast': ['*', ['apply', ['^', 'sin', 'x'], 'y'], 'z'],
    'mathjs': {"args": [{"name": "NaN"}, {"name": "z"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['*', ['apply', 'sin', 'x'], 'y'],
    'mathjs': {"args": [{"args": [{"name": "x"}], "fn": {"name": "sin"}}, {"name": "y"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['apply', ['^', 'sin', 2], 'x'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['apply', 'exp', 'x'],
    'mathjs': {"args": [{"name": "x"}], "fn": {"name": "exp"}}
  },
  {
    'ast': ['^', 'e', 'x'],
    'mathjs': {"args": [{"name": "e"}, {"name": "x"}], "fn": "pow", "implicit": false, "op": "^"}
  },
  {
    'ast': ['^', 'x', ['apply', 'factorial', 2]],
    'mathjs': {"args": [{"name": "x"}, {"args": [{"value": 2}], "fn": "factorial", "implicit": false, "op": "!"}], "fn": "pow", "implicit": false, "op": "^"}
  },
  {
    'ast': ['^', 'x', ['apply', 'factorial', ['apply', 'factorial', 2]]],
    'mathjs': {"args": [{"name": "x"}, {"args": [{"args": [{"value": 2}], "fn": "factorial", "implicit": false, "op": "!"}], "fn": "factorial", "implicit": false, "op": "!"}], "fn": "pow", "implicit": false, "op": "^"}
  },
  {
    'ast': ['^', ['_', 'x', 't'], 2],
    'mathjs': {"args": [{"name": "NaN"}, {"value": 2}], "fn": "pow", "implicit": false, "op": "^"}
  },
  {
    'ast': ['_', 'x', ['^', 'f', 2]],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['prime', ['_', 'x', 't']],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['_', 'x', ['prime', 'f']],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['tuple', 'x', 'y', 'z'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['+', ['tuple', 'x', 'y'],
      ['-', ['array', 'x', 'y']]
    ],
    'mathjs': {"args": [{"name": "NaN"}, {"args": [{"name": "NaN"}], "fn": "unaryMinus", "implicit": false, "op": "-"}], "fn": "add", "implicit": false, "op": "+"}
  },
  {
    'ast': ['*', 2, ['+', 'z', ['-', ['+', 'x', 1]]]],
    'mathjs': {"args": [{"value": 2}, {"args": [{"name": "z"}, {"args": [{"args": [{"name": "x"}, {"value": 1}], "fn": "add", "implicit": false, "op": "+"}], "fn": "unaryMinus", "implicit": false, "op": "-"}], "fn": "add", "implicit": false, "op": "+"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': ['set', 1, 2, 'x'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['set', 'x', 'x'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['set', 'x'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['interval', ['tuple', 1, 2],
      ['tuple', false, true]
    ],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['array', 1, 2],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['tuple', 1, 2],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['list', 1, 2, 3],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['=', 'x', 'a'],
    'mathjs': {"args": [{"name": "x"}, {"name": "a"}], "fn": "equal", "implicit": false, "op": "=="}
  },
  {
    'ast': ['=', 'x', 'y', 1],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"name": "y"}], "fn": "equal", "implicit": false, "op": "=="}, {"args": [{"name": "y"}, {"value": 1}], "fn": "equal", "implicit": false, "op": "=="}], "fn": "and", "implicit": false, "op": "and"}
  },
  {
    'ast': ['=', 'x', ['=', 'y', 1]],
    'mathjs': {"args": [{"name": "x"}, {"args": [{"name": "y"}, {"value": 1}], "fn": "equal", "implicit": false, "op": "=="}], "fn": "equal", "implicit": false, "op": "=="}
  },
  {
    'ast': ['=', ['=', 'x', 'y'], 1],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"name": "y"}], "fn": "equal", "implicit": false, "op": "=="}, {"value": 1}], "fn": "equal", "implicit": false, "op": "=="}
  },
  {
    'ast': ['ne', 7, 2],
    'mathjs': {"args": [{"value": 7}, {"value": 2}], "fn": "unequal", "implicit": false, "op": "!="}
  },
  {
    'ast': ['not', ['=', 'x', 'y']],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"name": "y"}], "fn": "equal", "implicit": false, "op": "=="}], "fn": "not", "implicit": false, "op": "not"}
  },
  {
    'ast': ['not', ['=', 'x', 'y']],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"name": "y"}], "fn": "equal", "implicit": false, "op": "=="}], "fn": "not", "implicit": false, "op": "not"}
  },
  {
    'ast': ['>', 'x', 'y'],
    'mathjs': {"args": [{"name": "x"}, {"name": "y"}], "fn": "larger", "implicit": false, "op": ">"}
  },
  {
    'ast': ['ge', 'x', 'y'],
    'mathjs': {"args": [{"name": "x"}, {"name": "y"}], "fn": "largerEq", "implicit": false, "op": ">="}
  },
  {
    'ast': ['gts', ['tuple', 'x', 'y', 'z'],
      ['tuple', true, true]
    ],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"name": "y"}], "fn": "larger", "implicit": false, "op": ">"}, {"args": [{"name": "y"}, {"name": "z"}], "fn": "larger", "implicit": false, "op": ">"}], "fn": "and", "implicit": false, "op": "and"}
  },
  {
    'ast': ['gts', ['tuple', 'x', 'y', 'z'],
      ['tuple', true, false]
    ],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"name": "y"}], "fn": "larger", "implicit": false, "op": ">"}, {"args": [{"name": "y"}, {"name": "z"}], "fn": "largerEq", "implicit": false, "op": ">="}], "fn": "and", "implicit": false, "op": "and"}
  },
  {
    'ast': ['gts', ['tuple', 'x', 'y', 'z'],
      ['tuple', false, true]
    ],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"name": "y"}], "fn": "largerEq", "implicit": false, "op": ">="}, {"args": [{"name": "y"}, {"name": "z"}], "fn": "larger", "implicit": false, "op": ">"}], "fn": "and", "implicit": false, "op": "and"}
  },
  {
    'ast': ['gts', ['tuple', 'x', 'y', 'z'],
      ['tuple', false, false]
    ],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"name": "y"}], "fn": "largerEq", "implicit": false, "op": ">="}, {"args": [{"name": "y"}, {"name": "z"}], "fn": "largerEq", "implicit": false, "op": ">="}], "fn": "and", "implicit": false, "op": "and"}
  },
  {
    'ast': ['<', 'x', 'y'],
    'mathjs': {"args": [{"name": "x"}, {"name": "y"}], "fn": "smaller", "implicit": false, "op": "<"}
  },
  {
    'ast': ['le', 'x', 'y'],
    'mathjs': {"args": [{"name": "x"}, {"name": "y"}], "fn": "smallerEq", "implicit": false, "op": "<="}
  },
  {
    'ast': ['lts', ['tuple', 'x', 'y', 'z'],
      ['tuple', true, true]
    ],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"name": "y"}], "fn": "smaller", "implicit": false, "op": "<"}, {"args": [{"name": "y"}, {"name": "z"}], "fn": "smaller", "implicit": false, "op": "<"}], "fn": "and", "implicit": false, "op": "and"}
  },
  {
    'ast': ['lts', ['tuple', 'x', 'y', 'z'],
      ['tuple', true, false]
    ],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"name": "y"}], "fn": "smaller", "implicit": false, "op": "<"}, {"args": [{"name": "y"}, {"name": "z"}], "fn": "smallerEq", "implicit": false, "op": "<="}], "fn": "and", "implicit": false, "op": "and"}
  },
  {
    'ast': ['lts', ['tuple', 'x', 'y', 'z'],
      ['tuple', false, true]
    ],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"name": "y"}], "fn": "smallerEq", "implicit": false, "op": "<="}, {"args": [{"name": "y"}, {"name": "z"}], "fn": "smaller", "implicit": false, "op": "<"}], "fn": "and", "implicit": false, "op": "and"}
  },
  {
    'ast': ['lts', ['tuple', 'x', 'y', 'z'],
      ['tuple', false, false]
    ],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"name": "y"}], "fn": "smallerEq", "implicit": false, "op": "<="}, {"args": [{"name": "y"}, {"name": "z"}], "fn": "smallerEq", "implicit": false, "op": "<="}], "fn": "and", "implicit": false, "op": "and"}
  },
  {
    'ast': ['>', ['<', 'x', 'y'], 'z'],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"name": "y"}], "fn": "smaller", "implicit": false, "op": "<"}, {"name": "z"}], "fn": "larger", "implicit": false, "op": ">"}
  },
  // {
  //   'ast': ['subset', 'A', 'B'],
  //   'mathjs': 'A ⊂ B'
  // },
  // {
  //   'ast': ['notsubset', 'A', 'B'],
  //   'mathjs': 'A ⊄ B'
  // },
  // {
  //   'ast': ['superset', 'A', 'B'],
  //   'mathjs': 'A ⊃ B'
  // },
  // {
  //   'ast': ['notsuperset', 'A', 'B'],
  //   'mathjs': 'A ⊅ B'
  // },
  // {
  //   'ast': ['in', 'x', 'A'],
  //   'mathjs': 'x ∈ A'
  // },
  // {
  //   'ast': ['notin', 'x', 'A'],
  //   'mathjs': 'x ∉ A'
  // },
  // {
  //   'ast': ['ni', 'A', 'x'],
  //   'mathjs': 'A ∋ x'
  // },
  // {
  //   'ast': ['notni', 'A', 'x'],
  //   'mathjs': 'A ∌ x'
  // },
  // {
  //   'ast': ['union', 'A', 'B'],
  //   'mathjs': 'A ∪ B'
  // },
  // {
  //   'ast': ['intersect', 'A', 'B'],
  //   'mathjs': 'A ∩ B'
  // },
  {
    'ast': ['and', 'A', 'B'],
    'mathjs': {"args": [{"name": "A"}, {"name": "B"}], "fn": "and", "implicit": false, "op": "and"}
  },
  {
    'ast': ['and', 'A', 'B'],
    'mathjs': {"args": [{"name": "A"}, {"name": "B"}], "fn": "and", "implicit": false, "op": "and"}
  },
  {
    'ast': ['or', 'A', 'B'],
    'mathjs': {"args": [{"name": "A"}, {"name": "B"}], "fn": "or", "implicit": false, "op": "or"}
  },
  {
    'ast': ['and', 'A', 'B', 'C'],
    'mathjs': {"args": [{"name": "A"}, {"name": "B"}, {"name": "C"}], "fn": "and", "implicit": false, "op": "and"}
  },
  {
    'ast': ['or', 'A', 'B', 'C'],
    'mathjs': {"args": [{"name": "A"}, {"name": "B"}, {"name": "C"}], "fn": "or", "implicit": false, "op": "or"}
  },
  {
    'ast': ['or', ['and', 'A', 'B'], 'C'],
    'mathjs': {"args": [{"args": [{"name": "A"}, {"name": "B"}], "fn": "and", "implicit": false, "op": "and"}, {"name": "C"}], "fn": "or", "implicit": false, "op": "or"}
  },
  {
    'ast': ['or', 'A', ['and', 'B', 'C']],
    'mathjs': {"args": [{"name": "A"}, {"args": [{"name": "B"}, {"name": "C"}], "fn": "and", "implicit": false, "op": "and"}], "fn": "or", "implicit": false, "op": "or"}
  },
  {
    'ast': ['not', ['=', 'x', 1]],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"value": 1}], "fn": "equal", "implicit": false, "op": "=="}], "fn": "not", "implicit": false, "op": "not"}
  },
  {
    'ast': ['not', ['=', 'x', 1]],
    'mathjs': {"args": [{"args": [{"name": "x"}, {"value": 1}], "fn": "equal", "implicit": false, "op": "=="}], "fn": "not", "implicit": false, "op": "not"}
  },
  {
    'ast': ['or', ['not', ['=', 'x', 'y']],
      ['ne', 'z', 'w']
    ],
    'mathjs': {"args": [{"args": [{"args": [{"name": "x"}, {"name": "y"}], "fn": "equal", "implicit": false, "op": "=="}], "fn": "not", "implicit": false, "op": "not"}, {"args": [{"name": "z"}, {"name": "w"}], "fn": "unequal", "implicit": false, "op": "!="}], "fn": "or", "implicit": false, "op": "or"}
  },
  {
    'ast': ['+', ['*', 1.2, 'e'],
      ['-', 3]
    ],
    'mathjs': {"args": [{"args": [{"value": 1.2}, {"name": "e"}], "fn": "multiply", "implicit": false, "op": "*"}, {"args": [{"value": 3}], "fn": "unaryMinus", "implicit": false, "op": "-"}], "fn": "add", "implicit": false, "op": "+"}
  },
  {
    'ast': 'infinity',
    'mathjs': {"name": "Infinity"}
  },
  {
    'ast': ['*', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j'],
    'mathjs': {"args": [{"name": "a"}, {"name": "b"}, {"name": "c"}, {"name": "d"}, {"name": "e"}, {"name": "f"}, {"name": "g"}, {"name": "h"}, {"name": "i"}, {"name": "j"}], "fn": "multiply", "implicit": false, "op": "*"}
  },
  {
    'ast': 2,
    'mathjs': {"value": 2}
  },
  {
    'ast': '',
    'mathjs':  {"name": ""}
  },
  {
    'ast':  ['matrix', ['tuple', 2, 2], ['tuple', ['tuple', 'a', 'b'], ['tuple', 'c', 'd']]],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['matrix', ['tuple', 1, 2], ['tuple', ['tuple', ['+', 'a', ['*', 3, 'y']], ['*', 2, ['apply', 'sin', 'theta']]]]],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['matrix', ['tuple', 2, 3], ['tuple', ['tuple', 8, 0, 0], ['tuple', 1, 2, 3]]],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['derivative_leibniz', 'x', 't'],
    'mathjs': {"name": "NaN"}
  },
  {
    'ast': ['derivative_leibniz_mult', 2, 'x', 't'],
    'mathjs': {"name": "NaN"}
  },

]


for (let objectToTest of objectsToTest) {
  test("parses " + objectToTest.ast + ' to ' + objectToTest.mathjs, () => {
    expect(converter.convert(objectToTest.ast)).toEqual(objectToTest.mathjs);
  });

}
