# Numeric root simplification — settled spec (item 9)

**Status: resolved and implemented (2026-08-05).** This replaces the earlier
"questions" note; the convention below was confirmed by the maintainer and is
what the code now does.

## The rule

A **number** under a root folds; a **variable** radicand never folds.

When a numeric radicand folds, use the **real** root if one exists, otherwise
the correct **principal complex** root — folding only when the principal value
is exactly representable, never falling back to a float.

This needs no assumptions engine and introduces no `abs`: the only cases that
fold have a *numeric* radicand, whose sign is known.

## What that produces

| input | real root? | result | status |
| --- | --- | --- | --- |
| `cbrt(-8)`, `nthroot(-8,3)`, `(-8)^(1/3)`, `nthroot(-32,5)` | yes | `-2` | unchanged |
| `sqrt(8)` | yes | `2·sqrt(2)` | unchanged (perfect-power extraction) |
| `sqrt(-1)` | no | `i` | **new** |
| `sqrt(-4)` | no | `2i` | **new** |
| `sqrt(-2)` | no | `i·sqrt(2)` | **new** |
| `sqrt(-8)` | no | `2·i·sqrt(2)` | **new** |
| `(-4)^(1/2)` | no | `2i` | **new** |
| `sqrt(x^2)`, `cbrt(x^3)`, `sqrt(16x²y⁴)` | — (variable) | unchanged | never folds |

`sqrt(16x²y⁴) → 4·sqrt(x²y⁴)` was already correct under this rule — the numeric
factor comes out, the variable factors stay under the radical.

## Scope / what is deferred

**Higher even roots of a negative number** (`(-16)^(1/4)`) are the principal
complex value `|r|^(1/q)·(cos(π/q)+i·sin(π/q))` — exact only when the angle lands
on the engine's surd lattice (`(-16)^(1/4) = √2(1+i)`, π/4 is on it) and a nested
radical otherwise (`(-1)^(1/5)`, `sin 36°` is off it). We **leave all higher
even roots symbolic for now** rather than fold some and not others; the square
root (q = 2) is always exact (`sqrt(-c) = sqrt(c)·i`) and is what the reported
cases needed. Building the general lattice form for roots is the follow-up if a
consumer needs it.

## Where it lives

- [`normalize/simplify.rs`](../packages/math-expressions-rs/src/normalize/simplify.rs) —
  `simplify_root` (the `sqrt`/`cbrt`/`nthroot` application form) and
  `fold_numeric_radical` (the `b^(p/q)` power form) each gained the
  negative-even-root branch; `principal_imaginary_sqrt` builds `m·i·sqrt(r)`.

## Why grading was never at stake — for the *even*-root cases

`equals` already evaluated all of the even-root cases above on the principal
complex branch, so it answered `sqrt(-4) == 2i` **true** before this change;
for those rows this was only ever a `simplify` / `.tree` display gap.

**Addendum (2026-08-14, eleventh review pass):** for *odd* roots of negatives
grading **was** at stake, in exactly the gap this spec left: the perfect-power
rows above folded real while a non-perfect radicand (`(-2)^(1/3)`) still
*evaluated* principal, so the branch depended on whether the radicand was a
perfect power and four DoenetML `<answer>` cases regressed against legacy. The
real-branch rule now extends to the evaluators: `rule_radical`'s `Pow` arm
pulls the sign out of a non-perfect odd root at simplify time
(`(-2)^(1/3) → -2^(1/3)`), and `eval_complex`, `CBRT::eval1` and
`NTHROOT::eval2` read `(negative real)^(1/odd)` on the real branch for
sampling. Even roots stay exactly as this spec settled them. See
`active-plans/PR84_REVIEW_KNOWN_ISSUES.md` ("Fixed during review") and
`tests/odd_root_real_branch.rs`.

## Verification

- Rust: full suite green (658 passing), clippy clean. Tests in
  `tests/doenet_open_items.rs` (`sqrt_of_negative_folds_to_principal_imaginary`,
  `prefer_a_real_root_when_one_exists`, `a_variable_radicand_never_folds`,
  `higher_even_root_of_a_negative_stays_symbolic_for_now`).
- js-compat differential: **zero change** (`4942 passed / 1353 failed` before and
  after), zero regressions — the fold is behaviour-preserving on everything the
  legacy corpus exercises. End-to-end through wasm in
  `spec/quick_doenet_printer_and_rounding.spec.ts`.
