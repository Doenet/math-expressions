# math-expressions Rust Port — Detailed Plan

## Goals and constraints

- **Target**: `wasm32-unknown-unknown`, exposed to JavaScript via `wasm-bindgen`
- **Use case**: Educational mathematics — expressions have tens of terms, not thousands
- **Philosophy**: Test-driven, port JS behavior faithfully before optimising, but do not neccessarily copy JS data structures
- **Algorithm source**: Borrow design patterns from SymEngine (sorted-args canonical form,
  immutable nodes, type-per-operator dispatch); borrow polynomial algorithms from SymPy's DMP layer
- **Sequence**: Parser → normalisation → polynomial algorithms → evaluation → equality → WASM bindings

---

## Hyperreal evaluation

**Verdict: do not use hyperreal as the number backend.**

Hyperreal's `Rational` is `Arc<RationalData>` where `RationalData` holds `BigUint` fields.
Every number, including `0` and `1`, allocates on the heap (even the pre-cached constants are
`Arc` clones). The `Real` struct contains `AtomicPrimitiveApproxCache`, which uses atomic
operations that are wasted overhead in single-threaded WASM.

Hyperreal is designed for certified geometric computation with lazy computable reals — a
different problem from CAS arithmetic. It has no polynomial type, no domain tower, and no
monomial ordering.

**Use instead**: a custom tiered `Number` enum with stack-allocated fast paths for common
cases (see §3). Depend on `num-bigint` + `num-rational` directly for the BigInt fallback,
identical to what both mathhook and SymPy use.

---

## 1. Crate layout

```
math-expressions-rs/
├── Cargo.toml
├── src/
│   ├── lib.rs              # wasm-bindgen exports; thin re-export layer
│   │
│   ├── num/                # §3 Number types
│   │   ├── mod.rs
│   │   ├── number.rs       # Number enum
│   │   └── gcd.rs          # i64 and BigInt GCD
│   │
│   ├── sym/                # §4 Symbol interning
│   │   └── mod.rs
│   │
│   ├── expr/               # §5 Expression tree
│   │   ├── mod.rs          # Expr enum, Canonical newtype
│   │   ├── build.rs        # constructors, smart constructors (Add, Mul normalise on build)
│   │   ├── visit.rs        # children()/map_children() — uniform traversal
│   │   ├── display.rs      # Debug / text rendering
│   │   └── hash.rs         # structural hash
│   │
│   ├── parse/              # §6 Parsers
│   │   ├── mod.rs
│   │   ├── error.rs        # ParseError { message, span } — spans threaded from the lexer
│   │   ├── lexer.rs        # tokeniser (shared by text + latex parsers)
│   │   ├── text.rs         # recursive descent — port of text-to-ast.js
│   │   └── latex.rs        # recursive descent — port of latex-to-ast.js
│   │
│   ├── norm/               # §7 Normalisation / tree rewriting
│   │   ├── mod.rs
│   │   ├── flatten.rs      # n-ary flattening for +, *
│   │   ├── order.rs        # canonical argument ordering (SymEngine-style)
│   │   ├── names.rs        # function-name / applied-function / negative-number normalisation
│   │   ├── arith.rs        # constant folding, zero/one elimination
│   │   └── simplify.rs     # higher-level rewrites (collect like terms, power rules, etc.)
│   │
│   ├── poly/               # §8 Polynomial layer
│   │   ├── mod.rs
│   │   ├── univariate.rs   # DUP: Vec<Number>, ascending degree
│   │   ├── multivariate.rs # DMP: recursive dense, SymPy model (see §8c)
│   │   ├── domain.rs       # Domain enum: Z, Q, F(p)
│   │   ├── gcd.rs          # pseudo-remainder GCD, subresultant
│   │   ├── factor.rs       # square-free, Kronecker (educational scope)
│   │   └── convert.rs      # Expr → poly and back
│   │
│   ├── eval/               # §9 Evaluation
│   │   ├── mod.rs
│   │   ├── numerical.rs    # f64 evaluation with variable bindings
│   │   ├── complex.rs      # complex-plane evaluation
│   │   └── finite_field.rs # Z/pZ evaluation
│   │
│   ├── eq/                 # §10 Equality testing
│   │   ├── mod.rs
│   │   ├── syntax.rs       # structural equality after the full normalisation suite
│   │   ├── finite_field.rs # finite-field rejection check
│   │   ├── complex.rs      # random complex-point sampling (the acceptance workhorse)
│   │   └── discrete_infinite.rs # discrete infinite set comparison (stubbed — see §17)
│   │
│   ├── assumptions/        # §11 Assumptions
│   │   └── mod.rs
│   │
│   └── output/             # §12 Output formats
│       ├── mod.rs
│       ├── text.rs
│       ├── latex.rs
│       └── js_tree.rs      # ↔ JS Tree JSON shape; all JS ad-hoc encodings live here
│
└── tests/                  # §13 TDD test files
    ├── fixtures/           # JSON fixtures auto-extracted from the JS spec data maps (§6d)
    ├── num.rs
    ├── sym.rs
    ├── text_parse.rs       # driven by fixtures from quick_text-to-ast.spec.js (~540 cases)
    ├── latex_parse.rs      # driven by fixtures from quick_latex-to-ast.spec.js (~630 cases)
    ├── norm.rs
    ├── poly.rs
    ├── eval.rs
    └── equality.rs
```

---

## 2. WASM / JS boundary strategy

The hot path is: JS sends a string, Rust parses it, evaluates or compares it, returns a
result. There is no need for JS to inspect the internal AST structure.

**Boundary contract** — a single `#[wasm_bindgen]` struct that owns its tree:
```rust
#[wasm_bindgen]
pub struct Expression(Expr);

parse_text(s: &str)  -> Result<Expression, ParseError>
parse_latex(s: &str) -> Result<Expression, ParseError>

impl Expression {
    to_text(&self)   -> String
    to_latex(&self)  -> String
    equals(&self, other: &Expression, opts: JsValue) -> bool
    evaluate(&self, bindings: JsValue) -> Result<f64, EvalError>
    simplify(&self)  -> Expression
    derivative(&self, var: &str) -> Result<Expression, JsValue>
}
```

Returning a `#[wasm_bindgen]` struct by value already gives us the opaque-handle pattern
for free: JS receives a generated class holding a pointer into WASM memory, with a
`.free()` method and automatic `FinalizationRegistry` cleanup when the JS object is
garbage-collected. No hand-rolled arena, free list, or leak-on-forget. Only primitives
and strings cross the boundary; there is no serde overhead converting full trees to JS
objects on every call.

**Important**: all read-only operations must take `&self` / `&Expression`. A wasm-bindgen
struct parameter taken *by value* is consumed — the JS object becomes unusable after one
call. Methods returning new expressions (`simplify`, `derivative`) return fresh owned
`Expression` values.

If the tree must be inspectable from JS (e.g. for rendering), add:
```rust
to_json(&self)     -> String       // serialise to JSON on demand
from_json(s: &str) -> Expression
```

---

## 3. Number type (`src/num/`)

