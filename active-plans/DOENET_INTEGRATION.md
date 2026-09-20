# math-expressions: what DoenetML still needs

**For:** maintainers of [`Doenet/math-expressions`](https://github.com/Doenet/math-expressions)
**Against:** `siefkenj/math-expressions@doenet`, `970c1c3`
**Date:** 2026-08-04

Each item below is self-contained enough to file as an issue, and carries the response it got
inline. Nothing that has already been fixed is repeated here — see the git history of this file if
you want the record of what was.

DoenetML has switched permanently to the Rust engine. There is no JavaScript engine to fall back to,
so everything below is on the path to shipping.

## Filed — three items

As filed. The [Response](#response) below resolves them: **1** is fixed, **2** is answered as not a
defect, and **3** is the only one still open.

**1. Display rounding loses precision at large magnitudes**
— 8 failures, and it is the *normal* display path rather than an edge case.

```js
me.round_numbers_to_precision_plus_decimals(2e21, 3, 2).tree;  // → 1.9999999999999997e+21
me.round_numbers_to_precision(2e21, 3).tree;                   // → 2e+21   ✓ digits alone is exact
```

`<number>` defaults to `displayDigits = 3`, `displayDecimals = 2`, so every large number a student
sees goes through the broken combination. Rounding `2e21` to 3 significant figures is `2.00e21`,
which is exactly representable. The value is parsed exactly and survives untouched until this step —
only the rounding corrupts it, and asking for *more* digits eventually returns the exact answer,
which points at a decimal-string round trip.

**2. `parseScientificNotation` has no effect**
— low severity, but a documented option that silently does nothing.

```js
new me.converters.textToAstObj({ parseScientificNotation: true }).convert("7e-12");
// → ["+",["*",7,"e"],-12]     expected 7e-12
```

Either honour it or drop it from the option list in `lib/converters/text-to-ast.ts`.

**3. WASM32 stack safety** — a crash class reachable
from student input, and already your own `STACK_SAFETY_PLAN.md`. Deep expressions can overflow the
~1 MB shadow stack, including on `Drop`, and the input arrives from a text box. Steps 1 and 2 of your
plan — iterative `Drop`, parser depth cap — close the vector end-to-end.

Items 1 and 2 are new in this revision and were found while auditing what we had assumed were our own
failures; see the note below. The two items we had open before are fixed in `970c1c3`:
`evaluate_to_constant()` now reports ±Infinity rather than a "no value" marker, and the printer implements the
ECMAScript scientific-notation threshold with `avoidScientificNotation` honored.

We also filed one of these wrongly and want that on the record: we claimed `panic = "abort"` was why
wasm panics reached us as a bare `unreachable`. It was not — std runs the panic hook before aborting;
what was missing was a hook at all, since the default writes to a stderr that goes nowhere on
wasm32-unknown-unknown. You installed one for 1,958 bytes and the diagnosability problem is gone.

## Where we are

`packages/doenetml-worker-javascript`: **344 failures of 3,436 executed — 90.0% passing.**

Four pins in a row of progress with no regressions: `cdc5343` → `02293bf` fixed 36 tests,
`02293bf` → `08bd4dc` fixed 10, `08bd4dc` → `970c1c3` fixed 59. None broke anything. `02293bf` also
let us delete the last two workarounds in our seam — with **no change in results either way**, which
is how we verify an upstream fix actually covers our usage. `packages/math/src/engine-rust.ts` is now
a straight re-export.

Most of what was left was ours: 61 coordinate/array mismatches from a bug in our own dependency
resolution, 16 unattributed `matchesPattern` cases, 15 blank-comparison scoring failures in our
`booleanLogic.js`, 12 tagged-value leaks into `.tree` consumers, and 5 residual `unexpected value
null` call sites. 8 were item 1 above.

**These counts are stale.** They were measured at pin `970c1c3`; the branch is several revisions past
it and most of the clusters above have since been closed from one side or the other. Treat them as
the shape of the work, not as current numbers, and re-measure before citing any of them.

### How items 1 and 2 were found

Worth recording because the pattern has now repeated: we had classified the scientific-notation
cluster as ours — the engine implements the threshold correctly, so our expectations looked like the
thing that was out of date. Instrumenting the component instead showed the parsed `value` was exactly
`2e21` and only `valueForDisplay` was wrong, which took three probes to narrow from "our expectations"
to a one-line call. Item 2 turned up in the same pass, after we spent a while assuming our own call
site was at fault before testing the flag in isolation.

The general lesson, for both of us: a cluster that looks like stale test expectations is worth one
boundary probe before it is written off, and the probe should print with `String()` rather than
`JSON.stringify` — `JSON.stringify(Infinity)` is `"null"`, which already cost us one wrong report and
nearly cost us a missed fix.

## Reproducing

```bash
git submodule update --init --recursive          # vendor/math-expressions @ 970c1c3
npm run build -w packages/math
cd packages/doenetml-worker-javascript
npx vitest run -t '@group1'                      # and @group2, @group3
npx vitest run -t '^(?!.*@(?:group1|group2|group3))'   # group4
```

Every engine-level claim is reproducible in isolation, without DoenetML:

```js
import me from "math-expressions";
console.log(String(me.fromAst(-Infinity).evaluate_to_constant()));
```

That form is what moved nine items off this list and onto ours — printed with `String()`, not
`JSON.stringify`, which renders `Infinity` as `null` and cost us a wrong report.

## Response

**Item 1 — display rounding: fixed.** Your read was right down to the mechanism. The float branch of
`Number::round_to_decimals` computed `(v · 10^d).round() / 10^d`, and both steps round: `2e23` is not
representable (`5^23 > 2^53`), so `2e21 · 100` was already wrong before the division made it worse.
Exact values never took that branch, which is why a `<number>` with a typed literal was fine and a
computed one was not, and why asking for more digits recovered — a larger `digits` drives `d`
negative, and `10^-19` happened to survive the round trip.

Rounding now goes through the float's exact binary value (`BigRational::from_float` is lossless),
rounds there, and returns the nearest f64 to the decimal that produces. That is what legacy was doing
with `parseFloat(math.format(v, {notation: "fixed", precision: n}))` — the decimal-string round trip
you inferred. Ties are unchanged (away from zero, resolved against the stored value, so `2.675` → 
`2.67`).

The differential corpus generated from your engine now matches **bit-exactly** on
`round_numbers_to_precision_plus_decimals`; it previously needed a `1e-12` relative tolerance, which
we have removed so the next divergence of this kind cannot hide under it.

**Item 2 — `parseScientificNotation`: not a defect; your repro reads a different case.** The option is
honored, and always was — but only for **uppercase `E`**, and only when the exponent ends the
expression or is followed by `, | ) } ]`. Both restrictions are legacy's, and the lowercase one is
load-bearing rather than an oversight: `e` is Euler's number in this grammar, so `1.2e-3` is
`1.2·e − 3`. Your own spec asserts exactly that (`spec/quick_text-to-ast.spec.js`, and it is still
asserted in our port), so we cannot widen the rule without breaking it.

```js
new me.converters.textToAstObj({ parseScientificNotation: true }).convert("7E-12");  // → 7e-12   ✓
new me.converters.textToAstObj({ parseScientificNotation: false }).convert("7E-12"); // → ["+",["*",7,"E"],-12]
new me.converters.textToAstObj({ parseScientificNotation: true }).convert("7e-12");  // → ["+",["*",7,"e"],-12]  (Euler)
```

So: neither honour-differently nor drop. What we have done instead is make the rule impossible to
misread — it is now stated on the option itself in both parsers, and pinned from both sides
(uppercase honored, lowercase not, delimiter required) in `tests/parser_options.rs` and
`spec/quick_doenet_display_rounding.spec.ts`.

That leaves the real question yours: students type `7e-12`. If DoenetML wants that to parse as
scientific notation, say so and we will add it as an explicit opt-in (a distinct value of the option,
so `1.2e-3` keeps its legacy meaning by default). We did not add it unilaterally because it changes a
grammar decision your own corpus depends on.

**Item 3 — stack safety: still open, and your summary of it is slightly optimistic.** Step 1
(iterative `Drop`) and step 2 (parser depth cap) are done, but they do not close the vector
end-to-end. The cap bounds *parser-produced* trees only, and `tear_down` runs at exactly one call
site — the wasm handle's `Drop` — so it covers neither the ~90 recursive traversals in the core nor
the intermediate trees those passes drop internally. Measured on a 1 MB stack, release profile, the
weakest passes (`flatten`, `serde::to_js`, `to_text`, `canonicalize`) trap at roughly **1,800 levels**;
in a debug build `to_js` traps at **126**, which is below the 128 that `from_js` will admit.

Two vectors are reachable from your side today and are not behind the parser cap:

- `unflatten_left` / `unflatten_right` turn width into depth with no bound. `["+", a1, …, a100000]`
  is depth 2 as JSON — serde_json's 128-deep limit never fires — and becomes a 100,000-deep tree that
  is then serialized and dropped recursively.
- `substitute_var` composes: `e = e.substitute_var("x", e)` **doubles** depth per call, so a dozen
  calls from a JS loop clears 1,800.

We are taking the cheap half first — explicit caps on `try_from_js` and `unflatten_*`, and small-stack
tests for the four heavy passes — rather than blocking on the full iterative-fold port (steps 3–5),
which is the larger piece of work. Flagging the two vectors above in case either is on a path you
already exercise.

**Verification.** Rust: 661 tests, 0 failing; clippy clean. The js-compat differential across 6,289
tests is unchanged at 1,384 failures — no regressions from either change, and the 7 new boundary tests
all pass. The rounding fix has no local test that was failing before, because the case only shows up
at your display path; `spec/quick_doenet_display_rounding.spec.ts` now covers it at the library
boundary in the form you filed it.

**One note on your remaining clusters.** "12 tagged-value leaks into `.tree` consumers" may be ours,
not yours: `me.round_numbers_to_decimals(-Infinity, 2).tree` is `{$: "-Inf"}`, where legacy gave
`-Infinity`. The tag is how non-finite values cross the wasm boundary, and `evaluate_to_constant()`
untags on the way back, but `.tree` does not. If those 12 are that shape, send one and we will take
it.

**Resolved — `.tree` now untags.** It was ours. `.tree` hands back `Infinity`, `-Infinity` and `NaN`
as JS scalars, matching legacy and what `typeof x === "number"` consumers test. The *wire* format
stays tagged in both directions, because JSON cannot hold those three values; the replacer re-tags on
the way in, so `fromAst(x).tree` is still a fixpoint — it just holds at the value level rather than
the wire level. `{$: "None"}` is the one exception in both directions: it has no JS scalar to become,
and you already emit and read it in that form.

Two consequences worth flagging, since they change what a caller sees:

- `me.utils.flatten` / `unflattenLeft` / `unflattenRight` / `match` take these untagged trees now, so
  a `.tree` value can be fed straight back into them. Previously they went through a bare
  `JSON.stringify`, which writes `null` for a non-finite — a silently wrong tree rather than an error.
- `evaluate_to_constant()` answers `NaN` — never `null` — whenever there is no numeric value, which
  is legacy's contract restored. It briefly distinguished the two, `NaN` for an indeterminate form
  and `null` for something genuinely undecided such as a free variable. That distinction is real but
  `null` is the wrong way to carry it: it coerces to `0`, satisfies `<=`, and slips past
  `Number.isNaN`, so an expression with no value read as a real one everywhere the consumer had not
  been individually taught otherwise. A caller that wants the distinction can ask `variables()`.
