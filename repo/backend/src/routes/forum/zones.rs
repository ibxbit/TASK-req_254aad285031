use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{CreateZoneRequest, Role, UpdateZoneRequest, Zone};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::auth::guard::AuthUser;

#[get("/zones")]
pub async fn list(pool: &State<MySqlPool>, _user: AuthUser) -> Result<Json<Vec<Zone>>, Status> {
    let rows: Vec<(Vec<u8>, String)> = sqlx::query_as("SELECT id, name FROM zones ORDER BY name")
        .fetch_all(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    let zones = rows
        .into_iter()
        .filter_map(|(id, name)| {
            Uuid::from_slice(&id).ok().map(|u| Zone {
                id: u.to_string(),
                name,
            })
        })
        .collect();
    Ok(Json(zones))
}

#[post("/zones", format = "json", data = "<req>")]
pub async fn create(
    pool: &State<MySqlPool>,
    user: AuthUser,
    req: Json<CreateZoneRequest>,
) -> Result<Json<Zone>, Status> {
    user.require_role(Role::Administrator)?;
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO zones (id, name) VALUES (?, ?)")
        .bind(&id.as_bytes()[..])
        .bind(&req.name)
        .execute(pool.inner())
        .await
        .map_err(|_| Status::BadRequest)?;
    Ok(Json(Zone {
        id: id.to_string(),
        name: req.name.clone(),
    }))
}

#[patch("/zones/<id>", format = "json", data = "<req>")]
pub async fn update(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<UpdateZoneRequest>,
) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let zid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let res = sqlx::query("UPDATE zones SET name = ? WHERE id = ?")
        .bind(&req.name)
        .bind(&zid.as_bytes()[..])
        .execute(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    if res.rows_affected() == 0 {
        return Err(Status::NotFound);
    }
    Ok(Status::NoContent)
}

#[delete("/zones/<id>")]
pub async fn delete(pool: &State<MySqlPool>, user: AuthUser, id: &str) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let zid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let res = sqlx::query("DELETE FROM zones WHERE id = ?")
        .bind(&zid.as_bytes()[..])
        .execute(pool.inner())
        .await
        .map_err(|_| Status::Conflict)?;
    if res.rows_affected() == 0 {
        return Err(Status::NotFound);
    }
    Ok(Status::NoContent)
}
