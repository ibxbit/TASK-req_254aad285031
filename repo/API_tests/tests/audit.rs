use api_tests::{get, skip_if_offline};

#[test]
fn verify_endpoint_requires_auth() {
    if !skip_if_offline("verify_endpoint_requires_auth") {
        return;
    }
    let resp = get("/api/audit/verify").expect("get");
    assert_eq!(resp.status(), 401);
}

#[test]
fn per_entity_endpoint_requires_auth() {
    if !skip_if_offline("per_entity_endpoint_requires_auth") {
        return;
    }
    let resp = get("/api/audit/review/00000000-0000-0000-0000-000000000000").expect("get");
    assert_eq!(resp.status(), 401);
}
