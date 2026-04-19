//! Frontend ↔ Backend path confidence layer.
//!
//! Every URL the browser frontend generates comes from two pure-Rust modules
//! in `frontend_core`:
//!
//!   - `frontend_core::api_paths` — id-parameterised path builders
//!   - `frontend_core::search::build_search_path` — search query-string builder
//!
//! These tests exercise those exact functions against the live backend, proving
//! that the frontend's URL output is accepted by the backend router without
//! any intermediate transformation. A 404 from these tests would indicate a
//! mismatch between what the frontend generates and what the backend mounts.
//!
//! The "full user journey" test additionally validates the auth→browse→act
//! flow end-to-end using frontend-generated paths throughout.
//!
//! Run:  API_TESTS_STRICT=1 cargo test -p api_tests --test fe_be_paths

use api_tests::{assert_status, create_service, get_auth, json_body, setup};
use frontend_core::{
    api_paths,
    search::{build_search_path, SearchParams},
};

// ============================================================
// Search path builder validates against live backend
// ============================================================

#[test]
fn frontend_empty_search_path_returns_200_array() {
    let Some(admin) = setup("fe_empty_search_path") else {
        return;
    };
    let path = build_search_path(&SearchParams::default());
    assert_eq!(
        path, "/api/services/search",
        "empty params must be bare path"
    );
    let resp = get_auth(&path, &admin).expect("search");
    assert_status(&resp, 200, "frontend empty search path");
    assert!(
        json_body(resp, "empty search").is_array(),
        "must return array"
    );
}

#[test]
fn frontend_search_path_with_sort_and_limit_returns_200_array() {
    let Some(admin) = setup("fe_search_sort_limit") else {
        return;
    };
    let path = build_search_path(&SearchParams {
        sort: Some("best_rated".into()),
        limit: Some(5),
        offset: Some(0),
        ..Default::default()
    });
    assert!(
        path.contains("sort=best_rated"),
        "sort must be in path: {path}"
    );
    assert!(path.contains("limit=5"), "limit must be in path: {path}");
    let resp = get_auth(&path, &admin).expect("search");
    assert_status(&resp, 200, "sorted/paginated search path");
    assert!(json_body(resp, "sorted search").is_array());
}

#[test]
fn frontend_search_path_with_price_filter_resolves_correctly() {
    let Some(admin) = setup("fe_search_price_filter") else {
        return;
    };
    let path = build_search_path(&SearchParams {
        min_price: Some(0.0),
        max_price: Some(9999.0),
        ..Default::default()
    });
    assert!(path.contains("min_price="), "min_price absent: {path}");
    assert!(path.contains("max_price="), "max_price absent: {path}");
    let resp = get_auth(&path, &admin).expect("search");
    assert_status(&resp, 200, "price-filtered search path");
}

#[test]
fn frontend_search_path_with_text_query_resolves_correctly() {
    let Some(admin) = setup("fe_search_text_query") else {
        return;
    };
    let path = build_search_path(&SearchParams {
        q: Some("test service".into()),
        ..Default::default()
    });
    assert!(
        path.contains("q=test%20service"),
        "encoded query absent: {path}"
    );
    let resp = get_auth(&path, &admin).expect("search");
    assert_status(&resp, 200, "text-query search path");
}

// ============================================================
// api_paths builders validated against live backend
// ============================================================

#[test]
fn frontend_api_path_service_by_id_resolves_to_backend_route() {
    let Some(admin) = setup("fe_service_by_id_path") else {
        return;
    };
    let Some(svc_id) = create_service(&admin, "fe-path-check") else {
        eprintln!("SKIP fe_service_by_id_path: could not create service");
        return;
    };
    let path = api_paths::service_by_id(&svc_id);
    assert_eq!(path, format!("/api/services/{svc_id}"));
    let resp = get_auth(&path, &admin).expect("service_by_id");
    assert_status(&resp, 200, "frontend service_by_id path");
    let v = json_body(resp, "service detail");
    assert_eq!(v["id"].as_str(), Some(svc_id.as_str()), "id must match");
}

#[test]
fn frontend_api_path_service_reviews_resolves_to_backend_route() {
    let Some(admin) = setup("fe_service_reviews_path") else {
        return;
    };
    let Some(svc_id) = create_service(&admin, "fe-reviews-path") else {
        eprintln!("SKIP fe_service_reviews_path: could not create service");
        return;
    };
    let path = api_paths::service_reviews(&svc_id);
    assert_eq!(path, format!("/api/services/{svc_id}/reviews"));
    let resp = get_auth(&path, &admin).expect("service_reviews");
    assert_status(&resp, 200, "frontend service_reviews path");
    assert!(json_body(resp, "reviews list").is_array());
}

#[test]
fn frontend_api_path_service_reputation_resolves_to_backend_route() {
    let Some(admin) = setup("fe_service_reputation_path") else {
        return;
    };
    let Some(svc_id) = create_service(&admin, "fe-rep-path") else {
        eprintln!("SKIP fe_service_reputation_path: could not create service");
        return;
    };
    let path = api_paths::service_reputation(&svc_id);
    assert_eq!(path, format!("/api/services/{svc_id}/reputation"));
    let resp = get_auth(&path, &admin).expect("reputation");
    assert_status(&resp, 200, "frontend service_reputation path");
}