**Design**: three-tier enum with stack-allocated fast paths.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Number {
    /// Common case: integers that fit in i64.  No allocation.
    Int(i64),
    /// Common case: reduced fractions with small numerator/denominator.  No allocation.
    /// Invariant: den > 0, gcd(|num|, den) == 1, den != 1 (den==1 → Int).
    Rat(i64, i64),
    /// Arbitrary precision fallback.  Boxed to keep Number small.
    Big(Box<BigNumber>),
    /// Floating-point value.  Produced by numerical evaluation only — user
    /// input never parses to Float (see "Exact decimal literals" below).
    Float(F64),
}

/// f64 wrapper providing Eq + Hash by bit pattern (f64 itself implements neither,
/// so `Float(f64)` directly would make the derives above fail to compile).
/// Policy: NaN == NaN, +0.0 != -0.0 (bit comparison) — fine for canonical ordering
/// and hashing; numeric comparisons in equality testing go through tolerances anyway.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct F64(u64);   // from/to f64 via to_bits()/from_bits()

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BigNumber {
    Int(BigInt),
    Rat(BigRational),
}
```

**Size**: `Number` fits in 24 bytes (discriminant + two i64s). `Box<BigNumber>` is 8 bytes on
the stack regardless of BigInt size.

**Arithmetic rules** (SymEngine-style — always reduce to canonical form):
- `Int(a) + Int(b)`: try `i64::checked_add`, promote to `Big` on overflow
- `Int(a) * Int(b)`: try `i64::checked_mul`, promote to `Big` on overflow
- `Int(a) / Int(b)`: GCD-reduce; if `den == 1` return `Int`, else `Rat`
- `Rat + Rat`: cross-multiply with checked arithmetic, fallback to `Big`
- Any `Big` operand: convert other to `BigRational`, compute, try to demote back

**Fast paths** (no-alloc cases cover >99% of educational math):
- All small integer arithmetic stays in `Int`
- `1/2 + 1/3 = 5/6` stays in `Rat`
- `2^10 = 1024` stays in `Int`

**Dependencies**: `num-bigint`, `num-rational`, `num-traits` (same versions mathhook uses).
No hyperreal.

**TDD milestone**: `tests/num.rs` — 50+ unit tests covering all arithmetic paths, overflow
promotion, GCD reduction, Display formatting.

### 3a. Exact decimal literals (decision 2026-07-16, deliberate JS divergence)

**All user-typed decimals parse as exact rationals, never floats.** `0.5` →
`Rat(1, 2)`, `0.1` → `Rat(1, 10)`, `1.5E-3` → `Rat(3, 2000)`; overlong
mantissas promote to `Big(BigRational)`. Uniformly — not only when the f64
would be inexact — so the tree shape never depends on binary representability
and `Float` gets a crisp meaning: *result of numerical evaluation only*.
Consequences and mechanics:

- `Number::from_decimal_str(text)` replaces `parse_js_float` in both parsers'
  NUMBER branches (needs gcd/reduction — lands with the `Number` arithmetic).
- Normalisation folds constants exactly; `0.1 + 0.2 = 0.3` becomes
  structurally true, shrinking reliance on tolerance-based equality.
- Formatters render any rational whose denominator is `2^a·5^b` in decimal
  form (`Rat(1,2)` → `0.5`, not `1/2`), so decimal input round-trips exactly.
  Other denominators render as `a/b` / `\frac{a}{b}`.
- JS boundary: `to_js` projects these rationals to the nearest f64 (what the
  JS trees actually hold), keeping the tree fixtures and the future
  differential harness meaningful. `Number::js_string` (sign-string quirk
  emulation) also goes through the f64 projection to stay JS-faithful
  (input `0.10` must yield the sign-string `0.1+`, not `0.10+`).

---

## 4. Symbol interning (`src/sym/`)

```rust
/// An interned symbol reference.  Copy type — 4 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sym(u32);

/// Thread-local string interner.
/// lookup(s) → Sym;  resolve(sym) → &str
```

Use a `HashMap<String, u32>` + `Vec<String>` interner in a `thread_local!`.  Symbol
comparison is then a `u32` comparison — critical for fast canonical ordering.

**TDD milestone**: `tests/sym.rs` — round-trip tests, deduplication, ordering stability.

---

## 5. Expression tree (`src/expr/`)

**Design follows SymEngine** for dispatch (the type of the node IS the operator), but
**one enum serves two layers with different invariants**:

1. **Faithful layer** (parser output): flat n-ary `Add`/`Mul` (the JS parser calls
   `flatten` before returning), but **unsorted and unfolded** — the JS parser tests
   assert as-parsed trees (`"1+x+3"` → `["+", 1, "x", 3]`: order preserved, `1+3` not
   combined). The parser builds with plain constructors only. This layer round-trips
   to text/LaTeX faithfully.
2. **Canonical layer** (produced on demand by `normalize()`, §7): additionally sorted,
   constant-folded, zero/one-eliminated, `Neg` rewritten to `Mul(-1, x)`, function
   names normalised. Structural equality is meaningful only here — without eliminating
   `Neg`, `x - y` and `-y + x` would never compare equal.

The JS library works the same way (`expr.tree` is as-parsed; normalisation happens
inside the equality path) but enforces the distinction by convention only. Here the
canonical layer gets a newtype so the compiler enforces it:

```rust
/// Proof-carrying wrapper: the inner tree has passed normalize().
/// Only norm::normalize() constructs it; eq::equals() and the polynomial
/// converters take &Canonical, so an unnormalised tree can never reach a
/// structural comparison by accident.
pub struct Canonical(Expr);
```

No enum duplication, zero runtime cost, and misuse becomes a type error.

The JS tree encodes several things ad hoc (chained inequalities as parallel
bool-tuples, interval closure as boolean leaves, blanks as a magic `＿` symbol,
five unrelated sequence heads). The Rust representation fixes these; a single
converter module (`output/js_tree.rs`) maps to/from the JS tree shape for fixtures
and interop, localising all the ad-hoc-ness in one place.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    // Atomic leaves
    Num(Number),
    Sym(Sym),
    Const(MathConst),       // Pi, E, I, Inf, NegInf, NaN
    Blank,                  // missing operand "＿" — a real variant, not a magic symbol
    Ldots,                  // "..." inside lists: "1,2,3,..." — ["ldots"] in the JS AST

    // Algebraic core (n-ary ops always flat; sorted only in canonical layer)
    Add(Vec<Expr>),
    Mul(Vec<Expr>),
    Pow(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),         // faithful layer only; canonicalised to Mul(-1, x)

    // Boolean / set algebra (n-ary, flattened, same invariants as Add/Mul)
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Not(Box<Expr>),
    Union(Vec<Expr>),
    Intersect(Vec<Expr>),

    // Function application.  The head is a full expression, NOT just a name:
    //   f'(x)     → Apply(Prime(f), [x])
    //   sin^2(x)  → Apply(Pow(sin, 2), [x])
    //   f_a^(b')(x) → Apply(Pow(Index(f, a), Prime(b)), [x])
    Apply(Box<Expr>, Vec<Expr>),

    // Notation nodes (from the parsers)
    Prime(Box<Expr>),               // f'  — ["prime", f] in the JS AST
    Index(Box<Expr>, Box<Expr>),    // x_i — ["_", x, i] in the JS AST

    // Sequences: one variant + kind, instead of five unrelated JS heads
    // ("tuple", "array", "list", "set", "altvector").  The coerce_tuples_arrays /
    // coerce_vectors equality options become a kind-equivalence relation instead
    // of cross-variant special cases.
    Seq(SeqKind, Vec<Expr>),

    // JS: ["interval", ["tuple", a, b], ["tuple", closed_l, closed_r]] with boolean
    // leaves.  Closure is metadata, not subexpressions — store it natively.
    Interval { endpoints: Box<(Expr, Expr)>, closed: (bool, bool) },

    // Relations, chained: "x < y <= z" → operands [x, y, z], ops [Lt, Le].
    // Invariant: operands.len() == ops.len() + 1.  Binary "a = b" is the 2-operand
    // case — one shape for both, unlike the JS ("=" vs "lts"/"gts" + bool-tuples).
    Relation { operands: Vec<Expr>, ops: Vec<RelOp> },

    // Row-major flat storage; invariant: entries.len() == rows * cols.
    // (JS uses nested tuples, which permits ragged rows and double indirection.)
    Matrix { rows: u32, cols: u32, entries: Vec<Expr> },

    // Calculus (first-class, like SymEngine's Derivative).
    // vars supports mixed partials: ∂³f/∂x²∂y → vars = [(x, 2), (y, 1)]
    Deriv { expr: Box<Expr>, vars: Vec<(Sym, u32)> },
    Integral { integrand: Box<Expr>, var: Sym, bounds: Option<Box<(Expr, Expr)>> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeqKind { Tuple, Array, List, Set, Vector, AltVector }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MathConst { Pi, E, I, Inf, NegInf, NaN }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelOp { Eq, Ne, Lt, Le, Gt, Ge, In, NotIn, Subset, Superset }
```

