use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use serde::Deserialize;
use shared::{CreateWarehouseZoneRequest, Role, WarehouseZone, WarehouseZoneChangeLog};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::audit;
use crate::auth::guard::AuthUser;
use crate::warehouse_audit::log_zone_change;

const MGMT: [Role; 2] = [Role::Administrator, Role::WarehouseManager];
const ENTITY: &str = "warehouse_zone";

#[post("/warehouse-zones", format = "json", data = "<req>")]
pub async fn create(
    pool: &State<MySqlPool>,
    user: AuthUser,
    req: Json<CreateWarehouseZoneRequest>,
) -> Result<Json<WarehouseZone>, Status> {
    user.require_any(&MGMT)?;
    let wid = Uuid::parse_str(&req.warehouse_id).map_err(|_| Status::BadRequest)?;
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(Status::BadRequest);
    }
    let id = Uuid::new_v4();

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let res = sqlx::query("INSERT INTO warehouse_zones (id, warehouse_id, name) VALUES (?, ?, ?)")
        .bind(&id.as_bytes()[..])
        .bind(&wid.as_bytes()[..])
        .bind(name)
        .execute(&mut *tx)
        .await;
    if let Err(sqlx::Error::Database(e)) = &res {
        if e.is_unique_violation() {
            return Err(Status::Conflict);
        }
    }
    res.map_err(|_| Status::BadRequest)?;

    log_zone_change(
        &mut tx,
        id,
        user.id,
        "create",
        None,
        Some(format!("warehouse_id={},name={}", wid, name)),
    )
    .await
    .map_err(|_| Status::InternalServerError)?;

    let payload = serde_json::json!({
        "action": "create",
        "id": id.to_string(),
        "warehouse_id": wid.to_string(),
        "name": name,
        "by": user.id.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, id, "create", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Json(WarehouseZone {
        id: id.to_string(),
        warehouse_id: wid.to_string(),
        name: name.to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateZoneRequest {
    pub name: String,
}

#[patch("/warehouse-zones/<id>", format = "json", data = "<req>")]
pub async fn rename(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<UpdateZoneRequest>,
) -> Result<Status, Status> {
    user.require_any(&MGMT)?;
    let zid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let new_name = req.name.trim();
    if new_name.is_empty() || new_name.chars().count() > 100 {
        return Err(Status::BadRequest);
    }

    let row: Option<(String,)> = sqlx::query_as("SELECT name FROM warehouse_zones WHERE id = ?")
        .bind(&zid.as_bytes()[..])
        .fetch_optional(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    let Some((old_name,)) = row else {
        return Err(Status::NotFound);
    };
    if old_name == new_name {
        return Ok(Status::NoContent);
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    sqlx::query("UPDATE warehouse_zones SET name = ? WHERE id = ?")
        .bind(new_name)
        .bind(&zid.as_bytes()[..])
        .execute(&mut *tx)
        .await
        .map_err(|_| Status::InternalServerError)?;

    log_zone_change(
        &mut tx,
        zid,
        user.id,
        "rename",
        Some(old_name.clone()),
        Some(new_name.to_string()),
    )
    .await
    .map_err(|_| Status::InternalServerError)?;

    let payload = serde_json::json!({
        "action": "rename",
        "id": zid.to_string(),
        "old_name": old_name,
        "new_name": new_name,
        "by": user.id.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, zid, "rename", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;
    Ok(Status::NoContent)
}

#[delete("/warehouse-zones/<id>")]
pub async fn delete(pool: &State<MySqlPool>, user: AuthUser, id: &str) -> Result<Status, Status> {
    user.require_any(&MGMT)?;
    let zid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let row: Option<(String,)> = sqlx::query_as("SELECT name FROM warehouse_zones WHERE id = ?")
        .bind(&zid.as_bytes()[..])
        .fetch_optional(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    let Some((old_name,)) = row else {
        return Err(Status::NotFound);
    };

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    log_zone_change(
        &mut tx,
        zid,
        user.id,
        "delete",
        Some(old_name.clone()),
        None,
    )
    .await
    .map_err(|_| Status::InternalServerError)?;

    let payload = serde_json::json!({
        "action": "delete",
        "id": zid.to_string(),
        "name": old_name,
        "by": user.id.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, zid, "delete", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    sqlx::query("DELETE FROM warehouse_zones WHERE id = ?")
        .bind(&zid.as_bytes()[..])
        .execute(&mut *tx)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;
    Ok(Status::NoContent)
}

#[get("/warehouse-zones/<id>/history")]
pub async fn history(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
) -> Result<Json<Vec<WarehouseZoneChangeLog>>, Status> {
    user.require_any(&MGMT)?;
    let zid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let rows: Vec<(
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        String,
        Option<String>,
        Option<String>,
        chrono::NaiveDateTime,
    )> = sqlx::query_as(
        "SELECT id, zone_id, changed_by, change_type, old_value, new_value, created_at \
         FROM warehouse_zone_change_log WHERE zone_id = ? ORDER BY created_at DESC",
    )
    .bind(&zid.as_bytes()[..])
    .fetch_all(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;

    Ok(Json(
        rows.into_iter()
            .filter_map(|(lid, z, u, ct, ov, nv, ts)| {
                Some(WarehouseZoneChangeLog {
                    id: Uuid::from_slice(&lid).ok()?.to_string(),
                    zone_id: Uuid::from_slice(&z).ok()?.to_string(),
                    changed_by: Uuid::from_slice(&u).ok()?.to_string(),
                    change_type: ct,
                    old_value: ov,
                    new_value: nv,
                    created_at: ts,
                })
            })
            .collect(),
    ))
}
