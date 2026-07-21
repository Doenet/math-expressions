# Calculus Limits Plan (`lim`)

> **PROGRESS (audited 2026-07-20):** NOT STARTED — greenfield design, zero
> implementation (`WHATS_LEFT.md` §B.4, items 35–40, P0–P5). No JS reference,
> so no differential corpus. Do not confuse with the shipped
> `src/resource_limits.rs` governor (unrelated to calculus limits).

Draft: 2026-07-20. Adds `lim_{x→a} f(x)` to `math-expressions-rs`.

Status: **design only — nothing implemented.** This is a greenfield
subsystem. Unlike matrices/integration, there is **no JS reference** in
upstream math-expressions, so there is no differential corpus to match and
we are free to design the representation and output — the only external
constraint is that notation should round-trip and read conventionally.

Companion note: the resource-governor module was renamed `limits.rs →
resource_limits.rs` (`Limits → ResourceLimits`) precisely so `limit`/`lim`
here is unambiguous. New caps for this engine are added to `ResourceLimits`
(§8), not to a private constant.

---

## 1. Scope

In scope (phased, §9):
- **Finite two-sided limits** `lim_{x→a} f(x)` for real `a`.
- **One-sided limits** `lim_{x→a^+}` / `lim_{x→a^-}`.
- **Limits at infinity** `lim_{x→∞}` / `lim_{x→-∞}`.
- **Infinite results** (`+∞`, `-∞`) and an explicit **does-not-exist** verdict
  distinct from "could not decide".
