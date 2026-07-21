# Arbitrary-Precision Evaluation Plan

Design for `evaluate_to_precision` and a reusable compiled evaluator in
`math-expressions-rs`, informed by a source-level analysis of
[tialaramex/realistic](https://github.com/tialaramex/realistic),
[timschmidt/hyperreal](https://github.com/timschmidt/hyperreal), and the
`stack_computable` prototype on
[siefkenj/realistic branch `experiments`](https://github.com/siefkenj/realistic/tree/experiments)
(clones under `tmp/realistic/`, `tmp/hyperreal/`, `tmp/realistic-experiments/`;
hyperreal descends from realistic and generalizes most of its machinery, and
the prototype is a direct trial of the flat-tape evaluation this plan builds
on — see §1c for what it proved and where it hit dead-ends).

**Hard requirements** (from the commissioning request):

1. **Stack-based** — no recursive tree walk anywhere in the evaluation path
   (compilation included); deep expressions must not overflow the native or
   wasm stack.
2. **Extensible** — adding a new function must be a local, table-driven change.
3. **Adversarial-input safe** — every loop bounded by the §7f `limits`
   machinery (operation counting, never wall-clock); pathological inputs
   degrade to an explicit `Unknown`, never a hang or a panic.
4. **f64 fast path** — when double arithmetic provably meets the requested
   tolerance, no bignum work happens at all.
5. **Quadrature-ready** — an expression compiles once and evaluates cheaply at
   many abscissae with a certified per-point error bound, so future adaptive
   quadrature (Gauss–Kronrod / adaptive Simpson) can consume it directly.

---

## 1. Technique inventory from the reference implementations

### 1a. `realistic` (Boehm-style computable reals + symbolic rational scale)

Representation: `Real = rational × Class` where `Class` tags a recognized
irrational factor (`One | Pi | Sqrt(q) | Exp(q) | Ln(q) | Log10(q) | SinPi(q)
| TanPi(q) | Irrational`, `src/real.rs:8-18`), backed by a lazy `Computable`
DAG (`Approximation` enum, `src/computable/approximation.rs:10-24`) evaluated
by `approx(p) -> BigInt` returning the value scaled by `2^-p`, accurate to
±1 at that scale.

| Technique | Where | Verdict for us |
|---|---|---|
| Scaled-integer approximation contract: `approx(p)` = `⌊value·2^-p⌉ ± 1` | `computable.rs:357-397` | **Adopt.** Simple, exact-rounding-friendly, no exponent field to manage; the whole kernel literature in both repos is written against it. |
| Guard-bit discipline per op: +2 bits for add, +3 for mul/inverse, series at `calc_precision = p − log2(2·terms) − 4` | `approximation.rs:48-117,164-202` | **Adopt** verbatim as the starting error budget for each kernel; validated by two shipped implementations. |
| MSD (most-significant-digit) steering: child precision derived from the *other* operand's magnitude for mul/inverse | `approximation.rs:79-117` | **Adopt** in kernel form (we get magnitudes from the f64 pre-pass, §4, so no iterative MSD search is needed in the common case). |
| Taylor kernels: exp/cos/atan in O(√p) terms, ln in O(p), with argument pre-scaling (`exp`: reduce to (−0.5, 2] by squaring; `ln`: to [0.5, 1.5); `cos`: mod 2π then double-angle) | `approximation.rs`, `computable.rs:119-251` | **Adopt** the kernels and reductions; they are small, dependency-free, and match our num-bigint stack. |
| π via Machin: `π = 4(4·atan(1/5) − atan(1/239))` with `IntegralAtan` leaves | `computable.rs:99-106` | **Adopt.** Sufficient far past our precision cap; AGM/Chudnovsky explicitly rejected by hyperreal's own audit (PERFORMANCE.md "Reference Audit") as not paying off below extreme precision. |
| `sqrt`: `BigInt::sqrt` below 50 result digits, Newton with precision-halving recursion above | `approximation.rs:204-256` | **Adopt**, but restructure the Newton doubling as a loop (it is self-recursion in realistic — conflicts with requirement 1). |
| Symbolic `Class` algebra (`√2·√3 → √6`, `eᵃ·eᵇ → e^(a+b)`, exact perfect-square extraction) | `real.rs:1042-1215` | **Reject as a layer.** This duplicates what our simplifier/canonicalizer already does symbolically at the `Expr` level — `simplify` is our `Class` algebra. The evaluator receives an already-simplified tree. |
| Sign by unbounded-ish refinement: loop to precision −2000, `compare_to` **panics on equal inputs** (`computable.rs:434`) | `computable.rs:399-451` | **Reject.** This is the canonical computable-real trap (equality is undecidable). We never decide equality numerically; all sign/zero queries are bounded and tri-state (§7). |
| Abort via `Signal: Arc<AtomicBool>` polled inside series loops | throughout | **Replace** with §7f operation-counted budgets — deterministic, wasm-friendly, no threads required. |

### 1b. `hyperreal` (industrialized descendant)

Same `approx(p)` contract; `Real = rational × Class × Option<Computable>` with
a much larger `Class`/`Approximation` vocabulary (150+ node kinds), lock-free
per-node caches, and heavy dispatch-trace-driven performance work.

| Technique | Where | Verdict for us |
|---|---|---|
| **Partially stack-based evaluator**: `approx_signal` flattens `Negate`/`Add`/`Offset` chains through an explicit `Frame` work-stack with an inline-16 buffer spilling to heap (`InlineStack<Frame, 16>`) — but *all other* node kinds still make a kernel call that recurses into children | `computable/node/approximation_queries.rs:67-169` | **Adopt the mechanism, complete the coverage.** Their flattening is partial because they retrofitted an existing recursive design. We are green-field: compile to a flat postorder tape (§3) so *every* op is non-recursive, not just the three chain-formers. |
| **f64 primitive-approximation cache**: lock-free `AtomicU64` storing an f64 (or tagged f32) in QNaN-punned bit patterns, purely as an accelerator, never semantic | `real/arithmetic/classification.rs:86-191` | **Adopt the idea, not the mechanism.** Our fast path is a first-class evaluation tier with *certified error bounds* (§4), not an opportunistic cache; single-threaded wasm makes the atomics pointless (this matches the existing PORTING_PLAN verdict on hyperreal). |
| **Bounded tri-state sign**: `sign_until(min_precision)` — structural facts → cached approx → cheap bounds → refinement loop with a hard precision floor, returning `Option<RealSign>` (`None` = "not proven within budget") | `approximation_queries.rs:274-310`, `real/arithmetic/facts.rs:517-560` | **Adopt** as the exact shape of every sign/zero/comparison query (§7). This is the fix for realistic's panic. |
| Guard bits at flattened add: children at `p−2`, one rounding at the end; offsets translate precision instead of doing arithmetic | `approximation_queries.rs:132-146` | **Adopt** — same constants as realistic, proven in the iterative setting we need. |
| Cancellation-aware kernels: `expm1`, `ln_1p`, `sqrt1pm1`, `hypot`, fused `sum_products`/`diff_of_products`, Horner `eval_poly` | `approximation/representation.rs` | **Adopt selectively, later.** Phase-2+ kernels; `eval_poly`-style fused tape ops are directly useful for quadrature integrands. |
| Argument reduction with exact-rational certificates (tangent sector certificates, half-π residual arithmetic, Payne–Hanek-style reduction for huge trig arguments) | PERFORMANCE.md, dispatch paths | **Adopt the simple form**: trig reduction = subtract the nearest multiple of π/2 computed from our π kernel at `arg_msd + p` guard precision, with a `limits` cap on argument magnitude (§7). The exact-rational certificate zoo is an optimization we don't need at our scale. |
| Dispatch tracing + promoted-slow-offender benchmark corpus | `dispatch_trace.md`, `benchmarks.md` | **Adopt the practice** in miniature: a `#[cfg(test)]` tier/escalation counter so tests can assert "this input stayed on the f64 tier" / "escalated once" (§9). |
| Statistical function suite (erf/pnorm/gamma/…), matrix kernels, serde of DAGs | throughout | **Out of scope.** Not part of the JS library's surface; the registry (§6) leaves the door open. |

### 1c. `siefkenj/realistic` branch `experiments` — the `stack_computable` prototype

Clone under `tmp/realistic-experiments/src/stack_computable/` (~940 lines).
This is a direct prototype of the tape idea: `StackComputable { stack:
Vec<OpStack> /* RPN */, literals: Vec<Literal> }`, expressions composed by
tape concatenation, evaluated by two **iterative** passes — a backward
precision-requirement pass (`update_required_precisions`, explicit index
stack) and a forward compute pass over a per-slot `Cache` of
`PrescaledValue(BigInt, Precision)`. It validates the architecture and, more
usefully, its dead-ends mark exactly where this plan must differ:

| Finding | Where | Lesson adopted |
|---|---|---|
| Flat RPN tape + side literal table + iterative eval works; kernels (exp/ln/cos series with realistic's exact guard-bit comments, `bound_log2` truncation bookkeeping) port to the tape setting unchanged | `stack_computable.rs:7-12`, `approximate.rs:45-180` | Confirms §3/§5 wholesale; the P2 kernels can start from this code, which already carries the ulp-error justification comments. |
| **Static precision planning fails for value-dependent ops.** `precision_needed` for `Mul` returns `out_precision * 2` because operand magnitudes are unknown statically — so a depth-k `Mul` chain demands ~2ᵏ·p bits at the leaves (exponential); `Inverse` can't plan at all and falls back to a magic re-request heuristic inherited from realistic (`(p·3)/2 − 16`, flagged "I don't know where this heuristic comes from") | `approximate.rs:249-254,40-43` | This is the strongest argument for the plan's Tier-0-first design: the f64 pass gives every node's magnitude (MSB) *before* Tier 2 plans precisions, so `Mul` plans `p + msd(other) + 3` instead of `2p`, and `Inverse` plans exactly. Magnitude-blind static planning is a proven dead-end. |
| **`NeedMorePrecision` → restart-from-index-0.** When a plan proves insufficient mid-pass, the prototype bumps the slot's requirement and rescans the whole tape (cache-skipping already-good slots) — chosen because the implicit `literal_index` cursor made resuming elsewhere fragile | `stack_computable.rs:179-220` | Two fixes: (a) literals referenced by index *in* the op (`Op::Const(u32)`, §3) so position bookkeeping can't force restarts; (b) per-node precision patching is dropped entirely — with magnitude-informed planning, under-provisioning is rare, and when it happens the **Ziv loop restarts the whole tape at 2w** (§5), which is simpler and amortizes identically. |
| **Per-op argument discovery is O(n) per operator** (`operator_argument_indices` walks backward counting arities → O(n²) on deep chains) | `stack_computable.rs:255-297` | The tape evaluator uses a proper value stack (pop/push, §3), making operand access O(1); `max_stack` is precomputed. Slot-addressed caching is not needed once restarts are gone. |
| **`PrescaledValue` invariants are the right value contract**: `rescale` only *coarsens* (returns `None` if asked to invent precision), `meets_precision` = stored ≤ required, ops may *overdeliver* precision (`Success(value, actual_prec)` — sqrt returns finer than asked) and consumers rescale | `prescaled_value.rs:12-41`, `approximate.rs:181-195` | Adopt verbatim as `MpFix`'s API (§5): coarsen-only `rescale`, overdelivery allowed, consumer-side normalization. |
| **Sqrt without Newton**: request the argument at even `2p−2` bits, take `BigInt::sqrt`, return at `p−1` | `approximate.rs:181-207` | Adopt as the P2 sqrt kernel — simpler than realistic's Newton doubling and correct; the argument-precision doubling is acceptable below our precision cap (revisit only if profiling says so). |
| Anti-freeze via `STOP_PRECISION = −10000` **panic**; `Failed` panics; π `unimplemented!` | `stack_computable.rs:16,132-136` | Confirms the failure-mode requirement: budget exhaustion must be a *value* (`Precise::Unknown`, §7), not a panic — the prototype shows how naturally the panic version creeps in. |

**Synthesis.** Both published libraries answer "what is a *number* object with
lazy precision?" Our problem is different: "evaluate an *expression* at a point to
a tolerance, possibly many times." That inversion drives the core decision:

> **We compile the expression once into a flat tape and re-run the tape at
> different precisions/bindings, instead of building a lazy computable-real
> DAG with per-node caches.** The tape gives requirement 1 (stack-based) by
> construction, requirement 5 (cheap re-evaluation for quadrature) by
> construction, and eliminates the cache/Arc/atomics machinery that the
> PORTING_PLAN already rejected. What we keep from the references is their
> *numerics*: the `approx(p)` scaled-integer contract, guard-bit constants,
> series kernels, argument reductions, and the bounded tri-state sign
> discipline.

---

## 2. Architecture overview

```
                       ┌────────────────────────────────────────────┐
   Expr (canonical) ──►│ compile() — iterative postorder, §3        │
                       │  · exact-rational subtrees folded at       │
                       │    compile time (existing canonicalize)    │
                       │  · variables → slot indices                │
                       └──────────────┬─────────────────────────────┘
                                      ▼
                          CompiledExpr { tape: Vec<Op>, nslots, nstack }
                                      │
              eval(bindings, tol) ────┤ (one entry point, tiered inside)
                                      ▼
        ┌─────────────── Tier R: exact ───────────────┐
        │ tape is constant-only and rational-only     │──► exact Number
        └──────────────────────┬──────────────────────┘
                               ▼
        ┌─────────────── Tier 0: f64 + error ─────────┐
        │ run tape over (f64 value, f64 abs-error)    │
        │ pairs; if err ≤ tol → done (THE fast path)  │──► f64 ± bound
        └──────────────────────┬──────────────────────┘
                               ▼ (bound too big / overflow / domain edge)
        ┌─────────────── Tier 2: MpFix Ziv loop ──────┐
        │ run tape over scaled BigInt at working      │
        │ precision w; if result stable to tol → done │──► MpFix ± 1 ulp
        │ else w ← 2·w, bounded by limits             │──► or Unknown
        └─────────────────────────────────────────────┘
```

- **No new dependencies.** Tier 2 is fixed-point over `num-bigint` (the same
  foundation as both reference crates), not an external bigfloat crate.
  `astro-float`/`dashu` were considered and rejected: they would be a second
  number system to audit, their internal loops are invisible to our `limits`
  instrumentation (requirement 3), and both references demonstrate that the
  ~10 kernels we need are a few hundred lines each on plain `BigInt`.
- **Complex arithmetic** is a pair of reals at every tier (Tier 0 already has
  `Complex64`); complex kernels are compositions of real ones (§6, phase 4).

## 3. Compilation: `Expr` → `CompiledExpr`

New module `src/precise/tape.rs`.

```rust
pub struct CompiledExpr {
    ops: Vec<Op>,          // postorder (RPN); evaluation is a single loop
    consts: Vec<Number>,   // exact constants referenced by index
    vars: Vec<String>,     // slot i binds to vars[i]
    max_stack: usize,      // computed at compile time; value stacks preallocate
}

enum Op {
    Const(u32),            // push consts[i]
    Var(u32),              // push binding for slot i
    Add(u32),              // pop n, push sum       (n-ary: canonical Add is flat)
    Mul(u32),              // pop n, push product
    Neg, Inv,              // Mul(-1,·) / Pow(·,-1) get dedicated cheap ops
    PowInt(i64),           // integer exponent: binary powering
    Pow,                   // general: exp(y·ln x) at Tier 2, powc at Tier 0
    Call(FnId, u8),        // registry function, arity
}
```

- **Compilation is iterative** — an explicit `Vec<(node, state)>` postorder
  walk, the same discipline hyperreal's `Frame` stack uses but over the whole
  grammar. No recursion anywhere (this also future-aligns with §5b
  STACK_SAFETY_PLAN: the evaluator will survive trees the *parser* can build).
- Input is the **canonical** layer (`canonicalize` already applied): `Div` and
  `Neg` are gone, `Add`/`Mul` are flat n-ary — which is exactly why n-ary
  `Op::Add(n)` beats hyperreal's binary-chain flattening: a 10 000-term sum is
  *one* op with one rounding step (guard bits `⌈log2 n⌉ + 1` instead of 2 per
  link, §5).
- **Compile-time folding**: any subtree with no `Var` and no irrational op
  folds through the existing exact canonicalizer into a single `Const`. A
  fully-rational expression short-circuits to **Tier R** and never evaluates.
- Tape length and `max_stack` are checked against `limits::current()`
  (`max_tape_ops`, new; default generous, e.g. 100 000) so a pathological
  expression fails fast at compile.
- Unsupported nodes (relations, matrices, `Blank`, unknown functions) → 
  `CompileError::NotNumeric`, mirroring `evaluate_to_constant`'s `None`.

## 4. Tier 0 — f64 with certified error bounds (the fast path)

New module `src/precise/tier0.rs`. The tape runs over

```rust
struct Approx64 { val: f64, err: f64 }   // |true − val| ≤ err (absolute)
```

Propagation rules (standard forward error analysis; ε = 2⁻⁵³, u = ε/2):

| Op | value | error bound |
|---|---|---|
| `Const(c)` | `c.to_f64()` | `|val|·u` (0 if exact in f64) |
| `Add(n)` | fold | `Σ errᵢ + (n−1)·|running sums|·u` (accumulated per step) |
| `Mul`/`Inv` | fold | `|a|·err_b + |b|·err_a + err_a·err_b + |val|·u` |
| `Call(f)` | `f(val)` | `Lip_f(val)·err_arg + |val|·cond_f·u` — each kernel supplies a Lipschitz/condition factor (§6) |

- Any `NaN`/`Inf`/domain-edge (`ln` near 0, `Lip` unbounded) → immediate,
  cheap **escalate** signal; no cleverness at this tier.
- Success criterion: `err ≤ tol` for the requested tolerance (`tol` derived
  from requested digits: `0.5·10^−digits·max(1,|val|)` for relative mode).
  Since f64 carries ~15.9 digits and quadrature tolerances are typically
  1e-6…1e-10, **this tier terminates almost every real workload** —
  requirement 4.
- This replaces hyperreal's QNaN-punned atomic cache with something stronger:
  not a cached guess, but a *certificate* that double arithmetic sufficed.
- Complex: same structure over `Complex64` with `err` bounding `|Δz|`;
  reuses the existing `eval_complex` kernel implementations, now
  error-annotated.

## 5. Tier 2 — `MpFix` fixed-point + Ziv escalation

New modules `src/precise/fix.rs`, `src/precise/ziv.rs`.

```rust
/// value ≈ mant · 2^scale, |error| ≤ 1 at the last bit (the realistic/
/// hyperreal `approx(p)` contract, reified as a value instead of a query).
/// API contract from the stack_computable prototype (§1c): `rescale` only
/// coarsens (never invents precision), kernels may overdeliver precision and
/// consumers normalize.
struct MpFix { mant: BigInt, scale: i32 }
```

- **Precision planning pass** (the §1c backward pass, made magnitude-aware):
  before a Tier-2 run, one backward sweep over the tape assigns each slot a
  target precision from the root request through per-op transfer functions —
  n-ary add: children at `w + ⌈log2 n⌉ + 1`, one rounding; mul/inv: `w +
  msd(other operand) + 3`; series ops: `w − 3` with `calc_precision = w −
  ⌈log2(2·terms)⌉ − 4`. The operand MSDs come from the Tier-0 pass (every
  node's f64 magnitude was already computed), which is what dissolves the
  prototype's two dead-ends: no `Mul → 2w` exponential over-request, no
  `Inverse` magic re-request heuristic. realistic's iterative `iter_msd`
  search survives only as the fallback when Tier 0 overflowed/underflowed at
  a node.
- The forward pass then runs the tape bottom-up over `MpFix` in a single
  sweep. If a kernel still finds its plan insufficient (rare: Tier-0
  magnitude was a bound, not exact), it does **not** patch and restart
  per-node like the prototype — it aborts the sweep and lets the Ziv loop
  rerun the whole tape at doubled `w`, which is simpler and amortizes
  identically.
- **Ziv loop** (`evaluate at increasing precision until the rounded answer
  stabilizes`): `w₀ = needed_bits(tol) + 32 + ⌈log2 tape_len⌉`; if the result
  interval still straddles a rounding boundary at `tol`, double `w`, re-run
  the tape, up to `limits.max_ziv_rounds` (default 6 → worst case 64×
  the target precision). Budget exhausted ⇒ `Unknown` (§7), never a loop.
- Constants: π (Machin, realistic `computable.rs:99-106`), e (`exp(1)`),
  ln 2 (for reductions) — computed once per (thread, precision) into a
  thread-local high-water-mark cache: store the finest approximation computed
  so far, rescale down for coarser requests (the monotonic-cache idea from
  both references, applied only where it pays — shared constants — instead of
  at every node).
- All series loops count iterations against `limits` (`max_series_terms`,
  default e.g. 100 000 — far above any legitimate request under
  `max_eval_precision_bits`, present as a backstop, same philosophy as the
  existing `max_factorial`).

## 6. Function kernels & extensibility

New module family `src/precise/kernels/`. One registry, three obligations per
function:

```rust
pub struct FnKernel {
    pub name: &'static str,          // + aliases ("asin"/"arcsin", as in diff.rs)
    pub arity: u8,
    /// Tier 0: f64 evaluation + error-propagation factor at the point.
    pub f64_eval: fn(&[Approx64]) -> Tier0Result,     // Value | Escalate
    /// Tier 2: MpFix at working precision w (±1 ulp contract), or Unsupported
    /// (⇒ compile rejects, or Tier 2 reports Unknown for exotic combinations).
    pub fix_eval: fn(&[MpFix], w: i32, budget: &mut OpBudget) -> FixResult,
    /// Monotonicity/range facts for future interval evaluation (§8); optional.
    pub facts: FnFacts,
}
static REGISTRY: &[FnKernel] = &[ /* one row per function */ ];
```

- **Adding a function = adding one `FnKernel` row** (plus its `fix_eval`
  kernel, typically an argument reduction + one of the four series templates
  below). The tape, tiers, Ziv loop, limits plumbing, and wasm surface never
  change. This is requirement 2, and it is deliberately *simpler* than both
  references, where a new function touches an enum, a dispatcher, a
  constructor, and (hyperreal) a Class classifier.
- Phase-2 kernel set (parity with today's `eval_complex` real path):
  `sqrt` (BigInt::sqrt + Newton-doubling *loop*), `exp`/`ln` (realistic's
  prescaled series + reductions), `sin`/`cos`/`tan` (cos kernel + π/2
  reduction), `atan`/`asin`/`acos`, `abs`, `log10`, `sinh`/`cosh`/`tanh`
  (from exp), plus n-th roots via `PowInt`+`Inv`+`sqrt` composition.
- Series templates (shared helpers, each loop iteration ticks the budget):
  alternating Taylor (cos/sin/atan), ratio-recurrence Taylor (exp), log
  series on reduced argument, Newton refinement (sqrt/inv). realistic's
  `calc_precision = p − ⌈log2(2·terms)⌉ − 4` truncation-error bookkeeping is
  adopted as-is.

## 7. Adversarial-input protection (and what we deliberately cannot decide)

New `limits` fields (§7f style — all operation counts, no wall-clock):

| Field | Default | Bounds |
|---|---|---|
| `max_eval_precision_bits` | 17 000 (≈ 5 000 decimal digits) | working precision `w` in Tier 2, per-request digit count |
| `max_ziv_rounds` | 6 | escalation loop |
| `max_series_terms` | 100 000 | every kernel series/Newton loop |
| `max_tape_ops` | 100 000 | compile-time tape length and per-run op count |
| `max_trig_arg_bits` | 4 096 | MSD of a trig/exp argument before reduction is refused (`sin(2^2^20)` answers `Unknown`, matching the spirit of the existing `max_pow_bits`) |

Failure semantics — **tri-state, never panic, never loop** (hyperreal's
`sign_until` shape, realistic's `compare_to` panic as the counterexample):

```rust
pub enum Precise<T> {
    Exact(T),          // Tier R: value is exact
    Bounded(T, ErrBound), // certified enclosure at requested tolerance
    Unknown(Reason),   // budget exhausted / domain edge / not numeric
}
```

- **Equality/zero-ness is never decided numerically.** A request like "is
  this exactly zero" is answered `|value| ≤ 2^-p` or `sign proven ±` within
  the budget, else `Unknown`. `exp(ln(2)−ln(2))−1`-style traps terminate in
  `max_ziv_rounds` with a tight interval around 0 and an honest `Unknown`
  for the sign. Symbolic zero detection remains `simplify`/`equals`' job.
- Everything is `Cell`-thread-local exactly like the existing `limits::with`,
  so tests and the wasm boundary can tighten budgets scope-locally.
- Memory: `w ≤ max_eval_precision_bits` bounds every mantissa; tape length
  bounds live values; peak memory is O(`max_tape_ops · max_eval_precision_bits`)
  — no repro-in-container needed to validate, it's arithmetic
  (per the no-OOM-repro policy).

## 8. Quadrature-readiness (future numeric integration)

The design choices above are what make this section short:

- **Compile once, evaluate N times.** `CompiledExpr::eval(&bindings, tol)`
  with a caller-owned reusable scratch (`EvalScratch` holding the Tier-0 and
  Tier-2 value stacks) — zero allocation per abscissa on the f64 tier.
- **Certified per-point error.** Adaptive quadrature needs `f(xᵢ)` *and*
  trustworthy error; `Bounded(v, e)` is exactly the input a Gauss–Kronrod /
  adaptive-Simpson refinement loop needs to keep its own error budget honest.
  Per-point tier escalation is automatic and local: only abscissae near a
  cancellation/singularity pay bignum cost.
- **Batch API sketch** (implemented in the integration phase, declared now so
  the tape design accounts for it):
  `eval_batch(var: &str, points: &[f64], tol) -> Vec<Precise<f64>>`.
- **Interval extension (optional, later):** run the same tape over
  `[lo, hi]` interval values using `FnFacts` monotonicity data — gives
  rigorous enclosures of `f` over subintervals for verified quadrature and
  for `equals`' sampling stage. The tape/registry design requires no change;
  it is a third value type over the same ops.
- Nodes/weights themselves (Legendre/Kronrod abscissae) are the integration
  feature's problem, not the evaluator's; at `tol ≤ 1e-15` they can be
  computed once via Tier 2 Newton on Legendre polynomials through this same
  API (`eval_poly`-style fused ops make this cheap, per hyperreal's Horner
  kernels).

## 9. Testing strategy

1. **Oracle corpus** — `scripts/generate-precision-corpus.py` (mpmath, seeded,
   committed JSON like the existing corpora): ~500 (expression, point,
   digits, expected-digits) rows across all kernels, digits ∈ {10, 50, 500}.
   Rust test asserts agreement to the last requested digit; snapshot-style
   known-failures file, same machinery as the other corpora.
2. **Soundness properties** (proptest, like `tests/autogenerated_fuzz_tests`):
   - *Bound honesty*: Tier 0's `err` always ≥ |Tier 2 reference − Tier 0 val|.
   - *Ziv monotonicity*: answers at digits d and d′ > d agree on the first d.
   - *Contract*: Tier 2 result within 1 ulp of an mpmath value at 2× precision.
3. **Adversarial suite** (extends `tests/norm.rs` limits tests):
   - 10⁶-node deep/wide expressions: compile+eval succeed or fail *fast*, no
     stack growth (assert with a small thread stack, e.g. 256 KiB spawn).
   - `sin(10^10^6)`, `exp(exp(exp(20)))`, `1/(x−x)`-shaped cancellations →
     `Unknown` within budget; op-count assertions, not wall-clock.
   - Escalation counters (`#[cfg(test)]`, hyperreal's dispatch-trace idea in
     miniature): assert `sin(1)@1e-10` never leaves Tier 0, and
     `(1+2^-80)−1` escalates exactly once.
4. **Differential vs existing evaluator**: for every row of the existing
   `evaluate-corpus.json`, `evaluate_to_precision(e, 15)` must agree with
   `evaluate_to_constant` to f64 tolerance — the new path may not regress the
   old one.

## 10. Phasing

> **Status: P1 + P2 ✓ done 2026-07-20** (`src/precise/`, `tests/precise.rs`,
> 10 tests). Implemented as designed with three notes:
> (a) **oracle substitution** — mpmath is unavailable in the container (no
> pip), so the P2 exit criteria run against hardcoded 50+-digit reference
> constants (√2, π, e, ln 2), self-consistency (d- vs 2d-digit prefix
> agreement across 8 expression shapes), identity round-trips
> (`exp(ln x) = sqrt(x²)` digit-for-digit at 60), and 500-digit π via two
> independent routes; a real mpmath corpus remains desirable when tooling
> allows. (b) The P1 differential harness covers the evaluate corpus
> *including bound-variable rows* through `eval_tape` (60%+ coverage
> asserted, zero disagreements at 1e-8). (c) The deep-tree test runs a
> 50 000-deep `sqrt` chain in a 512 KiB thread (compile + both tiers
> iterative; the `Expr` is leaked deliberately — recursive `Drop` is a §5b
> issue, not an evaluator one). One instructive bug found by the tests:
> `div_round` double-negated for negative divisors, silently corrupting
> every alternating series (π, `ln1p`) while leaving positive-divisor
> series (e, ln 2 constant) correct — caught by the known-constant oracle
> on first run.
>
> **P3 + P4 + P5 ✓ done 2026-07-20** — the plan is fully implemented
> (`tests/precise.rs`, 17 tests; wasm smoke 42/42):
> - **P3**: sin/cos/tan (π/2 reduction with BigInt quadrant arithmetic — the
>   absolute-precision contract makes Payne–Hanek unnecessary below
>   `max_trig_arg_bits`, new limit, 4096), asin/acos/atan (halving reductions
>   + odd series; reciprocal branch only for |x| ≥ 2 — recursing at |x| = 1
>   never terminates, caught by test), sinh/cosh/tanh (via exp; ÷2 is
>   `scale − 1`, not `+ 1` — second sign-convention bug caught by the digit
>   oracle), log10 (ln/ln 10 with a cached ln 10). Adversarial: `sin(2^5000)`
>   and `exp(exp(exp(20)))` answer Unknown fast (the exp kernel now guards
>   its reduction magnitude itself, not just via the planner — the complex
>   sweep reached it unguarded and hung the suite once).
> - **P4**: complex Tier 0 (`Complex64` + |Δz| bounds, principal branches
>   matching `eval_complex`) and a complex Tier 2 (`CFix` pairs of `MpFix`;
>   kernels composed from the real ones: csqrt cancellation-safe via the
>   larger-component trick, cln via atan2, casin/catan via log formulas),
>   `Precise::Complex` variant, orchestrated as a fallback after the real
>   tiers. `ln(−1)` = iπ to 40 digits; `asin(2)` matches `eval_complex`;
>   complex corpus rows: parity or honest Unknown, ≥50% covered.
> - **P5**: `CompiledExpr::eval_f64` (Tier-0-only per-abscissa path with
>   certified error, verified ≤ 1e−14 across 101 points of `exp(−x²)`),
>   `eval_batch` (per-point tier escalation), wasm
>   `evaluate_to_precision(digits) → String` incl. complex formatting.
>
> Remaining niceties (not in the phase table): an mpmath corpus when
> tooling allows; scratch allocation pooling if quadrature profiling ever
> shows the per-point `Vec` allocations matter.
>
> **Post-plan additions ✓ 2026-07-19**:
> - **`RootOf` evaluation** (the §2d cross-plan item with MATRIX_PLAN):
>   `Op::Root` tape leaf; Tier 0 carries certified bounds (real roots:
>   ulp-level from exact Sturm refinement; complex: the rigorous
>   `n·|p(z)/p′(z)|` simple-root bound with f64 Horner rounding majorized);
>   Tier 2 refines real roots by dyadic-rational Newton with an exact
>   sign-change certificate and complex roots by CFix Newton on a doubling
>   precision ladder under the same bound. `tests/precise_rootof.rs`.
> - **Certified quadrature** (`precise/quad.rs`,
>   `integrate_to_precision(f, x, a, b, digits)`): adaptive composite
>   Simpson whose *entire* error is rigorously bounded — Tier-0 certified
>   node values + a conservative outward-widened interval extension of the
>   tape evaluating the symbolic 4th derivative for the remainder
>   `(w⁵/2880)·sup|f⁗|` — under `max_quadrature_segments`. Guaranteed
>   digits or honest Unknown (poles, endpoint singularities, > 13 digits —
>   the f64-node ceiling). `tests/quadrature.rs`, 10 tests incl. a
>   per-digit-count guarantee sweep against π. Divergent vs
>   convergent-improper vs expensive currently all refuse identically —
>   the classification design is `tmp/DIVERGENCE_PLAN.md` (2026-07-20).
>
> **Adversarial hardening ✓ 2026-07-19** (`tests/rootof_adversarial.rs`,
> `tests/quadrature_poles.rs`) — five real findings from stress inputs, all
> fixed:
> - Sturm chains now use a primitive-normalized PRS (positive scaling
>   preserves sign-variation counts; the naive Euclidean chain's
>   exponential coefficient blowup made Wilkinson-20 take minutes);
> - root bounds are power-of-two Fujiwara, not Cauchy (t³ − 2·10³⁰ has a
>   Cauchy bound ~10³⁰ but roots at ~10¹⁰ — every wasted octave is a full
>   chain-evaluation bisection level), and Durand–Kerner seeds use the same
>   bound; the f64 seed refinement is sign-based (one evaluation per step)
>   with enough iterations for astronomically wide isolating intervals, and
>   the dyadic-Newton ladder iterates to convergence rather than a fixed
>   step count;
> - conjugate pairs are mirrored exactly (z, z̄) and pair *order* uses
>   re-clustering with a gap dichotomy (merge below tol/8, split above
>   8·tol, refuse between) — raw DK noise in `re` had ordered
>   imaginary-axis pairs arbitrarily;
> - quadrature's incremental error tracker is drift-checked: near-pole
>   segments carry ~10²⁰ remainder bounds whose push/pop cancellation once
>   collapsed the running sum and let a spurious break package 0 ± 10⁸ as
>   an "answer"; break candidates now re-sum exactly, and packaging
>   hard-verifies the digit target. Result: divergent/improper integrals
>   refuse in <1 ms; a smooth 10⁻⁶-wide spike (∫1/((x−½)²+10⁻¹²))
>   converges to certified 8 digits in ~7 ms.
> Suite: Wilkinson-20 exact, Mignotte pair split at 1.4·10⁻⁶ with 40
> certified digits each, deg-50 sparse ordering, 10⁻⁹-separated pairs
> honestly refused.

| Phase | Deliverable | Exit criteria |
|---|---|---|
| **P1** | `tape.rs` compile (iterative) + Tier R + Tier 0 real path, `Approx64` error rules, `Precise<T>` API, limits fields | differential test vs `evaluate_to_constant` green; deep-tree stack test green |
| **P2** | `MpFix`, planning pass, Ziv loop, kernels: add/mul/inv/powint/sqrt/exp/ln + π/e/ln2 constant cache (series kernels ported from the §1c prototype's `approximate.rs`, which already carries the ulp-error accounting) | mpmath corpus green at 10/50/500 digits for the P2 kernel set |
| **P3** | trig + inverse-trig kernels with π/2 reduction, `max_trig_arg_bits` | full corpus green; adversarial suite green |
| **P4** | complex tiers (pairs of reals; principal branches matching `eval_complex`) | complex corpus rows green; `evaluate_to_constant` parity kept |
| **P5** | quadrature hooks: `EvalScratch`, `eval_batch`, public `CompiledExpr`, wasm `evaluate_to_precision(digits) -> String` | wasm smoke rows; batch benchmark (≥10⁶ f64-tier evals/sec on the demo integrand) |

Estimated effort: P1–P2 are the bulk (~2–3 focused days); P3–P5 ~1 day each.

## 11. Open questions

1. **Output form**: digits-as-string (wasm-friendly, ties to `Precise`
   enclosure) vs a `Number::Big` rational approximation vs both. Plan assumes
   both: `to_digits(n)` and `to_rational()` on the result.
2. **Default `digits` cap** exposed to Doenet content authors (plan: 5 000 via
   `max_eval_precision_bits`; content rarely needs > 50).
3. Whether `equals`' numeric sampling stages should eventually consume Tier 0
   error bounds (would turn several heuristic tolerances into certificates) —
   out of scope here, noted as follow-up.
4. (Cross-plan) `tmp/MATRIX_PLAN.md` §2c plans certified polynomial root
   isolation (interval Newton at escalating precision, Mahler-bounded) as a
   consumer of this evaluator's Tier-0 → Tier-2 ladder — a second client
   shaping the same `Precise<T>`/limits API. `tmp/INTEGRATION_PLAN.md` §5
   adds a third: the generalized hypergeometric (pFq) numeric kernel is one
   more ratio-recurrence `FnKernel` row (§6), and its quadrature hooks (§8)
   power that plan's numeric self-verification of antiderivatives.
