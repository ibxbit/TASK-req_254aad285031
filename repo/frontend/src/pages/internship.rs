// Internship workspace — interns submit reports + attachments; mentors
// (and administrators) leave comments and approve reports. Role-gated so a
// single route serves all three audiences.

use dioxus::prelude::*;
use shared::{InternDashboard, ReportType, Role};
use wasm_bindgen::JsCast;

use crate::api::internships;
use crate::auth::use_auth;

const ATTACH_INPUT_ID: &str = "intern-attachment-input";

fn read_selected_file(input_id: &str) -> Option<web_sys::File> {
    let doc = web_sys::window()?.document()?;
    let el = doc.get_element_by_id(input_id)?;
    let input: web_sys::HtmlInputElement = el.dyn_into().ok()?;
    input.files()?.get(0)
}

#[component]
pub fn Internship() -> Element {
    let auth = use_auth();
    let (user_id, role) = {
        let s = auth.0.read();
        let uid = s.user.as_ref().map(|u| u.id.clone()).unwrap_or_default();
        let r = s.user.as_ref().map(|u| u.role).unwrap_or(Role::Requester);
        (uid, r)
    };
    let is_mentor = matches!(role, Role::Mentor | Role::Administrator);

    // Dashboard is loaded for the logged-in user (or for a looked-up intern
    // when the viewer is a mentor/admin).
    let mut viewed_intern = use_signal(|| user_id.clone());
    let mut dash = use_signal(|| None::<InternDashboard>);
    let mut error = use_signal(|| None::<String>);
    let mut info = use_signal(|| None::<String>);

    let mut report_type = use_signal(|| "WEEKLY".to_string());
    let mut content = use_signal(String::new);

    let mut attachment_report_id = use_signal(String::new);
    let mut comment_report_id = use_signal(String::new);
    let mut comment_text = use_signal(String::new);
    let mut approve_report_id = use_signal(String::new);

    let load_dash = move || {
        let uid = viewed_intern();
        spawn(async move {
            match internships::dashboard(&uid).await {
                Ok(d) => dash.set(Some(d)),
                Err(e) => error.set(Some(e)),
            }
        });
    };

    use_future(move || async move {
        let uid = viewed_intern();
        match internships::dashboard(&uid).await {
            Ok(d) => dash.set(Some(d)),
            Err(e) => error.set(Some(e)),
        }
    });

    let submit = move |_| {
        let rtype = match report_type().as_str() {
            "DAILY" => ReportType::Daily,
            "MONTHLY" => ReportType::Monthly,
            _ => ReportType::Weekly,
        };
        let text = content();
        if text.trim().is_empty() {
            return;
        }
        spawn(async move {
            match internships::submit_report(rtype, text).await {
                Ok(_) => {
                    content.set(String::new());
                    info.set(Some("Report submitted".into()));
                    load_dash();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let upload_attachment = move |_| {
        let rid = attachment_report_id();
        if rid.trim().is_empty() {
            error.set(Some("Enter a report id to attach to".into()));
            return;
        }
        let Some(file) = read_selected_file(ATTACH_INPUT_ID) else {
            error.set(Some("No file selected".into()));
            return;
        };
        spawn(async move {
            match internships::upload_attachment(&rid, &file).await {
                Ok(a) => info.set(Some(format!("Uploaded attachment {}", a.id))),
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let submit_comment = move |_| {
        let rid = comment_report_id();
        let txt = comment_text();
        if rid.trim().is_empty() || txt.trim().is_empty() {
            return;
        }
        spawn(async move {
            match internships::add_mentor_comment(&rid, txt).await {
                Ok(_) => {
                    comment_text.set(String::new());
                    info.set(Some("Comment posted".into()));
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let submit_approve = move |_| {
        let rid = approve_report_id();
        if rid.trim().is_empty() {
            return;
        }
        spawn(async move {
            match internships::approve_report(&rid).await {
                Ok(_) => info.set(Some("Report approved".into())),
                Err(e) => error.set(Some(e)),
            }
        });
    };

    rsx! {
        h2 { "Internship dashboard" }
        if let Some(m) = info() { p { class: "muted", "{m}" } }
        if let Some(m) = error() { p { class: "err", "{m}" } }

        if is_mentor {
            div { class: "card",
                h3 { "View intern" }
                div { class: "row",
                    label { "Intern id" }
                    input {
                        value: "{viewed_intern}",
                        oninput: move |e| viewed_intern.set(e.value()),
                    }
                    button { onclick: move |_| load_dash(), "Load" }
                }
            }
        }

        if let Some(d) = dash() {
            div { class: "card",
                p { "Plans: {d.plans_count}" }
                p {
                    "Reports — total: {d.reports_total} · approved: {d.reports_approved} · pending: {d.reports_pending} · late: {d.reports_late}"
                }
                p {
                    "By type — daily: {d.reports_by_type.daily} · weekly: {d.reports_by_type.weekly} · monthly: {d.reports_by_type.monthly}"
                }
            }
            h3 { "Recent reports" }
            if d.recent_reports.is_empty() {
                p { class: "muted", "No reports yet." }
            }
            for r in d.recent_reports.into_iter() {
                div { key: "{r.id}", class: "card",
                    strong { "{r.report_type:?}" }
                    " · {r.status:?} · late={r.is_late}"
                    p { "{r.content}" }
                    p { class: "muted",
                        "id: {r.id} · Due: {r.due_at} · Submitted: {r.submitted_at}"
                    }
                }
            }
        } else {
            p { class: "muted", "Loading..." }
        }

        if role == Role::Intern {
            h3 { "Submit report" }
            div { class: "card",
                div { class: "row",
                    label { "Type" }
                    select {
                        value: "{report_type}",
                        onchange: move |e| report_type.set(e.value()),
                        option { value: "DAILY", "Daily" }
                        option { value: "WEEKLY", "Weekly" }
                        option { value: "MONTHLY", "Monthly" }
                    }
                }
                div { class: "row",
                    textarea {
                        placeholder: "Content",
                        rows: "4",
                        value: "{content}",
                        oninput: move |e| content.set(e.value()),
                    }
                }
                button { onclick: submit, "Submit" }
            }

            h3 { "Attach file to a report" }
            div { class: "card",
                div { class: "row",
                    label { "Report id" }
                    input {
                        value: "{attachment_report_id}",
                        oninput: move |e| attachment_report_id.set(e.value()),
                    }
                }
                div { class: "row",
                    input { id: "{ATTACH_INPUT_ID}", r#type: "file" }
                    button { onclick: upload_attachment, "Upload attachment" }
                }
            }
        }

        if is_mentor {
            h3 { "Mentor actions" }
            div { class: "card",
                h4 { "Comment on report" }
                div { class: "row",
                    label { "Report id" }
                    input {
                        value: "{comment_report_id}",
                        oninput: move |e| comment_report_id.set(e.value()),
                    }
                }
                div { class: "row",
                    textarea {
                        placeholder: "Comment",
                        rows: "3",
                        value: "{comment_text}",
                        oninput: move |e| comment_text.set(e.value()),
                    }
                }
                button { onclick: submit_comment, "Post comment" }
            }
            div { class: "card",
                h4 { "Approve report" }
                div { class: "row",
                    label { "Report id" }
                    input {
                        value: "{approve_report_id}",
                        oninput: move |e| approve_report_id.set(e.value()),
                    }
                    button { onclick: submit_approve, "Approve" }
                }
            }
        }
    }
}
