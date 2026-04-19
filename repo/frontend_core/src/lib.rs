//! `frontend_core` — pure-Rust slice of the Dioxus frontend.
//!
//! These modules live outside the wasm-only `frontend` crate so they can be
//! exercised by `cargo test -p frontend_core` on the native target without
//! needing a browser. The `frontend` crate re-imports them and builds the
//! actual UI on top.

pub mod api_paths;
pub mod auth_state;
pub mod compare;
pub mod nav;
pub mod rating;
pub mod route;
pub mod search;
pub mod tag_selection;
pub mod url;
