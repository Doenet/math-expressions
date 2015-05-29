# math-expressions

Parse expressions like `sin^2 (x^3)` and do some basic computer
algebra with them, like symbolic differentiation and numerically
identifying equivalent expressions.

## example

```JavaScript
var Expression = require('math-expressions');

var f = Expression.fromText("sin^2 (x^3)");

console.log(f.toLatex());
```

## API

Coming soon!
