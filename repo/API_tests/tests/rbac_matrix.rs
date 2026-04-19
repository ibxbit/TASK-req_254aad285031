//! RBAC permission matrix: for a representative set of protected endpoints,
//! iterate every non-admin role and assert the expected 403. This is the
//! systematic version of `rbac_roles.rs` — one place to verify an endpoint
//! stays gated at the role level, across all the roles that should not
//! reach it.
//!
//! Roles exercised: moderator, service_manager, warehouse_manager, mentor,
//! intern, requester. Administrator is always allowed.

use api_tests::{
    assert_status, get_auth, login, patch_json_auth, post_empty_auth, post_json_auth,
    provision_user, setup,
};
use serde_json::json;

const NON_ADMIN_ROLES: &[&str] = &[
    "moderator",
    "service_manager",
    "warehouse_manager",
    "mentor",
    "intern",
    "requester",
];

fn fresh_token(admin: &str, role: &str) -> Option<String> {
    let (_, u, p) = provision_user(admin, role)?;
    login(&u, &p)
}

// Admin-only endpoints: every non-admin role must get 403.
#[test]
fn admin_only_endpoints_reject_all_non_admin_roles() {
    let Some(admin) = setup("admin_only_endpoints_reject_all_non_admin_roles") else {
        return;
    };
    for role in NON_ADMIN_ROLES {
        let Some(tok) = fresh_token(&admin, role) else {
            continue;
        };

        // Admin user list.
        assert_status(
            &get_auth("/api/admin/users", &tok).expect("au"),
            403,
            &format!("GET /admin/users as {role}"),
        );
        // Admin teams list.
        assert_status(
            &get_auth("/api/admin/teams", &tok).expect("at"),
            403,
            &format!("GET /admin/teams as {role}"),
        );
        // Create zone.
        assert_status(
            &post_json_auth("/api/zones", &tok, &json!({"name":"z"})).expect("z"),
            403,
            &format!("POST /zones as {role}"),
        );
        // Create board.
        assert_status(
            &post_json_auth(
                "/api/boards",
                &tok,
                &json!({"zone_id":"00000000-0000-0000-0000-000000000000","name":"b","visibility_type":"public"}),
            )
            .expect("b"),
            403,
            &format!("POST /boards as {role}"),
        );
        // Audit verify.
        assert_status(
            &get_auth("/api/audit/verify", &tok).expect("av"),
            403,
            &format!("GET /audit/verify as {role}"),
        );
        // Pin review.
        assert_status(
            &patch_json_auth(
                "/api/reviews/00000000-0000-0000-0000-000000000000/pin",
                &tok,
                &json!({"is_pinned": true}),
            )
            .expect("pin"),
            403,
            &format!("PATCH /reviews/<id>/pin as {role}"),
        );
        // Create review tag.
        assert_status(
            &post_json_auth("/api/review-tags", &tok, &json!({"name":"t"})).expect("t"),
            403,
            &format!("POST /review-tags as {role}"),
        );
    }
}

// Management-only endpoints (administrator + warehouse_manager).
#[test]
fn warehouse_management_endpoints_reject_non_management_roles() {
    let Some(admin) = setup("warehouse_management_endpoints_reject_non_management_roles") else {
        return;
    };
    for role in &[
        "moderator",
        "service_manager",
        "mentor",
        "intern",
        "requester",
    ] {
        let Some(tok) = fresh_token(&admin, role) else {
            continue;
        };
        assert_status(
            &post_json_auth("/api/warehouses", &tok, &json!({"name":"x"})).expect("w"),
            403,
            &format!("POST /warehouses as {role}"),
        );
        assert_status(
            &post_json_auth(
                "/api/warehouse-zones",
                &tok,
                &json!({"warehouse_id":"00000000-0000-0000-0000-000000000000","name":"z"}),
            )
            .expect("wz"),
            403,
            &format!("POST /warehouse-zones as {role}"),
        );
        assert_status(
            &post_json_auth(
                "/api/bins",
                &tok,
                &json!({
                    "zone_id":"00000000-0000-0000-0000-000000000000",
                    "name":"b","width_in":1.0,"height_in":1.0,"depth_in":1.0,
                    "max_load_lbs":0.0,"temp_zone":"ambient"
                }),
            )
            .expect("b"),
            403,
            &format!("POST /bins as {role}"),
        );
        // History endpoints are also management-only.
        assert_status(
            &get_auth(
                "/api/warehouses/00000000-0000-0000-0000-000000000000/history",
                &tok,
            )
            .expect("wh"),
            403,
            &format!("GET /warehouses/<id>/history as {role}"),
        );
        assert_status(
            &get_auth(
                "/api/bins/00000000-0000-0000-0000-000000000000/history",
                &tok,
            )
            .expect("bh"),
            403,
            &format!("GET /bins/<id>/history as {role}"),
        );
    }
}

