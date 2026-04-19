use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{CreateInternshipPlanRequest, InternshipPlan, Role};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::auth::guard::AuthUser;

#[post("/internships/plans", format = "json", data = "<req>")]
pub async fn create(
    pool: &State<MySqlPool>,
    user: AuthUser,
    req: Json<CreateInternshipPlanRequest>,
) -> Result<Json<InternshipPlan>, Status> {
    user.require_role(Role::Intern)?;
    let id = Uuid::new_v4();
    let now = chrono::Utc::now().naive_utc();
    sqlx::query(
        "INSERT INTO internship_plans (id, intern_id, content, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&id.as_bytes()[..])
    .bind(&user.id.as_bytes()[..])
    .bind(&req.content)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;
    Ok(Json(InternshipPlan {
        id: id.to_string(),
        intern_id: user.id.to_string(),
        content: req.content.clone(),
        created_at: now,
    }))
}
