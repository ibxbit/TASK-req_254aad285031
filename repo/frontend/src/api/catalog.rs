use shared::{Category, Service, ServiceComparison, Tag};

use super::client;

// SearchParams + URL builder live in frontend_core for testability.
pub use frontend_core::search::{build_search_path, SearchParams};

pub async fn search(p: &SearchParams) -> Result<Vec<Service>, String> {
    let path = build_search_path(p);
    client::get_json(&path).await
}

pub async fn favorite(service_id: &str) -> Result<(), String> {
    client::post_empty(&format!("/api/services/{}/favorite", service_id)).await
}

pub async fn compare(ids: &[String]) -> Result<Vec<ServiceComparison>, String> {
    if ids.is_empty() {
        return Err("Select at least one service".into());
    }
    if ids.len() > 3 {
        return Err("Compare at most 3 services".into());
    }
    let owned: Vec<String> = ids.to_vec();
    client::get_json(&frontend_core::api_paths::service_compare(&owned)).await
}

// Catalog browsing for the search filter panel.
pub async fn list_categories() -> Result<Vec<Category>, String> {
    client::get_json("/api/categories").await
}

pub async fn list_tags() -> Result<Vec<Tag>, String> {
    client::get_json("/api/tags").await
}
