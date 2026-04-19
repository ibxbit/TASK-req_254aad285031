use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use shared::{LoginRequest, LoginResponse};

// `AuthState` (and its serde shape + `is_logged_in`) lives in
// `frontend_core` so it is testable on the native target without a
// browser. This module wraps it with browser LocalStorage persistence.
pub use frontend_core::auth_state::{AuthState, STORAGE_KEY};

pub fn load_auth_state() -> AuthState {
    LocalStorage::get(STORAGE_KEY).unwrap_or_default()
}

pub fn save_auth_state(state: &AuthState) {
    let _ = LocalStorage::set(STORAGE_KEY, state);
}

pub fn clear_auth_state() {
    LocalStorage::delete(STORAGE_KEY);
}

#[derive(Clone, Copy)]
pub struct AuthSignal(pub Signal<AuthState>);

pub fn use_auth() -> AuthSignal {
    use_context::<AuthSignal>()
}

pub async fn login(username: String, password: String) -> Result<LoginResponse, String> {
    let req = LoginRequest { username, password };
    let resp = gloo_net::http::Request::post("/api/auth/login")
        .json(&req)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    match resp.status() {
        401 => Err("Invalid username or password".into()),
        423 => Err("Account locked. Try again in 15 minutes.".into()),
        s if (200..300).contains(&s) => resp
            .json::<LoginResponse>()
            .await
            .map_err(|e| e.to_string()),
        s => Err(format!("Login failed ({s})")),
    }
}

pub async fn logout(token: &str) {
    let _ = gloo_net::http::Request::post("/api/auth/logout")
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await;
    clear_auth_state();
}
