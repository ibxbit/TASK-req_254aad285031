// User CRUD queries. No business logic here.

use sqlx::MySqlPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UserAuthRow {
    pub id: Uuid,
    pub password_hash: String,
    pub role: String,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct UserProfileRow {
    pub id: Uuid,
    pub username: String,
    pub role: String,
    pub is_active: bool,
    pub sensitive_id_mask: Option<String>,
}

pub async fn count(pool: &MySqlPool) -> sqlx::Result<i64> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub async fn create(
    pool: &MySqlPool,
    id: Uuid,
    username: &str,
    password_hash: &str,
    role: &str,
) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO users (id, username, password_hash, role) VALUES (?, ?, ?, ?)")
        .bind(&id.as_bytes()[..])
        .bind(username)
        .bind(password_hash)
        .bind(role)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_auth_by_username(
    pool: &MySqlPool,
    username: &str,
) -> sqlx::Result<Option<UserAuthRow>> {
    let row: Option<(Vec<u8>, String, String, i8)> =
        sqlx::query_as("SELECT id, password_hash, role, is_active FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(pool)
            .await?;
    Ok(row.and_then(|(id_b, hash, role, is_active)| {
        let id = Uuid::from_slice(&id_b).ok()?;
        Some(UserAuthRow {
            id,
            password_hash: hash,
            role,
            is_active: is_active != 0,
        })
    }))
}

pub async fn find_profile_by_id(
    pool: &MySqlPool,
    id: Uuid,
) -> sqlx::Result<Option<UserProfileRow>> {
    let row: Option<(String, String, i8, Option<String>)> = sqlx::query_as(
        "SELECT username, role, is_active, sensitive_id_mask FROM users WHERE id = ?",
    )
    .bind(&id.as_bytes()[..])
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(username, role, is_active, mask)| UserProfileRow {
        id,
        username,
        role,
        is_active: is_active != 0,
        sensitive_id_mask: mask,
    }))
}

pub async fn list_all(pool: &MySqlPool) -> sqlx::Result<Vec<UserProfileRow>> {
    let rows: Vec<(Vec<u8>, String, String, i8, Option<String>)> = sqlx::query_as(
        "SELECT id, username, role, is_active, sensitive_id_mask FROM users ORDER BY username",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(id_b, username, role, is_active, mask)| {
            let id = Uuid::from_slice(&id_b).ok()?;
            Some(UserProfileRow {
                id,
                username,
                role,
                is_active: is_active != 0,
                sensitive_id_mask: mask,
            })
        })
        .collect())
}

pub async fn update_role(pool: &MySqlPool, id: Uuid, role: &str) -> sqlx::Result<u64> {
    let res = sqlx::query("UPDATE users SET role = ? WHERE id = ?")
        .bind(role)
        .bind(&id.as_bytes()[..])
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

pub async fn set_active(pool: &MySqlPool, id: Uuid, is_active: bool) -> sqlx::Result<u64> {
    let res = sqlx::query("UPDATE users SET is_active = ? WHERE id = ?")
        .bind(is_active as i8)
        .bind(&id.as_bytes()[..])
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

pub async fn set_password(pool: &MySqlPool, id: Uuid, password_hash: &str) -> sqlx::Result<u64> {
    let res = sqlx::query(
        "UPDATE users SET password_hash = ?, failed_login_count = 0, \
                        locked_until = NULL WHERE id = ?",
    )
    .bind(password_hash)
    .bind(&id.as_bytes()[..])
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

pub async fn set_sensitive_id(
    pool: &MySqlPool,
    id: Uuid,
    ciphertext: &[u8],
    mask: &str,
) -> sqlx::Result<u64> {
    let res =
        sqlx::query("UPDATE users SET sensitive_id_enc = ?, sensitive_id_mask = ? WHERE id = ?")
            .bind(ciphertext)
            .bind(mask)
            .bind(&id.as_bytes()[..])
            .execute(pool)
            .await?;
    Ok(res.rows_affected())
}

pub async fn get_sensitive_id_enc(pool: &MySqlPool, id: Uuid) -> sqlx::Result<Option<Vec<u8>>> {
    let row: Option<(Option<Vec<u8>>,)> =
        sqlx::query_as("SELECT sensitive_id_enc FROM users WHERE id = ?")
            .bind(&id.as_bytes()[..])
            .fetch_optional(pool)
            .await?;
    Ok(row.and_then(|(v,)| v))
}