**Notes on representation**:
- `Add`/`Mul`/`And`/`Or`/`Union`/`Intersect` are always flat (no `Add` inside `Add`) —
  in both layers
- Sorted args, folded constants, and eliminated zeros/ones are canonical-layer
  invariants only
- There is no `Bool` leaf: the JS only has boolean leaves because it encodes interval
  closure and inequality strictness as tuple entries; both are native fields here
- `Blank` is a dedicated variant: the parser inserts it for missing operands in many
  places (`"x^^"` → `Pow(Pow(x, Blank), Blank)`; `"y^sin(x)"` →
  `Mul(Pow(y, Apply(sin, [Blank])), x)`), and the `allow_blanks` equality option
  becomes a variant check instead of a magic-string comparison
- `Relation` costs two small `Vec`s even for binary `a = b`; at educational scale this
  is irrelevant, and one shape means one code path for normalisation and printing
- `Apply` boxes its head even when it is a bare symbol (`sin(x)`) — an inline-symbol
  fast path is a possible later micro-optimisation, not worth the match complexity now
- A uniform traversal API in `expr/visit.rs` (`children()`, `map_children(f)`) lets
  recursive passes (flatten, substitute, variable collection) be written once instead
  of per-variant in every pass

**Plain constructors** build faithful trees (parser uses these exclusively).
**Smart constructors** (in `build.rs`) enforce canonical invariants and are used by
`normalize()`, `simplify()`, and the polynomial/equality layers:
```rust
pub fn add(args: Vec<Expr>) -> Expr  // flattens, folds constants, removes zeros, sorts
pub fn mul(args: Vec<Expr>) -> Expr  // flattens, folds constants, removes ones, sorts
pub fn pow(base: Expr, exp: Expr) -> Expr  // handles 0^0, x^0 = 1, x^1 = x, etc.
pub fn neg(e: Expr) -> Expr          // rewrites to Mul(-1, e), folding into Num when possible
```

The parser must NOT route through the smart constructors — doing so would break
essentially all ~1,170 ported parser fixture tests (they assert unsorted, unfolded
trees).

**JS tree interop** (`output/js_tree.rs`): a bidirectional converter between `Expr`
and the JS `Tree` JSON shape (`["+", 1, "x", 3]`, `["apply", "sin", "x"]`, ...).
All the JS ad-hoc encodings live only here:
```
Seq(Tuple, ...)              ↔ ["tuple", ...]          (likewise array/list/set/altvector)
Relation{[x,y,z],[Lt,Le]}    ↔ ["lts", ["tuple",x,y,z], ["tuple",true,false]]
Relation{[a,b],[Eq]}         ↔ ["=", a, b]
Interval{(a,b),(false,true)} ↔ ["interval",["tuple",a,b],["tuple",false,true]]
Blank                        ↔ "＿"        Ldots ↔ ["ldots"]
Matrix{2,2,[a,b,c,d]}        ↔ JS nested-tuple matrix form
```
Used by the fixture tests (§6d compare via this mapping), `to_json`/`from_json`, and
any JS caller that needs to inspect trees. Everything else in the crate sees only the
clean `Expr`.

### 5b. Stack safety of `Expr` (planned — STACK_SAFETY_PLAN.md, not yet implemented)

wasm32 has a fixed 1 MB shadow stack; overflow is a trap that kills the instance.
Strategy: **bounded boundary + depth-oblivious core** — a nesting-depth cap at the
producers (§6e) plus iterative traversals internally. On the `Expr` type itself:

- **Iterative `Drop`** — deferred, and NOT "~25 lines": implementing `Drop for Expr`
  triggers **60 E0509 errors** (Rust forbids moving fields out of a `Drop` type, and
  the codebase move-matches `Expr` pervasively — `match e { Expr::Add(xs) => …xs… }`),
  so it is a ~60-site `mem::replace` refactor. And it is no longer urgent: the §6e
  cap bounds *every* producer's tree depth, so recursive `Drop` is bounded too
  (canonicalize deepens by ≤ a small constant). Revisit only if programmatic
  deep-tree construction is exposed, or fold into the flat-arena migration (§16),
  where `Drop` becomes one `Vec` free.
- **Derived `Clone`/`PartialEq`/`Hash`/`Debug` are recursive** — stage-1 equality is
  derived `PartialEq`, and `eq::opaque_key` currently uses derived `Debug`. Under the
  §6e cap these are bounded (internal passes deepen trees by ≤ a small constant, e.g.
  `Div → Mul[…, Pow(b,−1)]` adds one level); replace `opaque_key`'s Debug-key
  regardless, and decide derived-`PartialEq` from frame-size measurements.
- **Shared iterative `fold` driver** with a `children()` helper and a pre-order
  `Prune` hook: every post-parse pass (`flatten`, `canonicalize`, `cmp` (paired-stack
  variant), `eval_complex`/`free_symbols`/`contains_blank`/`coerce_seqs`,
  `to_js`/`from_js`, both formatters, `convert_units_in_term`) is a genuine
  post-order fold — the smart constructors already inspect only already-folded
  children, so they plug into the `exit` callback unchanged. Write the traversal
  once; port one pass per commit under the existing fixture/round-trip/equality-corpus
  oracle.
