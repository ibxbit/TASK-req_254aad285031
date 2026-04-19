//! Services layer — HTTP-agnostic business logic.
//!
//! Controllers delegate to services; services may call repositories (data
//! access) or other services. Services must not import Rocket request types
//! or construct HTTP responses — they return plain `Result`s.
//!
//! These re-exports give every service a stable `crate::services::*` path.
//! Underlying modules currently live at the crate root for historical
//! reasons; the re-exports let callers migrate incrementally.

pub use crate::audit;
pub use crate::face;
pub use crate::forum;
pub use crate::internships;
pub use crate::workorders;

/// Auth-related business logic.
///
/// `password` is pure (no DB). `session` and `lock` also touch the database
/// today — they double as repositories for their narrow domains. New code
/// should prefer the pattern demonstrated in `crate::repositories`.
pub mod auth {
    pub use crate::auth::guard;
    pub use crate::auth::lock;
    pub use crate::auth::password;
    pub use crate::auth::session;
}
