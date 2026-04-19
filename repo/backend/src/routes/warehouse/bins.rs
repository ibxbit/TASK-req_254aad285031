use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{Bin, BinChangeLog, CreateBinRequest, Role, UpdateBinRequest};
use sqlx::{MySql, MySqlPool, Transaction};
use uuid::Uuid;

use crate::audit;
use crate::auth::guard::AuthUser;

const MGMT: [Role; 2] = [Role::Administrator, Role::WarehouseManager];
const ENTITY: &str = "bin";

// ---------- Create ----------

#[post("/bins", format = "json", data = "<req>")]
pub async fn create(
    pool: &State<MySqlPool>,
    user: AuthUser,
    req: Json<CreateBinRequest>,
) -> Result<Json<Bin>, Status> {
    user.require_any(&MGMT)?;
    if req.width_in <= 0.0 || req.height_in <= 0.0 || req.depth_in <= 0.0 {
        return Err(Status::BadRequest);
    }
    if req.max_load_lbs < 0.0 {
        return Err(Status::BadRequest);
    }
    if req.temp_zone.trim().is_empty() {
        return Err(Status::BadRequest);
    }

    let zid = Uuid::parse_str(&req.zone_id).map_err(|_| Status::BadRequest)?;
    let id = Uuid::new_v4();
    let enabled = req.is_enabled.unwrap_or(true);

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    sqlx::query(
        "INSERT INTO bins (id, zone_id, name, width_in, height_in, depth_in, \
                           max_load_lbs, temp_zone, is_enabled) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id.as_bytes()[..])
    .bind(&zid.as_bytes()[..])
    .bind(&req.name)
    .bind(req.width_in)
    .bind(req.height_in)
    .bind(req.depth_in)
    .bind(req.max_load_lbs)
    .bind(&req.temp_zone)
    .bind(enabled as i8)
    .execute(&mut *tx)
    .await
    .map_err(|_| Status::BadRequest)?;

    // Structural change log: parity with warehouse_change_log /
    // warehouse_zone_change_log, which both write a 'create' row so the
    // per-entity history endpoint can rebuild the full lifecycle.
    // Serialise the new-value snapshot so the history endpoint can show
    // the dimensions the bin was born with.
    let new_snapshot = serde_json::json!({
        "zone_id": zid.to_string(),
        "name": req.name,
        "width_in": req.width_in,
        "height_in": req.height_in,
        "depth_in": req.depth_in,
        "max_load_lbs": req.max_load_lbs,
        "temp_zone": req.temp_zone,
        "is_enabled": enabled,
    })
    .to_string();
    log_change(&mut tx, id, user.id, "create", String::new(), new_snapshot)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let payload = serde_json::json!({
        "action": "create",
        "id": id.to_string(),
        "zone_id": zid.to_string(),
        "name": req.name,
        "width_in": req.width_in,
        "height_in": req.height_in,
        "depth_in": req.depth_in,
        "max_load_lbs": req.max_load_lbs,
        "temp_zone": req.temp_zone,
        "is_enabled": enabled,
    });
    audit::record_event_tx(&mut tx, ENTITY, id, "create", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Json(Bin {
        id: id.to_string(),
        zone_id: zid.to_string(),
        name: req.name.clone(),
        width_in: req.width_in,
        height_in: req.height_in,
        depth_in: req.depth_in,
        max_load_lbs: req.max_load_lbs,
        temp_zone: req.temp_zone.clone(),
        is_enabled: enabled,
    }))
}

// ---------- Patch with change logging + tamper-evident audit ----------

async fn log_change(
    tx: &mut Transaction<'_, MySql>,
    bin_id: Uuid,
    changed_by: Uuid,
    change_type: &str,
    old: String,
    new: String,
) -> sqlx::Result<()> {
    let log_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO bin_change_log \
         (id, bin_id, changed_by, change_type, old_value, new_value) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&log_id.as_bytes()[..])
    .bind(&bin_id.as_bytes()[..])
    .bind(&changed_by.as_bytes()[..])
    .bind(change_type)
    .bind(old)
    .bind(new)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[patch("/bins/<id>", format = "json", data = "<req>")]
pub async fn update(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<UpdateBinRequest>,
) -> Result<Status, Status> {
    user.require_any(&MGMT)?;
    let bid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    let row: Option<(String, f64, f64, f64, f64, String, i8)> = sqlx::query_as(
        "SELECT name, width_in, height_in, depth_in, max_load_lbs, temp_zone, is_enabled \
         FROM bins WHERE id = ?",
    )
    .bind(&bid.as_bytes()[..])
    .fetch_optional(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;

    let Some((name, w, h, d, load, temp, enabled)) = row else {
        return Err(Status::NotFound);
    };

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let mut changes = serde_json::Map::new();

    if let Some(new) = &req.name {
        if new != &name {
            log_change(&mut tx, bid, user.id, "name", name.clone(), new.clone())
                .await
                .map_err(|_| Status::InternalServerError)?;
            sqlx::query("UPDATE bins SET name = ? WHERE id = ?")
                .bind(new)
                .bind(&bid.as_bytes()[..])
                .execute(&mut *tx)
                .await
                .map_err(|_| Status::InternalServerError)?;
            changes.insert(
                "name".into(),
                serde_json::json!({ "old": name, "new": new }),
            );
        }
    }
    if let Some(new) = req.width_in {
        if (new - w).abs() > f64::EPSILON {
            log_change(
                &mut tx,
                bid,
                user.id,
                "width_in",
                w.to_string(),
                new.to_string(),
            )
            .await
            .map_err(|_| Status::InternalServerError)?;
            sqlx::query("UPDATE bins SET width_in = ? WHERE id = ?")
                .bind(new)
                .bind(&bid.as_bytes()[..])
                .execute(&mut *tx)
                .await
                .map_err(|_| Status::BadRequest)?;
            changes.insert(
                "width_in".into(),
                serde_json::json!({ "old": w, "new": new }),
            );
        }
    }
    if let Some(new) = req.height_in {
        if (new - h).abs() > f64::EPSILON {
            log_change(
                &mut tx,
                bid,
                user.id,
                "height_in",
                h.to_string(),
                new.to_string(),
            )
            .await
            .map_err(|_| Status::InternalServerError)?;
            sqlx::query("UPDATE bins SET height_in = ? WHERE id = ?")
                .bind(new)
                .bind(&bid.as_bytes()[..])
                .execute(&mut *tx)
                .await
                .map_err(|_| Status::BadRequest)?;
            changes.insert(
                "height_in".into(),
                serde_json::json!({ "old": h, "new": new }),
            );
        }
    }
    if let Some(new) = req.depth_in {
        if (new - d).abs() > f64::EPSILON {
            log_change(
                &mut tx,
                bid,
                user.id,
                "depth_in",
                d.to_string(),
                new.to_string(),
            )
            .await
            .map_err(|_| Status::InternalServerError)?;
            sqlx::query("UPDATE bins SET depth_in = ? WHERE id = ?")
                .bind(new)
                .bind(&bid.as_bytes()[..])
                .execute(&mut *tx)
                .await
                .map_err(|_| Status::BadRequest)?;
            changes.insert(
                "depth_in".into(),
                serde_json::json!({ "old": d, "new": new }),
            );
        }
    }
    if let Some(new) = req.max_load_lbs {
        if (new - load).abs() > f64::EPSILON {
            log_change(
                &mut tx,
                bid,
                user.id,
                "max_load_lbs",
                load.to_string(),
                new.to_string(),
            )
            .await
            .map_err(|_| Status::InternalServerError)?;
            sqlx::query("UPDATE bins SET max_load_lbs = ? WHERE id = ?")
                .bind(new)
                .bind(&bid.as_bytes()[..])
                .execute(&mut *tx)
                .await
                .map_err(|_| Status::BadRequest)?;
            changes.insert(
                "max_load_lbs".into(),
                serde_json::json!({ "old": load, "new": new }),
            );
        }
    }
    if let Some(new) = &req.temp_zone {
        if new != &temp {
            log_change(
                &mut tx,
                bid,
                user.id,
                "temp_zone",
                temp.clone(),
                new.clone(),
            )
            .await
            .map_err(|_| Status::InternalServerError)?;
            sqlx::query("UPDATE bins SET temp_zone = ? WHERE id = ?")
                .bind(new)
                .bind(&bid.as_bytes()[..])
                .execute(&mut *tx)
                .await
                .map_err(|_| Status::InternalServerError)?;
            changes.insert(
                "temp_zone".into(),
                serde_json::json!({ "old": temp, "new": new }),
            );
        }
    }
    if let Some(new) = req.is_enabled {
        let cur_bool = enabled != 0;
        if new != cur_bool {
            log_change(
                &mut tx,
                bid,
                user.id,
                "is_enabled",
                cur_bool.to_string(),
                new.to_string(),
            )
            .await
            .map_err(|_| Status::InternalServerError)?;
            sqlx::query("UPDATE bins SET is_enabled = ? WHERE id = ?")
                .bind(new as i8)
                .bind(&bid.as_bytes()[..])
                .execute(&mut *tx)
                .await
                .map_err(|_| Status::InternalServerError)?;
            changes.insert(
                "is_enabled".into(),
                serde_json::json!({ "old": cur_bool, "new": new }),
            );
        }
    }

    // Only emit an event_log entry if something actually changed.
    if !changes.is_empty() {
        let payload = serde_json::json!({
            "action": "update",
            "id": bid.to_string(),
            "changes": serde_json::Value::Object(changes),
        });
        audit::record_event_tx(&mut tx, ENTITY, bid, "update", &payload)
            .await
            .map_err(|_| Status::InternalServerError)?;
    }

    tx.commit().await.map_err(|_| Status::InternalServerError)?;
    Ok(Status::NoContent)
}

// ---------- History ----------

#[get("/bins/<id>/history")]
pub async fn history(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
) -> Result<Json<Vec<BinChangeLog>>, Status> {
    // Parity with /warehouses/<id>/history and /warehouse-zones/<id>/history:
    // structural change logs are management-only.
    user.require_any(&MGMT)?;
    let bid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let rows: Vec<(
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        String,
        Option<String>,
        Option<String>,
        chrono::NaiveDateTime,
    )> = sqlx::query_as(
        "SELECT id, bin_id, changed_by, change_type, old_value, new_value, created_at \
         FROM bin_change_log WHERE bin_id = ? ORDER BY created_at DESC",
    )
    .bind(&bid.as_bytes()[..])
    .fetch_all(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;

    let logs = rows
        .into_iter()
        .filter_map(|(lid, bid_b, uid, ct, ov, nv, ts)| {
            Some(BinChangeLog {
                id: Uuid::from_slice(&lid).ok()?.to_string(),
                bin_id: Uuid::from_slice(&bid_b).ok()?.to_string(),
                changed_by: Uuid::from_slice(&uid).ok()?.to_string(),
                change_type: ct,
                old_value: ov,
                new_value: nv,
                created_at: ts,
            })
        })
        .collect();
    Ok(Json(logs))
}
