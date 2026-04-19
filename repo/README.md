# Field Service Operations Hub

**Project type: fullstack web application** (Dioxus web frontend + Rocket
REST API + MySQL, packaged as a one-command Docker Compose stack).

Offline, on-premise field service operations platform. Every runtime
dependency — MySQL, backend, frontend bundle — ships in the compose file;
there is no external SaaS dependency and no required manual install step
for reviewers.

---

## Contents

- [Tech stack](#tech-stack)
- [Repository layout](#repository-layout)
- [Quick start (Docker — default)](#quick-start-docker--default)
- [Demo credentials](#demo-credentials)
- [Verification walkthrough](#verification-walkthrough)
- [Testing](#testing)
- [Contributing / native development](#contributing--native-development)
- [Backend architecture](#backend-architecture)
- [Security + policy posture](#security--policy-posture)
- [Constraints](#constraints)

---

## Tech stack

| Layer | Tech |
|---|---|
| Frontend | Dioxus 0.5 (web / WASM), served via nginx |
| Backend | Rocket 0.5 (Rust) |
| Database | MySQL 8.0+ |
| Encryption | AES-256-GCM, app-level, local master key file |

---

## Repository layout

```
repo/
├── backend/          # Rocket (Rust) API server
├── frontend/         # Dioxus web UI (wasm target)
├── frontend_core/    # Pure-Rust slice of the frontend — unit-tested natively
├── frontend_tests/   # dioxus-ssr rendered-layer tests (native target, no browser)
├── shared/           # Shared DTOs/enums (used by backend + frontend)
├── db/               # MySQL schema (idempotent; auto-loaded by compose)
├── config/           # Local config + encryption key (gitignored at rest)
├── API_tests/        # Rust reqwest integration tests against the live API
├── unit_tests/       # Cross-crate unit tests
├── .github/workflows/ci.yml  # CI: unit+frontend tests + strict API integration
├── docker-compose.yml
└── run_tests.sh      # Unified test runner (unit + API)
```

---

## Quick start (Docker — default)

```bash
docker compose up
# or, if your Docker installation uses the legacy CLI plugin:
docker-compose up
```

That is the whole setup. No `.env` edits, no manual SQL import, no
`cargo install`, no MySQL install on the host. On first run:

- MySQL loads `db/schema.sql` automatically via `/docker-entrypoint-initdb.d/`.
- The backend entrypoint generates `config/master.key` + a Docker-friendly
  `config/config.toml`.
- The frontend is served from nginx and proxies `/api → backend:8000`.

### Service list

| Service | URL | Notes |
|---|---|---|
| Web UI | http://localhost:3000 | Dioxus bundle via nginx; proxies `/api` |
| REST API | http://localhost:8000 | Rocket; reachable directly for curl/tests |
| MySQL | `localhost:3306` | User `hub_user` / pass `hub_pass`, db `field_service_hub` |

Stop the stack: `docker compose down` (data persists in the `mysql_data`
volume). Full reset: `docker compose down -v`.

---

## Demo credentials

The stack ships empty. Bootstrap the first administrator once via a single
curl call (see step 2 below); everything else is provisioned through the
admin API. The block below is the credential matrix used by the API tests
and the verification walkthrough — run the one-liner at the end of the
walkthrough to provision every row.

| Role | Username | Password | Access summary |
|---|---|---|---|
| Administrator | `admin` | `change-me-please-now` | Full access to every module and every `/api/admin/*` endpoint. Bootstrap password — change immediately in a real deployment. |
| Moderator | `moderator01` | `verifypass123!` | Forum moderation (posts, comments, board rules). |
| Service Manager | `service_manager01` | `verifypass123!` | Service catalog mutation, work-order completion. |
| Warehouse Manager | `warehouse_manager01` | `verifypass123!` | Warehouse / zone / bin CRUD + history. |
| Mentor | `mentor01` | `verifypass123!` | Comment on + approve intern reports; read intern dashboards. |
| Intern | `intern01` | `verifypass123!` | Submit weekly/monthly/daily reports + attachments. |
| Requester | `requester01` | `verifypass123!` | Create work orders; submit initial + follow-up reviews (with images + tags). |

> The demo passwords are **for local / review-only use**. Rotate them in
> any real deployment via `PATCH /api/admin/users/<id>/password`.

---

## Verification walkthrough

After `docker compose up` reports every service healthy:

### 1. Health probe

```bash
curl http://localhost:8000/api/health
# → {"status":"ok"}
```

### 2. Bootstrap the administrator (one-off; closed after first admin)

```bash
curl -X POST http://localhost:8000/api/auth/register \
     -H 'Content-Type: application/json' \
     -d '{"username":"admin","password":"change-me-please-now"}'
```

### 3. Log in, capture token

**With `jq` installed** (available in most Linux/macOS environments):

```bash
TOKEN=$(curl -s -X POST http://localhost:8000/api/auth/login \
    -H 'Content-Type: application/json' \
    -d '{"username":"admin","password":"change-me-please-now"}' \
  | jq -r '.token')
```

**Without `jq`** (manual extraction — no extra tools needed):

```bash
# Run the login command and copy the token value from the JSON response:
curl -s -X POST http://localhost:8000/api/auth/login \
    -H 'Content-Type: application/json' \
    -d '{"username":"admin","password":"change-me-please-now"}'
# Response: {"token":"<YOUR_TOKEN>","user":{...}}
# Copy the token string and export it:
export TOKEN=<paste-your-token-here>
```

### 4. Provision every demo role

```bash
for R in moderator service_manager warehouse_manager mentor intern requester; do
  curl -sS -X POST http://localhost:8000/api/admin/users \
    -H "Authorization: Bearer $TOKEN" \
    -H 'Content-Type: application/json' \
    -d "{\"username\":\"${R}01\",\"password\":\"verifypass123!\",\"role\":\"${R}\"}"
done
```

### 5. Open the UI

http://localhost:3000 — sign in as any row from the [Demo credentials](#demo-credentials)
matrix. The landing page lists only the modules that role may reach.

### 6. (Optional) Verify offline behaviour

Disconnect the network; the UI still renders (service-worker cache) and
writes fail with a typed `{"error":"offline"}` response.

### 7. (Optional) Verify the audit chain

```bash
curl -H "Authorization: Bearer $TOKEN" http://localhost:8000/api/audit/verify
```

---

## Testing

All test suites are Rust-native. Run everything in one command:

```bash
./run_tests.sh           # unit + API, in that order
./run_tests.sh unit      # cross-crate + frontend_core unit tests (no DB/server needed)
./run_tests.sh api       # reqwest integration tests (needs docker stack up)
```

### What runs where

| Suite | Command | What it covers |
|---|---|---|
| Backend + shared unit | `cargo test -p backend -p shared -p unit_tests` | Crypto round-trips, Argon2 hashing, audit hash chain, face image checks, deadline math, logging redaction, DTO serde round-trips. |
| Frontend unit | `cargo test -p frontend_core` | 45 unit + 6 workflow tests: AuthState, URL builder, route guards, nav visibility per role, tag toggle, rating clamp, API path builders — runs on the native target, no browser. |
| Frontend rendered-layer | `cargo test -p frontend_tests` | 50+ dioxus-ssr tests: renders Dioxus components to HTML for all pages (login, home per role, work orders, forum, admin, warehouse) and asserts catalog filter controls, sort options, datetime-local inputs, compare bar states, error state CSS, nav visibility per role, route path constants — no browser or wasm toolchain required. |
| API integration | `cargo test -p api_tests` (also via `./run_tests.sh api`) | 110+ reqwest tests: per-endpoint body-contract assertions for auth/session (login shape, /me shape, inactive→403, wrong-password→401, token usability), full RBAC matrix per role, ownership, review lifecycle, image upload limits, history authorization, forum visibility by team, lockout policy (5 failures→423, admin reset→200), catalog filter edges (single-sided availability, AND/OR semantics, pagination), frontend↔backend path confidence layer (validates frontend_core URL builders against live routes), cross-domain authorized response structure assertions, endpoint-smoke guardrail covering all 88 protected routes (91 total endpoints). |

### API test admin credentials

The API test suite provisions fresh users per test via the admin API. It
auto-discovers an admin token in this order:

1. `API_ADMIN_TOKEN` environment variable (CI use).
2. `API_ADMIN_USER` + `API_ADMIN_PASS` environment variables.
3. The documented bootstrap pair `admin` / `change-me-please-now`.
4. `POST /auth/register` (open only until the first admin exists).

If none succeed, the contract/RBAC tests **skip** with a `SKIP …` log
rather than failing, so `cargo test --workspace` on a fresh checkout is
still green.

### Strict mode (required for CI / release gates)

`API_TESTS_STRICT=1` turns every skip into a hard failure. **This flag
is mandatory for any acceptance or release gate** — without it, tests
that cannot reach the backend or obtain an admin token silently pass,
giving a false green signal. A green run in strict mode guarantees the
full suite actually executed end-to-end against the live backend.

```bash
docker compose up -d
curl -X POST http://localhost:8000/api/auth/register \
     -H 'Content-Type: application/json' \
     -d '{"username":"admin","password":"change-me-please-now"}'
API_TESTS_STRICT=1 ./run_tests.sh api
```

If the backend is unreachable or the admin token cannot be obtained,
tests panic with a `STRICT …` message identifying the missing
prerequisite. The CI job **must** set `API_TESTS_STRICT=1`; any pipeline
that omits it provides no coverage guarantee.

Local developer convenience: omit `API_TESTS_STRICT` (or set it to `0`)
to allow offline skips when running `cargo test --workspace` without the
Docker stack.

### CI pipeline (`.github/workflows/ci.yml`)

The CI workflow runs automatically on every push and pull request:

1. **`unit-and-frontend-tests`** — `cargo test -p backend -p shared -p unit_tests -p frontend_core -p frontend_tests`; no Docker required.
2. **`api-integration-strict`** — starts the Docker Compose stack, waits for the backend health endpoint, bootstraps the admin, then runs `API_TESTS_STRICT=1 cargo test -p api_tests --tests`.

**Release gate:** any pipeline running the API test suite **must** set `API_TESTS_STRICT=1`. Without it, tests that cannot reach the backend silently pass, giving a false green. The CI workflow always sets this flag; omitting it is a pipeline misconfiguration.

### Formatting + linting

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Contributing / native development

For contributors who want to run the Rust services directly on their host
(without Docker), see [CONTRIBUTING.md](CONTRIBUTING.md). That document
covers native prerequisites, key generation, database setup, and running the
services locally. **This is not required for review or evaluation** — the
Docker path above is fully self-contained.

---

## Backend architecture

Rocket application organised by domain module under `backend/src/`. The
**auth** module is the only one that follows a strict layered shape —
controller (routes), service helpers, repository. The other domains use
route handlers that call domain helper modules and issue `sqlx` queries
inline; that is the current intentional state.

| Directory | Responsibility |
|---|---|
| `backend/src/routes/` | Rocket route handlers (controllers). Parse, enforce RBAC, respond JSON. Most also issue `sqlx` queries directly. |
| `backend/src/{audit, auth, face, forum, internships, workorders, warehouse_audit}/` | HTTP-agnostic domain helpers (validation, deadline math, visibility/moderation predicates, hash-chain writer, image analysis). |
| `backend/src/services/` | Thin re-export shim over the domain modules. No business logic lives here. |
| `backend/src/repositories/` | Pure data access via `sqlx`. Currently only `users` uses this pattern. |

**Reference implementation — auth module:**

- Controller: `backend/src/routes/auth.rs`
- Services: `crate::auth::{password, session, lock, guard}`
- Repository: `backend/src/repositories/users.rs`

---

## Security + policy posture

- **RBAC.** Every mutation checks role (and object ownership where
  applicable). Admin-only endpoints route under `/api/admin/*`.
  `AuthUser::require_role` / `require_any` are the single enforcement
  point. Full permission matrix is covered by `API_tests/tests/rbac_matrix.rs`.
- **User lifecycle.** Bootstrap endpoint for the first admin; ongoing
  create / role-change / password-reset / activate-deactivate /
  sensitive-identifier are administrator-only. Team add/remove drives
  restricted board visibility.
- **Encryption at rest.** Sensitive identifiers stored as
  `nonce || ciphertext || tag` (AES-256-GCM) with the local master key.
  API and UI only ever see the mask (e.g. `XXX-XX-1234`). Plaintext never
  appears in logs or responses.
- **Face workflow.** Every capture must pass resolution, brightness,
  sharpness (Laplacian variance), and single-frontal-face checks. No
  always-pass fallback. Optional liveness challenge rows live in
  `face_liveness_challenges` and land in the audit chain.
- **Deadlines.** Weekly = Mon 12:00 local, monthly = 5th 17:00 local,
  grace 72h. Computed server-side; client-supplied `due_at` is rejected.
- **Integrity.** Every attachment (review image, report attachment, face
  image) stores a SHA-256 `content_hash`. Face validate verifies stored
  bytes against the stored hash before running checks.
- **Traceability.** `event_log` hash chain covers users, teams,
  warehouses, zones, bins, reports, reviews, face records. Structural
  change logs also live under `warehouse_change_log`,
  `warehouse_zone_change_log`, `bin_change_log`.
- **Logging.** JSON events on stderr for `auth.login.failed`,
  `authz.denied`, `validation.failed`, `audit.write.failed`,
  `auth.account.locked`. A redaction helper masks values for well-known
  sensitive keys (`password`, `token`, `ssn`, `secret`, `api_key`).
- **Review lifecycle.** One initial review + one follow-up review per
  completed work order, 14-day window, max 3 reviews/user/day,
  requester-selectable tags attached atomically. All enforced by DB
  constraints *and* service-level checks.
- **Session lifecycle.** Bearer tokens hash-stored in `sessions`. On
  `PATCH /api/admin/users/<id>/status` with `is_active=false`, every row
  for that user is deleted in the same transaction as the status flip
  and the auth guard also reads `is_active` on each request — so an
  in-flight token stops working immediately.

---

## Catalog search surface

Requester-facing catalog search is wired end-to-end through:

- `GET /api/categories` — flat list; `parent_id` reconstructs the tree.
- `GET /api/tags` — flat name list.
- `GET /api/services/search` — accepts: `q`, `min_price`, `max_price`,
  `min_rating`, `user_zip`, `sort`, **`available_from`**, **`available_to`**,
  **`categories=<csv>`** (service must match **all**), **`tags=<csv>`**
  (service must match **any**), `limit`, `offset`.

The Dioxus catalog page (`frontend/src/pages/catalog.rs`) exposes every
filter above in the UI and reads `GET /api/categories` + `GET /api/tags`
to render selectable checkbox lists. The URL builder is the shared
`frontend_core::search::build_search_path` and is unit-tested.

---

## Constraints

- No external services (offline / on-premise only).
- No background external jobs.
- All file handling local (`config.storage.*` paths).
- RBAC enforced at API boundary.
- Deterministic responses (fixed ordering + explicit tiebreakers).
- Audit chain (`event_log`) makes reviews / reports / face / warehouse
  mutations tamper-evident.
