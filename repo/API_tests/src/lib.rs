//! `api_tests` — Rust HTTP integration tests against a live backend.
//!
//! Tests in `tests/*.rs` use `reqwest::blocking` to hit the Rocket API on
//! `$API_BASE` (default `http://127.0.0.1:8000`). If the backend is not
//! reachable, tests gracefully *skip* (print "SKIP …" and return) rather
//! than failing — so `cargo test --workspace` on a fresh checkout still
//! passes even without `docker compose up`.
//!
//! ## Coverage policy
//!
//! Two axes:
//!
//! 1. **HTTP smoke coverage.** `tests/endpoint_smoke.rs` issues an
//!    unauthenticated call against every registered route and asserts the
//!    expected "not logged in" class (401/403). This gives a one-stop
//!    guardrail on the mount surface: no endpoint can silently go missing.
//! 2. **Contract coverage.** `tests/contract_*.rs` log in as real users,
//!    exercise the success path, and parse the response body to validate
//!    the field contract (not just status). This is the no-mock axis.
//!
//! ## Bootstrap + provisioning helpers
//!
//! `bootstrap_admin_token()` yields an admin token when one can be had,
//! and `None` otherwise — dependent tests skip with a `SKIP …` log rather
//! than failing. Resolution order:
//!
//! * `API_ADMIN_TOKEN` env var (pre-provisioned)
//! * `API_ADMIN_USER` / `API_ADMIN_PASS` env (login with explicit creds)
//! * `admin` / `change-me-please-now` (documented quickstart pair)
//! * `POST /auth/register` if users table is still empty
//!
//! `provision_user(token, role)` creates a role-tagged user via the admin
//! API and returns `(id, username, password)` for subsequent login.

use reqwest::blocking::{Client, Response};
use serde_json::{json, Value};
use std::time::Duration;

pub fn api_base() -> String {
    std::env::var("API_BASE").unwrap_or_else(|_| "http://127.0.0.1:8000".into())
}

pub fn client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("reqwest client")
}

