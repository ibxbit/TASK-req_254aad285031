//! Endpoint smoke: every registered route must either be `/health` (the
//! public entry) or reject unauthenticated calls with 401.
//!
//! This is the guardrail ensuring endpoint surface doesn't silently drift.
//! Each entry is explicit `(method, path, body_or_none)` so catalog of
//! endpoints lives side-by-side with what gets exercised. Contract-level
//! coverage for individual endpoints lives under `tests/contract_*.rs`.
//!
//! Current surface: 91 total endpoints (88 protected + 3 public).
//! Smoke catalog covers all 88 protected endpoints.

use api_tests::{api_base, client, skip_if_offline};
use reqwest::Method;
use serde_json::{json, Value};

const PLACEHOLDER_UUID: &str = "00000000-0000-0000-0000-000000000000";

fn assert_401(method: Method, path: &str, body: Option<Value>) {
    let url = format!("{}{}", api_base(), path);
    let c = client();
    let resp = if let Some(b) = body {
        c.request(method.clone(), &url).json(&b).send()
    } else {
        c.request(method.clone(), &url).send()
    }
    .unwrap_or_else(|e| panic!("{method} {path}: request failed: {e}"));
    assert_eq!(
        resp.status().as_u16(),
        401,
        "{method} {path}: expected 401, got {}",
        resp.status()
    );
}

