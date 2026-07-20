#!/usr/bin/env bash
# Build the wasm-bindgen package and run the JS smoke test (PORTING_PLAN.md §13).
# Requires: rustup target add wasm32-unknown-unknown, and wasm-bindgen-cli
# matching the wasm-bindgen crate version in Cargo.toml.
set -euo pipefail
cd "$(dirname "$0")/.."

TARGET_DIR="$(cargo metadata --no-deps --format-version 1 | node -e 'process.stdout.write(JSON.parse(require("fs").readFileSync(0)).target_directory)')"
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen "$TARGET_DIR/wasm32-unknown-unknown/release/math_expressions.wasm" \
  --out-dir pkg --target nodejs
# Optional post-link shrink (~10-15% on top of the release profile). Skipped
# when wasm-opt (from binaryen) is not installed.
if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Oz pkg/math_expressions_bg.wasm -o pkg/math_expressions_bg.wasm
else
  echo "note: wasm-opt not found; skipping post-link -Oz pass"
fi
echo '{"type":"commonjs"}' > pkg/package.json
cp scripts/wasm-smoke.cjs pkg/smoke.cjs
node pkg/smoke.cjs
