use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{AssignTagRequest, CreateTagRequest, Role, Tag};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::auth::guard::AuthUser;

const MANAGEMENT_ROLES: [Role; 2] = [Role::Administrator, Role::ServiceManager];

// GET /tags — read-only browsing for the requester search filter panel.
#[get("/tags")]
pub async fn list(pool: &State<MySqlPool>, _user: AuthUser) -> Result<Json<Vec<Tag>>, Status> {
    let rows: Vec<(Vec<u8>, String)> =
        sqlx::query_as("SELECT id, name FROM tags ORDER BY name, id")
            .fetch_all(pool.inner())
            .await
            .map_err(|_| Status::InternalServerError)?;
    let out = rows
        .into_iter()
        .filter_map(|(id, name)| {
            Some(Tag {
                id: Uuid::from_slice(&id).ok()?.to_string(),
                name,
            })
        })
        .collect();
    Ok(Json(out))
}

#[post("/tags", format = "json", data = "<req>")]
pub async fn create(
    pool: &State<MySqlPool>,
    user: AuthUser,
    req: Json<CreateTagRequest>,
) -> Result<Json<Tag>, Status> {
    user.require_any(&MANAGEMENT_ROLES)?;
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO tags (id, name) VALUES (?, ?)")
        .bind(&id.as_bytes()[..])
        .bind(&req.name)
        .execute(pool.inner())
        .await
        .map_err(|_| Status::BadRequest)?;
    Ok(Json(Tag {
        id: id.to_string(),
        name: req.name.clone(),
    }))
}

#[post("/services/<id>/tags", format = "json", data = "<req>")]
pub async fn assign(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<AssignTagRequest>,
) -> Result<Status, Status> {
    user.require_any(&MANAGEMENT_ROLES)?;
    let sid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let tid = Uuid::parse_str(&req.tag_id).map_err(|_| Status::BadRequest)?;
    sqlx::query("INSERT IGNORE INTO service_tags (service_id, tag_id) VALUES (?, ?)")
        .bind(&sid.as_bytes()[..])
        .bind(&tid.as_bytes()[..])
        .execute(pool.inner())
        .await
        .map_err(|_| Status::BadRequest)?;
    Ok(Status::Created)
}
