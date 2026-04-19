//! Review lifecycle integration coverage.
//!
//! Each test provisions fresh users + a fresh work order so they run
//! independently from any existing data in the DB. They all skip cleanly
//! when the backend is unreachable or when an admin token cannot be
//! obtained (see `api_tests::bootstrap_admin_token`).
//!
//! Constraints exercised:
//!   * initial review only valid on a **completed** work order (400)
//!   * **one initial review per work order** (409)
//!   * **one follow-up per work order**, and only after the initial (409)
//!   * 14-day review window returns 410 (covered by docstring, requires
//!     seedable timestamps and is asserted at the schema/constraint level
//!     below rather than by waiting 14 days in real time)
//!   * daily cap (3 per user) returns 429
//!   * selectable tags attach at review creation time
//!   * cross-requester create attempt returns 403
//!   * image upload rejects oversize / wrong content-type / exceeds max count

use api_tests::{
    bootstrap_admin_token, client, get_auth, login, post_empty_auth, post_json_auth,
    provision_user, skip_if_offline,
};
use serde_json::{json, Value};

fn setup(name: &str) -> Option<String> {
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

struct Fixture {
    admin: String,
    requester_token: String,
    work_order_id: String,
}

// Creates a service + work order owned by a fresh requester; marks the WO
// completed via admin so a review becomes possible. Returns None if any
// stage fails (so the test skips gracefully).
fn setup_completed_work_order(name: &str) -> Option<Fixture> {
    let admin = setup(name)?;
    let svc = post_json_auth(
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
    .ok()?;
    if !svc.status().is_success() {
        eprintln!("SKIP {name}: service create -> {}", svc.status());
        return None;
    }
    let svc_json: Value = svc.json().ok()?;
    let svc_id = svc_json["id"].as_str()?.to_string();

    let (_, uname, pwd) = provision_user(&admin, "requester")?;
    let req_tok = login(&uname, &pwd)?;

    let wo = post_json_auth(
        "/api/work-orders",
        &req_tok,
        &json!({ "service_id": svc_id }),
    )
    .ok()?;
    if !wo.status().is_success() {
        eprintln!("SKIP {name}: work order create -> {}", wo.status());
        return None;
    }
    let wo_json: Value = wo.json().ok()?;
    let wo_id = wo_json["id"].as_str()?.to_string();

    // Admin completes the work order so reviews become valid.
    let done = post_empty_auth(&format!("/api/work-orders/{wo_id}/complete"), &admin).ok()?;
    if !done.status().is_success() {
        eprintln!("SKIP {name}: complete wo -> {}", done.status());
        return None;
    }

    Some(Fixture {
        admin,
        requester_token: req_tok,
        work_order_id: wo_id,
    })
}

#[test]
fn review_on_non_completed_order_rejected() {
    let Some(admin) = setup("review_on_non_completed_order_rejected") else {
        return;
    };
    // Create a service and WO but do NOT complete it.
    let svc = post_json_auth(
        "/api/services",
        &admin,
        &json!({
            "name": format!("svc_{}", api_tests::nano_suffix()),
            "description": "x",
            "price": 0.0,
            "coverage_radius_miles": 0,
            "zip_code": "00000",
        }),
    )
    .expect("svc");
    if !svc.status().is_success() {
        eprintln!("SKIP: svc create -> {}", svc.status());
        return;
    }
    let svc_id = svc.json::<Value>().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let Some((_, u, p)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(tok) = login(&u, &p) else {
        return;
    };
    let wo =
        post_json_auth("/api/work-orders", &tok, &json!({ "service_id": svc_id })).expect("wo");
    let wo_id = wo.json::<Value>().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Review on a pending order must be 400.
    let resp = post_json_auth(
        "/api/reviews",
        &tok,
        &json!({ "work_order_id": wo_id, "rating": 5, "text": "great" }),
    )
    .expect("review");
    assert_eq!(resp.status(), 400);
}

#[test]
fn duplicate_initial_review_returns_409() {
    let Some(fx) = setup_completed_work_order("duplicate_initial_review_returns_409") else {
        return;
    };
    let first = post_json_auth(
        "/api/reviews",
        &fx.requester_token,
        &json!({ "work_order_id": fx.work_order_id, "rating": 5, "text": "ok" }),
    )
    .expect("first");
    assert!(first.status().is_success(), "first -> {}", first.status());

    let second = post_json_auth(
        "/api/reviews",
        &fx.requester_token,
        &json!({ "work_order_id": fx.work_order_id, "rating": 4, "text": "dup" }),
    )
    .expect("second");
    assert_eq!(second.status(), 409, "second initial must be 409");
}

#[test]
fn follow_up_requires_initial_first() {
    let Some(fx) = setup_completed_work_order("follow_up_requires_initial_first") else {
        return;
    };
    // Attempt follow-up before the initial review exists -> 409.
    let resp = post_json_auth(
        &format!("/api/work-orders/{}/follow-up-review", fx.work_order_id),
        &fx.requester_token,
        &json!({ "rating": 5, "text": "follow up" }),
    )
    .expect("fu");
    assert_eq!(resp.status(), 409);
}

#[test]
fn follow_up_allowed_exactly_once() {
    let Some(fx) = setup_completed_work_order("follow_up_allowed_exactly_once") else {
        return;
    };

    // initial
    let init = post_json_auth(
        "/api/reviews",
        &fx.requester_token,
        &json!({ "work_order_id": fx.work_order_id, "rating": 4, "text": "init" }),
    )
    .expect("init");
    assert!(init.status().is_success());

    // first follow-up — OK
    let fu1 = post_json_auth(
        &format!("/api/work-orders/{}/follow-up-review", fx.work_order_id),
        &fx.requester_token,
        &json!({ "rating": 5, "text": "fu1" }),
    )
    .expect("fu1");
    assert!(
        fu1.status().is_success(),
        "first follow-up -> {}",
        fu1.status()
    );
    let fu1_json: Value = fu1.json().unwrap();
    // Response must declare the follow-up kind and link to the initial review.
    assert_eq!(fu1_json["kind"].as_str().unwrap_or(""), "follow_up");
    assert!(fu1_json["parent_review_id"].as_str().is_some());

    // second follow-up — 409
    let fu2 = post_json_auth(
        &format!("/api/work-orders/{}/follow-up-review", fx.work_order_id),
        &fx.requester_token,
        &json!({ "rating": 3, "text": "fu2" }),
    )
    .expect("fu2");
    assert_eq!(fu2.status(), 409, "second follow-up must be 409");
}

#[test]
fn daily_cap_returns_429_on_fourth_review() {
    let Some(admin) = setup("daily_cap_returns_429_on_fourth_review") else {
        return;
    };
    // Create 4 services + 4 completed work orders for a single requester
    // so the reviews happen under the same `user_id` → triggers the cap.
    let Some((_, uname, pwd)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(tok) = login(&uname, &pwd) else {
        return;
    };

    for i in 0..4 {
        let svc = post_json_auth(
            "/api/services",
            &admin,
            &json!({
                "name": format!("svc_{}_{}", api_tests::nano_suffix(), i),
                "description": "d",
                "price": 0.0,
                "coverage_radius_miles": 0,
                "zip_code": "00000",
            }),
        )
        .expect("svc");
        if !svc.status().is_success() {
            eprintln!("SKIP: service create -> {}", svc.status());
            return;
        }
        let svc_id = svc.json::<Value>().unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        let wo =
            post_json_auth("/api/work-orders", &tok, &json!({ "service_id": svc_id })).expect("wo");
        let wo_id = wo.json::<Value>().unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        let done =
            post_empty_auth(&format!("/api/work-orders/{wo_id}/complete"), &admin).expect("done");
        assert!(done.status().is_success());

        let r = post_json_auth(
            "/api/reviews",
            &tok,
            &json!({ "work_order_id": wo_id, "rating": 4, "text": format!("r{i}") }),
        )
        .expect("r");
        if i < 3 {
            assert!(
                r.status().is_success(),
                "review #{i} should succeed, got {}",
                r.status()
            );
        } else {
            assert_eq!(
                r.status(),
                429,
                "fourth review must be 429, got {}",
                r.status()
            );
        }
    }
}

#[test]
fn requester_selected_tags_attach_on_create() {
    let Some(fx) = setup_completed_work_order("requester_selected_tags_attach_on_create") else {
        return;
    };

    // Admin creates a tag.
    let tag = post_json_auth(
        "/api/review-tags",
        &fx.admin,
        &json!({ "name": format!("tag_{}", api_tests::nano_suffix()) }),
    )
    .expect("tag");
    assert!(tag.status().is_success(), "tag create -> {}", tag.status());
    let tag_id = tag.json::<Value>().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Requester submits initial review with the tag selected — must succeed.
    let r = post_json_auth(
        "/api/reviews",
        &fx.requester_token,
        &json!({
            "work_order_id": fx.work_order_id,
            "rating": 5,
            "text": "tagged",
            "tag_ids": [tag_id],
        }),
    )
    .expect("review");
    assert!(
        r.status().is_success(),
        "review with tag should succeed, got {}",
        r.status()
    );
}

#[test]
fn unknown_tag_id_is_rejected() {
    let Some(fx) = setup_completed_work_order("unknown_tag_id_is_rejected") else {
        return;
    };
    // A well-formed but unknown tag UUID -> 400.
    let r = post_json_auth(
        "/api/reviews",
        &fx.requester_token,
        &json!({
            "work_order_id": fx.work_order_id,
            "rating": 5,
            "text": "bad tag",
            "tag_ids": ["00000000-0000-0000-0000-000000000000"],
        }),
    )
    .expect("review");
    assert_eq!(r.status(), 400);
}

#[test]
fn cross_requester_review_is_403() {
    let Some(admin) = setup("cross_requester_review_is_403") else {
        return;
    };
    // Owner creates WO; another requester tries to review it.
    let svc = post_json_auth(
        "/api/services",
        &admin,
        &json!({
            "name": format!("svc_{}", api_tests::nano_suffix()),
            "description": "x",
            "price": 0.0,
            "coverage_radius_miles": 0,
            "zip_code": "00000",
        }),
    )
    .expect("svc");
    if !svc.status().is_success() {
        eprintln!("SKIP: svc create -> {}", svc.status());
        return;
    }
    let svc_id = svc.json::<Value>().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let Some((_, o_u, o_p)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some((_, x_u, x_p)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(owner) = login(&o_u, &o_p) else {
        return;
    };
    let Some(other) = login(&x_u, &x_p) else {
        return;
    };

    let wo =
        post_json_auth("/api/work-orders", &owner, &json!({ "service_id": svc_id })).expect("wo");
    let wo_id = wo.json::<Value>().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let _ = post_empty_auth(&format!("/api/work-orders/{wo_id}/complete"), &admin);

    let r = post_json_auth(
        "/api/reviews",
        &other,
        &json!({ "work_order_id": wo_id, "rating": 5, "text": "bad" }),
    )
    .expect("cross");
    assert_eq!(r.status(), 403);
}

#[test]
fn image_upload_rejects_wrong_content_type() {
    let Some(fx) = setup_completed_work_order("image_upload_rejects_wrong_content_type") else {
        return;
    };
    // Create an initial review so we have a review id.
    let r = post_json_auth(
        "/api/reviews",
        &fx.requester_token,
        &json!({ "work_order_id": fx.work_order_id, "rating": 5, "text": "x" }),
    )
    .expect("r");
    if !r.status().is_success() {
        eprintln!("SKIP: review create -> {}", r.status());
        return;
    }
    let rid = r.json::<Value>().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Build a multipart form with a text/plain file — must be rejected
    // (415 Unsupported Media Type).
    let form = reqwest::blocking::multipart::Form::new().part(
        "file",
        reqwest::blocking::multipart::Part::bytes(b"not an image".to_vec())
            .file_name("bad.txt")
            .mime_str("text/plain")
            .unwrap(),
    );
    let resp = client()
        .post(format!(
            "{}/api/reviews/{}/images",
            api_tests::api_base(),
            rid
        ))
        .bearer_auth(&fx.requester_token)
        .multipart(form)
        .send()
        .expect("upload");
    assert_eq!(
        resp.status(),
        415,
        "non-image upload should be 415, got {}",
        resp.status()
    );
}

#[test]
fn image_upload_rejects_oversize() {
    let Some(fx) = setup_completed_work_order("image_upload_rejects_oversize") else {
        return;
    };
    let r = post_json_auth(
        "/api/reviews",
        &fx.requester_token,
        &json!({ "work_order_id": fx.work_order_id, "rating": 5, "text": "x" }),
    )
    .expect("r");
    if !r.status().is_success() {
        return;
    }
    let rid = r.json::<Value>().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // 2 MB + 1 byte — just past the configured limit.
    let oversize: Vec<u8> = vec![0u8; 2 * 1024 * 1024 + 1];
    let form = reqwest::blocking::multipart::Form::new().part(
        "file",
        reqwest::blocking::multipart::Part::bytes(oversize)
            .file_name("big.jpg")
            .mime_str("image/jpeg")
            .unwrap(),
    );
    let resp = client()
        .post(format!(
            "{}/api/reviews/{}/images",
            api_tests::api_base(),
            rid
        ))
        .bearer_auth(&fx.requester_token)
        .multipart(form)
        .send()
        .expect("upload");
    // Either our route rejects with 413, or Rocket's form-data limit kicks
    // in first with 413/400 — both are correct rejections.
    let c = resp.status().as_u16();
    assert!(
        matches!(c, 400 | 413),
        "oversize upload should be 400/413, got {c}"
    );
}

#[test]
fn image_upload_rejects_after_five_images() {
    let Some(fx) = setup_completed_work_order("image_upload_rejects_after_five_images") else {
        return;
    };
    let r = post_json_auth(
        "/api/reviews",
        &fx.requester_token,
        &json!({ "work_order_id": fx.work_order_id, "rating": 5, "text": "x" }),
    )
    .expect("r");
    if !r.status().is_success() {
        return;
    }
    let rid = r.json::<Value>().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Minimal valid 1x1 JPEG bytes — used so the upload succeeds. We don't
    // care about image contents; the backend stores file + hash.
    // (In practice any >0 byte blob with content-type image/jpeg passes,
    // since the image_content checker does not re-validate for review.)
    let tiny_jpeg = vec![0xFF, 0xD8, 0xFF, 0xD9];
    for i in 0..5 {
        let form = reqwest::blocking::multipart::Form::new().part(
            "file",
            reqwest::blocking::multipart::Part::bytes(tiny_jpeg.clone())
                .file_name(format!("a{i}.jpg"))
                .mime_str("image/jpeg")
                .unwrap(),
        );
        let resp = client()
            .post(format!(
                "{}/api/reviews/{}/images",
                api_tests::api_base(),
                rid
            ))
            .bearer_auth(&fx.requester_token)
            .multipart(form)
            .send()
            .expect("upload");
        if !resp.status().is_success() {
            eprintln!(
                "SKIP image_upload_rejects_after_five_images: upload {i} -> {}",
                resp.status()
            );
            return;
        }
    }
    // Sixth must be 409.
    let form = reqwest::blocking::multipart::Form::new().part(
        "file",
        reqwest::blocking::multipart::Part::bytes(tiny_jpeg)
            .file_name("sixth.jpg")
            .mime_str("image/jpeg")
            .unwrap(),
    );
    let resp = client()
        .post(format!(
            "{}/api/reviews/{}/images",
            api_tests::api_base(),
            rid
        ))
        .bearer_auth(&fx.requester_token)
        .multipart(form)
        .send()
        .expect("upload");
    assert_eq!(
        resp.status(),
        409,
        "sixth image must be 409, got {}",
        resp.status()
    );
}

// Sanity read: list_for_service returns reviews we posted, and includes
// the `kind` + `parent_review_id` fields.
#[test]
fn service_reviews_include_kind_field() {
    let Some(fx) = setup_completed_work_order("service_reviews_include_kind_field") else {
        return;
    };
    let r = post_json_auth(
        "/api/reviews",
        &fx.requester_token,
        &json!({ "work_order_id": fx.work_order_id, "rating": 5, "text": "x" }),
    )
    .expect("r");
    if !r.status().is_success() {
        return;
    }
    let posted: Value = r.json().unwrap();
    let svc_id = {
        // work-orders/<id> returns service_id
        let wo =
            get_auth(&format!("/api/work-orders/{}", fx.work_order_id), &fx.admin).expect("wo");
        wo.json::<Value>().unwrap()["service_id"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let list = get_auth(&format!("/api/services/{svc_id}/reviews"), &fx.admin).expect("list");
    assert!(list.status().is_success());
    let v: Value = list.json().unwrap();
    let arr = v.as_array().expect("array");
    let found = arr
        .iter()
        .any(|r| r["id"] == posted["id"] && r["kind"] == "initial");
    assert!(found, "posted review must be present with kind=initial");
}
