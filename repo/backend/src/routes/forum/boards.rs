use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{
    AssignModeratorRequest, AssignTeamRequest, Board, BoardModerator, BoardRule,
    CreateBoardRequest, CreateBoardRuleRequest, Role, UpdateBoardRequest, VisibilityType,
};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::auth::guard::AuthUser;
use crate::forum::{moderation, visibility};

// ---------- Boards ----------

#[get("/boards")]
pub async fn list(pool: &State<MySqlPool>, user: AuthUser) -> Result<Json<Vec<Board>>, Status> {
    let rows: Vec<(Vec<u8>, Vec<u8>, String, String, Vec<u8>)> = if user.role == Role::Administrator
    {
        sqlx::query_as(
            "SELECT id, zone_id, name, visibility_type, created_by FROM boards ORDER BY name",
        )
        .fetch_all(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?
    } else {
        sqlx::query_as(
            "SELECT b.id, b.zone_id, b.name, b.visibility_type, b.created_by \
                 FROM boards b \
                 WHERE b.visibility_type = 'public' \
                    OR EXISTS ( \
                        SELECT 1 FROM board_allowed_teams bat \
                        JOIN user_teams ut ON ut.team_id = bat.team_id \
                        WHERE bat.board_id = b.id AND ut.user_id = ? \
                    ) \
                 ORDER BY b.name",
        )
        .bind(&user.id.as_bytes()[..])
        .fetch_all(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?
    };

    let boards = rows
        .into_iter()
        .filter_map(|(id, zone_id, name, vt, created_by)| {
            Some(Board {
                id: Uuid::from_slice(&id).ok()?.to_string(),
                zone_id: Uuid::from_slice(&zone_id).ok()?.to_string(),
                name,
                visibility_type: VisibilityType::from_str(&vt)?,
                created_by: Uuid::from_slice(&created_by).ok()?.to_string(),
            })
        })
        .collect();
    Ok(Json(boards))
}

#[get("/boards/<id>")]
pub async fn get(pool: &State<MySqlPool>, user: AuthUser, id: &str) -> Result<Json<Board>, Status> {
    let bid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    if !visibility::user_can_see_board(pool.inner(), user.id, user.role, bid)
        .await
        .map_err(|_| Status::InternalServerError)?
    {
        return Err(Status::Forbidden);
    }
    let row: Option<(Vec<u8>, Vec<u8>, String, String, Vec<u8>)> = sqlx::query_as(
        "SELECT id, zone_id, name, visibility_type, created_by FROM boards WHERE id = ?",
    )
    .bind(&bid.as_bytes()[..])
    .fetch_optional(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;
    let Some((bid_b, zid_b, name, vt, cb)) = row else {
        return Err(Status::NotFound);
    };
    Ok(Json(Board {
        id: Uuid::from_slice(&bid_b)
            .map_err(|_| Status::InternalServerError)?
            .to_string(),
        zone_id: Uuid::from_slice(&zid_b)
            .map_err(|_| Status::InternalServerError)?
            .to_string(),
        name,
        visibility_type: VisibilityType::from_str(&vt).ok_or(Status::InternalServerError)?,
        created_by: Uuid::from_slice(&cb)
            .map_err(|_| Status::InternalServerError)?
            .to_string(),
    }))
}

#[post("/boards", format = "json", data = "<req>")]
pub async fn create(
    pool: &State<MySqlPool>,
    user: AuthUser,
    req: Json<CreateBoardRequest>,
) -> Result<Json<Board>, Status> {
    user.require_role(Role::Administrator)?;
    let bid = Uuid::new_v4();
    let zid = Uuid::parse_str(&req.zone_id).map_err(|_| Status::BadRequest)?;
    sqlx::query(
        "INSERT INTO boards (id, zone_id, name, visibility_type, created_by) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&bid.as_bytes()[..])
    .bind(&zid.as_bytes()[..])
    .bind(&req.name)
    .bind(req.visibility_type.as_str())
    .bind(&user.id.as_bytes()[..])
    .execute(pool.inner())
    .await
    .map_err(|_| Status::BadRequest)?;
    Ok(Json(Board {
        id: bid.to_string(),
        zone_id: zid.to_string(),
        name: req.name.clone(),
        visibility_type: req.visibility_type,
        created_by: user.id.to_string(),
    }))
}

#[patch("/boards/<id>", format = "json", data = "<req>")]
pub async fn update(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<UpdateBoardRequest>,
) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let bid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    if let Some(name) = &req.name {
        sqlx::query("UPDATE boards SET name = ? WHERE id = ?")
            .bind(name)
            .bind(&bid.as_bytes()[..])
            .execute(pool.inner())
            .await
            .map_err(|_| Status::InternalServerError)?;
    }
    if let Some(vt) = req.visibility_type {
        sqlx::query("UPDATE boards SET visibility_type = ? WHERE id = ?")
            .bind(vt.as_str())
            .bind(&bid.as_bytes()[..])
            .execute(pool.inner())
            .await
            .map_err(|_| Status::InternalServerError)?;
    }
    Ok(Status::NoContent)
}

#[delete("/boards/<id>")]
pub async fn delete(pool: &State<MySqlPool>, user: AuthUser, id: &str) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let bid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let res = sqlx::query("DELETE FROM boards WHERE id = ?")
        .bind(&bid.as_bytes()[..])
        .execute(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    if res.rows_affected() == 0 {
        return Err(Status::NotFound);
    }
    Ok(Status::NoContent)
}

// ---------- Moderators ----------

#[get("/boards/<id>/moderators")]
pub async fn list_moderators(
    pool: &State<MySqlPool>,
    _user: AuthUser,
    id: &str,
) -> Result<Json<Vec<BoardModerator>>, Status> {
    let bid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let rows: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)> =
        sqlx::query_as("SELECT id, board_id, user_id FROM board_moderators WHERE board_id = ?")
            .bind(&bid.as_bytes()[..])
            .fetch_all(pool.inner())
            .await
            .map_err(|_| Status::InternalServerError)?;
    let mods = rows
        .into_iter()
        .filter_map(|(id, board_id, user_id)| {
            Some(BoardModerator {
                id: Uuid::from_slice(&id).ok()?.to_string(),
                board_id: Uuid::from_slice(&board_id).ok()?.to_string(),
                user_id: Uuid::from_slice(&user_id).ok()?.to_string(),
            })
        })
        .collect();
    Ok(Json(mods))
}

#[post("/boards/<id>/moderators", format = "json", data = "<req>")]
pub async fn add_moderator(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<AssignModeratorRequest>,
) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let bid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let uid = Uuid::parse_str(&req.user_id).map_err(|_| Status::BadRequest)?;
    let mid = Uuid::new_v4();
    sqlx::query("INSERT INTO board_moderators (id, board_id, user_id) VALUES (?, ?, ?)")
        .bind(&mid.as_bytes()[..])
        .bind(&bid.as_bytes()[..])
        .bind(&uid.as_bytes()[..])
        .execute(pool.inner())
        .await
        .map_err(|_| Status::BadRequest)?;
    Ok(Status::Created)
}

#[delete("/boards/<id>/moderators/<user_id>")]
pub async fn remove_moderator(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    user_id: &str,
) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let bid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let uid = Uuid::parse_str(user_id).map_err(|_| Status::BadRequest)?;
    let res = sqlx::query("DELETE FROM board_moderators WHERE board_id = ? AND user_id = ?")
        .bind(&bid.as_bytes()[..])
        .bind(&uid.as_bytes()[..])
        .execute(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    if res.rows_affected() == 0 {
        return Err(Status::NotFound);
    }
    Ok(Status::NoContent)
}

// ---------- Rules ----------

#[get("/boards/<id>/rules")]
pub async fn list_rules(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
) -> Result<Json<Vec<BoardRule>>, Status> {
    let bid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    if !visibility::user_can_see_board(pool.inner(), user.id, user.role, bid)
        .await
        .map_err(|_| Status::InternalServerError)?
    {
        return Err(Status::Forbidden);
    }
    let rows: Vec<(Vec<u8>, String)> = sqlx::query_as(
        "SELECT id, content FROM board_rules WHERE board_id = ? ORDER BY created_at",
    )
    .bind(&bid.as_bytes()[..])
    .fetch_all(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;
    let rules = rows
        .into_iter()
        .filter_map(|(rid, content)| {
            Some(BoardRule {
                id: Uuid::from_slice(&rid).ok()?.to_string(),
                board_id: bid.to_string(),
                content,
            })
        })
        .collect();
    Ok(Json(rules))
}

#[post("/boards/<id>/rules", format = "json", data = "<req>")]
pub async fn create_rule(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<CreateBoardRuleRequest>,
) -> Result<Json<BoardRule>, Status> {
    let bid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    if !moderation::can_moderate_board(pool.inner(), user.role, bid, user.id)
        .await
        .map_err(|_| Status::InternalServerError)?
    {
        return Err(Status::Forbidden);
    }
    let rid = Uuid::new_v4();
    sqlx::query("INSERT INTO board_rules (id, board_id, content) VALUES (?, ?, ?)")
        .bind(&rid.as_bytes()[..])
        .bind(&bid.as_bytes()[..])
        .bind(&req.content)
        .execute(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    Ok(Json(BoardRule {
        id: rid.to_string(),
        board_id: bid.to_string(),
        content: req.content.clone(),
    }))
}

#[delete("/rules/<id>")]
pub async fn delete_rule(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
) -> Result<Status, Status> {
    let rid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let row: Option<(Vec<u8>,)> = sqlx::query_as("SELECT board_id FROM board_rules WHERE id = ?")
        .bind(&rid.as_bytes()[..])
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
    sqlx::query("DELETE FROM board_rules WHERE id = ?")
        .bind(&rid.as_bytes()[..])
        .execute(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    Ok(Status::NoContent)
}

// ---------- Allowed teams (RESTRICTED boards) ----------

#[post("/boards/<id>/teams", format = "json", data = "<req>")]
pub async fn allow_team(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<AssignTeamRequest>,
) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let bid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let tid = Uuid::parse_str(&req.team_id).map_err(|_| Status::BadRequest)?;
    sqlx::query("INSERT INTO board_allowed_teams (board_id, team_id) VALUES (?, ?)")
        .bind(&bid.as_bytes()[..])
        .bind(&tid.as_bytes()[..])
        .execute(pool.inner())
        .await
        .map_err(|_| Status::BadRequest)?;
    Ok(Status::Created)
}

#[delete("/boards/<id>/teams/<team_id>")]
pub async fn disallow_team(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    team_id: &str,
) -> Result<Status, Status> {
    user.require_role(Role::Administrator)?;
    let bid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let tid = Uuid::parse_str(team_id).map_err(|_| Status::BadRequest)?;
    let res = sqlx::query("DELETE FROM board_allowed_teams WHERE board_id = ? AND team_id = ?")
        .bind(&bid.as_bytes()[..])
        .bind(&tid.as_bytes()[..])
        .execute(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    if res.rows_affected() == 0 {
        return Err(Status::NotFound);
    }
    Ok(Status::NoContent)
}
