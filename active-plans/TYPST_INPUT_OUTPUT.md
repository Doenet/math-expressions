# Typst input/output plan

Add typst as a third syntax flavor alongside `text` and `latex`: parse typst
math into `Expr`, and render `Expr` back to typst math markup.

The typst *plugin* (a WASM module that receives equation content from a typst
document) lives in a **separate repository**. This plan covers only the parser
and formatter that live in `math-expressions-rs`. The one thing the plugin
constrains here: the plugin will call typst's `repr()` on an equation and hand
us the resulting **string**, so parsing `repr()` output is a first-class input
surface, not an afterthought.

## Goals

1. **Parse `repr()` output** — the structured element tree typst produces for an
   equation (e.g. `repr($mat(1,2;3,4)$)`). This is the primary path, since it is
   how the external plugin serializes an equation across the WASM boundary.
2. **Parse raw typst math markup** — the source syntax a human writes
   (`mat(1,2;3,4)`, `x^2 + 1`, `a/b`, `sin(x)`, `alpha`).
3. **Render `Expr` → typst markup** — a precedence pretty-printer, mirroring the
   existing `text`/`latex` formatters.

Non-goals: the typst-plugin WASM ABI/packaging (separate repo); rendering to
`repr()` form (only markup output is needed); typst *markup/code* mode outside
`$…$` math.

## Where this fits in the existing architecture

The library already has a per-syntax parse + output design over one central AST,
so typst is additive — **no changes to `Expr` or the math engine**.

- Central AST: `src/expr.rs` (`Expr`). Faithful-layer variants we reuse:
  `Div`, `Neg`, `Pow`, `Index`, `Matrix`, `Seq(Vector/Set/List/Tuple)`,
  `Apply`, `Relation`, `OtherOp` (escape hatch for `binom`, `vec`, `pm`, …).
- Parsers: `src/parse/{latex,text}.rs`, sharing `common.rs`, `lexer.rs`
  (`Flavor { Text, Latex }`), `error.rs`.
- Formatters: `src/output/{latex,text}.rs`, sharing the precedence ladder and
  greek/unicode table in `src/output/mod.rs`.
- JS surface: `src/wasm.rs` (`parse_text`/`parse_latex`, `.to_text()`/`.to_latex()`).
- Correctness oracle: `tests/roundtrip.rs` — parse → format → re-parse must be
  structurally equal; no hand-authored expected output.

New files (see manifest at the end): `src/parse/typst/{mod,repr,markup,lower}.rs`,
`src/output/typst.rs`, plus wiring in `parse/mod.rs`, `output/mod.rs`, `lib.rs`,
`wasm.rs`, and fixtures/tests.

## The `repr()` format

`repr($…$)` is **nested typst value syntax** describing an element tree — not
markup. Ground-truth output captured from typst 0.13.1:

```
$mat(1,2;3,4)$  → equation(block: false, body: mat(rows: (([1],[2]), ([3],[4]))))
$x^2 + 1$       → sequence(attach(base: [x], t: [2]), [ ], [+], [ ], [1])
$a/b$           → frac(num: [a], denom: [b])
$1/2 x^2$       → sequence(frac(num: [1], denom: [2]), [ ], attach(base: [x], t: [2]))
$sin(x)$        → sequence(op(text: [sin], limits: false), lr(body: sequence([(],[x],[)])))
$f(x) = x^2$    → sequence(sequence([f], lr(body: sequence([(],[x],[)]))), [ ], [=], [ ], attach(base: [x], t: [2]))
$x_i^2$         → attach(base: [x], t: [2], b: [i])
$R_1$           → attach(base: [R], b: [1])
$sqrt(x+1)$     → root(radicand: sequence([x], [+], [1]))
$root(3, x)$    → root(index: [3], radicand: [x])
$vec(1,2,3)$    → vec(children: ([1],[2],[3]))
$binom(n,k)$    → binom(upper: [n], lower: ([k],))
$abs(x)$        → lr(body: sequence([|],[x],[|]))
$floor(x)$      → lr(body: sequence([⌊],[x],[⌋]))
$sum_(k=0)^n k$ → sequence(attach(base: [∑], t: [n], b: sequence([k],[=],[0])), [ ], [k])
$-x$            → sequence([−], [x])
$x - y$         → sequence([x], [ ], [−], [ ], [y])
$x <= 5$        → sequence([x], [ ], [≤], [ ], [5])
$x != y$        → sequence([x], [ ], [≠], [ ], [y])
$x plus.minus y$→ sequence([x], [ ], [±], [ ], [y])
$3.14 dot pi$   → sequence([3.14], [ ], [⋅], [ ], [π])
$pi "radius"^2$ → sequence([π], [ ], [⋅], [ ], attach(base: [radius], t: [2]))
```

