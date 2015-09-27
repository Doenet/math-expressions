# math-expressions

Parse expressions like `sin^2 (x^3)` and do some basic computer
algebra with them, like symbolic differentiation and numerically
identifying equivalent expressions.

# Client-side use

There is a [demo available](https://rawgit.com/kisonecat/math-expressions/master/demo/index.html) which focuses on the equality testing.

## Installation

The built library is stored in `build/math-expressions.js`.  This is
packaged in a "unified" module format, so it can be treated as an
AMD-style or a CommonJS-style module (and imported via a module loader
like RequireJS).  If you are not using a module loader, you can import
it via

```HTML
<script type="text/javascript" src="math-expressions.js"></script>`
```

and it will add `MathExpression` to the global namespace.

## Example

`var f = MathExpression.fromText("sin^2 (x^3)");`

# Server-side use

## Installation

INSTRUCTIONS FOR npm install

## Example

```JavaScript
var MathExpression = require('math-expressions');

var f = MathExpression.fromText("sin^2 (x^3)");

console.log(f.tex());

var g = MathExpression.fromText("sin^2 x + cos^2 x");
var h = MathExpression.fromText("1");

console.log( g.equals(h) );

var g = MathExpression.fromText("x + x^2");
var h = MathExpression.fromText("x + x^3");

console.log( g.equals(h) );
```

# API

Coming soon!

# Contributing
