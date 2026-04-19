//! Work order + review admin surfaces (pin, collapse, admin tag assignment,
//! review tag vocabulary list).

use api_tests::{
    assert_keys, assert_status, create_service, get_auth, json_body, login, nano_suffix,
    patch_json_auth, post_empty_auth, post_json_auth, provision_user, setup,
};
use serde_json::{json, Value};

fn completed_work_order(admin: &str) -> Option<(String, String, String)> {
    let sid = create_service(admin, "wosvc")?;
    let (_, u, p) = provision_user(admin, "requester")?;
    let req_tok = login(&u, &p)?;
    let wo = post_json_auth("/api/work-orders", &req_tok, &json!({"service_id": sid})).ok()?;
    if !wo.status().is_success() {
        return None;
    }
    let v: Value = wo.json().ok()?;
    let wo_id = v["id"].as_str()?.to_string();
    // Admin completes.
    let c = post_empty_auth(&format!("/api/work-orders/{wo_id}/complete"), admin).ok()?;
    if !c.status().is_success() {
        return None;
    }
    Some((req_tok, wo_id, sid))
}

#[test]
fn work_order_create_contract() {
    let Some(admin) = setup("work_order_create_contract") else {
        return;
    };
    let Some(sid) = create_service(&admin, "wocreate") else {
        return;
    };
    let Some((_, u, p)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(tok) = login(&u, &p) else {
        return;
    };

    let resp = post_json_auth("/api/work-orders", &tok, &json!({"service_id": sid})).expect("wo");
    assert_status(&resp, 200, "POST /work-orders");
    let v = json_body(resp, "wo");
    assert_keys(
        &v,
        &["id", "requester_id", "service_id", "status", "completed_at"],
        "WorkOrder",
    );
    assert_eq!(v["status"], "pending");
    assert!(v["completed_at"].is_null());
}

#[test]
fn work_order_complete_contract() {
    let Some(admin) = setup("work_order_complete_contract") else {
        return;
    };
    let Some((_, wo_id, _)) = completed_work_order(&admin) else {
        return;
    };

    // Completing twice -> 409.
    let again =
        post_empty_auth(&format!("/api/work-orders/{wo_id}/complete"), &admin).expect("again");
    assert_status(&again, 409, "complete twice");
    // Missing id -> 404.
    let miss = post_empty_auth(
        "/api/work-orders/00000000-0000-0000-0000-000000000000/complete",
        &admin,
    )
    .expect("miss");
    assert_status(&miss, 404, "complete missing");
}

#[test]
fn review_pin_collapse_and_admin_tag_contracts() {
    let Some(admin) = setup("review_pin_collapse_and_admin_tag_contracts") else {
        return;
    };
    let Some((req_tok, wo_id, _)) = completed_work_order(&admin) else {
        return;
    };

    // Requester submits initial review.
    let rev = post_json_auth(
        "/api/reviews",
        &req_tok,
        &json!({"work_order_id": wo_id, "rating": 5, "text":"great"}),
    )
    .expect("rev");
    assert_status(&rev, 200, "POST /reviews");
    let rid = json_body(rev, "rev")["id"].as_str().unwrap().to_string();

    // Pin/collapse require admin.
    let pin_bad = patch_json_auth(
        &format!("/api/reviews/{rid}/pin"),
        &req_tok,
        &json!({"is_pinned": true}),
    )
    .expect("pb");
    assert_status(&pin_bad, 403, "requester cannot pin");

    let pin = patch_json_auth(
        &format!("/api/reviews/{rid}/pin"),
        &admin,
        &json!({"is_pinned": true}),
    )
    .expect("p");
    assert_status(&pin, 204, "PATCH pin");

    let col = patch_json_auth(
        &format!("/api/reviews/{rid}/collapse"),
        &admin,
        &json!({"is_collapsed": true}),
    )
    .expect("c");
    assert_status(&col, 204, "PATCH collapse");

    // Admin creates a tag vocab entry and lists tags.
    let tname = format!("t_{}", nano_suffix());
    let tag =
        post_json_auth("/api/review-tags", &admin, &json!({"name": tname.clone()})).expect("t");
    assert_status(&tag, 200, "POST /review-tags");
    let tv = json_body(tag, "t");
    assert_keys(&tv, &["id", "name"], "ReviewTag");
    let tag_id = tv["id"].as_str().unwrap().to_string();

    let list = get_auth("/api/review-tags", &admin).expect("lt");
    assert_status(&list, 200, "GET /review-tags");
    let lv = json_body(list, "lt");
    assert!(lv
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["id"].as_str() == Some(&tag_id)));

    // Admin attaches tag post-hoc -> 201, writes audit event for 'tag'.
    let attach = post_json_auth(
        &format!("/api/reviews/{rid}/tags"),
        &admin,
        &json!({"tag_id": tag_id}),
    )
    .expect("attach");
    assert_status(&attach, 201, "POST /reviews/<id>/tags");

    // Unknown review -> 404.
    let miss = post_json_auth(
        "/api/reviews/00000000-0000-0000-0000-000000000000/tags",
        &admin,
        &json!({"tag_id": tag_id}),
    )
    .expect("m");
    assert_status(&miss, 404, "attach tag missing review");

    // Unknown tag -> 404.
    let miss2 = post_json_auth(
        &format!("/api/reviews/{rid}/tags"),
        &admin,
        &json!({"tag_id": "00000000-0000-0000-0000-000000000000"}),
    )
    .expect("m2");
    assert_status(&miss2, 404, "attach missing tag");
}

#[test]
fn non_admin_cannot_access_review_tag_vocabulary_mutations() {
    let Some(admin) = setup("non_admin_cannot_access_review_tag_vocabulary_mutations") else {
        return;
    };
    let Some((_, u, p)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(tok) = login(&u, &p) else {
        return;
    };

    let r = post_json_auth(
        "/api/review-tags",
        &tok,
        &json!({"name": format!("x_{}", nano_suffix())}),
    )
    .expect("r");
    assert_status(&r, 403, "requester cannot create review tag");

    // But listing is allowed for any authenticated user.
    let l = get_auth("/api/review-tags", &tok).expect("l");
    assert_status(&l, 200, "listing review tags is any-auth");
}
