// Administrator-only user lifecycle endpoints.
//
// Bootstrap (public, users-table empty): POST /api/auth/register
// Ongoing admin management (Administrator role required):
//   GET    /api/admin/users                 list
//   POST   /api/admin/users                 create any role
//   PATCH  /api/admin/users/<id>/role       change role
//   PATCH  /api/admin/users/<id>/password   reset password
//   PATCH  /api/admin/users/<id>/status     activate/deactivate
//   PUT    /api/admin/users/<id>/sensitive  set encrypted sensitive id
//
// Every mutation writes an `audit::record_event` entry so role/lifecycle
// changes are tamper-evident alongside the rest of the system.

use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};
use shared::Role;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::audit;
use crate::auth::guard::AuthUser;
use crate::auth::password;
use crate::crypto::{mask_identifier, Encryptor};
use crate::logging;
use crate::repositories::users as users_repo;

const ENTITY: &str = "user";

#[derive(Debug, Serialize)]
pub struct AdminUserView {
    pub id: String,
    pub username: String,
    pub role: Role,
    pub is_active: bool,
    // Only the mask is ever shipped to clients.
    pub sensitive_id_mask: Option<String>,
}

fn row_to_view(r: users_repo::UserProfileRow) -> Option<AdminUserView> {
    Some(AdminUserView {
        id: r.id.to_string(),
        username: r.username,
        role: Role::from_str(&r.role)?,
        is_active: r.is_active,
        sensitive_id_mask: r.sensitive_id_mask,
    })
}

