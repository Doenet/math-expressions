#!/usr/bin/env bash
# Build the *nodejs-target* wasm-bindgen package for js-compat into ./vendor/wasm
# (git-ignored). The nodejs target instantiates the wasm synchronously at
# require() time — no async init — so the original synchronous math-expressions
# API (me.fromText(...).equals(...), no await) works under vitest's node runner.
# Delegates to the single source-of-truth build script in math-expressions-rs-wasm;
# the vendor dir is kept separate from that package's shared pkg/ (which the
# playground builds with --target web) to avoid clobber.
# Requires: rustup target add wasm32-unknown-unknown + wasm-bindgen-cli matching
# the wasm-bindgen crate version in Cargo.toml.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
exec bash "$HERE/../math-expressions-rs-wasm/crate/build-wasm.sh" nodejs "$HERE/vendor/wasm"