#[test]
fn frontend_api_path_warehouse_history_resolves_to_backend_route() {
    let Some(admin) = setup("fe_warehouse_history_path") else {
        return;
    };
    // Create a warehouse so we have a valid ID to probe.
    let body = serde_json::json!({ "name": format!("fe-hist-{}", api_tests::nano_suffix()) });
    let create = api_tests::post_json_auth("/api/warehouses", &admin, &body).expect("create wh");
    if create.status() != 200 {
        eprintln!("SKIP fe_warehouse_history_path: could not create warehouse");
        return;
    }
    let wh_v: serde_json::Value = create.json().unwrap();
    let wh_id = wh_v["id"].as_str().unwrap().to_string();

    let path = api_paths::warehouse_history(&wh_id);
    assert_eq!(path, format!("/api/warehouses/{wh_id}/history"));
    let resp = get_auth(&path, &admin).expect("warehouse history");
    assert_status(&resp, 200, "frontend warehouse_history path");
    assert!(json_body(resp, "warehouse history").is_array());
}

#[test]
fn frontend_api_paths_board_subresources_resolve() {
    let Some(admin) = setup("fe_board_subresource_paths") else {
        return;
    };

    // Need a zone and board.
    let zone_body = serde_json::json!({ "name": format!("fe-zone-{}", api_tests::nano_suffix()) });
    let zone_r = api_tests::post_json_auth("/api/zones", &admin, &zone_body).expect("zone");
    if zone_r.status() != 200 {
        eprintln!("SKIP fe_board_subresource_paths: could not create zone");
        return;
    }
    let zone_v: serde_json::Value = zone_r.json().unwrap();
    let zone_id = zone_v["id"].as_str().unwrap().to_string();

    let board_body = serde_json::json!({
        "zone_id": zone_id,
        "name": format!("fe-board-{}", api_tests::nano_suffix()),
        "visibility_type": "public"
    });
    let board_r = api_tests::post_json_auth("/api/boards", &admin, &board_body).expect("board");
    if board_r.status() != 200 {
        eprintln!("SKIP fe_board_subresource_paths: could not create board");
        return;
    }
    let board_v: serde_json::Value = board_r.json().unwrap();
    let board_id = board_v["id"].as_str().unwrap().to_string();

    // Validate all three board subresource paths generated by frontend_core.
    for (label, path) in [
        ("posts", api_paths::board_posts(&board_id)),
        ("rules", api_paths::board_rules(&board_id)),
        ("moderators", api_paths::board_moderators(&board_id)),
    ] {
        let resp = get_auth(&path, &admin).expect(label);
        assert_status(&resp, 200, &format!("frontend {label} path"));
    }
}

// ============================================================
// Full user journey: auth → browse → act
// ============================================================

#[test]
fn full_user_journey_auth_browse_protected_action() {
    let Some(admin) = setup("full_user_journey") else {
        return;
    };

    // 1. Auth: /me must return shape { id, username, role } with role = administrator
    let me = get_auth("/api/auth/me", &admin).expect("me");
    assert_status(&me, 200, "GET /auth/me");
    let me_v = json_body(me, "me");
    assert!(me_v["id"].is_string(), "me.id must be string: {me_v}");
    assert!(
        me_v["username"].is_string(),
        "me.username must be string: {me_v}"
    );
    assert_eq!(
        me_v["role"].as_str(),
        Some("administrator"),
        "bootstrap user must be administrator"
    );

    // 2. Browse: use frontend search path builder to list services
    let search_path = build_search_path(&SearchParams {
        sort: Some("best_rated".into()),
        limit: Some(10),
        offset: Some(0),
        ..Default::default()
    });
    let search = get_auth(&search_path, &admin).expect("search");
    assert_status(&search, 200, "frontend search path");
    let services = json_body(search, "services list");
    assert!(
        services.is_array(),
        "services search must return array: {services}"
    );

    // 3. Act: create a service, then verify it's retrievable via its id-based path
    let Some(svc_id) = create_service(&admin, "journey-svc") else {
        eprintln!("SKIP full_user_journey act phase: create_service failed");
        return;
    };

    // Use frontend_core path builder for the detail fetch
    let detail_path = api_paths::service_by_id(&svc_id);
    let detail = get_auth(&detail_path, &admin).expect("detail");
    assert_status(&detail, 200, "service detail via frontend path");
    let svc_v = json_body(detail, "service detail");
    assert_eq!(
        svc_v["id"].as_str(),
        Some(svc_id.as_str()),
        "retrieved service id must match created id"
    );

    // Use frontend_core path builder for the reviews list
    let reviews_path = api_paths::service_reviews(&svc_id);
    let reviews = get_auth(&reviews_path, &admin).expect("reviews");
    assert_status(&reviews, 200, "reviews via frontend path");
    assert!(json_body(reviews, "reviews").is_array());

    // Use frontend_core path builder for reputation
    let rep_path = api_paths::service_reputation(&svc_id);
    let rep = get_auth(&rep_path, &admin).expect("reputation");
    assert_status(&rep, 200, "reputation via frontend path");
}
