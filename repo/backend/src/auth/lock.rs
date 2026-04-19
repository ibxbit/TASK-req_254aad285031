use chrono::{Duration, Utc};
use sqlx::MySqlPool;
use uuid::Uuid;

use super::{LOCK_DURATION_MINUTES, MAX_FAILED_ATTEMPTS};

pub async fn is_locked(pool: &MySqlPool, user_id: Uuid) -> sqlx::Result<bool> {
    let row: Option<(Option<chrono::NaiveDateTime>,)> =
        sqlx::query_as("SELECT locked_until FROM users WHERE id = ?")
            .bind(&user_id.as_bytes()[..])
            .fetch_optional(pool)
            .await?;
    Ok(match row {
        Some((Some(until),)) => until > Utc::now().naive_utc(),
        _ => false,
    })
}

pub async fn register_failure(pool: &MySqlPool, user_id: Uuid) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;

    let row: (i32,) =
        sqlx::query_as("SELECT failed_login_count FROM users WHERE id = ? FOR UPDATE")
            .bind(&user_id.as_bytes()[..])
            .fetch_one(&mut *tx)
            .await?;

    let new_count = row.0 + 1;
    if new_count >= MAX_FAILED_ATTEMPTS {
        let until = Utc::now() + Duration::minutes(LOCK_DURATION_MINUTES);
        sqlx::query("UPDATE users SET failed_login_count = 0, locked_until = ? WHERE id = ?")
            .bind(until.naive_utc())
            .bind(&user_id.as_bytes()[..])
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query("UPDATE users SET failed_login_count = ? WHERE id = ?")
            .bind(new_count)
            .bind(&user_id.as_bytes()[..])
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn reset(pool: &MySqlPool, user_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE users SET failed_login_count = 0, locked_until = NULL WHERE id = ?")
        .bind(&user_id.as_bytes()[..])
        .execute(pool)
        .await?;
    Ok(())
}