(The full `equation(block: false, body: …)` wrapper is elided above for all but
the first two.)

Facts that drive the design:

1. **Value syntax**: nested function calls with positional + named args, arrays
   `((…),(…))`, string literals `"["`, and content-atom literals `[x]`, `[3.14]`,
   `[+]`, `[ ]`.
2. **Element vocabulary**: `equation, sequence, attach(base/t/b), frac(num/denom),
   mat(rows,delim), vec(children), root(radicand,index), lr(body),
   op(text,limits), binom(upper,lower), cases(children), limits(body),
   align-point`.
3. **Operators are Unicode content atoms**: `−` (U+2212, *not* ASCII `-`), `≠`,
   `≥`, `≤`, `±`, `⋅`, `→`, `∞`, `∑`, `⌊⌋`. A normalization table is required
   (partly already present in `output/mod.rs::greek_unicode` and the lexer).
4. **`sequence` is flat juxtaposition**: recovering `x - y`, or implicit-multiply
   `2 x` from `sequence([2],[ ],[x])`, needs a **precedence parse over the atom
   stream**. This is the hard part — and it is identical whether the tree came
   from `repr()` or from raw markup.
5. `repr()` is explicitly documented as **unstable across typst versions**. Pin a
   supported version and guard with differential tests against the pinned binary.

## Design: one shared lowering, two front-ends

Raw markup and `repr()` are two surface syntaxes for **the same typst element
tree**. Model that tree once and lower it once.

```
parse/typst/repr.rs     repr string        → ContentTree   (value-syntax parser)
parse/typst/markup.rs   raw "mat(1,2;3,4)" → ContentTree   (math-markup parser)
parse/typst/lower.rs    ContentTree        → Expr          (SHARED — the core)
output/typst.rs         Expr               → raw typst markup
```

- **`ContentTree`** mirrors typst's element model:
  ```
  enum Content {
      Seq(Vec<Content>),
      Text(String),                                   // [x], [3.14], [+], [ ]
      Attach { base, top: Option, bottom: Option },   // ^ / _
      Frac { num, denom },
      Root { radicand, index: Option },
      Mat { rows: Vec<Vec<Content>>, delim: Option<(String,String)> },
      Vec_ { children: Vec<Content>, delim: Option },
      Lr(Box<Content>),                               // delimiter group
      Op { text: String },                            // named operator (sin, …)
      Binom { upper, lower: Vec<Content> },
      Cases { rows: Vec<Content> },
      AlignPoint,
      // + Space as a Text(" ") or a dedicated variant
  }
  ```
