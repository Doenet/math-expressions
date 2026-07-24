# Design Questions

Running log of behaviours that are **decided and implemented** but arguable —
places where the Rust port made a judgement call worth revisiting. Each entry
records the current behaviour, why it is the way it is, and the open question so
a reviewer can confirm or change it. Add new entries at the top.

---

## Q1. Sign symmetry of `±` under negation and rendering

**Area:** `pm` (plus-minus) — `norm` (canonicalize/simplify) + `output` renderers.

### Current behaviour

`±x` denotes the value set `{x, −x}`, which is closed under negation. The port
uses that symmetry:

- **Negation is absorbed:** `−(±x)` canonicalizes to `±x` (the outer sign is
  dropped, not distributed). It does **not** become `−±x` / `-+x`.

  | input | `simplify` (unicode) | `simplify` (ascii) | raw, un-canonicalized |
  |---|---|---|---|
  | `−(±x)` | `± x` | `+- x` | `-(± x)` / `-(+- x)` |

  The raw form keeps parentheses (`Neg` always parenthesizes its operand), so
  even without simplification there is never a bare `-+x` collision.

- **Scaling pulls a scalar inside:** `2·±x → ±(2x)`, since `2·{x,−x} = {2x,−2x}`.
  Guarded to fire only when exactly one factor is a `±` and the rest carry none.

- **Independent `±` are never merged:** `±3 + ±3` stays `±3 + ±3` (value set
  `{6, 0, −6}`), and is *not* collapsed to `2·±3` (`{6, −6}`). This is the
  meaning-preservation guard in the `add` constructor.

### Why

The simplifier's oracle is *meaning-preserving + reduced*, **not** tree-match to
the JS reference (per `norm/simplify.rs` module docs). `−(±x) → ±x` and
`2·±x → ±(2x)` are both meaning-preserving and strictly more reduced, so they are
applied at the canonical level — which also lets `equals` settle
`equals(−(±x), ±x)` structurally at stage 1 instead of by numeric sampling.

### Deviations from JS (accepted)

JS applies `−(±x) → ±x` and `c·±x → ±(c·x)` as *simplify* transformations but is
deliberately conservative elsewhere; two consequences of doing this at the
canonical level in Rust differ from JS tree conventions:

1. **`±(−3)` vs `±3`.** JS keeps these as distinct trees ("no pm-of-negative
   rule"); the Rust negation rule only strips an *outer* `Neg` (`−(±x)`), it does
   **not** normalize a `Neg`/negative literal *inside* the `±` — so Rust also
   keeps `±(−3)` and `±3` distinct. Matches JS here. **Open question:** should we
   go further and canonicalize the `±` argument to a positive-leading
   representative (`±(−3) → ±3`)? It would be sound (same value set) and more
   reduced, but is a larger deviation from JS and unnecessary for `equals`
   (which already treats them equal numerically).

2. **Raw `5 − (±x)`.** The un-simplified tree `["+", 5, ["-", ["pm","x"]]]`
   renders `5 - ± x` (a spaced, unambiguous but slightly ugly `- ±` adjacency).
   `simplify` collapses it to `5 ± x`. **Open question:** should the `Add`
   renderer special-case a *negated* `±` operand (`Neg(pm(...))`) the way it
   already suppresses the `+` before a bare `±`, so the raw form also reads
   `5 ± x` without needing simplification? Currently only a bare `±` term gets
   the connective-suppression treatment.

### Status

Implemented and tested (`tests/pm.rs`, `src/pm.rs`, `src/norm/{mod,expand}.rs`,
`src/output/{latex,text}.rs`). The open questions above do not affect
correctness (`equals` is unaffected); they are cosmetic/canonical-form choices.
