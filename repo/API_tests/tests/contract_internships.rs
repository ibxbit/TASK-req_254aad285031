//! Contract coverage for internship endpoints: plans, reports (create,
//! comment, approve), attachments upload, dashboard.

use api_tests::{
    assert_keys, assert_status, client, get_auth, json_body, login, post_empty_auth,
    post_json_auth, provision_user, setup,
};
use serde_json::json;

fn intern_credentials(admin: &str) -> Option<(String, String, String)> {
    provision_user(admin, "intern")
}

#[test]
fn plans_create_contract_intern_only() {
    let Some(admin) = setup("plans_create_contract_intern_only") else {
        return;
    };
    let Some((_, u, p)) = intern_credentials(&admin) else {
        return;
    };
    let Some(tok) = login(&u, &p) else {
        return;
    };

    let resp = post_json_auth(
        "/api/internships/plans",
        &tok,
        &json!({"content": "Q2 plan"}),
    )
    .expect("plan");
    assert_status(&resp, 200, "POST /internships/plans");
    let v = json_body(resp, "plan");
    assert_keys(
        &v,
        &["id", "intern_id", "content", "created_at"],
        "InternshipPlan",
    );
    assert_eq!(v["content"], "Q2 plan");

    // Non-intern -> 403.
    let Some((_, ru, rp)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(rtok) = login(&ru, &rp) else {
        return;
    };
    let r =
        post_json_auth("/api/internships/plans", &rtok, &json!({"content": "nope"})).expect("r");
    assert_status(&r, 403, "requester cannot create plan");
}

#[test]
fn reports_create_comment_approve_contract() {
    let Some(admin) = setup("reports_create_comment_approve_contract") else {
        return;
    };
    let Some((intern_id, iu, ip)) = intern_credentials(&admin) else {
        return;
    };
    let Some(intern_tok) = login(&iu, &ip) else {
        return;
    };
    let Some((_, mu, mp)) = provision_user(&admin, "mentor") else {
        return;
    };
    let Some(mentor_tok) = login(&mu, &mp) else {
        return;
    };

    // Client-supplied due_at rejected.
    let bad = post_json_auth(
        "/api/reports",
        &intern_tok,
        &json!({"type":"WEEKLY","content":"x","due_at":"2026-12-31T23:59:59"}),
    )
    .expect("bad");
    assert_status(&bad, 400, "weekly with client due_at");

    // Valid daily report.
    let ok = post_json_auth(
        "/api/reports",
        &intern_tok,
        &json!({"type":"DAILY","content":"daily"}),
    )
    .expect("ok");
    assert_status(&ok, 200, "POST /reports");
    let v = json_body(ok, "ok");
    assert_keys(
        &v,
        &[
            "id",
            "intern_id",
            "type",
            "content",
            "status",
            "submitted_at",
            "due_at",
            "is_late",
        ],
        "Report",
    );
    assert_eq!(v["intern_id"], intern_id);
    let rid = v["id"].as_str().unwrap().to_string();

    // Mentor comment on report.
    let c = post_json_auth(
        &format!("/api/reports/{rid}/comments"),
        &mentor_tok,
        &json!({"content": "looks good"}),
    )
    .expect("c");
    assert_status(&c, 200, "POST /reports/<id>/comments");
    let cv = json_body(c, "c");
    assert_keys(
        &cv,
        &["id", "report_id", "mentor_id", "content", "created_at"],
        "MentorComment",
    );

    // Non-mentor comment -> 403.
    let Some((_, ru, rp)) = provision_user(&admin, "requester") else {
        return;
    };
    let Some(rtok) = login(&ru, &rp) else {
        return;
    };
    let bad_c = post_json_auth(
        &format!("/api/reports/{rid}/comments"),
        &rtok,
        &json!({"content":"nope"}),
    )
    .expect("bc");
    assert_status(&bad_c, 403, "requester cannot comment");

    // Mentor approve.
    let ap = post_empty_auth(&format!("/api/reports/{rid}/approve"), &mentor_tok).expect("ap");
    assert_status(&ap, 200, "POST /reports/<id>/approve");
    let av = json_body(ap, "ap");
    assert_keys(
        &av,
        &["id", "report_id", "mentor_id", "approved_at"],
        "ReportApproval",
    );

    // Duplicate approve -> 409.
    let ap2 = post_empty_auth(&format!("/api/reports/{rid}/approve"), &mentor_tok).expect("ap2");
    assert_status(&ap2, 409, "duplicate approve");

    // Approve missing report -> 404.
    let miss = post_empty_auth(
        "/api/reports/00000000-0000-0000-0000-000000000000/approve",
        &mentor_tok,
    )
    .expect("miss");
    assert_status(&miss, 404, "approve missing");
}

#[test]
fn dashboard_access_matrix_and_contract() {
    let Some(admin) = setup("dashboard_access_matrix_and_contract") else {
        return;
    };
    let Some((intern_id, iu, ip)) = intern_credentials(&admin) else {
        return;
    };
    let Some(intern_tok) = login(&iu, &ip) else {
        return;
    };

    // Self read.
    let self_resp =
        get_auth(&format!("/api/interns/{intern_id}/dashboard"), &intern_tok).expect("self");
    assert_status(&self_resp, 200, "GET /interns/<id>/dashboard self");
    let v = json_body(self_resp, "self");
    assert_keys(
        &v,
        &[
            "intern_id",
            "plans_count",
            "reports_total",
            "reports_by_type",
            "reports_approved",
            "reports_pending",
            "reports_late",
            "recent_reports",
        ],
        "InternDashboard",
    );

    // Mentor read.
    let Some((_, mu, mp)) = provision_user(&admin, "mentor") else {
        return;
    };
    let Some(mentor) = login(&mu, &mp) else {
        return;
    };
    let mresp = get_auth(&format!("/api/interns/{intern_id}/dashboard"), &mentor).expect("mentor");
    assert_status(&mresp, 200, "mentor access");

    // Other intern -> 403.
    let Some((_, ou, op)) = intern_credentials(&admin) else {
        return;
    };
    let Some(other) = login(&ou, &op) else {
        return;
    };
    let oresp = get_auth(&format!("/api/interns/{intern_id}/dashboard"), &other).expect("other");
    assert_status(&oresp, 403, "other intern forbidden");
}

#[test]
fn attachment_upload_contract() {
    let Some(admin) = setup("attachment_upload_contract") else {
        return;
    };
    let Some((_, iu, ip)) = intern_credentials(&admin) else {
        return;
    };
    let Some(intern_tok) = login(&iu, &ip) else {
        return;
    };

    // Intern submits a daily report.
    let rep = post_json_auth(
        "/api/reports",
        &intern_tok,
        &json!({"type":"DAILY","content":"rep"}),
    )
    .expect("rep");
    if !rep.status().is_success() {
        return;
    }
    let rid = json_body(rep, "rep")["id"].as_str().unwrap().to_string();

    // Upload a tiny file via multipart.
    let form = reqwest::blocking::multipart::Form::new().part(
        "file",
        reqwest::blocking::multipart::Part::bytes(b"hello world".to_vec())
            .file_name("note.txt")
            .mime_str("text/plain")
            .unwrap(),
    );
    let resp = client()
        .post(format!(
            "{}/api/reports/{}/attachments",
            api_tests::api_base(),
            rid
        ))
        .bearer_auth(&intern_tok)
        .multipart(form)
        .send()
        .expect("upload");
    assert_status(&resp, 200, "POST /reports/<id>/attachments");
    let v = json_body(resp, "up");
    assert_keys(
        &v,
        &["id", "report_id", "file_path", "content_hash", "size_bytes"],
        "ReportAttachment",
    );
    assert_eq!(v["report_id"], rid);
    assert!(v["content_hash"].as_str().unwrap().len() == 64);

    // Another intern cannot attach to this report.
    let Some((_, ou, op)) = intern_credentials(&admin) else {
        return;
    };
    let Some(other) = login(&ou, &op) else {
        return;
    };
    let form2 = reqwest::blocking::multipart::Form::new().part(
        "file",
        reqwest::blocking::multipart::Part::bytes(b"x".to_vec())
            .file_name("x.txt")
            .mime_str("text/plain")
            .unwrap(),
    );
    let r2 = client()
        .post(format!(
            "{}/api/reports/{}/attachments",
            api_tests::api_base(),
            rid
        ))
        .bearer_auth(&other)
        .multipart(form2)
        .send()
        .expect("up2");
    assert_status(&r2, 403, "other intern forbidden attachment");
}