- **Verification without a wasm harness**: run each pass in a
  `std::thread::Builder::stack_size(128 * 1024)` thread against a cap-depth tree and
  a 10⁵-node *wide* tree.
- Rejected: `stacker` (no wasm32 support — silently doesn't grow); flat-arena
  representation now (see §16 — natural post-Phase-7 optimisation, the `fold` driver
  survives that migration).

---

## 6. Parsers (`src/parse/`)

### 6a. Shared lexer (`lexer.rs`)

The JS lexer (`lib/converters/lexer.js`) is a first-match-wins ordered regex table;
each parser supplies its own rule table. Several rules use lookaheads
(`(?![a-zA-Z0-9])` keyword boundaries, the scientific-notation
followed-by-delimiter constraint) that Rust's `regex` crate does not support, so
the Rust lexer is hand-coded with the rules in the same order as the JS table
(ordering is semantic: `**` before `*`, `!=` and `!` are separate rules, ...).

**Token types are strings during the port, an enum after.** Both JS parsers compare
`token_type` strings in ~150 places each, and at least one comparison exploits
string structure (latex-to-ast.js line 637: `token_type[0] !== "|"` groups `"|"`
and `"|L"`). Porting with the JS strings verbatim makes every comparison a
mechanical transcription — eliminating re-encoding drift exactly when there is no
passing test suite to catch it. Once both parsers' fixture suites are green, a
single enum-ification pass converts token types with the fixtures as referee:

1. Port text parser with string tokens → fixtures green ✓ (done)
2. Port LaTeX parser with string tokens → fixtures green
3. Convert token strings to enum(s) across both parsers in one pass; decide then
   whether the parsers share one `TokenType` or keep parser-local enums (their
   alphabets overlap on NUMBER/VAR/`^`/`_` but each has private types — likely
   two enums). String-structure tricks like `"|"`/`"|L"` become explicit helpers
   (`is_pipe()`).

LaTeX lexer extensions needed beyond the text rules: literal patterns with embedded
`\s*` (`\left\s*(`), a letters-only keyword boundary `(?![a-zA-Z])` (the text
parser's is `(?![a-zA-Z0-9])`), and a custom whitespace rule.

### 6b. Text parser (`text.rs`)

Direct port of the recursive descent grammar in `text-to-ast.js`. Grammar (abbreviated):

```
statement_list   → statement (',' statement)*
statement        → statement_a ('|' statement_a)?
statement_a      → statement_b ('OR' statement_b)*
statement_b      → relation ('AND' relation)*
relation         → ('NOT')? expression (cmp_op expression)*
expression       → ('+' | '-')? term (('+' | '-' | 'UNION' | 'INTERSECT') term)*
term             → factor (('*' | '/') factor | implicit_factor)*
baseFactor       → '(' statement_list ')'
               | '[' statement_list ']'
               | '{' statement_list '}'
               | '|' statement '|'
               | number
               | variable
               | applied_fn '(' statement_list ')'
               | applied_fn factor         -- implicit application: sin x
               | baseFactor '_' baseFactor
               | baseFactor '!'
factor           → baseFactor ('^' baseFactor)*     -- iterative: ^ is LEFT-associative
```

Key behaviours to preserve (each needs a test; all verified against
`quick_text-to-ast.spec.js`):
- `xy` → `Mul([x, y])` (split symbols, `splitSymbols` option)
- `2x` → `Mul([2, x])` (implicit multiplication after number)
- `sin x` → `Apply(sin, [x])` (applied function without parens)
- `sin^2 x` → `Pow(Apply(sin, [x]), 2)` NOT `Apply(sin, [Pow(x, 2)])`
- `x^y^z` → `Pow(Pow(x, y), z)` — `^` is LEFT-associative (spec:
  `"x^y^z": ["^", ["^", "x", "y"], "z"]`); do not write the usual right-recursive rule
- `|x+1|` → `Apply(abs, [x+1])`
- `dx/dy` → Leibniz derivative form
- `f'` → `Prime(f)`; `f'(x)` → `Apply(Prime(f), [x])` — apply heads are expressions
- Blank insertion for missing operands: `x^^` → `Pow(Pow(x, Blank), Blank)`;
  `y^sin(x)` → `Mul(Pow(y, Apply(sin, [Blank])), x)` (sin grabs a blank, not the parens!)
- Chained inequalities parse to a single `Relation`: `x < y <= z` →
  `Relation { operands: [x, y, z], ops: [Lt, Le] }` (JS: `["lts", tuple, bool-tuple]`)
- Scientific notation folds at parse time: `3.1E-3` → `Num(Rat(31, 10000))`\
  (exact decimal literals per §3a; was `Float(0.0031)` before that decision)

### 6c. LaTeX parser (`latex.rs`)

Recursive descent over LaTeX tokens. Key constructs:

```
\frac{num}{den}        → Mul([num, Pow(den, -1)])
\sqrt{x}               → Apply(sqrt, [x])
\sqrt[n]{x}            → Apply(nthroot, [x, n])
x^{...}                → Pow(x, ...)
x_{...}                → subscript
\left( ... \right)     → grouping
\begin{matrix}...\end{matrix}  → Matrix
\sin, \cos, \log, ...  → named functions
\cdot, \times          → Mul
\frac{d^n f}{dx^n}     → Deriv
```

Single-digit special case: `x^2` in LaTeX (no braces) — superscript applies to exactly
one character/digit.

### 6d. TDD milestones for parsers

The spec files are plain data maps (`"input": expected_tree`) — ~540 cases in
`quick_text-to-ast.spec.js` and ~630 in `quick_latex-to-ast.spec.js`. **Do not
hand-port them.** Write a one-time Node script that imports each spec's data object
and dumps it to JSON fixtures under `tests/fixtures/`; the Rust tests iterate the
fixtures:

```rust
// tests/text_parse.rs
for (input, expected) in load_fixtures("text-to-ast.json") {
    assert_eq!(parse_text(&input).to_json_tree(), expected, "input: {input}");
}
```

This keeps the Rust tests in lockstep with the JS reference (re-run the script when
upstream specs change) and turns the parser phases mostly into implementation time.
The same script covers `quick_ast-to-text.spec.js` / `quick_ast-to-latex.spec.js`
fixtures for the output formatters (§12).

Two supplementary test sources beyond the extracted data maps:

1. **Oracle-generated edge fixtures** (`scripts/generate-edge-fixtures.mjs`): the
   upstream specs never exercise many lexer rules (unicode operator aliases,
   keyword boundaries, the `\big` delimiter families, `\var*` substitutions,
   sci-notation lookahead edges, several error paths). Since the Rust lexer is a
   hand-written reimplementation of regex tables, these dead spots are exactly
   where silent divergence hides. The script runs an edge-case corpus through the
   JS parsers and records whatever they produce (tree or error) as fixtures —
   JS as oracle, no invented expectations.
2. **Hand-ported option tests** (`tests/parser_options.rs`): the imperative test
   blocks at the bottom of the spec files (splitSymbols, custom function-symbol
   lists, allowSimplifiedFunctionApplication, parseLeibnizNotation off,
   parseScientificNotation off, conditional probability) — code paths no data-map
   fixture touches.

### 6e. Nesting-depth cap  ✓ done

Both parsers are recursive descent (deepest stacks in the codebase) driven by
untrusted input. A shared depth counter (`MAX_PARSE_DEPTH`, `parse/common.rs`) is
incremented on entry to the self-recursive functions and errors "Expression too
deeply nested" past the limit. Implementation findings that refined the plan:

- **Which functions to cap**: `statement`, `relation`, `expression`, `factor`,
  `base_factor` (via thin `enter()?` / `_inner` / `leave()` wrappers). The prefix
  chains (`----x`, `!!!!x`) recurse in `factor`/`relation`/`expression` *without*
  passing through `base_factor`, so those must be capped too. `statement` is
  essential and subtle: its bar-fallback catches the too-deep error, rewinds the
  lexer, and re-descends via `statement_bar_fallback` — capping `statement` (whose
  increment is *held* across both attempts) is what bounds nested `|…|`; capping
  only the descent functions is not enough (they `leave()` on unwind, freeing budget
  for the retry). Found via a stack-overflow that only `|…|` triggered.
- **Cap value = 64, not 256**. Measured (debug native): recursive descent costs ~4–5
  budget units and ~40 KB of stack *per bracket level*; a 1 MB stack overflows near
  25 bracket levels in debug and ~60 in release. Budget 256 fires above 63 levels —
  *after* the release stack already overflows, so it never protects. 64 fires at ~12
  levels: fits 1 MB in both profiles with margin, and is ~6× the whole fixture
  corpus's max nesting depth (which is **2**). Sized for the release wasm target; the
  real fix for a higher ceiling is an iterative parser (deferred).
- **`from_js` is NOT capped**: `serde_json` rejects deeply-nested JSON at
  deserialization (recursion limit 128), so the realistic `from_json` path never
  builds a tree deep enough to overflow. Documented on `from_js`. A hand-built
  `Value` could, but that is not a user-input vector.
- Deliberately NOT a pushdown-automaton rewrite: it would destroy the grammar-shaped
  readability that makes the parsers verifiable against the JS, for no benefit once
  the cap exists.

Tests: `tests/stack_safety.rs` — 10⁵-deep input of every recursion pattern yields a
`ParseError` (never a crash), reasonable nesting still parses, and a cap-depth tree
flows through canonicalize/equals/Drop in a 256 KB thread. (Overflow aborts the
process, so the deep tests run in explicitly-sized threads via `std::thread`.)

---

## 7. Normalisation (`src/norm/`)

### 7a. Flatten (`flatten.rs`)

Convert right-nested `Add`/`Mul` (and `And`/`Or`/`Union`/`Intersect`) into flat n-ary
form. The parser applies this before returning, matching the JS (`text-to-ast.js`
calls `flatten(result)` on its output) — so even faithful-layer trees are flat.

```rust
fn flatten(expr: Expr) -> Expr
// Add(Add(a, b), c) → Add(a, b, c)
// Mul(Mul(a, b), c) → Mul(a, b, c)
// all other variants: recurse into children
```

### 7b. Name and form normalisation (`names.rs`)

The equality path (§10 stage 1) depends on a larger normalisation suite than
constant folding. Port from the JS `Expression` methods used in `equality.js`:

- `normalize_function_names` — `arcsin` → `asin`, `ln` vs `log`, etc.
- `normalize_applied_functions` — canonical `Apply` shapes
- `normalize_negative_numbers` — `Neg(Num(n))` → `Num(-n)`, `Neg` → `Mul(-1, x)`
- `evaluate_numbers` (with `max_digits`) — fold numeric subexpressions to floats

(`normalize_angle_linesegment_arg_order` and `remove_scaling_units` follow when their
geometry/unit features are ported — see §17.)

`normalize()` is the single entry point that takes a faithful-layer tree (§5) to the
canonical layer: flatten → names → arith → order.

### 7c. Canonical ordering (`order.rs`)

**SymEngine-style**: a total order on `Expr` is defined so that commutative operators
always have sorted args. This makes structural equality a simple tree comparison.

Ordering priority (lower number = sorts earlier):
1. `Num` — sorted by value
2. `Const` — sorted by variant
3. `Sym` — sorted by interned index (alphabetical within session)
4. `Pow(base, _)` — sorted by base, then exponent
5. `Mul` — sort by leading non-numeric factor
6. `Apply` — sort by function name, then args
7. `Add` — sort by leading term

```rust
fn canonical_order(a: &Expr, b: &Expr) -> Ordering
```

Applied by the `add()` and `mul()` smart constructors. Once sorted, two equal expressions
have identical trees — structural equality becomes tree equality.

### 7d. Arithmetic simplification (`arith.rs`)

Applied during smart-constructor calls and by explicit `simplify()`:

- Constant folding: `Num(a) + Num(b) → Num(a+b)` (using `Number` arithmetic)
- Zero/one elimination: `x + 0 → x`, `x * 1 → x`, `x * 0 → 0`
- Power rules: `x^0 → 1`, `x^1 → x`, `0^0 → 1` (with warning), `1^n → 1`
- Negation: `--x → x`, `Neg(Num(n)) → Num(-n)`
- Rational combination: `Rat(a,b) + Rat(c,d) → Rat(ad+bc, bd)` (GCD-reduced)
- Combine like terms: `3x + 2x → 5x` (coefficient extraction + recombination)
- Combine powers: `x^2 * x^3 → x^5` (same base, add exponents)
- Flatten nested Pow: `(x^a)^b → x^(a*b)` when safe

### 7e. Higher rewrites (`simplify.rs`)

Non-trivial rewrites that require pattern matching:
- Collect common factors from `Add` terms
- Pull perfect squares from under square roots: `sqrt(12) → 2*sqrt(3)`
- Trig identity normalisation (sin²+cos²=1 for the equality-testing path)
- Logarithm rules: `log(a*b) → log(a)+log(b)`, `log(a^n) → n*log(a)`

These are applied as rewrite rules with a convergence loop (max iterations guard).

### 7f. Resource limits — decided 2026-07, NOT yet implemented

All expression input is untrusted (student answers), so every pass must stay
bounded on adversarial input. Current state: `Number` folds are capped
in-place (factorial ≤ 10000!, exact pow refused beyond ~10^6 result bits —
the cap composes because it re-checks materialized input sizes; decimal
exponents beyond ~10^6 digits approximate via f64), which makes the existing
pipeline polynomial in input size. Agreed follow-ups, in order:

1. **Stack safety** — the crash vector (deep untrusted input → wasm trap) is
   **closed** by the §6e parser depth cap (done). Remaining items are
   defense-in-depth, deferred: iterative `Drop` (§5b — 60-site refactor, now
   bounded by the cap so non-urgent) and the iterative `fold` driver for
   post-parse passes (§5b — also bounded by the cap; matters only if trees can
   exceed it via programmatic construction).
2. **`Limits` context when unpredictable machinery lands** (simplify §7e,
   polynomial GCD §8): `Limits { max_number_bits, max_nodes, fuel }` passed
   into those subsystems, generous defaults. Rewrite-to-convergence loops and
   polynomial GCD coefficient swell are inherently not boundable by local
   caps. Count **operations, not wall-clock** — verdicts must be identical on
   every machine (grading engine; reproducible tests).
3. **Perf note**: `add`/`mul` like-term combining is a linear scan with
   String-cloning symbol compares — quadratic on many-term inputs. Not a
   security issue (polynomial), but add an interned-Sym fast path before the
   poly layer leans on these constructors.
4. **Embedder wall**: internal limits make abuse hard; only a host timeout
   (web worker / task kill) makes it impossible. Document in the WASM
   bindings (§13) when they land.

---

## 8. Polynomial layer (`src/poly/`)

The polynomial layer exists separately from the expression tree — an `Expr` is converted
into a polynomial for coefficient-level algorithms, then converted back.

### 8a. Domain (`domain.rs`)

```rust
pub enum Domain {
    Z,          // ℤ — integer coefficients
    Q,          // ℚ — rational coefficients
    Fp(u64),    // ℤ/pℤ — prime field (for finite-field equality check)
}
```

The domain is tracked explicitly so GCD algorithms choose the right strategy.

### 8b. Dense univariate polynomial (`univariate.rs`)

```rust
pub struct DUP {
    pub coeffs: Vec<Number>,   // ascending: coeffs[i] is coeff of x^i
    pub domain: Domain,
}
```

For educational math, all univariate polynomials fit here. Operations:
- `add`, `sub`, `mul` (schoolbook — fine for small degree)
- `div_rem` (polynomial long division → returns `(quotient, remainder)`)
- `gcd` (via subresultant PRS to stay in ℤ — avoids coefficient explosion)
- `eval(x: Number) -> Number` (Horner's method)
- `degree() -> usize`

### 8c. Dense multivariate polynomial (`multivariate.rs`)

Following SymPy's DMP model: recursive — a polynomial in `K[x₁, x₂, ..., xₙ]` is stored
as a polynomial in `K[x₁, ..., xₙ₋₁][xₙ]`, i.e. as a `Vec<DUP>` where each element is
the coefficient polynomial in the remaining variables.

For educational math, most multivariate cases are bivariate or trivariate with low degree.

```rust
pub struct DMP {
    pub rep: DmpRep,          // recursive coefficient data — no metadata inside
    pub vars: Vec<Sym>,       // variable ordering (top level only)
    pub domain: Domain,       // (top level only)
}

// Pure nested-list representation, matching SymPy's dmp lists exactly.
// Invariants: uniform depth == vars.len() - 1 at every branch; degree descending;
// level k is a polynomial in vars[k] with coefficients one level down.
pub enum DmpRep {
    Ground(Vec<Number>),      // innermost level: univariate coefficient list
    Nested(Vec<DmpRep>),
}
```

Storing `vars`/`domain` only at the top level (rather than a nested `DMP` per
coefficient) avoids duplicating metadata at every node and removes the
all-levels-must-agree invariant; it also makes `DmpRep` line up 1:1 with the nested
lists SymPy's `dmp_*` algorithms are written against.

Operations: `add`, `sub`, `mul`, `div_rem` (pseudo-division for ℤ coefficients),
`gcd` (recursive using univariate GCD + content/primitive-part).

### 8d. GCD (`gcd.rs`)

**Univariate over ℤ** (subresultant PRS):
```
gcd(f, g) where f,g ∈ ℤ[x]
  1. extract content (integer GCD of coefficients) from each
  2. gcd of contents is part of result
  3. make both primitive
  4. subresultant PRS — computes remainder sequence without coefficient explosion
  5. last non-zero remainder is primitive GCD (up to sign)
  6. multiply by content GCD
```

**Univariate over ℚ**: divide numerators/denominators through to ℤ, apply ℤ GCD, divide back.

**Univariate over ℤ/pℤ**: Euclidean algorithm (all elements invertible in a field).

**Multivariate**: reduce to univariate by treating all but the main variable as coefficient
parameters. Use content/primitive-part decomposition recursively.

### 8e. Conversion (`convert.rs`)

```rust
// Expr → DUP or DMP (fails if expression is not polynomial in given variables)
fn expr_to_dup(e: &Expr, var: Sym, domain: Domain) -> Result<DUP, NotPolynomial>
fn expr_to_dmp(e: &Expr, vars: &[Sym], domain: Domain) -> Result<DMP, NotPolynomial>

// DUP / DMP → Expr
fn dup_to_expr(p: &DUP, var: Sym) -> Expr
fn dmp_to_expr(p: &DMP) -> Expr
```

**TDD milestone**: `tests/poly.rs` — port tests from SymPy's test_polys.py that are
relevant to educational math scope (GCD, factoring, arithmetic). Add tests mirroring
math-expressions' `factor()` and `simplify()` behaviors.

---

## 9. Evaluation (`src/eval/`)

### 9a. Numerical (`numerical.rs`)

Walk the expression tree, substituting variables and computing with `f64`:

```rust
fn eval_f64(e: &Expr, bindings: &HashMap<Sym, f64>) -> Result<f64, EvalError>
```

Unbound variables → `EvalError::UnboundVariable`. `Apply` dispatches to a built-in
function table: `sin`, `cos`, `tan`, `exp`, `ln`, `sqrt`, `abs`, `floor`, etc.

### 9b. Complex (`complex.rs`)

```rust
fn eval_complex(e: &Expr, bindings: &HashMap<Sym, Complex64>) -> Result<Complex64, EvalError>
```

Uses `num-complex`. Same dispatch table but complex variants.

### 9c. Finite field (`finite_field.rs`)

```rust
fn eval_fp(e: &Expr, bindings: &HashMap<Sym, u64>, p: u64) -> Result<u64, EvalError>
```

All arithmetic is `mod p`. Uses modular inverse for division. `p` is chosen to be a large
prime (e.g. `998_244_353`). Same dispatch table — trig functions are approximated by
polynomial expansion mod p (Taylor series truncated, or just not supported — the finite
field check is mainly for polynomial equality).

---

## 10. Equality testing (`src/eq/`)

Port the staged algorithm from `lib/expression/equality.js`. The actual JS chain
(verified against the source) is: syntactic → finite-field rejection → complex
sampling → discrete-infinite-set. Note that `equalsViaReal` is **commented out** in
the JS — there is no real-sampling stage; do not port dead code.

```rust
pub struct EqOptions {
    pub relative_tolerance: f64,             // default 1e-12
    pub absolute_tolerance: f64,             // default 0.0
    pub tolerance_for_zero: f64,             // default 1e-15
    pub allowed_error_in_numbers: f64,       // default 0.0
    pub include_error_in_number_exponents: bool,  // default false
    pub allowed_error_is_absolute: bool,     // default false
    pub allow_blanks: bool,                  // default false
    pub coerce_tuples_arrays: bool,          // default true
    pub coerce_vectors: bool,                // default true
}

pub fn equals(a: &Expr, b: &Expr, opts: &EqOptions) -> bool
```

**Stage 0 — blanks guard**: unless `allow_blanks`, either side containing an
`Expr::Blank` node → false (a variant check, not a magic-symbol scan).

**Stage 1 — syntactic check** (`syntax.rs`):
Both sides pass through the full normalisation suite (§7b/§7d):
`evaluate_numbers(max_digits=∞)` → `normalize_function_names` →
`normalize_applied_functions` → `normalize_negative_numbers` → `simplify()`, then
structural equality on the canonical trees. Fast accept if identical. Constant numeric
comparison respects `allowed_error_in_numbers`.

**Stage 2 — finite field check** (`finite_field.rs`):
Rejection only, and **skipped entirely when `allowed_error_in_numbers != 0`** (fuzzy
number matching invalidates exact modular evaluation). Assign random values in ℤ/pℤ to
all variables, evaluate both sides; if they differ → definitively not equal. Only
applicable to rational-function expressions; skip when the tree contains
transcendental functions that cannot be evaluated mod p. Resample if a random point
hits a pole (division by zero mod p).

**Stage 3 — complex sampling** (`complex.rs`):
The acceptance workhorse. Sample at random complex points; accept if
`|f(z) - g(z)|` is within tolerance at all samples (using `relative_tolerance` /
`absolute_tolerance` / `tolerance_for_zero`).

**Stage 4 — discrete infinite set** (`discrete_infinite.rs`):
Compare discrete-infinite-set expressions (e.g. periodic solution sets like
`x = π/4 + nπ`). Stubbed initially — see §17; the stub returns false, matching a
plain not-equal outcome.

The seeded RNG (deterministic for reproducible tests) uses `rand::SeedableRng` with a
fixed seed, same approach as `seedrandom` in the JS.

**TDD milestone**: `tests/equality.rs` — port `slow_math-expressions.spec.js` equivalence
pairs: `sin²x + cos²x = 1`, `(x+1)² = x²+2x+1`, `x²-1 = (x-1)(x+1)`, etc.

---

## 11. Assumptions (`src/assumptions/`)

```rust
pub struct Assumptions {
    /// Per-variable explicit facts
    by_var: HashMap<Sym, Vec<Assumption>>,
    /// Generic facts (applied to all variables)
    generic: Vec<Assumption>,
    /// Derived facts (computed from by_var + generic)
    derived: HashMap<Sym, Vec<Assumption>>,
}

pub enum Assumption {
    Sign(SignAssumption),           // > 0, < 0, != 0, >= 0, <= 0, = 0
    SetMembership(SetKind),         // ∈ ℤ, ∈ ℚ, ∈ ℝ, ∈ ℂ
    NonCommutative,
    // etc.
}
```

Port `lib/assumptions/assumptions.js` logic for:
- `add_assumption(tree)` — parse an `Expr` relation into an `Assumption`, add to `by_var`
- `get_assumptions(var)` — return all facts about a variable
- `calculate_derived_assumptions()` — transitive inference (if `x = 3`, infer `x > 0`)

Port `lib/assumptions/element_of_sets.js` for set membership logic.

**TDD milestone**: port a representative subset of `slow_assumptions.spec.js`.

---

## 12. Output formats (`src/output/`)  ✓ done (default converters)

**Architecture decision — clean-slate precedence-based printers walking
`Expr` directly** (not ports of the JS formatters). The JS `ast-to-text.js`/
`ast-to-latex.js` decide parenthesisation by regex-matching their own rendered
output over the ad-hoc JS tree shape; per the project goal ("do not repeat the
JS's design decisions"), the Rust formatters instead track numeric precedence:
`render(e) -> (String, prec)`, and a parent parenthesises a child only when
the child's precedence is below what the position requires. The precedence
ladder (`output/mod.rs::prec`) is derived from the parser grammars, so output
re-parses with minimal parentheses. `to_text`/`to_latex` take `&Expr` with no
`js_tree` hop (`from_js` exists only for the WASM `from_json` boundary).

**Correctness oracle — round-trip, not byte-matching.** `tests/roundtrip.rs`
takes every input in the parser tree-fixtures, parses it, renders it, and
re-parses: the result must be structurally equal (`PartialEq` on `Expr`). No
hand-authored expected strings; the JS output fixtures
(`ast-to-{text,latex}.json`) are kept as advisory reference only. A
`constructed_roundtrip` test adds expressions the fixture corpus misses
(NegInf in tight positions, floats needing many digits, huge integral floats).

Known inherent ambiguities are allowlisted with explanations in
`tests/roundtrip.rs` (`KNOWN_AMBIGUOUS`): raw expressions with a bare `d`
inside a fraction re-parse as Leibniz derivatives, and nested `|…|` is
formally ambiguous.

### 12a. Text (`text.rs`)

Precedence printer; unicode by default (`≤`, `θ`, `∞`) with an ascii option
field. Notable rules: sums split signs structurally (never from a `Mul`),
superscript/subscript slots hold only single tight atoms (anything else gets
parens), `∠A` shorthand vs `angle(...)`, Leibniz numerator/denominator
spacing so multi-char variables re-lex, floats render in positional decimal
(never exponential — the parsers' scientific-notation literals are
context-sensitive, so `3e-12` prints as `0.000000000003`).

### 12b. LaTeX (`latex.rs`)

Sibling printer exploiting brace grouping: `\frac{}{}`, `x^{...}`, `x_{...}`
are self-delimiting, so their contents never need parens. `\sqrt`/`\sqrt[n]`,
`\binom`, `\operatorname{}` fallback, `bmatrix`, `\circ` exponent units,
`Rat` renders as `\frac`.

**Deferred:** number padding (`padToDigits`/`padToDecimals`) and the option
tests (ascii/non-unicode output, `explicitMultiplicationSymbols`,
`matrixEnvironment`) — same follow-up pattern as `tests/parser_options.rs`.
Option fields are already on the `*Opts` structs.

---

## 13. WASM bindings (`src/lib.rs`)

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Expression(Expr);

#[wasm_bindgen]
pub fn parse_text(s: &str) -> Result<Expression, JsValue> { ... }

#[wasm_bindgen]
pub fn parse_latex(s: &str) -> Result<Expression, JsValue> { ... }

#[wasm_bindgen]
impl Expression {
    pub fn to_text(&self) -> String { ... }
    pub fn to_latex(&self) -> String { ... }
    // &self / &Expression throughout: a by-value wasm-bindgen parameter is CONSUMED
    // and the JS object becomes unusable after one call.
    pub fn equals(&self, other: &Expression, opts: JsValue) -> Result<bool, JsValue> { ... }
    pub fn simplify(&self) -> Expression { ... }
    pub fn derivative(&self, var: &str) -> Result<Expression, JsValue> { ... }
    pub fn evaluate_to_f64(&self, bindings: JsValue) -> Result<f64, JsValue> { ... }
}
```

No arena, no manual `free_expr`: wasm-bindgen generates a JS class whose instances own
the Rust value, expose `.free()`, and are auto-freed via `FinalizationRegistry` when
garbage-collected. `opts`/`bindings` cross the boundary as `JsValue` deserialised with
`serde-wasm-bindgen` (no JSON-string double parse).

Parse errors surface as structured `ParseError { message, span }` (§6) converted to a
JS object — error quality matters in an educational tool where malformed input is the
common case, and spans are painful to retrofit later.

**Stack & isolation notes** (§5b/§6e/§7f): size the shadow stack explicitly in the
wasm build (`-C link-arg=-zstack-size=N`) against measured worst-case frames ×
depth cap; document that embedders should run the module in a web worker (or
killable server task) — internal limits make abuse hard, only a host timeout makes
it impossible.

---

## 14. Key dependencies

```toml
[dependencies]
num-bigint  = { version = "0.4", features = ["serde"] }
num-rational = { version = "0.4" }
num-traits  = "0.2"
num-complex = "0.4"
# default-features = false is load-bearing: rand's defaults pull in getrandom, which
# fails on wasm32-unknown-unknown unless its js backend is enabled.  We seed
# explicitly (SmallRng::seed_from_u64) and never need OS entropy.
rand        = { version = "0.9", default-features = false, features = ["small_rng"] }
wasm-bindgen = "0.2"
serde       = { version = "1", features = ["derive"] }
serde-wasm-bindgen = "0.6"   # options/bindings across the JS boundary
serde_json  = "1"            # to_json()/from_json() and test fixtures only

[dev-dependencies]
wasm-bindgen-test = "0.3"
proptest = "1"
```

No rayon (not needed; single-threaded WASM). No lalrpop or nom (parsers are hand-written
recursive descent, matching the JS). No hyperreal. No ordered-float (the `F64` newtype
in §3 covers it).

---

## 15. Implementation sequence (TDD phases)

Each phase: write tests first, then implementation until all tests pass.

### Phase 1 — Numbers, symbols, and scaffolding (week 1)
- `Number` type with arithmetic, GCD, Display
- Symbol interner
- CI from day one: `cargo test` + `cargo build --target wasm32-unknown-unknown` +
  `wasm-pack test --node` on every push (WASM breakage is much cheaper to catch per-commit)
- Fixture-extraction Node script (§6d) — run once, commit the JSON fixtures
- Tests: `tests/num.rs` (including proptest arithmetic laws: commutativity,
  associativity, GCD-reduction invariants, overflow promotion round-trips), `tests/sym.rs`

### Phase 2 — Expression tree and text parser (weeks 2–3)
- `Expr` enum, plain constructors (faithful layer), smart constructors (canonical layer)
- Lexer (with spans) and `ParseError`
- Text parser (recursive descent, plain constructors only)
- Basic pretty-printer (for test assertion readability)
- Tests: `tests/text_parse.rs` (~540 fixture cases from `quick_text-to-ast.spec.js`)
- Property test: `parse_text(to_text(e)) == e` round-trip on generated trees

### Phase 3 — LaTeX parser (week 4)
- LaTeX lexer extensions
- LaTeX parser
- Tests: `tests/latex_parse.rs` (~630 fixture cases from `quick_latex-to-ast.spec.js`)
- Once both parsers are green: convert string token types to enum(s) (§6a step 3),
  with the fixture suites as the safety net

### Phase 4 — Normalisation (week 5)
- Prerequisite: complete `Number` arithmetic (§3) + exact decimal literals
  (§3a) — constant folding is exact from day one
- Flatten, canonical order, constant folding, zero/one elimination
- Name/form normalisation suite (§7b) — needed by equality stage 1
- `to_text` and `to_latex` output formatters (fixtures from `quick_ast-to-text` /
  `quick_ast-to-latex` specs)
- Tests: `tests/norm.rs`

### Phase 5 — Polynomial algorithms (weeks 6–7)
- First: stack safety steps 1–2 (§5b iterative `Drop` + §6e depth cap) and the
  `Limits` context (§7f) — poly GCD is the main coefficient-swell risk and must
  take a budget from day one; driver ports (§5b) can proceed in parallel
- `DUP`: arithmetic, division, GCD, `eval`
- `DMP`: arithmetic, division (pseudo-remainder), GCD
- `Domain` system
- `expr_to_dup`, `dup_to_expr` converters
- Tests: `tests/poly.rs`

### Phase 6 — Evaluation and equality (week 8)
- `eval_f64`, `eval_complex`, `eval_fp`
- Equality testing (stages 0–3; discrete-infinite-set stubbed per §10/§17)
- Tests: `tests/eval.rs`, `tests/equality.rs`
  (port from `slow_math-expressions.spec.js`)

### Phase 7 — WASM bindings and browser smoke test (week 9)
- `src/lib.rs` bindings
- `wasm-pack build`
- Drop-in JS wrapper maintaining the existing math-expressions API surface
- Manual smoke test: parse `"sin^2 x + cos^2 x"`, check `equals("1")` returns true
- Differential test harness: the JS library stays runnable as the reference oracle —
  generate random expressions (and replay the fixture corpus), run both
  implementations' parse/`equals`, diff the JSON trees and verdicts. Catches fidelity
  drift that fixture tests miss; keep it running in CI from here on.

### Phase 8 — Assumptions and differentiation (week 10)
- Assumptions system
- Symbolic differentiation (port `lib/expression/differentiation.js`)
- Tests: assumptions subset, differentiation cases

---

## 16. Notes on SymEngine design patterns borrowed

- **Type = operator**: `Expr::Add(...)` rather than `Expr::BinOp(Op::Add, ...)`. Dispatch
  via Rust `match` is monomorphic and zero-cost.
- **Sorted args**: `Add` and `Mul` always sort args in canonical order at construction time
  (the smart constructors enforce this). Structural equality then works directly.
- **Immutable nodes**: `Expr` is `Clone`; algorithms return new trees rather than mutating.
  No `Arc` needed (single-threaded WASM).
- **Symbol interning**: symbols are `u32` indices, comparison and hashing are O(1).
- **No flat arena initially**: a `Vec<Node>` + u32-index representation would solve
  stack safety structurally (no pointer chains; `Drop` is one `Vec` free) and speed up
  traversals, but invalidates every `match`-based pass and the faithful-port parsers
  mid-project. Natural post-Phase-7 optimisation, together with hash-consing below;
  the §5b `fold` driver survives that migration (only `children()` changes).
- **No hash-consing initially**: hash-consing (sharing identical subtrees) is an optimisation
  that can be added later behind the smart constructors without changing any API.

---

## 17. Notes on what is NOT ported (out of scope initially)

- MathML parser/output
- GLSL output
- Guppy output
- Matrix operations beyond basic arithmetic
- Numerical integration (`integrateNumerically`)
- `solve_linear`
- Units system (`remove_units`, `add_unit`) — until ported, equality stage 1 omits
  the JS `remove_scaling_units` normalisation step
- `±` (plus-minus) operator
- Piecewise functions
- Discrete infinite sets (`create_discrete_infinite_set`, `equalsDiscreteInfinite`) —
  note this is stage 4 of the `equals` chain (§10); the stub returns false until
  ported, so periodic solution sets (e.g. `x = π/4 + nπ`) compare as not-equal
- `normalize_angle_linesegment_arg_order` (geometry-specific normalisation)

These are phase 2 additions once the core is solid.
