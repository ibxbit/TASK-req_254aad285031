#!/usr/bin/env bash
# Run Playwright E2E tests against the live frontend.
#
# Prerequisites:
#   docker compose up          # backend + frontend must be running
#   npm install                # first time only, installs @playwright/test
#   npx playwright install chromium   # first time only
#
# Usage:
#   ./run.sh                   # headless (default)
#   ./run.sh --headed          # headed mode (browser window visible)
#   ./run.sh --debug           # debug mode (step through tests)
#   ./run.sh auth              # only run auth.spec.ts
#
# Environment:
#   FRONTEND_URL   default http://127.0.0.1:3000
#   API_BASE       default http://127.0.0.1:8000
#   ADMIN_USER     default admin
#   ADMIN_PASS     default change-me-please-now
#   API_ADMIN_TOKEN (optional) pre-provisioned admin bearer token

set -eu
cd "$(dirname "$0")"

# Install dependencies if node_modules is missing.
if [ ! -d node_modules ]; then
    echo "[e2e] Installing npm dependencies..."
    npm install
fi

# Install Playwright browser if not present.
if ! npx playwright --version >/dev/null 2>&1 || [ ! -d "$(npx playwright show-browsers 2>/dev/null | grep -m1 chromium || true)" ]; then
    echo "[e2e] Installing Playwright Chromium browser..."
    npx playwright install chromium 2>/dev/null || true
fi

# Pass any extra arguments through to playwright test.
ARGS="${*}"
if [ -z "$ARGS" ]; then
    npx playwright test
else
    npx playwright test $ARGS
fi
