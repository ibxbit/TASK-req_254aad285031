//! GET /api/categories and GET /api/tags browsing + search-filter wiring.
//!
//! Proves the missing requester-side read endpoints exist and that
//! search filters (categories, tags, availability window, pagination)
//! are accepted and applied.

use api_tests::{
    assert_keys, assert_status, create_service, get_auth, json_body, nano_suffix, post_json_auth,
    setup,
};
use serde_json::json;

#[test]
fn get_categories_returns_hierarchical_list() {
    let Some(admin) = setup("get_categories_returns_hierarchical_list") else {
        return;
    };

    // Root.
    let root = post_json_auth(
        "/api/categories",
        &admin,
        &json!({"name": format!("root_{}", nano_suffix())}),
    )
    .expect("root");
    assert_status(&root, 200, "POST /categories root");
    let root_v = json_body(root, "root");
    assert!(root_v["parent_id"].is_null());
    let root_id = root_v["id"].as_str().unwrap().to_string();

    // Child.
    let child = post_json_auth(
        "/api/categories",
        &admin,
        &json!({"name": format!("child_{}", nano_suffix()), "parent_id": root_id}),
    )
    .expect("child");
    assert_status(&child, 200, "POST /categories child");
    let child_v = json_body(child, "child");
    assert_eq!(child_v["parent_id"], root_id);
    let child_id = child_v["id"].as_str().unwrap().to_string();

    // List — contract: Vec<Category { id, parent_id, name }>.
    let list = get_auth("/api/categories", &admin).expect("list");
    assert_status(&list, 200, "GET /api/categories");
    let v = json_body(list, "list");
    assert!(v.is_array(), "categories response must be array");
    let arr = v.as_array().unwrap();
    let found_root = arr
        .iter()
        .find(|c| c["id"].as_str() == Some(&root_id))
        .expect("root must be in list");
    assert_keys(found_root, &["id", "parent_id", "name"], "Category");
    let found_child = arr
        .iter()
        .find(|c| c["id"].as_str() == Some(&child_id))
        .expect("child must be in list");
    assert_eq!(
        found_child["parent_id"], root_id,
        "child category must link to root by parent_id"
    );
}

#[test]
fn get_tags_returns_browse_list() {
    let Some(admin) = setup("get_tags_returns_browse_list") else {
        return;
    };
    let name = format!("tag_{}", nano_suffix());
    let create = post_json_auth("/api/tags", &admin, &json!({"name": name.clone()})).expect("c");
    assert_status(&create, 200, "POST /tags");

    let list = get_auth("/api/tags", &admin).expect("list");
    assert_status(&list, 200, "GET /api/tags");
    let v = json_body(list, "list");
    assert!(v.is_array());
    let found = v
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"].as_str() == Some(&name))
        .expect("tag must be in list");
    assert_keys(found, &["id", "name"], "Tag");
}

#[test]
fn categories_and_tags_browse_requires_auth() {
    // Unauthenticated GET on the browse endpoints must still be 401 —
    // the catalog stays inside the authenticated app surface.
    if !api_tests::skip_if_offline("categories_and_tags_browse_requires_auth") {
        return;
    }
    let resp = api_tests::get("/api/categories").expect("c");
    assert_status(&resp, 401, "GET /categories unauthenticated");
    let resp = api_tests::get("/api/tags").expect("t");
    assert_status(&resp, 401, "GET /tags unauthenticated");
}

#[test]
fn search_accepts_availability_category_and_tag_filters() {
    let Some(admin) = setup("search_accepts_availability_category_and_tag_filters") else {
        return;
    };

    // Seed a service + category + tag + availability window so the
    // filters actually have something to match against.
    let Some(sid) = create_service(&admin, "avcatfilter") else {
        return;
    };
    let cat = post_json_auth(
        "/api/categories",
        &admin,
        &json!({"name": format!("c_{}", nano_suffix())}),
    )
    .expect("cat");
    let cat_id = json_body(cat, "cat")["id"].as_str().unwrap().to_string();
    let tag = post_json_auth(
        "/api/tags",
        &admin,
        &json!({"name": format!("t_{}", nano_suffix())}),
    )
    .expect("tag");
    let tag_id = json_body(tag, "tag")["id"].as_str().unwrap().to_string();

    // Attach them to the service.
    assert_status(
        &post_json_auth(
            &format!("/api/services/{sid}/categories"),
            &admin,
            &json!({"category_id": cat_id}),
        )
        .expect("sc"),
        201,
        "assign category",
    );
    assert_status(
        &post_json_auth(
            &format!("/api/services/{sid}/tags"),
            &admin,
            &json!({"tag_id": tag_id}),
        )
        .expect("st"),
        201,
        "assign tag",
    );
    let _ = post_json_auth(
        &format!("/api/services/{sid}/availability"),
        &admin,
        &json!({
            "start_time": "2026-07-01T10:00:00",
            "end_time":   "2026-07-01T12:00:00",
        }),
    )
    .expect("av");

    // Search with every new filter. Service must appear.
    let path = format!(
        "/api/services/search?categories={cat_id}&tags={tag_id}\
         &available_from=2026-07-01T09:00:00&available_to=2026-07-01T13:00:00"
    );
    let resp = get_auth(&path, &admin).expect("search");
    assert_status(&resp, 200, "search with all filters");
    let v = json_body(resp, "search");
    assert!(v.is_array(), "search must return array");
    let hit = v
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["id"].as_str() == Some(&sid));
    assert!(hit, "seeded service must match its own filters, got: {v}");

    // Bogus category id -> service must NOT be returned (filter is
    // restrictive, not additive).
    let path_bogus =
        format!("/api/services/search?categories=00000000-0000-0000-0000-000000000000");
    let resp = get_auth(&path_bogus, &admin).expect("bogus");
    assert_status(&resp, 200, "search with bogus cat");
    let v = json_body(resp, "bogus");
    let leaked = v
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["id"].as_str() == Some(&sid));
    assert!(
        !leaked,
        "service must not appear when filtered by an unrelated category"
    );

    // Bad availability format -> 400.
    let resp =
        get_auth("/api/services/search?available_from=not-a-date", &admin).expect("bad-date");
    assert_status(&resp, 400, "bad datetime -> 400");

    // Pagination smoke.
    let resp = get_auth("/api/services/search?limit=5&offset=0", &admin).expect("p");
    assert_status(&resp, 200, "pagination");
    assert!(json_body(resp, "p").is_array());
}
