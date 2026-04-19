#!/usr/bin/env bash
# Unit tests — Rust only, three kinds together:
#   * in-tree  `#[cfg(test)]` modules inside shared/ and backend/ src files
#   * integration tests under unit_tests/tests/*.rs
#   * frontend pure-logic unit + workflow tests under frontend_core/*
#
# Runs on the native target (no wasm toolchain required). The wasm-only
# `frontend` binary crate is excluded.

set -eu

cd "$(dirname "$0")/.."

if ! command -v cargo >/dev/null 2>&1; then
    echo "ERROR: cargo not found in PATH. Install Rust: https://rustup.rs/"
    exit 1
fi

echo "cargo test -p unit_tests -p shared -p backend -p frontend_core -p frontend_tests --lib --tests"
cargo test -p unit_tests -p shared -p backend -p frontend_core -p frontend_tests --lib --tests
