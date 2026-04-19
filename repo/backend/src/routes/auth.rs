// Auth controllers — layered reference implementation.
//
// Flow:
//   Controller (this file)  -> parse request, enforce rules, format response
//   Service     (crate::services::auth::*)  -> password hashing, session, lock
//   Repository  (crate::repositories::users) -> SQL only

use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{LoginRequest, LoginResponse, Role, SessionUser};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::logging;
use crate::repositories::users as users_repo;
use crate::services::auth::guard::{AuthUser, BearerToken};
use crate::services::auth::{lock, password, session};

// Bootstrap endpoint: creates the first Administrator when the users table is empty.
#[post("/auth/register", format = "json", data = "<req>")]
pub async fn register(pool: &State<MySqlPool>, req: Json<LoginRequest>) -> Result<Status, Status> {
    let count = users_repo::count(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    if count > 0 {
        logging::permission_denied("-", "-", "POST /auth/register (bootstrap)");
        return Err(Status::Forbidden);
    }
    if password::validate(&req.password).is_err() {
        logging::validation_failed("POST /auth/register", "password too short");
        return Err(Status::BadRequest);
    }
    let hash = password::hash(&req.password).map_err(|_| Status::InternalServerError)?;
    let id = Uuid::new_v4();
    users_repo::create(
        pool.inner(),
        id,
        &req.username,
        &hash,
        Role::Administrator.as_str(),
    )
    .await
    .map_err(|_| Status::BadRequest)?;
    Ok(Status::Created)
}

#[post("/auth/login", format = "json", data = "<req>")]
pub async fn login(
    pool: &State<MySqlPool>,
    req: Json<LoginRequest>,
) -> Result<Json<LoginResponse>, Status> {
    let auth_row = users_repo::find_auth_by_username(pool.inner(), &req.username)
        .await
        .map_err(|_| Status::InternalServerError)?;
    let Some(auth_row) = auth_row else {
        logging::auth_failure(&req.username, "user_not_found");
        return Err(Status::Unauthorized);
    };

    if !auth_row.is_active {
        logging::auth_failure(&req.username, "user_inactive");
        return Err(Status::Forbidden);
    }

    if lock::is_locked(pool.inner(), auth_row.id)
        .await
        .map_err(|_| Status::InternalServerError)?
    {
        logging::auth_failure(&req.username, "locked");
        return Err(Status::Locked);
    }

    if !password::verify(&req.password, &auth_row.password_hash) {
        let _ = lock::register_failure(pool.inner(), auth_row.id).await;
        logging::auth_failure(&req.username, "bad_password");
        return Err(Status::Unauthorized);
    }

    lock::reset(pool.inner(), auth_row.id)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let token = session::create(pool.inner(), auth_row.id)
        .await
        .map_err(|_| Status::InternalServerError)?;
    let role = Role::from_str(&auth_row.role).ok_or(Status::InternalServerError)?;

    Ok(Json(LoginResponse {
        token,
        user: SessionUser {
            id: auth_row.id.to_string(),
            username: req.username.clone(),
            role,
        },
    }))
}

#[post("/auth/logout")]
pub async fn logout(pool: &State<MySqlPool>, _user: AuthUser, token: BearerToken) -> Status {
    let _ = session::delete(pool.inner(), &token.0).await;
    Status::NoContent
}

#[get("/auth/me")]
pub fn me(user: AuthUser) -> Json<SessionUser> {
    Json(SessionUser {
        id: user.id.to_string(),
        username: user.username,
        role: user.role,
    })
}
