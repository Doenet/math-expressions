# Internationalized Math Notation Plan (decimal / argument separators)

Status: **Phase 1 COMPLETE** (Rust port) — 2026-07-21. Decisions taken: **A1**
(strict by default, leniency opt-in via `also_accept_decimal`), **B1** (one
`src/notation.rs::NumberNotation`, re-exported, embedded in all four
option structs), **C1** (Phase-2 `group_separator`/`grouping`/`digits` fields
present now). Conformance fixture + suite (`tests/notation.rs`,
`tests/fixtures/notation/phase1.json`) green; full suite 482 pass, clippy clean.
Phase 2 (digit sets, grouping) remains — but the char-generic lexer/printer
already make the Arabic `٫`/`؛` **separators** work today (only non-Latin
digit-set translation and thousands grouping are unimplemented).
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

**Resolved during implementation — partial specification & ambiguity.** At the
wasm/JSON boundary, notation keys may be given individually (unspecified ones
keep their defaults). For convenience the two conventional pairs auto-complete
from the decimal alone: `decimalSeparator:"."` ⇒ argument `,`,
`decimalSeparator:","` ⇒ argument `;` (`NumberNotation::from_decimal` /
`paired_argument_separator`). An **explicit** incoherent pair (e.g. decimal and
argument both `,`, or a digit/letter separator) is a returned **error**
(`NumberNotation::validate` → `JsError`), never a panic or a silent misparse.

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
- [x] `src/notation.rs`: `NumberNotation` (+ `Grouping`, `Digits` enums, Phase-2
      variants stubbed) with `accepted_decimals`/`normalize_number` helpers.
      `pub use` at crate root.
- [x] Embed in `TextToAstOptions`, `LatexToAstOptions` (parse) and `TextOpts`,
      `LatexOpts` (print). Defaults unchanged ⇒ A2.

### 4.2 Lexer (`src/parse/lexer.rs`)
- [x] `scan_number`: hardcoded `b'.'` replaced by a `decimal_at` helper over the
      accepted decimal chars (char-generic, so multibyte Arabic works); Latin
      digits.
- [x] Token table: the static `l(",", Tok::Comma)` rules (text + LaTeX) were
      **removed**; `advance` emits `Tok::Comma` for the configured **argument**
      separator via a notation-aware intercept, and the decimal char is never a
      separator on its own.
- [x] `sci_delim_ok`: post-exponent delimiter now takes the argument separator
      instead of a hardcoded `,`.
- [x] Notation threaded into `Lexer::new`/`new_latex` (stored on `Lexer`).

### 4.3 Token → number
- [x] Normalization done at the parser call sites via
      `notation.normalize_number` (accepted decimals → `.`, group separators and
      the LaTeX `{,}` braces stripped) before `Number::from_decimal_str` /
      `parse_js_float` — keeps those helpers notation-agnostic.

### 4.4 Printing
- [x] `render_number` in `src/output/text.rs` **and** `src/output/latex.rs`:
      `.` → `decimal_separator` (also the positional-float path).
- [x] **A5**: LaTeX emits a decimal comma as `{,}`; the LaTeX number scanner was
      taught to read `{,}` back, so the round-trip law holds.
- [x] **Beyond §4.4 (required for the §6 round-trip law):** the **output**
      argument separator is also switched — function args, tuples/lists/sets/
      vectors, intervals, `linesegment`, and `\operatorname`/`\angle` args now
      join on the configured separator, else a printed `(1,2)` would re-parse as
      the decimal `1.2` under comma notation. (Text matrices stay `,` — they are
      display-only, not parseable.)

### 4.5 wasm boundary (`crate/src/parse.rs`, `core_ops.rs`)
- [x] `read_notation` reads a `notation` JSON sub-object into the parse options
      (`parse_text_with_options` / `parse_latex_with_options`); new
      `to_text_with_options` / `to_latex_with_options` thread it into printing.
      (The wasm surface moved to the sibling `math-expressions-wasm` crate; the
      plan's `src/wasm.rs` path is stale.)

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

- [x] Schema (versioned, `version: 1`): rows of
      `{ name, notation, input_text?, input_latex?, expected_ast,
      expected_text_out?, expected_latex_out? }`, plus a `failing` list of
      `(notation, input)` that must error.
- [x] Location: `tests/fixtures/notation/phase1.json`.
- [x] `tests/notation.rs`: each row parses under `notation`, compares AST (via
      `js_tree`), prints text/latex under `notation` and compares; `failing`
      rows asserted to error.
- [x] Round-trip law (A6): `parse(print(ast, N), N) == ast` (text + latex);
      notation independence: `parse(t, comma_N) == parse(equiv_t, period_N)`.

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
