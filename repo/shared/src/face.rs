use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceRecord {
    pub id: String,
    pub user_id: String,
    pub version: i32,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceImage {
    pub id: String,
    pub face_record_id: String,
    pub file_path: String,
    pub hash: String,
    pub perceptual_hash: String,
    pub resolution: String,
    pub brightness_score: f64,
    pub blur_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceAudit {
    pub id: String,
    pub face_record_id: String,
    pub action: String,
    pub performed_by: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceCheckResult {
    pub name: String,
    pub passed: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceValidationResult {
    pub passed: bool,
    pub checks: Vec<FaceCheckResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceLivenessChallenge {
    pub id: String,
    pub face_record_id: String,
    pub challenge: String,
    pub passed: bool,
    pub notes: Option<String>,
    pub performed_by: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceRecordDetail {
    pub record: FaceRecord,
    pub images: Vec<FaceImage>,
    pub audits: Vec<FaceAudit>,
    #[serde(default)]
    pub liveness: Vec<FaceLivenessChallenge>,
}
