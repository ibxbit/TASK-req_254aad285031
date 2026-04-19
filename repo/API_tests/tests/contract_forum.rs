//! Contract coverage for /api/zones, /api/boards, /api/posts, /api/comments,
//! /api/rules — every forum endpoint with body contract + negative cases.

use api_tests::{
    assert_keys, assert_status, delete_auth, get_auth, json_body, login, nano_suffix,
    patch_json_auth, post_json_auth, provision_user, setup,
};
use serde_json::{json, Value};

fn make_zone_and_board(admin: &str, visibility: &str) -> Option<(String, String)> {
    let zone = post_json_auth(
        "/api/zones",
        admin,
        &json!({"name": format!("z_{}", nano_suffix())}),
    )
    .ok()?;
    if !zone.status().is_success() {
        return None;
    }
    let zv: Value = zone.json().ok()?;
    let zid = zv["id"].as_str()?.to_string();
    let board = post_json_auth(
        "/api/boards",
        admin,
        &json!({
            "zone_id": zid,
            "name": format!("b_{}", nano_suffix()),
            "visibility_type": visibility,
        }),
    )
    .ok()?;
    if !board.status().is_success() {
        return None;
    }
    let bv: Value = board.json().ok()?;
    Some((zid, bv["id"].as_str()?.to_string()))
}

#[test]
fn zones_crud_contract() {
    let Some(admin) = setup("zones_crud_contract") else {
        return;
    };

    let name = format!("z_{}", nano_suffix());
    let create = post_json_auth("/api/zones", &admin, &json!({"name": name})).expect("z");
    assert_status(&create, 200, "POST /zones");
    let v = json_body(create, "POST /zones");
    assert_keys(&v, &["id", "name"], "Zone contract");
    assert_eq!(v["name"], name);
    let zid = v["id"].as_str().unwrap().to_string();

    let list = get_auth("/api/zones", &admin).expect("zl");
    assert_status(&list, 200, "GET /zones");
    let lv = json_body(list, "list");
    assert!(lv.is_array());
    assert!(lv
        .as_array()
        .unwrap()
        .iter()
        .any(|z| z["id"].as_str() == Some(&zid)));

    let renamed = format!("z2_{}", nano_suffix());
    let upd = patch_json_auth(
        &format!("/api/zones/{zid}"),
        &admin,
        &json!({"name": renamed}),
    )
    .expect("zu");
    assert_status(&upd, 204, "PATCH /zones/<id>");

    let missing = patch_json_auth(
        "/api/zones/00000000-0000-0000-0000-000000000000",
        &admin,
        &json!({"name":"x"}),
    )
    .expect("zu-miss");
    assert_status(&missing, 404, "PATCH missing zone");

    let del = delete_auth(&format!("/api/zones/{zid}"), &admin).expect("zd");
    assert_status(&del, 204, "DELETE /zones/<id>");
    let del_again = delete_auth(&format!("/api/zones/{zid}"), &admin).expect("zd2");
    assert_status(&del_again, 404, "DELETE missing zone");
}

#[test]
fn zones_create_requires_admin() {
    let Some(admin) = setup("zones_create_requires_admin") else {
        return;
    };
    let Some((_, u, p)) = provision_user(&admin, "moderator") else {
        return;
    };
    let Some(tok) = login(&u, &p) else {
        return;
    };
    let r = post_json_auth("/api/zones", &tok, &json!({"name":"no"})).expect("r");
    assert_status(&r, 403, "moderator cannot create zones");
}