- **`lower.rs`** (shared core) does, in order:
  1. Map Unicode operator atoms → operator tokens (normalization table).
  2. **Precedence-parse each `Seq`** (Pratt) over the atom/node stream:
     relations, `+`/`−`, implicit + explicit (`⋅`) multiplication, unary sign.
  3. Structural nodes: `Attach`→`Pow`/`Index` (both present → `Pow(Index(...))`
     matching typst layout), `Frac`→`Div`, `Root`→`Apply(sqrt)` or nth-root,
     `Mat`→`Matrix`, `Vec_`→`Seq(Vector)`, `Binom`→`OtherOp("binom")`,
     `Cases`→`OtherOp`/`Relation` set, `Lr`→grouping vs `abs`/`floor`/`ceil`
     (decided by the group's first/last delimiter atom).
  4. **Function application**: a name/`Op` immediately followed by an `Lr` paren
     group → `Apply` (reuse the `functions.rs` applied-name registry, as the
     latex parser does).
- **`output/typst.rs`** is the easiest piece — a precedence pretty-printer
  modeled on `output/text.rs` (~500 lines), reusing the ladder in
  `output/mod.rs`. Emits `frac(a, b)`, `x^(…)`, `x_(…)`, `mat(…;…)`, `sqrt(…)`,
  `sin(…)`, greek names (`alpha`, `pi`), relations (`<=`, `!=`), `dot`.

Why share the lowering: the precedence parse in step 2 is the only real
algorithmic risk, and doing it once means the raw and repr paths cannot diverge
in how they interpret `2x`, `x - y`, or `f(x)`.

## `Expr` mapping reference

| typst element / markup            | `Expr`                                        |
|-----------------------------------|-----------------------------------------------|
| `[3.14]`, number atom             | `Num`                                         |
| `[x]`, `[radius]`, `"str"`        | `Sym`                                         |
| `[π]`/`pi`, `[α]`/`alpha`         | `Sym("pi")`, `Sym("alpha")` (names, not glyphs)|
| `[∞]`/`oo`                        | `Const(Inf)`                                  |
| `attach(base, t)` / `x^2`         | `Pow`                                         |
| `attach(base, b)` / `x_i`         | `Index`                                       |
| `attach(base, t, b)`              | `Pow(Index(base, b), t)`                      |
| `frac(num, denom)` / `a/b`        | `Div`                                         |
| `[−] x` (leading)                 | `Neg`                                         |
| `a [+] b`, `a [−] b`              | `Add` / `Add` with `Neg`                      |
| juxtaposition `2 x`, `a [⋅] b`    | `Mul`                                         |
| `op(text: sin)` + `lr(( x ))`     | `Apply(sin, [x])`                             |
| `[f]` + `lr(( x ))`               | `Apply(f, [x])`                               |
| `root(radicand)`                  | `Apply(sqrt, [·])`                            |
| `root(index: n, radicand)`        | nth root (`Pow(·, 1/n)` or `Apply` — decide)  |
| `mat(rows: …)`                    | `Matrix`                                       |
| `vec(children: …)`                | `Seq(Vector, …)`                              |
| `binom(upper, lower)`             | `OtherOp("binom", …)`                          |
| `lr(( … ))`                       | grouping (transparent)                        |
| `lr(\| … \|)`                     | `Apply(abs, …)`                               |
| `lr(⌊ … ⌋)` / `lr(⌈ … ⌉)`         | `Apply(floor/ceil, …)`                        |
| `[=] [≠] [<] [≤] [>] [≥]`         | `Relation` (chained)                          |
| `[±]` / `plus.minus`             | `OtherOp("pm", …)`                            |
| `integral … dif x` (`[∫]` + `dif`)| `Apply(Sym("int"), [integrand])`; bounds via `Index`/`Pow`; `dif x`→`OtherOp("d",[x])` |
| `sum`/`product`/`lim` (big ops)   | **no existing target — see risks**            |
| `h(…)`, `styled(child: …, ..)`    | styling/spacing wrappers — **skip/unwrap**    |

## Implementation phases

Each phase is independently landable and testable.

**Phase 0 — scaffolding.** Add `src/parse/typst/mod.rs` and the `Content` enum
in `lower.rs`. Wire empty modules into `parse/mod.rs`, `output/mod.rs`, `lib.rs`.

**Phase 1 — `output/typst.rs` first.** Self-contained; produces the roundtrip
harness the parsers will be developed against. Model on `output/text.rs`. Add
`to_typst` / `TypstOpts` re-exports and a `.to_typst()` method in `wasm.rs`.

**Phase 2 — `lower.rs` (shared core).** Unicode-op table, `Seq` precedence parse,
structural-node mapping, function-application detection. Unit-tested by
hand-building `Content` trees (no parser needed yet).

**Phase 3 — `repr.rs` (primary path).** Value-syntax parser: nested calls, named
args, arrays, string + content-atom literals → `Content`. Must **skip styling
wrappers** (`h(…)`, `styled(child: …, ..)`) and tolerate `..` elided fields —
these appear around e.g. the `dif` differential. Then `parse_typst_repr(s) ->
Expr` via `lower`. This unblocks the external plugin.

**Phase 4 — `markup.rs` (raw path).** Math-markup grammar → `Content`:
`^`/`_` (with `(...)` groups), `/` = fraction, function calls `mat`/`vec`/`frac`/
`sqrt`/`root`/`abs`/`floor`, `;` row separators, named args (`delim: "["`),
symbol names (`alpha`, `pi`, `oo`), dotted symbols (`plus.minus`), `dot`, strings.
Then `parse_typst(s) -> Expr` via `lower`.

**Phase 5 — polish.** `integral` lowering (greedy integrand + `dif`
normalization), `sum`/`prod`/`lim` decision, `cases`, error messages, options
(allowed symbols, function symbols) matching `LatexToAstOptions` shape.

An MVP that delivers the plugin's primary need is **Phases 0–3** (repr in,
markup out) — skip Phase 4 initially.

