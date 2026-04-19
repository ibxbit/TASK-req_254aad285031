#!/usr/bin/env bash
# Review endpoint guardrails (no token + malformed payload).
# Deep flow tests (duplicate rejection, 14-day window, daily cap) require a
# bootstrapped admin + seed data and belong in integration tests.
set -eu
API="${API_BASE:-http://localhost:8000}"

# Unauthenticated -> 401
code=$(curl -sS -o /dev/null -w '%{http_code}' \
    -X POST "$API/api/reviews" \
    -H 'Content-Type: application/json' \
    -d '{"work_order_id":"00000000-0000-0000-0000-000000000000","rating":3,"text":"x"}')
if [ "$code" != "401" ]; then
    echo "FAIL POST /reviews unauth expected 401, got $code"; exit 1
fi
echo "OK POST /reviews unauthenticated -> 401"

# Malformed JSON -> 400/401 (auth happens first on some routes; both are
# deterministic non-2xx — we just assert the request is rejected).
code=$(curl -sS -o /dev/null -w '%{http_code}' \
    -X POST "$API/api/reviews" \
    -H 'Content-Type: application/json' \
    -d 'not-json')
case "$code" in
    400|401|422) echo "OK POST /reviews malformed -> $code" ;;
    *) echo "FAIL POST /reviews malformed expected 400/401/422, got $code"; exit 1 ;;
esac

# /services/<bad>/reputation should reject non-UUID id
code=$(curl -sS -o /dev/null -w '%{http_code}' \
    "$API/api/services/not-a-uuid/reputation")
case "$code" in
    400|401) echo "OK /reputation bad id -> $code" ;;
    *) echo "FAIL /reputation bad id expected 400/401, got $code"; exit 1 ;;
esac
