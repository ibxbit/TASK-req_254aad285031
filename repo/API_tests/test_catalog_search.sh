#!/usr/bin/env bash
# /api/services/search — guardrails without a token and for invalid params.
set -eu
API="${API_BASE:-http://localhost:8000}"

# No auth -> 401
code=$(curl -sS -o /dev/null -w '%{http_code}' "$API/api/services/search")
if [ "$code" != "401" ]; then
    echo "FAIL /services/search without auth expected 401, got $code"; exit 1
fi
echo "OK /services/search unauth -> 401"

# Compare without ids — must reject (401 or 400)
code=$(curl -sS -o /dev/null -w '%{http_code}' "$API/api/services/compare")
case "$code" in
    400|401|422) echo "OK /services/compare missing ids -> $code" ;;
    *) echo "FAIL /services/compare missing ids expected 4xx, got $code"; exit 1 ;;
esac
