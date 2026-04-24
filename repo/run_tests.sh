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

# On Windows/Git-Bash pwd returns /c/... which Docker can't resolve as a host
# path; pwd -W gives the native C:/... form that Docker Desktop understands.
REPO="$(pwd -W 2>/dev/null || pwd)"

WHICH="${1:-default}"
RC=0

run_unit() {
    echo
    echo "============================================================"
    echo " Unit tests — cargo test (unit_tests crate + in-tree tests)"
    echo "============================================================"
    if ! docker run --rm \
        -v "$REPO:/workspace" \
        -w //workspace \
        rust:1.88-bookworm \
        bash -c "cargo test -p unit_tests -p shared -p backend -p frontend_core -p frontend_tests --lib --tests"; then
        RC=1
    fi
}

run_api() {
    echo
    echo "============================================================"
    echo " API tests  — cargo test -p api_tests"
    echo "============================================================"
    if ! docker run --rm \
        -v "$REPO:/workspace" \
        -w //workspace \
        --network host \
        -e API_BASE="${API_BASE:-http://127.0.0.1:8000}" \
        rust:1.88-bookworm \
        bash -c "cargo test -p api_tests --tests -- --nocapture"; then
        RC=1
    fi
}

run_e2e() {
    echo
    echo "============================================================"
    echo " E2E tests  — Playwright (requires docker compose up)"
    echo "============================================================"
    # Build the prebuilt image. npm deps and test sources are baked in so
    # the run step needs no network access and no runtime `npm install`.
    # Layers are cached, so repeat builds are fast when package-lock and
    # specs are unchanged.
    if ! docker build -t field-service-hub-e2e:local "$REPO/e2e"; then
        RC=1
        return
    fi
    if ! docker run --rm \
        --network host \
        -e FRONTEND_URL="${FRONTEND_URL:-http://127.0.0.1:3000}" \
        -e API_BASE="${API_BASE:-http://127.0.0.1:8000}" \
        field-service-hub-e2e:local; then
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
