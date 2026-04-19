use shared::Role;
use sqlx::MySqlPool;
use uuid::Uuid;

// Enforces board visibility at the query level.
// Administrator bypasses filter to support system management.
pub async fn user_can_see_board(
    pool: &MySqlPool,
    user_id: Uuid,
    role: Role,
    board_id: Uuid,
) -> sqlx::Result<bool> {
    if role == Role::Administrator {
        return Ok(true);
    }
    let row: Option<(String,)> = sqlx::query_as("SELECT visibility_type FROM boards WHERE id = ?")
        .bind(&board_id.as_bytes()[..])
        .fetch_optional(pool)
        .await?;
    let Some((vt,)) = row else {
        return Ok(false);
    };
    match vt.as_str() {
        "public" => Ok(true),
        "restricted" => {
            let r: Option<(i64,)> = sqlx::query_as(
                "SELECT 1 FROM board_allowed_teams bat \
                 JOIN user_teams ut ON ut.team_id = bat.team_id \
                 WHERE bat.board_id = ? AND ut.user_id = ? LIMIT 1",
            )
            .bind(&board_id.as_bytes()[..])
            .bind(&user_id.as_bytes()[..])
            .fetch_optional(pool)
            .await?;
            Ok(r.is_some())
        }
        _ => Ok(false),
    }
}
