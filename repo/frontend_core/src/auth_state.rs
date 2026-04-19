//! Persisted auth state. Mirrors what the frontend keeps in LocalStorage
//! under the `fsh_auth` key. Serde round-trips matter because the storage
//! payload must survive browser reloads.

use serde::{Deserialize, Serialize};
use shared::SessionUser;

pub const STORAGE_KEY: &str = "fsh_auth";

#[derive(Clone, Default, PartialEq, Serialize, Deserialize, Debug)]
pub struct AuthState {
    pub token: Option<String>,
    pub user: Option<SessionUser>,
}

impl AuthState {
    pub fn is_logged_in(&self) -> bool {
        self.token.is_some() && self.user.is_some()
    }

    pub fn bearer_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {t}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::Role;

    fn sample_user() -> SessionUser {
        SessionUser {
            id: "00000000-0000-0000-0000-000000000001".into(),
            username: "alice".into(),
            role: Role::Administrator,
        }
    }

    #[test]
    fn default_is_logged_out() {
        let s = AuthState::default();
        assert!(!s.is_logged_in());
        assert!(s.bearer_header().is_none());
    }

    #[test]
    fn both_token_and_user_required_for_logged_in() {
        let mut s = AuthState::default();
        s.token = Some("t".into());
        assert!(!s.is_logged_in(), "token alone is not enough");
        s.token = None;
        s.user = Some(sample_user());
        assert!(!s.is_logged_in(), "user alone is not enough");
        s.token = Some("t".into());
        assert!(s.is_logged_in());
    }

    #[test]
    fn bearer_header_formats_correctly() {
        let s = AuthState {
            token: Some("abc123".into()),
            user: Some(sample_user()),
        };
        assert_eq!(s.bearer_header().as_deref(), Some("Bearer abc123"));
    }

    #[test]
    fn round_trips_through_serde_json() {
        let s = AuthState {
            token: Some("xyz".into()),
            user: Some(sample_user()),
        };
        let j = serde_json::to_string(&s).unwrap();
        // Keep the "token" field labelling — frontend and backend agree on it.
        assert!(j.contains("\"token\":\"xyz\""));
        let back: AuthState = serde_json::from_str(&j).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn storage_key_is_stable() {
        // Renaming the storage key silently logs every user out on deploy —
        // pin it so the test fails if someone edits the constant.
        assert_eq!(STORAGE_KEY, "fsh_auth");
    }
}
