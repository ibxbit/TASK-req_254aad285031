use shared::{
    CreateFollowUpReviewRequest, CreateReviewRequest, CreateWorkOrderRequest, Review, ReviewImage,
    ReviewTag, WorkOrder,
};

use super::client;

pub async fn create_order(service_id: String) -> Result<WorkOrder, String> {
    let body = CreateWorkOrderRequest { service_id };
    client::post_json("/api/work-orders", &body).await
}

pub async fn get_order(id: &str) -> Result<WorkOrder, String> {
    client::get_json(&format!("/api/work-orders/{id}")).await
}

pub async fn complete_order(id: &str) -> Result<WorkOrder, String> {
    // complete is POST with no body; the backend returns the updated
    // WorkOrder so we read the JSON back.
    use gloo_storage::Storage as _;
    let path = format!("/api/work-orders/{id}/complete");
    let req = gloo_net::http::Request::post(&path);
    // Re-use the bearer helper by routing through client's post_json — but
    // that requires a body. Implement inline:
    let state: Option<crate::auth::AuthState> = gloo_storage::LocalStorage::get("fsh_auth").ok();
    let builder = if let Some(s) = state {
        if let Some(t) = s.token {
            req.header("Authorization", &format!("Bearer {t}"))
        } else {
            req
        }
    } else {
        req
    };
    let resp = builder.send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn list_review_tags() -> Result<Vec<ReviewTag>, String> {
    client::get_json("/api/review-tags").await
}

pub async fn list_reviews_for_service(service_id: &str) -> Result<Vec<Review>, String> {
    client::get_json(&format!("/api/services/{service_id}/reviews")).await
}

pub async fn create_initial_review(
    work_order_id: String,
    rating: u8,
    text: String,
    tag_ids: Vec<String>,
) -> Result<Review, String> {
    let body = CreateReviewRequest {
        work_order_id,
        rating,
        text,
        tag_ids,
    };
    client::post_json("/api/reviews", &body).await
}

pub async fn create_follow_up_review(
    work_order_id: &str,
    rating: u8,
    text: String,
    tag_ids: Vec<String>,
) -> Result<Review, String> {
    let body = CreateFollowUpReviewRequest {
        rating,
        text,
        tag_ids,
    };
    client::post_json(
        &format!("/api/work-orders/{work_order_id}/follow-up-review"),
        &body,
    )
    .await
}

pub async fn upload_review_image(
    review_id: &str,
    file: &web_sys::File,
) -> Result<ReviewImage, String> {
    let form = web_sys::FormData::new().map_err(|_| "FormData unavailable".to_string())?;
    form.append_with_blob("file", file)
        .map_err(|_| "FormData append failed".to_string())?;
    client::upload_multipart(&format!("/api/reviews/{review_id}/images"), &form).await
}