pub fn backend_reachable() -> bool {
    let url = format!("{}/api/health", api_base());
    Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map(|c| {
            c.get(url)
                .send()
                .map(|r| r.status().is_success())
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

/// Strict mode is enabled by `API_TESTS_STRICT=1` (or `true`). In strict
/// mode any prerequisite failure — backend unreachable, admin token
/// unavailable — panics the test instead of skipping it. Use this in CI
/// where a green run must imply the full suite actually executed.
pub fn strict_mode() -> bool {
    matches!(
        std::env::var("API_TESTS_STRICT")
            .unwrap_or_default()
            .as_str(),
        "1" | "true" | "TRUE" | "yes"
    )
}

pub fn skip_if_offline(test_name: &str) -> bool {
    if !backend_reachable() {
        if strict_mode() {
            panic!(
                "STRICT {test_name}: backend not reachable at {} (set API_TESTS_STRICT=0 to allow skip)",
                api_base()
            );
        }
        eprintln!("SKIP {test_name}: backend not reachable at {}", api_base());
        return false;
    }
    true
}

// -------- request helpers --------

pub fn get(path: &str) -> reqwest::Result<Response> {
    client().get(format!("{}{}", api_base(), path)).send()
}

pub fn post_json<B: serde::Serialize>(path: &str, body: &B) -> reqwest::Result<Response> {
    client()
        .post(format!("{}{}", api_base(), path))
        .json(body)
        .send()
}

pub fn post_empty(path: &str) -> reqwest::Result<Response> {
    client().post(format!("{}{}", api_base(), path)).send()
}

pub fn patch_json<B: serde::Serialize>(path: &str, body: &B) -> reqwest::Result<Response> {
    client()
        .patch(format!("{}{}", api_base(), path))
        .json(body)
        .send()
}

pub fn put_json<B: serde::Serialize>(path: &str, body: &B) -> reqwest::Result<Response> {
    client()
        .put(format!("{}{}", api_base(), path))
        .json(body)
        .send()
}

pub fn delete(path: &str) -> reqwest::Result<Response> {
    client().delete(format!("{}{}", api_base(), path)).send()
}

pub fn get_auth(path: &str, token: &str) -> reqwest::Result<Response> {
    client()
        .get(format!("{}{}", api_base(), path))
        .bearer_auth(token)
        .send()
}

pub fn post_json_auth<B: serde::Serialize>(
    path: &str,
    token: &str,
    body: &B,
) -> reqwest::Result<Response> {
    client()
        .post(format!("{}{}", api_base(), path))
        .bearer_auth(token)
        .json(body)
        .send()
}

pub fn post_empty_auth(path: &str, token: &str) -> reqwest::Result<Response> {
    client()
        .post(format!("{}{}", api_base(), path))
        .bearer_auth(token)
        .send()
}

pub fn patch_json_auth<B: serde::Serialize>(
    path: &str,
    token: &str,
    body: &B,
) -> reqwest::Result<Response> {
    client()
        .patch(format!("{}{}", api_base(), path))
        .bearer_auth(token)
        .json(body)
        .send()
}

pub fn put_json_auth<B: serde::Serialize>(
    path: &str,
    token: &str,
    body: &B,
) -> reqwest::Result<Response> {
    client()
        .put(format!("{}{}", api_base(), path))
        .bearer_auth(token)
        .json(body)
        .send()
}

pub fn delete_auth(path: &str, token: &str) -> reqwest::Result<Response> {
    client()
        .delete(format!("{}{}", api_base(), path))
        .bearer_auth(token)
        .send()
}

// -------- assertion helpers --------

pub fn assert_status(resp: &Response, expected: u16, ctx: &str) {
    assert_eq!(
        resp.status().as_u16(),
        expected,
        "{ctx}: expected HTTP {expected}, got {}",
        resp.status()
    );
}

/// Parses the response body as JSON and returns it, panicking with a
/// contextual message on failure. Use this instead of raw `.json()` so
/// failures point at the caller.
pub fn json_body(resp: Response, ctx: &str) -> Value {
    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    serde_json::from_str(&text).unwrap_or_else(|e| {
        panic!("{ctx}: non-JSON response (status {status}): {e} — body: {text}")
    })
}

/// Assert the JSON object contains the listed string keys. Catches silent
/// DTO field removals.
pub fn assert_keys(v: &Value, keys: &[&str], ctx: &str) {
    let obj = v
        .as_object()
        .unwrap_or_else(|| panic!("{ctx}: expected JSON object, got {v}"));
    for k in keys {
        assert!(
            obj.contains_key(*k),
            "{ctx}: missing key `{k}` in JSON: {v}"
        );
    }
}

// -------- bootstrap / login helpers --------

pub fn login(username: &str, password: &str) -> Option<String> {
    let body = json!({ "username": username, "password": password });
    let resp = post_json("/api/auth/login", &body).ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: Value = resp.json().ok()?;
    v.get("token")?.as_str().map(|s| s.to_string())
}

pub fn bootstrap_admin_token() -> Option<String> {
    if let Ok(t) = std::env::var("API_ADMIN_TOKEN") {
        if !t.is_empty() {
            return Some(t);
        }
    }
    if let (Ok(u), Ok(p)) = (
        std::env::var("API_ADMIN_USER"),
        std::env::var("API_ADMIN_PASS"),
    ) {
        if let Some(t) = login(&u, &p) {
            return Some(t);
        }
    }
    if let Some(t) = login("admin", "change-me-please-now") {
        return Some(t);
    }
    let body = json!({ "username": "admin", "password": "change-me-please-now" });
    let _ = post_json("/api/auth/register", &body);
    login("admin", "change-me-please-now")
}

pub fn provision_user(admin_token: &str, role: &str) -> Option<(String, String, String)> {
    let suffix = nano_suffix();
    let username = format!("apit_{role}_{suffix}");
    let password = "verifypass123!".to_string();
    let body = json!({
        "username": username,
        "password": password,
        "role": role,
    });
    let resp = post_json_auth("/api/admin/users", admin_token, &body).ok()?;
    if !resp.status().is_success() {
        eprintln!(
            "provision_user({role}) failed: HTTP {} — cannot run dependent test",
            resp.status()
        );
        return None;
    }
    let v: Value = resp.json().ok()?;
    let id = v.get("id")?.as_str()?.to_string();
    Some((id, username, password))
}

/// Shorthand for tests that need backend + admin. Returns the admin token
/// or prints `SKIP` and yields None. In strict mode (API_TESTS_STRICT=1),
/// missing prerequisites panic instead of skipping.
pub fn setup(name: &str) -> Option<String> {
    if !skip_if_offline(name) {
        return None;
    }
    match bootstrap_admin_token() {
        Some(t) => Some(t),
        None => {
            if strict_mode() {
                panic!(
                    "STRICT {name}: no administrator token available (set API_ADMIN_TOKEN or bootstrap admin/change-me-please-now)"
                );
            }
            eprintln!("SKIP {name}: no administrator token available");
            None
        }
    }
}

pub fn nano_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// Convenience: create a fresh service via admin. Returns the service id.
pub fn create_service(admin: &str, name_prefix: &str) -> Option<String> {
    let body = json!({
        "name": format!("{name_prefix}_{}", nano_suffix()),
        "description": "integration-test service",
        "price": 0.0,
        "coverage_radius_miles": 0,
        "zip_code": "00000",
    });
    let resp = post_json_auth("/api/services", admin, &body).ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: Value = resp.json().ok()?;
    v.get("id")?.as_str().map(|s| s.to_string())
}
