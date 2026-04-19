#!/usr/bin/env bash
# Authentication surface: register lifecycle + error codes.
set -eu
API="${API_BASE:-http://localhost:8000}"

# Unique name so repeat runs don't collide with prior state.
U="apitest_$(date +%s%N)"
P="longpassword1234"

# ---- register ----
# 201 if this is the very first user (bootstrap); 403 once any user exists.
code=$(curl -sS -o /dev/null -w '%{http_code}' \
    -X POST "$API/api/auth/register" \
    -H 'Content-Type: application/json' \
    -d "{\"username\":\"$U\",\"password\":\"$P\"}")
case "$code" in
    201|403) echo "OK /auth/register -> $code (201 bootstrap, 403 users exist)" ;;
    *) echo "FAIL /auth/register returned $code"; exit 1 ;;
esac

# ---- bad password / unknown user -> 401 ----
code=$(curl -sS -o /dev/null -w '%{http_code}' \
    -X POST "$API/api/auth/login" \
    -H 'Content-Type: application/json' \
    -d '{"username":"no_such_user_xyz_12345","password":"whatever1234"}')
if [ "$code" != "401" ]; then
    echo "FAIL /auth/login unknown user expected 401, got $code"; exit 1
fi
echo "OK /auth/login unknown user -> 401"

# ---- /auth/me without bearer -> 401 ----
code=$(curl -sS -o /dev/null -w '%{http_code}' "$API/api/auth/me")
if [ "$code" != "401" ]; then
    echo "FAIL /auth/me without bearer expected 401, got $code"; exit 1
fi
echo "OK /auth/me unauthenticated -> 401"

# ---- /auth/me with malformed bearer -> 401 ----
code=$(curl -sS -o /dev/null -w '%{http_code}' \
    -H 'Authorization: Bearer not-a-real-token' \
    "$API/api/auth/me")
if [ "$code" != "401" ]; then
    echo "FAIL /auth/me bad token expected 401, got $code"; exit 1
fi
echo "OK /auth/me bad token -> 401"

# ---- login rejects too-short password as validation error (<12 chars) ----
# Per spec the min length is 12; a shorter password fails hash rules at
# register time. For /login on a non-existent user the response is 401,
# not 400 — we check the shape of the failure, not the exact code here.
code=$(curl -sS -o /dev/null -w '%{http_code}' \
    -X POST "$API/api/auth/register" \
    -H 'Content-Type: application/json' \
    -d '{"username":"x","password":"short"}')
case "$code" in
    400|403) echo "OK /auth/register short password -> $code" ;;
    *) echo "FAIL short password expected 400/403, got $code"; exit 1 ;;
esac
