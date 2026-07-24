# math-expressions Rust Port — Detailed Plan

> **PROGRESS (audited 2026-07-20):** IN PROGRESS — master porting plan; core
> port (parsers, normalization, poly, eval, equality, wasm) is landed and
> differential-tested. Remaining gaps live in `WHATS_LEFT.md`: §A.1 converters
> (MathML/GLSL/Guppy/mathjs — most marked *not needed for Doenet*) and item 20
> derivative step-recording (**deferred**). Rust is ahead of JS on integrate,
> arbitrary precision, rootof, symbolic eigen, ODE. Not DONE.

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

**Update 2026-07-19**: the verdict above concerns the *number backend* only. For
arbitrary-precision **evaluation** (a separate, planned feature), the techniques from
hyperreal and its ancestor `realistic` were analyzed in depth and selectively adopted —
see `active-plans/DONE_ARBITRARY_PERCISION_PLAN.md` (scaled-integer `approx(p)` contract, guard-bit
discipline, series kernels, bounded tri-state sign; rejected: lazy DAG + per-node caches
in favor of a compiled evaluation tape).

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

(`normalize_angle_linesegment_arg_order` ✓ done — sorts `angle`/`linesegment` args;
lives both in `canonicalize` for the full `equals` path and in `norm/syntactic.rs` for
`equalsViaSyntax`. See §10.)

**Scaling units ✓ done** (2026-07). `desugar_units` in `src/norm/mod.rs` is the
equality-time analogue of JS `remove_scaling_units` + numerical unit removal. Rather
than a JS-style two-function split (scale-only vs. full removal), it rewrites the three
units to plain arithmetic — `n% → n/100`, `n deg → n·pi/180`, `$n → $·n` — so the
existing like-term folding and numerical sampling need no unit special-casing:
`$3+$2 = $5` falls out of like-term combining, and `$` sampling as a free variable
keeps `$5 ≠ 5`. Applied only in the full `equals` path, never `equalsViaSyntax`, so
`50%` and `1/2` stay syntactically distinct (matching the `symbolic_nonequivalences`
corpus). Parser already emitted the `["unit", …]` nodes; this closed the last gap.
Corpus: 678 → 685/824.

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

> ⚠️ **Not started, and must be a redesign — see §7e′.** The JS simplifier is the
> library's performance bottleneck (its tests are the quarantined `slow_*` suite).
> Do not port it as-is; §7e′ records the slow-test oracle, the diagnosed root
> causes, and the intended algorithm.

Non-trivial rewrites that require pattern matching:
- Collect common factors from `Add` terms
- Pull perfect squares from under square roots: `sqrt(12) → 2*sqrt(3)`
- Trig identity normalisation (sin²+cos²=1 for the equality-testing path)
- Logarithm rules: `log(a*b) → log(a)+log(b)`, `log(a^n) → n*log(a)`

These are applied as rewrite rules with a convergence loop (max iterations guard).

### 7i″. Review hardening ✓ done 2026-07-18

A second multi-agent review of the diff/expand/ops/wasm work found 10 issues; all
fixed (regression-tested with bounded parameters — the review itself froze the
container twice by empirically reproducing the blowups, hence the new devcontainer
20 GB memory cap and the analytical-verification policy):
- **expand blowup (DoS)**: `distribute_product` now combines like terms after
  every factor (`(a+b)^64` = 65 live terms, was 2⁶⁴ clones) and bails to the
  unexpanded product past a 4 000-raw-term cap. Div arm keeps the denominator
  *unexpanded* (doc contract + mathjs parity) and routes through the same
  capped distributor.
- **Rounding DoS ×2**: `round_to_decimals` resolves |d| > 4000 semantically
  (unchanged / zero-by-magnitude) instead of materializing 10^|d|;
  `round_numbers_to_precision` locates the leading digit via the new
  `Number::magnitude_log10` (f64 when finite, bit-length fallback for Big
  values beyond f64 range) with i64 arithmetic — no more i32 overflow on
  pasted `1e-400` / 350-digit literals. (First clamp draft used ±10⁶: memory-
  safe but ground debug-mode bignum for minutes; semantic resolution replaced it.)
- **diff catch-all**: undifferentiable shapes containing the variable (tuples,
  relations, subscripts…) now yield an opaque `derivative(e, var)` OtherOp
  (samples as an opaque atom) instead of a false `0`.
- **variables()**: Apply heads dropped wholesale (JS `tree.slice(2)` parity) —
  `f'(x)` no longer leaks `f` as a variable.
