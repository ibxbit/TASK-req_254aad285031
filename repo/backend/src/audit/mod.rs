// Append-only tamper-evident audit log with per-entity SHA-256 hash chains.
//
//   hash = SHA256( payload_bytes || prev_hash_hex_bytes )
//
// Payload is serialized via `serde_json::to_vec` on a `serde_json::Value`.
// serde_json's default `Map` is a `BTreeMap` which emits keys in
// deterministic alphabetical order -> canonical bytes, reproducible hash.

use sha2::{Digest, Sha256};
use shared::{AuditVerifyIssue, AuditVerifyReport};
use sqlx::{MySql, MySqlPool, Transaction};
use std::collections::HashMap;
use uuid::Uuid;

pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

pub fn compute_hash(payload: &[u8], prev_hash: &str) -> String {
    let mut h = Sha256::new();
    h.update(payload);
    h.update(prev_hash.as_bytes());
    hex::encode(h.finalize())
}

/// Non-transactional helper. Opens its own tx to atomically read the tail
/// and append the new event for this (entity_type, entity_id).
pub async fn record_event(
    pool: &MySqlPool,
    entity_type: &str,
    entity_id: Uuid,
    action: &str,
    payload: &serde_json::Value,
) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    record_event_tx(&mut tx, entity_type, entity_id, action, payload).await?;
    tx.commit().await
}

/// Transactional helper. Use this when the caller already has an open tx
/// so audit write + primary mutation are atomic together.
pub async fn record_event_tx(
    tx: &mut Transaction<'_, MySql>,
    entity_type: &str,
    entity_id: Uuid,
    action: &str,
    payload: &serde_json::Value,
) -> sqlx::Result<()> {
    let payload_bytes = serde_json::to_vec(payload)
        .map_err(|e| sqlx::Error::Protocol(format!("payload encode: {e}")))?;
    let payload_str = std::str::from_utf8(&payload_bytes)
        .map_err(|e| sqlx::Error::Protocol(format!("payload utf8: {e}")))?;

    let tail: Option<(String,)> = sqlx::query_as(
        "SELECT hash FROM event_log \
         WHERE entity_type = ? AND entity_id = ? \
         ORDER BY sequence DESC LIMIT 1 FOR UPDATE",
    )
    .bind(entity_type)
    .bind(&entity_id.as_bytes()[..])
    .fetch_optional(&mut **tx)
    .await?;

    let prev_hash = tail
        .map(|(h,)| h)
        .unwrap_or_else(|| GENESIS_HASH.to_string());

    let hash = compute_hash(&payload_bytes, &prev_hash);
    let event_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO event_log \
         (id, entity_type, entity_id, action, payload, prev_hash, hash) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&event_id.as_bytes()[..])
    .bind(entity_type)
    .bind(&entity_id.as_bytes()[..])
    .bind(action)
    .bind(payload_str)
    .bind(&prev_hash)
    .bind(&hash)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Walk every entity's chain and flag events whose stored hash disagrees with
/// recomputed hash, or whose prev_hash breaks the chain from the previous event.
pub async fn verify_chain(pool: &MySqlPool) -> sqlx::Result<AuditVerifyReport> {
    let rows: Vec<(Vec<u8>, String, Vec<u8>, String, String, String)> = sqlx::query_as(
        "SELECT id, entity_type, entity_id, payload, prev_hash, hash \
         FROM event_log \
         ORDER BY entity_type, entity_id, sequence",
    )
    .fetch_all(pool)
    .await?;

    let mut total = 0i64;
    let mut verified = 0i64;
    let mut tampered = 0i64;
    let mut issues = Vec::new();
    let mut last_hash: HashMap<(String, Vec<u8>), String> = HashMap::new();

    for (id_b, etype, eid_b, payload, prev_hash, hash) in rows {
        total += 1;
        let key = (etype.clone(), eid_b.clone());
        let expected_prev = last_hash
            .get(&key)
            .cloned()
            .unwrap_or_else(|| GENESIS_HASH.to_string());

        let event_id = Uuid::from_slice(&id_b)
            .map(|u| u.to_string())
            .unwrap_or_default();
        let entity_id = Uuid::from_slice(&eid_b)
            .map(|u| u.to_string())
            .unwrap_or_default();

        if prev_hash != expected_prev {
            tampered += 1;
            issues.push(AuditVerifyIssue {
                event_id: event_id.clone(),
                entity_type: etype.clone(),
                entity_id: entity_id.clone(),
                reason: "prev_hash_mismatch".into(),
            });
            last_hash.insert(key, hash);
            continue;
        }

        let recomputed = compute_hash(payload.as_bytes(), &prev_hash);
        if recomputed != hash {
            tampered += 1;
            issues.push(AuditVerifyIssue {
                event_id,
                entity_type: etype,
                entity_id,
                reason: "hash_mismatch".into(),
            });
            last_hash.insert(key, hash);
            continue;
        }

        verified += 1;
        last_hash.insert(key, hash);
    }

    Ok(AuditVerifyReport {
        total_events: total,
        verified,
        tampered,
        issues,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_changes_when_prev_changes() {
        let a = compute_hash(b"x", GENESIS_HASH);
        let b = compute_hash(b"x", "deadbeef");
        assert_ne!(a, b);
    }

    #[test]
    fn hash_is_hex_64_chars() {
        let a = compute_hash(b"payload", GENESIS_HASH);
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn hash_is_stable() {
        let a = compute_hash(b"payload", "prev");
        let b = compute_hash(b"payload", "prev");
        assert_eq!(a, b);
    }
}
