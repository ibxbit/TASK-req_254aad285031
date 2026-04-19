use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{CreatePostRequest, PinPostRequest, Post};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::auth::guard::AuthUser;
use crate::forum::{moderation, visibility};

#[get("/boards/<id>/posts")]
pub async fn list_by_board(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
) -> Result<Json<Vec<Post>>, Status> {
    let bid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    if !visibility::user_can_see_board(pool.inner(), user.id, user.role, bid)
        .await
        .map_err(|_| Status::InternalServerError)?
    {
        return Err(Status::Forbidden);
    }
    let rows: Vec<(Vec<u8>, Vec<u8>, String, String, i8, chrono::NaiveDateTime)> = sqlx::query_as(
        "SELECT id, author_id, title, content, is_pinned, created_at \
             FROM posts WHERE board_id = ? \
             ORDER BY is_pinned DESC, created_at DESC",
    )
    .bind(&bid.as_bytes()[..])
    .fetch_all(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;

    let posts = rows
        .into_iter()
        .filter_map(|(pid, aid, title, content, pinned, ts)| {
            Some(Post {
                id: Uuid::from_slice(&pid).ok()?.to_string(),
                board_id: bid.to_string(),
                author_id: Uuid::from_slice(&aid).ok()?.to_string(),
                title,
                content,
                is_pinned: pinned != 0,
                created_at: ts,
            })
        })
        .collect();
    Ok(Json(posts))
}

#[get("/posts/<id>")]
pub async fn get(pool: &State<MySqlPool>, user: AuthUser, id: &str) -> Result<Json<Post>, Status> {
    let pid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let row: Option<(
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        String,
        String,
        i8,
        chrono::NaiveDateTime,
    )> = sqlx::query_as(
        "SELECT id, board_id, author_id, title, content, is_pinned, created_at \
             FROM posts WHERE id = ?",
    )
    .bind(&pid.as_bytes()[..])
    .fetch_optional(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;
    let Some((pid_b, bid_b, aid_b, title, content, pinned, ts)) = row else {
        return Err(Status::NotFound);
    };
    let bid = Uuid::from_slice(&bid_b).map_err(|_| Status::InternalServerError)?;
    if !visibility::user_can_see_board(pool.inner(), user.id, user.role, bid)
        .await
        .map_err(|_| Status::InternalServerError)?
    {
        return Err(Status::Forbidden);
    }
    Ok(Json(Post {
        id: Uuid::from_slice(&pid_b)
            .map_err(|_| Status::InternalServerError)?
            .to_string(),
        board_id: bid.to_string(),
        author_id: Uuid::from_slice(&aid_b)
            .map_err(|_| Status::InternalServerError)?
            .to_string(),
        title,
        content,
        is_pinned: pinned != 0,
        created_at: ts,
    }))
}

#[post("/posts", format = "json", data = "<req>")]
pub async fn create(
    pool: &State<MySqlPool>,
    user: AuthUser,
    req: Json<CreatePostRequest>,
) -> Result<Json<Post>, Status> {
    let bid = Uuid::parse_str(&req.board_id).map_err(|_| Status::BadRequest)?;
    if !visibility::user_can_see_board(pool.inner(), user.id, user.role, bid)
        .await
        .map_err(|_| Status::InternalServerError)?
    {
        return Err(Status::Forbidden);
    }
    let pid = Uuid::new_v4();
    let now = chrono::Utc::now().naive_utc();
    sqlx::query(
        "INSERT INTO posts (id, board_id, author_id, title, content, is_pinned, created_at) \
         VALUES (?, ?, ?, ?, ?, 0, ?)",
    )
    .bind(&pid.as_bytes()[..])
    .bind(&bid.as_bytes()[..])
    .bind(&user.id.as_bytes()[..])
    .bind(&req.title)
    .bind(&req.content)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;
    Ok(Json(Post {
        id: pid.to_string(),
        board_id: bid.to_string(),
        author_id: user.id.to_string(),
        title: req.title.clone(),
        content: req.content.clone(),
        is_pinned: false,
        created_at: now,
    }))
}

#[patch("/posts/<id>/pin", format = "json", data = "<req>")]
pub async fn pin(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<PinPostRequest>,
) -> Result<Status, Status> {
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
    if !moderation::can_moderate_board(pool.inner(), user.role, bid, user.id)
        .await
        .map_err(|_| Status::InternalServerError)?
    {
        return Err(Status::Forbidden);
    }
    sqlx::query("UPDATE posts SET is_pinned = ? WHERE id = ?")
        .bind(req.is_pinned as i8)
        .bind(&pid.as_bytes()[..])
        .execute(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    Ok(Status::NoContent)
}