#[test]
fn boards_crud_and_visibility_contract() {
    let Some(admin) = setup("boards_crud_and_visibility_contract") else {
        return;
    };
    let Some((_zid, bid)) = make_zone_and_board(&admin, "public") else {
        return;
    };

    // GET /boards
    let list = get_auth("/api/boards", &admin).expect("list");
    assert_status(&list, 200, "GET /boards");
    let lv = json_body(list, "list");
    assert!(lv.is_array());
    let found = lv
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["id"].as_str() == Some(&bid))
        .expect("created board must be listed");
    assert_keys(
        found,
        &["id", "zone_id", "name", "visibility_type", "created_by"],
        "Board",
    );
    assert_eq!(found["visibility_type"], "public");

    // GET /boards/<id>
    let one = get_auth(&format!("/api/boards/{bid}"), &admin).expect("one");
    assert_status(&one, 200, "GET /boards/<id>");

    // PATCH visibility -> restricted
    let upd = patch_json_auth(
        &format!("/api/boards/{bid}"),
        &admin,
        &json!({"visibility_type":"restricted"}),
    )
    .expect("upd");
    assert_status(&upd, 204, "PATCH /boards/<id>");

    let one = get_auth(&format!("/api/boards/{bid}"), &admin).expect("one2");
    assert_eq!(
        json_body(one, "one2")["visibility_type"],
        "restricted",
        "update reflected"
    );

    // DELETE /boards/<id> -> 204, repeat -> 404
    let del = delete_auth(&format!("/api/boards/{bid}"), &admin).expect("d");
    assert_status(&del, 204, "DELETE /boards/<id>");
    let del2 = delete_auth(&format!("/api/boards/{bid}"), &admin).expect("d2");
    assert_status(&del2, 404, "DELETE missing board");
}

#[test]
fn board_moderators_assign_remove() {
    let Some(admin) = setup("board_moderators_assign_remove") else {
        return;
    };
    let Some((_, bid)) = make_zone_and_board(&admin, "public") else {
        return;
    };
    let Some((uid, _u, _p)) = provision_user(&admin, "moderator") else {
        return;
    };

    let add = post_json_auth(
        &format!("/api/boards/{bid}/moderators"),
        &admin,
        &json!({"user_id": uid}),
    )
    .expect("add");
    assert_status(&add, 201, "POST /boards/<id>/moderators");

    // Duplicate -> 400/409 (insert bound to unique key).
    let dup = post_json_auth(
        &format!("/api/boards/{bid}/moderators"),
        &admin,
        &json!({"user_id": uid}),
    )
    .expect("dup");
    assert!(
        matches!(dup.status().as_u16(), 400 | 409),
        "duplicate mod -> {}",
        dup.status()
    );

    let rm = delete_auth(&format!("/api/boards/{bid}/moderators/{uid}"), &admin).expect("rm");
    assert_status(&rm, 204, "DELETE mod");
    let rm2 = delete_auth(&format!("/api/boards/{bid}/moderators/{uid}"), &admin).expect("rm2");
    assert_status(&rm2, 404, "DELETE absent mod");
}

#[test]
fn board_rules_crud() {
    let Some(admin) = setup("board_rules_crud") else {
        return;
    };
    let Some((_, bid)) = make_zone_and_board(&admin, "public") else {
        return;
    };

    // GET rules — empty array.
    let list = get_auth(&format!("/api/boards/{bid}/rules"), &admin).expect("list");
    assert_status(&list, 200, "GET rules");
    assert!(json_body(list, "list").is_array());

    // Create.
    let c = post_json_auth(
        &format!("/api/boards/{bid}/rules"),
        &admin,
        &json!({"content": "Be nice"}),
    )
    .expect("c");
    assert_status(&c, 200, "POST rule");
    let v = json_body(c, "rule");
    assert_keys(&v, &["id", "board_id", "content"], "BoardRule");
    let rid = v["id"].as_str().unwrap().to_string();

    // Delete.
    let d = delete_auth(&format!("/api/rules/{rid}"), &admin).expect("d");
    assert_status(&d, 204, "DELETE rule");

    // Missing rule -> 404.
    let d2 = delete_auth("/api/rules/00000000-0000-0000-0000-000000000000", &admin).expect("d2");
    assert_status(&d2, 404, "DELETE missing rule");
}

