pub mod guard;
pub mod lock;
pub mod password;
pub mod session;

pub use guard::AuthUser;

pub const MIN_PASSWORD_LEN: usize = 12;
pub const MAX_FAILED_ATTEMPTS: i32 = 5;
pub const LOCK_DURATION_MINUTES: i64 = 15;
pub const SESSION_LIFETIME_HOURS: i64 = 8;
