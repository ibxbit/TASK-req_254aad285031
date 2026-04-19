use api_tests::{get, skip_if_offline};

#[test]
fn search_without_token_is_401() {
    if !skip_if_offline("search_without_token_is_401") {
        return;
    }
    let resp = get("/api/services/search").expect("get");
    assert_eq!(resp.status(), 401);
}

#[test]
fn compare_missing_ids_rejected() {
    if !skip_if_offline("compare_missing_ids_rejected") {
        return;
    }
    let resp = get("/api/services/compare").expect("get");
    let code = resp.status().as_u16();
    assert!(
        matches!(code, 400 | 401 | 422),
        "compare without ids should be 4xx, got {code}"
    );
}