#[test]
fn post_lifecycle_and_visibility() {
    let Some(admin) = setup("post_lifecycle_and_visibility") else {
        return;
    };
    let Some((_, bid)) = make_zone_and_board(&admin, "public") else {
        return;
    };

    let create = post_json_auth(
        "/api/posts",
        &admin,
        &json!({"board_id": bid, "title": "hello", "content": "world"}),
    )
    .expect("p");
    assert_status(&create, 200, "POST /posts");
    let pv = json_body(create, "post");
    assert_keys(
        &pv,
        &[
            "id",
            "board_id",
            "author_id",
            "title",
            "content",
            "is_pinned",
            "created_at",
        ],
        "Post",
    );
    let pid = pv["id"].as_str().unwrap().to_string();

    // list_by_board.
    let list = get_auth(&format!("/api/boards/{bid}/posts"), &admin).expect("l");
    assert_status(&list, 200, "GET posts");
    assert!(json_body(list, "l")
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["id"].as_str() == Some(&pid)));

    // GET /posts/<id>
    let one = get_auth(&format!("/api/posts/{pid}"), &admin).expect("o");
    assert_status(&one, 200, "GET /posts/<id>");

    // Pin (admin always can moderate).
    let pin = patch_json_auth(
        &format!("/api/posts/{pid}/pin"),
        &admin,
        &json!({"is_pinned": true}),
    )
    .expect("pin");
    assert_status(&pin, 204, "PATCH pin");

    // Pin on missing post -> 404
    let missing = patch_json_auth(
        "/api/posts/00000000-0000-0000-0000-000000000000/pin",
        &admin,
        &json!({"is_pinned":true}),
    )
    .expect("miss");
    assert_status(&missing, 404, "pin missing");
}

#[test]
fn comments_crud_contract() {
    let Some(admin) = setup("comments_crud_contract") else {
        return;
    };
    let Some((_, bid)) = make_zone_and_board(&admin, "public") else {
        return;
    };
    let post = post_json_auth(
        "/api/posts",
        &admin,
        &json!({"board_id": bid, "title": "t", "content": "c"}),
    )
    .expect("p");
    let pid = json_body(post, "p")["id"].as_str().unwrap().to_string();

    // Create comment.
    let c = post_json_auth(
        "/api/comments",
        &admin,
        &json!({"post_id": pid, "content": "first!"}),
    )
    .expect("c");
    assert_status(&c, 200, "POST /comments");
    let cv = json_body(c, "comment");
    assert_keys(
        &cv,
        &["id", "post_id", "author_id", "content", "created_at"],
        "Comment",
    );
    let cid = cv["id"].as_str().unwrap().to_string();

    // List comments.
    let list = get_auth(&format!("/api/posts/{pid}/comments"), &admin).expect("l");
    assert_status(&list, 200, "GET comments");
    let lv = json_body(list, "l");
    assert!(lv.is_array());
    assert!(lv
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["id"].as_str() == Some(&cid)));

    // Author deletes own comment -> 204.
    let d = delete_auth(&format!("/api/comments/{cid}"), &admin).expect("d");
    assert_status(&d, 204, "DELETE /comments/<id>");

    // Delete missing -> 404.
    let d2 = delete_auth("/api/comments/00000000-0000-0000-0000-000000000000", &admin).expect("d2");
    assert_status(&d2, 404, "DELETE missing comment");
}

#[test]
fn comment_delete_blocked_for_non_author() {
    let Some(admin) = setup("comment_delete_blocked_for_non_author") else {
        return;
    };
    let Some((_, bid)) = make_zone_and_board(&admin, "public") else {
        return;
    };
    let Some((_, u, p)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(tok) = login(&u, &p) else {
        return;
    };

    // Admin creates post + comment.
    let post_resp = post_json_auth(
        "/api/posts",
        &admin,
        &json!({"board_id": bid, "title": "t", "content": "c"}),
    )
    .expect("p");
    let pid = json_body(post_resp, "p")["id"]
        .as_str()
        .unwrap()
        .to_string();

    let comment_resp = post_json_auth(
        "/api/comments",
        &admin,
        &json!({"post_id": pid, "content": "hi"}),
    )
    .expect("c");
    let cid = json_body(comment_resp, "c")["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Non-author requester attempts delete -> expect 403/404.
    let r = delete_auth(&format!("/api/comments/{cid}"), &tok).expect("r");
    assert!(
        matches!(r.status().as_u16(), 403 | 404),
        "non-author delete -> {}",
        r.status()
    );
}
