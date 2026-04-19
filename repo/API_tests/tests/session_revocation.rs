//! Session revocation + inactive-user guard coverage.
//!
//! Business rule: once an administrator deactivates a user
//! (`PATCH /api/admin/users/<id>/status {is_active:false}`) every
//! outstanding bearer token for that user MUST stop working immediately.
//! The auth guard is the second line of defence — even if a session row
//! somehow survives, a request from a user with `is_active = 0` must
//! 401.
//!
//! Each test provisions its own user so it is independent of any other
//! run. Tests skip cleanly when no admin token is available (or panic in
//! strict mode — see `api_tests::strict_mode`).

use api_tests::{
    assert_status, get_auth, json_body, login, patch_json_auth, provision_user, setup,
};
use serde_json::json;

#[test]
fn active_user_can_use_token_and_loses_it_after_deactivation() {
    let Some(admin) = setup("active_user_can_use_token_and_loses_it_after_deactivation") else {
        return;
    };
    let Some((user_id, u, p)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(tok) = login(&u, &p) else {
        eprintln!("SKIP: login failed for provisioned user");
        return;
    };

    // ---- Before deactivation: token works ----
    let before = get_auth("/api/auth/me", &tok).expect("me-before");
    assert_status(&before, 200, "GET /auth/me before deactivation");
    let v = json_body(before, "me-before");
    assert_eq!(
        v["id"], user_id,
        "/auth/me must return the provisioned user's id"
    );
    assert_eq!(v["username"], u);
    assert_eq!(v["role"], "requester");

    // ---- Deactivate via admin ----
    let deact = patch_json_auth(
        &format!("/api/admin/users/{user_id}/status"),
        &admin,
        &json!({"is_active": false}),
    )
    .expect("deactivate");
    assert_status(&deact, 204, "PATCH /admin/users/<id>/status deactivate");

    // ---- After deactivation: same token is 401 on every protected route ----
    // The critical check: /auth/me is the simplest authed surface.
    let after_me = get_auth("/api/auth/me", &tok).expect("me-after");
    assert_status(
        &after_me,
        401,
        "GET /auth/me after deactivation must 401 (session revoked + is_active check)",
    );

    // Verify a second protected endpoint to prove the 401 is not a quirk
    // of a single handler — the guard itself is the thing that fires.
    let after_catalog = get_auth("/api/services/search", &tok).expect("search-after");
    assert_status(
        &after_catalog,
        401,
        "protected GET /services/search after deactivation must 401",
    );
}

#[test]
fn deactivated_user_cannot_log_in_again() {
    let Some(admin) = setup("deactivated_user_cannot_log_in_again") else {
        return;
    };
    let Some((user_id, u, p)) = provision_user(&admin, "requester") else {
        return;
    };

    // Deactivate.
    let deact = patch_json_auth(
        &format!("/api/admin/users/{user_id}/status"),
        &admin,
        &json!({"is_active": false}),
    )
    .expect("d");
    assert_status(&deact, 204, "deactivate");

    // Login attempt -> 403 (backend returns Forbidden for inactive
    // accounts; see backend/src/routes/auth.rs).
    let body = serde_json::json!({ "username": u, "password": p });
    let resp = api_tests::post_json("/api/auth/login", &body).expect("login");
    assert_status(
        &resp,
        403,
        "login for deactivated user must 403 (user_inactive)",
    );
}

#[test]
fn reactivating_user_restores_login_but_old_tokens_stay_revoked() {
    let Some(admin) = setup("reactivating_user_restores_login_but_old_tokens_stay_revoked") else {
        return;
    };
    let Some((user_id, u, p)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(old_tok) = login(&u, &p) else {
        return;
    };

    // Deactivate -> old token dies.
    let _ = patch_json_auth(
        &format!("/api/admin/users/{user_id}/status"),
        &admin,
        &json!({"is_active": false}),
    )
    .expect("d");

    let dead = get_auth("/api/auth/me", &old_tok).expect("dead");
    assert_status(&dead, 401, "old token must be dead after deactivation");

    // Reactivate.
    let react = patch_json_auth(
        &format!("/api/admin/users/{user_id}/status"),
        &admin,
        &json!({"is_active": true}),
    )
    .expect("r");
    assert_status(&react, 204, "reactivate");

    // The old token must NOT come back to life — reactivation doesn't
    // re-issue sessions. User logs in fresh.
    let still_dead = get_auth("/api/auth/me", &old_tok).expect("still-dead");
    assert_status(
        &still_dead,
        401,
        "reactivation must not resurrect old sessions",
    );

    // A fresh login yields a new working token.
    let Some(new_tok) = login(&u, &p) else {
        panic!("re-login after reactivation must succeed");
    };
    let fresh = get_auth("/api/auth/me", &new_tok).expect("fresh");
    assert_status(&fresh, 200, "fresh login after reactivate works");
}

#[test]
fn admin_cannot_deactivate_self() {
    let Some(admin) = setup("admin_cannot_deactivate_self") else {
        return;
    };
    // First establish our own user id via /auth/me.
    let me = get_auth("/api/auth/me", &admin).expect("me");
    let v = json_body(me, "me");
    let my_id = v["id"].as_str().expect("id").to_string();

    let resp = patch_json_auth(
        &format!("/api/admin/users/{my_id}/status"),
        &admin,
        &json!({"is_active": false}),
    )
    .expect("self");
    assert_status(
        &resp,
        409,
        "admin must not be able to deactivate themselves",
    );

    // And admin token still works.
    let check = get_auth("/api/auth/me", &admin).expect("c");
    assert_status(
        &check,
        200,
        "admin token still works after self-deactivate guard",
    );
}
