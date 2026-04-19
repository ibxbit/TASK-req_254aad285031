//! Catalog search filter edge coverage.
//!
//! Covers:
//! - Single-sided availability filter (only `available_from` or only
//!   `available_to`) — filter requires both sides; one-sided is silently
//!   ignored and the endpoint still returns 200.
//! - Multi-category AND semantics: service must carry ALL listed categories.
//! - Multi-tag OR semantics: service matching ANY listed tag is returned.
//! - Pagination boundary values: limit=1, offset past end of result set.

use api_tests::{
    assert_status, create_service, get_auth, json_body, nano_suffix, post_json_auth, setup,
};
use serde_json::json;

// ---- helpers ----

fn make_category(admin: &str, prefix: &str) -> String {
    let r = post_json_auth(
        "/api/categories",
        admin,
        &json!({"name": format!("{prefix}_{}", nano_suffix())}),
    )
    .expect("category");
    json_body(r, "make_category")["id"]
        .as_str()
        .expect("id")
        .to_string()
}

fn make_tag(admin: &str, prefix: &str) -> String {
    let r = post_json_auth(
        "/api/tags",
        admin,
        &json!({"name": format!("{prefix}_{}", nano_suffix())}),
    )
    .expect("tag");
    json_body(r, "make_tag")["id"]
        .as_str()
        .expect("id")
        .to_string()
}

fn attach_category(admin: &str, sid: &str, cat_id: &str) {
    post_json_auth(
        &format!("/api/services/{sid}/categories"),
        admin,
        &json!({"category_id": cat_id}),
    )
    .expect("attach_category");
}

fn attach_tag(admin: &str, sid: &str, tag_id: &str) {
    post_json_auth(
        &format!("/api/services/{sid}/tags"),
        admin,
        &json!({"tag_id": tag_id}),
    )
    .expect("attach_tag");
}

fn add_availability(admin: &str, sid: &str, from: &str, to: &str) {
    post_json_auth(
        &format!("/api/services/{sid}/availability"),
        admin,
        &json!({"start_time": from, "end_time": to}),
    )
    .expect("add_availability");
}

fn search(admin: &str, qs: &str) -> serde_json::Value {
    let resp = get_auth(&format!("/api/services/search?{qs}"), admin).expect("search");
    assert_status(&resp, 200, &format!("search?{qs}"));
    json_body(resp, qs)
}

fn ids_in(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|s| s["id"].as_str().map(str::to_string))
        .collect()
}

// ---- tests ----

#[test]
fn single_sided_available_from_only_is_silently_ignored() {
    // The search endpoint requires BOTH available_from AND available_to to apply
    // the filter (if only one side is provided the clause is skipped). The
    // response must still be 200 — not 400.
    let Some(admin) = setup("single_sided_available_from_only_is_silently_ignored") else {
        return;
    };

    let resp = get_auth(
        "/api/services/search?available_from=2026-07-01T09:00:00",
        &admin,
    )
    .expect("req");
    assert_status(&resp, 200, "single-sided available_from must be 200");
    assert!(
        json_body(resp, "single-from").is_array(),
        "response must be array"
    );
}

#[test]
fn single_sided_available_to_only_is_silently_ignored() {
    let Some(admin) = setup("single_sided_available_to_only_is_silently_ignored") else {
        return;
    };

    let resp = get_auth(
        "/api/services/search?available_to=2026-07-01T12:00:00",
        &admin,
    )
    .expect("req");
    assert_status(&resp, 200, "single-sided available_to must be 200");
    assert!(
        json_body(resp, "single-to").is_array(),
        "response must be array"
    );
}

#[test]
fn multi_category_and_semantics() {
    // Service A has cat1 + cat2.  Service B has only cat1.
    // Filter categories=cat1,cat2 → only A appears; B is excluded.
    let Some(admin) = setup("multi_category_and_semantics") else {
        return;
    };

    let cat1 = make_category(&admin, "mc1");
    let cat2 = make_category(&admin, "mc2");

    let Some(sid_a) = create_service(&admin, "multcat_a") else {
        return;
    };
    let Some(sid_b) = create_service(&admin, "multcat_b") else {
        return;
    };

    attach_category(&admin, &sid_a, &cat1);
    attach_category(&admin, &sid_a, &cat2);
    attach_category(&admin, &sid_b, &cat1);

    let v = search(&admin, &format!("categories={cat1},{cat2}"));
    let found_ids = ids_in(&v);

    assert!(
        found_ids.contains(&sid_a),
        "service with both categories must appear, got: {v}"
    );
    assert!(
        !found_ids.contains(&sid_b),
        "service with only one of the two categories must be excluded, got: {v}"
    );
}

