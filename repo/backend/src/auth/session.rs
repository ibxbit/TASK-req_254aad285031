use chrono::{Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::MySqlPool;
use uuid::Uuid;

use super::SESSION_LIFETIME_HOURS;

pub fn new_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn hash_token(token: &str) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(token.as_bytes());
    h.finalize().to_vec()
}

pub async fn create(pool: &MySqlPool, user_id: Uuid) -> sqlx::Result<String> {
    let token = new_token();
    let token_hash = hash_token(&token);
    let expires = Utc::now() + Duration::hours(SESSION_LIFETIME_HOURS);
    sqlx::query("INSERT INTO sessions (token_hash, user_id, expires_at) VALUES (?, ?, ?)")
        .bind(token_hash)
        .bind(&user_id.as_bytes()[..])
        .bind(expires.naive_utc())
        .execute(pool)
        .await?;
    Ok(token)
}

pub async fn lookup(pool: &MySqlPool, token: &str) -> sqlx::Result<Option<Uuid>> {
    let token_hash = hash_token(token);
    let row: Option<(Vec<u8>, chrono::NaiveDateTime)> =
        sqlx::query_as("SELECT user_id, expires_at FROM sessions WHERE token_hash = ?")
            .bind(token_hash)
            .fetch_optional(pool)
            .await?;
    let Some((uid_bytes, expires_at)) = row else {
        return Ok(None);
    };
    if expires_at < Utc::now().naive_utc() {
        return Ok(None);
    }
    Ok(Uuid::from_slice(&uid_bytes).ok())
}

pub async fn delete(pool: &MySqlPool, token: &str) -> sqlx::Result<()> {
    let token_hash = hash_token(token);
    sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
        .bind(token_hash)
        .execute(pool)
        .await?;
    Ok(())
}

/// Revoke every active session row for a user. Used by admin deactivation
/// so an in-flight bearer token stops working immediately.
pub async fn delete_all_for_user(pool: &MySqlPool, user_id: Uuid) -> sqlx::Result<u64> {
    let res = sqlx::query("DELETE FROM sessions WHERE user_id = ?")
        .bind(&user_id.as_bytes()[..])
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Same as `delete_all_for_user` but inside an open transaction so the
/// session delete lands atomically with the row that triggered it (e.g.
/// the admin `UPDATE users SET is_active = 0`).
pub async fn delete_all_for_user_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    user_id: Uuid,
) -> sqlx::Result<u64> {
    let res = sqlx::query("DELETE FROM sessions WHERE user_id = ?")
        .bind(&user_id.as_bytes()[..])
        .execute(&mut **tx)
        .await?;
    Ok(res.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_token_produces_64_char_hex_string() {
        let t = new_token();
        assert_eq!(t.len(), 64, "token must be 64 hex chars (32 bytes): {t}");
        assert!(
            t.chars().all(|c| c.is_ascii_hexdigit()),
            "token must be lowercase hex: {t}"
        );
    }

    #[test]
    fn new_token_is_different_each_call() {
        let a = new_token();
        let b = new_token();
        assert_ne!(a, b, "two generated tokens must not be identical");
    }

    #[test]
    fn hash_token_is_deterministic() {
        let h1 = hash_token("abc-token-123");
        let h2 = hash_token("abc-token-123");
        assert_eq!(h1, h2, "same input must produce same hash");
    }

    #[test]
    fn hash_token_produces_32_bytes() {
        let h = hash_token("anything");
        assert_eq!(h.len(), 32, "SHA-256 output must be 32 bytes");
    }

    #[test]
    fn hash_token_differs_for_different_inputs() {
        let h1 = hash_token("token-a");
        let h2 = hash_token("token-b");
        assert_ne!(h1, h2, "different inputs must produce different hashes");
    }

    #[test]
    fn hash_token_known_vector() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let h = hash_token("");
        assert_eq!(
            h[0], 0xe3,
            "first byte of SHA-256('') must be 0xe3 (known vector)"
        );
        assert_eq!(h.len(), 32);
    }
}
