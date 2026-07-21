#!/usr/bin/env bash
# Build the wasm-binding crate to a *nodejs-target* wasm-bindgen package for
# js-compat. The nodejs target instantiates the wasm synchronously at require()
# time, which preserves the original math-expressions synchronous JS API
# (me.fromText(...).equals(...) with no await) and works directly under vitest's
# node runner.
#
# Output lands in ./vendor/wasm (git-ignored). Kept separate from the wasm
# package's shared pkg/ (which the playground builds with --target web) to avoid
# clobber.
#
# Requires: rustup target add wasm32-unknown-unknown + wasm-bindgen-cli matching
# the wasm-bindgen crate version in Cargo.toml.
set -euo pipefail
cd "$(dirname "$0")"

TARGET_DIR="$(cargo metadata --no-deps --format-version 1 \
  | node -e 'process.stdout.write(JSON.parse(require("fs").readFileSync(0)).target_directory)')"

cargo build -p math-expressions-wasm --target wasm32-unknown-unknown --release
wasm-bindgen "$TARGET_DIR/wasm32-unknown-unknown/release/math_expressions_wasm.wasm" \
  --out-dir vendor/wasm --target nodejs
# wasm-bindgen emits `export class` (ESM syntax) even for --target nodejs on this
# toolchain; mark the dir CommonJS so Node loads it as CJS and the generated
# `${__dirname}/…_bg.wasm` lookup resolves. Without this it is parsed as ESM
# (repo root is "type":"module") and the wasm path breaks.
echo '{"type":"commonjs"}' > vendor/wasm/package.json
echo "Built js-compat wasm (nodejs target) into vendor/wasm"
