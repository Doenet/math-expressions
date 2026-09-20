#!/usr/bin/env bash
# Build the wasm-bindgen packages this package ships, into ./vendor (git-ignored).
# Both targets are built, because both are published:
#
#   vendor/wasm      --target nodejs — the fallback `lib/_wasm.ts` loads through
#                    `createRequire` when nothing was injected. It instantiates
#                    the wasm synchronously at require() time, so the original
#                    synchronous API (me.fromText(...).equals(...), no await)
#                    works under Node and vitest with no init step.
#
#   vendor/wasm-web  --target web — ESM with an async `default()` and a
#                    synchronous `initSync()`, for browser and Web Worker hosts.
#                    Those cannot use the nodejs build, so they instantiate this
#                    one from bytes and hand it to `setWasmModule` (see
#                    `lib/_wasm.ts`). Shipping it is what lets such a host — for
#                    instance DoenetML — consume this package from npm with no
#                    Rust toolchain of its own; building it here is the only
#                    place cargo is needed.
#
# Pass a single target name to build just one (`./build-wasm.sh nodejs`).
#
# Delegates to the single source-of-truth build script in math-expressions-rs-wasm;
# the vendor dirs are kept separate from that package's shared pkg/ (which the
# playground builds with --target web) to avoid clobber.
# Requires: rustup target add wasm32-unknown-unknown + wasm-bindgen-cli matching
# the wasm-bindgen crate version in Cargo.toml.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
BUILD="$HERE/../math-expressions-rs-wasm/build-wasm.sh"

targets=("$@")
if [ "${#targets[@]}" -eq 0 ]; then
  targets=(nodejs web)
fi

# Both targets share one `cargo build`; only wasm-bindgen runs twice.
for target in "${targets[@]}"; do
  case "$target" in
    nodejs) bash "$BUILD" nodejs "$HERE/vendor/wasm" ;;
    web) bash "$BUILD" web "$HERE/vendor/wasm-web" ;;
    *)
      echo "build-wasm.sh: unknown target '$target' (expected nodejs or web)" >&2
      exit 2
      ;;
  esac
done
