npm install but also expose a single monolithc MathExpressions library

get rid of undersdcore
 
"build" library

learner skin -- animations -- prettier, people like it

target date for oppia

steve in two weeks

let steve lewis know about oppia team

# math-expressions

INSTRUCTIONS FOR npm install

Parse expressions like `sin^2 (x^3)` and do some basic computer
algebra with them, like symbolic differentiation and numerically
identifying equivalent expressions.

## example

```JavaScript
var Expression = require('math-expressions');

var f = Expression.fromText("sin^2 (x^3)");

console.log(f.tex());

var g = Expression.fromText("sin^2 x + cos^2 x");
var h = Expression.fromText("1");

console.log( g.equals(h) );

var g = Expression.fromText("x + x^2");
var h = Expression.fromText("x + x^3");

console.log( g.equals(h) );
```

## API

Coming soon!

## Contributing

tolerance?

restrict certain syntax elements?

some sample explorations

design discussions

unambiguous parsing

root

dual license APACHE
