use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLog {
    pub id: String,
    pub sequence: i64,
    pub entity_type: String,
    pub entity_id: String,
    pub action: String,
    /// Canonical JSON string (exact bytes used in hash computation).
    pub payload: String,
    pub prev_hash: String,
    pub hash: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditVerifyIssue {
    pub event_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditVerifyReport {
    pub total_events: i64,
    pub verified: i64,
    pub tampered: i64,
    pub issues: Vec<AuditVerifyIssue>,
}
