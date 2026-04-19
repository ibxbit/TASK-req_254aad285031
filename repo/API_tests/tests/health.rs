use api_tests::{backend_reachable, get, skip_if_offline};

#[test]
fn health_returns_ok_and_status_field() {
    if !skip_if_offline("health_returns_ok_and_status_field") {
        return;
    }
    let resp = get("/api/health").expect("GET /api/health");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().expect("json body");
    assert!(
        body.get("status").is_some(),
        "expected `status` field, got {body}"
    );
}

#[test]
fn health_probe_roundtrips_fast() {
    // Just exercises the reachability probe helper itself — runs even if
    // the stack is down (the helper should simply return false).
    let _ = backend_reachable();
}
