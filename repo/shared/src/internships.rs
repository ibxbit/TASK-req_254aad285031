use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ReportType {
    Daily,
    Weekly,
    Monthly,
}

impl ReportType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "daily" => Some(Self::Daily),
            "weekly" => Some(Self::Weekly),
            "monthly" => Some(Self::Monthly),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    OnTime,
    Late,
}

impl ReportStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OnTime => "on_time",
            Self::Late => "late",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "on_time" => Some(Self::OnTime),
            "late" => Some(Self::Late),
            _ => None,
        }
    }
}

// --- Plans ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternshipPlan {
    pub id: String,
    pub intern_id: String,
    pub content: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInternshipPlanRequest {
    pub content: String,
}

// --- Reports ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub intern_id: String,
    #[serde(rename = "type")]
    pub report_type: ReportType,
    pub content: String,
    pub status: ReportStatus,
    pub submitted_at: NaiveDateTime,
    pub due_at: NaiveDateTime,
    pub is_late: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReportRequest {
    #[serde(rename = "type")]
    pub report_type: ReportType,
    pub content: String,
    // Optional override; if omitted, computed from report_type.
    pub due_at: Option<NaiveDateTime>,
}

// --- Attachments ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportAttachment {
    pub id: String,
    pub report_id: String,
    pub file_path: String,
    #[serde(default)]
    pub content_hash: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<i64>,
}

// --- Mentor comments ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentorComment {
    pub id: String,
    pub report_id: String,
    pub mentor_id: String,
    pub content: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMentorCommentRequest {
    pub content: String,
}

// --- Approvals ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportApproval {
    pub id: String,
    pub report_id: String,
    pub mentor_id: String,
    pub approved_at: NaiveDateTime,
}

// --- Dashboard ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportsByType {
    pub daily: i64,
    pub weekly: i64,
    pub monthly: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternDashboard {
    pub intern_id: String,
    pub plans_count: i64,
    pub reports_total: i64,
    pub reports_by_type: ReportsByType,
    pub reports_approved: i64,
    pub reports_pending: i64,
    pub reports_late: i64,
    pub recent_reports: Vec<Report>,
}
