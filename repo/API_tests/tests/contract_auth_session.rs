//! Response-contract assertions for authentication and session endpoints.
//!
//! Verifies:
//!   - POST /api/auth/login   → { token: string, user: { id, username, role } }
//!   - GET  /api/auth/me      → { id, username, role }
//!   - Inactive user login    → 403 (not 401)
//!   - Wrong password         → 401 (distinct from inactive-user 403)
//!   - POST /api/auth/logout  → 204; subsequent GET /auth/me → 401

use api_tests::{
    assert_keys, assert_status, get_auth, json_body, login, nano_suffix, patch_json_auth,
    post_empty_auth, post_json, provision_user, setup,
};
use serde_json::json;

#[test]
fn login_response_has_token_and_user_contract() {
    let Some(admin) = setup("login_response_has_token_and_user_contract") else {
        return;
    };
    let Some((_, username, password)) = provision_user(&admin, "requester") else {
        return;
    };

    let resp = post_json(
        "/api/auth/login",
        &json!({ "username": username, "password": password }),
    )
    .expect("login");
    assert_status(&resp, 200, "POST /api/auth/login");

    let v = json_body(resp, "POST /api/auth/login");
    assert!(
        v["token"].is_string() && !v["token"].as_str().unwrap().is_empty(),
        "token must be a non-empty string: {v}"
    );
    let user = &v["user"];
    assert_keys(user, &["id", "username", "role"], "login user contract");
    assert_eq!(user["username"].as_str(), Some(username.as_str()));
    assert_eq!(user["role"].as_str(), Some("requester"));
}

#[test]
fn me_response_has_id_username_role() {
    let Some(admin) = setup("me_response_has_id_username_role") else {
        return;
    };
    let Some((_, username, password)) = provision_user(&admin, "intern") else {
        return;
    };
    let Some(token) = login(&username, &password) else {
        return;
    };

    let resp = get_auth("/api/auth/me", &token).expect("me");
    assert_status(&resp, 200, "GET /api/auth/me");

    let v = json_body(resp, "GET /api/auth/me");
    assert_keys(&v, &["id", "username", "role"], "me contract");
    assert_eq!(v["username"].as_str(), Some(username.as_str()));
    assert_eq!(v["role"].as_str(), Some("intern"));
    assert!(v["id"].is_string(), "id must be a string (UUID): {v}");
}

#[test]
fn wrong_password_returns_401_not_403() {
    let Some(admin) = setup("wrong_password_returns_401_not_403") else {
        return;
    };
    let Some((_, username, _)) = provision_user(&admin, "requester") else {
        return;
    };

    let resp = post_json(
        "/api/auth/login",
        &json!({ "username": username, "password": "definitely_wrong_password_xyz" }),
    )
    .expect("login attempt");
    assert_status(&resp, 401, "wrong password must be 401");
}

#[test]
fn inactive_user_login_returns_403_not_401() {
    let Some(admin) = setup("inactive_user_login_returns_403_not_401") else {
        return;
    };
    let uname = format!("inactive_{}", nano_suffix());
    let Some((user_id, _, _)) = provision_user(&admin, "requester") else {
        return;
    };

    // Re-provision with known credentials under our chosen username.
    let password = "activepass9999!";
    let create = api_tests::post_json_auth(
        "/api/admin/users",
        &admin,
        &json!({ "username": uname, "password": password, "role": "requester" }),
    )
    .expect("create");
    if create.status() != 200 {
        eprintln!("SKIP inactive_user_login_returns_403_not_401: create failed");
        return;
    }
    let v = json_body(create, "create inactive user");
    let id = v["id"].as_str().unwrap().to_string();

    // Deactivate the user.
    let deact = patch_json_auth(
        &format!("/api/admin/users/{id}/status"),
        &admin,
        &json!({ "is_active": false }),
    )
    .expect("deactivate");
    assert_status(&deact, 204, "deactivate user");

    // Login attempt must return 403 (account inactive), not 401 (bad credentials).
    let resp = post_json(
        "/api/auth/login",
        &json!({ "username": uname, "password": password }),
    )
    .expect("inactive login");
    assert_status(&resp, 403, "inactive user must be 403 not 401");
    let _ = user_id; // suppress unused warning
}

#[test]
fn logout_invalidates_token() {
    let Some(admin) = setup("logout_invalidates_token") else {
        return;
    };
    let Some((_, username, password)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(token) = login(&username, &password) else {
        return;
    };

    // Token works before logout.
    let me_before = get_auth("/api/auth/me", &token).expect("me before");
    assert_status(&me_before, 200, "GET /auth/me before logout");

    // Logout returns 204.
    let logout = post_empty_auth("/api/auth/logout", &token).expect("logout");
    assert_status(&logout, 204, "POST /auth/logout");

    // Token is now invalid.
    let me_after = get_auth("/api/auth/me", &token).expect("me after");
    assert_status(&me_after, 401, "GET /auth/me after logout must be 401");
}

#[test]
fn login_response_token_is_usable_for_bearer_auth() {
    let Some(admin) = setup("login_response_token_is_usable_for_bearer_auth") else {
        return;
    };
    let Some((_, username, password)) = provision_user(&admin, "moderator") else {
        return;
    };

    let login_resp = post_json(
        "/api/auth/login",
        &json!({ "username": username, "password": password }),
    )
    .expect("login");
    assert_status(&login_resp, 200, "login");

    let v = json_body(login_resp, "login token");
    let token = v["token"].as_str().unwrap().to_string();

    // Token extracted from body must work as a Bearer credential.
    let me = get_auth("/api/auth/me", &token).expect("me");
    assert_status(&me, 200, "token from login body must authenticate /auth/me");
    let me_v = json_body(me, "me after token extract");
    assert_eq!(me_v["username"].as_str(), Some(username.as_str()));
}