- **wasm `evaluate_to_constant`** returns `Option<f64>` (JS `number|undefined`,
  preserving upstream's null-vs-value) with a magnitude-relative imaginary
  tolerance.
- **finite_field pole-conservatism**: since nested-pow flattening, `1/x^a` is
  `Pow(x, Mul(-1,a))`; a zero base with a non-literal exponent is now NaN
  (skip prime) in the generic pow path, restoring the reciprocal-pole
  semantics the literal-negative-exponent branch used to guarantee.
- **NEW BUG caught by the review's scratch probes**: `mul()`'s like-power
  combining can produce an integer power of a product, which the distribution
  rule returns as a `Mul` — previously pushed *nested* into the outer product,
  breaking the flat-Mul canonical invariant (`z·(xy)^{1/2}·(xy)^{3/2}` →
  `Mul([z, Mul([x²,y²])])`). `mul()` now refolds distributed factors so they
  merge/cancel (`x⁻²·(xy)^{1/2}·(xy)^{3/2}` → `y²`). Regression in
  `tests/norm.rs`. Note: `equals("(x^2)^(1/2)", "x")` is *true* and correct —
  the JS oracle returns true (lenient branch-cut region acceptance, same
  mechanism as the log identities).
- **Structure**: `Expr::children()` / `Expr::any_subexpr(pred)` are now the
  single read-only traversal (contains_var / contains_blank /
  involves_nonfinite are one-liners over it); `sym::CONSTANT_SYMBOLS` +
  `is_constant_symbol` is the single source for the `pi`/`e`/`i` set.

Deferred from the review (recorded, not fixed): the §7f `Limits{fuel}` context
(5 ad-hoc caps now exist — the plan's own trigger has fired); diff's O(n²)
`contains_var` rescans (per-call template parsing ✓ fixed 2026-07-18 with a
*thread-local* parse cache — a global OnceLock would leak one thread's Sym ids
into another's interner); corpus
snapshot-machinery dedup across the 5 corpus tests; generator-script helper
dedup; `constants_to_floats` ignoring the `Const(MathConst::Pi/E)` spelling.

### 7h″. Number normalization ✓ done 2026-07-18

`src/ops.rs` + `Number::round_to_decimals`: `constants_to_floats` (pi/e → float,
`i` kept), `round_numbers_to_decimals(n)`, `round_numbers_to_precision(n)` (sig
figs). Rounding is **exact on rationals** (ties away from zero) — since user
decimals are exact `Rat`, `2.345` is exactly `469/200` and rounds to `2.35` with
no float ambiguity (matches JS's intended output, which JS itself only gets via
float correction). Tests: `tests/number_ops.rs` (JS-verified values). Also on the
wasm `Expression`.

### 7g″. Numeric evaluation: `evaluate` / `evaluate_to_constant` ✓ done 2026-07-18

`src/ops.rs`, built on `eval_complex`:
- `evaluate(e, &HashMap<String,f64>) -> Option<Complex64>` — evaluate at real
  bindings; `None` on unbound var / non-finite result. **Complex principal
  branch, matching mathjs `.evaluate`**: `x^(1/3)` at `x=-8` is `1+i√3`, not `-2`.
- `evaluate_to_constant(e) -> Option<Complex64>` — `None` if the *original*
  expression has any genuine free variable (`pi`/`e`/`i` excluded, and it does
  NOT cancel first, so `x−x` → `None` not `0`); otherwise **simplify** then
  evaluate, so real-domain reductions apply (`(-8)^(1/3)` → `-2`). This free-var-
  first + simplify behaviour was reverse-engineered from the differential corpus
  (11 cancellation cases like `x−x`, `y/y` initially diverged).
Tests: `tests/evaluate.rs` + `tests/evaluate_corpus.rs` (differential, random
real bindings, **499/500**; the 1 snapshot failure is a log branch-cut sign tie
at a negative-real argument — inherent to complex `powc`/`ln` at the cut).

### 7f″. Utilities: `substitute` and `variables` ✓ done 2026-07-18

`src/ops.rs`: `substitute(e, &HashMap<String,Expr>)` (simultaneous, one-pass, no
simplification — `x²` with `x→2` is `2²` not `4`; `{x:y, y:x}` swaps) and
`variables(e) -> Vec<String>` (free variable names in first-appearance order,
deduped; `pi`/`e`/`i` included as they are ordinary symbols, function-application
heads excluded). `diff.rs`'s template substitution now reuses `ops::substitute`.
Tests: `tests/ops.rs` (hand cases) + `tests/ops_corpus.rs` (differential:
`variables` matches JS's array *exactly* incl. order, `substitute` via `equals`,
200 random inputs). Note: `.factor()` is NOT in this library version (no
polynomial layer needed for the public API).

### 7e″. Expansion (`expand`) ✓ done 2026-07-18

`me.expand()` (mathjs-backed) is ported in `src/norm/expand.rs` (`pub fn expand`):
distribute multiplication / division-numerator / negation over sums,
multinomial-expand non-negative integer powers of sums (capped at
`MAX_EXPAND_POWER = 64` for untrusted input), recursing everywhere incl. function
args; denominators and non-integer/negative powers are left intact. Built on the
canonical smart constructors, so output is canonical and like-terms-combined.
Tests: `tests/expand.rs` (hand cases) + `tests/expand_corpus.rs` (differential vs
JS `.expand()`, **246/247**, the 1 snapshot failure a degenerate `…/(2−2)`
division-by-zero). **Finding:** mathjs `expand` HANGS on some inputs (a
product-of-sums divided/wrapped by a sum, e.g.
`((x+6)^3·(a−2)^3·(z−y)^2)/(2+6)`); Rust expands them in ~ms. The generator uses a
killed-on-timeout subprocess; the hanging inputs are recorded and
`does_not_hang_where_mathjs_does` asserts Rust stays bounded.

### 7e′. Simplification performance — REDESIGN, do not port the JS algorithm as-is

> **Placeholder for a from-scratch simplifier.** The JS simplification code is the
> slowest part of the library — slow enough that its tests are quarantined into a
> separate `slow_*` suite (`package.json` runs only `vitest … spec/quick_`; the
> `slow_*` specs are excluded from the default run). When §7e is implemented, treat
> this as a **clean-slate design task**, not a line-by-line port. The JS behaviour is
> the correctness oracle (via the `slow_*` fixtures); its *algorithm* is not.

**Slow tests whose cost is the simplification algorithm** (the port must be *fast* on
these, using them as the performance oracle):

| Spec | Tests | Why simplify-bound |
|---|---|---|
| `spec/slow_simplify.spec.js` | 74 | The simplify suite itself — `evaluate_numbers`, `collect_like_terms_factors`, root/log/trig rewrites. 154 `.simplify()` calls. |
| `spec/slow_assumptions.spec.js` | 44 | Assumption-driven simplification; every `is_positive`/`simplify`-under-assumptions path runs the same matcher. 8k lines. |
| `spec/slow_check-symbolic-equality-numerical-errors.spec.js` | — | Symbolic equality = normalize → **simplify** → `equalsViaSyntax`; the simplify pass dominates. |
| `spec/slow_math-expressions.spec.js` (symbolic pairs) | 14 | Full `equals` calls simplify before `equalsViaSyntax`; the symbolic-equivalence cases are simplify-bound. |

(For contrast, `slow_polynomial` (GCD coefficient swell), `slow_matrix` (matrix ops),
`slow_rational`, and `slow_check-equality-numerical-errors` (complex sampling) are slow for
**other** reasons and are *not* simplify targets.)

**Root causes diagnosed in the JS source (what NOT to reproduce):**
1. **Combinatorial pattern matching.** `collect_like_terms_factors` matches templates with
   `allow_permutations: true` (28 call sites), and `matchOps` in `trees/basic.js` backtracks
   over **operand permutations** — worst-case super-polynomial in the number of terms/factors
   of a sum/product. Large flat `Add`/`Mul` nodes blow up.
2. **Unbounded re-normalization to a fixpoint.** `evaluate_numbers_sub` is re-run through
   `default_order` repeatedly with a literal `// TODO: determine how often have to repeat` —
   no principled convergence bound; each pass re-sorts and re-matches the whole tree.
3. **Whole-tree re-sorting per pass.** `default_order` sorts the entire tree on every
   iteration rather than maintaining order incrementally.

**Design direction for the Rust simplifier (to be fleshed out when §7e starts):**
- Canonical form already gives sorted, flattened, like-combined `Add`/`Mul` (this is `norm`,
  done and cheap). Build rewrites *on top of the canonical invariant* so like-term collection
  is a **linear merge over sorted operands**, never a permutation search.
- Represent rewrite rules as a fixed, ordered set applied **bottom-up once** with an explicit,
  small convergence bound (fuel from the §7f `Limits` context) — not "repeat until stable".
- Key terms/factors by a cheap structural key (interned-symbol index or a hash of the canonical
  subtree) so matching is hash/lookup, not template unification.
- Keep it assumption-aware but assumption-*optional*: the equality path needs only the
  assumption-free subset, so the common case must not pay for the assumptions machinery.
- Gate everything on operation-count `fuel` (§7f) — untrusted student input.

**Oracle — decided 2026-07-17: own-reducedness metric, NOT tree-match to JS.** The
earlier "assert identical results to JS" wording is superseded — matching JS's exact
tree conventions (arg order, `Neg` vs `Mul(-1,·)`, `Div` shape) contradicts the
clean-slate mandate, and JS `.simplify()` output is convention-laden. Instead the
simplifier is judged on two intrinsic properties, with JS only a correctness
cross-check:
- **meaning-preserving**: `equals(simplify(e), e)` for every input (correctness).
- **reduced (fixpoint)**: `simplify(simplify(e)) == simplify(e)` structurally — no
  rule applies twice; this is our definition of "fully reduced", independent of JS.
- **JS cross-check (advisory)**: `equals(simplify(e), js_simplify(e))` — confirms we
  didn't reduce past or short of JS's *meaning*, without copying its *form*.

**Presentation layer ✓ added 2026-07-18 (`norm/present.rs`, `tests/display.rs`).**
The canonical form is optimized for equality, not reading (`1 + x^2 + 2 x`,
`x^(-1)` for `1/x`), which fails the "recognizable as simplest form by a calculus
student" bar. A display-only `present` pass now converts canonical → faithful at
the end of every user-facing operation (`simplify`, `simplify_with`, `expand`,
`derivative`, `evaluate_numbers`, `reduce_rational`; wasm inherits): Add terms
sort by descending total degree with graded-lex tie-break (constants/functions
degree 0, so they land last; matches the JS oracle on every probed case including
`-x + 2` and `x^(-n + 1)`), negative exponents become `Div` with rational
coefficients joining the fraction bar (`(2/3)x⁻¹ → 2/(3 x)`, `(1/2)x → x/2`),
negative leading coefficients become `Neg`, `Mul` factors sort alphabetically by
base, and non-integer rational *exponents* display as fractions (`x^(3/2)`, not
`x^1.5` — scoped to exponents so additive decimal folds keep the §3a decimal
round-trip: `0.1 + 0.2 → 0.3`, not `3/10`). The pass is meaning-preserving and
idempotent (`tests/display.rs` asserts both), and internal consumers that
pattern-match canonical shapes use the unpresented cores (`simplify_core`,
`expand_core` — `eq/`, `grade.rs` coefficient extraction, `evaluate_to_constant`).

**Baseline measured 2026-07-17** (`tests/simplify_corpus.rs`, 342 real inputs
harvested from `slow_simplify.spec.js` by `scripts/generate-simplify-corpus.mjs`):
the existing aggressive `canonicalize` already matches JS `.simplify()` structurally
138/342 and semantically 289/342. The 53 semantic misses cluster into three in-scope
targets: **∞/NaN folding** (~22: `1/0→∞`, `0/0→NaN`, `0·∞→NaN` — arith-layer),
**tuple/vector componentwise arithmetic** (~13: `(a,b)+(c,d)→(a+c,b+d)`,
`c·(a,b)→(ca,cb)`), and **radical simplification** (~7: `(-8)^(1/3)→-2`,
`cbrt(-x²)→-cbrt(x²)`, pull perfect powers from roots — also unblocks the last
equality-corpus failure). Blank-containing inputs (~3) are false misses (the equals
blank-guard), not gaps. The other ~150 structural-only diffs are pure form and are
*not* failures under the reducedness oracle.

**Acceptance:** all three clusters reduce to a meaning-preserving fixpoint; the corpus
harness snapshots remaining reducedness gaps (known-failures list, shrink over time,
same pattern as the equality corpus); everything gated on operation-count `fuel` (§7f).

**Status 2026-07-17 — scaffold + all three clusters landed** (`src/norm/simplify.rs`,
`simplify` exported). `simplify` runs a bottom-up rewrite to a canonical fixpoint
(bounded by `MAX_ROUNDS`, a `fuel` stand-in until §7f lands). Corpus:
**289 → 327/342** agree with JS `.simplify()`, and the two hard invariants
(meaning-preserving, idempotent fixpoint) hold across the corpus.
- **tuple/vector arithmetic** (+14): scalar·seq distribution and componentwise
  addition of same-kind/length vector-like sequences (Tuple/Array/Vector/AltVector),
  which together also cover subtraction and mixed shapes via the fixpoint loop.
- **radical simplification** (+13): odd-root sign pulling (`cbrt(-x²)→-cbrt(x²)`),
  perfect q-th-power extraction from integer coefficients (`cbrt(-16x⁴)→-2·cbrt(2x⁴)`,
  `sqrt(12)→2√3`), and exact numeric powers (`(-8)^(1/3)→-2`). Real-domain identities,
  so validated against JS output (the complex `equals` correctly rejects them).
- **∞/NaN folding** (+11): poles (`1/0→∞`), infinity absorption in ±/·, `x/∞→0`,
  `∞−∞→NaN`.
- **trig Pythagorean** (equality-corpus only): `C·sin(θ)² + C·cos(θ)² → C`, matching
  both `sin(x)^2` and `sin^2(x)` canonical spellings. Doesn't move the simplify-corpus
  number (JS `.simplify()` doesn't apply it either), but wired into `equals` stage 1 it
  closes the last equality-corpus gap → **824/824**.

The 15 residual gaps are all intended/irreducible, not simplifier bugs: 10 **exact-model
divergences** where our answer is the correct CAS choice and JS follows float semantics
(`0·∞`/`0/0`→`0`, `0^0`→`1`, signed-zero `6/-0`→`+∞`); 3 **blank artifacts** (`equals`'
stage-0 guard can't compare `Blank`s); 1 **deferred** power-of-product expansion
(`cbrt((-x)^3)`); 1 **`equals` float/rational bridging** quirk (`0.5·7`→exact `7/2` vs
JS `3.5`). The corpus's meaning-preserving invariant exempts blank and non-finite results
(outside `equals`' finite-complex-sampling domain), mirroring the equality path.

Done since: `simplify` wired into `equals` stage 1 (§10) and the trig Pythagorean rule
added → equality corpus **824/824**. Still open (follow-ons): broader trig/log identity
rules; power-of-product expansion (`cbrt((-x)^3)`); the `slow_assumptions` corpus and
assumption-aware subset (§11).

**Review + hardening 2026-07-17** (multi-agent review of the §7e diff; all fixes landed,
regression-tested in `tests/equality.rs`):
- **DoS**: `extract_qth_power` trial division was unbounded — `sqrt(<19-digit prime>)`
  stalled `equals()` 5–16 s. Now O(log) integer-nth-root for the perfect-power case,
  divisor cap 1024 for partial extraction (large prime factors just stay under the
  radical).
- **∞/NaN folds were unsound with symbolic operands**: `x·∞ == ∞`, `x/0 == 1/0`,
  `∞·(a,b) == ∞`, and `x+∞−∞ == y+∞−∞` all compared *true*. Folds now fire only when
  every operand is a definite constant (`Num`, `Const`, zero-pole, or the constant
  symbols `pi`/`e`/`i` — the same set the evaluator binds). Also added the missing
  `(−∞)^n` parity arm.
- **Coercion ordering**: `coerce_seqs` now runs *before* simplify in `equals`, so
  `[1,2]+(3,4) == [4,6]` combines componentwise under the coercion flags.
- **Altitude**: the `sin^2(x)` head-exponent unification moved from a double-match in
  the trig rule into `canon_apply` (reusing syntactic.rs's MOVE_EXPONENT_OUTSIDE set) —
  the canonical layer now has ONE spelling for powers of applied functions.
- **Perf**: `equals` regained a canonicalize-only fast path (stage 1a) before the
  rewrite clusters run; `rewrite` threads a fired-flag so a no-op pass skips the
  re-canonicalize and tree compare (`simplify_canonical` entry point avoids double
  canonicalization).
- **Dedup**: shared `map_children` (syntactic.rs, now generic over `FnMut`),
  `split_coeff` reused by the radical rule, `eq::contains_blank` made pub.
- **Test hardening**: a panic in `simplify` now *fails* the corpus invariants test
  (was silently skipped); the `agrees ||` escape in the meaning-preserving invariant is
  documented as a known hole (acceptable while rules are identity-derived, not
  JS-pattern-matched).
Corpora after hardening: equality **824/824**, simplify 327/342 (same 15 documented
gaps; `∞·i` kept working via the constant-symbol set).

### 7f. Resource limits — ✓ Limits context done 2026-07-18

**`src/resource_limits.rs`**: one `Limits` struct owns all deterministic caps (previously
seven scattered constants): `max_expand_power`, `max_expand_terms`,
`max_simplify_rounds`, `max_trial_divisor`, `max_factorial`, `max_residues`,
`max_round_decimals`, `max_pow_bits`. Defaults = the former constants. Sites
read a thread-local via `limits::current()` (no signature churn; single-threaded
WASM, same precedent as Sym interning); embedders scope overrides with
`limits::with(custom, || …)` (panic-safe restore). All counts are operations/
sizes, never wall-clock — verdicts identical on every machine. Scoped-override
behaviour is tested (`limits_are_scoped_and_effective` in tests/norm.rs).
Remaining §7f items (below) unchanged: stack-safety follow-ups are still
deferred, and the host-timeout embedder wall still applies.

<details><summary>original decision notes</summary>

### (original) Resource limits — decided 2026-07

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

</details>

---

## 8. Polynomial layer (`src/poly/`) — ✓ core done 2026-07-18 (scoped)

**Implemented** (`src/poly/mod.rs`, crate-internal): the recursive dense
multivariate model of §8c over **ℚ only** (no Domain enum — the one public
consumer needs ℚ), with add/mul/shift, exact division, pseudo-remainder,
content/primitive-part, and **primitive-PRS GCD**. Ground gcd uses the
integer-primitive rational convention (`gcd(p1/q1, p2/q2) = gcd(p1q2, p2q1)/(q1q2)`),
which both normalizes results and bounds PRS coefficient swell. Deterministic
caps: degree ≤ 64 per variable, PRS/division steps ≤ 128 (§7f).

**Consumer**: `ops::reduce_rational` (public + wasm) — bottom-up over the
canonical tree, splits each `Mul` into numerator/denominator (negative-integer
`Pow` factors), converts both to polynomials over the shared variables,
cancels the GCD, normalizes rational content into a single numerator scalar,
and re-canonicalizes. JS-oracle cases: `(x²−1)/(x−1) → x+1`,
`(x²−5x+6)/(x²−4) → (x−3)/(x+2)`, multivariate `(x²−y²)/(x−y) → x+y`;
non-polynomial fractions (`sin x / x`, `π/x`) untouched. Tests:
`tests/reduce_rational.rs` (canonical structural equality + value
preservation). NOT implemented from the original sketch (no consumer):
Domain ℤ/𝔽p, factoring, `solve_linear` — see §17.

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

### 9c. Finite field — ✓ done, see §10 stage 2 (`src/eq/finite_field.rs`)

The finite-field evaluator lives with the equality tester (it exists only to serve the
rejection filter) rather than under `eval/`. It uses the JS construction (`e` = primitive
root, small primes ≡ 1 mod 4, multivalued `ZmodN`, Tonelli–Shanks), *not* a single large
prime — that is what makes exp/trig identities hold in the field. The sketch below was the
original plan and is superseded.

<details><summary>original sketch (superseded)</summary>

```rust
fn eval_fp(e: &Expr, bindings: &HashMap<Sym, u64>, p: u64) -> Result<u64, EvalError>
```

All arithmetic is `mod p`. Uses modular inverse for division. `p` is chosen to be a large
prime (e.g. `998_244_353`). Same dispatch table — trig functions are approximated by
polynomial expansion mod p (Taylor series truncated, or just not supported — the finite
field check is mainly for polynomial equality).

</details>

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
    pub allowed_error_in_numbers: f64,       // default 0.0; != 0 ⇒ collapse numeric leaves before stage 1 (§18, #372)
    pub include_error_in_number_exponents: bool,  // default false
    pub allowed_error_is_absolute: bool,     // default false
    pub allow_blanks: bool,                  // default false
    pub coerce_tuples_arrays: bool,          // default true; pattern matching must reuse this too (§18, #797)
    pub coerce_vectors: bool,                // default true
}

pub fn equals(a: &Expr, b: &Expr, opts: &EqOptions) -> bool
```

**Fuzzy number matching ✓ done 2026-07-18** (`allowed_error_in_numbers`,
`include_error_in_number_exponents`, `allowed_error_is_absolute` on `EqOptions`):
stage 1 gains a fuzzy structural compare (`fuzzy_tree_eq`, port of
`trees/basic.js equal` — numbers within `max(1e-14, err)·min|l,r|` relative or
`max(1e-14·min, err)` absolute; `Pow` exponents exact unless included); the
finite-field stage is skipped when an error is allowed (exact arithmetic would
reject what the allowance accepts, JS parity); and the sampler adds a
**first-order sensitivity tolerance** (port of the JS `tolerance_function`):
numbers in the first argument are replaced by parameters, and
`err · Σᵢ ∂f/∂pᵢ · valᵢ` is evaluated at each sample point as extra tolerance
(built with the already-ported `derivative`). Constant expressions honour it
too. JS-oracle tests in tests/equality.rs (`3.14 = π` at 1%, exponent
exemption, absolute mode, …).

**Stage 0 — blanks guard**: unless `allow_blanks`, either side containing an
`Expr::Blank` node → false (a variant check, not a magic-symbol scan).

**Stage 1 — canonical structural check** (in `equals`):
Both sides go through `desugar_units` then the aggressive `canonicalize`
(≈ JS `evaluate_numbers` + name normalisations + `simplify`), then structural
equality on the canonical trees, with `coerce_seqs` for tuple/array/vector coercion.
Fast accept if identical. Note this is the *mathematical* accept path; it is more
permissive than JS's `equalsViaSyntax`, which we expose separately (below).

**`equals_syntactic` — the real `equalsViaSyntax` ✓ done** (2026-07,
`src/norm/syntactic.rs`). A faithful port of `lib/expression/equality/syntax.js`: a
*form* check, NOT the aggressive path. It applies only the four light passes —
`normalize_function_names` (incl. `sqrt`/`cbrt`/`nthroot`→powers, `e^x`→`exp`,
`f^(-1)`→`af`, `binom`→`nCr`), `normalize_applied_functions` (exponents/primes move
outside applications), `normalize_negative_numbers`, `normalize_angle_linesegment_arg_order`
— then compares trees *order-sensitively* (our derived `PartialEq` + `coerce_seqs`). It
does NOT flatten, reorder, fold, combine like terms, or eliminate `Div`, so `ln(x) =
log(x)` and `cos^(-1)(x) = arccos(x)` but `(x+y)+z ≠ z+x+y` and `3+2 ≠ 5`. This is the
"is the answer in the requested form?" primitive for grading. The parser preserving
operand order (it flattens `+`/`*` chains but does not reorder) is what makes an
order-sensitive compare on the faithful tree viable. Result: `symbolic_equivalences`
132/132 and `symbolic_nonequivalences` 200/200 — both categories now perfect.
Geometry arg-order sorting was also added to `canonicalize` (JS applies it in the full
chain too), fixing the `∠ABC=∠CBA` / `linesegment(A,B)=linesegment(B,A)` equivalences.
Corpus: 691 → 811/824.

**Stage 2 — finite field check** (`finite_field.rs`) — **NOT ported, and now known to
be a prerequisite for the log identities** (see below). Rejection only, and **skipped
when `allowed_error_in_numbers != 0`**. Assign random values in ℤ/pℤ to all variables
(transcendental subtrees keyed as opaque atoms, like the complex sampler), evaluate
both sides *exactly*; if they differ → definitively not equal. Resample on a mod-p pole.

**Stage 3 — complex sampling** (in `eq/mod.rs`, `equals_numerical`):
Currently a **strict** sampler: agree within tolerance at *every* pole-free point, reject
on the first disagreement. This is correct on its own but leaves *branch-cut identities*
(e.g. `log(a^2 b) = 2 log a + log b`, `x log y = log(y^x)`) unproven — they disagree at
complex points wherever the principal branches misalign.

**Finding (2026-07): the log identities need stage 2 first.** JS's stage 3 is *lenient* —
`find_equality_region` accepts if it finds one small neighborhood where both agree at
≥`MINIMUM_MATCHES` (10) clustered points (analyticity ⟹ identical), *tolerating*
branch-mismatch points up to `NUMBER_TRIES` (100). Two safety refinements beyond JS:
(a) a **constant** expression (no free vars) is compared directly, so genuine zeros like
`sin(pi)=0` are handled; (b) a sample point is *usable* only if finite, in bounds, AND
**nonzero** — after canonicalization a genuine zero function never reaches here, so an
exact `0.0` is underflow, and excluding it stops `x^sin(x)` vs `x^cos(x)` from being
accepted where both underflow. Scales `[10,1,100,0.1,1000,0.01]` are each tried
`NUMBER_TRIES` times; large scales first so a non-identity shows its global disagreement.

**Stage 2 — finite-field rejection ✓ done** (2026-07, `src/eq/finite_field.rs`). The exact
filter that makes stage-3 leniency safe. Both canonical trees are evaluated in ℤ/pℤ for the
9 JS primes (≡ 1 mod 4) with variables (and opaque atoms) bound to random field elements;
disjoint value multisets at any prime ⇒ definitely unequal. Exact modular arithmetic has no
magnitude, so `e^(10x)` vs `e^(10x)+C` is caught. The construction: `e` = a **primitive
root** `g`, `exp(x)=g^x`, and `sin`/`cos` use a 4th root of unity `i=g^((p-1)/4)`, so exp/
trig identities hold in the field; `log` is NaN (⇒ log identities are *not* rejected, left
to the sampler). Ported: `ZmodN`-style multivalued arithmetic (sqrt ⇒ two roots), Tonelli–
Shanks, primitive roots, `powerMod`, `eulerPhi`. Subtleties found and fixed: negative-int
exponents are reciprocals (`1/base^|k|`, so a zero base is a pole not `0`); high-precision
decimals (float approximations of π etc.) are skipped, mirroring JS's `approximate` flag,
so near-equal float coefficients aren't wrongly split; multivalued results reject only when
value sets are *disjoint*. Result: **0 false positives** across the corpus's 319
non-equivalence pairs.

Together stages 2+3 fix all 6 log-expansion identities plus `(-1)^n cos^n = (-cos)^n`.
Corpus: 816 → **823/824**. The single remaining failure was an `elementof`-set structural
case (nested `sin^2+cos^2` inside set membership), unrelated to numeric equality —
**closed 2026-07-17 → 824/824** by wiring `simplify` (§7e) into stage 1 and adding the
trig Pythagorean rule (see below).

**Stage 1 uses `simplify`, not just `canonicalize` ✓ done** (2026-07-17). The stage-1
structural compare now runs `simplify` (canonicalize + the §7e rewrite clusters, incl. the
trig Pythagorean identity) on both sides, matching JS's `evaluate_numbers` + name
normalization + `simplify` chain. This lets real-domain structural identities be decided
here instead of by sampling. The `sin²+cos²` inside `elementof` reduces to `1` on both
sides, so the trees become identical — the case sampling could never reach (an `elementof`
relation is not numerically evaluable). No equality-corpus regressions; the
known-failures snapshot is now **empty**. A subtlety found: `canonicalize` does *not* move a
function-head exponent outside its application (that lives in the syntactic normalizer), so
`sin^2(x)` stays `Apply(Pow(sin,2),[x])` while `sin(x)^2` is `Pow(Apply(sin,[x]),2)`; the
trig rule (`norm/simplify.rs::trig_square_base`) matches **both** spellings.

**Review finding (2026-07): unsimplified zero-functions vs `0` are a *simplify* gap, not a
sampler gap.** The sampler skips any sample where either side is exactly `0.0` (a variable
expression hitting exact zero is underflow — counting it, even one-sided via
`tolerance_for_zero`, accepts `x^sin(x)` = `x^cos(x)` and even `x^sin(x)` = `0`, both of
which underflow across whole regions). Consequence: `sin²x+cos²x−1 == 0` is unprovable
numerically. That matches JS: its sampler *also* rejects that pair (the float residue at
scale 10 exceeds `tolerance_for_zero`); JS accepts it in the **simplify stage** via the
Pythagorean rewrite — the same missing §7e rule behind the `elementof` corpus failure. So
both cases resolve together when trig simplify is ported. Constants (`sin(pi) == 0`) are
unaffected: a no-free-symbols pair is compared directly, where a genuine zero is fine.

**Branch-cut-free equality wins ✓ done** (2026-07), all via exact/deterministic routes
(no leniency, no false-positive risk): `nthroot(x) → sqrt(x)` in `canon_apply`;
subscripted log `log_b(x) = ln x / ln b` in `eval` (`head_evaluable`/`free_symbols` kept
consistent) → fixes `log_2(8)=3` and `log_a(b)=log(b)/log(a)`; and complex **gamma**
(Lanczos) for `factorial`, so the exact recurrence `Γ(n+2)=(n+1)Γ(n+1)` gives
`(n+1)·n! = (n+1)!` and `n/n! = 1/(n-1)!`. Corpus: 811 → 816/824. (The log-expansion
identities and `(-1)^n cos^n = (-cos)^n` were then unblocked by stage 2 — see below.)

**Relation dispatch ✓ done** (2026-07, in `src/eq/mod.rs`). Before stage 3, two
two-operand comparison relations (`=`, `<`, `≤`, plus `>`/`≥` folded by
canonicalization) are compared by their *standard forms* `lhs - rhs`: equal iff the
two differences are numerically **proportional** (JS `component_equals` with
`allow_proportional`). `=` accepts any nonzero constant factor; inequalities require a
positive real factor (a negative one reverses direction), so `5x+2y=3 ≡ 6-4y=10x` and
`5q-9z<2u+9z ≡ 27z-5q>-4u+5q-9z`, while `5q<9z ≢ 5q>9z`. The factor is pinned at the
first jointly-nonzero sample and verified at the rest. Deliberately **not** in
`equalsViaSyntax` (`equals_syntactic`): the same rearranged pairs stay syntactically
distinct, so a form-grading ("is the answer in the requested form?") check still
separates `5x+2y=3` from `6-4y=10x`. `≠` and set relations are excluded (matches JS).
Corpus: 685 → 691/824.

**Stage 4 — discrete infinite set ✓ done 2026-07-18** (`src/eq/discrete_infinite.rs`,
port of `lib/expression/equality/discrete_infinite_set.js` + `sets.js`):
- Sets are unions of arithmetic progressions, `OtherOp("discrete_infinite_set",
  [Seq(Tuple,[offset, period, min_index, max_index]),…])`; built by
  `create_discrete_infinite_set(offsets, periods, min, max)` (offsets/periods may
  be lists). Exported: `create_discrete_infinite_set`, `match_discrete_infinite`
  (partial-credit score 0..1, JS `match_partial` parity incl. tuple-level
  residue-fraction coverage).
- Containment = residue-class covering after normalizing by the candidate's
  period. Symbolic offsets work because only *ratios/differences* must fold to
  numbers, which canonical like-term/power combining does exactly
  (`(π/4)/π → ¼`, `(a+3)/3 − a/3 → 1`). Residue count capped at 10 000
  (JS unbounded) — conservative not-contained beyond.
- Also the listed-sequence form: `{0+7k, k≥0}` equals the *user-typable*
  `"0, 7, 14, 21, ..."` (list ending in ldots, ≥3 elements, integer min_index).
- **Chain position differs from JS deliberately**: dispatched BEFORE the
  finite-field/sampling stages (type-directed, like relations) — those stages
  treat the set's OtherOp as an opaque atom and would definitively reject pairs
  stage 4 accepts; JS's versions merely fail to conclude, so it can run last.
- Divergence (documented): JS needs an explicit `c ≠ 0` assumption to fold
  `2c/c`; our assumption-free canonicalizer folds it unconditionally (the
  `x/x → 1` class), so such pairs compare equal without assumptions.
- Tests: `tests/sets.rs` — all of `quick_sets.spec.js` except the assumptions
  case (basic/overcounting/partial scores ×13/symbolic offsets/list
  comparison/simplified sets).

The seeded RNG (deterministic for reproducible tests) uses `rand::SeedableRng` with a
fixed seed, same approach as `seedrandom` in the JS.

**TDD milestone**: `tests/equality.rs` — port `slow_math-expressions.spec.js` equivalence
pairs: `sin²x + cos²x = 1`, `(x+1)² = x²+2x+1`, `x²-1 = (x-1)(x+1)`, etc.

---

## 11. Assumptions (`src/assumptions/`) — ✓ core done 2026-07-18

`src/assumptions/mod.rs`. Design differs from the sketch below (kept for
reference): facts are stored as **canonical relation `Expr`s** per variable
(no separate `Assumption` enum — the relation tree is already the right
representation), and derivation happens at **query time** instead of storage
time (no `derived` map; `x = 3` answers `is_positive` by adopting the
literal's facts).

- **Storage**: `Assumptions { by_var: HashMap<String, Vec<Expr>> }` with
  `add` (canonicalizes, splits `And` conjuncts, files under every mentioned
  variable), `get` (facts re-conjoined, JS `get_assumptions` shape), `remove`,
  `clear`. NOT ported: generic assumptions (`add_generic_assumption`),
  `not_commutative`.
- **Queries** (the eight three-valued predicates of `element_of_sets.js`,
  `Option<bool>` = JS true/false/undefined): `is_integer`, `is_real`,
  `is_complex`, `is_nonzero`, `is_nonnegative`, `is_positive`, `is_negative`,
  `is_nonpositive`. Clean-slate bottom-up `Facts` inference over the canonical
  tree (~450 lines vs the JS's ~2100 of AST matching): literals exact; `pi`/`e`
  positive transcendentals; `i` complex-not-real; variables from their stored
  bounds (`x>c`, `∈ Z/Q/R/C`, `≠0`, `= literal`); closure rules for
  Add/Mul/Pow/abs/exp/sqrt/sin/cos/tan/log. JS conservatisms deliberately
  mirrored: unassumed variables fully unknown (no default-real), odd-power
  signs not inferred (`x³|x<0` → U), no interval arithmetic on sums
  (`x−3|x>4` → U), `x≠0` gives nonzero-ness without realness.
- **Validation**: `tests/fixtures/assumptions-corpus.json`
  (`scripts/generate-assumptions-corpus.mjs` — deterministic enumeration, 14
  assumption contexts × 39 expressions × 8 queries = 4368 verdicts from the JS
  oracle). Result: **0 definite T-vs-F conflicts** (hard-asserted:
  `no_definite_conflicts_with_js`) and **4363/4368 agree**; the 5 snapshotted
  divergences are all *our-definite/JS-unknown strengthenings that are
  mathematically correct* (real squares `x·x` ≥ 0; `log` of nonzero is
  complex). Hand tests in `tests/assumptions.rs`.
- **Assumption-aware simplification ✓ done 2026-07-18**:
  `simplify_with(e, &Assumptions)` threads the context through the rewrite
  rounds; the assumption cluster (active only when facts exist) resolves
  `sqrt` of even powers by the base's known sign — `sqrt(x²) → x` under
  `x ≥ 0`, `→ |x|` when merely real (JS parity confirmed by probe: `x < 0`
  yields `|x|`, NOT `−x`, and `abs` itself is never rewritten). Products and
  higher even powers included (`sqrt(x²y²) → xy`, `sqrt(x⁴) → x²`). Tests in
  `tests/doenet_utils.rs`.
- **Generic assumptions ✓ done 2026-07-18**: `add_generic`/`remove_generic` —
  patterns in the designated variable `x` applied to any variable without
  specific facts (specific facts win; a pattern mentioning the target variable
  as a different symbol is skipped, JS parity).
- **Finite-set membership ✓ done 2026-07-18** (§18/DoenetML #1504):
  `grade::evaluate_membership` — `3 ∈ {1,2,3}` → `Some(true)` (value-level via
  `equals`, so `2/2 ∈ {1}` holds), definitively false only for closed
  candidates/members, `None` for symbolic uncertainty; `∋`/`∌` orientations
  fold via canonicalization.
- **abs under assumptions ✓ done 2026-07-18 — deliberate divergence from JS**
  (user-mandated): `|u| → u` under `u ≥ 0`, `|u| → −u` under `u ≤ 0` (JS never
  rewrites `abs`); composed, `sqrt(x²) | x<0 → −x` where JS stops at `|x|`.
- Still open: `slow_assumptions` broader corpus.

<details><summary>original design sketch</summary>

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

</details>

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
- WASM bindings ✓ **done 2026-07-18** (`src/wasm.rs`, gated `#[cfg(target_arch =
  "wasm32")]`). An opaque `Expression` handle owns the parsed tree; only
  primitives/strings cross the boundary (no tree serialisation). Exposed:
  `parse_text` / `parse_latex` (→ `Result<Expression, JsError>`), and methods
  `to_text`, `to_latex`, `equals`, `simplify`, `expand`, `derivative(var)`,
  `variables()` (→ JS string array), `evaluate_to_constant()` (→ f64 / NaN).
  `wasm-bindgen` is a wasm32-only dependency, so native builds/tests are
  untouched; `crate-type = ["cdylib", "rlib"]` keeps the rlib for tests.
  Build + smoke test are one script: `scripts/build-wasm.sh` (cargo build
  --target wasm32 → `wasm-bindgen --target nodejs` → `pkg/` → runs
  `scripts/wasm-smoke.cjs`). Smoke test: **14/14** (round-trip, equals incl.
  `sin²+cos²==1`, simplify, expand, derivative incl. chain rule, variables,
  evaluate_to_constant, latex parse). `pkg/` is gitignored (build output).
  Follow-ups: `substitute`/`evaluate`-with-bindings and complex results need a
  map/richer boundary type (serde-wasm-bindgen); browser (vs nodejs) target.
- Differential test harness ✓ **done 2026-07-17** (`scripts/generate-fuzz-tests.mjs`
  + `tests/autogenerated_fuzz_tests.rs`, fixture `autogenerated-fuzz-tests.json`). A
  seeded grammar generates random text expressions; the JS library is the oracle.
  Cases are **pruned by structural signature** (operator/function makeup): a candidate
  is kept only if its signature is not already covered by the hand-written/corpus tests
  or another kept case. The existing corpora cover ~430 signatures; the novel space is
  1000+ and has yielded ZERO divergences, so the fixture keeps only a **bounded sample**
  (caps 60 parse / 40 equals) as coverage insurance rather than thousands of near-clones.
  Three checks, all green (no snapshots — any divergence is a real regression):
  - **parse fidelity** — `to_js(parse(input))` vs JS `.tree` (faithful layer, exact).
  - **equals agreement** — Rust `equals(a,b)` vs JS verdict.
  - **hang resistance** — see below.
  Teeth verified by planting a corrupted tree / flipped verdict (tests fail as expected).
  **Finding — JS `equals` hangs (infinite loop in its sampler) on some inputs; Rust does
  not.** Minimal case: `abs(-pi) == 4` (no large exponent). JS never returns; Rust
  answers `false` in ~0.3 ms. Three confirmed hangs are recorded in the fixture's
  `jsHangs` and asserted responsive by `does_not_hang_where_js_does`. The generator
  computes verdicts in a killed-on-timeout subprocess so a hang can't stall generation.
  This is a real robustness win for the port over the reference library.
  Still to do: the WASM bindings themselves (bullets above); optionally replay the
  existing fixture corpora through the same diff.

### Phase 8 — Assumptions and differentiation (week 10)
- Assumptions system — NOT started.
- Symbolic differentiation ✓ **done 2026-07-18** (`src/diff.rs`, `derivative(e, var)`).
  **Key finding:** the public `me.derivative(var)` does NOT use the hand-written
  `derivative_with_story` in `differentiation.js` (that is only the pedagogical
  step-by-step "story"); it delegates to **mathjs** `math.derivative`. So the port
  targets mathjs's *behaviour*: standard sum/product/quotient/power/chain rules plus a
  function-derivative table extracted verbatim from mathjs output (incl. hyperbolics and
  inverse-trig; the table entries are text templates in a placeholder, substituted with
  the argument — same shape as the JS). Result is `simplify`d. Correctness is checked
  against the JS oracle via `equals` (semantic), so tree shape need not match.
  - Tests: `tests/diff.rs` (hand cases per rule) + `tests/derivative_corpus.rs`
    (differential: random differentiable inputs, JS `.derivative('x')` oracle,
    `equals`-compared). **298/300**; the 2 snapshot failures are degenerate/limitation
    classes, not diff bugs: `x/(y−y)` (division by zero) and a zero-function that needs
    distributive expansion to reduce to exact `0` (the sampler can't confirm a
    zero-function — the documented `sin²+cos²−1==0` class).
  - **Simplify gaps found & fixed via the corpus** (both in the `pow` smart constructor,
    always-valid for integer outer exponents): nested-power flattening `(x^a)^k → x^(a·k)`
    (§7d, was never implemented) and power-of-product distribution `(a·b)^k → a^k·b^k`.
    These let removable singularities like `d/dx((y/x)·x)` reduce to 0, and bumped the
    simplify corpus 327 → 328 with no equality-corpus regression (still 824/824).
  - Not ported: the `derivative_with_story` explanation text; multi-arg / non-symbol
    heads (atan2, log-with-base) fall back to prime notation.
  - **Follow-up (cross-cutting) — re-evaluate string-literal usage; migrate built-in
    *names* to an enum.** Built-in function dispatch is currently scattered raw-string
    matching on `Sym::name()`: the evaluator's dispatch table (`eval/mod.rs`), the trig/
    root/name-normalisation passes (`norm/simplify.rs`, `norm/syntactic.rs`, `norm/mod.rs`
    — ~24 `name() == "…"` sites plus several `match name().as_str()` blocks), and
    `outer_derivative`'s table keys (`diff.rs`) all spell `"sin"`/`"cos"`/`"sqrt"`/… by
    hand, with alias handling (`arcsin`/`asin`, `ln`/`log`) repeated per site. Audit these
    and back the built-in-function identity with a single `BuiltinFn` enum resolved once (at
    parse or first-normalise), so the scattered matches become exhaustive `match`es the
    compiler checks and alias-collapsing lives in one place. Same play as the §6a token-
    string→enum conversion — do it once the corpora (equality/simplify/derivative) are the
    safety net, decide then whether `Apply` heads carry `BuiltinFn` or stay `Sym` with the
    enum as a resolved-on-lookup view. Scope note: this targets the *keys/names*. The
    `outer_derivative` RHS templates stay verbatim strings — they are whole expressions kept
    as the mathjs oracle (§15 Phase 8) — but their runtime-parse `.expect` (`diff.rs`) could
    move to a compile-time-checked build once the keys are an enum.

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
- Matrix operations beyond basic arithmetic — **now planned as a beyond-JS
  capability**: see `active-plans/DONE_MATRIX_PLAN.md` (2026-07-19; symbolic matrix algebra,
  det/rref, and abstract eigenvalues/eigenvectors via a `RootOf` construct).
  **Layer 1 (M1 + M2/§1b) implemented 2026-07-19**: canonical matrix
  arithmetic (entrywise sums, segmented non-commutative products with literal
  folding, binary powers, `A⁰ → I`, `A^(−k)` folding through the exact
  inverse for invertible rational matrices), `transpose`/`trace`/`matmul`,
  tiered `det` (rational elimination / polynomial Bareiss / symbolic
  cofactor), `matrix_inverse` (assumption-gated for symbolic entries),
  assumption-gated `rref`/`rank`/`nullspace` (`src/matrix.rs`), presentation
  guard for matrix bases; `tests/matrix.rs` (31 tests, TDD).
- Numerical integration (`integrateNumerically`) — future consumer of
  `active-plans/DONE_ARBITRARY_PERCISION_PLAN.md` §8 (quadrature hooks); **symbolic**
  integration is now planned as a beyond-JS capability, see
  `active-plans/INTEGRATION_PLAN.md` (2026-07-19; complete rational-function engine +
  curated Rubi-subset rules + hypergeometric terminal forms)

### 17a. Doenet-interop surface ✓ done 2026-07-19

The JS library re-exports its dependencies (`me.math` = customized mathjs
which also smuggles in `numeric` via `math.import`; plus internals
`me.class`/`me.utils`/`me.converters`/`me.reviver`), and Doenet consumes
them (measured against a DoenetML clone). Everything feasible outside the
other plans is now implemented:

- **`src/numeric.rs`** — f64 replacements for Doenet's `me.math` usage:
  `math_mod`/`gcd_f64`/`lcm_f64` (mathjs conventions), statistics
  (`mean`/`median`/unbiased `variance`/`std_dev`/`quantile_seq` linear
  interpolation), `lusolve` (partial-pivot elimination), and `eigs`
  (Householder→Hessenberg + complex shifted-QR with Wilkinson shifts +
  null-vector eigenvectors; mathjs result shape at the wasm boundary).
  Bounded loops, `None`/NaN failures, no panics.
- **`src/js_match.rs`** — `me.utils.match` in its default mode (the only
  mode Doenet uses) over raw JS-tree JSON: wildcard binding, associative
  flattening + grouping, repeated-wildcard consistency, the
  unary-minus-of-product case; plus `flatten`/`unflattenLeft/Right`.
- **`js_tree::try_from_js`** — non-panicking tree parsing for the wasm
  boundary (wasm aborts on panic).
- **`ops::round_numbers_to_precision_plus_decimals`** — the combined
  rounding Doenet calls (f64 params: JS passes `±Infinity` to disable modes).
- **wasm surface** — `from_ast`, `to_serialized`/`from_serialized` (the
  `{objectType: "math-expression", tree}` reviver shape), `match_template`,
  `flatten_ast`/`unflatten_left`/`unflatten_right`,
  `parse_text_with_options`/`parse_latex_with_options` (JS-spelled parser
  parameters), and the numeric bindings. Smoke: 39/39.
- **Oracle corpus**: `scripts/generate-numeric-corpus.mjs` →
  `tests/numeric_corpus.rs` (differential vs `me.math`, `me.utils.match`,
  and the JS rounding; 200 cases, all green) + `tests/numeric.rs` unit cases.

Deliberately NOT here: `dopri`/ODE solving → `active-plans/DONE_ODE_PLAN.md`; exact
matrix/eigen algebra → `active-plans/DONE_MATRIX_PLAN.md`; `me.ZmodN` (unused by Doenet);
mathjs `fraction`/`complex` object constructors (Doenet's uses are served by
`evaluate_to_complex` and the exact number tower).
- `solve_linear`
- Units system: the three scaling units (`%`, `deg`, `$`) are handled at equality
  time by `desugar_units` (§7b, done). A general `add_unit`/unit-algebra layer beyond
  these three is still out of scope.
- `±` (plus-minus) operator
- Piecewise functions
- ~~Discrete infinite sets~~ ✓ ported 2026-07-18 — see §10 stage 4
  (`src/eq/discrete_infinite.rs`)
- The JS method tail subsumed by the canonical layer and deliberately not
  given 1:1 ports: `clean`, `default_order`, `collapse_unary_minus`,
  `normalize_negative_numbers` (all inside `canonicalize`), `remove_scaling_units`
  (`desugar_units`), template `match`/`collect_like_terms_factors` (the §7e′
  redesign replaced pattern-matching simplification), `equalWithSignErrors`/
  `equalSpecifiedSignErrors`, `isAnalytic`, `set_small_zero`, `to_intervals`,
  `expand_relations`, `substitute_abs`, sub/superscript string conversions,
  vector/matrix method aliases (`dot_prod`, `cross_prod`, `vector_add`, …;
  componentwise tuple arithmetic lives in `simplify`'s seq cluster), and the
  mathjs passthroughs (`toXML`, `math`, `f`).

**DoenetML utilities ✓ done 2026-07-18** (`src/ops.rs`, JS-probed semantics):
`get_component`/`substitute_component` (0-based sequence indexing),
`subscripts_to_strings`/`strings_to_subscripts` (`x_1` ↔ flat symbol, numeric
suffix restored as a number), `to_intervals` (2-element `(a,b)`/`[a,b]` →
open/closed `Interval`, recursing through unions; other shapes untouched).
Fuzzy grading `allowed_error_in_numbers` is done under §10.

**Grading helpers ✓ done 2026-07-18** (`src/grade.rs`, JS-oracle tests in
tests/grade.rs): `equal_specified_sign_errors`/`equal_with_sign_errors`
(single-position subtree negations, recursive for n > 1; returns the smallest
error count); `solve_linear` (simplify under assumptions → expanded
lhs−rhs=0 → linear-coefficient extraction on the canonical sum — no template
matcher needed — with provably-nonzero coefficient required, inequality
direction flipping for provably-negative coefficients, `None` when the sign is
unknown; handles symbolic coefficients under `a ≠ 0`); and
`evaluate_membership` (§18).

**Public-surface status 2026-07-18**: everything else on the JS `me.*` /
expression-method surface is ported: parse (text/latex), to_text/to_latex,
equals (+syntactic, +discrete, +partial-credit sets), simplify, expand,
derivative, evaluate / evaluate_to_constant / evaluate_numbers, substitute,
variables, functions, operators, reduce_rational, rounding/constants
normalization, create_discrete_infinite_set, assumptions core (§11), all under
§7f limits, all reachable from JS via the wasm bindings (§13).

These are phase 2 additions once the core is solid.

---

## 18. Upstream behaviour bugs (DoenetML issues) this port should address

Surveyed the open [Doenet/DoenetML issues](https://github.com/Doenet/DoenetML/issues)
(2026-07) for ones whose root cause is `math-expressions` behaviour (parsing,
equality, simplification, evaluation, solving) rather than rendering/components. Each is
recorded here with the diagnosed root cause and the fix, cross-referenced to the section
that owns it. The clean-slate port is the natural place to fix most of these correctly
rather than patch the JS.

### In scope — actionable within the current port

| Issue | Symptom | Owning §  | Fix direction |
|---|---|---|---|
| [#1504](https://github.com/Doenet/DoenetML/issues/1504) | `containselement`/`∋` always `false` in a `<boolean>` (parses fine in `<math>`) | §11, §10 | Evaluate set membership to a truth value; reversed operators fold to `elementof` with operands swapped |
| [#372](https://github.com/Doenet/DoenetML/issues/372) | symbolic equality + `allowedErrorInNumbers` rejects `0.3333` for `1/3` | §10 stage 1 | Under `allowed_error_in_numbers != 0`, collapse numeric-only subtrees to `Float` *before* the structural compare |
| [#797](https://github.com/Doenet/DoenetML/issues/797) | `matchesPattern` ignores the tuple≈vector≈interval convention `equals` honours | §10 (`coerce_seqs`), §5 (`SeqKind`) | Pattern matcher must run the same seq-kind coercion `equals` uses before comparing |
| [#187](https://github.com/Doenet/DoenetML/issues/187) | `matchBlanks` conflates a genuine blank (`/2`) with unparseable input | §5 (`Blank`), §10 (`allow_blanks`) | The dedicated `Blank` variant already distinguishes blank from invalid math — the enabling change for changing `matchBlanks`'s default/semantics |
| [#1337](https://github.com/Doenet/DoenetML/issues/1337) | round-off near function-domain endpoints | §3a | Exact-decimal literals remove the parse-time float error; residual endpoint clamping is component-side |

Detail:

- **#1504 — set membership must produce a boolean.** `<math>` parses `x elementof
  {x,y,z}`, `∈`, `containselement`, `∋` correctly, but `<boolean>` returns `false` for the
  reversed forms (`containselement`/`∋`). Root cause is truth-value *evaluation* of
  membership, not parsing. In the port, the `element_of_sets` logic (§11, ported from
  `lib/assumptions/element_of_sets.js` — cf. the in-progress `lib/assumptions/element_of_sets.ts`)
  must decide membership when the set is an explicit finite `Seq(Set, …)`, and
  `RelOp::In`/its reverse (`∋`/`containselement`) must canonicalise to a single oriented
  relation so both directions evaluate identically. Note the equality corpus already
  exercises `sin²+cos²` *inside* `elementof` (§10, closed 2026-07-17); extend that path to
  return a decided boolean rather than only structural equality.

- **#372 — tolerance must reach inside fractions.** Symbolic equality compares syntax
  trees, so `1/3` (a `Mul(1, Pow(3,−1))`) never structurally matches the number `0.3333`,
  yet `0.999/3.001` *is* accepted because it stays a division node. The port's stage-1
  compare (§10) already folds constants exactly; add: when `EqOptions.allowed_error_in_numbers
  != 0`, evaluate numeric-only subtrees to `Float` on both sides before comparing, so a bare
  decimal and an exact fraction reduce to comparable numbers within tolerance. Effectively
  `allowed_error_in_numbers` should imply numeric-leaf collapse (JS `simplifyOnCompare="numbers"`).
  Interaction to preserve: stage 2 (finite-field) is already skipped when
  `allowed_error_in_numbers != 0` (§10), so this only affects stages 1/3.

- **#797 — pattern matching must reuse the seq-kind coercion.** `equals` treats a bare
  `(5,4)` tuple as equal to a vector or open interval via `coerce_seqs` (§10; the JS
  `coerce_tuple_array_vectors` in `lib/expression/equality/coersion.js`). `matchesPattern`
  is a DoenetML component, but it relies on this library convention and currently ignores
  it. If/when a pattern-matching primitive is exposed from the port, it must thread the
  `coerce_tuples_arrays`/`coerce_vectors` flags through the match the same way `equals`
  does — the §5 `Seq(SeqKind, …)` design already makes this a kind-equivalence check rather
  than cross-variant special-casing, so the matcher and `equals` can share one routine.

- **#187 — blanks are a first-class variant here.** The JS ambiguity (is `matchBlanks`
  default sensible now that `/2` parses?) stems from JS conflating a blank placeholder with
  invalid math. The port's dedicated `Expr::Blank` variant (§5) and the `allow_blanks`
  variant check (§10 stage 0) already separate the two cleanly, which is the prerequisite
  for DoenetML to safely change `matchBlanks`'s default.

- **#1337 — exact decimals shrink the float-error surface.** Parsing all user decimals as
  exact rationals (§3a) removes the parse/fold-time rounding this issue suspects; whatever
  remains is domain-endpoint clamping in the `<function>`/`regionBetweenCurves` components,
  not the expression library.

### Out of scope for now — record the fix for when the feature is ported

These map to features listed in §17 as not-yet-ported; noting the fix direction so it is
not re-diagnosed later.

- **#163 & #940 — spurious roots/minima at vertical asymptotes** (`<solveEquations>`
  returns bogus solutions of `tan(x)=1` at poles; the mathjs-derivative extrema code finds
  a spurious minimum near an asymptote). Solving/extrema is out of scope (§17, `solve_linear`
  / numerical integration). Fix when ported: after bracketing a sign change, reject the
  candidate unless the function value there is finite and near zero — i.e. distinguish a
  genuine root from a `+∞ → −∞` pole crossing. The same pole guard removes #940's spurious
  extremum.

- **#709 — 4×4 eigendecomposition silently fails with a large entry** (works at `10`, fails
  at `100` — a conditioning/robustness problem in the `numeric` library, with no error
  surfaced). Matrix operations beyond basic arithmetic are out of scope (§17). Fix when
  ported: use a better-conditioned eigensolver that emits an error on non-convergence
  instead of returning an empty result (cf. upstream #1415, migrating `eigs`/`lusolve` off
  `numeric`).

### Mostly DoenetML-side — library provides only the parse

- **#1000 — function domain union syntax** (`domain="(-4,-1) union (1,4)"` falls back to the
  full real line). The port's parser already produces `Union` nodes (§5/§6), so the library
  side is covered; the fallback is in DoenetML's `IntervalList.js`/`Function.js`, which
  reject multi-interval domains. Library obligation: ensure a union of intervals parses and
  round-trips cleanly so the component can consume it.

- **#1300** (`\)` prematurely ends an `<m>`) and **#1263** (document the math operators) are
  MathJax-delimiter and documentation issues respectively, not `math-expressions` behaviour;
  listed only to mark them surveyed-and-excluded.
