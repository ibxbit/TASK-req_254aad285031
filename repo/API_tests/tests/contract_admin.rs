//! Contract coverage for /api/admin/*. For each endpoint:
//!   * method + path + payload,
//!   * status class (200/4xx as applicable),
//!   * response body field contract.

use api_tests::{
    assert_keys, assert_status, delete_auth, get_auth, json_body, login, nano_suffix, patch_json,
    patch_json_auth, post_empty_auth, post_json_auth, provision_user, setup,
};
use serde_json::json;

#[test]
fn admin_users_list_contract() {
    let Some(admin) = setup("admin_users_list_contract") else {
        return;
    };
    let resp = get_auth("/api/admin/users", &admin).expect("get");
    assert_status(&resp, 200, "GET /api/admin/users");
    let v = json_body(resp, "GET /api/admin/users");
    assert!(v.is_array(), "response must be an array");
    // At minimum the bootstrap admin should be in there.
    let arr = v.as_array().unwrap();
    assert!(!arr.is_empty(), "admin users list must not be empty");
    let first = &arr[0];
    assert_keys(
        first,
        &["id", "username", "role", "is_active"],
        "AdminUserView contract",
    );
}

#[test]
fn admin_users_create_then_mutate_contract() {
    let Some(admin) = setup("admin_users_create_then_mutate_contract") else {
        return;
    };
    let username = format!("apit_matrix_{}", nano_suffix());
    let create = post_json_auth(
        "/api/admin/users",
        &admin,
        &json!({"username": username, "password":"verifypass123!", "role":"intern"}),
    )
    .expect("create");
    assert_status(&create, 200, "POST /api/admin/users");
    let v = json_body(create, "POST /api/admin/users");
    assert_keys(
        &v,
        &["id", "username", "role", "is_active"],
        "create contract",
    );
    assert_eq!(v["username"], username);
    assert_eq!(v["role"], "intern");
    assert_eq!(v["is_active"], true);
    let id = v["id"].as_str().unwrap().to_string();

    // Duplicate username -> 409.
    let dup = post_json_auth(
        "/api/admin/users",
        &admin,
        &json!({"username": username, "password":"verifypass123!", "role":"intern"}),
    )
    .expect("dup");
    assert_status(&dup, 409, "duplicate username");

    // Role update -> 204.
    let role_resp = patch_json_auth(
        &format!("/api/admin/users/{id}/role"),
        &admin,
        &json!({"role": "requester"}),
    )
    .expect("role");
    assert_status(&role_resp, 204, "PATCH /admin/users/<id>/role");

    // Password reset -> 204, short password -> 400.
    let pw_bad = patch_json_auth(
        &format!("/api/admin/users/{id}/password"),
        &admin,
        &json!({"password": "x"}),
    )
    .expect("pw bad");
    assert_status(&pw_bad, 400, "short password");
    let pw_ok = patch_json_auth(
        &format!("/api/admin/users/{id}/password"),
        &admin,
        &json!({"password": "anotherlongpass456!"}),
    )
    .expect("pw ok");
    assert_status(&pw_ok, 204, "PATCH /admin/users/<id>/password");

    // Status deactivate -> 204.
    let status = patch_json_auth(
        &format!("/api/admin/users/{id}/status"),
        &admin,
        &json!({"is_active": false}),
    )
    .expect("status");
    assert_status(&status, 204, "PATCH /admin/users/<id>/status");

    // Sensitive put -> 204. Body never includes plaintext; server masks.
    let sensitive = api_tests::put_json_auth(
        &format!("/api/admin/users/{id}/sensitive"),
        &admin,
        &json!({"value": "123-45-6789"}),
    )
    .expect("sensitive");
    assert_status(&sensitive, 204, "PUT /admin/users/<id>/sensitive");

    // Unknown id -> 404 across the mutation endpoints.
    let missing = "00000000-0000-0000-0000-000000000000";
    let missing_role = patch_json_auth(
        &format!("/api/admin/users/{missing}/role"),
        &admin,
        &json!({"role":"intern"}),
    )
    .expect("missing role");
    assert_status(&missing_role, 404, "missing user role update");
}

