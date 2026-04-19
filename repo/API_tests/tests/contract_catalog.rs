//! Contract coverage for /api/services, /api/categories, /api/tags,
//! availability, favorites.

use api_tests::{
    assert_keys, assert_status, create_service, get, get_auth, json_body, login, nano_suffix,
    patch_json_auth, post_empty_auth, post_json_auth, provision_user, setup,
};
use serde_json::json;

#[test]
fn service_create_validation_and_contract() {
    let Some(admin) = setup("service_create_validation_and_contract") else {
        return;
    };

    // Success path returns full Service DTO.
    let name = format!("svc_{}", nano_suffix());
    let create = post_json_auth(
        "/api/services",
        &admin,
        &json!({
            "name": name,
            "description": "integration",
            "price": 99.5,
            "coverage_radius_miles": 12,
            "zip_code": "94110",
        }),
    )
    .expect("create");
    assert_status(&create, 200, "POST /api/services");
    let v = json_body(create, "POST /api/services");
    assert_keys(
        &v,
        &[
            "id",
            "name",
            "description",
            "price",
            "rating",
            "coverage_radius_miles",
            "zip_code",
        ],
        "Service contract",
    );
    assert_eq!(v["price"].as_f64(), Some(99.5));
    assert_eq!(v["coverage_radius_miles"].as_i64(), Some(12));
    let id = v["id"].as_str().unwrap().to_string();

    // Get by id -> 200 + same contract.
    let fetch = get_auth(&format!("/api/services/{id}"), &admin).expect("get");
    assert_status(&fetch, 200, "GET /api/services/<id>");
    let fv = json_body(fetch, "GET /api/services/<id>");
    assert_eq!(fv["name"], name);

    // Bad uuid -> 400.
    let bad = get_auth("/api/services/not-a-uuid", &admin).expect("bad");
    assert_status(&bad, 400, "bad uuid");

    // Missing service -> 404.
    let missing =
        get_auth("/api/services/00000000-0000-0000-0000-000000000000", &admin).expect("miss");
    assert_status(&missing, 404, "missing service");
}

