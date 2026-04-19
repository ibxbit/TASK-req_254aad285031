use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkOrderStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl WorkOrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "in_progress" => Some(Self::InProgress),
            "completed" => Some(Self::Completed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

// --- Work order ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkOrder {
    pub id: String,
    pub requester_id: String,
    pub service_id: String,
    pub status: WorkOrderStatus,
    pub completed_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkOrderRequest {
    pub service_id: String,
}

// --- Review ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewKind {
    Initial,
    FollowUp,
}

impl ReviewKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::FollowUp => "follow_up",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "initial" => Some(Self::Initial),
            "follow_up" => Some(Self::FollowUp),
            _ => None,
        }
    }
}

fn default_kind() -> ReviewKind {
    ReviewKind::Initial
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub id: String,
    pub work_order_id: String,
    pub user_id: String,
    pub rating: u8,
    pub text: String,
    pub is_pinned: bool,
    pub is_collapsed: bool,
    #[serde(default = "default_kind")]
    pub kind: ReviewKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_review_id: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReviewRequest {
    pub work_order_id: String,
    pub rating: u8,
    pub text: String,
    // Requester-side tag selection at submission time (optional). Each entry
    // must resolve to an existing row in `review_tags`. Unknown tag ids are
    // rejected with 400 — avoids silent discards.
    #[serde(default)]
    pub tag_ids: Vec<String>,
}

// Follow-up review: one per completed work order, must be submitted by
// the same requester, and only *after* the initial review already exists.
// Same 14-day window / 3-per-day anti-spam cap as the initial review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFollowUpReviewRequest {
    pub rating: u8,
    pub text: String,
    #[serde(default)]
    pub tag_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinReviewRequest {
    pub is_pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollapseReviewRequest {
    pub is_collapsed: bool,
}

// --- Review image ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewImage {
    pub id: String,
    pub review_id: String,
    pub file_path: String,
    pub size: i32,
    pub content_type: String,
    #[serde(default)]
    pub content_hash: Option<String>,
}

// --- Review tags ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewTag {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReviewTagRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignReviewTagRequest {
    pub tag_id: String,
}

// --- Reputation ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationBreakdownEntry {
    pub review_id: String,
    pub rating: u8,
    pub days_since: f64,
    pub weight: f64,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reputation {
    pub service_id: String,
    pub final_score: f64,
    pub total_reviews: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breakdown: Option<Vec<ReputationBreakdownEntry>>,
}
