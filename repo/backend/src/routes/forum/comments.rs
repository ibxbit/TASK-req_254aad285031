use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{Comment, CreateCommentRequest};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::auth::guard::AuthUser;
use crate::forum::{moderation, visibility};

#[get("/posts/<id>/comments")]
pub async fn list_by_post(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
) -> Result<Json<Vec<Comment>>, Status> {
    let pid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let row: Option<(Vec<u8>,)> = sqlx::query_as("SELECT board_id FROM posts WHERE id = ?")
        .bind(&pid.as_bytes()[..])
        .fetch_optional(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    let Some((bid_b,)) = row else {
        return Err(Status::NotFound);
    };
    let bid = Uuid::from_slice(&bid_b).map_err(|_| Status::InternalServerError)?;
    if !visibility::user_can_see_board(pool.inner(), user.id, user.role, bid)
        .await
        .map_err(|_| Status::InternalServerError)?
    {
        return Err(Status::Forbidden);
    }
    let rows: Vec<(Vec<u8>, Vec<u8>, String, chrono::NaiveDateTime)> = sqlx::query_as(
        "SELECT id, author_id, content, created_at FROM comments \
         WHERE post_id = ? ORDER BY created_at",
    )
    .bind(&pid.as_bytes()[..])
    .fetch_all(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;
    let comments = rows
        .into_iter()
        .filter_map(|(cid, aid, content, ts)| {
            Some(Comment {
                id: Uuid::from_slice(&cid).ok()?.to_string(),
                post_id: pid.to_string(),
                author_id: Uuid::from_slice(&aid).ok()?.to_string(),
                content,
                created_at: ts,
            })
        })
        .collect();
    Ok(Json(comments))
}

#[post("/comments", format = "json", data = "<req>")]
pub async fn create(
    pool: &State<MySqlPool>,
    user: AuthUser,
    req: Json<CreateCommentRequest>,
) -> Result<Json<Comment>, Status> {
    let pid = Uuid::parse_str(&req.post_id).map_err(|_| Status::BadRequest)?;
    let row: Option<(Vec<u8>,)> = sqlx::query_as("SELECT board_id FROM posts WHERE id = ?")
        .bind(&pid.as_bytes()[..])
        .fetch_optional(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    let Some((bid_b,)) = row else {
        return Err(Status::NotFound);
    };
    let bid = Uuid::from_slice(&bid_b).map_err(|_| Status::InternalServerError)?;
    if !visibility::user_can_see_board(pool.inner(), user.id, user.role, bid)
        .await
        .map_err(|_| Status::InternalServerError)?
    {
        return Err(Status::Forbidden);
    }
    let cid = Uuid::new_v4();
    let now = chrono::Utc::now().naive_utc();
    sqlx::query(
        "INSERT INTO comments (id, post_id, author_id, content, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&cid.as_bytes()[..])
    .bind(&pid.as_bytes()[..])
    .bind(&user.id.as_bytes()[..])
    .bind(&req.content)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;
    Ok(Json(Comment {
        id: cid.to_string(),
        post_id: pid.to_string(),
        author_id: user.id.to_string(),
        content: req.content.clone(),
        created_at: now,
    }))
}

// Per spec: only board moderators and administrators can remove comments.
#[delete("/comments/<id>")]
pub async fn delete(pool: &State<MySqlPool>, user: AuthUser, id: &str) -> Result<Status, Status> {
    let cid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let row: Option<(Vec<u8>,)> = sqlx::query_as(
        "SELECT p.board_id FROM comments c \
         JOIN posts p ON p.id = c.post_id \
         WHERE c.id = ?",
    )
    .bind(&cid.as_bytes()[..])
    .fetch_optional(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;
    let Some((bid_b,)) = row else {
        return Err(Status::NotFound);
    };
    let bid = Uuid::from_slice(&bid_b).map_err(|_| Status::InternalServerError)?;
    if !moderation::can_moderate_board(pool.inner(), user.role, bid, user.id)
        .await
        .map_err(|_| Status::InternalServerError)?
    {
        return Err(Status::Forbidden);
    }
    sqlx::query("DELETE FROM comments WHERE id = ?")
        .bind(&cid.as_bytes()[..])
        .execute(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    Ok(Status::NoContent)
}
