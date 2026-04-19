#!/usr/bin/env bash
# Warehouse history endpoints must require management role (administrator /
# warehouse_manager). This script covers the unauthenticated case (401) for
# all three history paths; role-403 matrix lives in the Rust integration
# tests (API_tests/tests/rbac_roles.rs).
set -eu
API="${API_BASE:-http://localhost:8000}"

check_401() {
    local path="$1"
    local code
    code=$(curl -sS -o /dev/null -w '%{http_code}' "$API$path")
    if [ "$code" != "401" ]; then
        echo "FAIL  GET $path expected 401, got $code"
        return 1
    fi
    echo "OK    GET $path -> 401"
}

fails=0
check_401 "/api/warehouses/00000000-0000-0000-0000-000000000000/history"       || fails=$((fails+1))
check_401 "/api/warehouse-zones/00000000-0000-0000-0000-000000000000/history"  || fails=$((fails+1))
# Regression for the previous authorization gap — bins history used to be
# reachable without a management role.
check_401 "/api/bins/00000000-0000-0000-0000-000000000000/history"             || fails=$((fails+1))

[ "$fails" -eq 0 ] || exit 1
