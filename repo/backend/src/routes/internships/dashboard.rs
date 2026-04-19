use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{InternDashboard, Report, ReportsByType, Role};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::auth::guard::AuthUser;
use crate::routes::internships::reports::row_to_report;

#[get("/interns/<id>/dashboard")]
pub async fn get(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
) -> Result<Json<InternDashboard>, Status> {
    let intern_id = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    // Access: self, mentor, admin.
    if intern_id != user.id && !matches!(user.role, Role::Mentor | Role::Administrator) {
        return Err(Status::Forbidden);
    }

    let plans: (i64,) = sqlx::query_as("SELECT CAST(COUNT(*) AS SIGNED) FROM internship_plans WHERE intern_id = ?")
        .bind(&intern_id.as_bytes()[..])
        .fetch_one(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;

    // Aggregated counts in a single scan.
    // approved_count = rows with an approval joined in; pending = rest.
    // CAST to SIGNED to avoid DECIMAL return type from SUM().
    let agg: (i64, i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT \
            CAST(COUNT(*) AS SIGNED), \
            CAST(COALESCE(SUM(CASE WHEN r.report_type = 'daily'   THEN 1 ELSE 0 END), 0) AS SIGNED), \
            CAST(COALESCE(SUM(CASE WHEN r.report_type = 'weekly'  THEN 1 ELSE 0 END), 0) AS SIGNED), \
            CAST(COALESCE(SUM(CASE WHEN r.report_type = 'monthly' THEN 1 ELSE 0 END), 0) AS SIGNED), \
            CAST(COALESCE(SUM(CASE WHEN ap.id IS NOT NULL THEN 1 ELSE 0 END), 0) AS SIGNED), \
            CAST(COALESCE(SUM(CASE WHEN ap.id IS NULL     THEN 1 ELSE 0 END), 0) AS SIGNED), \
            CAST(COALESCE(SUM(CASE WHEN r.is_late = 1     THEN 1 ELSE 0 END), 0) AS SIGNED) \
         FROM reports r \
         LEFT JOIN report_approvals ap ON ap.report_id = r.id \
         WHERE r.intern_id = ?",
    )
    .bind(&intern_id.as_bytes()[..])
    .fetch_one(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;

    let recent_rows: Vec<(
        Vec<u8>,
        Vec<u8>,
        String,
        String,
        String,
        chrono::NaiveDateTime,
        chrono::NaiveDateTime,
        i8,
    )> = sqlx::query_as(
        "SELECT id, intern_id, report_type, content, status, submitted_at, due_at, is_late \
         FROM reports WHERE intern_id = ? \
         ORDER BY submitted_at DESC LIMIT 10",
    )
    .bind(&intern_id.as_bytes()[..])
    .fetch_all(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;

    let recent: Vec<Report> = recent_rows.into_iter().filter_map(row_to_report).collect();

    Ok(Json(InternDashboard {
        intern_id: intern_id.to_string(),
        plans_count: plans.0,
        reports_total: agg.0,
        reports_by_type: ReportsByType {
            daily: agg.1,
            weekly: agg.2,
            monthly: agg.3,
        },
        reports_approved: agg.4,
        reports_pending: agg.5,
        reports_late: agg.6,
        recent_reports: recent,
    }))
}