#[get("/admin/users")]
pub async fn list(
    pool: &State<MySqlPool>,
    user: AuthUser,
) -> Result<Json<Vec<AdminUserView>>, Status> {
    if user.require_role(Role::Administrator).is_err() {
        logging::permission_denied(&user.id.to_string(), user.role.as_str(), "GET /admin/users");
        return Err(Status::Forbidden);
    }
    let rows = users_repo::list_all(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    Ok(Json(rows.into_iter().filter_map(row_to_view).collect()))
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: Role,
}

#[post("/admin/users", format = "json", data = "<req>")]
pub async fn create(
    pool: &State<MySqlPool>,
    user: AuthUser,
    req: Json<CreateUserRequest>,
) -> Result<Json<AdminUserView>, Status> {
    if user.require_role(Role::Administrator).is_err() {
        logging::permission_denied(
            &user.id.to_string(),
            user.role.as_str(),
            "POST /admin/users",
        );
        return Err(Status::Forbidden);
    }
    if req.username.trim().is_empty() || req.username.chars().count() > 64 {
        logging::validation_failed("POST /admin/users", "invalid username");
        return Err(Status::BadRequest);
    }
    if password::validate(&req.password).is_err() {
        logging::validation_failed("POST /admin/users", "password too short");
        return Err(Status::BadRequest);
    }

    let hash = password::hash(&req.password).map_err(|_| Status::InternalServerError)?;
    let id = Uuid::new_v4();

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let res = sqlx::query(
        "INSERT INTO users (id, username, password_hash, role, is_active) \
         VALUES (?, ?, ?, ?, 1)",
    )
    .bind(&id.as_bytes()[..])
    .bind(&req.username)
    .bind(&hash)
    .bind(req.role.as_str())
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
        "username": req.username,
        "role": req.role.as_str(),
        "created_by": user.id.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, id, "create", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;

    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Json(AdminUserView {
        id: id.to_string(),
        username: req.username.clone(),
        role: req.role,
        is_active: true,
        sensitive_id_mask: None,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub role: Role,
}

#[patch("/admin/users/<id>/role", format = "json", data = "<req>")]
pub async fn update_role(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<UpdateRoleRequest>,
) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let uid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    // Guardrail: don't let an administrator demote themselves to a
    // non-admin role if they're the only admin (would lock out the system).
    if uid == user.id && req.role != Role::Administrator {
        let other_admins: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users WHERE role = 'administrator' AND id <> ? AND is_active = 1",
        )
        .bind(&uid.as_bytes()[..])
        .fetch_one(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
        if other_admins.0 == 0 {
            logging::validation_failed(
                "PATCH /admin/users/<id>/role",
                "would leave no active administrator",
            );
            return Err(Status::Conflict);
        }
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;
    let rows = sqlx::query("UPDATE users SET role = ? WHERE id = ?")
        .bind(req.role.as_str())
        .bind(&uid.as_bytes()[..])
        .execute(&mut *tx)
        .await
        .map_err(|_| Status::InternalServerError)?
        .rows_affected();
    if rows == 0 {
        return Err(Status::NotFound);
    }

    let payload = serde_json::json!({
        "action": "update_role",
        "id": uid.to_string(),
        "new_role": req.role.as_str(),
        "by": user.id.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, uid, "update_role", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;
    tx.commit().await.map_err(|_| Status::InternalServerError)?;
    Ok(Status::NoContent)
}

#[derive(Debug, Deserialize)]
pub struct UpdatePasswordRequest {
    pub password: String,
}

#[patch("/admin/users/<id>/password", format = "json", data = "<req>")]
pub async fn update_password(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<UpdatePasswordRequest>,
) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let uid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    if password::validate(&req.password).is_err() {
        return Err(Status::BadRequest);
    }
    let hash = password::hash(&req.password).map_err(|_| Status::InternalServerError)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;
    let rows = sqlx::query(
        "UPDATE users SET password_hash = ?, failed_login_count = 0, \
                        locked_until = NULL WHERE id = ?",
    )
    .bind(&hash)
    .bind(&uid.as_bytes()[..])
    .execute(&mut *tx)
    .await
    .map_err(|_| Status::InternalServerError)?
    .rows_affected();
    if rows == 0 {
        return Err(Status::NotFound);
    }

    let payload = serde_json::json!({
        "action": "reset_password",
        "id": uid.to_string(),
        "by": user.id.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, uid, "reset_password", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;
    tx.commit().await.map_err(|_| Status::InternalServerError)?;
    Ok(Status::NoContent)
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub is_active: bool,
}

#[patch("/admin/users/<id>/status", format = "json", data = "<req>")]
pub async fn update_status(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<UpdateStatusRequest>,
) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let uid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    if uid == user.id && !req.is_active {
        logging::validation_failed(
            "PATCH /admin/users/<id>/status",
            "admin cannot deactivate self",
        );
        return Err(Status::Conflict);
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;
    let rows = sqlx::query("UPDATE users SET is_active = ? WHERE id = ?")
        .bind(req.is_active as i8)
        .bind(&uid.as_bytes()[..])
        .execute(&mut *tx)
        .await
        .map_err(|_| Status::InternalServerError)?
        .rows_affected();
    if rows == 0 {
        return Err(Status::NotFound);
    }

    // On deactivation, revoke every active session for the user in the
    // same transaction as the status flip. Without this, an attacker (or
    // a still-signed-in tab) could keep using the pre-deactivation
    // bearer token until natural expiry.
    let revoked = if !req.is_active {
        crate::auth::session::delete_all_for_user_tx(&mut tx, uid)
            .await
            .map_err(|_| Status::InternalServerError)?
    } else {
        0
    };

    let payload = serde_json::json!({
        "action": "update_status",
        "id": uid.to_string(),
        "is_active": req.is_active,
        "by": user.id.to_string(),
        "sessions_revoked": revoked,
    });
    audit::record_event_tx(&mut tx, ENTITY, uid, "update_status", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;
    tx.commit().await.map_err(|_| Status::InternalServerError)?;
    Ok(Status::NoContent)
}

#[derive(Debug, Deserialize)]
pub struct UpdateSensitiveRequest {
    pub value: String,
}

#[put("/admin/users/<id>/sensitive", format = "json", data = "<req>")]
pub async fn update_sensitive(
    pool: &State<MySqlPool>,
    enc: &State<Encryptor>,
    user: AuthUser,
    id: &str,
    req: Json<UpdateSensitiveRequest>,
) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let uid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    if req.value.trim().is_empty() || req.value.len() > 128 {
        return Err(Status::BadRequest);
    }
    let ct = enc
        .encrypt(req.value.as_bytes())
        .map_err(|_| Status::InternalServerError)?;
    let mask = mask_identifier(&req.value);

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let rows =
        sqlx::query("UPDATE users SET sensitive_id_enc = ?, sensitive_id_mask = ? WHERE id = ?")
            .bind(&ct)
            .bind(&mask)
            .bind(&uid.as_bytes()[..])
            .execute(&mut *tx)
            .await
            .map_err(|_| Status::InternalServerError)?
            .rows_affected();
    if rows == 0 {
        return Err(Status::NotFound);
    }

    // Audit payload records only the MASK, never plaintext.
    let payload = serde_json::json!({
        "action": "set_sensitive_id",
        "id": uid.to_string(),
        "mask": mask,
        "by": user.id.to_string(),
    });
    audit::record_event_tx(&mut tx, ENTITY, uid, "set_sensitive_id", &payload)
        .await
        .map_err(|_| Status::InternalServerError)?;
    tx.commit().await.map_err(|_| Status::InternalServerError)?;

    Ok(Status::NoContent)
}
