#!/usr/bin/env bash
# Audit endpoints — admin-gated; without a token must 401.
set -eu
API="${API_BASE:-http://localhost:8000}"

code=$(curl -sS -o /dev/null -w '%{http_code}' "$API/api/audit/verify")
if [ "$code" != "401" ]; then
    echo "FAIL /audit/verify without token expected 401, got $code"; exit 1
fi
echo "OK /audit/verify unauth -> 401"

code=$(curl -sS -o /dev/null -w '%{http_code}' \
    "$API/api/audit/review/00000000-0000-0000-0000-000000000000")
if [ "$code" != "401" ]; then
    echo "FAIL /audit/<type>/<id> without token expected 401, got $code"; exit 1
fi
echo "OK /audit/<type>/<id> unauth -> 401"
