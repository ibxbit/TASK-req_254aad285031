use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{CreateWorkOrderRequest, Role, WorkOrder, WorkOrderStatus};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::auth::guard::AuthUser;

#[post("/work-orders", format = "json", data = "<req>")]
pub async fn create(
    pool: &State<MySqlPool>,
    user: AuthUser,
    req: Json<CreateWorkOrderRequest>,
) -> Result<Json<WorkOrder>, Status> {
    user.require_any(&[Role::Requester, Role::Administrator])?;
    let sid = Uuid::parse_str(&req.service_id).map_err(|_| Status::BadRequest)?;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO work_orders (id, requester_id, service_id, status) \
         VALUES (?, ?, ?, 'pending')",
    )
    .bind(&id.as_bytes()[..])
    .bind(&user.id.as_bytes()[..])
    .bind(&sid.as_bytes()[..])
    .execute(pool.inner())
    .await
    .map_err(|_| Status::BadRequest)?;
    Ok(Json(WorkOrder {
        id: id.to_string(),
        requester_id: user.id.to_string(),
        service_id: sid.to_string(),
        status: WorkOrderStatus::Pending,
        completed_at: None,
    }))
}

#[get("/work-orders/<id>")]
pub async fn get(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
) -> Result<Json<WorkOrder>, Status> {
    let wid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let row: Option<(
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        String,
        Option<chrono::NaiveDateTime>,
    )> = sqlx::query_as(
        "SELECT id, requester_id, service_id, status, completed_at \
             FROM work_orders WHERE id = ?",
    )
    .bind(&wid.as_bytes()[..])
    .fetch_optional(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;
    let Some((wid_b, req_b, svc_b, status, completed_at)) = row else {
        return Err(Status::NotFound);
    };
    let requester = Uuid::from_slice(&req_b).map_err(|_| Status::InternalServerError)?;
    // Requester sees own; admin/service_manager see all.
    if requester != user.id && !matches!(user.role, Role::Administrator | Role::ServiceManager) {
        return Err(Status::Forbidden);
    }
    Ok(Json(WorkOrder {
        id: Uuid::from_slice(&wid_b)
            .map_err(|_| Status::InternalServerError)?
            .to_string(),
        requester_id: requester.to_string(),
        service_id: Uuid::from_slice(&svc_b)
            .map_err(|_| Status::InternalServerError)?
            .to_string(),
        status: WorkOrderStatus::from_str(&status).ok_or(Status::InternalServerError)?,
        completed_at,
    }))
}

#[post("/work-orders/<id>/complete")]
pub async fn complete(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
) -> Result<Json<WorkOrder>, Status> {
    user.require_any(&[Role::Administrator, Role::ServiceManager])?;
    let wid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    let row: Option<(Vec<u8>, Vec<u8>, String)> =
        sqlx::query_as("SELECT requester_id, service_id, status FROM work_orders WHERE id = ?")
            .bind(&wid.as_bytes()[..])
            .fetch_optional(pool.inner())
            .await
            .map_err(|_| Status::InternalServerError)?;
    let Some((req_b, svc_b, status)) = row else {
        return Err(Status::NotFound);
    };
    if status == "completed" {
        return Err(Status::Conflict);
    }
    if status == "cancelled" {
        return Err(Status::BadRequest);
    }

    let now = chrono::Utc::now().naive_utc();
    sqlx::query("UPDATE work_orders SET status = 'completed', completed_at = ? WHERE id = ?")
        .bind(now)
        .bind(&wid.as_bytes()[..])
        .execute(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(WorkOrder {
        id: wid.to_string(),
        requester_id: Uuid::from_slice(&req_b)
            .map_err(|_| Status::InternalServerError)?
            .to_string(),
        service_id: Uuid::from_slice(&svc_b)
            .map_err(|_| Status::InternalServerError)?
            .to_string(),
        status: WorkOrderStatus::Completed,
        completed_at: Some(now),
    }))
}