// The full endpoint catalog. Paths use a placeholder UUID for <id> slots
// so routing itself resolves (no 404-at-router) and the auth guard is the
// first guard to fire.
fn catalog() -> Vec<(Method, String, Option<Value>)> {
    let body = |b: Value| Some(b);
    vec![
        // ---------- Auth ----------
        (Method::POST, "/api/auth/logout".into(), None),
        (Method::GET, "/api/auth/me".into(), None),
        // ---------- Admin / users ----------
        (Method::GET, "/api/admin/users".into(), None),
        (
            Method::POST,
            "/api/admin/users".into(),
            body(json!({"username":"x","password":"x","role":"requester"})),
        ),
        (
            Method::PATCH,
            format!("/api/admin/users/{PLACEHOLDER_UUID}/role"),
            body(json!({"role":"requester"})),
        ),
        (
            Method::PATCH,
            format!("/api/admin/users/{PLACEHOLDER_UUID}/password"),
            body(json!({"password":"x"})),
        ),
        (
            Method::PATCH,
            format!("/api/admin/users/{PLACEHOLDER_UUID}/status"),
            body(json!({"is_active":false})),
        ),
        (
            Method::PUT,
            format!("/api/admin/users/{PLACEHOLDER_UUID}/sensitive"),
            body(json!({"value":"v"})),
        ),
        // ---------- Admin / teams ----------
        (Method::GET, "/api/admin/teams".into(), None),
        (
            Method::POST,
            "/api/admin/teams".into(),
            body(json!({"name":"x"})),
        ),
        (
            Method::DELETE,
            format!("/api/admin/teams/{PLACEHOLDER_UUID}"),
            None,
        ),
        (
            Method::GET,
            format!("/api/admin/teams/{PLACEHOLDER_UUID}/members"),
            None,
        ),
        (
            Method::POST,
            format!("/api/admin/teams/{PLACEHOLDER_UUID}/members"),
            body(json!({"user_id":PLACEHOLDER_UUID})),
        ),
        (
            Method::DELETE,
            format!("/api/admin/teams/{PLACEHOLDER_UUID}/members/{PLACEHOLDER_UUID}"),
            None,
        ),
        // ---------- Forum / zones ----------
        (Method::GET, "/api/zones".into(), None),
        (Method::POST, "/api/zones".into(), body(json!({"name":"x"}))),
        (
            Method::PATCH,
            format!("/api/zones/{PLACEHOLDER_UUID}"),
            body(json!({"name":"x"})),
        ),
        (
            Method::DELETE,
            format!("/api/zones/{PLACEHOLDER_UUID}"),
            None,
        ),
        // ---------- Forum / boards ----------
        (Method::GET, "/api/boards".into(), None),
        (Method::GET, format!("/api/boards/{PLACEHOLDER_UUID}"), None),
        (
            Method::POST,
            "/api/boards".into(),
            body(json!({"zone_id":PLACEHOLDER_UUID,"name":"x","visibility_type":"public"})),
        ),
        (
            Method::PATCH,
            format!("/api/boards/{PLACEHOLDER_UUID}"),
            body(json!({})),
        ),
        (
            Method::DELETE,
            format!("/api/boards/{PLACEHOLDER_UUID}"),
            None,
        ),
        (
            Method::POST,
            format!("/api/boards/{PLACEHOLDER_UUID}/moderators"),
            body(json!({"user_id":PLACEHOLDER_UUID})),
        ),
        (
            Method::DELETE,
            format!("/api/boards/{PLACEHOLDER_UUID}/moderators/{PLACEHOLDER_UUID}"),
            None,
        ),
        (
            Method::GET,
            format!("/api/boards/{PLACEHOLDER_UUID}/rules"),
            None,
        ),
        (
            Method::POST,
            format!("/api/boards/{PLACEHOLDER_UUID}/rules"),
            body(json!({"content":"x"})),
        ),
        (
            Method::DELETE,
            format!("/api/rules/{PLACEHOLDER_UUID}"),
            None,
        ),
        (
            Method::POST,
            format!("/api/boards/{PLACEHOLDER_UUID}/teams"),
            body(json!({"team_id":PLACEHOLDER_UUID})),
        ),
        (
            Method::DELETE,
            format!("/api/boards/{PLACEHOLDER_UUID}/teams/{PLACEHOLDER_UUID}"),
            None,
        ),
        // ---------- Forum / posts ----------
        (
            Method::GET,
            format!("/api/boards/{PLACEHOLDER_UUID}/posts"),
            None,
        ),
        (Method::GET, format!("/api/posts/{PLACEHOLDER_UUID}"), None),
        (
            Method::POST,
            "/api/posts".into(),
            body(json!({"board_id":PLACEHOLDER_UUID,"title":"t","content":"c"})),
        ),
        (
            Method::PATCH,
            format!("/api/posts/{PLACEHOLDER_UUID}/pin"),
            body(json!({"is_pinned":true})),
        ),
        // ---------- Forum / comments ----------
        (
            Method::GET,
            format!("/api/posts/{PLACEHOLDER_UUID}/comments"),
            None,
        ),
        (
            Method::POST,
            "/api/comments".into(),
            body(json!({"post_id":PLACEHOLDER_UUID,"content":"c"})),
        ),
        (
            Method::DELETE,
            format!("/api/comments/{PLACEHOLDER_UUID}"),
            None,
        ),
        // ---------- Catalog / services ----------
        (
            Method::POST,
            "/api/services".into(),
            body(
                json!({"name":"x","description":"y","price":1.0,"coverage_radius_miles":0,"zip_code":"00000"}),
            ),
        ),
        (
            Method::PATCH,
            format!("/api/services/{PLACEHOLDER_UUID}"),
            body(json!({})),
        ),
        (
            Method::GET,
            format!("/api/services/{PLACEHOLDER_UUID}"),
            None,
        ),
        (Method::GET, "/api/services/search".into(), None),
        (Method::GET, "/api/services/compare".into(), None),
        (
            Method::POST,
            format!("/api/services/{PLACEHOLDER_UUID}/favorite"),
            None,
        ),
        // ---------- Catalog / categories + tags + availability ----------
        (Method::GET, "/api/categories".into(), None),
        (
            Method::POST,
            "/api/categories".into(),
            body(json!({"name":"c"})),
        ),
        (
            Method::POST,
            format!("/api/services/{PLACEHOLDER_UUID}/categories"),
            body(json!({"category_id":PLACEHOLDER_UUID})),
        ),
        (Method::GET, "/api/tags".into(), None),
        (Method::POST, "/api/tags".into(), body(json!({"name":"t"}))),
        (
            Method::POST,
            format!("/api/services/{PLACEHOLDER_UUID}/tags"),
            body(json!({"tag_id":PLACEHOLDER_UUID})),
        ),
        (
            Method::POST,
            format!("/api/services/{PLACEHOLDER_UUID}/availability"),
            body(json!({"start_time":"2026-05-01T10:00:00","end_time":"2026-05-01T12:00:00"})),
        ),
        // ---------- Work orders ----------
        (
            Method::POST,
            "/api/work-orders".into(),
            body(json!({"service_id":PLACEHOLDER_UUID})),
        ),
        (
            Method::GET,
            format!("/api/work-orders/{PLACEHOLDER_UUID}"),
            None,
        ),
        (
            Method::POST,
            format!("/api/work-orders/{PLACEHOLDER_UUID}/complete"),
            None,
        ),
        // ---------- Reviews ----------
        (
            Method::POST,
            "/api/reviews".into(),
            body(json!({"work_order_id":PLACEHOLDER_UUID,"rating":5,"text":"x"})),
        ),
        (
            Method::POST,
            format!("/api/work-orders/{PLACEHOLDER_UUID}/follow-up-review"),
            body(json!({"rating":5,"text":"x"})),
        ),
        (
            Method::GET,
            format!("/api/services/{PLACEHOLDER_UUID}/reviews"),
            None,
        ),
        (
            Method::PATCH,
            format!("/api/reviews/{PLACEHOLDER_UUID}/pin"),
            body(json!({"is_pinned":true})),
        ),
        (
            Method::PATCH,
            format!("/api/reviews/{PLACEHOLDER_UUID}/collapse"),
            body(json!({"is_collapsed":true})),
        ),
        (Method::GET, "/api/review-tags".into(), None),
        (
            Method::POST,
            "/api/review-tags".into(),
            body(json!({"name":"t"})),
        ),
        (
            Method::POST,
            format!("/api/reviews/{PLACEHOLDER_UUID}/tags"),
            body(json!({"tag_id":PLACEHOLDER_UUID})),
        ),
        (
            Method::POST,
            format!("/api/reviews/{PLACEHOLDER_UUID}/images"),
            None,
        ),
        (
            Method::GET,
            format!("/api/services/{PLACEHOLDER_UUID}/reputation"),
            None,
        ),
        // ---------- Internships ----------
        (
            Method::POST,
            "/api/internships/plans".into(),
            body(json!({"content":"x"})),
        ),
        (
            Method::POST,
            "/api/reports".into(),
            body(json!({"type":"DAILY","content":"x"})),
        ),
        (
            Method::POST,
            format!("/api/reports/{PLACEHOLDER_UUID}/comments"),
            body(json!({"content":"x"})),
        ),
        (
            Method::POST,
            format!("/api/reports/{PLACEHOLDER_UUID}/approve"),
            None,
        ),
        (
            Method::POST,
            format!("/api/reports/{PLACEHOLDER_UUID}/attachments"),
            None,
        ),
        (
            Method::GET,
            format!("/api/interns/{PLACEHOLDER_UUID}/dashboard"),
            None,
        ),
        // ---------- Warehouse ----------
        (
            Method::POST,
            "/api/warehouses".into(),
            body(json!({"name":"x"})),
        ),
        (
            Method::PATCH,
            format!("/api/warehouses/{PLACEHOLDER_UUID}"),
            body(json!({"name":"y"})),
        ),
        (
            Method::DELETE,
            format!("/api/warehouses/{PLACEHOLDER_UUID}"),
            None,
        ),
        (
            Method::GET,
            format!("/api/warehouses/{PLACEHOLDER_UUID}/history"),
            None,
        ),
        (Method::GET, "/api/warehouses/tree".into(), None),
        (
            Method::POST,
            "/api/warehouse-zones".into(),
            body(json!({"warehouse_id":PLACEHOLDER_UUID,"name":"x"})),
        ),
        (
            Method::PATCH,
            format!("/api/warehouse-zones/{PLACEHOLDER_UUID}"),
            body(json!({"name":"y"})),
        ),
        (
            Method::DELETE,
            format!("/api/warehouse-zones/{PLACEHOLDER_UUID}"),
            None,
        ),
        (
            Method::GET,
            format!("/api/warehouse-zones/{PLACEHOLDER_UUID}/history"),
            None,
        ),
        (
            Method::POST,
            "/api/bins".into(),
            body(
                json!({"zone_id":PLACEHOLDER_UUID,"name":"x","width_in":1.0,"height_in":1.0,"depth_in":1.0,"max_load_lbs":0.0,"temp_zone":"ambient"}),
            ),
        ),
        (
            Method::PATCH,
            format!("/api/bins/{PLACEHOLDER_UUID}"),
            body(json!({})),
        ),
        (
            Method::GET,
            format!("/api/bins/{PLACEHOLDER_UUID}/history"),
            None,
        ),
        // ---------- Face ----------
        (Method::POST, "/api/faces".into(), None),
        (
            Method::POST,
            format!("/api/faces/{PLACEHOLDER_UUID}/validate"),
            None,
        ),
        (
            Method::POST,
            format!("/api/faces/{PLACEHOLDER_UUID}/liveness"),
            body(json!({"challenge":"blink","passed":true})),
        ),
        (
            Method::POST,
            format!("/api/faces/{PLACEHOLDER_UUID}/deactivate"),
            None,
        ),
        (Method::GET, format!("/api/faces/{PLACEHOLDER_UUID}"), None),
        // ---------- Audit ----------
        (Method::GET, "/api/audit/verify".into(), None),
        (
            Method::GET,
            format!("/api/audit/review/{PLACEHOLDER_UUID}"),
            None,
        ),
    ]
}

