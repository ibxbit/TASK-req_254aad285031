use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use serde::Deserialize;
use shared::{
    Bin, CreateWarehouseRequest, Role, Warehouse, WarehouseChangeLog, WarehouseTreeNode,
    WarehouseZoneNode,
};
use sqlx::MySqlPool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::audit;
use crate::auth::guard::AuthUser;
use crate::warehouse_audit::log_warehouse_change;

const MGMT: [Role; 2] = [Role::Administrator, Role::WarehouseManager];
const ENTITY: &str = "warehouse";

#[post("/warehouses", format = "json", data = "<req>")]
pub async fn create(
    pool: &State<MySqlPool>,
    user: AuthUser,
    req: Json<CreateWarehouseRequest>,
) -> Result<Json<Warehouse>, Status> {
    user.require_any(&MGMT)?;
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(Status::BadRequest);
    }

    let id = Uuid::new_v4();
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let res = sqlx::query("INSERT INTO warehouses (id, name) VALUES (?, ?)")
        .bind(&id.as_bytes()[..])
        .bind(name)
        .execute(&mut *tx)
        .await;
    if let Err(sqlx::Error::Database(e)) = &res {
        if e.is_unique_violation() {
            return Err(Status::Conflict);
        }
    }
    res.map_err(|_| Status::BadRequest)?;

    log_warehouse_change(&mut tx, id, user.id, "create", None, Some(name.to_string()))
        .await
        .map_err(|_| Status::InternalServerError)?;

    let payload = serde_json::json!({
        "action": "create",
        "id": id.to_string(),
        "name": name,
        "by": user.id.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, id, "create", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Json(Warehouse {
        id: id.to_string(),
        name: name.to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateWarehouseRequest {
    pub name: String,
}

#[patch("/warehouses/<id>", format = "json", data = "<req>")]
pub async fn rename(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<UpdateWarehouseRequest>,
) -> Result<Status, Status> {
    user.require_any(&MGMT)?;
    let wid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let new_name = req.name.trim();
    if new_name.is_empty() || new_name.chars().count() > 100 {
        return Err(Status::BadRequest);
    }

    let row: Option<(String,)> = sqlx::query_as("SELECT name FROM warehouses WHERE id = ?")
        .bind(&wid.as_bytes()[..])
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
    sqlx::query("UPDATE warehouses SET name = ? WHERE id = ?")
        .bind(new_name)
        .bind(&wid.as_bytes()[..])
        .execute(&mut *tx)
        .await
        .map_err(|_| Status::InternalServerError)?;

    log_warehouse_change(
        &mut tx,
        wid,
        user.id,
        "rename",
        Some(old_name.clone()),
        Some(new_name.to_string()),
    )
    .await
    .map_err(|_| Status::InternalServerError)?;

    let payload = serde_json::json!({
        "action": "rename",
        "id": wid.to_string(),
        "old_name": old_name,
        "new_name": new_name,
        "by": user.id.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, wid, "rename", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;
    Ok(Status::NoContent)
}

#[delete("/warehouses/<id>")]
pub async fn delete(pool: &State<MySqlPool>, user: AuthUser, id: &str) -> Result<Status, Status> {
    user.require_any(&MGMT)?;
    let wid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    let row: Option<(String,)> = sqlx::query_as("SELECT name FROM warehouses WHERE id = ?")
        .bind(&wid.as_bytes()[..])
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

    // Record the change BEFORE deleting the row so the FK on
    // warehouse_change_log.warehouse_id still points at an existing row.
    log_warehouse_change(
        &mut tx,
        wid,
        user.id,
        "delete",
        Some(old_name.clone()),
        None,
    )
    .await
    .map_err(|_| Status::InternalServerError)?;

    let payload = serde_json::json!({
        "action": "delete",
        "id": wid.to_string(),
        "name": old_name,
        "by": user.id.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, wid, "delete", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    sqlx::query("DELETE FROM warehouses WHERE id = ?")
        .bind(&wid.as_bytes()[..])
        .execute(&mut *tx)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;
    Ok(Status::NoContent)
}

#[get("/warehouses/<id>/history")]
pub async fn history(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
) -> Result<Json<Vec<WarehouseChangeLog>>, Status> {
    user.require_any(&MGMT)?;
    let wid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let rows: Vec<(
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        String,
        Option<String>,
        Option<String>,
        chrono::NaiveDateTime,
    )> = sqlx::query_as(
        "SELECT id, warehouse_id, changed_by, change_type, old_value, new_value, created_at \
         FROM warehouse_change_log WHERE warehouse_id = ? ORDER BY created_at DESC",
    )
    .bind(&wid.as_bytes()[..])
    .fetch_all(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;

    Ok(Json(
        rows.into_iter()
            .filter_map(|(lid, w, u, ct, ov, nv, ts)| {
                Some(WarehouseChangeLog {
                    id: Uuid::from_slice(&lid).ok()?.to_string(),
                    warehouse_id: Uuid::from_slice(&w).ok()?.to_string(),
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

#[get("/warehouses/tree")]
pub async fn tree(
    pool: &State<MySqlPool>,
    _user: AuthUser,
) -> Result<Json<Vec<WarehouseTreeNode>>, Status> {
    let warehouses: Vec<(Vec<u8>, String)> =
        sqlx::query_as("SELECT id, name FROM warehouses ORDER BY name")
            .fetch_all(pool.inner())
            .await
            .map_err(|_| Status::InternalServerError)?;

    let zones: Vec<(Vec<u8>, Vec<u8>, String)> =
        sqlx::query_as("SELECT id, warehouse_id, name FROM warehouse_zones ORDER BY name")
            .fetch_all(pool.inner())
            .await
            .map_err(|_| Status::InternalServerError)?;

    let bins: Vec<(Vec<u8>, Vec<u8>, String, f64, f64, f64, f64, String, i8)> = sqlx::query_as(
        "SELECT id, zone_id, name, width_in, height_in, depth_in, max_load_lbs, \
                temp_zone, is_enabled \
         FROM bins ORDER BY name",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;

    let mut bins_by_zone: HashMap<Uuid, Vec<Bin>> = HashMap::new();
    for (bid, zid, name, w, h, d, load, temp, enabled) in bins {
        let (Ok(bid), Ok(zid)) = (Uuid::from_slice(&bid), Uuid::from_slice(&zid)) else {
            continue;
        };
        bins_by_zone.entry(zid).or_default().push(Bin {
            id: bid.to_string(),
            zone_id: zid.to_string(),
            name,
            width_in: w,
            height_in: h,
            depth_in: d,
            max_load_lbs: load,
            temp_zone: temp,
            is_enabled: enabled != 0,
        });
    }

    let mut zones_by_wh: HashMap<Uuid, Vec<WarehouseZoneNode>> = HashMap::new();
    for (zid, wid, name) in zones {
        let (Ok(zid), Ok(wid)) = (Uuid::from_slice(&zid), Uuid::from_slice(&wid)) else {
            continue;
        };
        let zone_bins = bins_by_zone.remove(&zid).unwrap_or_default();
        zones_by_wh.entry(wid).or_default().push(WarehouseZoneNode {
            id: zid.to_string(),
            name,
            bins: zone_bins,
        });
    }

    let tree: Vec<WarehouseTreeNode> = warehouses
        .into_iter()
        .filter_map(|(id_b, name)| {
            let id = Uuid::from_slice(&id_b).ok()?;
            let zones = zones_by_wh.remove(&id).unwrap_or_default();
            Some(WarehouseTreeNode {
                id: id.to_string(),
                name,
                zones,
            })
        })
        .collect();

    Ok(Json(tree))
}
