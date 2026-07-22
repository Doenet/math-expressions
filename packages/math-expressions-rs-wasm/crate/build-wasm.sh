#!/usr/bin/env bash
# Build the wasm-binding crate with wasm-bindgen. Single source of truth for the
# wasm build — the playground and js-compat call this with different targets
# rather than carrying their own copies.
#
# Usage: build-wasm.sh [target] [out-dir]
#   target  : nodejs (default) | web        -- wasm-bindgen --target
#   out-dir : default ../pkg                 -- where wasm-bindgen writes.
#             Relative paths resolve against this script's dir (crate/); pass an
#             absolute path to write elsewhere (js-compat does).
#
# For the nodejs target the output dir also gets a {"type":"commonjs"} marker
# (so Node require() resolves the sibling _bg.wasm) and the JS smoke test runs.
#
# Requires: rustup target add wasm32-unknown-unknown, and wasm-bindgen-cli
# matching the wasm-bindgen crate version in Cargo.toml.
set -euo pipefail
cd "$(dirname "$0")"

TARGET="${1:-nodejs}"
OUT_DIR="${2:-../pkg}"

TARGET_DIR="$(cargo metadata --no-deps --format-version 1 | node -e 'process.stdout.write(JSON.parse(require("fs").readFileSync(0)).target_directory)')"
cargo build -p math-expressions-wasm --target wasm32-unknown-unknown --release
wasm-bindgen "$TARGET_DIR/wasm32-unknown-unknown/release/math_expressions_wasm.wasm" \
  --out-dir "$OUT_DIR" --target "$TARGET"

# Optional post-link shrink (~10-15% on top of the release profile). Skipped
# when wasm-opt (from binaryen) is not installed.
if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Oz "$OUT_DIR/math_expressions_wasm_bg.wasm" -o "$OUT_DIR/math_expressions_wasm_bg.wasm"
else
  echo "note: wasm-opt not found; skipping post-link -Oz pass"
fi

if [ "$TARGET" = "nodejs" ]; then
  echo '{"type":"commonjs"}' > "$OUT_DIR/package.json"
  cp wasm-smoke.cjs "$OUT_DIR/smoke.cjs"
  node "$OUT_DIR/smoke.cjs"
fi
