use api_tests::{get, post_json, skip_if_offline};
use serde_json::json;

#[test]
fn register_returns_201_or_403() {
    if !skip_if_offline("register_returns_201_or_403") {
        return;
    }
    // Username is unique-per-run so repeated test runs don't collide on the
    // UNIQUE(username) constraint.
    let uname = format!("apitest_{}", nano_suffix());
    let body = json!({ "username": uname, "password": "longpassword1234" });
    let resp = post_json("/api/auth/register", &body).expect("register");
    // 201 = first-user bootstrap; 403 = users table already populated.
    assert!(
        resp.status() == 201 || resp.status() == 403,
        "expected 201 or 403, got {}",
        resp.status()
    );
}

#[test]
fn login_unknown_user_returns_401() {
    if !skip_if_offline("login_unknown_user_returns_401") {
        return;
    }
    let body = json!({ "username": "no_such_user_xyz_12345", "password": "whatever12345" });
    let resp = post_json("/api/auth/login", &body).expect("login");
    assert_eq!(resp.status(), 401);
}

#[test]
fn me_without_bearer_returns_401() {
    if !skip_if_offline("me_without_bearer_returns_401") {
        return;
    }
    let resp = get("/api/auth/me").expect("me");
    assert_eq!(resp.status(), 401);
}

#[test]
fn me_with_malformed_token_returns_401() {
    if !skip_if_offline("me_with_malformed_token_returns_401") {
        return;
    }
    let resp = api_tests::get_auth("/api/auth/me", "not-a-real-token").expect("me");
    assert_eq!(resp.status(), 401);
}

#[test]
fn register_rejects_short_password() {
    if !skip_if_offline("register_rejects_short_password") {
        return;
    }
    let body = json!({ "username": "x", "password": "short" });
    let resp = post_json("/api/auth/register", &body).expect("register");
    // 400 when bootstrap slot is open (validation rejects it); 403 once
    // another admin exists. Either is a correct rejection.
    let code = resp.status().as_u16();
    assert!(
        matches!(code, 400 | 403),
        "expected 400 or 403 for short password, got {code}"
    );
}

fn nano_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}
