use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{AuditVerifyReport, EventLog, Role};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::audit;
use crate::auth::guard::AuthUser;

#[get("/audit/verify")]
pub async fn verify(
    pool: &State<MySqlPool>,
    user: AuthUser,
) -> Result<Json<AuditVerifyReport>, Status> {
    user.require_role(Role::Administrator)?;
    let report = audit::verify_chain(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    Ok(Json(report))
}

#[get("/audit/<entity_type>/<entity_id>")]
pub async fn list_for_entity(
    pool: &State<MySqlPool>,
    user: AuthUser,
    entity_type: &str,
    entity_id: &str,
) -> Result<Json<Vec<EventLog>>, Status> {
    user.require_role(Role::Administrator)?;
    let eid = Uuid::parse_str(entity_id).map_err(|_| Status::BadRequest)?;

    let rows: Vec<(
        Vec<u8>,
        i64,
        String,
        Vec<u8>,
        String,
        String,
        String,
        String,
        chrono::NaiveDateTime,
    )> = sqlx::query_as(
        "SELECT id, sequence, entity_type, entity_id, action, payload, \
                prev_hash, hash, created_at \
         FROM event_log \
         WHERE entity_type = ? AND entity_id = ? \
         ORDER BY sequence",
    )
    .bind(entity_type)
    .bind(&eid.as_bytes()[..])
    .fetch_all(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;

    let events = rows
        .into_iter()
        .filter_map(
            |(id_b, seq, etype, eid_b, action, payload, prev_hash, hash, ts)| {
                Some(EventLog {
                    id: Uuid::from_slice(&id_b).ok()?.to_string(),
                    sequence: seq,
                    entity_type: etype,
                    entity_id: Uuid::from_slice(&eid_b).ok()?.to_string(),
                    action,
                    payload,
                    prev_hash,
                    hash,
                    created_at: ts,
                })
            },
        )
        .collect();
    Ok(Json(events))
}
