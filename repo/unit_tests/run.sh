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

CMD="cargo test -p unit_tests -p shared -p backend -p frontend_core -p frontend_tests --lib --tests"

if command -v cargo >/dev/null 2>&1; then
    echo "$CMD"
    exec $CMD
else
    echo "cargo not in PATH — running inside rust:1.88-bookworm container"
    exec docker run --rm \
        -v "$(pwd):/workspace" \
        -w /workspace \
        rust:1.88-bookworm \
        bash -c "$CMD"
fi
