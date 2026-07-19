#!/usr/bin/env bash

# Use VIM as the command line git editor. Not everyone's preference, but oh well...
git config core.editor vim

# Toolchain for the Rust->wasm build (math-expressions-rs/scripts/build-wasm.sh).
# The version must match the wasm-bindgen crate pin in math-expressions-rs/Cargo.toml.
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version "=0.2.126" --locked