## Testing strategy

- **Roundtrip** (`tests/roundtrip.rs`): extend with typst — for every fixture
  `Expr`, `to_typst` then `parse_typst` (markup) must be structurally equal.
- **Differential vs typst binary**: a fixture generator drives the real `typst`
  CLI to emit `repr()` for a corpus of equations (pin the version), and asserts
  `parse_typst_repr(repr) == parse_typst(raw_markup)` — i.e. both front-ends
  lower identically. The generator script + a checked-in JSON fixture keep CI
  offline. (A typst 0.13.1 binary was already used to capture the examples above.)
- **Unit tests** for `lower.rs` on hand-built `Content` trees (sign handling,
  implicit multiply, `attach` sup+sub, `lr` disambiguation, application).
- **Fixtures**: add `tests/fixtures/typst-repr.json` and `typst-markup.json`
  mirroring the existing `text-to-ast.json` / `latex-to-ast.json` shape.

## Open decisions / risks

- **`integral` — reuse the existing convention.** math-expressions has **no
  integral node**; the parsers represent one as `Apply(Sym("int"), [integrand])`
  with bounds on the head (`∫_a^b` → `Pow(Index(Sym("int"), a), b)`) and the
  differential folded into the integrand as `OtherOp("d", [x])` (extracted from a
  `Mul` when a `Sym("d")` factor precedes another factor). Lower typst's integral
  onto exactly this — no new AST. Two lowering wrinkles:
  - **∫ is a loose symbol in a flat `sequence`, not a bracket**: nothing marks
    where the integrand ends. Replay the parser's greedy rule — on hitting `[∫]`,
    lower the rest of the sequence as the integrand. Bounds line up directly:
    `attach(base: [∫], t: [b], b: [a])` → `Pow(Index(Sym("int"), a), b)`.
  - **the differential reprs as wrapper noise**: `dif x` →
    `sequence(h(amount: …, weak: true), styled(child: [d], ..))` — a spacing
    `h(...)` plus a `styled`-wrapped `[d]` with `..` elided fields. The repr value
    parser must **skip `h(…)`/`styled(…)` and tolerate `..`** everywhere (general
    styling artifacts, not integral-specific); then normalize the recovered `d`
    (from `dif` or a literal `[d]`) to `Sym("d")` so the existing
    integrand/differential extraction produces `OtherOp("d", [x])`.
  - **inert notation, not computation**: `Apply(Sym("int"), …)` is a faithful
    notation node; symbolic integration is the separate `integrate(expr, var)`
    API, not triggered by simplifying an `int` node. So this is round-trippable
    notation only — consistent with existing text/latex behavior.
  - Edge cases: bare `$integral$` → `Apply(Sym("int"), [Blank])`; iterated
    `integral integral … dif x dif y` — greedy-integrand + per-`dif` pairing.