- Text + LaTeX parsing and printing of `lim` notation.
- A numeric **verification gate** (mirrors integration's diff-gate) so a
  symbolic verdict is only returned when independent high-precision sampling
  agrees.

Explicit non-goals (at least initially):
- Multivariate / iterated / path limits (`lim_{(x,y)→(0,0)}`).
- Limits of sequences with number-theoretic subtlety, `limsup`/`liminf`.
- Symbolic limits in a *parameter* (`lim_{x→0} sin(a x)/x` returning `a` is
  fine; classifying by cases on `a`'s sign is a stretch goal, §6.4).
- Complex-variable limits (the engine is real-line only; the sampler may use
  the existing complex evaluator internally but the point/direction are real).

---

## 2. Representation

`lim` carries structured metadata (variable, approach point, direction), so
it is **not** a good fit for the stringly `OtherOp` tail. Two options:

**Option A (recommended): a dedicated `Expr::Limit` variant.**
```rust
Expr::Limit {
    var: Sym,               // the bound variable
    point: Box<Expr>,       // approach target: a constant, or ±Inf via MathConst
    dir: LimitDir,          // TwoSided | FromAbove | FromBelow
    body: Box<Expr>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum LimitDir { TwoSided, FromAbove, FromBelow }
```
- Pros: type-safe, self-documenting, `var` is bound (shadowing/`variables()`
  must exclude it — see §5), impossible to malform.
- Cons: a new variant touches the ~10 blessed match sites (`children`,
  `map_children`, `canonicalize`, `order::cmp`, structural-eq, both printers,
  `js_tree` to/from, `diff`, `eval` opaque handling). The compiler enforces
  each, so this is mechanical, not risky — and is exactly the "one variant,
  many arms" cost the improvement plan already accounts for.

**Option B: `OtherOp("lim", [var, point, dir_flag, body])`.**
- Pros: zero new variants; parsers/printers already have an `OtherOp` path.
- Cons: `var` is not recognized as bound (breaks `variables()`, substitution,
  and any pass that treats every `Sym` as free); direction encoded as a magic
  operand; every algorithm must re-validate arity/shape. This fights the
  maintainability priority.

**Decision: Option A.** The binding semantics of `var` are the deciding
factor — a limit is a *binder*, like an integral's `dx`, and the codebase has
no binder today, so modeling it as free-symbol soup would leak bugs into
unrelated passes. Treat this variant addition as the first real binder and
document it as the template for a future `∫…dx` / `Σ` binder if those ever go
symbolic.

`point` reuses `MathConst::Inf` / `MathConst::NegInf` for limits at infinity,
so no separate "at infinity" flag is needed; `dir` is meaningless (forced
`TwoSided`) when `point` is infinite (only one side exists) — canonicalization
normalizes it (§4).

---

## 3. Parsing

### 3.1 Text (`src/parse/text.rs`)
Grammar addition (a prefix operator, like the existing applied-function and
`derivative_leibniz` handling):
```
lim_{x->a} body
lim_{x->a^+} body        lim_{x->a-} body      (both spellings)
lim_{x->inf} body
limit(body, x, a)        limit(body, x, a, "+")   (function-call spelling)
```
- Add `lim` / `limit` as recognized heads. `lim` uses subscript-arrow
  notation; `limit(...)` is the applied-function fallback so tooling can
  build it without notation.
- The `->` / `→` arrow token already exists for relations; reuse it inside the
  subscript group. `^+`/`^-` (or trailing `+`/`-`) sets `dir`.
- Approach point parses as a normal expression, then must reduce to a
  constant or ±∞ at canonicalization (§4); otherwise the whole thing is an
  honest parse of an `OtherOp("lim", …)`-shaped error node? No — keep it a
  `Limit` with a non-constant `point` and let the engine decline. Parser stays
  permissive; the engine is the judge.

### 3.2 LaTeX (`src/parse/latex.rs`)
```
\lim_{x \to a} body
\lim_{x \to a^+} body
\lim_{x \to \infty} body
```
- `\lim` becomes a known control word (add to the notation table — this is
  the same Phase 2 shared-notation work the improvement plan describes; until
  then, a local arm).
- Subscript body `x \to a` parses as a 2-operand relation with `RelOp` arrow;
  the parser destructures it into `(var, point)`. A superscript `+`/`-` on the
  point sets `dir`.

### 3.3 Registry / notation touchpoints
`lim` is notation, not an applied math function, so it does **not** get a
`FnDef` in `crate::functions`. It is handled in the parser grammar directly
(like `\frac`, `derivative_leibniz`). Keep this boundary explicit in the
`functions/mod.rs` "what stays outside the registry" note.

---

## 4. Canonicalization (`src/norm/mod.rs`)

`canonicalize` gets a `Expr::Limit` arm:
- Canonicalize `point` and `body`.
- If `point` canonicalizes to `+∞`/`-∞`, force `dir = TwoSided` (single-sided
  by nature).
- Do **not** canonicalize *through* the binder in a way that captures `var`:
  `body` is canonicalized in the ordinary way (the bound `var` is just a
  symbol to the sub-pass), which is safe because canonicalization never
  invents or substitutes free symbols.
- `order::cmp` needs a total order for `Limit` (rank it among the notation
  nodes; compare by `(var, point, dir, body)`).
- Structural equality falls out of the derived/`cmp` machinery.

Canonicalization does **not** evaluate the limit — that is the engine's job
(§6), invoked only by an explicit `.limit()` call, exactly as `integrate` is
separate from `canonicalize`.

---

## 5. Binding semantics (the cross-cutting change)

`Limit.var` is bound. Audit every pass that enumerates symbols:
- `ops::variables` / `free_symbols` (in `eval`): must **remove** `var` from
  the free set of the `body` (add it back only if it also appears free in
  `point`, which would be unusual/ill-formed).
- `ops::substitute`: must not substitute `var` inside the body (shadowing).
- `diff::derivative`: `d/dy lim_{x→a} f(x,y)` differentiates the body w.r.t.
  `y ≠ x`; `d/dx` of a limit binding `x` treats it as constant → 0.
- `eval` (`eval_complex`): a `Limit` with unresolved value is an **opaque
  atom** (like an unknown function) — it samples by structure so `equals`
  still works syntactically. Once the engine can resolve it, callers simplify
  first, then evaluate.

This binder audit is small but must be exhaustive; a checklist test (§9 P1)
enumerates the passes.

---

## 6. The evaluation engine (`src/limit/`)

Module layout (mirrors `integrate/`):
```
src/limit/
  mod.rs        // public `limit()`, the staged pipeline, the verify gate
  algebraic.rs  // factor/cancel, rationalize, common-denominator
  lhopital.rs   // indeterminate-form detection + L'Hôpital recursion
  known.rs      // table of standard limits & asymptotic rules
  numeric.rs    // certified one-sided numeric probe (wraps precise::)
```

Public entry (mirrors `integrate`'s signature and honesty contract):
```rust
pub fn limit(body: &Expr, var: &str, point: &Expr, dir: LimitDir,
             a: &Assumptions) -> LimitResult;

pub enum LimitResult {
    Value(Expr),        // a finite (possibly symbolic) limit
    Infinite(Sign),     // +∞ or -∞
    DoesNotExist,       // proven to not exist (e.g. sided limits disagree)
    Unknown,            // engine declined within budget — honest, not a guess
}
```

### 6.1 Staged pipeline (first hit wins; all share one fuel budget)
Order matters — cheap/exact before expensive/heuristic, like integration §2:

0. **Preprocess.** `simplify` the body; `reduce_rational`; if `point` is
   finite, also try `factor`. Reduces most removable discontinuities away.
1. **Direct substitution (continuity).** Substitute `var := point` and
   `ops::evaluate_to_constant` / exact fold. If it yields a finite defined
   value and the body is continuous there (no division by zero, no boundary
   of a domain-restricted function per the `FnDef` domain guard), return it.
   This resolves the large majority of classroom limits.
2. **Algebraic (`algebraic.rs`).** For `0/0` rational forms: factor numerator
   and denominator, cancel the common `(x − a)` factor (reuse `upoly`/`poly`
   GCD and the rational engine), re-substitute. For roots: rationalize
   (multiply by conjugate). Handles `(x²−1)/(x−1) → 2`.
3. **Known-limits table (`known.rs`).** Standard results the other stages
   won't discover cheaply:
   - `sin(x)/x → 1`, `(1−cos x)/x → 0`, `(e^x−1)/x → 1`, `ln(1+x)/x → 1`
     as `x→0`;
   - `(1 + k/x)^x → e^k`, `(1+x)^(1/x) → e` as `x→0`/`∞`;
   - polynomial/rational **end behavior** at ±∞ by leading-term ratio
     (degree compare — exact, no sampling);
   - exponential-vs-polynomial and log-vs-polynomial growth orderings at ∞.
   Consider expressing per-function asymptotics as a new **`FnDef` facet**
   (`asymptotics`) so adding a function's growth rule stays one-place — ties
   into the registry work already done.
4. **L'Hôpital (`lhopital.rs`).** Detect an indeterminate form
   (`0/0`, `∞/∞`; reduce `0·∞`, `∞−∞`, `1^∞`, `0^0`, `∞^0` to one of those via
   log/reciprocal transforms), then replace `f/g` by `f'/g'` (reuse
   `diff::derivative`) and recurse into the pipeline. Guard with a dedicated
   recursion cap (`max_lhopital_depth`) — L'Hôpital can loop forever on
   `e^x/e^x`-style forms, so bail to Stage 6 (or `Unknown`) when depth or the
   shared fuel runs out. Never apply L'Hôpital without first *confirming* the
   indeterminate form (applying it to a determinate form is the classic wrong
   answer).
5. **Series (future / stretch, §9 P4).** A symbolic Taylor/Laurent expansion
   around `a` would subsume most of stages 2–4 and handle `∞−∞`
   cancellations cleanly — **but no symbolic series machinery exists today**
   (only numeric series kernels inside `precise/`). This is the largest piece
   of new math and is deferred; the pipeline is designed so it slots in as a
   stage without disturbing the others.
6. **One-sided reconciliation.** For a two-sided limit, compute both
   one-sided limits (recurse with `FromAbove`/`FromBelow`); equal ⇒ that
   value, unequal ⇒ `DoesNotExist`. This is also how `1/x` at `0` is proven
   to not exist (─∞ vs +∞), and how the sign of an infinite limit is pinned
   using the assumptions/sign machinery (`assumptions::is_positive` of the
   denominator near `a`).

### 6.2 The verification gate (mandatory, mirrors integration)
Before returning `Value(v)` or `Infinite(s)`, **numerically confirm** it
(`numeric.rs`): sample the body at a geometric sequence of points approaching
`a` from the required side(s) using `precise::eval_batch` at increasing
precision, and check the samples converge toward `v` (or diverge with the
claimed sign). A symbolic verdict that fails numeric confirmation is
downgraded to `Unknown` — never returned wrong. This is the exact discipline
`integrate` uses (`equals(derivative(F), f)`), adapted to a convergence check.

The numeric probe is *also* a standalone fallback: when every symbolic stage
declines but the samples converge convincingly (Richardson-extrapolated,
with a certified error bound from `precise`), return `Value` with the
recognized closed form if `evaluate_to_precision`'s digits match a small
constant, else `Unknown`. Be conservative — numeric-only evidence must be
strong (bounded error, monotone convergence) to avoid asserting a limit that
a subtle oscillation would refute.

### 6.3 Does-not-exist vs Unknown
Keep these strictly separate (the `DoesNotExist`/`Unknown` split in the enum):
- `DoesNotExist`: *proven* — sided limits disagree, or bounded oscillation
  (`sin(1/x)` at 0) is detected by the sampler failing to converge while
  staying bounded across scales.
- `Unknown`: engine ran out of methods/fuel. The default for anything not
  positively resolved.

### 6.4 Symbolic parameters (stretch)
`lim_{x→0} sin(a x)/(a x)` should give `1` (treat `a` as a nonzero constant
if assumptions say so). Case-splitting on an unconstrained parameter's sign
is out of scope initially; return `Unknown` rather than guessing a branch.

---

## 7. Output (`src/output/text.rs`, `src/output/latex.rs`)

- Text: `lim_{x->a} ( body )`, `lim_{x->a^+} ( body )`, `lim_{x->infinity}`.
- LaTeX: `\lim_{x \to a} body`, `\lim_{x \to a^{+}} body`,
  `\lim_{x \to \infty} body`. Reuse the existing arrow/`\infty` spellings and
  the Leibniz-style subscript rendering already in `render_leibniz`.
- Precedence: `lim` binds like a big operator — its body extends to the right
  at low precedence (`\lim_{…} x + 1` means `(\lim … x) + 1`? No —
  conventionally the operator scopes the following product; match
  `derivative_leibniz`/integral conventions and wrap the body when ambiguous).
- Round-trip test: `parse(to_text(e)) == e` and the LaTeX analogue, on a
  corpus of limit expressions (§9 P1).

`js_tree.rs` gets `to_js`/`try_from_js` arms so the JS `tree` interop still
serializes (as `["lim", var, point, dirflag, body]` or similar) — chosen to
be self-consistent, since there is no upstream shape to match.

---

## 8. Resource limits (`src/resource_limits.rs`)

Add fields to `ResourceLimits` (the newly-renamed struct), defaults generous
but adversary-safe:
- `max_limit_steps: i64` — shared fuel across the whole pipeline (like
  `max_integration_steps`). Default ~256.
- `max_lhopital_depth: u32` — L'Hôpital recursion cap. Default ~16.
- `max_limit_probe_points: usize` — numeric samples per side in the gate.
  Default ~64.
- `max_limit_probe_bits: u32` — precision ceiling for the probe (defer to
  `max_eval_precision_bits` unless a tighter cap is wanted).

All counted as operations/sizes, never wall-clock (the module's invariant),
so limit verdicts stay machine-independent and reproducible.

---

## 9. Phasing

Each phase ends green (native tests + wasm smoke), mirroring how the earlier
work was staged.

- **P0 — Representation + notation.** `Expr::Limit` variant + `LimitDir`; all
  ~10 match arms; parser (text `limit(...)` call form first, then `lim_{}`
  notation); printers; `js_tree`; round-trip corpus. **No evaluation yet** —
  `limit()` returns `Unknown`. Ships parse/print/serialize with zero math.
- **P1 — Binder audit + direct substitution.** The §5 checklist (variables /
  substitute / diff / eval-opaque); Stage 0–1 of the engine (preprocess +
  continuity); the numeric verification gate skeleton. Resolves continuous
  limits (`lim_{x→2} x²+1 = 5`).
- **P2 — Algebraic + one-sided + infinity.** Stage 2 (factor/cancel/
  rationalize), Stage 6 (sided reconciliation, DNE), rational end-behavior at
  ±∞. Resolves `(x²−1)/(x−1)`, `1/x` DNE, `(2x²+1)/(x²−3) → 2`.
- **P3 — L'Hôpital + known table.** Stages 3–4 with indeterminate-form
  detection and the `FnDef::asymptotics` facet. Resolves `sin x / x`,
  `(e^x−1)/x`, `x ln x → 0`.
- **P4 — Series (stretch).** Symbolic Taylor/Laurent expansion; subsumes hard
  `∞−∞` and nested indeterminate forms. Largest effort; gated on demand.
- **P5 — WASM surface.** `Expression::limit(var, point, dir)` returning the
  result (finite expression, an `∞` marker, `"DNE"`, or `undefined` for
  `Unknown`), plus a `limit_from_text("lim_{x->0} sin(x)/x")` convenience.

---

## 10. Testing (no JS oracle)

Since upstream has no limits, build fidelity three ways:
1. **Hand-authored corpus** of `(expression, variable, point, dir) → expected`
   covering each stage and each verdict kind, incl. DNE and Unknown.
2. **Symbolic-vs-numeric cross-check** in-crate: for every corpus entry whose
   answer is finite, the numeric gate (`precise::eval_batch`) must agree — this
   is a property test, not a fixed oracle, and doubles as the gate's own test.
3. **Offline differential** against an external CAS (SymPy/Maxima) run *by the
   author*, snapshotted into fixtures like the other corpora — not a runtime
   dependency.
4. **Adversarial**: oscillatory (`sin(1/x)`), essential singularities
   (`e^(1/x)` two-sided), L'Hôpital loops (`e^x/e^x`), deep nesting — all must
   terminate under fuel and return `DoesNotExist`/`Unknown`, never hang or lie.

---

## 11. Risks & mitigations

- **L'Hôpital non-termination / wrong application** → confirm indeterminate
  form before each step; hard depth cap; verification gate catches a bad
  value.
- **Numeric probe asserting a false limit** (oscillation aliasing the sample
  grid) → require certified error bounds + monotone/Cauchy convergence across
  *scales*, geometric point spacing, and cross-check against a symbolic stage
  when one fired.
- **Binder leaks** (a pass treating `var` as free) → the §5 checklist test
  enumerates every symbol-visiting pass; the compiler flags the match arms.
- **Series scope creep** → P4 is explicitly optional and isolated behind a
  pipeline stage; P0–P3 deliver a genuinely useful engine without it.
- **Bundle size** → the engine reuses `diff`, `precise`, `norm`, `factor`; new
  code is algorithmic, not table-heavy. If it grows, gate `limit` behind a
  cargo feature (the improvement plan's Phase 5 convention).
