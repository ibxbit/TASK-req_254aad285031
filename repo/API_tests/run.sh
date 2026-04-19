#!/usr/bin/env bash
# API tests — Rust integration tests via reqwest against a live backend.
#
# Tests probe $API_BASE/api/health on each test and SKIP (print + return
# without failing) if the backend isn't reachable — so a fresh checkout
# without `docker compose up` still yields a green build.
#
# For meaningful API coverage, bring the stack up first:
#   docker compose up -d

set -eu

cd "$(dirname "$0")/.."

if ! command -v cargo >/dev/null 2>&1; then
    echo "ERROR: cargo not found in PATH. Install Rust: https://rustup.rs/"
    exit 1
fi

: "${API_BASE:=http://127.0.0.1:8000}"
export API_BASE

echo "API_BASE=$API_BASE"
echo "cargo test -p api_tests --tests"
cargo test -p api_tests --tests -- --nocapture
