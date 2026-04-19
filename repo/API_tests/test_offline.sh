#!/usr/bin/env bash
# Offline compliance: frontend bundle must not reference external CDNs and
# must serve the service worker at /sw.js so cached operation works with
# no internet.
set -eu

FRONT="${FRONTEND_BASE:-http://localhost:3000}"

if ! curl -sf --max-time 3 "$FRONT/" >/dev/null 2>&1; then
    echo "SKIP offline check: frontend not reachable at $FRONT"
    exit 0
fi

# 1) Service worker served at scope root.
code=$(curl -sS -o /dev/null -w '%{http_code}' "$FRONT/sw.js")
if [ "$code" != "200" ]; then
    echo "FAIL /sw.js expected 200, got $code"
    exit 1
fi
echo "OK /sw.js served"

# 2) Index must not reference any https:// URL (no CDN usage).
html=$(mktemp); trap 'rm -f "$html"' EXIT
curl -sS "$FRONT/" > "$html"

# Allow-list: localhost / 127.0.0.1 / XML namespaces that browsers ignore.
if grep -Eo 'https?://[a-zA-Z0-9./:_-]+' "$html" \
    | grep -vE '^(https?://(localhost|127\.0\.0\.1)|http://www\.w3\.org)' \
    | grep -q .; then
    echo "FAIL frontend index references an external URL:"
    grep -Eo 'https?://[a-zA-Z0-9./:_-]+' "$html" | sort -u
    exit 1
fi
echo "OK frontend index contains no external CDN references"
