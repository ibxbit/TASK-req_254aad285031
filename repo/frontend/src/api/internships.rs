use shared::{
    CreateMentorCommentRequest, CreateReportRequest, InternDashboard, MentorComment, Report,
    ReportApproval, ReportAttachment, ReportType,
};

use super::client;

pub async fn dashboard(intern_id: &str) -> Result<InternDashboard, String> {
    client::get_json(&format!("/api/interns/{}/dashboard", intern_id)).await
}

pub async fn submit_report(report_type: ReportType, content: String) -> Result<Report, String> {
    let body = CreateReportRequest {
        report_type,
        content,
        due_at: None,
    };
    client::post_json("/api/reports", &body).await
}

pub async fn add_mentor_comment(report_id: &str, content: String) -> Result<MentorComment, String> {
    let body = CreateMentorCommentRequest { content };
    client::post_json(&format!("/api/reports/{report_id}/comments"), &body).await
}

pub async fn approve_report(report_id: &str) -> Result<ReportApproval, String> {
    // Approve endpoint takes no body. Issue a POST and parse the response.
    use gloo_storage::Storage as _;
    let state: Option<crate::auth::AuthState> = gloo_storage::LocalStorage::get("fsh_auth").ok();
    let mut req = gloo_net::http::Request::post(&format!("/api/reports/{report_id}/approve"));
    if let Some(s) = state {
        if let Some(t) = s.token {
            req = req.header("Authorization", &format!("Bearer {t}"));
        }
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn upload_attachment(
    report_id: &str,
    file: &web_sys::File,
) -> Result<ReportAttachment, String> {
    let form = web_sys::FormData::new().map_err(|_| "FormData unavailable".to_string())?;
    form.append_with_blob("file", file)
        .map_err(|_| "FormData append failed".to_string())?;
    client::upload_multipart(&format!("/api/reports/{report_id}/attachments"), &form).await
}
