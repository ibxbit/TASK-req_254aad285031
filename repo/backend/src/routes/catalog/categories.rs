use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{AssignCategoryRequest, Category, CreateCategoryRequest, Role};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::auth::guard::AuthUser;

const MANAGEMENT_ROLES: [Role; 2] = [Role::Administrator, Role::ServiceManager];

// GET /categories — read-only browsing for requesters building a search
// filter. Returns a flat list sorted so roots appear first and children
// follow alphabetically; the frontend reconstructs the tree via
// `parent_id`.
#[get("/categories")]
pub async fn list(pool: &State<MySqlPool>, _user: AuthUser) -> Result<Json<Vec<Category>>, Status> {
    let rows: Vec<(Vec<u8>, Option<Vec<u8>>, String)> = sqlx::query_as(
        "SELECT id, parent_id, name FROM categories \
         ORDER BY (parent_id IS NULL) DESC, name, id",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;
    let out = rows
        .into_iter()
        .filter_map(|(id, pid, name)| {
            Some(Category {
                id: Uuid::from_slice(&id).ok()?.to_string(),
                parent_id: pid.and_then(|b| Uuid::from_slice(&b).ok().map(|u| u.to_string())),
                name,
            })
        })
        .collect();
    Ok(Json(out))
}

#[post("/categories", format = "json", data = "<req>")]
pub async fn create(
    pool: &State<MySqlPool>,
    user: AuthUser,
    req: Json<CreateCategoryRequest>,
) -> Result<Json<Category>, Status> {
    user.require_any(&MANAGEMENT_ROLES)?;
    let id = Uuid::new_v4();
    let parent_uuid = match &req.parent_id {
        Some(p) => Some(Uuid::parse_str(p).map_err(|_| Status::BadRequest)?),
        None => None,
    };
    sqlx::query("INSERT INTO categories (id, parent_id, name) VALUES (?, ?, ?)")
        .bind(&id.as_bytes()[..])
        .bind(parent_uuid.map(|u| u.as_bytes().to_vec()))
        .bind(&req.name)
        .execute(pool.inner())
        .await
        .map_err(|_| Status::BadRequest)?;
    Ok(Json(Category {
        id: id.to_string(),
        parent_id: parent_uuid.map(|u| u.to_string()),
        name: req.name.clone(),
    }))
}

#[post("/services/<id>/categories", format = "json", data = "<req>")]
pub async fn assign(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<AssignCategoryRequest>,
) -> Result<Status, Status> {
    user.require_any(&MANAGEMENT_ROLES)?;
    let sid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let cid = Uuid::parse_str(&req.category_id).map_err(|_| Status::BadRequest)?;
    sqlx::query("INSERT IGNORE INTO service_categories (service_id, category_id) VALUES (?, ?)")
        .bind(&sid.as_bytes()[..])
        .bind(&cid.as_bytes()[..])
        .execute(pool.inner())
        .await
        .map_err(|_| Status::BadRequest)?;
    Ok(Status::Created)
}
