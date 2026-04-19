use shared::Role;
use sqlx::MySqlPool;
use uuid::Uuid;

// Scoped per board: only users assigned to board_moderators for THIS board
// (plus Administrator) are allowed to moderate it. The "moderator" role alone
// grants no forum powers.
pub async fn is_board_moderator(
    pool: &MySqlPool,
    board_id: Uuid,
    user_id: Uuid,
) -> sqlx::Result<bool> {
    let r: Option<(i64,)> =
        sqlx::query_as("SELECT 1 FROM board_moderators WHERE board_id = ? AND user_id = ? LIMIT 1")
            .bind(&board_id.as_bytes()[..])
            .bind(&user_id.as_bytes()[..])
            .fetch_optional(pool)
            .await?;
    Ok(r.is_some())
}

pub async fn can_moderate_board(
    pool: &MySqlPool,
    role: Role,
    board_id: Uuid,
    user_id: Uuid,
) -> sqlx::Result<bool> {
    if role == Role::Administrator {
        return Ok(true);
    }
    is_board_moderator(pool, board_id, user_id).await
}
