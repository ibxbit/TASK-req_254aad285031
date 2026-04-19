//! Role-based 403 coverage and object-ownership authorization.
//!
//! Complements `rbac.rs` (which only covers 401 for unauthenticated calls).
//! Provisions role-specific users via the admin API, then asserts that
//! endpoints correctly refuse cross-role access with 403.

use api_tests::{
    api_base, bootstrap_admin_token, client, get_auth, login, post_empty_auth, post_json_auth,
    provision_user, skip_if_offline,
};
use serde_json::json;

fn macro_skip(name: &str) -> Option<String> {
    if !skip_if_offline(name) {
        return None;
    }
    match bootstrap_admin_token() {
        Some(t) => Some(t),
        None => {
            eprintln!("SKIP {name}: no administrator token available");
            None
        }
    }
}

#[test]
fn non_admin_cannot_list_admin_users() {
    let Some(admin) = macro_skip("non_admin_cannot_list_admin_users") else {
        return;
    };
    let Some((_, uname, pwd)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(req_token) = login(&uname, &pwd) else {
        eprintln!("SKIP non_admin_cannot_list_admin_users: login failed");
        return;
    };
    let resp = get_auth("/api/admin/users", &req_token).expect("get");
    assert_eq!(
        resp.status(),
        403,
        "non-admin must see 403 on GET /api/admin/users, got {}",
        resp.status()
    );
}

#[test]
fn non_admin_cannot_create_review_tag() {
    let Some(admin) = macro_skip("non_admin_cannot_create_review_tag") else {
        return;
    };
    let Some((_, uname, pwd)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(req_token) = login(&uname, &pwd) else {
        eprintln!("SKIP non_admin_cannot_create_review_tag: login failed");
        return;
    };
    let resp = post_json_auth(
        "/api/review-tags",
        &req_token,
        &json!({ "name": format!("needs_admin_{}", api_tests::nano_suffix()) }),
    )
    .expect("post");
    assert_eq!(resp.status(), 403);
}

#[test]
fn non_manager_cannot_create_warehouse() {
    let Some(admin) = macro_skip("non_manager_cannot_create_warehouse") else {
        return;
    };
    let Some((_, uname, pwd)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(tok) = login(&uname, &pwd) else {
        eprintln!("SKIP non_manager_cannot_create_warehouse: login failed");
        return;
    };
    let resp = post_json_auth(
        "/api/warehouses",
        &tok,
        &json!({ "name": format!("wh_{}", api_tests::nano_suffix()) }),
    )
    .expect("post");
    assert_eq!(resp.status(), 403);
}

#[test]
fn non_intern_cannot_submit_report() {
    let Some(admin) = macro_skip("non_intern_cannot_submit_report") else {
        return;
    };
    let Some((_, uname, pwd)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(tok) = login(&uname, &pwd) else {
        eprintln!("SKIP non_intern_cannot_submit_report: login failed");
        return;
    };
    let resp = post_json_auth(
        "/api/reports",
        &tok,
        &json!({ "type": "DAILY", "content": "nope" }),
    )
    .expect("post");
    assert_eq!(resp.status(), 403);
}

// Ownership: only the requester who owns a work order can read it (admins /
// service managers may read all). A different requester must get 403.
#[test]
fn work_order_ownership_is_enforced() {
    let Some(admin) = macro_skip("work_order_ownership_is_enforced") else {
        return;
    };
    let Some((_, owner_user, owner_pwd)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some((_, other_user, other_pwd)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(owner_tok) = login(&owner_user, &owner_pwd) else {
        return;
    };
    let Some(other_tok) = login(&other_user, &other_pwd) else {
        return;
    };

    // Admin creates a service that the requester can refer to.
    let svc_resp = post_json_auth(
        "/api/services",
        &admin,
        &json!({
            "name": format!("svc_{}", api_tests::nano_suffix()),
            "description": "test",
            "price": 0.0,
            "coverage_radius_miles": 0,
            "zip_code": "00000",
        }),
    )
    .expect("svc");
    if !svc_resp.status().is_success() {
        eprintln!("SKIP work_order_ownership_is_enforced: could not create service");
        return;
    }
    let svc: serde_json::Value = svc_resp.json().expect("svc json");
    let svc_id = svc["id"].as_str().expect("svc id");

    // Owner creates a work order.
    let wo_resp = post_json_auth(
        "/api/work-orders",
        &owner_tok,
        &json!({ "service_id": svc_id }),
    )
    .expect("wo");
    assert!(
        wo_resp.status().is_success(),
        "create wo -> {}",
        wo_resp.status()
    );
    let wo: serde_json::Value = wo_resp.json().expect("wo json");
    let wo_id = wo["id"].as_str().expect("wo id");

    // Other requester tries to read — should be 403 (not 404: we don't
    // leak existence by returning 404 for ownership failures).
    let cross = get_auth(&format!("/api/work-orders/{wo_id}"), &other_tok).expect("cross get");
    assert_eq!(
        cross.status(),
        403,
        "cross-requester work order must be 403"
    );

    // Owner reads their own — must succeed.
    let own = get_auth(&format!("/api/work-orders/{wo_id}"), &owner_tok).expect("own get");
    assert!(own.status().is_success());
}

// Bins history is a management-only endpoint (parity with warehouse +
// warehouse-zones history). A plain requester must be 403.
#[test]
fn bins_history_requires_management_role() {
    let Some(admin) = macro_skip("bins_history_requires_management_role") else {
        return;
    };
    let Some((_, uname, pwd)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(tok) = login(&uname, &pwd) else {
        return;
    };
    // Even if the bin id doesn't exist, the role guard must fire first ->
    // 403 comes back before the lookup would return 404.
    let resp = get_auth(
        "/api/bins/00000000-0000-0000-0000-000000000000/history",
        &tok,
    )
    .expect("get");
    assert_eq!(
        resp.status(),
        403,
        "requester must not see bin change history, got {}",
        resp.status()
    );
}

#[test]
fn warehouses_history_requires_management_role() {
    let Some(admin) = macro_skip("warehouses_history_requires_management_role") else {
        return;
    };
    let Some((_, uname, pwd)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(tok) = login(&uname, &pwd) else {
        return;
    };
    let resp = get_auth(
        "/api/warehouses/00000000-0000-0000-0000-000000000000/history",
        &tok,
    )
    .expect("get");
    assert_eq!(resp.status(), 403);
}

#[test]
fn warehouse_zones_history_requires_management_role() {
    let Some(admin) = macro_skip("warehouse_zones_history_requires_management_role") else {
        return;
    };
    let Some((_, uname, pwd)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(tok) = login(&uname, &pwd) else {
        return;
    };
    let resp = get_auth(
        "/api/warehouse-zones/00000000-0000-0000-0000-000000000000/history",
        &tok,
    )
    .expect("get");
    assert_eq!(resp.status(), 403);
}

// Token attached but invalid — still 401, not 403 (protects against role
// escalation via guessed-token headers).
#[test]
fn invalid_bearer_is_401_not_403() {
    if !skip_if_offline("invalid_bearer_is_401_not_403") {
        return;
    }
    let c = client();
    let resp = c
        .get(format!("{}/api/admin/users", api_base()))
        .bearer_auth("not-a-real-token")
        .send()
        .expect("get");
    assert_eq!(resp.status(), 401);

    let empty = post_empty_auth("/api/auth/logout", "not-a-real-token").expect("post");
    assert_eq!(empty.status(), 401);
}
