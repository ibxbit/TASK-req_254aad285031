# Targeted Static Recheck (Requested 3 Items)

Scope: static code/test inspection only. No runtime execution.

## 1) Tamper-proof audit chain completeness

- **Status:** Partial Fix
- **What is fixed:**
  - Review image upload now records hash-chained event inside same DB transaction as insert (`repo/backend/src/routes/workorders/images.rs:115`, `repo/backend/src/routes/workorders/images.rs:144`).
  - Internship attachment upload now records hash-chained event inside same DB transaction as insert (`repo/backend/src/routes/internships/attachments.rs:98`, `repo/backend/src/routes/internships/attachments.rs:125`).
  - Audit helper used is transactional append-only chain (`repo/backend/src/audit/mod.rs:40`).
- **Remaining gap:**
  - Internship attachment audit payload does not include file type/MIME metadata (has path/size/hash only), while requested metadata was path + size + type (`repo/backend/src/routes/internships/attachments.rs:117`).
- **Conclusion:** atomic chain logging is implemented; metadata completeness is still slightly incomplete for attachment type.

## 2) Lockout policy proof in API tests

- **Status:** Partial Fix
- **What is fixed:**
  - New lockout lifecycle test exists and checks 5 failed logins -> 6th is 423, then reset/unlock path (`repo/API_tests/tests/lockout_policy.rs:21`).
  - Backend lockout logic supports this lifecycle (`repo/backend/src/auth/lock.rs:29`, `repo/backend/src/routes/auth.rs:66`).
- **Remaining gap:**
  - Reset step in the new test calls password-reset endpoint with POST helper, but backend route is PATCH (`repo/API_tests/tests/lockout_policy.rs:70`, `repo/backend/src/routes/admin/users.rs:212`).
- **Conclusion:** coverage intent is correct, but test method mismatch makes the new proof potentially non-executable as written.

## 3) Catalog filter edge coverage

- **Status:** Mostly Fixed
- **What is fixed:**
  - Added dedicated edge test file covering:
    - single-sided availability behavior (`repo/API_tests/tests/catalog_filter_edges.rs:86`, `repo/API_tests/tests/catalog_filter_edges.rs:107`)
    - multi-category AND semantics (`repo/API_tests/tests/catalog_filter_edges.rs:125`)
    - multi-tag OR semantics (`repo/API_tests/tests/catalog_filter_edges.rs:160`)
    - pagination boundaries (`repo/API_tests/tests/catalog_filter_edges.rs:237`, `repo/API_tests/tests/catalog_filter_edges.rs:256`)
  - Existing test also covers invalid datetime format -> 400 (`repo/API_tests/tests/catalog_browse_and_filters.rs:184`).
- **Minor remaining gap:**
  - No explicit test for reversed availability range (`available_from > available_to`) behavior.
- **Conclusion:** requested edge coverage is substantially addressed; one additional invalid-combination case would complete it.

## Overall

- **Result:** 2/3 items are only partially closed due to small but material residual gaps (attachment type metadata in audit payload; lockout test HTTP method mismatch).
