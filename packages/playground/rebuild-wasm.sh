#!/usr/bin/env bash
# Build the browser-targeted (--target web) wasm-bindgen package the playground
# static-copies at serve/build time (see vite.config.ts). Delegates to the
# single source-of-truth build script in math-expressions-rs-wasm; the web build
# lands in that package's pkg/ (git-ignored). Run after changing the Rust core
# or its wasm bindings.
# Requires: rustup target add wasm32-unknown-unknown, and wasm-bindgen-cli
# matching the wasm-bindgen crate version in Cargo.toml.
set -euo pipefail
exec bash "$(dirname "$0")/../math-expressions-rs-wasm/build-wasm.sh" web
