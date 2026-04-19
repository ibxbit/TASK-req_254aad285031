use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibilityType {
    Public,
    Restricted,
}

impl VisibilityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            VisibilityType::Public => "public",
            VisibilityType::Restricted => "restricted",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "public" => Some(Self::Public),
            "restricted" => Some(Self::Restricted),
            _ => None,
        }
    }
}

// --- Zone ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateZoneRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateZoneRequest {
    pub name: String,
}

// --- Board ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub id: String,
    pub zone_id: String,
    pub name: String,
    pub visibility_type: VisibilityType,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBoardRequest {
    pub zone_id: String,
    pub name: String,
    pub visibility_type: VisibilityType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBoardRequest {
    pub name: Option<String>,
    pub visibility_type: Option<VisibilityType>,
}

// --- Board rules ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardRule {
    pub id: String,
    pub board_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBoardRuleRequest {
    pub content: String,
}

// --- Moderators / Teams ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardModerator {
    pub id: String,
    pub board_id: String,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignModeratorRequest {
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignTeamRequest {
    pub team_id: String,
}

// --- Post ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: String,
    pub board_id: String,
    pub author_id: String,
    pub title: String,
    pub content: String,
    pub is_pinned: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePostRequest {
    pub board_id: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinPostRequest {
    pub is_pinned: bool,
}

// --- Comment ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub post_id: String,
    pub author_id: String,
    pub content: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommentRequest {
    pub post_id: String,
    pub content: String,
}