#[test]
fn multi_tag_or_semantics() {
    // Service A has tagX.  Service B has tagY.  Service C has neither.
    // Filter tags=tagX,tagY → both A and B appear; C is excluded.
    let Some(admin) = setup("multi_tag_or_semantics") else {
        return;
    };

    let tag_x = make_tag(&admin, "tx");
    let tag_y = make_tag(&admin, "ty");

    let Some(sid_a) = create_service(&admin, "multag_a") else {
        return;
    };
    let Some(sid_b) = create_service(&admin, "multag_b") else {
        return;
    };
    let Some(sid_c) = create_service(&admin, "multag_c") else {
        return;
    };

    attach_tag(&admin, &sid_a, &tag_x);
    attach_tag(&admin, &sid_b, &tag_y);
    // sid_c gets no tags.

    let v = search(&admin, &format!("tags={tag_x},{tag_y}"));
    let found_ids = ids_in(&v);

    assert!(
        found_ids.contains(&sid_a),
        "service with tagX must appear in tagX,tagY filter, got: {v}"
    );
    assert!(
        found_ids.contains(&sid_b),
        "service with tagY must appear in tagX,tagY filter, got: {v}"
    );
    assert!(
        !found_ids.contains(&sid_c),
        "service with neither tag must be excluded, got: {v}"
    );
}

#[test]
fn availability_filter_both_sides_matches_overlapping_window() {
    // Create a service with a known window 10:00-12:00.
    // Search window 09:00-11:00 overlaps → service appears.
    // Search window 13:00-14:00 does not overlap → service absent.
    let Some(admin) = setup("availability_filter_both_sides_matches_overlapping_window") else {
        return;
    };

    let Some(sid) = create_service(&admin, "avail_edge") else {
        return;
    };
    add_availability(&admin, &sid, "2026-08-01T10:00:00", "2026-08-01T12:00:00");

    // Overlapping: search 09:00-11:00 (start < window_end AND end > window_start).
    let v_hit = search(
        &admin,
        "available_from=2026-08-01T09:00:00&available_to=2026-08-01T11:00:00",
    );
    assert!(
        ids_in(&v_hit).contains(&sid),
        "overlapping window must match, got: {v_hit}"
    );

    // Non-overlapping: search 13:00-14:00 (entirely after the window).
    let v_miss = search(
        &admin,
        "available_from=2026-08-01T13:00:00&available_to=2026-08-01T14:00:00",
    );
    assert!(
        !ids_in(&v_miss).contains(&sid),
        "non-overlapping window must not match, got: {v_miss}"
    );
}

#[test]
fn pagination_limit_one_returns_at_most_one_result() {
    let Some(admin) = setup("pagination_limit_one_returns_at_most_one_result") else {
        return;
    };

    // Seed two services so there is something to paginate over.
    create_service(&admin, "paglim");
    create_service(&admin, "paglim");

    let v = search(&admin, "limit=1&offset=0");
    let arr = v.as_array().expect("must be array");
    assert!(
        arr.len() <= 1,
        "limit=1 must return at most 1 result, got {}",
        arr.len()
    );
}

#[test]
fn pagination_offset_beyond_results_returns_empty_array() {
    let Some(admin) = setup("pagination_offset_beyond_results_returns_empty_array") else {
        return;
    };

    let v = search(&admin, "limit=50&offset=999999");
    let arr = v.as_array().expect("must be array");
    assert!(
        arr.is_empty(),
        "offset past last row must return empty array, got {} items",
        arr.len()
    );
}

#[test]
fn reversed_availability_range_is_permissive_not_error() {
    // available_from > available_to (inverted window). The backend has no
    // explicit guard on this — `parse_dt` only validates format, not ordering.
    // Contract: the endpoint returns 200 with an array (not 400). The
    // inverted filter is applied as-is, producing a stricter-than-usual
    // predicate (service window must start before `to` AND end after `from`,
    // which requires spanning the reversed interval). Asserting 200 + array
    // pins this permissive behavior so any future change to return 400
    // triggers a deliberate, visible test update.
    let Some(admin) = setup("reversed_availability_range_is_permissive_not_error") else {
        return;
    };

    // Seed a service with window 10:00-12:00. With reversed filter
    // from=14:00/to=13:00, the SQL becomes: start_time < 13:00 AND
    // end_time > 14:00. Our window ends at 12:00, so it does NOT match.
    let Some(sid) = create_service(&admin, "revavail") else {
        return;
    };
    add_availability(&admin, &sid, "2026-08-01T10:00:00", "2026-08-01T12:00:00");

    let resp = get_auth(
        "/api/services/search?available_from=2026-08-01T14:00:00&available_to=2026-08-01T13:00:00",
        &admin,
    )
    .expect("req");
    // Must be 200 — the implementation is intentionally permissive.
    assert_status(
        &resp,
        200,
        "reversed availability range must return 200 not 400",
    );
    let v = json_body(resp, "reversed-range");
    assert!(v.is_array(), "response must be a JSON array, got: {v}");

    // The seeded service's window (10:00-12:00) cannot satisfy
    // start_time < 13:00 AND end_time > 14:00 simultaneously, so it must
    // not appear in the results.
    assert!(
        !ids_in(&v).contains(&sid),
        "service with non-spanning window must not match reversed filter, got: {v}"
    );
}
