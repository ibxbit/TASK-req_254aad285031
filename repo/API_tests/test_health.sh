#!/usr/bin/env bash
# /api/health must return 200 + JSON with "status".
set -eu
API="${API_BASE:-http://localhost:8000}"

body=$(mktemp)
trap 'rm -f "$body"' EXIT

code=$(curl -sS -o "$body" -w '%{http_code}' "$API/api/health")

if [ "$code" != "200" ]; then
    echo "FAIL: expected 200 from /api/health, got $code"
    cat "$body"
    exit 1
fi

if ! grep -q '"status"' "$body"; then
    echo "FAIL: /api/health body missing \"status\""
    cat "$body"
    exit 1
fi

echo "OK /api/health (200, status field present)"
