# Targeted Recheck of Reported Issues (Static-Only)

## Scope

- Static inspection only; no execution.
- Checked requested items in code + tests + docs.

## Results

### 1) Lockout lifecycle test method mismatch

- **Status:** Fixed
- **Evidence:**
  - Test now uses `patch_json_auth` for reset call: `repo/API_tests/tests/lockout_policy.rs:70`
  - Backend route is PATCH: `repo/backend/src/routes/admin/users.rs:212`
  - Expected `204` retained: `repo/API_tests/tests/lockout_policy.rs:76`
- **Conclusion:** Test method and API contract now match.

### 2) API integration strict-mode gating (skip-pass risk)

- **Status:** Fixed (documentation + framework support)
- **Evidence:**
  - Strict mode implemented (panic on missing prereqs): `repo/API_tests/src/lib.rs:64`, `repo/API_tests/src/lib.rs:79`, `repo/API_tests/src/lib.rs:281`
  - README marks strict mode mandatory for CI/release gates: `repo/README.md:196`, `repo/README.md:214`
  - README still explicitly scopes non-strict skips to local convenience: `repo/README.md:217`
- **Conclusion:** Project now provides explicit mandatory strict-gate guidance and strict behavior in test harness.
- **Static boundary note:** Cannot confirm actual CI pipeline config from repository content provided.

### 3) Catalog reversed availability range coverage gap

- **Status:** Fixed
- **Evidence:**
  - New deterministic reversed-range test added: `repo/API_tests/tests/catalog_filter_edges.rs:270`
  - Test asserts current contract behavior (200 + array) and non-match for seeded non-spanning window: `repo/API_tests/tests/catalog_filter_edges.rs:298`, `repo/API_tests/tests/catalog_filter_edges.rs:305`
- **Conclusion:** The missing edge case is now explicitly covered.

## Final Assessment

- Requested three issues are resolved at static-code level.
- Remaining boundary: CI enforcement itself is **Cannot Confirm Statistically** without pipeline files/execution evidence.
