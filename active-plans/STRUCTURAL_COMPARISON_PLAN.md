# Structural comparison plan (answer-structure checks for teaching)

> **STATUS (2026-07-21):** IN PROGRESS — **F0, F1 landed; F3 landed**; F2
> partially done; F4 deferred by design. All paths relative to
> `packages/math-expressions-rs/`.
>
> - **F0 (inverted-parse design, §3):** parsing is now **always faithful** —
>   `convert` drops the whole-tree `flatten` (no `preserve_grouping` flag), so
>   the raw associative grouping survives. `flatten` moved to the *leading step*
>   of the four consumers that need a canonical shape: `normalize_syntactic`,
>   the output formatters (`to_text`/`to_latex`), `js_tree::to_js`, and
>   `check_structural_comparison`. The value path (`equals`/`simplify`) already
>   flattens via `canonicalize`. Net observable behavior unchanged; 463 tests +
>   the JS differential corpora stay green.
> - **F1** (`src/equality_structural/`, `tests/structural.rs`): `StructuralComparison` +
>   `check_structural_comparison` (unary form predicate) + `structural_equality`
>   (form + value, see §5), 13 checks (`ReducedFraction`, `MixedNumber`,
>   `ImproperFraction`, `Decimal`, `ExactValue`, `CombinedLikeTerms`, `Expanded`,
>   `FactoredCompletely`, `SingleFraction`, `NoNegativeExponents`,
>   `RadicalSimplified`, `CompletedSquare`, `HasIntegrationConstant`) plus the
>   binary `SameStructure` (whole-tree identity = JS `equalsViaSyntax`, kept under its
>   `equals_syntactic` alias). **No `grade` function** — per the JS `equalsVia*`
>   model, callers compose the entry points per problem. **Vocabulary
>   reconciled** (§5): one structural framework — `equals` = value,
>   `structural_equality`/`StructuralComparison` = structural (with `SameStructure`
>   the base case), `check_structural_comparison` = unary form. 15 tests; clippy
>   clean (host + wasm).
> - **F2 (partial):** `ExactValue` and `Decimal{places:None}` shipped in F1 —
>   they need **no** tag (a faithful decimal is `Num(Rat)`, structurally
>   distinct from `Int`/`Div`). **Deferred:** the `Prov` tag itself, i.e.
>   decimal **place-counting** (`Decimal{places:Some}`) and `MulStyleIs` — the
>   only two checks that genuinely need it. Rationale: the tag requires
>   struct-variant surgery on `Num`/`Mul` (**331 construct/match sites** —
>   measured) against a green suite; poor risk/reward for two niche checks. Left
>   as a self-contained follow-up (§4 unchanged).
> - **F3** (`src/wasm.rs`): `Expression.check_structural_comparison(json)`
>   (returns a JSON `StructuralComparisonResult`) and
>   `Expression.structural_equality(key, json)` (form + value → bool).
> - **F4:** deferred by design (no sourced directive needs source spans).
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

## 3. Parser: always faithful (inverted-flatten design)

Rather than a parse-time flag, **`flatten` moves out of the parser entirely**.
`convert()` (both `src/parse/text.rs` and `src/parse/latex.rs`) no longer
applies the whole-tree `flatten`, so the raw associative grouping survives
(individual terms are still locally flattened for unit detection, and the
integrand keeps its flatten for differential extraction — both are functional
parse steps, not normalization). `flatten` becomes the **leading step of every
consumer that needs the canonical n-ary shape**:

- `norm::normalize_syntactic` (the `equalsViaSyntax`/`equals_syntactic` path),
- the output formatters `to_text`/`to_latex` (`src/output/mod.rs`),
- `js_tree::to_js` (keeps `tree_json` a flat JS AST for DoenetML),
- `check_structural_comparison` (flattens *faithfully* — merges grouping but
  keeps `Div`/`Neg`/order/spelling, never canonicalizes).

The value path (`equals`/`simplify`/`expand`/…) already flattens implicitly via
`canonicalize`, so it needs no change. Consequence audit: only these four
non-canonicalizing consumers relied on parser-flatten, all corpus-guarded, so a
missed site fails loudly. Net observable behavior is unchanged; the win is clean
layering (parse = syntax; flatten = first normalization pass) and a faithful
tree that is always available with no flag.

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

## 5. The `StructuralComparison` API (`src/equality_structural/`, new)

