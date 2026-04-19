use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{
    CreateMentorCommentRequest, CreateReportRequest, MentorComment, Report, ReportApproval,
    ReportStatus, ReportType, Role,
};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::audit;
use crate::auth::guard::AuthUser;
use crate::config::PolicyConfig;
use crate::internships::{client_can_override_due, compute_due_at, evaluate_lateness};
use crate::logging;

const ENTITY: &str = "report";

// ---------- Submit a report ----------

#[post("/reports", format = "json", data = "<req>")]
pub async fn create(
    pool: &State<MySqlPool>,
    policy: &State<PolicyConfig>,
    user: AuthUser,
    req: Json<CreateReportRequest>,
) -> Result<Json<Report>, Status> {
    user.require_role(Role::Intern)?;

    // Weekly/monthly/daily deadlines are server-authoritative. A client-
    // supplied `due_at` is never allowed to bypass policy; reject with 400.
    if req.due_at.is_some() && !client_can_override_due(req.report_type) {
        logging::validation_failed(
            "POST /reports",
            "client attempted to override server-computed due_at",
        );
        return Err(Status::BadRequest);
    }

    let now = chrono::Utc::now().naive_utc();
    let due_at = compute_due_at(req.report_type, now, policy);

    let late = match evaluate_lateness(now, due_at, policy.late_grace_hours) {
        Ok(flag) => flag,
        Err(_) => {
            logging::validation_failed("POST /reports", "past grace window");
            return Err(Status::Gone);
        }
    };
    let status = if late {
        ReportStatus::Late
    } else {
        ReportStatus::OnTime
    };

    let id = Uuid::new_v4();

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    sqlx::query(
        "INSERT INTO reports \
         (id, intern_id, report_type, content, status, submitted_at, due_at, is_late) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id.as_bytes()[..])
    .bind(&user.id.as_bytes()[..])
    .bind(req.report_type.as_str())
    .bind(&req.content)
    .bind(status.as_str())
    .bind(now)
    .bind(due_at)
    .bind(late as i8)
    .execute(&mut *tx)
    .await
    .map_err(|_| Status::InternalServerError)?;

    let payload = serde_json::json!({
        "action": "create",
        "id": id.to_string(),
        "intern_id": user.id.to_string(),
        "type": req.report_type.as_str(),
        "content": req.content,
        "status": status.as_str(),
        "submitted_at": now.to_string(),
        "due_at": due_at.to_string(),
        "is_late": late,
    });
    audit::record_event_tx(&mut tx, ENTITY, id, "create", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Json(Report {
        id: id.to_string(),
        intern_id: user.id.to_string(),
        report_type: req.report_type,
        content: req.content.clone(),
        status,
        submitted_at: now,
        due_at,
        is_late: late,
    }))
}

// ---------- Mentor comment ----------

#[post("/reports/<id>/comments", format = "json", data = "<req>")]
pub async fn add_comment(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<CreateMentorCommentRequest>,
) -> Result<Json<MentorComment>, Status> {
    user.require_any(&[Role::Mentor, Role::Administrator])?;
    let rid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    let exists: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM reports WHERE id = ?")
        .bind(&rid.as_bytes()[..])
        .fetch_optional(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    if exists.is_none() {
        return Err(Status::NotFound);
    }

    let cid = Uuid::new_v4();
    let now = chrono::Utc::now().naive_utc();

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    sqlx::query(
        "INSERT INTO mentor_comments (id, report_id, mentor_id, content, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&cid.as_bytes()[..])
    .bind(&rid.as_bytes()[..])
    .bind(&user.id.as_bytes()[..])
    .bind(&req.content)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| Status::InternalServerError)?;

    let payload = serde_json::json!({
        "action": "comment",
        "id": cid.to_string(),
        "report_id": rid.to_string(),
        "mentor_id": user.id.to_string(),
        "content": req.content,
        "created_at": now.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, rid, "comment", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Json(MentorComment {
        id: cid.to_string(),
        report_id: rid.to_string(),
        mentor_id: user.id.to_string(),
        content: req.content.clone(),
        created_at: now,
    }))
}

// ---------- Mentor approval ----------

#[post("/reports/<id>/approve")]
pub async fn approve(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
) -> Result<Json<ReportApproval>, Status> {
    user.require_any(&[Role::Mentor, Role::Administrator])?;
    let rid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    let exists: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM reports WHERE id = ?")
        .bind(&rid.as_bytes()[..])
        .fetch_optional(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    if exists.is_none() {
        return Err(Status::NotFound);
    }

    let aid = Uuid::new_v4();
    let now = chrono::Utc::now().naive_utc();

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let res = sqlx::query(
        "INSERT INTO report_approvals (id, report_id, mentor_id, approved_at) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(&aid.as_bytes()[..])
    .bind(&rid.as_bytes()[..])
    .bind(&user.id.as_bytes()[..])
    .bind(now)
    .execute(&mut *tx)
    .await;

    if let Err(sqlx::Error::Database(e)) = &res {
        if e.is_unique_violation() {
            return Err(Status::Conflict);
        }
    }
    res.map_err(|_| Status::InternalServerError)?;

    let payload = serde_json::json!({
        "action": "approve",
        "id": aid.to_string(),
        "report_id": rid.to_string(),
        "mentor_id": user.id.to_string(),
        "approved_at": now.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, rid, "approve", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Json(ReportApproval {
        id: aid.to_string(),
        report_id: rid.to_string(),
        mentor_id: user.id.to_string(),
        approved_at: now,
    }))
}

// Helper used by dashboard.
pub(crate) fn row_to_report(
    row: (
        Vec<u8>,
        Vec<u8>,
        String,
        String,
        String,
        chrono::NaiveDateTime,
        chrono::NaiveDateTime,
        i8,
    ),
) -> Option<Report> {
    let (id, intern_id, rtype, content, status, submitted_at, due_at, is_late) = row;
    Some(Report {
        id: Uuid::from_slice(&id).ok()?.to_string(),
        intern_id: Uuid::from_slice(&intern_id).ok()?.to_string(),
        report_type: ReportType::from_str(&rtype)?,
        content,
        status: ReportStatus::from_str(&status)?,
        submitted_at,
        due_at,
        is_late: is_late != 0,
    })
}