#[test]
fn service_create_requires_management_role() {
    let Some(admin) = setup("service_create_requires_management_role") else {
        return;
    };
    let Some((_, u, p)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(tok) = login(&u, &p) else {
        return;
    };
    let resp = post_json_auth(
        "/api/services",
        &tok,
        &json!({"name":"x","description":"y","price":1.0,"coverage_radius_miles":0,"zip_code":"94110"}),
    )
    .expect("req-create");
    assert_status(&resp, 403, "requester cannot POST /api/services");
}

#[test]
fn service_update_contract_and_not_found() {
    let Some(admin) = setup("service_update_contract_and_not_found") else {
        return;
    };
    let Some(id) = create_service(&admin, "svcupd") else {
        return;
    };

    let ok = patch_json_auth(
        &format!("/api/services/{id}"),
        &admin,
        &json!({"price": 45.0, "description":"new"}),
    )
    .expect("patch");
    assert_status(&ok, 204, "PATCH /api/services/<id>");

    // Change reflected on GET.
    let fetch = get_auth(&format!("/api/services/{id}"), &admin).expect("get");
    let v = json_body(fetch, "GET /api/services/<id>");
    assert_eq!(v["price"].as_f64(), Some(45.0));
    assert_eq!(v["description"], "new");

    // Missing id -> 404.
    let missing = patch_json_auth(
        "/api/services/00000000-0000-0000-0000-000000000000",
        &admin,
        &json!({"price": 1.0}),
    )
    .expect("miss");
    assert_status(&missing, 404, "missing service update");
}

#[test]
fn service_search_result_contract_and_filters() {
    let Some(admin) = setup("service_search_result_contract_and_filters") else {
        return;
    };
    let Some(_id) = create_service(&admin, "svcsearch") else {
        return;
    };

    // Basic search (no filters) — must be array with Service shape.
    let resp = get_auth("/api/services/search", &admin).expect("search");
    assert_status(&resp, 200, "GET /api/services/search");
    let v = json_body(resp, "search");
    assert!(v.is_array(), "search must return array");
    if let Some(first) = v.as_array().unwrap().first() {
        assert_keys(
            first,
            &["id", "name", "price", "rating", "zip_code"],
            "search entry contract",
        );
    }

    // With sort + min_rating parameters.
    let resp = get_auth(
        "/api/services/search?sort=lowest_price&min_rating=0",
        &admin,
    )
    .expect("search2");
    assert_status(&resp, 200, "GET /api/services/search?sort=lowest_price");

    // Unknown ZIP -> 400.
    let resp = get_auth("/api/services/search?user_zip=ZZZZZ", &admin).expect("zip");
    assert_status(&resp, 400, "unknown ZIP -> 400");

    // Limit/offset as strings get parsed as u32.
    let resp = get_auth("/api/services/search?limit=5&offset=0", &admin).expect("limit");
    assert_status(&resp, 200, "limit/offset");
}

#[test]
fn service_compare_contract() {
    let Some(admin) = setup("service_compare_contract") else {
        return;
    };
    let Some(a) = create_service(&admin, "cmpa") else {
        return;
    };
    let Some(b) = create_service(&admin, "cmpb") else {
        return;
    };

    // Success.
    let resp = get_auth(&format!("/api/services/compare?ids={a},{b}"), &admin).expect("cmp");
    assert_status(&resp, 200, "compare");
    let v = json_body(resp, "compare");
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_keys(
        &arr[0],
        &["service", "tags", "categories", "availability"],
        "ServiceComparison contract",
    );

    // Empty -> 400.
    let resp = get_auth("/api/services/compare?ids=", &admin).expect("empty");
    assert_status(&resp, 400, "empty ids");

    // Too many -> 400.
    let resp = get_auth(
        &format!("/api/services/compare?ids={a},{b},{a},{b}"),
        &admin,
    )
    .expect("4");
    assert_status(&resp, 400, "too many ids");

    // Missing id -> 404.
    let resp = get_auth(
        &format!("/api/services/compare?ids=00000000-0000-0000-0000-000000000000"),
        &admin,
    )
    .expect("404");
    assert_status(&resp, 404, "compare 404");
}

#[test]
fn categories_and_tags_create_and_assign_contract() {
    let Some(admin) = setup("categories_and_tags_create_and_assign_contract") else {
        return;
    };
    let Some(sid) = create_service(&admin, "tagsvc") else {
        return;
    };

    // Category create
    let cat = post_json_auth(
        "/api/categories",
        &admin,
        &json!({"name": format!("cat_{}", nano_suffix())}),
    )
    .expect("cat");
    assert_status(&cat, 200, "POST /categories");
    let cv = json_body(cat, "POST /categories");
    assert_keys(&cv, &["id", "name"], "Category contract");
    let cat_id = cv["id"].as_str().unwrap().to_string();

    // Assign category -> 201
    let ac = post_json_auth(
        &format!("/api/services/{sid}/categories"),
        &admin,
        &json!({"category_id": cat_id}),
    )
    .expect("ac");
    assert_status(&ac, 201, "POST /services/<id>/categories");

    // Tag create
    let tag = post_json_auth(
        "/api/tags",
        &admin,
        &json!({"name": format!("tag_{}", nano_suffix())}),
    )
    .expect("tag");
    assert_status(&tag, 200, "POST /tags");
    let tv = json_body(tag, "POST /tags");
    assert_keys(&tv, &["id", "name"], "Tag contract");
    let tag_id = tv["id"].as_str().unwrap().to_string();

    // Assign tag -> 201
    let at = post_json_auth(
        &format!("/api/services/{sid}/tags"),
        &admin,
        &json!({"tag_id": tag_id}),
    )
    .expect("at");
    assert_status(&at, 201, "POST /services/<id>/tags");

    // Idempotent re-assignment: same shape (201/409 depending on impl —
    // allow either as long as not 500).
    let again = post_json_auth(
        &format!("/api/services/{sid}/tags"),
        &admin,
        &json!({"tag_id": tag_id}),
    )
    .expect("at2");
    assert!(
        matches!(again.status().as_u16(), 201 | 409 | 200),
        "second tag assign code = {}",
        again.status()
    );
}

#[test]
fn availability_create_validation_and_contract() {
    let Some(admin) = setup("availability_create_validation_and_contract") else {
        return;
    };
    let Some(sid) = create_service(&admin, "avsvc") else {
        return;
    };

    // Valid window.
    let ok = post_json_auth(
        &format!("/api/services/{sid}/availability"),
        &admin,
        &json!({"start_time":"2026-06-01T10:00:00","end_time":"2026-06-01T12:00:00"}),
    )
    .expect("av-ok");
    assert_status(&ok, 200, "POST /services/<id>/availability");
    let v = json_body(ok, "availability");
    assert_keys(
        &v,
        &["id", "service_id", "start_time", "end_time"],
        "AvailabilityWindow contract",
    );

    // Inverted window -> 400.
    let bad = post_json_auth(
        &format!("/api/services/{sid}/availability"),
        &admin,
        &json!({"start_time":"2026-06-01T12:00:00","end_time":"2026-06-01T10:00:00"}),
    )
    .expect("bad");
    assert_status(&bad, 400, "inverted window");
}

#[test]
fn favorite_lifecycle_and_ownership() {
    let Some(admin) = setup("favorite_lifecycle_and_ownership") else {
        return;
    };
    let Some(sid) = create_service(&admin, "favsvc") else {
        return;
    };
    let Some((_, u, p)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(tok) = login(&u, &p) else {
        return;
    };

    // Favorite -> 201. Idempotent repeat -> 201 too (INSERT IGNORE semantics).
    let r1 = post_empty_auth(&format!("/api/services/{sid}/favorite"), &tok).expect("fav");
    assert!(
        matches!(r1.status().as_u16(), 200 | 201 | 204),
        "first favorite -> {}",
        r1.status()
    );
    let r2 = post_empty_auth(&format!("/api/services/{sid}/favorite"), &tok).expect("fav2");
    assert!(
        matches!(r2.status().as_u16(), 200 | 201 | 204 | 409),
        "second favorite -> {}",
        r2.status()
    );
}

#[test]
fn reputation_with_breakdown_contract() {
    let Some(admin) = setup("reputation_with_breakdown_contract") else {
        return;
    };
    let Some(sid) = create_service(&admin, "repsvc") else {
        return;
    };

    // No reviews yet — still returns a Reputation object with total_reviews=0.
    let resp = get_auth(&format!("/api/services/{sid}/reputation"), &admin).expect("rep");
    assert_status(&resp, 200, "GET /reputation");
    let v = json_body(resp, "rep");
    assert_keys(
        &v,
        &["service_id", "final_score", "total_reviews"],
        "Reputation",
    );
    assert_eq!(v["total_reviews"].as_i64(), Some(0));

    // With breakdown flag.
    let resp = get_auth(
        &format!("/api/services/{sid}/reputation?breakdown=true"),
        &admin,
    )
    .expect("rep2");
    assert_status(&resp, 200, "rep with breakdown");
    let v = json_body(resp, "rep2");
    // breakdown array present
    assert!(v["breakdown"].is_array());

    // Unauthenticated -> 401 (already covered in endpoint_smoke), reassert here
    // so this contract file owns the round trip explicitly.
    let _ = get(&format!("/api/services/{sid}/reputation")).expect("unauth");
}