- **`sum` / `product` / `lim` — no existing target.** Unlike `integral`, these
  are **not represented anywhere** in math-expressions (Rust *or* the JS
  reference): no token, no node, and a bare `\sum` isn't even in
  `allowed_latex_symbols`, so latex rejects it. Typst reprs them identically to
  ∫ (`sum_(k=0)^n` → `attach(base: [∑], t: [n], b: sequence([k],[=],[0]))`), so
  lowering them is mechanically trivial — the open question is *what to lower to*.
  Options: (a) **invent** an int-style convention `Apply(Sym("sum"|"prod"), …)`
  with bounds on the head — round-trippable notation only, understood by nothing
  downstream (simplify/evaluate/diff); or (b) **reject** as unsupported in v1
  (clear parse error). Recommendation: (b) for v1, revisit if a real consumer
  needs them. The rest of the design does not depend on this choice.
  - This makes typst consistent with latex: unrecognized symbols like
    `sum`/`product` coming from typst should produce a parse error, exactly as
    `\sum`/`\prod` do today from latex (they are absent from
    `allowed_latex_symbols`, so they lex as an unknown `LATEXCOMMAND` and the
    parser rejects them). The typst front-ends should reuse the same
    unrecognized-symbol error path rather than silently dropping or guessing.
- **nth root** representation: `Pow(x, 1/n)` (canonical-friendly) vs a faithful
  `Apply`. Follow whatever the latex parser does for `\sqrt[n]{}`.
- **`repr()` instability**: pin the supported typst version range; the
  differential fixtures are the guard when bumping it.
- **Content vs string ambiguity**: `[radius]` (a variable) and `$"radius"$` (a
  string) are indistinguishable in repr — both lower to `Sym`, which is fine.
- **Delimiter-group semantics**: `lr` disambiguation relies on the first/last
  atom being a known delimiter glyph; unknown delimiters fall back to grouping.

## File manifest

```
src/parse/typst/mod.rs      pub use; TypstToAst{,Repr} entry points + options
src/parse/typst/lower.rs    Content enum + Content → Expr (shared core)
src/parse/typst/repr.rs     repr string → Content
src/parse/typst/markup.rs   raw typst math → Content
src/output/typst.rs         Expr → typst markup (+ TypstOpts)
src/parse/mod.rs            + pub mod typst;
src/output/mod.rs           + pub mod typst;  + to_typst / TypstOpts re-export
src/lib.rs                  + re-exports (to_typst, TypstToAst, …)
src/wasm.rs                 + parse_typst, parse_typst_repr, .to_typst()
tests/roundtrip.rs          + typst roundtrip
tests/typst_parse.rs        repr + markup parse fixtures
tests/fixtures/typst-*.json fixtures (differential-generated)
scripts/gen-typst-fixtures  drives the pinned typst CLI to build fixtures
```

## Rough effort

| Piece                                   | Effort     |
|-----------------------------------------|------------|
| `output/typst.rs` (Phase 1)             | 1–2 days   |
| `ContentTree` + `lower.rs` (Phase 2)    | 3–5 days   |
| `repr.rs` (Phase 3)                     | 2–3 days   |
| `markup.rs` (Phase 4)                   | 3–5 days   |
| tests + differential fixtures           | 2–3 days   |

MVP (Phases 0–3: repr in, markup out) ≈ **1 week**; full coverage ≈ **2–3 weeks**.