// Intern-only endpoints.
#[test]
fn intern_only_endpoints_reject_non_intern_roles() {
    let Some(admin) = setup("intern_only_endpoints_reject_non_intern_roles") else {
        return;
    };
    for role in &[
        "moderator",
        "service_manager",
        "warehouse_manager",
        "mentor",
        "requester",
    ] {
        let Some(tok) = fresh_token(&admin, role) else {
            continue;
        };
        assert_status(
            &post_json_auth("/api/internships/plans", &tok, &json!({"content":"x"})).expect("p"),
            403,
            &format!("POST /internships/plans as {role}"),
        );
        assert_status(
            &post_json_auth("/api/reports", &tok, &json!({"type":"DAILY","content":"x"}))
                .expect("r"),
            403,
            &format!("POST /reports as {role}"),
        );
    }
}

// Mentor/Administrator-only endpoints.
#[test]
fn mentor_endpoints_reject_non_mentor_roles() {
    let Some(admin) = setup("mentor_endpoints_reject_non_mentor_roles") else {
        return;
    };
    for role in &[
        "moderator",
        "service_manager",
        "warehouse_manager",
        "intern",
        "requester",
    ] {
        let Some(tok) = fresh_token(&admin, role) else {
            continue;
        };
        assert_status(
            &post_json_auth(
                "/api/reports/00000000-0000-0000-0000-000000000000/comments",
                &tok,
                &json!({"content":"c"}),
            )
            .expect("mc"),
            403,
            &format!("POST /reports/<id>/comments as {role}"),
        );
        assert_status(
            &post_empty_auth(
                "/api/reports/00000000-0000-0000-0000-000000000000/approve",
                &tok,
            )
            .expect("ma"),
            403,
            &format!("POST /reports/<id>/approve as {role}"),
        );
    }
}

// ServiceManager/Administrator-only endpoints.
#[test]
fn service_mgmt_endpoints_reject_non_service_manager_roles() {
    let Some(admin) = setup("service_mgmt_endpoints_reject_non_service_manager_roles") else {
        return;
    };
    // Service creation is allowed for service_manager + administrator.
    for role in &[
        "moderator",
        "warehouse_manager",
        "mentor",
        "intern",
        "requester",
    ] {
        let Some(tok) = fresh_token(&admin, role) else {
            continue;
        };
        assert_status(
            &post_json_auth(
                "/api/services",
                &tok,
                &json!({"name":"x","description":"y","price":1.0,"coverage_radius_miles":0,"zip_code":"00000"}),
            )
            .expect("svc"),
            403,
            &format!("POST /services as {role}"),
        );
    }
    // And service_manager is allowed.
    let Some(tok) = fresh_token(&admin, "service_manager") else {
        return;
    };
    let resp = post_json_auth(
        "/api/services",
        &tok,
        &json!({"name":"sm_svc","description":"y","price":1.0,"coverage_radius_miles":0,"zip_code":"00000"}),
    )
    .expect("sm");
    assert_status(&resp, 200, "service_manager can create services");
}
