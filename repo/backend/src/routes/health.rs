use rocket::serde::json::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[get("/health")]
pub fn healthcheck() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
