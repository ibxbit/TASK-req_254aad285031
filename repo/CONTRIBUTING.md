# Contributing — Native Development Setup

> **This document is for contributors only.** The supported reviewer and
> evaluator workflow is `docker compose up` in `README.md`. Nothing in this
> file is required to run, evaluate, or test the project. Do not use this as
> a setup guide unless you specifically want to run Rust services directly on
> your host.

---

## Prerequisites

- Rust stable (≥ 1.78) + `cargo` — install via [rustup.rs](https://rustup.rs/)
- MySQL 8.0+ reachable on localhost
- Dioxus CLI for the frontend dev loop:
  ```bash
  cargo install dioxus-cli --version 0.5
  ```

## Configure + key generation

```bash
cp config/config.example.toml config/config.toml
# Edit database.url to match your local MySQL user/password.

# Linux / macOS
head -c 32 /dev/urandom > ./config/master.key
chmod 600 ./config/master.key

# Windows PowerShell
$b = New-Object byte[] 32
[System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($b)
[IO.File]::WriteAllBytes("./config/master.key", $b)
```

## Database

```bash
mysql -u root -p < db/schema.sql
```

`schema.sql` is idempotent — safe to re-apply after pulling schema changes.

## Run services

```bash
cd backend && cargo run --release        # terminal 1
cd frontend && dx serve --platform web   # terminal 2
```

Bootstrap admin and provision demo roles with the same curl commands as the
Docker path (steps 2–4 of the verification walkthrough in README.md).

## Run tests natively

```bash
# Unit + frontend rendered-layer tests (no DB or server required)
./unit_tests/run.sh

# API integration tests (requires backend + MySQL running above)
API_TESTS_STRICT=1 ./API_tests/run.sh
```
