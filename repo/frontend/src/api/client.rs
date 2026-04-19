// Minimal HTTP client wrappers. Reads the bearer token from LocalStorage so
// these helpers are callable from anywhere (not tied to Dioxus context).
//
// The GET/POST/PATCH/DELETE helpers cover the JSON-bodied endpoints the UI
// calls. Multipart uploads (review images, report attachments) use the
// browser-native FormData through gloo_net.

use gloo_net::http::{Request, RequestBuilder};
use gloo_storage::{LocalStorage, Storage};
use serde::{de::DeserializeOwned, Serialize};

const STORAGE_KEY: &str = "fsh_auth";

fn bearer() -> Option<String> {
    let state: crate::auth::AuthState = LocalStorage::get(STORAGE_KEY).ok()?;
    state.token.map(|t| format!("Bearer {t}"))
}

fn with_auth(b: RequestBuilder) -> RequestBuilder {
    match bearer() {
        Some(auth) => b.header("Authorization", &auth),
        None => b,
    }
}

fn http_err(status: u16) -> String {
    format!("HTTP {status}")
}

pub async fn get_json<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let resp = with_auth(Request::get(path))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(http_err(resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn post_json<B: Serialize, T: DeserializeOwned>(
    path: &str,
    body: &B,
) -> Result<T, String> {
    let req = with_auth(Request::post(path))
        .json(body)
        .map_err(|e| e.to_string())?;
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(http_err(resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn post_empty(path: &str) -> Result<(), String> {
    let resp = with_auth(Request::post(path))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(http_err(resp.status()));
    }
    Ok(())
}

// POST a JSON body but don't require a JSON response (e.g. 204 NoContent).
pub async fn post_json_no_response<B: Serialize>(path: &str, body: &B) -> Result<(), String> {
    let req = with_auth(Request::post(path))
        .json(body)
        .map_err(|e| e.to_string())?;
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(http_err(resp.status()));
    }
    Ok(())
}

pub async fn patch_json<B: Serialize>(path: &str, body: &B) -> Result<(), String> {
    let req = with_auth(Request::patch(path))
        .json(body)
        .map_err(|e| e.to_string())?;
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(http_err(resp.status()));
    }
    Ok(())
}

pub async fn delete_empty(path: &str) -> Result<(), String> {
    let resp = with_auth(Request::delete(path))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(http_err(resp.status()));
    }
    Ok(())
}

// Multipart file upload. `form_data` is a browser FormData (built in the
// caller from an <input type="file"> event). We don't set Content-Type —
// gloo_net lets the browser populate the boundary automatically.
pub async fn upload_multipart<T: DeserializeOwned>(
    path: &str,
    form_data: &web_sys::FormData,
) -> Result<T, String> {
    let req = with_auth(Request::post(path))
        .body(form_data)
        .map_err(|e| e.to_string())?;
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(http_err(resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}
