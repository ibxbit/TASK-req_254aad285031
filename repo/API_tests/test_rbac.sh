#!/usr/bin/env bash
# RBAC: verify unauthenticated requests to protected endpoints return 401.
# Covers one representative route per module — broader role checks require
# provisioned users and happen in integration tests.
set -eu
API="${API_BASE:-http://localhost:8000}"

check_401() {
    local method="$1"
    local path="$2"
    local body="${3:-}"
    local code
    if [ -n "$body" ]; then
        code=$(curl -sS -o /dev/null -w '%{http_code}' \
            -X "$method" "$API$path" \
            -H 'Content-Type: application/json' \
            -d "$body")
    else
        code=$(curl -sS -o /dev/null -w '%{http_code}' \
            -X "$method" "$API$path")
    fi
    if [ "$code" != "401" ]; then
        echo "FAIL  $method $path expected 401, got $code"
        return 1
    fi
    echo "OK    $method $path -> 401"
}

fails=0
check_401 GET  "/api/services/search"                              || fails=$((fails+1))
check_401 POST "/api/services"            '{"name":"x"}'           || fails=$((fails+1))
check_401 POST "/api/warehouses"          '{"name":"x"}'           || fails=$((fails+1))
check_401 POST "/api/reviews"             '{"rating":5}'           || fails=$((fails+1))
check_401 POST "/api/reports"             '{"type":"DAILY"}'       || fails=$((fails+1))
check_401 GET  "/api/warehouses/tree"                              || fails=$((fails+1))
check_401 GET  "/api/boards"                                       || fails=$((fails+1))
check_401 GET  "/api/audit/verify"                                 || fails=$((fails+1))

[ "$fails" -eq 0 ] || exit 1
