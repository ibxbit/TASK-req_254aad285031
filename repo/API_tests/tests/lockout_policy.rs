//! Login lockout lifecycle tests.
//!
//! Proves that:
//! - Five consecutive wrong-password attempts lock the account.
//! - The sixth attempt (even correct password) returns 423 Locked.
//! - An admin password reset clears the lock.
//! - The user can log in with the new password after the reset.

use api_tests::{
    assert_status, json_body, nano_suffix, patch_json_auth, post_json, post_json_auth, setup,
};
use serde_json::json;

fn wrong_login(username: &str) -> reqwest::blocking::Response {
    post_json(
        "/api/auth/login",
        &json!({"username": username, "password": "definitelyWrongPassword99!"}),
    )
    .expect("login request")
}

#[test]
fn five_failures_lock_then_admin_reset_unlocks() {
    let Some(admin) = setup("five_failures_lock_then_admin_reset_unlocks") else {
        return;
    };

    // Provision a fresh user so the failure counter starts at 0.
    let username = format!("locktest_{}", nano_suffix());
    let correct_password = "verifypass123!";
    let create = post_json_auth(
        "/api/admin/users",
        &admin,
        &json!({"username": username, "password": correct_password, "role": "requester"}),
    )
    .expect("create");
    assert_status(&create, 200, "create locktest user");
    let user_id = json_body(create, "create")["id"]
        .as_str()
        .expect("id field")
        .to_string();

    // Five consecutive wrong-password attempts must each return 401.
    for i in 1..=5 {
        let r = wrong_login(&username);
        assert_status(
            &r,
            401,
            &format!("wrong-password attempt {i} must be 401 (not yet locked)"),
        );
    }

    // Sixth attempt — account is now locked — must return 423.
    let locked = wrong_login(&username);
    assert_status(&locked, 423, "sixth attempt must be 423 Locked");

    // Correct password also gets 423 while the lock is active.
    let also_locked = post_json(
        "/api/auth/login",
        &json!({"username": username, "password": correct_password}),
    )
    .expect("correct-while-locked");
    assert_status(
        &also_locked,
        423,
        "correct password during lockout must still be 423",
    );

    // Admin resets the password. The reset clears failed_login_count and
    // locked_until in the same UPDATE, so no separate unlock step is needed.
    let new_password = "resetPassword99!";
    let reset = patch_json_auth(
        &format!("/api/admin/users/{user_id}/password"),
        &admin,
        &json!({"password": new_password}),
    )
    .expect("reset");
    assert_status(&reset, 204, "admin password reset must be 204");

    // After reset, login with the new password must succeed.
    let after = post_json(
        "/api/auth/login",
        &json!({"username": username, "password": new_password}),
    )
    .expect("login after reset");
    assert_status(&after, 200, "login after reset must be 200");
    let body = json_body(after, "login-after-reset");
    assert!(
        body["token"].is_string(),
        "response must contain a token after unlock, got: {body}"
    );
}

#[test]
fn successful_login_before_lockout_resets_counter() {
    let Some(admin) = setup("successful_login_before_lockout_resets_counter") else {
        return;
    };

    let username = format!("lockrst_{}", nano_suffix());
    let password = "verifypass123!";
    let create = post_json_auth(
        "/api/admin/users",
        &admin,
        &json!({"username": username, "password": password, "role": "requester"}),
    )
    .expect("create");
    assert_status(&create, 200, "create user");

    // Four failed attempts (below threshold).
    for i in 1..=4 {
        let r = wrong_login(&username);
        assert_status(&r, 401, &format!("wrong attempt {i}"));
    }

    // One successful login resets the counter.
    let ok = post_json(
        "/api/auth/login",
        &json!({"username": username, "password": password}),
    )
    .expect("good login");
    assert_status(&ok, 200, "successful login resets counter");

    // Four more bad attempts starting from a clean slate → still 401, not 423.
    for i in 1..=4 {
        let r = wrong_login(&username);
        assert_status(
            &r,
            401,
            &format!("post-reset wrong attempt {i} must be 401 not 423"),
        );
    }

    // Correct password still works — not locked.
    let still_ok = post_json(
        "/api/auth/login",
        &json!({"username": username, "password": password}),
    )
    .expect("still good");
    assert_status(
        &still_ok,
        200,
        "user must not be locked after counter reset",
    );
}
