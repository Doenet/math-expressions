#!/usr/bin/env bash
# Rebuild the Rust implementation to a browser-targeted wasm-bindgen package in
# the crate's own pkg/ directory. The playground resolves that location and
# static-copies the assets at serve/build time (see vite.config.js), so nothing
# is vendored into the playground source tree. Run after changing
# math-expressions-rs.
# Requires: rustup target add wasm32-unknown-unknown, and wasm-bindgen-cli
# matching the wasm-bindgen crate version in Cargo.toml.
set -euo pipefail
cd "$(dirname "$0")/../math-expressions-rs"

TARGET_DIR="$(cargo metadata --no-deps --format-version 1 \
  | node -e 'process.stdout.write(JSON.parse(require("fs").readFileSync(0)).target_directory)')"

cargo build --target wasm32-unknown-unknown --release
wasm-bindgen "$TARGET_DIR/wasm32-unknown-unknown/release/math_expressions.wasm" \
  --out-dir pkg --target web
echo "Rebuilt math-expressions-rs/pkg (web target) for the playground."
