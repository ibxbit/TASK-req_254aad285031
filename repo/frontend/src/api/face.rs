use shared::FaceRecordDetail;

use super::client;

pub async fn list_for_user(user_id: &str) -> Result<Vec<FaceRecordDetail>, String> {
    client::get_json(&format!("/api/faces/{}", user_id)).await
}

pub async fn deactivate(face_id: &str) -> Result<(), String> {
    client::post_empty(&format!("/api/faces/{}/deactivate", face_id)).await
}
