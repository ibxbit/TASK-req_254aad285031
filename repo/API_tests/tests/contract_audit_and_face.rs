//! Contract coverage for audit chain verification + face records endpoints.
//! Exercises the "admin only" lanes and the face capture/upload pipeline
//! that other test files don't touch.

use api_tests::{
    assert_keys, assert_status, client, get_auth, json_body, login, post_empty_auth,
    post_json_auth, provision_user, setup,
};
use serde_json::json;

#[test]
fn audit_verify_contract() {
    let Some(admin) = setup("audit_verify_contract") else {
        return;
    };
    let resp = get_auth("/api/audit/verify", &admin).expect("v");
    assert_status(&resp, 200, "GET /api/audit/verify");
    let v = json_body(resp, "verify");
    // Contract = shared::AuditVerifyReport (see shared/src/audit.rs):
    //   { total_events: i64, verified: i64, tampered: i64,
    //     issues: Vec<AuditVerifyIssue> }
    assert_keys(
        &v,
        &["total_events", "verified", "tampered", "issues"],
        "AuditVerifyReport",
    );
    assert!(v["total_events"].as_i64().is_some(), "total_events is i64");
    assert!(v["verified"].as_i64().is_some(), "verified is i64");
    assert!(v["tampered"].as_i64().is_some(), "tampered is i64");
    assert!(v["issues"].is_array(), "issues is array");
    // Invariant: every event is either verified or tampered.
    let total = v["total_events"].as_i64().unwrap();
    let verified = v["verified"].as_i64().unwrap();
    let tampered = v["tampered"].as_i64().unwrap();
    assert_eq!(
        total,
        verified + tampered,
        "total_events must equal verified + tampered"
    );
}

#[test]
fn audit_verify_requires_admin() {
    let Some(admin) = setup("audit_verify_requires_admin") else {
        return;
    };
    let Some((_, u, p)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(tok) = login(&u, &p) else {
        return;
    };
    let r = get_auth("/api/audit/verify", &tok).expect("r");
    assert_status(&r, 403, "requester cannot verify audit");
}

#[test]
fn audit_list_for_entity_contract() {
    let Some(admin) = setup("audit_list_for_entity_contract") else {
        return;
    };
    // A random entity id shouldn't 500 — it just returns []. Contract is
    // "array of EventLog".
    let resp = get_auth(
        "/api/audit/review/00000000-0000-0000-0000-000000000000",
        &admin,
    )
    .expect("r");
    assert_status(&resp, 200, "GET /api/audit/<entity>/<id>");
    let v = json_body(resp, "list");
    assert!(v.is_array());

    // Bad UUID -> 400.
    let bad = get_auth("/api/audit/review/not-a-uuid", &admin).expect("bad");
    assert_status(&bad, 400, "bad entity id");
}

// 1x1 PNG bytes (valid header) that the face pipeline should still reject
// with 422 on "resolution below minimum" — we're validating the error
// shape of the endpoint, not creating valid face records.
fn tiny_png_bytes() -> Vec<u8> {
    // 1x1 PNG (red pixel) from a canned reference encoding.
    const DATA: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0x99, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0x5B, 0x9E, 0xD9, 0xD5, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    DATA.to_vec()
}

#[test]
fn face_upload_undersized_returns_422() {
    let Some(admin) = setup("face_upload_undersized_returns_422") else {
        return;
    };
    let form = reqwest::blocking::multipart::Form::new().part(
        "file",
        reqwest::blocking::multipart::Part::bytes(tiny_png_bytes())
            .file_name("tiny.png")
            .mime_str("image/png")
            .unwrap(),
    );
    let resp = client()
        .post(format!("{}/api/faces", api_tests::api_base()))
        .bearer_auth(&admin)
        .multipart(form)
        .send()
        .expect("upload");
    // 422 from the resolution check, or 400 if earlier validation fires.
    assert!(
        matches!(resp.status().as_u16(), 400 | 422),
        "undersized face upload -> {}",
        resp.status()
    );
}

#[test]
fn face_upload_wrong_content_type_is_415() {
    let Some(admin) = setup("face_upload_wrong_content_type_is_415") else {
        return;
    };
    let form = reqwest::blocking::multipart::Form::new().part(
        "file",
        reqwest::blocking::multipart::Part::bytes(b"not-an-image".to_vec())
            .file_name("x.txt")
            .mime_str("text/plain")
            .unwrap(),
    );
    let resp = client()
        .post(format!("{}/api/faces", api_tests::api_base()))
        .bearer_auth(&admin)
        .multipart(form)
        .send()
        .expect("upload");
    assert_status(&resp, 415, "wrong content-type for face");
}

#[test]
fn face_endpoints_with_missing_record_return_404() {
    let Some(admin) = setup("face_endpoints_with_missing_record_return_404") else {
        return;
    };
    let missing = "00000000-0000-0000-0000-000000000000";
    let v = post_empty_auth(&format!("/api/faces/{missing}/validate"), &admin).expect("v");
    assert_status(&v, 404, "validate missing face");
    let d = post_empty_auth(&format!("/api/faces/{missing}/deactivate"), &admin).expect("d");
    assert_status(&d, 404, "deactivate missing face");
    let l = post_json_auth(
        &format!("/api/faces/{missing}/liveness"),
        &admin,
        &json!({"challenge":"blink","passed":true}),
    )
    .expect("l");
    // Either 404 (no such face record) or 400 (bad challenge) depending on
    // order of checks. Both are explicit rejections, not 500.
    assert!(
        matches!(l.status().as_u16(), 400 | 404),
        "liveness on missing face -> {}",
        l.status()
    );

    // Listing faces for a user returns 200 + array (possibly empty). This
    // is the admin-read path.
    let list = get_auth(&format!("/api/faces/{missing}"), &admin).expect("list");
    assert_status(&list, 200, "list faces for user");
    let v = json_body(list, "list");
    assert!(v.is_array());
}
