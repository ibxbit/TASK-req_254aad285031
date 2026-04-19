// Administrator-only team management.
//
//   GET    /api/admin/teams
//   POST   /api/admin/teams                            create
//   DELETE /api/admin/teams/<id>                       delete
//   GET    /api/admin/teams/<id>/members
//   POST   /api/admin/teams/<id>/members               add user
//   DELETE /api/admin/teams/<id>/members/<user_id>
//
// Team membership powers restricted board visibility (forum::visibility).

use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};
use shared::Role;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::audit;
use crate::auth::guard::AuthUser;

const ENTITY: &str = "team";

#[derive(Debug, Serialize)]
pub struct TeamView {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct TeamMember {
    pub user_id: String,
    pub username: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: String,
}

#[get("/admin/teams")]
pub async fn list(pool: &State<MySqlPool>, user: AuthUser) -> Result<Json<Vec<TeamView>>, Status> {
    user.require_role(Role::Administrator)?;
    let rows: Vec<(Vec<u8>, String)> = sqlx::query_as("SELECT id, name FROM teams ORDER BY name")
        .fetch_all(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    Ok(Json(
        rows.into_iter()
            .filter_map(|(id, name)| {
                Some(TeamView {
                    id: Uuid::from_slice(&id).ok()?.to_string(),
                    name,
                })
            })
            .collect(),
    ))
}

#[post("/admin/teams", format = "json", data = "<req>")]
pub async fn create(
    pool: &State<MySqlPool>,
    user: AuthUser,
    req: Json<CreateTeamRequest>,
) -> Result<Json<TeamView>, Status> {
    user.require_role(Role::Administrator)?;
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(Status::BadRequest);
    }

    let id = Uuid::new_v4();
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let res = sqlx::query("INSERT INTO teams (id, name) VALUES (?, ?)")
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

    Ok(Json(TeamView {
        id: id.to_string(),
        name: name.to_string(),
    }))
}

#[delete("/admin/teams/<id>")]
pub async fn delete(pool: &State<MySqlPool>, user: AuthUser, id: &str) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let tid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let rows = sqlx::query("DELETE FROM teams WHERE id = ?")
        .bind(&tid.as_bytes()[..])
        .execute(&mut *tx)
        .await
        .map_err(|_| Status::InternalServerError)?
        .rows_affected();
    if rows == 0 {
        return Err(Status::NotFound);
    }

    let payload = serde_json::json!({
        "action": "delete",
        "id": tid.to_string(),
        "by": user.id.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, tid, "delete", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;
    tx.commit().await.map_err(|_| Status::InternalServerError)?;
    Ok(Status::NoContent)
}

#[get("/admin/teams/<id>/members")]
pub async fn list_members(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
) -> Result<Json<Vec<TeamMember>>, Status> {
    user.require_role(Role::Administrator)?;
    let tid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    let rows: Vec<(Vec<u8>, String, String)> = sqlx::query_as(
        "SELECT u.id, u.username, u.role FROM user_teams ut \
         JOIN users u ON u.id = ut.user_id \
         WHERE ut.team_id = ? ORDER BY u.username",
    )
    .bind(&tid.as_bytes()[..])
    .fetch_all(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;

    Ok(Json(
        rows.into_iter()
            .filter_map(|(id_b, username, role)| {
                Some(TeamMember {
                    user_id: Uuid::from_slice(&id_b).ok()?.to_string(),
                    username,
                    role,
                })
            })
            .collect(),
    ))
}

#[post("/admin/teams/<id>/members", format = "json", data = "<req>")]
pub async fn add_member(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<AddMemberRequest>,
) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let tid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let uid = Uuid::parse_str(&req.user_id).map_err(|_| Status::BadRequest)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let res = sqlx::query("INSERT INTO user_teams (user_id, team_id) VALUES (?, ?)")
        .bind(&uid.as_bytes()[..])
        .bind(&tid.as_bytes()[..])
        .execute(&mut *tx)
        .await;
    if let Err(sqlx::Error::Database(e)) = &res {
        if e.is_unique_violation() {
            return Err(Status::Conflict);
        }
    }
    res.map_err(|_| Status::BadRequest)?;

    let payload = serde_json::json!({
        "action": "add_member",
        "team_id": tid.to_string(),
        "user_id": uid.to_string(),
        "by": user.id.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, tid, "add_member", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;
    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Status::Created)
}

#[delete("/admin/teams/<id>/members/<user_id>")]
pub async fn remove_member(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    user_id: &str,
) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let tid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let uid = Uuid::parse_str(user_id).map_err(|_| Status::BadRequest)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let rows = sqlx::query("DELETE FROM user_teams WHERE user_id = ? AND team_id = ?")
        .bind(&uid.as_bytes()[..])
        .bind(&tid.as_bytes()[..])
        .execute(&mut *tx)
        .await
        .map_err(|_| Status::InternalServerError)?
        .rows_affected();
    if rows == 0 {
        return Err(Status::NotFound);
    }

    let payload = serde_json::json!({
        "action": "remove_member",
        "team_id": tid.to_string(),
        "user_id": uid.to_string(),
        "by": user.id.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, tid, "remove_member", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;
    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Status::NoContent)
}
