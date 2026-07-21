# Form grading plan (syntactic answer-form checks for teaching)

> **STATUS (2026-07-21):** DRAFT — designed, not started. Greenfield; no
> implementation yet. All paths below are relative to
> `packages/math-expressions-rs/` (crate moved to `packages/` this session).
>
> **One-line goal:** grade the *form a student wrote* ("is it factored?", "is
> the fraction reduced?", "is it a decimal or an exact value?") as a suite of
> opt-in, composable predicates over the faithful (pre-normalization) `Expr` —
> distinct from `equals`, which only decides mathematical value.

---

## 1. Problem

`equals` / `equals_syntactic` (`src/eq/mod.rs`) answer *"is this the same
value/expression?"*. They cannot answer *"did the student write it in the
required form?"* — because the required-form questions teachers ask
(§ Appendix A) are about the **tree as typed**, and two of them
(decimal-vs-exact, multiplication style) are about facts the current parser
**discards before the AST exists**:

- The lexer maps every decimal spelling to an exact rational
  (`0.5`, `0.50`, `.5` → `Num(Rat(1,2))`; `from_decimal_str`, `src/num.rs`),
  so "as a decimal to 3 places" is unanswerable from the tree.
- Implicit and explicit multiplication and every glyph (`*·×•⋅`, `\cdot`,
  `\times`) all collapse to one `Tok::Times` → `Expr::Mul` with no marker
  (`src/parse/lexer.rs`).
- `convert()` calls `flatten(result)` last (`src/parse/text.rs:157`,
  `src/parse/latex.rs:207`), erasing associative grouping
  (`(a+b)+c` ≡ `a+b+c`).

Everything else a teacher asks about (factored / expanded / reduced / radical
form / negative exponents / standard-vs-vertex form / `+C`) **is** recoverable
from the faithful `Expr`, which already preserves `Div`, `Neg`, operand order,
and function-name spelling. `src/grade.rs` today contains only *value*-based
helpers (sign-error tolerance, linear solve, set membership) — no form checks.

## 2. Design principles (validated by prior art — Appendix A)

1. **Predicates, not one comparator.** STACK ships independent answer tests
   (`FacForm`, `Expanded`, `LowestTerms`, `SingleFrac`, `PartFrac`,
   `CompletedSquare`); WeBWorK ships strict contexts. Standards *diverge* on
   what to enforce (CCSSM/TEKS/Florida-HS mandate many forms; Ontario mandates
   almost none; Florida scopes "simplest form" out of specific 4–5 fraction
   benchmarks — Appendix A). So form-checking must be **per-problem opt-in**,
   never global.
2. **Check structure, not a canonical rewrite.** STACK's own warning:
   "establishing an expression is factored is *not* the same as comparing it
   to `factor(ex)`." Predicates inspect the faithful tree; they do **not**
   canonicalize first (that would erase the very form under test).
3. **Value and form are orthogonal, and usually both wanted.** A factored
   answer must (a) *be* factored **and** (b) `equals` the key. Predicates
   return form verdicts; callers `&&` them with `equals` as needed.
4. **Tags for the two irrecoverable facts, not source maps.** No sourced
   directive needs raw-input character positions (Appendix A). Carry the two
   missing facts as small self-contained tags that serialize across the wasm
   boundary; defer byte-span source maps until a UI feature (caret-on-error)
   actually demands them. See `active-plans/` prior discussion / the
   provenance-vs-source-map analysis.

## 3. Parser: a faithful (no-flatten) mode

Add `preserve_grouping: bool` to `TextToAstOptions` (`src/parse/text.rs`,
default `false`). When set, `convert()` skips the terminal `flatten(result)`
and returns the raw grouping. **The raw tree is analysis-only** — it violates
the "n-ary ops always flat" invariant (`src/expr.rs`) that `equals` /
`canonicalize` / smart constructors rely on, so it must never be fed into the
algebra layer. Latex parser mirrors the flag.

## 4. Node provenance: the `Prov` tag

Add owned, `Copy`-friendly provenance to the two variants that lose
information, carried **invisibly to `Eq`/`Hash`/`Ord`** so the algebra engine
is untouched (`equals` compares with `==` at `src/eq/mod.rs:88/101/178/200`;
`factor.rs` uses `HashMap<Expr,_>`; `order::cmp` sorts operands — all must stay
pure tree operations):

```rust
// src/expr.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MulStyle { Implicit, Star, Cdot, Times, Dot, Space }

/// Original literal spelling of a number, e.g. "0.50", "1/2" is NOT here
/// (that is a Div node); this captures decimal/scientific spelling only.
#[derive(Debug, Clone, Default)]
pub struct NumProv { pub literal: Option<Box<str>> } // "0.50", "6.02e23"

/// Parser provenance. Present on the faithful tree; dropped the moment a smart
/// constructor rebuilds the node (canonicalize/simplify). Invisible to the
/// derived Eq/Hash/Ord: all instances compare equal + hash identically, so
/// `Expr` equality stays pure tree equality.
#[derive(Debug, Clone, Default)]
pub struct Prov { pub num: NumProv, pub mul: Vec<MulStyle> } // one per Mul gap

impl PartialEq for Prov { fn eq(&self, _: &Self) -> bool { true } }
impl Eq for Prov {}
impl std::hash::Hash for Prov { fn hash<H: std::hash::Hasher>(&self, _: &mut H) {} }
impl PartialOrd for Prov { fn partial_cmp(&self, _: &Self) -> Option<std::cmp::Ordering> { Some(std::cmp::Ordering::Equal) } }
impl Ord for Prov { fn cmp(&self, _: &Self) -> std::cmp::Ordering { std::cmp::Ordering::Equal } }
```

Attach only to the variants that need it, as **struct-form variants** so the
~100 `Num`/~100 `Mul` *match* sites can add `..` and ignore `prov`, while only
*construction* sites supply one (`Prov::default()` everywhere except the two
parsers):

```rust
Num  { value: Number, prov: Prov },        // was Num(Number)   — 228 sites
Mul  { factors: Vec<Expr>, prov: Prov },   // was Mul(Vec<Expr>) — 103 sites
```

`n-ary` `Mul` spans `n−1` operators, so `mul` is a `Vec<MulStyle>`, not a
scalar. Provenance is meaningful **only pre-normalization**, which is exactly
where form analysis runs — the smart constructors correctly reset it.

> Churn is bounded and mechanical (measured construct/match sites: `Num` 228,
> `Mul` 103). Keep it to these two variants; do not tag the rest.

## 5. The `FormCheck` API (`src/form.rs`, new)

```rust
pub enum FormCheck {
    ReducedFraction,          // no common factor in a Div; no surd in denom
    MixedNumber,              // a + b/c shape (b<c)
    ImproperFraction,         // single Div, |num| >= |den|
    Decimal { places: Option<u32> },   // Num carries a decimal literal
    ExactValue,               // NOT a decimal literal (rational/radical/const)
    CombinedLikeTerms,        // no two combinable terms; no unreduced arith
    Expanded,                 // product-of-sums fully distributed
    FactoredCompletely,       // product of prime (over ℚ) non-monomial factors
    SingleFraction,           // exactly one top-level Div
    PartialFractions,         // sum of proper Divs, distinct denominators
    NoNegativeExponents,      // no Pow(_, negative-literal)
    RadicalSimplified,        // no radical in a denominator; radicand squarefree
    CompletedSquare,          // a(x-h)^2 + k template
    MatchesForm(Template),    // slope-intercept / vertex / standard / point-slope …
    MulStyleIs(MulStyle),     // implicit vs explicit (rare; style rubrics)
    HasIntegrationConstant,   // sum contains a free constant symbol
}

pub struct FormReport { pub ok: bool, pub why: Option<String> } // why: student feedback

/// Analyze the FAITHFUL tree (parse with preserve_grouping=true). Never
/// canonicalizes the input — that would erase the form under test.
pub fn check_form(e: &Expr, check: &FormCheck) -> FormReport;

/// Convenience: form AND value. `equals`-gate is the caller's choice.
pub fn grade(student: &Expr, key: &Expr, checks: &[FormCheck], opts: &EqOptions)
    -> Vec<FormReport>;
```

Analyzers are structural predicates over the faithful `Expr`. Reuse the
existing pattern engine (`src/js_match.rs`) for the `MatchesForm` /
`CompletedSquare` template checks rather than hand-rolling. `FactoredCompletely`
leans on `src/factor.rs` (irreducibility over ℚ) but tests the *student's*
shape — it does not replace the student tree with `factor()`'s output.
`ReducedFraction` / `RadicalSimplified` reuse `src/ratform.rs` and
`src/upoly.rs` squarefree machinery *as oracles*, applied to the written
denominator/radicand.

## 6. Directive → check mapping (grading target)

| Teacher directive (Appendix A) | `FormCheck` | Grade band |
|---|---|---|
| "simplest / lowest terms" | `ReducedFraction` | 3–5 |
| "as a mixed number" / "improper fraction" | `MixedNumber` / `ImproperFraction` | 4–5 |
| "as a decimal (to N places)" / "as a fraction" | `Decimal{places}` / `ExactValue` | 4+, AP |
| "give the exact value" / "leave in exact form" | `ExactValue` | 8, 11–univ |
| "combine like terms" / "simplify" | `CombinedLikeTerms` | 6–7 |
| "expand" | `Expanded` | 7+ |
| "factor completely" | `FactoredCompletely` | 7–12 |
| slope-intercept / standard / vertex / point-slope | `MatchesForm(_)` | 8–12 |
| "simplest radical form" / "rationalize the denominator" | `RadicalSimplified` | 8–12 |
| "write without negative exponents" | `NoNegativeExponents` | 8+ |
| "as a single fraction" / "partial fractions" | `SingleFraction` / `PartialFractions` | 9–univ |
| "complete the square" | `CompletedSquare` | 9–11 |
| indefinite integral (`+C`) | `HasIntegrationConstant` | univ |
| implicit vs explicit `×` (style rubric) | `MulStyleIs(_)` | any |

## 7. Phasing

- **F0** — `preserve_grouping` parse mode (§3). No `Prov` yet. Unblocks all
  structure-only checks. Verify existing suites stay green (algebra path
  unchanged; `tests/numeric.rs` etc.).
- **F1** — structure-only `FormCheck`s (everything except `Decimal`,
  `ExactValue`, `MulStyleIs`): `ReducedFraction`, `CombinedLikeTerms`,
  `Expanded`, `FactoredCompletely`, `SingleFraction`, `NoNegativeExponents`,
  `RadicalSimplified`, `MatchesForm`, `CompletedSquare`,
  `HasIntegrationConstant`. New `src/form.rs` + `tests/form.rs`.
- **F2** — `Prov` on `Num`/`Mul` (§4); wire `Decimal`/`ExactValue`/`MulStyleIs`.
  The churny phase — bounded to two variants.
- **F3** — wasm surface: `Expression.check_form(json)` /
  `grade(student, key, checks_json)` returning `FormReport`s
  (`src/wasm.rs`), JSON-serializable so DoenetML consumes verdicts + `why`.
- **F4 (deferred, optional)** — byte-span source maps, *iff* a UI feature needs
  caret-on-error highlighting. Not required by any sourced directive.

## 8. Open questions

- Feedback granularity: does `FormReport.why` need the offending *subtree*
  (structural pointer) rather than prose? Structural pointers are cheap on the
  faithful tree and don't need spans.
- `CombinedLikeTerms` after implicit distribution (`2(x+3)` — is it
  "unsimplified" or a legitimate factored form?) — the directive is
  problem-dependent; likely a flag on the check.
- Assumption sensitivity (STACK: `(a^x)^y=a^(xy)` only under `a>0`) — some form
  checks are context-dependent; thread `&Assumptions` where relevant.
- Do we expose per-standard presets (a "CCSSM-8" bundle vs "Ontario" bundle)
  given the divergence, or leave bundling to DoenetML?

---

## Appendix A — Sources & motivation

Compiled from a fanned-out, adversarially-verified web research pass
(71 claims confirmed / 4 refuted across ~19 sources; primary standards texts
prioritized). Motivation: establish *which* syntactic/form distinctions real
teachers and standards actually require, so the API targets real needs rather
than hypothetical ones.

**Primary — standards.**
- **CCSSM** (thecorestandards.org): `3.NF.A.3`, `4.NF.A.1` — recognize AND
  *generate* equivalent fractions via `a/b=(n·a)/(n·b)`; **no** "simplest form"
  mandate at these grades. `4.NF.C.5/6` — decimal↔fraction form (`0.62`→
  `62/100`), rewrite to a given denominator. `4.NF.B.3.b/c` — mixed↔improper.
  `7.EE.A.1` — factor/expand linear expressions (operations on *form*).
  `8.EE.A.1` — integer/negative-exponent equivalents ("without negative
  exponents"). `8.EE.A.2` — root symbols, √2 irrational (exact vs decimal).
  `8.EE.A.3` — scientific notation. `8.EE.B.6` — derive `y=mx`, `y=mx+b`
  (slope-intercept).
- **Florida B.E.S.T.** (2020): `MA.1.NSO.1.2` — standard/expanded/word form.
  `MA.8.AR.3.3` — slope-intercept mandated. `MA.8.NSO.1.4/1.5` — scientific
  notation. `MA.912.AR` — lines in standard/slope-intercept/point-slope;
  quadratics in standard/factored/vertex; polynomials factored over the reals.
  **Divergence:** "simplest form" removed — but **scoped** to specific Grade
  4–5 fraction benchmarks (`MA.4.FR.2.x`, `MA.5.NSO.2.2`, "Within this
  benchmark"), **not** a blanket policy (`MA.6.NSO.3.1` still uses it). *(An
  over-generalized "Florida never requires simplest form" claim was surfaced
  and REFUTED by the verifier against the primary PDF — corrected here.)*
- **Ontario Grades 1–8 mathematics curriculum:** caps manipulation at
  monomials deg 1 (Gr 6–7) / binomials deg 1 with integers (Gr 8); **no**
  "simplify to simplest form", "factor", "standard form", or "slope-intercept"
  directive. Equivalence framed as a value/representation task. → strongest
  evidence that form-checking must be opt-in, not global.
- **AP Precalculus scoring guidelines (College Board):** decimal answers
  correct to 3 places (round or truncate; trailing zeros optional); a distinct
  **"decimal presentation error"** category penalizes a value-correct answer
  given to too few digits; an *exact* answer where a *decimal* was requested
  earns only partial credit. → decimal-vs-exact + digit-count is a real,
  graded form distinction (motivates the `NumProv` literal tag).

**Primary — prior-art autograders (the strongest design evidence).**
- **STACK** (documentation.help / stack docs): answer tests that check *form*
  vs value — `FacForm` (factored over ℚ, handles `(2-x)(3-x)` sign/order),
  `Expanded` (`x²-(a+b)x+ab` is NOT expanded), `LowestTerms` (numbers in lowest
  terms, no surds/complex in denominator, *without* checking equivalence),
  `SingleFrac`, `PartFrac`, `CompletedSquare`. Explicit warning: "establishing
  an expression is factored is not the same as comparing it to `factor(ex)`."
  `simp:false` leaves `1+1` unevaluated to **preserve written form** (≡ our
  no-flatten mode); term order controlled via `orderless`/`ordergreat`
  (≡ our faithful operand order); `select()`/predicates inspect subexpressions.
  *(One mechanism sub-claim about `radcan` auto-normalization inside
  equivalence tests was REFUTED against STACK's own equivalence-test docs;
  omitted.)*
- **WeBWorK** (webwork.maa.org wiki + forum): default checking is **value**-
  based — an unsimplified `x+1-1` is marked correct when `x` is expected;
  `MathObjects.reduce()` is cosmetic only, "explicitly not a CAS". Form is
  enforced with opt-in strict contexts: `LimitedPolynomial-Strict` (rejects
  unsimplified), `PolynomialFactors-Strict` + `LimitedPowers::OnlyIntegers`
  (factored form; can force `k(ax+b)(ax+b)` over `k(ax+b)^2` to require
  repeated factors written out). → confirms the form/value separation and the
  opt-in model.

**Secondary — directive semantics.**
- "Factor completely" = factor until all non-monomial factors are prime, GCF
  pulled first; verify by re-multiplying (multiple teaching sources).
- "Combine like terms" = same variables AND same exponents; variable order
  irrelevant (`3xy`=`3yx`, so normalize order); may require prior distribution.
- "Rationalize the denominator" / "simplest radical form" = no radical left in
  the denominator, radicand square-free; a value-preserving *convention*.
- Indefinite integrals require a single `+C` by convention.

**Commentary.**
- **H. Wu** (math.berkeley.edu): reduced/lowest terms is a classroom
  *tradition*, "no theorem outlawing non-reduced fractions"; a narrower
  small-integer reduce convention is defensible. → "simplest form" is a
  form/convention rule, correctly modeled as an *opt-in* check, not a value law.

**Net motivation.** Real form-directives reduce to (a) structural facts on the
faithful tree + (b) two small tags (number spelling, mult glyph). No sourced
directive requires raw-input character positions; the one thing source maps
uniquely provide (caret-on-error UI) is not mandated by any standard or
autograder in the corpus. Hence §2.4 and the F4 deferral.
