use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{AvailabilityWindow, CreateAvailabilityRequest, Role};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::auth::guard::AuthUser;

const MANAGEMENT_ROLES: [Role; 2] = [Role::Administrator, Role::ServiceManager];

#[post("/services/<id>/availability", format = "json", data = "<req>")]
pub async fn create(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<CreateAvailabilityRequest>,
) -> Result<Json<AvailabilityWindow>, Status> {
    user.require_any(&MANAGEMENT_ROLES)?;
    if req.end_time <= req.start_time {
        return Err(Status::BadRequest);
    }
    let sid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let aid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO availability (id, service_id, start_time, end_time) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(&aid.as_bytes()[..])
    .bind(&sid.as_bytes()[..])
    .bind(req.start_time)
    .bind(req.end_time)
    .execute(pool.inner())
    .await
    .map_err(|_| Status::BadRequest)?;
    Ok(Json(AvailabilityWindow {
        id: aid.to_string(),
        service_id: sid.to_string(),
        start_time: req.start_time,
        end_time: req.end_time,
    }))
}
