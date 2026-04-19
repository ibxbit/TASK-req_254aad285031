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

: "${API_BASE:=http://127.0.0.1:8000}"
export API_BASE

CMD="cargo test -p api_tests --tests -- --nocapture"

if command -v cargo >/dev/null 2>&1; then
    echo "API_BASE=$API_BASE"
    echo "$CMD"
    exec $CMD
else
    echo "cargo not in PATH — running inside rust:1.88-bookworm container (--network host)"
    exec docker run --rm \
        -v "$(pwd):/workspace" \
        -w /workspace \
        --network host \
        -e API_BASE="$API_BASE" \
        rust:1.88-bookworm \
        bash -c "$CMD"
fi
