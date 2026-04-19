use shared::{
    AssignModeratorRequest, AssignTeamRequest, Board, BoardRule, CreateBoardRequest,
    CreateBoardRuleRequest, CreatePostRequest, CreateZoneRequest, Post, UpdateBoardRequest,
    VisibilityType, Zone,
};

use super::client;

pub async fn list_boards() -> Result<Vec<Board>, String> {
    client::get_json("/api/boards").await
}

pub async fn list_posts(board_id: &str) -> Result<Vec<Post>, String> {
    client::get_json(&format!("/api/boards/{}/posts", board_id)).await
}

pub async fn create_post(board_id: String, title: String, content: String) -> Result<Post, String> {
    let body = CreatePostRequest {
        board_id,
        title,
        content,
    };
    client::post_json("/api/posts", &body).await
}

// ---------- Admin / moderator flows ----------

pub async fn list_zones() -> Result<Vec<Zone>, String> {
    client::get_json("/api/zones").await
}

pub async fn create_zone(name: String) -> Result<Zone, String> {
    let body = CreateZoneRequest { name };
    client::post_json("/api/zones", &body).await
}

pub async fn delete_zone(id: &str) -> Result<(), String> {
    client::delete_empty(&format!("/api/zones/{id}")).await
}

pub async fn create_board(
    zone_id: String,
    name: String,
    visibility: VisibilityType,
) -> Result<Board, String> {
    let body = CreateBoardRequest {
        zone_id,
        name,
        visibility_type: visibility,
    };
    client::post_json("/api/boards", &body).await
}

pub async fn update_board(
    id: &str,
    name: Option<String>,
    visibility: Option<VisibilityType>,
) -> Result<(), String> {
    let body = UpdateBoardRequest {
        name,
        visibility_type: visibility,
    };
    client::patch_json(&format!("/api/boards/{id}"), &body).await
}

pub async fn delete_board(id: &str) -> Result<(), String> {
    client::delete_empty(&format!("/api/boards/{id}")).await
}

pub async fn list_board_rules(board_id: &str) -> Result<Vec<BoardRule>, String> {
    client::get_json(&format!("/api/boards/{board_id}/rules")).await
}

pub async fn create_board_rule(board_id: &str, content: String) -> Result<BoardRule, String> {
    let body = CreateBoardRuleRequest { content };
    client::post_json(&format!("/api/boards/{board_id}/rules"), &body).await
}

pub async fn delete_board_rule(rule_id: &str) -> Result<(), String> {
    client::delete_empty(&format!("/api/rules/{rule_id}")).await
}

pub async fn add_moderator(board_id: &str, user_id: String) -> Result<(), String> {
    let body = AssignModeratorRequest { user_id };
    client::post_json_no_response(&format!("/api/boards/{board_id}/moderators"), &body).await
}

pub async fn remove_moderator(board_id: &str, user_id: &str) -> Result<(), String> {
    client::delete_empty(&format!("/api/boards/{board_id}/moderators/{user_id}")).await
}

pub async fn allow_team(board_id: &str, team_id: String) -> Result<(), String> {
    let body = AssignTeamRequest { team_id };
    client::post_json_no_response(&format!("/api/boards/{board_id}/teams"), &body).await
}

pub async fn disallow_team(board_id: &str, team_id: &str) -> Result<(), String> {
    client::delete_empty(&format!("/api/boards/{board_id}/teams/{team_id}")).await
}
