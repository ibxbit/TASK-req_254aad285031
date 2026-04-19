#!/usr/bin/env bash
# Run every test suite. Rust tests are the primary coverage layer; Playwright
# E2E tests exercise the real browser against the live Docker stack.
#
# Usage:
#   ./run_tests.sh             # unit + API (no E2E — E2E needs docker compose)
#   ./run_tests.sh unit        # unit tests only
#   ./run_tests.sh api         # API tests only (needs backend reachable)
#   ./run_tests.sh e2e         # Playwright E2E tests (needs full stack on :3000)
#   ./run_tests.sh all         # unit + API + E2E
#
# Environment:
#   API_BASE       default http://127.0.0.1:8000
#   FRONTEND_URL   default http://127.0.0.1:3000  (E2E only)
#
# For API/E2E tests, bring up the stack first:
#   docker compose up -d

set -eu

cd "$(dirname "$0")"

WHICH="${1:-default}"
RC=0

run_unit() {
    echo
    echo "============================================================"
    echo " Unit tests — cargo test (unit_tests crate + in-tree tests)"
    echo "============================================================"
    if ! ./unit_tests/run.sh; then
        RC=1
    fi
}

run_api() {
    echo
    echo "============================================================"
    echo " API tests  — cargo test -p api_tests"
    echo "============================================================"
    if ! ./API_tests/run.sh; then
        RC=1
    fi
}

run_e2e() {
    echo
    echo "============================================================"
    echo " E2E tests  — Playwright (requires docker compose up)"
    echo "============================================================"
    if ! ./e2e/run.sh; then
        RC=1
    fi
}

case "$WHICH" in
    unit)    run_unit ;;
    api)     run_api ;;
    e2e)     run_e2e ;;
    all)     run_unit; run_api; run_e2e ;;
    default) run_unit; run_api ;;
    *)
        echo "usage: $0 [unit|api|e2e|all]"
        exit 2
        ;;
esac

echo
if [ "$RC" -eq 0 ]; then
    echo "ALL TESTS PASSED"
else
    echo "TESTS FAILED"
fi
exit "$RC"
