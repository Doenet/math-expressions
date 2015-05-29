# math-expressions

Parse expressions like `sin^2 (x^3)` and do some basic computer
algebra with them, like symbolic differentiation and numerically
identifying equivalent expressions.

## example

```JavaScript
var Expression = require('math-expressions');

var f = Expression.fromText("sin^2 (x^3)");

console.log(f.toLatex());

var g = Expression.fromText("sin^2 x + cos^2 x");
var h = Expression.fromText("1");

console.log( g.equals(h) );

var g = Expression.fromText("x + x^2");
var h = Expression.fromText("x + x^3");

console.log( g.equals(h) );
```

## API

Coming soon!
