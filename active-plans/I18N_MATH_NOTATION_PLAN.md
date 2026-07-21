# Internationalized Math Notation Plan (decimal / argument separators)

Status: **DRAFT — not started.** Created 2026-07-21.
Upstream: [Doenet/DoenetML#1528](https://github.com/Doenet/DoenetML/issues/1528)
("Math notation localization: configurable decimal separator — requirements
for math-expressions"). Scope of *this* plan: the Rust port
(`packages/math-expressions-rs`). JS `math-expressions` is a parallel effort;
both must pass the same conformance fixtures (§6).

---

## 0. Problem

The decimal separator is hardcoded as `.` and the argument/tuple/list
separator as `,` throughout the lexer and printer. Learners in comma-decimal
locales type `3,14` and get errors or misparses. Because `,` is *load-bearing*
(function args, tuples, lists, sets, intervals, matrices), the two meanings
collide, so — per the issue — **notation must be an explicit, declared input to
every parse and print, never inferred from the string.**

Under comma-decimal notation the argument separator becomes `;`:

| Construct | `.`-notation | `,`-notation |
|---|---|---|
| function apply | `f(x,y)` | `f(x;y)` |
| point/tuple | `(1,2)` | `(1;2)` |
| list | `1,2,3` | `1;2;3` |
| decimal | `1.2` | `1,2` |

**Required now:** `.`/`,` decimal paired with `,`/`;` separators (Latin
digits). **Design-for, don't build:** Arabic `٫`/`؛`, digit grouping, non-Latin
digit sets. **Out of scope** (issue): localized function names (`sen`/`sin`),
reversed interval brackets `]a,b[`, alternate ×/÷ glyphs, repeating-decimal
notation.

## 1. Why the Rust port is well-positioned

Three of the issue's hardest requirements are already met by the architecture,
so this is a **lexer + formatter change, not a parser rewrite**:

- **A1 (no global state).** Parsers are instances built from an options struct
  (`TextToAst::new(opts)`); nothing thread-local. Notation is one more field.
- **A6 (canonical, notation-independent AST).** User input is stored as an
  **exact rational, not a string or float**: a NUMBER token flows through
  `Number::from_decimal_str` (`src/num.rs:388`) at `src/parse/text.rs:741`. So
  `1,2` and `1.2` both become `6/5` — identical ASTs, canonical storage, and
  drift-free round-trips come almost for free.
- **Grammar core is untouched.** Every separator use funnels through
  `statement_list` reading a single `Tok::Comma` (`src/parse/text.rs:168`). If
  the *lexer* maps the configured argument separator → `Tok::Comma` and folds
  the configured decimal char into number scanning, the 1200-line
  recursive-descent parser needs **zero** changes.

## 2. Decisions for you (defaults are my recommendation, first)

**A. `also_accept_decimal` leniency (issue open-Q2).** Under comma notation,
also accept `.` as a decimal?
- **A1 (recommended): strict by default** — only the declared separator; `.`
  under comma-notation is a syntax error. Lenient acceptance opt-in per parse.
- A2: lenient by default (`{'.'}`) — friendlier, but `f(1.5;2)` and ambiguous
  edge cases (`1.2,3`?) need defined behavior and more lexer states.

**B. Where the notation record lives.** A shared `NumberNotation` type used by
both parse-options and print-options structs.
- **B1 (recommended):** new `src/notation.rs` with one `NumberNotation`,
  re-exported; embedded in `TextToAstOptions`/`LatexToAstOptions`/`TextOpts`/
  `LatexOpts`.
- B2: duplicate minimal fields per struct (rejected — drift risk).

**C. Design-for fields present from day one?**
- **C1 (recommended):** include `group_separator`, `grouping`, `digits` in the
  struct now (Latin/none defaults, Phase-2 implementation), so the wire/JSON
  shape is stable and matches the JS side.
- C2: add them only in Phase 2 (smaller now, but a breaking option-shape change
  later).

## 3. The config record (Phase 1 shape)

```rust
// src/notation.rs
pub struct NumberNotation {
    pub decimal_separator: char,      // '.' | ',' | '٫'      (default '.')
    pub argument_separator: char,     // ',' | ';' | '؛'      (default ',')
    pub also_accept_decimal: Option<Vec<char>>, // lenient input (default None)
    // ---- design-for (Phase 2; Latin/none defaults keep A2 no-op) ----
    pub group_separator: Option<char>,          // thousands grouping
    pub grouping: Grouping,                      // Western(3) | Indian(2-2-3) | None
    pub digits: Digits,                          // Latin | Arabic | Devanagari
}
impl Default for NumberNotation { /* '.'/','/None/None/None/Latin — byte-identical to today */ }
```

**A2 invariant:** at `Default`, decimal `.` and separator `,` and Latin digits,
so every stage is a literal no-op and existing fixture output is byte-identical.
This is asserted in Phase 1 by re-running the *current* corpus under the default
notation and diffing to zero.

## 4. Phase 1 — required-now (`.`/`,` ↔ `,`/`;`, Latin)

Each step keeps `cargo test` + clippy green.

### 4.1 Config + threading
- [ ] `src/notation.rs`: `NumberNotation` (+ `Grouping`, `Digits` enums, Phase-2
      variants stubbed). `pub use` at crate root.
- [ ] Embed in `TextToAstOptions`, `LatexToAstOptions` (parse) and `TextOpts`,
      `LatexOpts` (print). Defaults unchanged ⇒ A2.

### 4.2 Lexer (`src/parse/lexer.rs`)
- [ ] `scan_number` (L870): replace hardcoded `b'.'` (L877, L883) with
      `decimal_separator`; replace `is_ascii_digit()` with a `digit_class`
      predicate (Latin now).
- [ ] Token table (`l(",", Tok::Comma)`, L515): build from notation — emit
      `Tok::Comma` for the configured **argument** separator; never tokenize the
      decimal char as a separator.
- [ ] `sci_delim_ok` (L848): the post-exponent delimiter set hardcodes `,` —
      use the argument separator instead.
- [ ] Thread notation into `Lexer` construction (it already carries `flavor` /
      `sci_notation` state).

### 4.3 Token → number
- [ ] `Number::from_decimal_str` (`src/num.rs:388`) and `parse_js_float`
      (`src/parse/common.rs:96`, used for Leibniz/derivative-order tokens):
      normalize the configured decimal char to `.` before interpreting.

### 4.4 Printing
- [ ] `render_number` in `src/output/text.rs:182` **and**
      `src/output/latex.rs:112`: swap `.` → `decimal_separator`; same for the
      positional-float path (`f64_positional_string`).
- [ ] **A5**: LaTeX must emit a decimal comma as `{,}` (else MathJax/MathQuill
      apply trailing punctuation thin-space). Applies to decimals and floats.

### 4.5 wasm boundary (`src/wasm.rs`)
- [ ] Thread notation into `parse_text` / `parse_latex` / `to_text` / `to_latex`
      (extend the existing `parse_text_with_options` JSON entry point; add the
      notation record to the printed-output options).

## 5. Phase 2 — design-for (not required now)
- [ ] Digit sets: `digit_class` + `digit_value(char)` for Arabic-Indic /
      Devanagari; printer emits the configured digit set.
- [ ] Group separators + `Grouping` (Western 3, Indian 2-2-3) in the printer;
      the parser tolerates/ignores group separators on input.
- [ ] Arabic `٫` decimal + `؛` argument separator (config already accepts them;
      just exercise + fixture).

## 6. Conformance fixtures (the cross-impl handoff — do alongside Phase 1)

The issue's critical deliverable: a **language-neutral, versioned JSON fixture**
that both JS and Rust pass.

- [ ] Schema (versioned): rows of
      `{ notation, input_text, input_latex, expected_ast, expected_text_out,
      expected_latex_out }`, plus a `failing` list of `(notation, input)` that
      must error, per notation.
- [ ] Location: `tests/fixtures/notation/*.json` (mirrors the existing
      differential-fixture layout; `scripts/extract-fixtures.mjs` pattern).
- [ ] New `tests/notation.rs`: for each row, parse under `notation` → compare
      AST (via `js_tree`) → print text/latex under `notation` → compare;
      assert `failing` rows error.
- [ ] Round-trip law (A6): `parse(print(ast, N), N) == ast`; notation
      independence: `parse(t, comma_N) == parse(equiv_t, period_N)`.

## 7. Risks
- **A2 regressions** — the whole point is byte-identical defaults; guard with
  the existing corpus re-run at default notation (diff = 0) in CI.
- **Leniency states** (decision A) can balloon lexer complexity — keep strict
  default.
- **`sci_delim_ok` / `,` entanglement** — the exponent-delimiter set and the
  argument separator are the same character today; splitting them cleanly is the
  subtle lexer bit.
- Group-separator input parsing (Phase 2) reintroduces ambiguity with the
  argument separator in some locales — defer, and require the declared notation
  to disambiguate.

## 8. Out of scope (per the issue, restated so it is a decision not an omission)
Localized function names, reversed interval brackets, alternate multiplication/
division glyphs, repeating-decimal notation. The parser's function/operator
tables are **not** localized by this plan.