#[test]
fn admin_users_update_rejects_when_not_admin() {
    let Some(admin) = setup("admin_users_update_rejects_when_not_admin") else {
        return;
    };
    let Some((_, u, p)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(req) = login(&u, &p) else {
        return;
    };
    let resp = patch_json_auth(
        "/api/admin/users/00000000-0000-0000-0000-000000000000/role",
        &req,
        &json!({"role": "intern"}),
    )
    .expect("req-patch");
    assert_status(&resp, 403, "non-admin PATCH /admin/users");
}

#[test]
fn admin_register_bootstrap_closed_when_users_exist() {
    let Some(_admin) = setup("admin_register_bootstrap_closed_when_users_exist") else {
        return;
    };
    // Admin exists now by contract -> register must 403.
    let resp = patch_json(
        "/api/auth/register",
        &json!({"username":"a","password":"b"}),
    );
    let _ = resp; // patch is just a way to reach the endpoint; actual test below.
    let post_resp = api_tests::post_json(
        "/api/auth/register",
        &json!({"username":"nope","password":"longenough12345"}),
    )
    .expect("register");
    assert_status(&post_resp, 403, "register closed after bootstrap");
}

#[test]
fn admin_teams_full_lifecycle_contract() {
    let Some(admin) = setup("admin_teams_full_lifecycle_contract") else {
        return;
    };
    let name = format!("team_{}", nano_suffix());

    // Create.
    let create =
        post_json_auth("/api/admin/teams", &admin, &json!({"name": name})).expect("create team");
    assert_status(&create, 200, "POST /admin/teams");
    let v = json_body(create, "POST /admin/teams");
    assert_keys(&v, &["id", "name"], "team contract");
    assert_eq!(v["name"], name);
    let team_id = v["id"].as_str().unwrap().to_string();

    // List — must include our team.
    let list = get_auth("/api/admin/teams", &admin).expect("list");
    assert_status(&list, 200, "GET /admin/teams");
    let list_json = json_body(list, "GET /admin/teams");
    let found = list_json
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["id"].as_str() == Some(&team_id));
    assert!(found, "created team not in list");

    // Duplicate name -> 409.
    let dup = post_json_auth("/api/admin/teams", &admin, &json!({"name": name})).expect("dup team");
    assert_status(&dup, 409, "dup team name");

    // Add member.
    let Some((user_id, _, _)) = provision_user(&admin, "requester") else {
        return;
    };
    let add = post_json_auth(
        &format!("/api/admin/teams/{team_id}/members"),
        &admin,
        &json!({"user_id": user_id}),
    )
    .expect("add");
    assert_status(&add, 201, "add member");

    // Members list.
    let members =
        get_auth(&format!("/api/admin/teams/{team_id}/members"), &admin).expect("members");
    assert_status(&members, 200, "list members");
    let members_v = json_body(members, "list members");
    let m_arr = members_v.as_array().unwrap();
    assert!(m_arr
        .iter()
        .any(|m| m["user_id"].as_str() == Some(&user_id)));
    // Member contract includes role + username.
    assert_keys(
        &m_arr[0],
        &["user_id", "username", "role"],
        "member contract",
    );

    // Remove member -> 204.
    let rm = delete_auth(
        &format!("/api/admin/teams/{team_id}/members/{user_id}"),
        &admin,
    )
    .expect("rm");
    assert_status(&rm, 204, "remove member");
    // Remove again -> 404.
    let rm_again = delete_auth(
        &format!("/api/admin/teams/{team_id}/members/{user_id}"),
        &admin,
    )
    .expect("rm again");
    assert_status(&rm_again, 404, "remove member twice");

    // Delete team -> 204, repeat -> 404.
    let del = delete_auth(&format!("/api/admin/teams/{team_id}"), &admin).expect("del");
    assert_status(&del, 204, "delete team");
    let del_again = delete_auth(&format!("/api/admin/teams/{team_id}"), &admin).expect("del again");
    assert_status(&del_again, 404, "delete absent team");
}

#[test]
fn admin_logout_ends_session() {
    let Some(admin) = setup("admin_logout_ends_session") else {
        return;
    };
    // Use this token for logout so we don't invalidate the shared admin token.
    let Some((_, u, p)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(tok) = login(&u, &p) else {
        return;
    };

    let me = get_auth("/api/auth/me", &tok).expect("me1");
    assert_status(&me, 200, "GET /auth/me before logout");
    assert_eq!(json_body(me, "me1")["username"].as_str(), Some(u.as_str()));

    let logout = post_empty_auth("/api/auth/logout", &tok).expect("logout");
    assert_status(&logout, 204, "POST /auth/logout");

    // After logout, token is 401.
    let me2 = get_auth("/api/auth/me", &tok).expect("me2");
    assert_status(&me2, 401, "GET /auth/me after logout");
}
