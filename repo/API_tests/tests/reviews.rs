use api_tests::{api_base, client, post_json, skip_if_offline};
use serde_json::json;

#[test]
fn post_reviews_without_token_is_401() {
    if !skip_if_offline("post_reviews_without_token_is_401") {
        return;
    }
    let body = json!({
        "work_order_id": "00000000-0000-0000-0000-000000000000",
        "rating": 3,
        "text": "x",
    });
    let resp = post_json("/api/reviews", &body).expect("post");
    assert_eq!(resp.status(), 401);
}

#[test]
fn post_reviews_malformed_json_rejected() {
    if !skip_if_offline("post_reviews_malformed_json_rejected") {
        return;
    }
    let url = format!("{}/api/reviews", api_base());
    let resp = client()
        .post(&url)
        .header("Content-Type", "application/json")
        .body("not-json")
        .send()
        .expect("post");
    let code = resp.status().as_u16();
    assert!(
        matches!(code, 400 | 401 | 422),
        "malformed JSON should be 400/401/422, got {code}"
    );
}

#[test]
fn reputation_with_bad_uuid_is_rejected() {
    if !skip_if_offline("reputation_with_bad_uuid_is_rejected") {
        return;
    }
    let resp = api_tests::get("/api/services/not-a-uuid/reputation").expect("get");
    let code = resp.status().as_u16();
    // 401 (auth first) or 400 (bad uuid) — both are deterministic rejections.
    assert!(
        matches!(code, 400 | 401),
        "bad uuid reputation should be 400/401, got {code}"
    );
}
