import astToMathjs from '../lib/converters/ast-to-mathjs';
import math from 'mathjs';
import _ from 'underscore';
var converter = new astToMathjs();
let reviver = math.json.reviver;


const objectsToTest = [
  {
    'ast': ['*', ['/', 1, 2], 'x'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"OperatorNode","op":"/","fn":"divide","args":[{"mathjs":"ConstantNode","value":1},{"mathjs":"ConstantNode","value":2}],"implicit":false},{"mathjs":"SymbolNode","name":"x"}],"implicit":false}', reviver)
  },
  {
    'ast': ['+', 1, 'x', 3],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"+","fn":"add","args":[{"mathjs":"ConstantNode","value":1},{"mathjs":"SymbolNode","name":"x"},{"mathjs":"ConstantNode","value":3}],"implicit":false}', reviver)
  },
  {
    'ast': ['+', 1, ['-', 'x'],
      ['-', 3]
    ],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"+","fn":"add","args":[{"mathjs":"ConstantNode","value":1},{"mathjs":"OperatorNode","op":"-","fn":"unaryMinus","args":[{"mathjs":"SymbolNode","name":"x"}],"implicit":false},{"mathjs":"OperatorNode","op":"-","fn":"unaryMinus","args":[{"mathjs":"ConstantNode","value":3}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['+', 1, ['-', ['-', 'x']]],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"+","fn":"add","args":[{"mathjs":"ConstantNode","value":1},{"mathjs":"OperatorNode","op":"-","fn":"unaryMinus","args":[{"mathjs":"OperatorNode","op":"-","fn":"unaryMinus","args":[{"mathjs":"SymbolNode","name":"x"}],"implicit":false}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['apply', 'log', 'x'],
    'mathjs': JSON.parse('{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"log"},"args":[{"mathjs":"SymbolNode","name":"x"}]}', reviver)
  },
  {
    'ast': ['apply', 'ln', 'x'],
    'mathjs': JSON.parse('{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"ln"},"args":[{"mathjs":"SymbolNode","name":"x"}]}', reviver)
  },
  {
    'ast': ['apply', 'abs', 'x'],
    'mathjs': JSON.parse('{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"abs"},"args":[{"mathjs":"SymbolNode","name":"x"}]}', reviver)
  },
  {
    'ast': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
    'mathjs': JSON.parse('{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"abs"},"args":[{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"sin"},"args":[{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"abs"},"args":[{"mathjs":"SymbolNode","name":"x"}]}]}]}', reviver)

  },
  {
    'ast': ['^', 'x', 2],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"^","fn":"pow","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"ConstantNode","value":2}],"implicit":false}', reviver)
  },
  {
    'ast': ['-', ['^', 'x', 2]],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"-","fn":"unaryMinus","args":[{"mathjs":"OperatorNode","op":"^","fn":"pow","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"ConstantNode","value":2}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['^', 'x', 47],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"^","fn":"pow","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"ConstantNode","value":47}],"implicit":false}', reviver)
  },
  {
    'ast': ['*', ['^', 'x', 'a'], 'b'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"OperatorNode","op":"^","fn":"pow","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"a"}],"implicit":false},{"mathjs":"SymbolNode","name":"b"}],"implicit":false}', reviver)
  },
  {
    'ast': ['^', 'x', ['apply', 'factorial', 'a']],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"^","fn":"pow","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"OperatorNode","op":"!","fn":"factorial","args":[{"mathjs":"SymbolNode","name":"a"}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['vector', 1, 2],
    'mathjs': JSON.parse('{"mathjs":"ArrayNode","items":[{"mathjs":"ConstantNode","value":1},{"mathjs":"ConstantNode","value":2}]}', reviver)
  },
  {
    'ast': ['*', 'x', 'y', 'z'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"},{"mathjs":"SymbolNode","name":"z"}],"implicit":false}', reviver)
  },
  {
    'ast': ['*', 'c', ['+', 'a', 'b']],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"SymbolNode","name":"c"},{"mathjs":"OperatorNode","op":"+","fn":"add","args":[{"mathjs":"SymbolNode","name":"a"},{"mathjs":"SymbolNode","name":"b"}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['*', ['+', 'a', 'b'], 'c'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"OperatorNode","op":"+","fn":"add","args":[{"mathjs":"SymbolNode","name":"a"},{"mathjs":"SymbolNode","name":"b"}],"implicit":false},{"mathjs":"SymbolNode","name":"c"}],"implicit":false}', reviver)
  },
  {
    'ast': ['apply', 'factorial', 'a'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"!","fn":"factorial","args":[{"mathjs":"SymbolNode","name":"a"}],"implicit":false}', reviver)
  },
  {
    'ast': 'theta',
    'mathjs': JSON.parse('{"mathjs":"SymbolNode","name":"theta"}', reviver)
  },
  {
    'ast': ['*', 't', 'h', 'e', 't', 'a'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"SymbolNode","name":"t"},{"mathjs":"SymbolNode","name":"h"},{"mathjs":"SymbolNode","name":"e"},{"mathjs":"SymbolNode","name":"t"},{"mathjs":"SymbolNode","name":"a"}],"implicit":false}', reviver)
  },
  {
    'ast': ['apply', 'cos', 'theta'],
    'mathjs': JSON.parse('{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"cos"},"args":[{"mathjs":"SymbolNode","name":"theta"}]}', reviver)
  },
  {
    'ast': ['*', 'c', 'o', 's', 'x'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"SymbolNode","name":"c"},{"mathjs":"SymbolNode","name":"o"},{"mathjs":"SymbolNode","name":"s"},{"mathjs":"SymbolNode","name":"x"}],"implicit":false}', reviver)
  },
  {
    'ast': ['apply', 'abs', ['apply', 'sin', ['apply', 'abs', 'x']]],
    'mathjs': JSON.parse('{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"abs"},"args":[{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"sin"},"args":[{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"abs"},"args":[{"mathjs":"SymbolNode","name":"x"}]}]}]}', reviver)
  },
  {
    'ast': ['*', 'blah', 'x'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"SymbolNode","name":"blah"},{"mathjs":"SymbolNode","name":"x"}],"implicit":false}', reviver)
  },
  {
    'ast': ['apply', 'abs', ['=', ['+', 'x', 3], 2]],
    'mathjs': JSON.parse('{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"abs"},"args":[{"mathjs":"OperatorNode","op":"==","fn":"equal","args":[{"mathjs":"OperatorNode","op":"+","fn":"add","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"ConstantNode","value":3}],"implicit":false},{"mathjs":"ConstantNode","value":2}],"implicit":false}]}', reviver)
  },
  {
    'ast': ['_', 'x', ['_', 'y', 'z']],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['_', 'x', ['_', 'y', 'z']],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['_', ['_', 'x', 'y'], 'z'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['^', 'x', ['^', 'y', 'z']],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"^","fn":"pow","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"OperatorNode","op":"^","fn":"pow","args":[{"mathjs":"SymbolNode","name":"y"},{"mathjs":"SymbolNode","name":"z"}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['^', ['^', 'x', 'y'], 'z'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"^","fn":"pow","args":[{"mathjs":"OperatorNode","op":"^","fn":"pow","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false},{"mathjs":"SymbolNode","name":"z"}],"implicit":false}', reviver)
  },
  {
    'ast': ['^', 'x', ['_', 'y', 'z']],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['^', ['_', 'x', 'y'], 'z'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['*', 'x', 'y', ['apply', 'factorial', 'z']],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"},{"mathjs":"OperatorNode","op":"!","fn":"factorial","args":[{"mathjs":"SymbolNode","name":"z"}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': 'x',
    'mathjs': JSON.parse('{"mathjs":"SymbolNode","name":"x"}', reviver)
  },
  {
    'ast': 'f',
    'mathjs': JSON.parse('{"mathjs":"SymbolNode","name":"f"}', reviver)
  },
  {
    'ast': ['*', 'f', 'g'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"SymbolNode","name":"f"},{"mathjs":"SymbolNode","name":"g"}],"implicit":false}', reviver)
  },
  {
    'ast': ['+', 'f', 'g'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"+","fn":"add","args":[{"mathjs":"SymbolNode","name":"f"},{"mathjs":"SymbolNode","name":"g"}],"implicit":false}', reviver)
  },
  {
    'ast': ['apply', 'f', 'x'],
    'mathjs': JSON.parse('{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"f"},"args":[{"mathjs":"SymbolNode","name":"x"}]}', reviver)
  },
  {
    'ast': ['apply', 'f', ['tuple', 'x', 'y', 'z']],
    'mathjs': JSON.parse('{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"f"},"args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"},{"mathjs":"SymbolNode","name":"z"}]}', reviver)
  },
  {
    'ast': ['*', 'f', ['apply', 'g', 'x']],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"SymbolNode","name":"f"},{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"g"},"args":[{"mathjs":"SymbolNode","name":"x"}]}],"implicit":false}', reviver)
  },
  {
    'ast': ['*', 'f', 'p', 'x'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"SymbolNode","name":"f"},{"mathjs":"SymbolNode","name":"p"},{"mathjs":"SymbolNode","name":"x"}],"implicit":false}', reviver)
  },
  {
    'ast': ['*', 'f', 'x'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"SymbolNode","name":"f"},{"mathjs":"SymbolNode","name":"x"}],"implicit":false}', reviver)
  },
  {
    'ast': ['prime', 'f'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['*', 'f', ['prime', 'g']],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['*', ['prime', 'f'], 'g'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['*', ['prime', 'f'],
      ['prime', ['prime', 'g']]
    ],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['prime', 'x'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['apply', ['prime', 'f'], 'x'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['prime', ['apply', 'f', 'x']],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['prime', ['apply', 'sin', 'x']],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['apply', ['prime', 'sin'], 'x'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['apply', ['prime', ['prime', 'f']], 'x'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['prime', ['prime', ['apply', 'sin', 'x']]],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['^', ['apply', 'f', 'x'],
      ['_', 't', 'y']
    ],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['apply', ['_', 'f', 't'], 'x'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['_', ['apply', 'f', 'x'], 't'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['apply', ['^', 'f', 2], 'x'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['^', ['apply', 'f', 'x'], 2],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"^","fn":"pow","args":[{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"f"},"args":[{"mathjs":"SymbolNode","name":"x"}]},{"mathjs":"ConstantNode","value":2}],"implicit":false}', reviver)
  },
  {
    'ast': ['apply', ['^', ['prime', 'f'], 'a'], 'x'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['apply', ['^', 'f', ['prime', 'a']], 'x'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['apply', ['^', ['_', 'f', 'a'],
      ['prime', 'b']
    ], 'x'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['apply', ['^', ['prime', ['_', 'f', 'a']], 'b'], 'x'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['apply', 'sin', 'x'],
    'mathjs': JSON.parse('{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"sin"},"args":[{"mathjs":"SymbolNode","name":"x"}]}', reviver)
  },
  {
    'ast': ['*', ['apply', ['^', 'sin', 'x'], 'y'], 'z'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['*', ['apply', 'sin', 'x'], 'y'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"sin"},"args":[{"mathjs":"SymbolNode","name":"x"}]},{"mathjs":"SymbolNode","name":"y"}],"implicit":false}', reviver)
  },
  {
    'ast': ['apply', ['^', 'sin', 2], 'x'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['apply', 'exp', 'x'],
    'mathjs': JSON.parse('{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"exp"},"args":[{"mathjs":"SymbolNode","name":"x"}]}', reviver)
  },
  {
    'ast': ['^', 'e', 'x'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"^","fn":"pow","args":[{"mathjs":"SymbolNode","name":"e"},{"mathjs":"SymbolNode","name":"x"}],"implicit":false}', reviver)
  },
  {
    'ast': ['^', 'x', ['apply', 'factorial', 2]],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"^","fn":"pow","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"OperatorNode","op":"!","fn":"factorial","args":[{"mathjs":"ConstantNode","value":2}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['^', 'x', ['apply', 'factorial', ['apply', 'factorial', 2]]],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"^","fn":"pow","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"OperatorNode","op":"!","fn":"factorial","args":[{"mathjs":"OperatorNode","op":"!","fn":"factorial","args":[{"mathjs":"ConstantNode","value":2}],"implicit":false}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['^', ['_', 'x', 't'], 2],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['_', 'x', ['^', 'f', 2]],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['prime', ['_', 'x', 't']],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['_', 'x', ['prime', 'f']],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['tuple', 'x', 'y', 'z'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['+', ['tuple', 'x', 'y'],
      ['-', ['array', 'x', 'y']]
    ],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['*', 2, ['+', 'z', ['-', ['+', 'x', 1]]]],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"ConstantNode","value":2},{"mathjs":"OperatorNode","op":"+","fn":"add","args":[{"mathjs":"SymbolNode","name":"z"},{"mathjs":"OperatorNode","op":"-","fn":"unaryMinus","args":[{"mathjs":"OperatorNode","op":"+","fn":"add","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"ConstantNode","value":1}],"implicit":false}],"implicit":false}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['set', 1, 2, 'x'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['set', 'x', 'x'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['set', 'x'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['interval', ['tuple', 1, 2],
      ['tuple', false, true]
    ],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['array', 1, 2],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['tuple', 1, 2],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['list', 1, 2, 3],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['=', 'x', 'a'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"==","fn":"equal","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"a"}],"implicit":false}', reviver)
  },
  {
    'ast': ['=', 'x', 'y', 1],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"and","fn":"and","args":[{"mathjs":"OperatorNode","op":"==","fn":"equal","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false},{"mathjs":"OperatorNode","op":"==","fn":"equal","args":[{"mathjs":"SymbolNode","name":"y"},{"mathjs":"ConstantNode","value":1}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['=', 'x', ['=', 'y', 1]],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"==","fn":"equal","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"OperatorNode","op":"==","fn":"equal","args":[{"mathjs":"SymbolNode","name":"y"},{"mathjs":"ConstantNode","value":1}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['=', ['=', 'x', 'y'], 1],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"==","fn":"equal","args":[{"mathjs":"OperatorNode","op":"==","fn":"equal","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false},{"mathjs":"ConstantNode","value":1}],"implicit":false}', reviver)
  },
  {
    'ast': ['ne', 7, 2],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"!=","fn":"unequal","args":[{"mathjs":"ConstantNode","value":7},{"mathjs":"ConstantNode","value":2}],"implicit":false}', reviver)
  },
  {
    'ast': ['not', ['=', 'x', 'y']],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"not","fn":"not","args":[{"mathjs":"OperatorNode","op":"==","fn":"equal","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['not', ['=', 'x', 'y']],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"not","fn":"not","args":[{"mathjs":"OperatorNode","op":"==","fn":"equal","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['>', 'x', 'y'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":">","fn":"larger","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false}', reviver)
  },
  {
    'ast': ['ge', 'x', 'y'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":">=","fn":"largerEq","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false}', reviver)
  },
  {
    'ast': ['gts', ['tuple', 'x', 'y', 'z'],
      ['tuple', true, true]
    ],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"and","fn":"and","args":[{"mathjs":"OperatorNode","op":">","fn":"larger","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false},{"mathjs":"OperatorNode","op":">","fn":"larger","args":[{"mathjs":"SymbolNode","name":"y"},{"mathjs":"SymbolNode","name":"z"}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['gts', ['tuple', 'x', 'y', 'z'],
      ['tuple', true, false]
    ],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"and","fn":"and","args":[{"mathjs":"OperatorNode","op":">","fn":"larger","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false},{"mathjs":"OperatorNode","op":">=","fn":"largerEq","args":[{"mathjs":"SymbolNode","name":"y"},{"mathjs":"SymbolNode","name":"z"}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['gts', ['tuple', 'x', 'y', 'z'],
      ['tuple', false, true]
    ],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"and","fn":"and","args":[{"mathjs":"OperatorNode","op":">=","fn":"largerEq","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false},{"mathjs":"OperatorNode","op":">","fn":"larger","args":[{"mathjs":"SymbolNode","name":"y"},{"mathjs":"SymbolNode","name":"z"}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['gts', ['tuple', 'x', 'y', 'z'],
      ['tuple', false, false]
    ],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"and","fn":"and","args":[{"mathjs":"OperatorNode","op":">=","fn":"largerEq","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false},{"mathjs":"OperatorNode","op":">=","fn":"largerEq","args":[{"mathjs":"SymbolNode","name":"y"},{"mathjs":"SymbolNode","name":"z"}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['<', 'x', 'y'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"<","fn":"smaller","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false}', reviver)
  },
  {
    'ast': ['le', 'x', 'y'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"<=","fn":"smallerEq","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false}', reviver)
  },
  {
    'ast': ['lts', ['tuple', 'x', 'y', 'z'],
      ['tuple', true, true]
    ],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"and","fn":"and","args":[{"mathjs":"OperatorNode","op":"<","fn":"smaller","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false},{"mathjs":"OperatorNode","op":"<","fn":"smaller","args":[{"mathjs":"SymbolNode","name":"y"},{"mathjs":"SymbolNode","name":"z"}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['lts', ['tuple', 'x', 'y', 'z'],
      ['tuple', true, false]
    ],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"and","fn":"and","args":[{"mathjs":"OperatorNode","op":"<","fn":"smaller","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false},{"mathjs":"OperatorNode","op":"<=","fn":"smallerEq","args":[{"mathjs":"SymbolNode","name":"y"},{"mathjs":"SymbolNode","name":"z"}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['lts', ['tuple', 'x', 'y', 'z'],
      ['tuple', false, true]
    ],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"and","fn":"and","args":[{"mathjs":"OperatorNode","op":"<=","fn":"smallerEq","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false},{"mathjs":"OperatorNode","op":"<","fn":"smaller","args":[{"mathjs":"SymbolNode","name":"y"},{"mathjs":"SymbolNode","name":"z"}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['lts', ['tuple', 'x', 'y', 'z'],
      ['tuple', false, false]
    ],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"and","fn":"and","args":[{"mathjs":"OperatorNode","op":"<=","fn":"smallerEq","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false},{"mathjs":"OperatorNode","op":"<=","fn":"smallerEq","args":[{"mathjs":"SymbolNode","name":"y"},{"mathjs":"SymbolNode","name":"z"}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['>', ['<', 'x', 'y'], 'z'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":">","fn":"larger","args":[{"mathjs":"OperatorNode","op":"<","fn":"smaller","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false},{"mathjs":"SymbolNode","name":"z"}],"implicit":false}', reviver)
  },
  {
    'ast': ['subset', 'A', 'B'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['notsubset', 'A', 'B'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['superset', 'A', 'B'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['notsuperset', 'A', 'B'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['in', 'x', 'A'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['notin', 'x', 'A'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['ni', 'A', 'x'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['notni', 'A', 'x'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['union', 'A', 'B'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['intersect', 'A', 'B'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['and', 'A', 'B'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"and","fn":"and","args":[{"mathjs":"SymbolNode","name":"A"},{"mathjs":"SymbolNode","name":"B"}],"implicit":false}', reviver)
  },
  {
    'ast': ['and', 'A', 'B'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"and","fn":"and","args":[{"mathjs":"SymbolNode","name":"A"},{"mathjs":"SymbolNode","name":"B"}],"implicit":false}', reviver)
  },
  {
    'ast': ['or', 'A', 'B'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"or","fn":"or","args":[{"mathjs":"SymbolNode","name":"A"},{"mathjs":"SymbolNode","name":"B"}],"implicit":false}', reviver)
  },
  {
    'ast': ['and', 'A', 'B', 'C'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"and","fn":"and","args":[{"mathjs":"SymbolNode","name":"A"},{"mathjs":"SymbolNode","name":"B"},{"mathjs":"SymbolNode","name":"C"}],"implicit":false}', reviver)
  },
  {
    'ast': ['or', 'A', 'B', 'C'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"or","fn":"or","args":[{"mathjs":"SymbolNode","name":"A"},{"mathjs":"SymbolNode","name":"B"},{"mathjs":"SymbolNode","name":"C"}],"implicit":false}', reviver)
  },
  {
    'ast': ['or', ['and', 'A', 'B'], 'C'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"or","fn":"or","args":[{"mathjs":"OperatorNode","op":"and","fn":"and","args":[{"mathjs":"SymbolNode","name":"A"},{"mathjs":"SymbolNode","name":"B"}],"implicit":false},{"mathjs":"SymbolNode","name":"C"}],"implicit":false}', reviver)
  },
  {
    'ast': ['or', 'A', ['and', 'B', 'C']],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"or","fn":"or","args":[{"mathjs":"SymbolNode","name":"A"},{"mathjs":"OperatorNode","op":"and","fn":"and","args":[{"mathjs":"SymbolNode","name":"B"},{"mathjs":"SymbolNode","name":"C"}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['not', ['=', 'x', 1]],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"not","fn":"not","args":[{"mathjs":"OperatorNode","op":"==","fn":"equal","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"ConstantNode","value":1}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['not', ['=', 'x', 1]],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"not","fn":"not","args":[{"mathjs":"OperatorNode","op":"==","fn":"equal","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"ConstantNode","value":1}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['or', ['not', ['=', 'x', 'y']],
      ['ne', 'z', 'w']
    ],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"or","fn":"or","args":[{"mathjs":"OperatorNode","op":"not","fn":"not","args":[{"mathjs":"OperatorNode","op":"==","fn":"equal","args":[{"mathjs":"SymbolNode","name":"x"},{"mathjs":"SymbolNode","name":"y"}],"implicit":false}],"implicit":false},{"mathjs":"OperatorNode","op":"!=","fn":"unequal","args":[{"mathjs":"SymbolNode","name":"z"},{"mathjs":"SymbolNode","name":"w"}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': ['+', ['*', 1.2, 'e'],
      ['-', 3]
    ],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"+","fn":"add","args":[{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"ConstantNode","value":1.2},{"mathjs":"SymbolNode","name":"e"}],"implicit":false},{"mathjs":"OperatorNode","op":"-","fn":"unaryMinus","args":[{"mathjs":"ConstantNode","value":3}],"implicit":false}],"implicit":false}', reviver)
  },
  {
    'ast': 'infinity',
    'mathjs': JSON.parse('{"mathjs":"SymbolNode","name":"Infinity"}', reviver)
  },
  {
    'ast': ['*', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j'],
    'mathjs': JSON.parse('{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"SymbolNode","name":"a"},{"mathjs":"SymbolNode","name":"b"},{"mathjs":"SymbolNode","name":"c"},{"mathjs":"SymbolNode","name":"d"},{"mathjs":"SymbolNode","name":"e"},{"mathjs":"SymbolNode","name":"f"},{"mathjs":"SymbolNode","name":"g"},{"mathjs":"SymbolNode","name":"h"},{"mathjs":"SymbolNode","name":"i"},{"mathjs":"SymbolNode","name":"j"}],"implicit":false}', reviver)
  },
  {
    'ast': 2,
    'mathjs': JSON.parse('{"mathjs":"ConstantNode","value":2}', reviver)
  },
  {
    'ast': '',
    'mathjs':  JSON.parse('{"mathjs":"SymbolNode","name":""}', reviver)
  },
  {
    'ast':  ['matrix', ['tuple', 2, 2], ['tuple', ['tuple', 'a', 'b'], ['tuple', 'c', 'd']]],
    'mathjs': JSON.parse('{"mathjs":"ArrayNode","items":[{"mathjs":"ArrayNode","items":[{"mathjs":"SymbolNode","name":"a"},{"mathjs":"SymbolNode","name":"b"}]},{"mathjs":"ArrayNode","items":[{"mathjs":"SymbolNode","name":"c"},{"mathjs":"SymbolNode","name":"d"}]}]}', reviver)
  },
  {
    'ast': ['matrix', ['tuple', 1, 2], ['tuple', ['tuple', ['+', 'a', ['*', 3, 'y']], ['*', 2, ['apply', 'sin', 'theta']]]]],
    'mathjs': JSON.parse('{"mathjs":"ArrayNode","items":[{"mathjs":"ArrayNode","items":[{"mathjs":"OperatorNode","op":"+","fn":"add","args":[{"mathjs":"SymbolNode","name":"a"},{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"ConstantNode","value":3},{"mathjs":"SymbolNode","name":"y"}],"implicit":false}],"implicit":false},{"mathjs":"OperatorNode","op":"*","fn":"multiply","args":[{"mathjs":"ConstantNode","value":2},{"mathjs":"FunctionNode","fn":{"mathjs":"SymbolNode","name":"sin"},"args":[{"mathjs":"SymbolNode","name":"theta"}]}],"implicit":false}]}]}', reviver)
  },
  {
    'ast': ['matrix', ['tuple', 2, 3], ['tuple', ['tuple', 8, 0, 0], ['tuple', 1, 2, 3]]],
    'mathjs': JSON.parse('{"mathjs":"ArrayNode","items":[{"mathjs":"ArrayNode","items":[{"mathjs":"ConstantNode","value":8},{"mathjs":"ConstantNode","value":0},{"mathjs":"ConstantNode","value":0}]},{"mathjs":"ArrayNode","items":[{"mathjs":"ConstantNode","value":1},{"mathjs":"ConstantNode","value":2},{"mathjs":"ConstantNode","value":3}]}]}', reviver)
  },
  {
    'ast': ['derivative_leibniz', 'x', 't'],
    'mathjs': {'implemented': false }
  },
  {
    'ast': ['derivative_leibniz_mult', 2, 'x', 't'],
    'mathjs': {'implemented': false }
  },

]


for (let objectToTest of objectsToTest) {
  test("parses " + objectToTest.ast + ' to ' + objectToTest.mathjs, () => {
    if(objectToTest.mathjs.implemented === false) {
      expect(() => converter.convert(objectToTest.ast)).toThrow("not implemented");
    }
    else {
      expect(converter.convert(objectToTest.ast)).toEqual(objectToTest.mathjs);
    }
  });

}