#[test]
fn every_protected_endpoint_requires_auth() {
    if !skip_if_offline("every_protected_endpoint_requires_auth") {
        return;
    }
    let entries = catalog();
    // Lower bound lets the catalog grow; higher bound catches silent
    // endpoint removals from this guardrail.
    assert!(
        entries.len() >= 80 && entries.len() <= 200,
        "endpoint catalog length drift: {}",
        entries.len()
    );
    eprintln!(
        "endpoint_smoke: exercising {} protected endpoints",
        entries.len()
    );
    for (method, path, body) in entries {
        assert_401(method, &path, body);
    }
}

// ---------- Unauthenticated public endpoints ----------

#[test]
fn health_is_public_and_returns_ok_body() {
    if !skip_if_offline("health_is_public_and_returns_ok_body") {
        return;
    }
    let resp = api_tests::get("/api/health").expect("get");
    api_tests::assert_status(&resp, 200, "GET /api/health");
    let v = api_tests::json_body(resp, "GET /api/health");
    assert_eq!(v["status"].as_str(), Some("ok"));
}

#[test]
fn register_is_public_route_with_400_or_403() {
    if !skip_if_offline("register_is_public_route_with_400_or_403") {
        return;
    }
    // 2xx (bootstrap open) or 403 (closed) — both prove the endpoint is
    // reachable without a token. A 401 here would be a regression.
    let resp = api_tests::post_json(
        "/api/auth/register",
        &serde_json::json!({"username":"x","password":"x"}),
    )
    .expect("post");
    let c = resp.status().as_u16();
    assert!(
        matches!(c, 201 | 400 | 403 | 409),
        "register unexpected status {c}"
    );
}

