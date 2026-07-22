# Fixture / corpus generators

These `*.mjs` scripts regenerate the JSON fixtures and differential corpora under
`../tests/fixtures/` by running the **original JavaScript library as the oracle**
and snapshotting its output. The generated fixtures are committed, so
`cargo test` never needs these scripts — re-run them only when you intentionally
change the oracle or want to extend a corpus.

## Oracle path

The old JS library now lives out-of-tree at `tmp/js-legacy/` (git-ignored, kept
on disk). These scripts therefore import it via
`../../../tmp/js-legacy/lib/...` and read specs from
`../../../tmp/js-legacy/spec/...`. A fresh clone will **not** have `tmp/js-legacy`
(it is git-ignored), so regeneration only works in a working tree that still has
the legacy JS on disk.

> **Migration:** once `packages/js-compat` (the TypeScript drop-in replacement)
> lands, switch the oracle import here to that package. The scripts then no
> longer depend on a git-ignored path, and regeneration works from a clean
> checkout.

## Wasm build

The wasm build/smoke flow moved out of this crate: the wasm-bindgen bindings now
live in the separate `math-expressions-wasm` crate at
`../../math-expressions-rs-wasm/` (`src-rust/`), whose `build-wasm.sh` compiles it
and whose `tests/` end-to-end suite (`npm test`) verifies it. This crate is a
pure Rust library.