```rust
pub enum StructuralComparison {
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
    MatchesTemplate(Template),    // slope-intercept / vertex / standard / point-slope …
    MulStyleIs(MulStyle),     // implicit vs explicit (rare; style rubrics)
    HasIntegrationConstant,   // sum contains a free constant symbol
    SameStructure,                 // BINARY: whole-tree identity = JS equalsViaSyntax
}

pub struct StructuralComparisonResult { pub ok: bool, pub why: Option<String> } // why: student feedback

/// Unary form predicate. Flattens the input faithfully (merges grouping, keeps
/// Div/Neg/order/spelling) but NEVER canonicalizes — that would erase the form.
pub fn check_structural_comparison(e: &Expr, check: &StructuralComparison) -> StructuralComparisonResult;

/// The autograder primitive and a sibling to `equals` (JS `equalsVia*` model,
/// NOT a batch "grade"): `student` is in the form `comparison` AND value-equal
/// to `key`. Pure form → `check_structural_comparison`; pure value → `equals`.
pub fn structural_equality(student: &Expr, key: &Expr,
    comparison: &StructuralComparison, opts: &EqOptions) -> bool;
```

**Vocabulary reconciliation (value vs structural).** There is one structural
framework, not a competing "syntactic" one. `equals` = *value*.
`structural_equality` = *structural*, with a `StructuralComparison` method:
`SameStructure` is order-sensitive whole-tree identity — the JS `equalsViaSyntax`,
kept under its JS-parity name `equals_syntactic` (`equals_syntactic(a,b,o)` ==
`structural_equality(a,b,&SameStructure,o)`); every other method is a specific-form
criterion (also requiring value equality). `check_structural_comparison` is the
*unary* form predicate (no key; rejects `SameStructure`). So "syntactic equality" is
just the `SameStructure` structural comparison — no separate concept.

Analyzers are structural predicates over the faithful `Expr`. Reuse the
existing pattern engine (`src/js_match.rs`) for the `MatchesTemplate` /
`CompletedSquare` template checks rather than hand-rolling. `FactoredCompletely`
leans on `src/factor.rs` (irreducibility over ℚ) but tests the *student's*
shape — it does not replace the student tree with `factor()`'s output.
`ReducedFraction` / `RadicalSimplified` reuse `src/ratform.rs` and
`src/upoly.rs` squarefree machinery *as oracles*, applied to the written
denominator/radicand.

## 6. Directive → check mapping (grading target)

| Teacher directive (Appendix A) | `StructuralComparison` | Grade band |
|---|---|---|
| "simplest / lowest terms" | `ReducedFraction` | 3–5 |
| "as a mixed number" / "improper fraction" | `MixedNumber` / `ImproperFraction` | 4–5 |
| "as a decimal (to N places)" / "as a fraction" | `Decimal{places}` / `ExactValue` | 4+, AP |
| "give the exact value" / "leave in exact form" | `ExactValue` | 8, 11–univ |
| "combine like terms" / "simplify" | `CombinedLikeTerms` | 6–7 |
| "expand" | `Expanded` | 7+ |
| "factor completely" | `FactoredCompletely` | 7–12 |
| slope-intercept / standard / vertex / point-slope | `MatchesTemplate(_)` | 8–12 |
| "simplest radical form" / "rationalize the denominator" | `RadicalSimplified` | 8–12 |
| "write without negative exponents" | `NoNegativeExponents` | 8+ |
| "as a single fraction" / "partial fractions" | `SingleFraction` / `PartialFractions` | 9–univ |
| "complete the square" | `CompletedSquare` | 9–11 |
| indefinite integral (`+C`) | `HasIntegrationConstant` | univ |
| implicit vs explicit `×` (style rubric) | `MulStyleIs(_)` | any |

## 7. Phasing

- **F0 (done)** — inverted-parse design (§3): `flatten` removed from `convert`,
  moved to the four non-canonicalizing consumers. No `Prov` yet. Algebra path
  unchanged; all suites (incl. `tests/numeric.rs`) + JS corpora green.
- **F1 (done)** — `StructuralComparison` + `check_structural_comparison` +
  `structural_equality`: `ReducedFraction`, `MixedNumber`, `ImproperFraction`,
  `Decimal`, `ExactValue`, `CombinedLikeTerms`, `Expanded`, `FactoredCompletely`,
  `SingleFraction`, `NoNegativeExponents`, `RadicalSimplified`,
  `CompletedSquare`, `HasIntegrationConstant`. `src/equality_structural/` +
  `tests/structural.rs`. (`MatchesTemplate`/`PartialFractions` deferred.)
- **F2 (partial)** — `Prov` on `Num`/`Mul` (§4) for `Decimal{places:Some}` /
  `MulStyleIs` deferred; `ExactValue`/`Decimal{places:None}` already shipped
  tag-free in F1.
- **F3 (done)** — wasm surface: `Expression.check_structural_comparison(json)`
  and `Expression.structural_equality(key, json)` (`src/wasm.rs`),
  JSON-serializable so DoenetML consumes verdicts + `why`.
- **F4 (deferred, optional)** — byte-span source maps, *iff* a UI feature needs
  caret-on-error highlighting. Not required by any sourced directive.

## 8. Open questions

- Feedback granularity: does `StructuralComparisonResult.why` need the offending *subtree*
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