#[test]
fn login_is_public_route_with_401_for_unknown_user() {
    if !skip_if_offline("login_is_public_route_with_401_for_unknown_user") {
        return;
    }
    let resp = api_tests::post_json(
        "/api/auth/login",
        &serde_json::json!({"username":"nope_nope_xxx","password":"whatever12345"}),
    )
    .expect("post");
    api_tests::assert_status(&resp, 401, "unknown login must be 401");
}

// ============================================================
// Cross-domain authorized happy-path response contract checks
// ============================================================
//
// The smoke tests above only verify that endpoints require auth.
// These tests verify that, given a valid token, representative endpoints
// across all domains return 200 with parseable (and structurally correct)
// response bodies. This closes the observability gap: a route can exist and
// require auth but still be broken under valid credentials.

#[test]
fn authorized_cross_domain_endpoints_return_parseable_bodies() {
    let Some(admin) = api_tests::setup("smoke_authorized_cross_domain") else {
        return;
    };

    // Auth domain
    let me = api_tests::get_auth("/api/auth/me", &admin).expect("me");
    api_tests::assert_status(&me, 200, "GET /auth/me");
    let me_v = api_tests::json_body(me, "me");
    api_tests::assert_keys(&me_v, &["id", "username", "role"], "/auth/me contract");

    // Catalog domain – search returns array
    let search = api_tests::get_auth("/api/services/search", &admin).expect("search");
    api_tests::assert_status(&search, 200, "GET /services/search");
    assert!(
        api_tests::json_body(search, "services/search").is_array(),
        "services/search must return JSON array"
    );

    // Catalog domain – categories and tags return arrays
    let cats = api_tests::get_auth("/api/categories", &admin).expect("cats");
    api_tests::assert_status(&cats, 200, "GET /categories");
    assert!(
        api_tests::json_body(cats, "categories").is_array(),
        "categories must return JSON array"
    );

    let tags = api_tests::get_auth("/api/tags", &admin).expect("tags");
    api_tests::assert_status(&tags, 200, "GET /tags");
    assert!(
        api_tests::json_body(tags, "tags").is_array(),
        "tags must return JSON array"
    );

    // Forum domain – boards returns array
    let boards = api_tests::get_auth("/api/boards", &admin).expect("boards");
    api_tests::assert_status(&boards, 200, "GET /boards");
    assert!(
        api_tests::json_body(boards, "boards").is_array(),
        "boards must return JSON array"
    );

    // Admin domain – users list returns non-empty array with user contract
    let users = api_tests::get_auth("/api/admin/users", &admin).expect("users");
    api_tests::assert_status(&users, 200, "GET /admin/users");
    let users_v = api_tests::json_body(users, "admin/users");
    assert!(users_v.is_array(), "admin/users must return JSON array");
    let arr = users_v.as_array().unwrap();
    assert!(
        !arr.is_empty(),
        "admin/users must contain at least the bootstrap admin"
    );
    api_tests::assert_keys(
        &arr[0],
        &["id", "username", "role", "is_active"],
        "user contract",
    );

    // Warehouse domain – tree returns parseable JSON (may be empty)
    let tree = api_tests::get_auth("/api/warehouses/tree", &admin).expect("tree");
    api_tests::assert_status(&tree, 200, "GET /warehouses/tree");
    let _ = api_tests::json_body(tree, "warehouses/tree");

    // Audit domain – verify returns parseable JSON
    let audit = api_tests::get_auth("/api/audit/verify", &admin).expect("audit");
    api_tests::assert_status(&audit, 200, "GET /audit/verify");
    let _ = api_tests::json_body(audit, "audit/verify");

    // Review tags – returns array
    let rtags = api_tests::get_auth("/api/review-tags", &admin).expect("rtags");
    api_tests::assert_status(&rtags, 200, "GET /review-tags");
    assert!(
        api_tests::json_body(rtags, "review-tags").is_array(),
        "review-tags must return JSON array"
    );

    // Zones – returns array
    let zones = api_tests::get_auth("/api/zones", &admin).expect("zones");
    api_tests::assert_status(&zones, 200, "GET /zones");
    assert!(
        api_tests::json_body(zones, "zones").is_array(),
        "zones must return JSON array"
    );
}

#[test]
fn unauthorized_response_format_is_consistent_across_domains() {
    if !skip_if_offline("smoke_unauthorized_format") {
        return;
    }
    // Pick representative endpoints from distinct domains and verify
    // that all return exactly 401 (not 403, 500, or redirect) without a token.
    // This guards against domain-specific auth misconfiguration.
    let probe_endpoints: &[(&str, Option<serde_json::Value>)] = &[
        ("/api/auth/me", None),
        ("/api/admin/users", None),
        ("/api/boards", None),
        ("/api/services/search", None),
        ("/api/warehouses/tree", None),
        ("/api/audit/verify", None),
        ("/api/review-tags", None),
        ("/api/zones", None),
    ];
    for (path, _body) in probe_endpoints {
        let resp = api_tests::get(path).expect(path);
        assert_eq!(
            resp.status().as_u16(),
            401,
            "unauthenticated {path} must return 401, got {}",
            resp.status()
        );
    }
}
