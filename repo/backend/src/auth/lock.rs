use chrono::{Duration, Utc};
use sqlx::MySqlPool;
use uuid::Uuid;

use super::{LOCK_DURATION_MINUTES, MAX_FAILED_ATTEMPTS};

fn lock_is_active(locked_until: Option<chrono::NaiveDateTime>, now: chrono::NaiveDateTime) -> bool {
    matches!(locked_until, Some(until) if until > now)
}

fn next_failure_state(current_count: i32, now: chrono::DateTime<Utc>) -> (i32, Option<chrono::NaiveDateTime>) {
    let new_count = current_count + 1;
    if new_count >= MAX_FAILED_ATTEMPTS {
        let until = now + Duration::minutes(LOCK_DURATION_MINUTES);
        (0, Some(until.naive_utc()))
    } else {
        (new_count, None)
    }
}

pub async fn is_locked(pool: &MySqlPool, user_id: Uuid) -> sqlx::Result<bool> {
    let row: Option<(Option<chrono::NaiveDateTime>,)> =
        sqlx::query_as("SELECT locked_until FROM users WHERE id = ?")
            .bind(&user_id.as_bytes()[..])
            .fetch_optional(pool)
            .await?;
    Ok(lock_is_active(
        row.and_then(|(until,)| until),
        Utc::now().naive_utc(),
    ))
}

pub async fn register_failure(pool: &MySqlPool, user_id: Uuid) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;

    let row: (i32,) =
        sqlx::query_as("SELECT failed_login_count FROM users WHERE id = ? FOR UPDATE")
            .bind(&user_id.as_bytes()[..])
            .fetch_one(&mut *tx)
            .await?;

    let (next_count, lock_until) = next_failure_state(row.0, Utc::now());
    if let Some(until) = lock_until {
        sqlx::query("UPDATE users SET failed_login_count = 0, locked_until = ? WHERE id = ?")
            .bind(until)
            .bind(&user_id.as_bytes()[..])
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query("UPDATE users SET failed_login_count = ? WHERE id = ?")
            .bind(next_count)
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn lock_is_active_only_for_future_timestamps() {
        let now = Utc::now().naive_utc();
        assert!(!lock_is_active(None, now));
        assert!(!lock_is_active(Some(now), now));
        assert!(!lock_is_active(Some(now - Duration::minutes(1)), now));
        assert!(lock_is_active(Some(now + Duration::minutes(1)), now));
    }

    #[test]
    fn next_failure_state_increments_when_below_threshold() {
        let now = Utc::now();
        let (count, lock_until) = next_failure_state(2, now);
        assert_eq!(count, 3);
        assert!(lock_until.is_none());
    }

    #[test]
    fn next_failure_state_locks_and_resets_at_threshold() {
        let now = Utc::now();
        let (count, lock_until) = next_failure_state(MAX_FAILED_ATTEMPTS - 1, now);
        assert_eq!(count, 0);
        let until = lock_until.expect("lock timestamp should be set at threshold");
        let min_expected = (now + Duration::minutes(LOCK_DURATION_MINUTES)).naive_utc();
        assert!(until >= min_expected);
    }
}
