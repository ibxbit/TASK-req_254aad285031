//! RBAC: every protected endpoint must reject unauthenticated calls with 401.
//!
//! Broad role-matrix tests (admin vs requester, etc.) need provisioned users
//! and live data; those belong in a fuller end-to-end suite and are out of
//! scope here.

use api_tests::{client, skip_if_offline};
use serde_json::json;

fn assert_401(method: reqwest::Method, path: &str, body: Option<serde_json::Value>) {
    let url = format!("{}{}", api_tests::api_base(), path);
    let c = client();
    let resp = if let Some(b) = body {
        c.request(method, &url).json(&b).send()
    } else {
        c.request(method, &url).send()
    }
    .unwrap_or_else(|e| panic!("request failed: {e}"));
    assert_eq!(
        resp.status(),
        401,
        "expected 401 for unauthenticated {} {}, got {}",
        resp.status().as_str(),
        path,
        resp.status()
    );
}

#[test]
fn unauthenticated_requests_return_401() {
    if !skip_if_offline("unauthenticated_requests_return_401") {
        return;
    }

    assert_401(reqwest::Method::GET, "/api/services/search", None);
    assert_401(
        reqwest::Method::POST,
        "/api/services",
        Some(json!({"name": "x"})),
    );
    assert_401(
        reqwest::Method::POST,
        "/api/warehouses",
        Some(json!({"name": "x"})),
    );
    assert_401(
        reqwest::Method::POST,
        "/api/reviews",
        Some(json!({"rating": 5})),
    );
    assert_401(
        reqwest::Method::POST,
        "/api/reports",
        Some(json!({"type": "DAILY"})),
    );
    assert_401(reqwest::Method::GET, "/api/warehouses/tree", None);
    assert_401(reqwest::Method::GET, "/api/boards", None);
    assert_401(reqwest::Method::GET, "/api/audit/verify", None);
}
